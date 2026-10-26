use crate::{DocumentState, Schema, text};
use tower_lsp_server::ls_types::*;

pub fn handle_code_actions(
    doc: &DocumentState,
    range: Range,
    schema: Option<&Schema>,
) -> Vec<CodeActionOrCommand> {
    let Some(schema) = schema else {
        return Vec::new();
    };
    let mut corrections = Vec::new();
    for assignment in &doc.symbols.assignments {
        if assignment.is_internal
            || assignment.selection_range.end < range.start
            || assignment.selection_range.start > range.end
        {
            continue;
        }
        let renamed = schema
            .get_property(&assignment.full_path)
            .and_then(|p| p.constraints.renamed_to.as_deref());
        if renamed.is_some_and(|target| {
            doc.symbols
                .assignments
                .iter()
                .any(|a| a.full_path == target)
        }) {
            continue;
        }
        let Some(suggested) = renamed.or_else(|| schema.suggest_setting(&assignment.full_path))
        else {
            continue;
        };
        let replacement = if assignment.namespace.is_empty() {
            suggested
        } else {
            // An assignment inside a namespace cannot rename the surrounding block.
            let Some(relative) = suggested.strip_prefix(&format!("{}.", assignment.namespace))
            else {
                continue;
            };
            relative
        };
        corrections.push(CodeActionOrCommand::CodeAction(CodeAction {
            title: format!("Rename {} to {suggested}", assignment.full_path),
            kind: Some(CodeActionKind::QUICKFIX),
            is_preferred: Some(true),
            edit: Some(WorkspaceEdit {
                changes: Some(
                    [(
                        doc.uri.clone(),
                        vec![TextEdit {
                            range: assignment.selection_range,
                            new_text: replacement.into(),
                        }],
                    )]
                    .into(),
                ),
                ..Default::default()
            }),
            ..Default::default()
        }));
    }
    let missing: Vec<_> = schema
        .get_recommended_properties()
        .into_iter()
        .filter(|(name, _)| !doc.symbols.assignments.iter().any(|a| a.full_path == *name))
        .collect();
    let end = text::position(&doc.content, doc.content.len());
    let prefix = if doc.content.is_empty() || doc.content.ends_with('\n') {
        ""
    } else {
        "\n"
    };
    let action = |title: String, source: String| {
        CodeActionOrCommand::CodeAction(CodeAction {
            title,
            kind: Some(CodeActionKind::QUICKFIX),
            edit: Some(WorkspaceEdit {
                changes: Some(
                    [(
                        doc.uri.clone(),
                        vec![TextEdit {
                            range: Range::new(end, end),
                            new_text: format!("{prefix}{source}"),
                        }],
                    )]
                    .into(),
                ),
                ..Default::default()
            }),
            ..Default::default()
        })
    };
    let mut actions: Vec<_> = missing
        .iter()
        .map(|(name, default)| {
            action(
                format!("Insert default for {name}"),
                format!("{name} = {default}\n"),
            )
        })
        .collect();
    if !missing.is_empty() {
        actions.push(action(
            "Insert all recommended properties".into(),
            missing
                .into_iter()
                .map(|(name, value)| format!("{name} = {value}\n"))
                .collect(),
        ));
    }
    corrections.extend(actions);
    corrections
}
