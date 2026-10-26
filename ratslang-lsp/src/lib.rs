mod analysis;
mod document;
mod handlers;
mod schema;
mod text;

use dashmap::DashMap;
pub use document::{AssignmentInfo, DocumentState, NamespaceInfo, SymbolTable};
pub use schema::{Schema, SchemaProperty};
use std::{
    collections::HashSet,
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};
use tower_lsp_server::{LanguageServer, LspService, Server, jsonrpc::Result, ls_types::*};

pub struct Backend {
    db: DashMap<Uri, DocumentState>,
    schema_cache: DashMap<Uri, Arc<Schema>>,
    generation: AtomicU64,
}
impl Default for Backend {
    fn default() -> Self {
        Self::new()
    }
}
impl Backend {
    pub fn new() -> Self {
        Self {
            db: DashMap::new(),
            schema_cache: DashMap::new(),
            generation: AtomicU64::new(0),
        }
    }
    pub fn get_document(&self, uri: &Uri) -> Option<DocumentState> {
        self.db.get(uri).map(|v| v.clone())
    }
    pub fn update_document(&self, uri: Uri, doc: DocumentState) {
        self.db.insert(uri, doc);
    }
    pub fn remove_document(&self, uri: &Uri) {
        self.db.remove(uri);
    }
}
pub struct RatslangBackend {
    backend: Backend,
    client: tower_lsp_server::Client,
    http: reqwest::Client,
}
impl RatslangBackend {
    pub fn new(client: tower_lsp_server::Client) -> Self {
        Self {
            backend: Backend::new(),
            client,
            http: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(5))
                .build()
                .expect("HTTP client"),
        }
    }
    fn schema_uri(doc: &DocumentState) -> Option<Uri> {
        let reference = doc.schema_ref.as_ref()?;
        if reference.is_empty() {
            return None;
        }
        let base = reqwest::Url::parse(doc.uri.as_str()).ok()?;
        base.join(reference).ok()?.as_str().parse().ok()
    }
    async fn load_schema(&self, uri: &Uri) -> std::result::Result<Arc<Schema>, String> {
        let local = uri.as_str().starts_with("file:");
        let generation = self.backend.generation.load(Ordering::SeqCst);
        if let Some(schema) = self.backend.schema_cache.get(uri) {
            return Ok(schema.clone());
        }
        let content = if let Some(doc) = self.backend.get_document(uri) {
            doc.content
        } else if local {
            let path = uri.to_file_path().ok_or("Invalid file schema URI")?;
            tokio::fs::read_to_string(path)
                .await
                .map_err(|e| e.to_string())?
        } else if uri.as_str().starts_with("https://") || uri.as_str().starts_with("http://") {
            self.http
                .get(uri.as_str())
                .send()
                .await
                .map_err(|e| e.to_string())?
                .error_for_status()
                .map_err(|e| e.to_string())?
                .text()
                .await
                .map_err(|e| e.to_string())?
        } else {
            return Err(format!("Unsupported schema URI: {}", uri.as_str()));
        };
        if !local
            && !DocumentState::new(uri.clone(), content.clone(), 0)
                .symbols
                .includes
                .is_empty()
        {
            return Err(
                "Remote schemas must be self-contained; remote includes are not supported".into(),
            );
        }
        let path = local.then(|| uri.to_file_path()).flatten();
        let parent = path
            .as_deref()
            .and_then(Path::parent)
            .unwrap_or(Path::new("."));
        let schema = Arc::new(Schema::load(&content, parent, Some(uri))?);
        if self.backend.generation.load(Ordering::SeqCst) == generation {
            self.backend
                .schema_cache
                .insert(uri.clone(), schema.clone());
        }
        Ok(schema)
    }
    fn schema_for(&self, doc: &DocumentState) -> Option<Arc<Schema>> {
        Self::schema_uri(doc).and_then(|uri| self.backend.schema_cache.get(&uri).map(|s| s.clone()))
    }
    fn invalidate(&self) {
        self.backend.generation.fetch_add(1, Ordering::SeqCst);
        // An edit invalidates local files, not immutable remote defaults on
        // every keystroke. Remote schemas refresh when the server restarts.
        self.backend
            .schema_cache
            .retain(|uri, _| !uri.as_str().starts_with("file:"));
    }
    async fn refresh_documents(&self) {
        let generation = self.backend.generation.load(Ordering::SeqCst);
        let docs: Vec<_> = self
            .backend
            .db
            .iter()
            .map(|entry| entry.value().clone())
            .collect();
        for doc in docs {
            let mut diagnostics = Vec::new();
            let schema = if let Some(uri) = Self::schema_uri(&doc) {
                match self.load_schema(&uri).await {
                    Ok(schema) => Some(schema),
                    Err(error) => {
                        let range = doc.schema_range.unwrap_or_default();
                        diagnostics.push(Diagnostic {
                            range,
                            severity: Some(DiagnosticSeverity::ERROR),
                            message: format!("Cannot load schema {}: {error}", uri.as_str()),
                            source: Some("ratslang".into()),
                            ..Default::default()
                        });
                        None
                    }
                }
            } else {
                if doc.schema_ref.is_some() {
                    diagnostics.push(Diagnostic {
                        range: doc.schema_range.unwrap_or_default(),
                        severity: Some(DiagnosticSeverity::ERROR),
                        message: "Schema directive requires a valid local path or HTTP(S) URL"
                            .into(),
                        source: Some("ratslang".into()),
                        ..Default::default()
                    });
                }
                None
            };
            diagnostics.extend(analysis::analyze(&doc, schema.as_deref()));
            if self.backend.generation.load(Ordering::SeqCst) == generation
                && self
                    .backend
                    .get_document(&doc.uri)
                    .is_some_and(|current| current.version == doc.version)
            {
                self.client
                    .publish_diagnostics(doc.uri, diagnostics, Some(doc.version))
                    .await;
            }
        }
    }
    fn find_definition(
        &self,
        doc: &DocumentState,
        path: &str,
        visited: &mut HashSet<Uri>,
        depth: usize,
    ) -> Option<Location> {
        if depth > 64 || !visited.insert(doc.uri.clone()) {
            return None;
        }
        // Source order determines whether a local definition or include wins.
        let local = doc
            .symbols
            .assignments
            .iter()
            .rev()
            .find(|a| a.full_path == path);
        for include in doc.symbols.includes.iter().rev() {
            if local.is_some_and(|a| a.range.start > include.range.start) {
                break;
            }
            let relative = if include.namespace.is_empty() {
                path
            } else {
                match path.strip_prefix(&format!("{}.", include.namespace)) {
                    Some(path) => path,
                    None => continue,
                }
            };
            let Some(parent) = doc.uri.to_file_path() else {
                continue;
            };
            let Some(parent) = parent.parent() else {
                continue;
            };
            let Ok(file) = parent.join(&include.path).canonicalize() else {
                continue;
            };
            let Some(uri) = Uri::from_file_path(&file) else {
                continue;
            };
            let included = self.backend.get_document(&uri).or_else(|| {
                std::fs::read_to_string(file)
                    .ok()
                    .map(|source| DocumentState::new(uri, source, 0))
            });
            if let Some(included) = included
                && let Some(found) =
                    self.find_definition(&included, relative, &mut visited.clone(), depth + 1)
            {
                return Some(found);
            }
        }
        local.map(|a| Location::new(doc.uri.clone(), a.selection_range))
    }
}
impl LanguageServer for RatslangBackend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            server_info: Some(ServerInfo {
                name: "ratslang-lsp".into(),
                version: Some(env!("CARGO_PKG_VERSION").into()),
            }),
            capabilities: ServerCapabilities {
                position_encoding: Some(PositionEncodingKind::UTF16),
                text_document_sync: Some(TextDocumentSyncCapability::Options(
                    TextDocumentSyncOptions {
                        open_close: Some(true),
                        change: Some(TextDocumentSyncKind::INCREMENTAL),
                        save: Some(TextDocumentSyncSaveOptions::Supported(true)),
                        ..Default::default()
                    },
                )),
                completion_provider: Some(CompletionOptions {
                    trigger_characters: Some(vec![".".into(), "=".into(), " ".into()]),
                    ..Default::default()
                }),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                definition_provider: Some(OneOf::Left(true)),
                document_symbol_provider: Some(OneOf::Left(true)),
                code_action_provider: Some(CodeActionProviderCapability::Simple(true)),
                ..Default::default()
            },
            offset_encoding: None,
        })
    }
    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "ratslang-lsp initialized")
            .await;
    }
    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }
    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let doc = params.text_document;
        self.backend.update_document(
            doc.uri.clone(),
            DocumentState::new(doc.uri, doc.text, doc.version),
        );
        self.invalidate();
        self.refresh_documents().await;
    }
    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri;
        // Keep the map lock only while applying changes, never across an await.
        let changed = self.backend.db.get_mut(&uri).is_some_and(|mut doc| {
            doc.apply_changes(params.content_changes, params.text_document.version)
        });
        if changed {
            self.invalidate();
            self.refresh_documents().await;
        }
    }
    async fn did_save(&self, _: DidSaveTextDocumentParams) {
        self.invalidate();
        self.refresh_documents().await;
    }
    async fn did_change_watched_files(&self, _: DidChangeWatchedFilesParams) {
        self.invalidate();
        self.refresh_documents().await;
    }
    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let uri = params.text_document.uri;
        self.backend.remove_document(&uri);
        self.invalidate();
        self.client.publish_diagnostics(uri, Vec::new(), None).await;
        self.refresh_documents().await;
    }
    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let at = params.text_document_position;
        Ok(self.backend.get_document(&at.text_document.uri).map(|doc| {
            let schema = self.schema_for(&doc);
            CompletionResponse::Array(handlers::completion::handle_completion(
                &doc,
                at.position,
                schema.as_deref(),
            ))
        }))
    }
    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let at = params.text_document_position_params;
        Ok(self
            .backend
            .get_document(&at.text_document.uri)
            .and_then(|doc| {
                let schema = self.schema_for(&doc);
                handlers::hover::handle_hover(&doc, at.position, schema.as_deref())
            }))
    }
    async fn document_symbol(
        &self,
        params: DocumentSymbolParams,
    ) -> Result<Option<DocumentSymbolResponse>> {
        Ok(self
            .backend
            .get_document(&params.text_document.uri)
            .map(|doc| {
                DocumentSymbolResponse::Nested(handlers::symbols::handle_document_symbols(&doc))
            }))
    }
    async fn code_action(&self, params: CodeActionParams) -> Result<Option<CodeActionResponse>> {
        Ok(self
            .backend
            .get_document(&params.text_document.uri)
            .map(|doc| {
                let schema = self.schema_for(&doc);
                handlers::code_actions::handle_code_actions(&doc, params.range, schema.as_deref())
            }))
    }
    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        let at = params.text_document_position_params;
        let Some(doc) = self.backend.get_document(&at.text_document.uri) else {
            return Ok(None);
        };
        // Includes and the schema declaration themselves are navigable.
        if doc
            .schema_range
            .is_some_and(|range| range.start <= at.position && at.position <= range.end)
        {
            return Ok(Self::schema_uri(&doc)
                .map(|uri| GotoDefinitionResponse::Scalar(Location::new(uri, Range::default()))));
        }
        if let Some(include) = doc
            .symbols
            .includes
            .iter()
            .find(|i| i.range.start <= at.position && at.position <= i.range.end)
            && let Some(file) = doc
                .uri
                .to_file_path()
                .and_then(|p| p.parent().map(|parent| parent.join(&include.path)))
            && let Some(uri) = Uri::from_file_path(file)
        {
            return Ok(Some(GotoDefinitionResponse::Scalar(Location::new(
                uri,
                Range::default(),
            ))));
        }
        let Some(path) = doc.path_at(at.position) else {
            return Ok(None);
        };
        if path == "_schema" {
            return Ok(Self::schema_uri(&doc)
                .map(|uri| GotoDefinitionResponse::Scalar(Location::new(uri, Range::default()))));
        }
        // References bind to definitions preceding the assignment, including
        // an earlier definition when a variable refers to its previous value.
        let mut lookup = doc.clone();
        if let Some(assignment) = doc.assignment_at(at.position)
            && at.position > assignment.selection_range.end
        {
            lookup
                .symbols
                .assignments
                .retain(|a| a.range.start < assignment.range.start);
            lookup
                .symbols
                .includes
                .retain(|i| i.range.start < assignment.range.start);
        }
        let namespace = text::offset(&doc.content, at.position)
            .map(|byte| doc.namespace_at(byte))
            .unwrap_or("");
        let mut candidates = Vec::new();
        if !namespace.is_empty() && !path.starts_with(&format!("{namespace}.")) {
            candidates.push(format!("{namespace}.{path}"));
        }
        candidates.push(path);
        let schema = self.schema_for(&doc);
        let definition = candidates.iter().find_map(|path| {
            self.find_definition(&lookup, path, &mut HashSet::new(), 0)
                .or_else(|| {
                    schema.as_ref().and_then(|schema| {
                        schema.get_property(path).and_then(|p| p.location.clone())
                    })
                })
        });
        Ok(definition.map(GotoDefinitionResponse::Scalar))
    }
}
pub async fn run_server() {
    let (service, socket) = LspService::new(RatslangBackend::new);
    Server::new(tokio::io::stdin(), tokio::io::stdout(), socket)
        .serve(service)
        .await;
}
#[cfg(test)]
mod tests;
