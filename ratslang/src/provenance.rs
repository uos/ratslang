//! Source locations and copy-on-assignment history, retained by the compiler.
use crate::*;
use logos::Logos;
use std::collections::BTreeMap;

/// A byte range and one-based Unicode line/column in the original input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceLocation {
    pub file: Option<PathBuf>,
    pub bytes: std::ops::Range<usize>,
    pub line: usize,
    pub column: usize,
}
/// One assignment before and after reference resolution.
#[derive(Debug, Clone, PartialEq)]
pub struct Definition {
    pub name: String,
    pub expression: Rhs,
    pub value: Rhs,
    pub location: Option<SourceLocation>,
    pub included_from: Vec<SourceLocation>,
    pub initial_state: bool,
}
/// The effective definition, earlier assignments, and referenced definitions.
#[derive(Debug, Clone, PartialEq)]
pub struct Explanation {
    pub name: String,
    pub value: Rhs,
    pub winner: Definition,
    pub history: Vec<Definition>,
    pub dependencies: Vec<Definition>,
}
#[derive(Debug, Clone, Default)]
pub(crate) struct Trace {
    records: Vec<Definition>,
    dependencies: Vec<Vec<usize>>,
    latest: HashMap<Var, usize>,
}
pub(crate) type Locations = Rc<RefCell<BTreeMap<usize, std::ops::Range<usize>>>>;
pub(crate) struct Parsed {
    pub ast: Vec<StatementKindOwned>,
    pub locations: Vec<(SourceLocation, Vec<SourceLocation>)>,
}
#[derive(Clone)]
pub(crate) struct Context<'a> {
    pub source: &'a str,
    pub file: Option<PathBuf>,
    pub byte_offset: usize,
    pub line_offset: usize,
    pub included_from: Vec<SourceLocation>,
}
impl Context<'_> {
    pub fn location(&self, range: std::ops::Range<usize>) -> SourceLocation {
        let start = range.start.min(self.source.len());
        let end = Token::lexer(&self.source[start..range.end.min(self.source.len())])
            .spanned()
            .filter(|(token, _)| !matches!(token, Ok(Token::NewLine)))
            .map(|(_, span)| start + span.end)
            .last()
            .unwrap_or(start);
        let before = &self.source[..start];
        let line = before.bytes().filter(|b| *b == b'\n').count() + 1 + self.line_offset;
        let column = before.rsplit('\n').next().unwrap_or("").chars().count() + 1;
        SourceLocation {
            file: self.file.clone(),
            bytes: range.start + self.byte_offset..end + self.byte_offset,
            line,
            column,
        }
    }
}
impl Trace {
    pub fn build(
        original: &[StatementKindOwned],
        resolved: &[StatementKindOwned],
        locations: &[(SourceLocation, Vec<SourceLocation>)],
        state: &HashMap<Var, Rhs>,
    ) -> Self {
        let mut trace = Self::default();
        // Initial state has the same lookup priority as the compiler cache.
        let mut initial = HashMap::new();
        let mut sorted: Vec<_> = state.iter().collect();
        sorted.sort_by_key(|(var, _)| format_var(var));
        for (var, value) in sorted {
            let index = trace.records.len();
            trace.records.push(Definition {
                name: format_var(var),
                expression: value.clone(),
                value: value.clone(),
                location: None,
                included_from: vec![],
                initial_state: true,
            });
            trace.dependencies.push(vec![]);
            initial.insert(var.clone(), index);
            trace.latest.insert(var.clone(), index);
        }
        for (i, (original, resolved)) in original.iter().zip(resolved).enumerate() {
            let StatementKindOwned::VariableDef(Statement::AssignLeft { lhs, rhs, .. }) = original;
            let StatementKindOwned::VariableDef(Statement::AssignLeft { rhs: values, .. }) =
                resolved;
            let mut pending = Vec::new();
            for (j, var) in lhs.iter().enumerate() {
                // Resolution of duplicate LHS names uses their first position.
                if lhs[..j].contains(var) {
                    continue;
                }
                let expression = rhs[if rhs.len() == 1 { 0 } else { j }].clone();
                let mut dependencies = Vec::new();
                references(&expression, &mut |reference| {
                    if let Some(index) = initial
                        .get(reference)
                        .or_else(|| trace.latest.get(reference))
                    {
                        if !dependencies.contains(index) {
                            dependencies.push(*index);
                        }
                    }
                });
                let (location, included_from) = &locations[i];
                let index = trace.records.len();
                trace.records.push(Definition {
                    name: format_var(var),
                    expression,
                    value: values[if values.len() == 1 { 0 } else { j }].clone(),
                    location: Some(location.clone()),
                    included_from: included_from.clone(),
                    initial_state: false,
                });
                trace.dependencies.push(dependencies);
                pending.push((var.clone(), index));
            }
            for (var, index) in pending {
                if !initial.contains_key(&var) {
                    trace.latest.insert(var, index);
                }
            }
        }
        trace
    }
    fn explain(&self, var: &Var, value: Rhs) -> Option<Explanation> {
        let index = *self.latest.get(var)?;
        let name = format_var(var);
        let mut indices = std::collections::BTreeSet::new();
        let mut pending = self.dependencies[index].clone();
        while let Some(i) = pending.pop() {
            if indices.insert(i) {
                pending.extend(&self.dependencies[i]);
            }
        }
        Some(Explanation {
            name: name.clone(),
            value,
            winner: self.records[index].clone(),
            history: self
                .records
                .iter()
                .filter(|r| r.name == name)
                .cloned()
                .collect(),
            dependencies: indices
                .into_iter()
                .map(|i| self.records[i].clone())
                .collect(),
        })
    }
}
fn references(rhs: &Rhs, visit: &mut impl FnMut(&Var)) {
    match rhs {
        Rhs::Var(var) => visit(var),
        Rhs::Array(values) => {
            for value in values {
                references(value, visit);
            }
        }
        _ => {}
    }
}
impl VariableHistory {
    /// Explain a value without recording an application read. Namespace views are supported.
    pub fn explain(&self, name: &str) -> anyhow::Result<Option<Explanation>> {
        let var: Var = name.parse()?;
        let full = Self::prepend_ns(&var, &self.namespace_context);
        let Some(value) = self.resolve_recursive(&var, None)? else {
            return Ok(None);
        };
        Ok(self.trace.explain(&full, value))
    }
}

#[cfg(test)]
#[path = "provenance_tests.rs"]
mod tests;
