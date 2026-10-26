use super::*;
fn folder(label: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("ratslang-tools-{label}-{}", std::process::id()));
    std::fs::create_dir_all(&path).unwrap();
    path
}
#[test]
fn required_deprecated_and_renamed_rules_are_shared_and_user_presence_matters() {
    let defaults = compile_code(
        "# @required\nname = \"\"\n# @deprecated Prefer new\n# @renamed_to new\nold = 1\nnew = 1\n",
    )
    .unwrap();
    let user = compile_code("old = 3\nnaem = \"robot\"\n").unwrap();
    let issues = validate_schema(&user.vars, &defaults.vars).unwrap();
    assert!(issues.iter().any(|i| i.kind == IssueKind::Required));
    assert!(
        issues
            .iter()
            .any(|i| i.kind == IssueKind::Deprecated && i.replacement.as_deref() == Some("new"))
    );
    assert!(
        issues
            .iter()
            .any(|i| i.kind == IssueKind::Unknown && i.replacement.as_deref() == Some("name"))
    );
    assert!(
        !validate_schema(
            &compile_code("name = \"robot\"\n").unwrap().vars,
            &defaults.vars
        )
        .unwrap()
        .iter()
        .any(ConfigIssue::is_error)
    );
    assert!(compile_code("# @required maybe\nx = 1\n").is_err());
    assert!(compile_code("# @renamed_to invalid-key\nx = 1\n").is_err());
}
#[test]
fn layered_configs_support_paths_with_spaces_and_cli_provenance() {
    let folder = folder("paths with spaces");
    std::fs::write(
        folder.join("default.rl"),
        "# @required\nname = \"\"\nx = 1\ny = 0\n",
    )
    .unwrap();
    std::fs::write(folder.join("user.rl"), "name = \"robot\"\nx = 2\ny = x\n").unwrap();
    let config = Configuration::load(
        &folder.join("user.rl"),
        Some(&folder.join("default.rl")),
        &["x = 3".into()],
    )
    .unwrap();
    assert!(config.issues().unwrap().is_empty());
    let x = config.explain("x").unwrap().unwrap();
    assert_eq!(x.history.len(), 3);
    assert_eq!(config.layer(&x.winner), "CLI");
    assert_eq!(
        config
            .explain("y")
            .unwrap()
            .unwrap()
            .value
            .to_source()
            .unwrap(),
        "2"
    );
    assert_eq!(
        config.layer(&config.explain("name").unwrap().unwrap().winner),
        "file"
    );
    std::fs::remove_dir_all(folder).unwrap();
}
#[test]
fn minimization_preserves_comments_exact_floats_required_overrides_and_semantics() {
    let folder = folder("minimize");
    std::fs::write(
        folder.join("default.rl"),
        "# @required\nname = \"robot\"\nsensor.range = 1m\ngain = 1.0\n",
    )
    .unwrap();
    let source = "# @schema ./default.rl\n# Range comment\nsensor {\n range = 100cm # inline comment\n}\nname = \"robot\"\ngain = 1.0000001\n";
    std::fs::write(folder.join("user.rl"), source).unwrap();
    let config = Configuration::load(
        &folder.join("user.rl"),
        Some(&folder.join("default.rl")),
        &[],
    )
    .unwrap();
    let edited = config.minimize().unwrap();
    assert!(
        edited.contains("# range = 100cm # inline comment"),
        "{edited}"
    );
    assert!(edited.contains("# Range comment"));
    assert!(edited.contains("\nname = \"robot\""));
    assert!(edited.contains("\ngain = 1.0000001"));
    assert_eq!(
        std::fs::read_to_string(folder.join("user.rl")).unwrap(),
        source
    );
    std::fs::write(folder.join("user.rl"), &edited).unwrap();
    assert_eq!(
        Configuration::load(
            &folder.join("user.rl"),
            Some(&folder.join("default.rl")),
            &[]
        )
        .unwrap()
        .minimize()
        .unwrap(),
        edited
    );
    std::fs::remove_dir_all(folder).unwrap();
}
#[test]
fn migration_preserves_comments_handles_nested_namespaces_and_rejects_conflicts() {
    let folder = folder("migrate");
    let defaults = folder.join("default.rl");
    let user = folder.join("user.rl");
    std::fs::write(
        &defaults,
        "# @renamed_to output.path\noutput.export.path = \"\"\noutput.path = \"\"\n",
    )
    .unwrap();
    std::fs::write(
        &user,
        "# Keep\noutput.export {\n path = \"run.txt\" # explanation\n}\n",
    )
    .unwrap();
    let config = Configuration::load(&user, Some(&defaults), &[]).unwrap();
    let edited = config.migrate().unwrap();
    assert!(edited.contains("# path = \"run.txt\" # explanation"));
    assert!(edited.contains("output.path = \"run.txt\""));
    std::fs::write(&user, &edited).unwrap();
    assert_eq!(
        Configuration::load(&user, Some(&defaults), &[])
            .unwrap()
            .migrate()
            .unwrap(),
        edited
    );
    std::fs::write(
        &user,
        "output.export.path = \"old.txt\"\noutput.path = \"new.txt\"\n",
    )
    .unwrap();
    assert!(
        Configuration::load(&user, Some(&defaults), &[])
            .unwrap()
            .migrate()
            .is_err()
    );
    std::fs::write(
        &user,
        "output.export.path = \"old.txt\"\ncopy = output.export.path\n",
    )
    .unwrap();
    assert!(
        Configuration::load(&user, Some(&defaults), &[])
            .unwrap()
            .migrate()
            .is_err()
    );
    std::fs::remove_dir_all(folder).unwrap();
}
#[test]
fn generation_carries_documentation_and_selects_required_recommended_or_full() {
    let folder = folder("generate");
    let defaults = folder.join("default.rl");
    std::fs::write(&defaults, "# Robot identifier\n# @required\nname = \"\"\n# @recommended\nrate = 10\nother = 2\n# @recommended\nsensor {\nrange = 1m\n}\n# @deprecated\nold = 0\n").unwrap();
    let schema = compile_file(&defaults, None, None).unwrap();
    let minimal = generate(&schema, false).unwrap();
    assert!(minimal.contains("# Robot identifier"));
    assert!(minimal.contains("name =") && minimal.contains("rate ="));
    assert!(minimal.contains("sensor.range = 1m"));
    assert!(!minimal.contains("other =") && !minimal.contains("old ="));
    assert!(generate(&schema, true).unwrap().contains("other = 2"));
    assert!(compile_code(&minimal).is_ok());
    std::fs::remove_dir_all(folder).unwrap();
}
#[test]
fn formatter_preserves_strings_comments_include_spelling_and_crlf() {
    let source = "# heading\r\nsensor {\r\nrange=25cm # inline\r\nvalues = [1,\r\n2]\r\ntext=\"first\r\n  second\"\r\n}\r\n= ./included.rl\r\n";
    let formatted = format_document(source).unwrap();
    assert!(
        formatted.contains("  range = 25cm # inline\r\n"),
        "{formatted}"
    );
    assert!(formatted.contains("\"first\r\n  second\""));
    assert!(formatted.contains("= ./included.rl\r\n"));
    assert_eq!(format_document(&formatted).unwrap(), formatted);
    assert!(format_document("x = [\n").is_err());
}

#[test]
fn compiler_assignment_spans_retain_units_pairs_broadcasts_and_nested_arrays() {
    let source = "= ./does-not-exist.rl\nrobot {\n# @type mass\npayload = 750g # keep comment\na, b = 1m/s, 9.81m/s^2\nx, y = [[1kg, 2g], [3mg, 4ug]]\ninner.key = _stored\n}\n";
    let assignments = source_assignments(source).unwrap();
    let expected = [
        ("payload", "750g"),
        ("a", "1m/s"),
        ("b", "9.81m/s^2"),
        ("x", "[[1kg, 2g], [3mg, 4ug]]"),
        ("y", "[[1kg, 2g], [3mg, 4ug]]"),
        ("inner.key", "_stored"),
    ];
    assert_eq!(assignments.len(), expected.len());
    for (assignment, (key, value)) in assignments.iter().zip(expected) {
        assert_eq!(assignment.namespace, "robot");
        assert_eq!(assignment.name, key);
        assert_eq!(&source[assignment.selection.clone()], key);
        assert_eq!(&source[assignment.value.clone()], value);
    }
    assert!(assignments[0].annotations.contains(&"type mass".into()));
    assert!(source_assignments("value = [").is_err());
}

#[test]
fn a_user_template_drops_annotations_and_retired_settings_but_keeps_the_rules() {
    let schema = "# @schema ./self.rl\n\n# Payload, mount included.\n# @type mass\n# @min 500g\npayload = 2kg\n\nsensor {\n    # Distance to the target.\n    # @type length\n    distance = 1m\n}\n\n# Old names, kept for migration.\nold {\n    # @deprecated Moved\n    # @renamed_to sensor.distance\n    distance = 1m\n}\n";
    let template = user_template(schema).unwrap();
    assert_eq!(
        template,
        "# @schema ./self.rl\n\n# Payload, mount included.\npayload = 2kg\n\nsensor {\n    # Distance to the target.\n    distance = 1m\n}\n"
    );

    // Same values as the schema, and the schema's rules still check a user
    // file that has none of its own.
    let defaults = compile_code(schema).unwrap();
    let edited = compile_code(&template.replace("2kg", "100g")).unwrap();
    let issues = validate_schema(&edited.vars, &defaults.vars).unwrap();
    assert!(
        issues
            .iter()
            .any(|issue| issue.name == "payload" && issue.kind == IssueKind::Invalid),
        "{issues:?}"
    );
    let unchanged = compile_code(&template).unwrap();
    assert!(
        validate_schema(&unchanged.vars, &defaults.vars)
            .unwrap()
            .is_empty()
    );
}

#[test]
fn a_starter_carries_the_schema_only_when_asked() {
    let defaults = "# Mass.\n# @recommended\n# @type mass\npayload = 2kg\n\n# Range.\n# @type length\nrange = 1m\n";
    let full = starter(defaults, Selection::All, None).unwrap();
    assert!(
        !full.contains("@schema") && full.contains("range = 1m"),
        "{full}"
    );
    let recommended = starter(
        defaults,
        Selection::Annotated("recommended"),
        Some("https://example.org/releases/download/v1.0.0/default.rl"),
    )
    .unwrap();
    assert!(
        recommended
            .starts_with("# @schema https://example.org/releases/download/v1.0.0/default.rl\n\n"),
        "{recommended}"
    );
    assert!(recommended.contains("payload = 2kg") && !recommended.contains("range"));
    assert!(starter(defaults, Selection::Annotated("missing"), None).is_err());
    assert!(starter(defaults, Selection::All, Some("a\nb")).is_err());
}
