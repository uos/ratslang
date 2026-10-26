use super::*;
use crate::{
    analysis::analyze,
    handlers::{
        code_actions::handle_code_actions, completion::handle_completion, hover::handle_hover,
    },
    text,
};
fn doc(source: &str) -> DocumentState {
    DocumentState::new("file:///tmp/editor.rl".parse().unwrap(), source.into(), 1)
}
fn change(range: Range, source: &str) -> TextDocumentContentChangeEvent {
    TextDocumentContentChangeEvent {
        range: Some(range),
        range_length: None,
        text: source.into(),
    }
}

#[test]
fn symbols_follow_wrappers_namespaces_multiple_assignments_and_schema() {
    let doc = doc(
        "_schema = \"./default.rl\"\nsensor {\n count, enabled = 3, true\n nested {\n distance = 2m\n }\n}\n",
    );
    let names: Vec<_> = doc
        .symbols
        .assignments
        .iter()
        .map(|a| a.full_path.as_str())
        .collect();
    assert_eq!(
        names,
        [
            "_schema",
            "sensor.count",
            "sensor.enabled",
            "sensor.nested.distance"
        ]
    );
    assert_eq!(doc.symbols.assignments[2].expression, "true");
    assert_eq!(doc.schema_ref.as_deref(), Some("./default.rl"));
    assert_eq!(doc.symbols.namespaces.len(), 2);
    assert_eq!(handlers::symbols::handle_document_symbols(&doc).len(), 2);
}
#[test]
fn positions_round_trip_utf16_crlf_newline_and_eof() {
    let source = "# 😀µ\r\ncount = 3\n";
    for (byte, _) in source
        .char_indices()
        .chain(std::iter::once((source.len(), '\0')))
    {
        if byte > 0 && source.as_bytes()[byte - 1] == b'\r' {
            continue;
        }
        let at = text::position(source, byte);
        assert_eq!(text::offset(source, at), Some(byte), "{byte}: {at:?}");
    }
    assert_eq!(text::position("a\nb", 2), Position::new(1, 0));
    assert_eq!(text::position("😀", 4), Position::new(0, 2));
    assert_eq!(text::offset("😀", Position::new(0, 1)), None);
}
#[test]
fn incremental_edits_apply_in_order_and_preserve_the_document() {
    let mut doc = doc("label = \"😀a\"\ncount = 1\n");
    assert!(doc.apply_changes(
        vec![
            change(Range::new(Position::new(0, 11), Position::new(0, 12)), "b"),
            change(Range::new(Position::new(1, 8), Position::new(1, 9)), "20"),
        ],
        2
    ));
    assert_eq!(doc.content, "label = \"😀b\"\ncount = 20\n");
    assert_eq!(doc.symbols.assignments.len(), 2);
    assert!(!doc.apply_changes(vec![change(Range::default(), "bad")], 1));
    assert!(!doc.apply_changes(
        vec![change(
            Range::new(Position::new(0, 10), Position::new(0, 10)),
            "bad"
        )],
        3
    ));
    assert_eq!(doc.version, 2);
}
#[test]
fn schema_uses_actual_defaults_comments_and_shared_constraints() {
    let schema = Schema::from_ratslang_source(
        "sensor {\n# Human explanation\n# @recommended\n# @type length\n# @min_exclusive 0m\ndistance = 50cm\n# @enum CPU, Backend\nbackend = CPU\ntranslation = [1m,\n2m, 3m]\n}\n",
    );
    let prop = schema.get_property("sensor.distance").unwrap();
    assert_eq!(prop.default.as_deref(), Some("50cm"));
    assert_eq!(prop.description.as_deref(), Some("Human explanation"));
    assert!(prop.constraints.minimum_exclusive);
    assert_eq!(
        schema
            .get_property("sensor.backend")
            .unwrap()
            .constraints
            .allowed
            .as_ref()
            .unwrap(),
        &vec!["CPU".to_string(), "Backend".into()]
    );
    assert!(
        schema
            .get_property("sensor.translation")
            .unwrap()
            .default
            .as_deref()
            .unwrap()
            .contains("3m")
    );
    assert_eq!(
        schema.get_recommended_properties(),
        [("sensor.distance".into(), "50cm".into())]
    );
    assert_eq!(
        schema.get_children_at_path("sensor").0,
        ["backend", "distance", "translation"]
    );
}
#[test]
fn legacy_schema_annotations_still_work() {
    let schema = Schema::from_ratslang_source(
        "# @type = enum[CPU,Backend]\n# @description = \"Engine\"\nbackend = CPU\n",
    );
    let prop = schema.get_property("backend").unwrap();
    assert_eq!(prop.description.as_deref(), Some("Engine"));
    assert_eq!(prop.constraints.allowed.as_ref().unwrap().len(), 2);
}
#[test]
fn semantic_diagnostics_use_compiler_and_schema_with_units() {
    let schema = Schema::from_ratslang_source(
        "# @min_exclusive 0m\ndistance = 1m\n# @enum CPU, Backend\nbackend = CPU\n",
    );
    assert!(analyze(&doc("distance = 1um\nbackend = Backend\n"), Some(&schema)).is_empty());
    let errors = analyze(
        &doc("distance = 0mm\nbackend = Other\ntypo = true\n"),
        Some(&schema),
    );
    assert_eq!(errors.len(), 3);
    assert!(
        errors
            .iter()
            .any(|e| e.message.contains("distance") && e.range.start.line == 0)
    );
    assert!(errors.iter().any(|e| e.message.contains("CPU")));
    assert!(
        errors
            .iter()
            .any(|e| e.severity == Some(DiagnosticSeverity::WARNING))
    );
    assert!(!analyze(&doc("range = 1m..2s\n"), None).is_empty());
    assert!(
        analyze(&doc("text = \"first\nsecond\"\n"), None).is_empty(),
        "compiler syntax wins over stale tree-sitter grammar"
    );
}
#[test]
fn compiler_syntax_errors_have_precise_positions() {
    let errors = analyze(&doc("# 😀\ncount = [\n"), None);
    assert!(!errors.is_empty());
    assert!(
        errors
            .iter()
            .all(|e| e.source.as_deref() == Some("ratslang"))
    );
    assert!(errors[0].range.start.line >= 1);
}
#[test]
fn completion_and_hover_resolve_nested_namespace_context() {
    let schema = Schema::from_ratslang_source(
        "sensor {\n# How far\n# @min 1mm\ndistance = 50cm\n# @enum CPU, Backend\nbackend = CPU\n}\n",
    );
    let document = doc("sensor {\n di\n}\n");
    let items = handle_completion(&document, Position::new(1, 3), Some(&schema));
    assert!(items.iter().any(|i| i.label == "distance"), "{items:?}");
    let document = doc("sensor {\n backend = \n}\n");
    let items = handle_completion(&document, Position::new(1, 11), Some(&schema));
    assert!(items.iter().any(|i| i.label == "CPU"), "{items:?}");
    let document = doc("sensor {\n distance = 3m\n}\n");
    let hover = handle_hover(&document, Position::new(1, 5), Some(&schema)).unwrap();
    let HoverContents::Markup(contents) = hover.contents else {
        panic!()
    };
    assert!(contents.value.contains("How far") && contents.value.contains("Minimum"));
}
#[test]
fn enum_completion_replaces_existing_quotes_without_duplication() {
    let schema = Schema::from_ratslang_source("# @enum CPU, Backend\nbackend = CPU\n");
    let document = doc("backend = \"CP\"\n");
    let item = handle_completion(&document, Position::new(0, 13), Some(&schema))
        .into_iter()
        .find(|i| i.label == "CPU")
        .unwrap();
    let Some(CompletionTextEdit::Edit(edit)) = item.text_edit else {
        panic!()
    };
    let mut updated = document;
    assert!(updated.apply_changes(vec![change(edit.range, &edit.new_text)], 2));
    assert_eq!(updated.content, "backend = \"CPU\"\n");
}
#[test]
fn incomplete_inline_enum_values_still_complete() {
    let document = doc("# @enum CPU, Backend\nbackend = ");
    let items = handle_completion(&document, Position::new(1, 10), None);
    assert!(items.iter().any(|i| i.label == "CPU"), "{items:?}");
}
#[test]
fn recommended_actions_append_full_paths_and_valid_defaults() {
    let schema = Schema::from_ratslang_source(
        "sensor {\n# @recommended\nrange = 2m\n# @recommended\ncount = 3\n}\n",
    );
    let document = doc("sensor {\n range = 1m\n}\n");
    let actions = handle_code_actions(
        &document,
        Range::new(Position::new(1, 0), Position::new(1, 8)),
        Some(&schema),
    );
    assert_eq!(actions.len(), 2);
    let CodeActionOrCommand::CodeAction(action) = actions.last().unwrap() else {
        panic!()
    };
    assert!(action.command.is_none());
    let edit = &action.edit.as_ref().unwrap().changes.as_ref().unwrap()[&document.uri][0];
    assert_eq!(edit.new_text, "sensor.count = 3\n");
    assert_eq!(edit.range.start, Position::new(3, 0));
    let source = format!("{}{}", document.content, edit.new_text);
    assert!(ratslang::compile_code(&source).is_ok());
}
#[test]
fn schema_cache_keys_resolve_relative_references_per_document() {
    let a = DocumentState::new(
        "file:///tmp/a/user.rl".parse().unwrap(),
        "_schema = \"./default.rl\"\n".into(),
        1,
    );
    let b = DocumentState::new(
        "file:///tmp/b/user.rl".parse().unwrap(),
        a.content.clone(),
        1,
    );
    assert_ne!(
        RatslangBackend::schema_uri(&a),
        RatslangBackend::schema_uri(&b)
    );
}
#[tokio::test]
async fn definitions_follow_namespaced_includes_and_schema_locations() {
    let folder =
        std::env::temp_dir().join(format!("ratslang-lsp-definition-{}", std::process::id()));
    std::fs::create_dir_all(&folder).unwrap();
    std::fs::write(folder.join("included.rl"), "count = 3\n").unwrap();
    let uri = Uri::from_file_path(folder.join("user.rl")).unwrap();
    let document = DocumentState::new(
        uri,
        "sensor {\n= ./included.rl\n}\nother {\n= ./included.rl\n}\n".into(),
        1,
    );
    let (service, _) = LspService::new(RatslangBackend::new);
    let definition = service
        .inner()
        .find_definition(&document, "sensor.count", &mut HashSet::new(), 0)
        .unwrap();
    assert!(definition.uri.as_str().ends_with("included.rl"));
    let schema = Schema::load(&document.content, &folder, Some(&document.uri)).unwrap();
    assert_eq!(
        schema
            .get_property("sensor.count")
            .unwrap()
            .default
            .as_deref(),
        Some("3")
    );
    let source_docs = schema.get_property("sensor.count").unwrap().documentation();
    assert!(source_docs.contains("included.rl:1"), "{source_docs}");
    std::fs::write(folder.join("cycle.rl"), "= ./cycle.rl\n").unwrap();
    assert!(
        Schema::load("= ./cycle.rl\n", &folder, None)
            .unwrap_err()
            .contains("cyclic include")
    );
    std::fs::remove_dir_all(folder).unwrap();
}

#[test]
fn dotted_namespace_symbols_and_eof_comments_are_supported() {
    let document = doc("sensor.front {\n count = 3\n}\n# final comment");
    let symbols = handlers::symbols::handle_document_symbols(&document);
    assert_eq!(symbols[0].name, "sensor.front");
    assert_eq!(symbols[0].children.as_ref().unwrap()[0].name, "count");
    assert!(analyze(&document, None).is_empty());
}

#[test]
fn schema_overrides_keep_documentation_and_recommended_status() {
    let schema = Schema::from_ratslang_source(
        "# Useful count\n# @recommended\n# @min 1\ncount = 3\ncount = 5\n",
    );
    let count = schema.get_property("count").unwrap();
    assert_eq!(count.description.as_deref(), Some("Useful count"));
    assert!(count.recommended);
    assert_eq!(count.default.as_deref(), Some("5"));
}

#[test]
fn include_order_selects_the_final_definition_and_preserves_its_documentation() {
    let folder = std::env::temp_dir().join(format!("ratslang-lsp-order-{}", std::process::id()));
    std::fs::create_dir_all(&folder).unwrap();
    std::fs::write(folder.join("included.rl"), "# Included count\ncount = 3\n").unwrap();
    let uri = Uri::from_file_path(folder.join("default.rl")).unwrap();
    let after = Schema::load(
        "= ./included.rl\n# Local count\ncount = 5\n",
        &folder,
        Some(&uri),
    )
    .unwrap();
    assert_eq!(
        after
            .get_property("count")
            .unwrap()
            .location
            .as_ref()
            .unwrap()
            .uri,
        uri
    );
    assert_eq!(
        after.get_property("count").unwrap().description.as_deref(),
        Some("Local count")
    );
    let before = Schema::load(
        "# Local count\ncount = 5\n= ./included.rl\n",
        &folder,
        Some(&uri),
    )
    .unwrap();
    assert!(
        before
            .get_property("count")
            .unwrap()
            .location
            .as_ref()
            .unwrap()
            .uri
            .as_str()
            .ends_with("included.rl")
    );
    assert_eq!(
        before.get_property("count").unwrap().default.as_deref(),
        Some("3")
    );
    std::fs::remove_dir_all(folder).unwrap();
}

#[tokio::test]
async fn references_navigate_to_the_definition_at_the_time_of_assignment() {
    let document = doc("count = 1\nlimit = count\ncount = 2\n");
    let uri = document.uri.clone();
    let (service, _) = LspService::new(RatslangBackend::new);
    service
        .inner()
        .backend
        .update_document(uri.clone(), document);
    let params = serde_json::from_value(serde_json::json!({"textDocument":{"uri":uri.as_str()},"position":{"line":1,"character":10}})).unwrap();
    let response = service
        .inner()
        .goto_definition(params)
        .await
        .unwrap()
        .unwrap();
    let GotoDefinitionResponse::Scalar(location) = response else {
        panic!()
    };
    assert_eq!(location.range.start.line, 0);
}

#[test]
fn header_comments_select_local_and_https_schemas_without_configuration_variables() {
    for reference in [
        "./default.rl",
        "/tmp/default.rl",
        "file:///tmp/default.rl",
        "https://example.org/default.rl",
    ] {
        let document = doc(&format!(
            "# User overrides\n\n# @schema {reference}\nsensor.count = 2\n"
        ));
        assert_eq!(document.schema_ref.as_deref(), Some(reference));
        assert_eq!(document.schema_range.unwrap().start, Position::new(2, 0));
        assert!(
            document
                .symbols
                .assignments
                .iter()
                .all(|a| a.full_path != "_schema")
        );
        let uri = RatslangBackend::schema_uri(&document).unwrap();
        if reference.starts_with("https:") {
            assert_eq!(uri.as_str(), reference);
        } else {
            assert_eq!(uri.as_str(), "file:///tmp/default.rl");
        }
        assert!(ratslang::compile_code(&document.content).is_ok());
    }
    assert_eq!(
        doc("# @schema = \"./my defaults.rl\"\n")
            .schema_ref
            .as_deref(),
        Some("./my defaults.rl")
    );
    assert_eq!(
        doc("# @schema old.rl\n# @schema new.rl\n_schema = \"legacy.rl\"\n")
            .schema_ref
            .as_deref(),
        Some("new.rl")
    );
    assert!(
        doc("value = 1\n# @schema ignored.rl\n")
            .schema_ref
            .is_none()
    );
    assert!(doc("# @schemas ignored.rl\n").schema_ref.is_none());
    assert!(RatslangBackend::schema_uri(&doc("# @schema\n")).is_none());
    let mut document = doc("# @schema old.rl\n");
    document.update("# @schema new.rl\n".into(), 2);
    assert_eq!(document.schema_ref.as_deref(), Some("new.rl"));
}

#[test]
fn unknown_settings_suggest_unique_nearby_names_and_mark_the_key() {
    let schema = Schema::from_ratslang_source("sensor.distance = 1m\nsensor.count = 3\n");
    for typo in ["sensor.distnace", "sensor.distanc", "sensor.distannce"] {
        assert_eq!(schema.suggest_setting(typo), Some("sensor.distance"));
        let document = doc(&format!("{typo} = 25cm\n"));
        let diagnostics = analyze(&document, Some(&schema));
        assert_eq!(diagnostics.len(), 1);
        assert!(
            diagnostics[0]
                .message
                .contains("Did you mean sensor.distance?")
        );
        assert_eq!(
            diagnostics[0].range,
            document.symbols.assignments[0].selection_range
        );
        assert_eq!(
            diagnostics[0].code,
            Some(NumberOrString::String("unknown-setting".into()))
        );
    }
    assert!(schema.suggest_setting("sensor.distance").is_none());
    assert!(schema.suggest_setting("unrelated_setting").is_none());
    let ambiguous = Schema::from_ratslang_source("cat = 1\ncar = 2\n");
    assert!(ambiguous.suggest_setting("cap").is_none());
    assert!(analyze(&doc("sensor.distnace = 25cm\n"), None).is_empty());
}

#[test]
fn typo_quickfix_changes_only_the_selected_key_inside_its_namespace() {
    let schema = Schema::from_ratslang_source("sensor.distance = 1m\nsensor.count = 3\n");
    let document =
        doc("# Keep this comment\nsensor {\n distnace = 25cm # keep this too\n counnt = 4\n}\n");
    let range = document.symbols.assignments[0].selection_range;
    let actions = handle_code_actions(&document, range, Some(&schema));
    assert_eq!(actions.len(), 1);
    let CodeActionOrCommand::CodeAction(action) = &actions[0] else {
        panic!()
    };
    assert_eq!(action.is_preferred, Some(true));
    let edit = &action.edit.as_ref().unwrap().changes.as_ref().unwrap()[&document.uri][0];
    assert_eq!(edit.new_text, "distance");
    let mut updated = document.clone();
    assert!(updated.apply_changes(vec![change(edit.range, &edit.new_text)], 2));
    assert_eq!(
        updated.content,
        "# Keep this comment\nsensor {\n distance = 25cm # keep this too\n counnt = 4\n}\n"
    );
    assert_eq!(analyze(&updated, Some(&schema)).len(), 1);
    // A namespace typo needs a block edit; do not insert a fully qualified key inside it.
    let document = doc("sensro {\n distance = 2m\n}\n");
    assert!(
        handle_code_actions(
            &document,
            document.symbols.assignments[0].selection_range,
            Some(&schema)
        )
        .is_empty()
    );
}

#[test]
fn hover_distinguishes_an_override_from_its_default_and_both_sources() {
    let schema_uri: Uri = "https://example.org/default.rl".parse().unwrap();
    let schema = Schema::load(
        "# Sensor range\nsensor.distance = 2m\n",
        std::path::Path::new("."),
        Some(&schema_uri),
    )
    .unwrap();
    let document = doc("# user overrides\nsensor.distance = 25cm\n");
    let hover = handle_hover(&document, Position::new(1, 3), Some(&schema)).unwrap();
    let HoverContents::Markup(markup) = hover.contents else {
        panic!()
    };
    assert!(
        markup.value.contains("**Default:** `2m`"),
        "{}",
        markup.value
    );
    assert!(markup.value.contains("**Override assignment:** `25cm`"));
    assert!(markup.value.contains("https://example.org/default.rl:2"));
    assert!(markup.value.contains("file:///tmp/editor.rl:2"));
    let hover = handle_hover(&document, Position::new(1, 3), None).unwrap();
    let HoverContents::Markup(markup) = hover.contents else {
        panic!()
    };
    assert!(markup.value.contains("**Assignment:** `25cm`"));
    assert!(!markup.value.contains("**Default:"));
}

#[test]
fn lifecycle_annotations_diagnose_required_overrides_and_offer_deprecation_fixes() {
    let schema = Schema::from_ratslang_source(
        "# Device ID\n# @required\nname = \"\"\n# @deprecated Old range key\n# @renamed_to sensor.distance\nold = 1m\nsensor.distance = 1m\n",
    );
    let document = doc("# @schema ./default.rl\nold = 2m\n");
    let diagnostics = analyze(&document, Some(&schema));
    assert!(
        diagnostics
            .iter()
            .any(|d| d.message.contains("Required setting name"))
    );
    assert!(diagnostics.iter().any(|d| {
        d.tags
            .as_ref()
            .is_some_and(|tags| tags.contains(&DiagnosticTag::DEPRECATED))
    }));
    let actions = handle_code_actions(
        &document,
        document.symbols.assignments[0].selection_range,
        Some(&schema),
    );
    assert!(actions.iter().any(
        |a| matches!(a, CodeActionOrCommand::CodeAction(a) if a.title == "Insert default for name")
    ));
    let rename = actions
        .iter()
        .find_map(|a| match a {
            CodeActionOrCommand::CodeAction(a) if a.title == "Rename old to sensor.distance" => {
                Some(a)
            }
            _ => None,
        })
        .unwrap();
    assert_eq!(
        rename.edit.as_ref().unwrap().changes.as_ref().unwrap()[&document.uri][0].new_text,
        "sensor.distance"
    );
    assert!(
        analyze(
            &doc("name = \"robot\"\nsensor.distance = 2m\n"),
            Some(&schema)
        )
        .is_empty()
    );
}
#[test]
fn physical_constraints_and_reference_provenance_reach_hover() {
    let schema = Schema::from_ratslang_source(
        "# @type angle\n# @max 180deg\nangle = 90deg\n# @type velocity\nspeed = 1m/s\n",
    );
    assert!(analyze(&doc("angle = 200deg\nspeed = 1m/s2\n"), Some(&schema)).len() == 2);
    let document = doc("base = 30deg\nangle = base\nbase = 60deg\n");
    let hover = handle_hover(&document, Position::new(1, 3), Some(&schema)).unwrap();
    let HoverContents::Markup(markup) = hover.contents else {
        panic!()
    };
    assert!(markup.value.contains("Effective value"));
    assert!(
        markup
            .value
            .contains("Referenced definitions at assignment time")
    );
    assert!(markup.value.contains("`base`"));
    assert!(markup.value.contains("editor.rl:1"));
}

#[test]
fn mass_completions_hover_and_constraint_diagnostics_use_the_new_dimension() {
    let schema = Schema::from_ratslang_source(
        "# Payload limit\n# @type mass\n# @min 500g\n# @max 2kg\npayload = 1kg\n",
    );
    let document = doc("payload = 750g\n");
    assert_eq!(
        document.symbols.assignments[0].expression,
        "750g",
        "{}",
        document.tree.root_node().to_sexp()
    );
    assert!(analyze(&document, Some(&schema)).is_empty());
    assert!(!analyze(&doc("payload = 250mg\n"), Some(&schema)).is_empty());
    assert!(!analyze(&doc("payload = 1m\n"), Some(&schema)).is_empty());
    let hover = handle_hover(&document, Position::new(0, 3), Some(&schema)).unwrap();
    let HoverContents::Markup(markup) = hover.contents else {
        panic!()
    };
    assert!(markup.value.contains("Payload limit"));
    assert!(markup.value.contains("`mass`"));
    let document = doc("payload = ");
    let completions = handle_completion(&document, Position::new(0, 10), Some(&schema));
    assert!(completions.iter().any(|item| item.label == "1kg"));
}
#[test]
fn numeric_overflow_is_diagnosed_without_a_schema_or_finite_annotation() {
    let huge = format!("{}.0", "9".repeat(310));
    for source in [format!("value = {huge}\n"), format!("value = [{huge}]\n")] {
        let diagnostics = analyze(&doc(&source), None);
        assert!(
            diagnostics.iter().any(|d| d.message.contains("finite")),
            "{diagnostics:?}"
        );
    }
}

#[test]
fn removed_finite_annotation_is_an_error_in_the_editor() {
    for annotation in ["finite", "finite true", "finite false"] {
        let diagnostics = analyze(&doc(&format!("# @{annotation}\nvalue = 1\n")), None);
        assert!(
            diagnostics
                .iter()
                .any(|d| d.message.contains("@finite is not supported")),
            "{diagnostics:?}"
        );
    }
}
