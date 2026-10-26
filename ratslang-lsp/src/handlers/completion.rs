use crate::{DocumentState, Schema, schema::SchemaProperty, text};
use std::collections::BTreeMap;
use tower_lsp_server::ls_types::*;

pub fn handle_completion(
    doc: &DocumentState,
    at: Position,
    schema: Option<&Schema>,
) -> Vec<CompletionItem> {
    let Some(byte) = text::offset(&doc.content, at) else {
        return Vec::new();
    };
    let start = doc.content[..byte].rfind('\n').map(|n| n + 1).unwrap_or(0);
    let before = &doc.content[start..byte];
    if before.trim_start().starts_with('#') {
        return Vec::new();
    }
    let namespace = doc.namespace_at(byte);
    let mut items = BTreeMap::new();
    if let Some((lhs, rhs)) = before.split_once('=') {
        let lhs = lhs.trim();
        let path = if namespace.is_empty() {
            lhs.into()
        } else {
            format!("{namespace}.{lhs}")
        };
        let local = Schema::local_property(doc, &path);
        let prop = schema
            .and_then(|s| s.get_property(&path))
            .or(local.as_ref());
        let inline = Schema::incomplete_annotations(doc, byte);
        let rules = prop
            .map(|p| p.constraints.overlay(&inline))
            .unwrap_or(inline);
        if let Some(options) = &rules.allowed {
            for option in options {
                let insert = if rhs.trim_start().starts_with('"')
                    || option.chars().any(char::is_whitespace)
                {
                    format!("\"{option}\"")
                } else {
                    option.clone()
                };
                items.insert(
                    option.clone(),
                    value_item(
                        doc,
                        at,
                        rhs,
                        option,
                        &insert,
                        CompletionItemKind::ENUM_MEMBER,
                    ),
                );
            }
        } else if prop.and_then(|p| p.value_type.as_deref()) == Some("boolean")
            || rules.value_type == Some(ratslang::ValueType::Boolean)
        {
            for value in ["true", "false"] {
                items.insert(
                    value.into(),
                    value_item(doc, at, rhs, value, value, CompletionItemKind::VALUE),
                );
            }
        }
        if let Some(prop) = prop
            && let Some(default) = &prop.default
        {
            items.entry(default.clone()).or_insert_with(|| {
                value_item(doc, at, rhs, default, default, CompletionItemKind::VALUE)
            });
        }
        if items.is_empty() {
            for assignment in &doc.symbols.assignments {
                if assignment.full_path != path {
                    items.insert(
                        assignment.full_path.clone(),
                        value_item(
                            doc,
                            at,
                            rhs,
                            &assignment.full_path,
                            &assignment.full_path,
                            CompletionItemKind::REFERENCE,
                        ),
                    );
                }
            }
        }
        return items.into_values().collect();
    }
    let word = &doc.content[text::word(&doc.content, byte).start..byte];
    let (prefix, partial) = word.rsplit_once('.').unwrap_or(("", word));
    let path = if namespace.is_empty() {
        prefix.to_owned()
    } else if prefix.is_empty() {
        namespace.to_owned()
    } else {
        format!("{namespace}.{prefix}")
    };
    if let Some(schema) = schema {
        let (props, namespaces) = schema.get_children_at_path(&path);
        for name in props {
            let full = if path.is_empty() {
                name.clone()
            } else {
                format!("{path}.{name}")
            };
            if let Some(prop) = schema.get_property(&full) {
                let mut item = property_item(&name, prop);
                item.text_edit = Some(CompletionTextEdit::Edit(TextEdit {
                    range: Range::new(
                        Position::new(
                            at.line,
                            at.character
                                .saturating_sub(partial.encode_utf16().count() as u32),
                        ),
                        at,
                    ),
                    new_text: name.clone(),
                }));
                items.insert(name, item);
            }
        }
        for name in namespaces {
            items.entry(name.clone()).or_insert(CompletionItem {
                label: name,
                kind: Some(CompletionItemKind::MODULE),
                ..Default::default()
            });
        }
    }
    let prefix = if path.is_empty() {
        String::new()
    } else {
        format!("{path}.")
    };
    for full in doc
        .symbols
        .assignments
        .iter()
        .filter(|a| !a.is_internal)
        .map(|a| &a.full_path)
        .chain(doc.symbols.namespaces.keys())
    {
        if let Some(rest) = full.strip_prefix(&prefix) {
            let name = rest.split('.').next().unwrap_or("");
            if !name.is_empty() {
                items.entry(name.into()).or_insert(CompletionItem {
                    label: name.into(),
                    kind: Some(if rest.contains('.') {
                        CompletionItemKind::MODULE
                    } else {
                        CompletionItemKind::PROPERTY
                    }),
                    ..Default::default()
                });
            }
        }
    }
    items.into_values().collect()
}
fn property_item(name: &str, prop: &SchemaProperty) -> CompletionItem {
    CompletionItem {
        label: name.into(),
        kind: Some(CompletionItemKind::PROPERTY),
        detail: prop.value_type.clone(),
        tags: (prop.constraints.deprecated.is_some() || prop.constraints.renamed_to.is_some())
            .then(|| vec![CompletionItemTag::DEPRECATED]),
        documentation: Some(Documentation::MarkupContent(MarkupContent {
            kind: MarkupKind::Markdown,
            value: prop.documentation(),
        })),
        ..Default::default()
    }
}
fn value_item(
    doc: &DocumentState,
    at: Position,
    rhs: &str,
    label: &str,
    insert: &str,
    kind: CompletionItemKind,
) -> CompletionItem {
    let typed = rhs.trim_start();
    CompletionItem {
        label: label.into(),
        kind: Some(kind),
        text_edit: Some(CompletionTextEdit::Edit(TextEdit {
            range: doc
                .assignment_at(at)
                .filter(|a| a.value_range.start <= at && at <= a.value_range.end)
                .map(|a| a.value_range)
                .unwrap_or_else(|| {
                    Range::new(
                        Position::new(
                            at.line,
                            at.character
                                .saturating_sub(typed.encode_utf16().count() as u32),
                        ),
                        at,
                    )
                }),
            new_text: insert.into(),
        })),
        ..Default::default()
    }
}
