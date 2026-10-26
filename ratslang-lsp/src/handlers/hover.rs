use crate::{DocumentState, Schema};
use tower_lsp_server::ls_types::*;

pub fn handle_hover(doc: &DocumentState, at: Position, schema: Option<&Schema>) -> Option<Hover> {
    let path = doc.path_at(at)?;
    let file = doc.uri.to_file_path();
    let parent = file
        .as_deref()
        .and_then(std::path::Path::parent)
        .unwrap_or(std::path::Path::new("."));
    let local = Schema::load(&doc.content, parent, Some(&doc.uri))
        .ok()
        .and_then(|s| s.get_property(&path).cloned())
        .or_else(|| Schema::local_property(doc, &path));
    let property = schema
        .and_then(|s| s.get_property(&path))
        .or(local.as_ref())?;
    let mut value = property.documentation();
    if let Some(assignment) = doc
        .symbols
        .assignments
        .iter()
        .rev()
        .find(|a| a.full_path == path)
    {
        if schema.and_then(|s| s.get_property(&path)).is_some() {
            value.push_str(&format!(
                "\n\n**Override assignment:** `{}`\n\n**Override source:** [{}:{}]({})",
                assignment.expression,
                doc.uri.as_str(),
                assignment.selection_range.start.line + 1,
                doc.uri.as_str()
            ));
        } else {
            // Without a schema, the local assignment is a value rather than a default.
            value = value
                .replace("**Default:**", "**Assignment:**")
                .replace("**Default source:**", "**Source:**");
        }
    }
    if let Some(local) = &local
        && let Some(provenance) = &local.provenance
    {
        if let Ok(current) = provenance.value.to_source() {
            value.push_str(&format!("\n\n**Effective value:** `{current}`"));
        }
        if !provenance.dependencies.is_empty() {
            value.push_str("\n\n**Referenced definitions at assignment time:**");
            for definition in &provenance.dependencies {
                let source = definition
                    .location
                    .as_ref()
                    .map(|location| {
                        let uri = location
                            .file
                            .as_ref()
                            .and_then(Uri::from_file_path)
                            .unwrap_or_else(|| doc.uri.clone());
                        format!("{}:{}", uri.as_str(), location.line)
                    })
                    .unwrap_or_else(|| "initial state".into());
                if let Ok(expression) = definition.value.to_source() {
                    value.push_str(&format!(
                        "\n- `{}` = `{expression}` ({source})",
                        definition.name
                    ));
                }
            }
        }
        if provenance.history.len() > 1 {
            value.push_str("\n\n**Assignment history:**");
            for definition in &provenance.history {
                if let Ok(expression) = definition.expression.to_source() {
                    let line = definition.location.as_ref().map(|l| l.line).unwrap_or(0);
                    value.push_str(&format!("\n- Line {line}: `{expression}`"));
                }
            }
        }
    }
    if value.is_empty() {
        return None;
    }
    Some(Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value,
        }),
        range: doc.assignment_at(at).map(|a| a.selection_range),
    })
}
