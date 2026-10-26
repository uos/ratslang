use super::*;
use crate::compile_code;

fn check(source: &str) -> Result<crate::Evaluated> {
    crate::compile_code_with_state(
        &format!("{source}\n"),
        &std::env::current_dir()?,
        None,
        std::io::sink(),
        false,
    )
}

#[test]
fn enum_choices_accept_identifiers_quoted_strings_and_paths() {
    for source in [
        "# @enum CPU, Backend\nbackend = CPU",
        "# @enum CPU, Backend\nbackend = \"Backend\"",
        "# @enum ./cpu, ./backend\nbackend = ./cpu",
        "# @enum \"Best Effort\", Reliable\nbackend = \"Best Effort\"",
        "# @type = enum[CPU,Backend]\nbackend = CPU",
        "# @type = string\n# @options = [CPU,Backend]\nbackend = Backend",
    ] {
        assert!(check(source).is_ok(), "{source}");
    }
    for value in ["Other", "cpu", "true", "3"] {
        let error = check(&format!("# @enum CPU, Backend\nbackend = {value}"))
            .unwrap_err()
            .to_string();
        assert!(
            error.contains("backend") && error.contains("CPU"),
            "{error}"
        );
    }
}

#[test]
fn numeric_limits_are_inclusive_and_integer_comparison_is_exact() {
    for value in ["0", "5", "10", "5.5"] {
        assert!(check(&format!("# @min 0\n# @max 10\ncount = {value}")).is_ok());
    }
    for value in ["-1", "11", "\"five\""] {
        let error = check(&format!("# @min 0\n# @max 10\ncount = {value}"))
            .unwrap_err()
            .to_string();
        assert!(error.contains("count"), "{error}");
    }
    assert!(check("# @max 9223372036854775806\ncount = 9223372036854775807").is_err());
    assert!(check("# @min 9223372036854775807\ncount = 9223372036854775806").is_err());
}

#[test]
fn physical_limits_compare_units_and_reject_wrong_dimensions() {
    assert!(check("# @min 1mm\n# @max 1m\ndistance = 50cm").is_ok());
    assert!(check("# @minimum = 1000us\n# @maximum = 2s\ntimeout = 1ms").is_ok());
    for source in [
        "# @min 1mm\ndistance = 1um",
        "# @max 1m\ndistance = 200cm",
        "# @min 1m\ntimeout = 2s",
        "# @min 1mm\ndistance = 1",
        "# @min 2s\n# @max 1m\nvalue = 3s",
    ] {
        assert!(check(source).is_err(), "{source}");
    }
}

#[test]
fn declared_types_cover_scalars_arrays_and_ranges() {
    let source = "# @type integer\ncount = 3\n# @type number\nratio = 0.5\n# @type boolean\non = true\n# @type path\nfile = ./config.rl\n# @type array<length>\ntranslation = [1m, 2cm, 3mm]\n# @type range<time>\ntimeout = 1ms..2s\n# @type range<length>\nrange = ..\n";
    let evaluated = check(source).unwrap();
    assert_eq!(evaluated.vars.constraints().unwrap().len(), 7);
    for source in [
        "# @type integer\ncount = 1.0",
        "# @type length\nvalue = 3s",
        "# @type array<length>\nvalue = [1m, 2s]",
        "# @type range<time>\nvalue = 1mm..2m",
    ] {
        assert!(check(source).is_err(), "{source}");
    }
}

#[test]
fn constraints_survive_namespaces_references_and_later_overrides() {
    let source = "base = 3\nsensor {\n# @min 1\n# @max 10\ncount = base\n}\nsensor.count = 8\n";
    let evaluated = check(source).unwrap();
    assert!(
        evaluated.vars.usage_vars().1.is_empty(),
        "validation must not mark fields read"
    );
    assert!(
        evaluated
            .vars
            .constraints()
            .unwrap()
            .contains_key("sensor.count")
    );
    assert!(
        evaluated
            .vars
            .filter_ns(&["sensor"])
            .constraints()
            .unwrap()
            .contains_key("count")
    );
    assert!(check(&(source.to_owned() + "sensor.count = 11")).is_err());
    assert!(check("# @min 1\na, b = 2, 3\nb = 0").is_err());
    assert!(
        check("# @min 0\nvalue = -1\nvalue = 2").is_ok(),
        "only final values are validated"
    );
}

#[test]
fn malformed_or_contradictory_rules_fail_clearly() {
    for source in [
        "# @enum\nvalue = CPU",
        "# @enum CPU,,Backend\nvalue = CPU",
        "# @enum CPU,CPU\nvalue = CPU",
        "# @enum \"unclosed\nvalue = CPU",
        "# @min\nvalue = 1",
        "# @min twelve\nvalue = 1",
        "# @min 10\n# @max 5\nvalue = 7",
        "# @type nonsense\nvalue = 1",
        "# @type time\n# @min 1m\nvalue = 2s",
        "# @enum CPU,Backend\n# @min 1\nvalue = CPU",
        "# @min 0\nsensor { value = 1\n}\n",
    ] {
        assert!(check(source).is_err(), "{source}");
    }
}

#[test]
fn metadata_does_not_replace_filtering_tags_or_change_usage() {
    let source = "# @recommended\n# @enum CPU, Backend\n# @min_lookalike\nbackend = CPU\n";
    let evaluated = compile_code(source).unwrap();
    assert!(evaluated.annotations.contains_key("recommended"));
    assert!(!evaluated.annotations.contains_key("enum CPU, Backend"));
    assert!(evaluated.annotations.contains_key("min_lookalike"));
    assert!(evaluated.vars.usage_vars().1.is_empty());
    let vars = crate::VariableHistory::new(vec![crate::StatementKindOwned::VariableDef(
        Statement::AssignLeft {
            lhs: vec!["value".parse().unwrap()],
            rhs: vec![Rhs::Val(Val::StringVal("# @min 1".into()))],
            annotations: vec![],
        },
    )]);
    assert!(vars.constraints().unwrap().is_empty());
}

#[test]
fn included_rules_follow_the_include_namespace_and_parent_overrides() {
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("ratslang-constraints-{unique}"));
    std::fs::create_dir(&dir).unwrap();
    std::fs::write(
        dir.join("rules.rl"),
        "# @min 1mm\n# @max 1m\ndistance = 50cm\n",
    )
    .unwrap();
    for (value, valid) in [("75cm", true), ("2m", false)] {
        let source = format!("sensor {{\n= ./rules.rl\n}}\nsensor.distance = {value}\n");
        assert_eq!(
            crate::compile_code_with_state(&source, &dir, None, std::io::sink(), false).is_ok(),
            valid
        );
    }
    std::fs::remove_file(dir.join("rules.rl")).unwrap();
    std::fs::remove_dir(dir).unwrap();
}

#[test]
fn numeric_types_accept_numeric_bounds_without_losing_precision() {
    assert!(check("# @type float\n# @min 0\nratio = 0.5").is_ok());
    assert!(check("# @type integer\n# @min 0.5\ncount = 1").is_ok());
    assert!(check("# @min -0.5\ncount = -1").is_err());
    assert!(check("# @max -0.5\ncount = 0").is_err());
    assert!(check("# @min 9223372036854775808.0\ncount = 9223372036854775807").is_err());
    assert!(check("# @max 9223372036854775807\ncount = 9223372036854775808.0").is_err());
}

#[test]
fn ast_formatting_retains_constraint_declarations() {
    let evaluated =
        check("# @enum CPU, Backend\nbackend = CPU\n# @min 1mm\ndistance = 2mm").unwrap();
    let ast = crate::Root {
        vardefs: evaluated
            .vars
            .ast
            .iter()
            .map(|statement| {
                let StatementKindOwned::VariableDef(statement) = statement;
                statement.clone()
            })
            .collect(),
    };
    let source = ast.to_source().unwrap();
    assert!(source.contains("# @enum CPU, Backend"));
    assert!(source.contains("# @min 1mm"));
    let reloaded = check(&source).unwrap();
    assert_eq!(
        evaluated.vars.constraints().unwrap(),
        reloaded.vars.constraints().unwrap()
    );
    assert!(check(&(source + "\nbackend = Other")).is_err());
}

#[test]
fn exclusive_bounds_preserve_strict_limits_and_override_inclusivity() {
    for source in [
        "# @min_exclusive 0m\nvalue = 1um",
        "# @min 0\n# @max_exclusive 0.5\nvalue = 0",
        "# @min_exclusive 0\nvalue = 1\n# @min 0\nvalue = 0",
        "# @max_exclusive 1\nvalue = 0\n# @max 1\nvalue = 1",
    ] {
        assert!(check(source).is_ok(), "{source}");
    }
    for source in [
        "# @min_exclusive 0m\nvalue = 0mm",
        "# @max_exclusive 0.5\nvalue = 0.5",
        "# @min_exclusive 1\n# @max 1\nvalue = 1",
        "# @min 1\n# @max_exclusive 1\nvalue = 1",
        "# @min 0\nvalue = 1\n# @min_exclusive 0\nvalue = 0",
    ] {
        assert!(check(source).is_err(), "{source}");
    }
}

#[test]
fn numeric_array_constraints_check_every_element_with_units() {
    assert!(check("# @type array<length>\n# @min_exclusive 0m\nvalue = [1um, 1m]").is_ok());
    assert!(check("# @type array<array<number>>\n# @min 0\n# @max_exclusive 255\nvalue = [[0, 254.9], [1, 2]]").is_ok());
    let error = check("# @min_exclusive 0\nvalue = [[1, 2], [3, 0]]")
        .unwrap_err()
        .to_string();
    assert!(
        error.contains("value") && error.contains("element [1]"),
        "{error}"
    );
    assert!(check("# @min 0m\nvalue = [1mm, 2s]").is_err());
    assert!(check("# @max_exclusive 255\nvalue = [0, 255, 1]").is_err());
}

#[test]
fn finite_constraints_reject_nonfinite_scalar_and_array_values() {
    let rules = Constraints::default();
    for value in [f64::INFINITY, f64::NEG_INFINITY, f64::NAN] {
        let value = Rhs::Val(Val::NumVal(NumVal::Floating(value)));
        assert!(rules.validate("value", &value).is_err());
        assert!(
            rules
                .validate("value", &Rhs::Array(vec![Box::new(value)]))
                .is_err()
        );
    }
    assert!(check("value = [[1, -2.5], [0, 3]]").is_ok());
    assert!(check("# @finite yes\nvalue = 1").is_err());
}

#[test]
fn enum_case_annotations_preserve_case_variants_without_changing_values() {
    let evaluated = check("# @enum CPU, Backend\n# @enum_case insensitive\nvalue = cPu").unwrap();
    assert_eq!(
        evaluated.vars.resolve("value").unwrap(),
        Some(Rhs::Val(Val::StringVal("cPu".into())))
    );
    assert!(check("# @enum CPU, Backend\n# @enum_case insensitive\nvalue = Other").is_err());
    assert!(check("# @enum CPU\n# @enum_case insensitive\nvalue = CPU\n# @enum_case sensitive\nvalue = cpu").is_err());
    assert!(check("# @enum CPU\n# @enum_case maybe\nvalue = CPU").is_err());
}

#[test]
fn tooling_parses_constraints_without_evaluating_an_assignment() {
    let rules =
        Constraints::from_annotations(["type length", "min_exclusive 0m", "max 1m"]).unwrap();
    assert!(rules.minimum_exclusive);
    assert_eq!(rules.value_type, Some(ValueType::Length));
    assert!(Constraints::from_annotations(["min 10", "max 1"]).is_err());
}

#[test]
fn tooling_syntax_diagnostics_use_compiler_spans_and_accept_eof_comments() {
    assert!(crate::syntax_diagnostics("count = 3 # final comment").is_empty());
    let source = "# 😀\ncount = [\n";
    let errors = crate::syntax_diagnostics(source);
    assert!(!errors.is_empty());
    assert!(
        errors
            .iter()
            .all(|e| e.range.start <= e.range.end && e.range.end <= source.len())
    );
}
