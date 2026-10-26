use crate::DocumentState;
use tower_lsp_server::ls_types::*;

pub fn handle_document_symbols(doc: &DocumentState) -> Vec<DocumentSymbol> {
    #[allow(deprecated)]
    fn symbol(
        name: String,
        kind: SymbolKind,
        range: Range,
        selection_range: Range,
        children: Vec<DocumentSymbol>,
    ) -> DocumentSymbol {
        DocumentSymbol {
            name,
            kind,
            range,
            selection_range,
            children: (!children.is_empty()).then_some(children),
            detail: None,
            tags: None,
            deprecated: None,
        }
    }
    fn children(doc: &DocumentState, namespace: &str) -> Vec<DocumentSymbol> {
        let mut symbols = Vec::new();
        let prefix = if namespace.is_empty() {
            String::new()
        } else {
            format!("{namespace}.")
        };
        for (name, ns) in &doc.symbols.namespaces {
            if let Some(local) = name.strip_prefix(&prefix)
                && ns.parent == namespace
            {
                symbols.push(symbol(
                    local.into(),
                    SymbolKind::NAMESPACE,
                    ns.range,
                    ns.selection_range,
                    children(doc, name),
                ));
            }
        }
        for assignment in &doc.symbols.assignments {
            if assignment.namespace == namespace {
                symbols.push(symbol(
                    assignment.name.clone(),
                    if assignment.is_internal {
                        SymbolKind::CONSTANT
                    } else {
                        SymbolKind::PROPERTY
                    },
                    assignment.range,
                    assignment.selection_range,
                    Vec::new(),
                ));
            }
        }
        symbols.sort_by_key(|s| s.range.start);
        symbols
    }
    children(doc, "")
}
