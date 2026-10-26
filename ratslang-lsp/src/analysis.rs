use crate::{DocumentState, Schema, text};
use std::path::Path;
use tower_lsp_server::ls_types::*;

fn diagnostic(range: Range, message: String) -> Diagnostic {
    Diagnostic {
        range,
        severity: Some(DiagnosticSeverity::ERROR),
        message,
        source: Some("ratslang".into()),
        ..Default::default()
    }
}
pub fn analyze(doc: &DocumentState, schema: Option<&Schema>) -> Vec<Diagnostic> {
    let syntax = ratslang::syntax_diagnostics(&doc.content);
    if !syntax.is_empty() {
        return syntax
            .into_iter()
            .map(|e| diagnostic(text::range(&doc.content, e.range), e.message))
            .collect();
    }
    let path = doc.uri.to_file_path();
    let parent = path
        .as_deref()
        .and_then(Path::parent)
        .unwrap_or(Path::new("."));
    let local = match Schema::load(&doc.content, parent, Some(&doc.uri)) {
        Ok(local) => local,
        Err(message) => {
            let range = doc
                .symbols
                .assignments
                .iter()
                .rev()
                .find(|a| message.starts_with(&format!("{}:", a.full_path)))
                .map(|a| a.value_range)
                .unwrap_or_else(|| text::range(&doc.content, 0..doc.content.len()));
            return vec![diagnostic(range, message)];
        }
    };
    let mut diagnostics = Vec::new();
    for (name, property) in &local.properties {
        if let Some(value @ ratslang::Rhs::Range { .. }) = &property.value {
            let rules = ratslang::Constraints {
                value_type: crate::schema::inferred_type(value),
                ..Default::default()
            };
            if let Err(error) = rules.validate(name, value) {
                let range = doc
                    .symbols
                    .assignments
                    .iter()
                    .rev()
                    .find(|a| a.full_path == *name)
                    .map(|a| a.value_range)
                    .unwrap_or_default();
                diagnostics.push(diagnostic(range, error.to_string()));
            }
        }
    }
    if let Some(schema) = schema {
        for (name, property) in &schema.properties {
            if property.constraints.required == Some(true) && !local.properties.contains_key(name) {
                let mut error = diagnostic(
                    doc.schema_range.unwrap_or_default(),
                    format!("Required setting {name} must be explicitly assigned"),
                );
                error.code = Some(NumberOrString::String("required-setting".into()));
                diagnostics.push(error);
            }
        }
        for (name, property) in &local.properties {
            if name.starts_with('_') {
                continue;
            }
            let range = doc
                .symbols
                .assignments
                .iter()
                .rev()
                .find(|a| a.full_path == *name)
                .map(|a| a.value_range)
                .or_else(|| doc.symbols.includes.first().map(|i| i.range))
                .unwrap_or_default();
            if let Some(expected) = schema.get_property(name) {
                if expected.constraints.deprecated.is_some()
                    || expected.constraints.renamed_to.is_some()
                {
                    let mut warning = diagnostic(
                        range,
                        format!(
                            "{name}: {}{}",
                            expected
                                .constraints
                                .deprecated
                                .as_deref()
                                .unwrap_or("This setting was renamed"),
                            expected
                                .constraints
                                .renamed_to
                                .as_ref()
                                .map(|target| format!("; use {target}"))
                                .unwrap_or_default()
                        ),
                    );
                    warning.severity = Some(DiagnosticSeverity::WARNING);
                    warning.tags = Some(vec![DiagnosticTag::DEPRECATED]);
                    warning.code = Some(NumberOrString::String("deprecated-setting".into()));
                    diagnostics.push(warning);
                }

                let mut rules = expected.constraints.overlay(&property.constraints);
                if rules.value_type.is_none() {
                    rules.value_type = expected
                        .value
                        .as_ref()
                        .and_then(crate::schema::inferred_type);
                }
                if let Some(value) = &property.value
                    && let Err(error) = rules.validate(name, value)
                {
                    diagnostics.push(diagnostic(range, error.to_string()));
                }
            } else {
                let assignment = doc
                    .symbols
                    .assignments
                    .iter()
                    .rev()
                    .find(|a| a.full_path == *name);
                let suggestion = schema.suggest_setting(name);
                let message = match suggestion {
                    Some(suggested) => {
                        format!("Unknown setting: {name}. Did you mean {suggested}?")
                    }
                    None => format!("Unknown setting: {name}"),
                };
                let mut item = diagnostic(
                    assignment.map(|a| a.selection_range).unwrap_or(range),
                    message,
                );
                item.severity = Some(DiagnosticSeverity::WARNING);
                item.code = Some(NumberOrString::String("unknown-setting".into()));
                diagnostics.push(item);
            }
        }
    }
    diagnostics
}
