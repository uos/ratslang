//! Schema validation, source-preserving edits, and layered configuration tooling.
use crate::*;
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IssueKind {
    Unknown,
    Required,
    Deprecated,
    Invalid,
}
#[derive(Debug, Clone)]
pub struct ConfigIssue {
    pub name: String,
    pub kind: IssueKind,
    pub message: String,
    pub replacement: Option<String>,
    pub location: Option<SourceLocation>,
}
impl ConfigIssue {
    pub fn is_error(&self) -> bool {
        matches!(self.kind, IssueKind::Required | IssueKind::Invalid)
    }
}
/// A deterministic, unique nearby name, accounting for adjacent transpositions.
pub fn suggest_setting<'a>(
    name: &str,
    candidates: impl IntoIterator<Item = &'a str>,
) -> Option<&'a str> {
    let length = name.chars().count();
    if length > 256 {
        return None;
    }
    let limit = if length < 5 {
        1
    } else if length < 12 {
        2
    } else {
        3
    };
    let mut best = limit + 1;
    let mut result = None;
    for candidate in candidates {
        if candidate == name {
            return None;
        }
        let other_length = candidate.chars().count();
        if other_length > 256 || length.abs_diff(other_length) > limit {
            continue;
        }
        let distance = spelling_distance(name, candidate);
        if distance < best {
            best = distance;
            result = Some(candidate);
        } else if distance == best {
            result = None;
        }
    }
    result
}
fn spelling_distance(a: &str, b: &str) -> usize {
    let a: Vec<_> = a.chars().collect();
    let b: Vec<_> = b.chars().collect();
    let mut previous_previous: Vec<usize> = (0..=b.len()).collect();
    let mut previous = previous_previous.clone();
    for (i, ac) in a.iter().enumerate() {
        let mut current = vec![i + 1; b.len() + 1];
        for (j, bc) in b.iter().enumerate() {
            current[j + 1] = (previous[j + 1] + 1)
                .min(current[j] + 1)
                .min(previous[j] + usize::from(ac != bc));
            if i > 0 && j > 0 && *ac == b[j - 1] && a[i - 1] == *bc {
                current[j + 1] = current[j + 1].min(previous_previous[j - 1] + 1);
            }
        }
        previous_previous = previous;
        previous = current;
    }
    previous[b.len()]
}
fn names(vars: &VariableHistory) -> BTreeSet<String> {
    vars.ast
        .iter()
        .flat_map(|statement| {
            let StatementKindOwned::VariableDef(Statement::AssignLeft { lhs, .. }) = statement;
            lhs.iter().map(format_var)
        })
        .chain(vars.var_cache.keys().map(format_var))
        .collect()
}
fn values(vars: &VariableHistory) -> anyhow::Result<BTreeMap<String, Rhs>> {
    names(vars)
        .into_iter()
        .map(|name| {
            let value = vars
                .resolve_recursive(&name.parse()?, None)?
                .ok_or_else(|| anyhow!("missing value: {name}"))?;
            Ok((name, value))
        })
        .collect()
}
/// Validate explicit user settings against defaults, including presence and lifecycle metadata.
pub fn validate_schema(
    user: &VariableHistory,
    defaults: &VariableHistory,
) -> anyhow::Result<Vec<ConfigIssue>> {
    validate_effective(user, defaults, &names(user))
}
fn validate_effective(
    effective: &VariableHistory,
    defaults: &VariableHistory,
    provided: &BTreeSet<String>,
) -> anyhow::Result<Vec<ConfigIssue>> {
    let known = names(defaults);
    let rules = defaults.constraints()?;
    let local_rules = effective.constraints()?;
    let mut issues = Vec::new();
    for (name, rule) in &rules {
        if rule.required == Some(true) && !provided.contains(name) {
            issues.push(ConfigIssue {
                name: name.clone(),
                kind: IssueKind::Required,
                message: format!("Required setting {name} must be explicitly assigned"),
                replacement: None,
                location: None,
            });
        }
    }
    for name in provided {
        if name.split('.').any(|part| part.starts_with('_')) {
            continue;
        }
        let location = effective.explain(name)?.and_then(|e| e.winner.location);
        let issue = |kind, message, replacement| ConfigIssue {
            name: name.clone(),
            kind,
            message,
            replacement,
            location: location.clone(),
        };
        if !known.contains(name) {
            let suggestion =
                suggest_setting(name, known.iter().map(String::as_str)).map(str::to_owned);
            let message = match &suggestion {
                Some(s) => format!("Unknown setting {name}; did you mean {s}?"),
                None => format!("Unknown setting {name}"),
            };
            issues.push(issue(IssueKind::Unknown, message, suggestion));
            continue;
        }
        if let Some(rule) = rules.get(name) {
            if rule.deprecated.is_some() || rule.renamed_to.is_some() {
                issues.push(issue(
                    IssueKind::Deprecated,
                    format!(
                        "{name}: {}{}",
                        rule.deprecated
                            .as_deref()
                            .unwrap_or("This setting was renamed"),
                        rule.renamed_to
                            .as_ref()
                            .map(|s| format!("; use {s}"))
                            .unwrap_or_default()
                    ),
                    rule.renamed_to.clone(),
                ));
            }
            let rule = local_rules
                .get(name)
                .map(|later| rule.overlay(later))
                .unwrap_or_else(|| rule.clone());
            if let Some(value) = effective.resolve_recursive(&name.parse()?, None)? {
                if let Err(error) = rule.validate(name, &value) {
                    issues.push(issue(IssueKind::Invalid, error.to_string(), None));
                }
            }
        }
    }
    Ok(issues)
}
/// Defaults, a user file, and CLI assignments compiled as one ordered program.
/// Includes retain their actual source file and include chain in `explain`.
pub struct Configuration {
    pub compiled: Evaluated,
    pub schema: Option<Evaluated>,
    pub input: PathBuf,
    source: String,
    schema_path: Option<PathBuf>,
    provided: BTreeSet<String>,
    overrides: Vec<String>,
}
impl Configuration {
    pub fn load(input: &Path, schema: Option<&Path>, overrides: &[String]) -> anyhow::Result<Self> {
        let input = input.canonicalize()?;
        let source = std::fs::read_to_string(&input)?;
        let parent = input.parent().unwrap_or(Path::new("."));
        let schema_path = schema.map(|p| p.canonicalize()).transpose()?;
        let schema = schema_path
            .as_ref()
            .map(|p| compile_file(p, None, None))
            .transpose()?;
        let user = VariableHistory::new(parse_to_ast(&input, vec![])?.ast);
        let mut provided = names(&user);
        for value in overrides {
            let (key, _) = value
                .split_once('=')
                .ok_or_else(|| anyhow!("--set expects KEY=VALUE"))?;
            anyhow::ensure!(
                key.trim().split('.').all(crate::format::valid_key),
                "invalid override key"
            );
            let override_program = compile_code(value)?;
            anyhow::ensure!(
                override_program.vars.ast.len() == 1 && names(&override_program.vars).len() == 1,
                "--set expects one KEY=VALUE assignment"
            );
            provided.extend(names(&override_program.vars));
        }
        let mut files = Vec::new();
        if let Some(path) = &schema_path {
            files.push(path.clone());
        }
        files.push(input.clone());
        let cli_source = format!("{}\n", overrides.join("\n"));
        let mut compiled = compile_layers(&files, &cli_source, parent, None)?;
        compiled.vars.warn_on_drop = false;
        Ok(Self {
            compiled,
            schema,
            input,
            source,
            schema_path,
            provided,
            overrides: overrides.to_vec(),
        })
    }
    pub fn issues(&self) -> anyhow::Result<Vec<ConfigIssue>> {
        match &self.schema {
            Some(schema) => validate_effective(&self.compiled.vars, &schema.vars, &self.provided),
            None => Ok(vec![]),
        }
    }
    pub fn explain(&self, name: &str) -> anyhow::Result<Option<Explanation>> {
        self.compiled.vars.explain(name)
    }
    /// "CLI", "file", "default", or "include" for the winning definition.
    pub fn layer(&self, definition: &Definition) -> &'static str {
        match definition.location.as_ref().and_then(|l| l.file.as_ref()) {
            None => "CLI",
            Some(path) if path == &self.input => "file",
            Some(path) if Some(path) == self.schema_path.as_ref() => "default",
            Some(_) => "include",
        }
    }
    fn preview(&self, source: &str) -> anyhow::Result<Evaluated> {
        let files: Vec<_> = self.schema_path.iter().cloned().collect();
        let source = format!("{source}\n{}\n", self.overrides.join("\n"));
        compile_layers(
            &files,
            &source,
            self.input.parent().unwrap(),
            Some(self.input.clone()),
        )
    }
    /// Comment out redundant literal overrides, retaining every comment and original value.
    /// Annotated assignments, references, includes, and multi-assignments are kept.
    /// The edit is accepted only after recompilation proves effective values unchanged.
    pub fn minimize(&self) -> anyhow::Result<String> {
        let schema = self
            .schema
            .as_ref()
            .ok_or_else(|| anyhow!("minimize requires --schema"))?;
        let rules = schema.vars.constraints()?;
        let user = compile_file(&self.input, None, None)?;
        let mut edits = Vec::new();
        for name in &self.provided {
            if rules.get(name).is_some_and(|r| r.required == Some(true)) {
                continue;
            }
            let Some(explanation) = user.vars.explain(name)? else {
                continue;
            };
            let winner = explanation.winner;
            let Some(location) = &winner.location else {
                continue;
            };
            if location.file.as_ref() != Some(&self.input) || !explanation.dependencies.is_empty() {
                continue;
            }
            if constraints::annotations_before(&self.source, location.bytes.start)
                .iter()
                .any(|a| !schema_annotation(a))
            {
                continue;
            }
            let statement = &self.source[location.bytes.clone()];
            if statement.split('=').next().unwrap_or("").contains(',') {
                continue;
            }
            if !schema
                .vars
                .resolve_recursive(&name.parse()?, None)?
                .as_ref()
                .is_some_and(|value| exact_value(value, &winner.value))
            {
                continue;
            }
            let commented = statement
                .split_inclusive('\n')
                .map(|line| format!("# {line}"))
                .collect::<String>();
            edits.push((location.bytes.clone(), commented));
        }
        let edited = apply_edits(&self.source, edits)?;
        let candidate = self.preview(&edited)?;
        anyhow::ensure!(
            exact_maps(&values(&candidate.vars)?, &values(&self.compiled.vars)?),
            "minimization would change referenced values; original source preserved"
        );
        Ok(edited)
    }
    /// Migrate explicit renamed settings. Original statements become comments;
    /// replacement assignments are appended at the root, so namespaces remain valid.
    /// Reject conflicting destinations or any unrelated effective-value changes.
    pub fn migrate(&self) -> anyhow::Result<String> {
        let schema = self
            .schema
            .as_ref()
            .ok_or_else(|| anyhow!("migrate requires --schema"))?;
        let user = compile_file(&self.input, None, None)?;
        let rules = schema.vars.constraints()?;
        let mut edits = Vec::new();
        let mut appended = String::new();
        let mut renamed = BTreeMap::new();
        for name in &self.provided {
            let Some(target) = rules.get(name).and_then(|r| r.renamed_to.as_ref()) else {
                continue;
            };
            anyhow::ensure!(
                names(&schema.vars).contains(target),
                "{name}: replacement {target} is absent from schema"
            );
            anyhow::ensure!(
                !self.provided.contains(target) && !renamed.values().any(|v| v == target),
                "{name}: destination {target} already has an override"
            );
            let Some(explanation) = user.vars.explain(name)? else {
                continue;
            };
            for definition in &explanation.history {
                let Some(location) = &definition.location else {
                    continue;
                };
                anyhow::ensure!(
                    location.file.as_ref() == Some(&self.input),
                    "{name} is defined in an include; migrate that file directly"
                );
                let statement = &self.source[location.bytes.clone()];
                anyhow::ensure!(
                    !statement.split('=').next().unwrap_or("").contains(','),
                    "split multi-assignments before migrating {name}"
                );
                anyhow::ensure!(
                    constraints::annotations_before(&self.source, location.bytes.start)
                        .iter()
                        .all(|a| schema_annotation(a)),
                    "{name} has local annotations; move its declaration before migrating"
                );
                edits.push((
                    location.bytes.clone(),
                    statement
                        .split_inclusive('\n')
                        .map(|line| format!("# {line}"))
                        .collect(),
                ));
            }
            appended.push_str(&format!("{target} = {}\n", explanation.value.to_source()?));
            renamed.insert(name.clone(), target.clone());
        }
        let mut edited = apply_edits(&self.source, edits)?;
        if !appended.is_empty() {
            if !edited.ends_with('\n') {
                edited.push('\n');
            }
            edited.push_str(&appended);
        }
        let candidate = self.preview(&edited)?;
        let old = values(&self.compiled.vars)?;
        let new = values(&candidate.vars)?;
        for (name, value) in &old {
            if let Some(target) = renamed.get(name) {
                anyhow::ensure!(
                    new.get(target).is_some_and(|new| exact_value(new, value)),
                    "migration changed {name}"
                );
            } else if !renamed.values().any(|target| target == name) {
                anyhow::ensure!(
                    new.get(name).is_some_and(|new| exact_value(new, value)),
                    "migration would change dependent setting {name}; original source preserved"
                );
            }
        }
        Ok(edited)
    }
}
// Construct include nodes directly so filesystem paths may contain spaces.
fn compile_layers(
    files: &[PathBuf],
    source: &str,
    parent: &Path,
    file: Option<PathBuf>,
) -> anyhow::Result<Evaluated> {
    let locations: crate::provenance::Locations = Default::default();
    let tokens = Token::lexer(source)
        .spanned()
        .map(|(token, span)| (token.unwrap_or(Token::Error), span.into()));
    let stream = Stream::from_iter(tokens)
        .map((0..source.len()).into(), |(token, span): (_, _)| {
            (token, span)
        });
    let parsed = parser_with_locations(source, Some(locations.clone()))
        .parse(stream)
        .into_result()
        .map_err(|errors| anyhow!("invalid source: {errors:?}"))?;
    let mut ast: Vec<_> = files
        .iter()
        .map(|path| {
            Ok(StatementKindOwnedPass1::Include {
                namespace: vec![],
                path: path
                    .to_str()
                    .ok_or_else(|| anyhow!("path is not UTF-8"))?
                    .into(),
            })
        })
        .collect::<anyhow::Result<_>>()?;
    ast.extend(parsed.into_iter().map(Into::into));
    let mut spans = std::iter::repeat_n(0..0, files.len())
        .chain(locations.borrow().values().cloned().collect::<Vec<_>>());
    let context = crate::provenance::Context {
        source,
        file,
        byte_offset: 0,
        line_offset: 0,
        included_from: vec![],
    };
    evaluate_parsed(do_pass1(ast, parent, &mut spans, &context)?, None, source)
}
fn exact_value(a: &Rhs, b: &Rhs) -> bool {
    match (a, b) {
        (
            Rhs::Val(Val::NumVal(NumVal::Floating(a))),
            Rhs::Val(Val::NumVal(NumVal::Floating(b))),
        ) => a == b,
        (Rhs::Val(Val::UnitedVal(a)), Rhs::Val(Val::UnitedVal(b))) => {
            a.unit() == b.unit() && a.si_value() == b.si_value()
        }
        (Rhs::Val(Val::NumVal(NumVal::Integer(a))), Rhs::Val(Val::NumVal(NumVal::Integer(b)))) => {
            a == b
        }
        (Rhs::Val(Val::StringVal(a)), Rhs::Val(Val::StringVal(b)))
        | (Rhs::Path(a), Rhs::Path(b)) => a == b,
        (Rhs::Val(Val::BoolVal(a)), Rhs::Val(Val::BoolVal(b))) => a == b,
        (Rhs::Array(a), Rhs::Array(b)) => {
            a.len() == b.len() && a.iter().zip(b).all(|(a, b)| exact_value(a, b))
        }
        (Rhs::Range { from: a, to: b }, Rhs::Range { from: c, to: d }) => {
            let bound = |a: &Option<Val>, b: &Option<Val>| match (a, b) {
                (None, None) => true,
                (Some(a), Some(b)) => exact_value(&Rhs::Val(a.clone()), &Rhs::Val(b.clone())),
                _ => false,
            };
            bound(a, c) && bound(b, d)
        }
        (Rhs::EmptyRange, Rhs::EmptyRange) => true,
        _ => false,
    }
}
fn exact_maps(a: &BTreeMap<String, Rhs>, b: &BTreeMap<String, Rhs>) -> bool {
    a.len() == b.len()
        && a.iter()
            .all(|(name, value)| b.get(name).is_some_and(|b| exact_value(value, b)))
}
fn schema_annotation(annotation: &str) -> bool {
    annotation.strip_prefix("schema").is_some_and(|rest| {
        rest.is_empty() || rest.starts_with(|c: char| c.is_whitespace() || c == '=')
    })
}
fn apply_edits(
    source: &str,
    mut edits: Vec<(std::ops::Range<usize>, String)>,
) -> anyhow::Result<String> {
    edits.sort_by_key(|(range, _)| range.start);
    edits.dedup_by(|a, b| a.0 == b.0);
    anyhow::ensure!(
        edits
            .windows(2)
            .all(|pair| pair[0].0.end <= pair[1].0.start),
        "overlapping edits"
    );
    let mut output = source.to_owned();
    for (range, text) in edits.into_iter().rev() {
        output.replace_range(range, &text);
    }
    Ok(output)
}
/// A defaults schema as a file for people to edit: the same settings, values
/// and comments, without the annotation lines only validation and editors read,
/// and without deprecated or renamed settings. `# @schema` directives stay.
///
/// Nothing is lost by leaving the annotations out: a setting keeps the rules of
/// its first declaration, so a file loaded against the schema is checked just
/// as strictly.
pub fn user_template(source: &str) -> anyhow::Result<String> {
    let lines: Vec<&str> = source.lines().collect();
    let mut drop = vec![false; lines.len()];
    let line_of = |byte: usize| source[..byte.min(source.len())].matches('\n').count();
    let is_comment = |line: &str| line.trim_start().starts_with('#');
    let is_annotation = |line: &str| {
        let trimmed = line.trim_start();
        trimmed.starts_with('#') && {
            let payload = trimmed[1..].trim_start();
            payload.starts_with('@') && !payload.starts_with("@schema")
        }
    };

    // A retired setting goes together with the comments that describe it.
    for assignment in source_assignments(source)? {
        let retired = assignment
            .annotations
            .iter()
            .any(|a| a.starts_with("deprecated") || a.starts_with("renamed_to"));
        if !retired {
            continue;
        }
        let first = line_of(assignment.statement.start);
        let last = line_of(assignment.statement.end.saturating_sub(1)).min(lines.len() - 1);
        drop[first..=last].fill(true);
        let mut above = first;
        while above > 0 && is_comment(lines[above - 1]) {
            above -= 1;
            drop[above] = true;
        }
    }
    for (index, line) in lines.iter().enumerate() {
        if is_annotation(line) {
            drop[index] = true;
        }
    }

    // A block left with nothing but comments goes too, with its own comments.
    loop {
        let kept: Vec<usize> = (0..lines.len()).filter(|&i| !drop[i]).collect();
        let empty = kept.iter().enumerate().find_map(|(position, &open)| {
            if !lines[open].trim_end().ends_with('{') {
                return None;
            }
            let close = kept[position + 1..]
                .iter()
                .find(|&&i| !lines[i].trim().is_empty() && !is_comment(lines[i]))?;
            lines[*close]
                .trim_start()
                .starts_with('}')
                .then_some((open, *close))
        });
        let Some((open, close)) = empty else {
            break;
        };
        drop[open..=close].fill(true);
        let mut above = open;
        while above > 0 && is_comment(lines[above - 1]) {
            above -= 1;
            drop[above] = true;
        }
    }

    let mut out = String::new();
    let mut blank = true;
    for (index, line) in lines.iter().enumerate() {
        if drop[index] {
            continue;
        }
        let is_blank = line.trim().is_empty();
        if is_blank && blank {
            continue;
        }
        blank = is_blank;
        out.push_str(line);
        out.push('\n');
    }
    while out.ends_with("\n\n") {
        out.pop();
    }
    Ok(out)
}

/// Which settings a generated configuration holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Selection<'a> {
    /// Every current setting, as [`user_template`] writes it.
    All,
    /// Only the settings carrying this annotation, such as `recommended`.
    Annotated(&'a str),
}

/// A configuration for people to edit, generated from a defaults schema.
///
/// With `schema`, the file starts with a `# @schema` directive pointing at
/// it, typically the defaults file of the release that generated it, see
/// [`release_schema_url`]. Editors then check and
/// complete the file against the rules the generated text leaves out.
pub fn starter(
    defaults: &str,
    selection: Selection,
    schema: Option<&str>,
) -> anyhow::Result<String> {
    let body = match selection {
        Selection::All => user_template(defaults)?,
        Selection::Annotated(annotation) => {
            let mut compiled = compile_code(defaults)?;
            compiled.vars = compiled.vars.with_drop_warning(false);
            compiled
                .to_string_filtered(annotation)
                .ok_or_else(|| anyhow!("no setting carries @{annotation}"))?
        }
    };
    Ok(match schema {
        Some(schema) => {
            anyhow::ensure!(
                !schema.trim().is_empty() && !schema.contains(['\n', '\r']),
                "a schema reference must be a single non-empty line"
            );
            format!("# @schema {schema}\n\n{body}")
        }
        None => body,
    })
}

/// Generate selected defaults as explicit overrides. Required values are always included.
pub fn generate(defaults: &Evaluated, full: bool) -> anyhow::Result<String> {
    let rules = defaults.vars.constraints()?;
    let mut result = String::new();
    for name in names(&defaults.vars) {
        if name.split('.').any(|p| p.starts_with('_')) {
            continue;
        }
        let rule = rules.get(&name);
        if rule.is_some_and(|r| r.deprecated.is_some() || r.renamed_to.is_some()) {
            continue;
        }
        let recommended = defaults.vars.ast.iter().any(|statement| {
            let StatementKindOwned::VariableDef(Statement::AssignLeft {
                lhs, annotations, ..
            }) = statement;
            lhs.iter().any(|v| format_var(v) == name)
                && annotations.iter().any(|a| a == "recommended")
        });
        let inherited_recommended = defaults
            .vars
            .explain(&name)?
            .and_then(|e| e.winner.location)
            .is_some_and(|location| {
                let source = location
                    .file
                    .as_ref()
                    .and_then(|p| std::fs::read_to_string(p).ok())
                    .unwrap_or_else(|| defaults.source_text.clone());
                parse_annotations(&source)
                    .get("recommended")
                    .is_some_and(|ranges| {
                        ranges.iter().any(|(start, end)| {
                            source
                                .lines()
                                .nth(*start)
                                .is_some_and(|line| line.trim().ends_with('{'))
                                && *start < location.line
                                && location.line - 1 <= *end
                        })
                    })
            });
        if !full
            && !recommended
            && !inherited_recommended
            && !rule.is_some_and(|r| r.required == Some(true))
        {
            continue;
        }
        if let Some(explanation) = defaults.vars.explain(&name)? {
            if let Some(location) = &explanation.winner.location {
                if let Some(path) = &location.file {
                    let source = std::fs::read_to_string(path)?;
                    let mut comments: Vec<_> = source[..location.bytes.start]
                        .lines()
                        .rev()
                        .take_while(|l| l.trim().is_empty() || l.trim().starts_with('#'))
                        // Rules stay in the schema; the copy keeps the prose.
                        .filter(|l| {
                            let trimmed = l.trim();
                            trimmed.starts_with('#') && !trimmed[1..].trim_start().starts_with('@')
                        })
                        .collect();
                    comments.reverse();
                    for line in comments {
                        result.push_str(line.trim_start());
                        result.push('\n');
                    }
                }
            }
            result.push_str(&format!("{name} = {}\n\n", explanation.value.to_source()?));
        }
    }
    Ok(result)
}
/// Format indentation and assignment spacing while retaining comments, includes,
/// references, literal spelling, and line endings. Multiline strings stay untouched.
pub fn format_document(source: &str) -> anyhow::Result<String> {
    anyhow::ensure!(
        syntax_diagnostics(source).is_empty(),
        "cannot format invalid Ratslang syntax"
    );
    let tokens: Vec<_> = Token::lexer(source).spanned().collect();
    let mut output = String::new();
    let mut offset = 0;
    let mut depth = 0usize;
    for line in source.split_inclusive('\n') {
        let end = offset + line.len();
        let multiline_string = tokens.iter().any(|(t, span)| {
            matches!(t, Ok(Token::String(_)))
                && span.start < end
                && span.end > offset
                && source[span.clone()].contains('\n')
        });
        if multiline_string {
            // Structural tokens beside a multiline string still affect following lines.
            for (token, span) in &tokens {
                if span.start >= offset && span.start < end {
                    match token {
                        Ok(Token::BlockStart | Token::LBracket) => depth += 1,
                        Ok(Token::BlockEnd | Token::RBracket) => depth = depth.saturating_sub(1),
                        _ => {}
                    }
                }
            }
            output.push_str(line);
            offset = end;
            continue;
        }
        let line_tokens: Vec<_> = tokens
            .iter()
            .filter(|(_, span)| span.start >= offset && span.start < end)
            .collect();
        let closes = line_tokens
            .iter()
            .take_while(|(t, _)| matches!(t, Ok(Token::BlockEnd | Token::RBracket)))
            .count();
        let mut formatted = line.trim_start_matches([' ', '\t']).to_owned();
        for (token, span) in line_tokens.iter().rev() {
            if matches!(token, Ok(Token::OpAssignToLeft)) {
                let indent_removed = line.len() - line.trim_start_matches([' ', '\t']).len();
                let start = span.start - offset - indent_removed;
                let left = formatted[..start].trim_end_matches([' ', '\t']);
                let right = formatted[start + 1..].trim_start_matches([' ', '\t']);
                formatted = if left.is_empty() {
                    format!("= {right}")
                } else {
                    format!("{left} = {right}")
                };
            }
        }
        if !formatted.trim().is_empty() {
            output.push_str(&"  ".repeat(depth.saturating_sub(closes)));
        }
        output.push_str(&formatted);
        for (token, _) in line_tokens {
            match token {
                Ok(Token::BlockStart | Token::LBracket) => depth += 1,
                Ok(Token::BlockEnd | Token::RBracket) => depth = depth.saturating_sub(1),
                _ => {}
            }
        }
        offset = end;
    }
    anyhow::ensure!(
        syntax_diagnostics(&output).is_empty(),
        "formatting produced invalid syntax"
    );
    Ok(output)
}

#[cfg(test)]
#[path = "tooling_tests.rs"]
mod tests;

/// Syntax-only assignment spans for editor tooling. Includes are not read.
/// This uses the compiler grammar, so new quantity units do not depend on an
/// editor's tree-sitter grammar having been updated yet.
#[derive(Debug, Clone)]
pub struct SourceAssignment {
    pub name: String,
    pub namespace: String,
    pub selection: std::ops::Range<usize>,
    pub value: std::ops::Range<usize>,
    pub statement: std::ops::Range<usize>,
    pub annotations: Vec<String>,
}
pub fn source_assignments(source: &str) -> anyhow::Result<Vec<SourceAssignment>> {
    let padded = format!("{source}\n");
    let locations: crate::provenance::Locations = Default::default();
    let tokens = Token::lexer(&padded)
        .spanned()
        .map(|(t, span)| (t.unwrap_or(Token::Error), span.into()));
    let stream =
        Stream::from_iter(tokens).map((0..padded.len()).into(), |(t, span): (_, _)| (t, span));
    let ast = parser_with_locations(&padded, Some(locations.clone()))
        .parse(stream)
        .into_result()
        .map_err(|_| anyhow!("incomplete or invalid syntax"))?;
    let context = crate::provenance::Context {
        source: &padded,
        file: None,
        byte_offset: 0,
        line_offset: 0,
        included_from: vec![],
    };
    let mut ranges = locations
        .borrow()
        .values()
        .cloned()
        .collect::<Vec<_>>()
        .into_iter();
    let mut out = Vec::new();
    source_assignments_walk(ast, "", &padded, &context, &mut ranges, &mut out)?;
    Ok(out)
}
fn source_assignments_walk(
    ast: Vec<StatementKindPass1>,
    namespace: &str,
    source: &str,
    context: &crate::provenance::Context<'_>,
    ranges: &mut impl Iterator<Item = std::ops::Range<usize>>,
    out: &mut Vec<SourceAssignment>,
) -> anyhow::Result<()> {
    for statement in ast {
        match statement {
            StatementKindPass1::VariableNamespaceBlock { ns, stmts } => {
                let namespace = if namespace.is_empty() {
                    ns.join(".")
                } else {
                    format!("{namespace}.{}", ns.join("."))
                };
                source_assignments_walk(
                    stmts.into_iter().map(|s| *s).collect(),
                    &namespace,
                    source,
                    context,
                    ranges,
                    out,
                )?;
            }
            StatementKindPass1::Include { .. } => {
                ranges
                    .next()
                    .ok_or_else(|| anyhow!("missing include span"))?;
            }
            StatementKindPass1::VariableDef(Statement::AssignLeft {
                lhs, annotations, ..
            }) => {
                let statement = context
                    .location(
                        ranges
                            .next()
                            .ok_or_else(|| anyhow!("missing assignment span"))?,
                    )
                    .bytes;
                let mut keys = Vec::new();
                let mut values = Vec::new();
                let mut group = None;
                let mut equal = false;
                let mut depth = 0usize;
                for (token, span) in Token::lexer(&source[statement.clone()]).spanned() {
                    let range = span.start + statement.start..span.end + statement.start;
                    if matches!(token, Ok(Token::NewLine)) {
                        continue;
                    }
                    if matches!(token, Ok(Token::OpAssignToLeft)) && !equal {
                        if let Some(group) = group.take() {
                            keys.push(group);
                        }
                        equal = true;
                        continue;
                    }
                    if matches!(token, Ok(Token::Comma)) && depth == 0 {
                        if let Some(group) = group.take() {
                            if equal {
                                values.push(group);
                            } else {
                                keys.push(group);
                            }
                        }
                        continue;
                    }
                    let group: &mut std::ops::Range<usize> = group.get_or_insert(range.clone());
                    group.end = range.end;
                    match token {
                        Ok(Token::LBracket) => depth += 1,
                        Ok(Token::RBracket) => depth = depth.saturating_sub(1),
                        _ => {}
                    }
                }
                if let Some(group) = group {
                    values.push(group);
                }
                anyhow::ensure!(
                    keys.len() == lhs.len() && !values.is_empty(),
                    "invalid assignment spans"
                );
                for (index, var) in lhs.into_iter().enumerate() {
                    out.push(SourceAssignment {
                        name: format_var(&var),
                        namespace: namespace.into(),
                        selection: keys[index].clone(),
                        value: values[if values.len() == 1 { 0 } else { index }].clone(),
                        statement: statement.clone(),
                        annotations: annotations.clone(),
                    });
                }
            }
        }
    }
    Ok(())
}
