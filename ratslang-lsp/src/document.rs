use crate::text;
use std::collections::BTreeMap;
use tower_lsp_server::ls_types::*;
use tree_sitter::{Node, Parser, Tree};

#[derive(Debug, Clone)]
pub struct DocumentState {
    pub uri: Uri,
    pub content: String,
    pub version: i32,
    pub tree: Tree,
    pub symbols: SymbolTable,
    pub schema_ref: Option<String>,
    pub schema_range: Option<Range>,
}
#[derive(Debug, Clone, Default)]
pub struct SymbolTable {
    pub namespaces: BTreeMap<String, NamespaceInfo>,
    pub assignments: Vec<AssignmentInfo>,
    pub includes: Vec<IncludeInfo>,
}
#[derive(Debug, Clone)]
pub struct NamespaceInfo {
    pub parent: String,
    pub range: Range,
    pub selection_range: Range,
    pub bytes: std::ops::Range<usize>,
    pub documentation: String,
    pub recommended: bool,
}
#[derive(Debug, Clone)]
pub struct AssignmentInfo {
    pub name: String,
    pub full_path: String,
    pub namespace: String,
    pub range: Range,
    pub selection_range: Range,
    pub value_range: Range,
    pub value_bytes: std::ops::Range<usize>,
    pub expression: String,
    pub annotations: Vec<String>,
    pub documentation: String,
    pub is_internal: bool,
}
#[derive(Debug, Clone)]
pub struct IncludeInfo {
    pub path: String,
    pub namespace: String,
    pub range: Range,
}
fn qualify(namespace: &str, name: &str) -> String {
    if namespace.is_empty() {
        name.into()
    } else {
        format!("{namespace}.{name}")
    }
}
pub fn comments(source: &str, byte: usize) -> (Vec<String>, String) {
    let mut annotations = Vec::new();
    let mut docs = Vec::new();
    for line in source[..byte].lines().rev() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let Some(comment) = line.strip_prefix('#') else {
            break;
        };
        let comment = comment.trim();
        if let Some(annotation) = comment.strip_prefix('@') {
            annotations.push(annotation.to_owned());
        } else {
            docs.push(comment.to_owned());
        }
    }
    annotations.reverse();
    docs.reverse();
    if let Some(description) = annotations.iter().find_map(|a| {
        a.strip_prefix("description")
            .and_then(|a| a.trim().strip_prefix('='))
    }) {
        return (
            annotations.clone(),
            description.trim().trim_matches('"').to_owned(),
        );
    }
    (annotations, docs.join("\n"))
}
// A schema directive is editor metadata, confined to the leading comment block.
// The last header directive wins, matching ordinary Ratslang assignments.
fn header_schema(source: &str) -> Option<(String, Range)> {
    let mut found = None;
    let mut offset = 0;
    for line in source.split_inclusive('\n') {
        let trimmed = line.trim();
        if !trimmed.is_empty() {
            let Some(comment) = trimmed.strip_prefix('#') else {
                break;
            };
            if let Some(rest) = comment.trim_start().strip_prefix("@schema")
                && (rest.is_empty() || rest.starts_with(|c: char| c.is_whitespace() || c == '='))
            {
                let value = rest.trim().strip_prefix('=').unwrap_or(rest.trim()).trim();
                let reference =
                    serde_json::from_str::<String>(value).unwrap_or_else(|_| value.to_owned());
                let start = offset + line.find('#').unwrap();
                found = Some((
                    reference,
                    Range::new(
                        text::position(source, start),
                        text::position(source, offset + line.trim_end().len()),
                    ),
                ));
            }
        }
        offset += line.len();
    }
    found
}
impl DocumentState {
    pub fn new(uri: Uri, content: String, version: i32) -> Self {
        let mut parser = Parser::new();
        parser
            .set_language(&tree_sitter_ratslang::LANGUAGE.into())
            .expect("compatible grammar");
        let tree = parser.parse(&content, None).expect("parser not cancelled");
        let symbols = SymbolTable::from_tree(&tree, &content);
        let declaration = header_schema(&content).or_else(|| {
            symbols
                .assignments
                .iter()
                .rev()
                .find(|a| a.full_path == "_schema")
                .map(|a| {
                    (
                        a.expression.trim().trim_matches('"').to_owned(),
                        a.value_range,
                    )
                })
        });
        let (schema_ref, schema_range) = match declaration {
            Some((reference, range)) => (Some(reference), Some(range)),
            None => (None, None),
        };
        Self {
            uri,
            content,
            version,
            tree,
            symbols,
            schema_ref,
            schema_range,
        }
    }
    pub fn update(&mut self, content: String, version: i32) {
        *self = Self::new(self.uri.clone(), content, version);
    }
    pub fn apply_changes(
        &mut self,
        changes: Vec<TextDocumentContentChangeEvent>,
        version: i32,
    ) -> bool {
        if version <= self.version {
            return false;
        }
        let mut content = self.content.clone();
        for change in changes {
            if let Some(range) = change.range {
                let (Some(start), Some(end)) = (
                    text::offset(&content, range.start),
                    text::offset(&content, range.end),
                ) else {
                    return false;
                };
                if start > end {
                    return false;
                }
                content.replace_range(start..end, &change.text);
            } else {
                content = change.text;
            }
        }
        self.update(content, version);
        true
    }
    pub fn byte_to_position(byte: usize, content: &str) -> Position {
        text::position(content, byte)
    }
    pub fn namespace_at(&self, byte: usize) -> &str {
        self.symbols
            .namespaces
            .iter()
            .filter(|(_, ns)| ns.bytes.contains(&byte))
            .max_by_key(|(name, _)| name.len())
            .map(|(name, _)| name.as_str())
            .unwrap_or("")
    }
    pub fn assignment_at(&self, at: Position) -> Option<&AssignmentInfo> {
        self.symbols
            .assignments
            .iter()
            .rev()
            .find(|a| a.range.start <= at && at <= a.range.end)
    }
    pub fn path_at(&self, at: Position) -> Option<String> {
        if let Some(a) = self.assignment_at(at)
            && at <= a.selection_range.end
        {
            return Some(a.full_path.clone());
        }
        let byte = text::offset(&self.content, at)?;
        let word = &self.content[text::word(&self.content, byte)];
        if word.is_empty() {
            return None;
        }
        let namespace = self.namespace_at(byte);
        let local = qualify(namespace, word);
        if self
            .symbols
            .assignments
            .iter()
            .any(|a| a.full_path == local)
        {
            Some(local)
        } else {
            Some(word.to_owned())
        }
    }
}
impl SymbolTable {
    pub fn from_tree(tree: &Tree, content: &str) -> Self {
        let mut symbols = Self::default();
        symbols.walk(tree.root_node(), content, "");
        if let Ok(assignments) = ratslang::tooling::source_assignments(content) {
            symbols.assignments = assignments
                .into_iter()
                .map(|assignment| {
                    let (_, documentation) = comments(content, assignment.statement.start);
                    let name = assignment.name;
                    AssignmentInfo {
                        full_path: qualify(&assignment.namespace, &name),
                        is_internal: name
                            .rsplit('.')
                            .next()
                            .is_some_and(|part| part.starts_with('_')),
                        name,
                        namespace: assignment.namespace,
                        range: text::range(content, assignment.statement),
                        selection_range: text::range(content, assignment.selection),
                        value_range: text::range(content, assignment.value.clone()),
                        expression: content[assignment.value.clone()].to_owned(),
                        value_bytes: assignment.value,
                        annotations: assignment.annotations,
                        documentation,
                    }
                })
                .collect();
        }
        symbols
    }
    fn walk(&mut self, node: Node, source: &str, namespace: &str) {
        let value = |node: Node| node.utf8_text(source.as_bytes()).unwrap_or("").to_owned();
        let children: Vec<_> = node
            .named_children(&mut node.walk())
            .filter(|n| n.kind() != "comment")
            .collect();
        match node.kind() {
            "namespace_block" => {
                if let Some(name) = children.first() {
                    let full = qualify(namespace, &value(*name));
                    let (annotations, documentation) = comments(source, node.start_byte());
                    self.namespaces.insert(
                        full.clone(),
                        NamespaceInfo {
                            parent: namespace.into(),
                            range: text::range(source, node.byte_range()),
                            selection_range: text::range(source, name.byte_range()),
                            bytes: node.byte_range(),
                            documentation,
                            recommended: annotations.iter().any(|a| a == "recommended"),
                        },
                    );
                    if let Some(block) = children.iter().find(|n| n.kind() == "block") {
                        self.walk(*block, source, &full);
                    }
                }
            }
            "assignment" => {
                let Some(rhs) = children.iter().find(|n| n.kind() == "rhs_values") else {
                    return;
                };
                let values: Vec<_> = rhs
                    .named_children(&mut rhs.walk())
                    .filter(|n| n.kind() == "rhs_value")
                    .collect();
                let (annotations, documentation) = comments(source, node.start_byte());
                for (index, lhs) in children
                    .iter()
                    .filter(|n| {
                        matches!(
                            n.kind(),
                            "identifier" | "internal_variable" | "variable_path"
                        )
                    })
                    .enumerate()
                {
                    let name = value(*lhs);
                    let Some(rhs) = values
                        .get(index)
                        .or_else(|| values.first().filter(|_| values.len() == 1))
                    else {
                        continue;
                    };
                    self.assignments.push(AssignmentInfo {
                        full_path: qualify(namespace, &name),
                        namespace: namespace.into(),
                        is_internal: name
                            .rsplit('.')
                            .next()
                            .is_some_and(|part| part.starts_with('_')),
                        name,
                        range: text::range(source, node.byte_range()),
                        selection_range: text::range(source, lhs.byte_range()),
                        value_range: text::range(source, rhs.byte_range()),
                        value_bytes: rhs.byte_range(),
                        expression: value(*rhs),
                        annotations: annotations.clone(),
                        documentation: documentation.clone(),
                    });
                }
            }
            "include" => {
                if let Some(path) = children.iter().find(|n| n.kind() == "path") {
                    self.includes.push(IncludeInfo {
                        path: value(*path),
                        namespace: namespace.into(),
                        range: text::range(source, path.byte_range()),
                    });
                }
            }
            _ => {
                for child in children {
                    self.walk(child, source, namespace);
                }
            }
        }
    }
}
