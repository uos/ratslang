use super::*;
use crate::{Length, Time, micrometer, millisecond};
use ::serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Config {
    sensor: Sensor,
    enabled: bool,
    count: u32,
    ratio: f64,
    tags: Vec<String>,
    missing: Option<String>,
    mode: Mode,
}
#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Sensor {
    #[serde(with = "length")]
    distance: Length,
    #[serde(with = "time")]
    timeout: Time,
}
#[derive(Debug, Serialize, Deserialize, PartialEq)]
enum Mode {
    Fast,
    Tuned { iterations: u32 },
    Pair(u32, bool),
    Named(String),
}

#[test]
fn typed_configuration_round_trip() {
    let config = Config {
        sensor: Sensor {
            distance: Length::new::<micrometer>(1.),
            timeout: Time::new::<millisecond>(250.),
        },
        enabled: true,
        count: 5,
        ratio: 1.,
        tags: vec!["a".into(), "b".into()],
        missing: None,
        mode: Mode::Tuned { iterations: 3 },
    };
    let source = to_string(&config).unwrap();
    assert!(source.contains("sensor.distance = 1um\n"), "{source}");
    assert!(source.contains("sensor.timeout = 250ms\n"), "{source}");
    assert!(!source.contains("missing"));
    assert_eq!(from_str::<Config>(&source).unwrap(), config);
}

#[test]
fn evaluates_references_and_overrides() {
    let sensor: Sensor =
        from_str("base = 1um\ndistance = base\ndistance = 2um\ntimeout = 0.25s\n").unwrap();
    assert_eq!(sensor.distance, Length::new::<micrometer>(2.));
    assert_eq!(sensor.timeout, Time::new::<millisecond>(250.));
}

#[test]
fn rejects_wrong_dimensions_and_bare_quantities() {
    for source in [
        "distance = 1s\ntimeout = 1s",
        "distance = 1m\ntimeout = 1m",
        "distance = 1\ntimeout = 1s",
    ] {
        assert!(from_str::<Sensor>(source).is_err(), "{source}");
    }
    assert!(from_str::<BTreeMap<String, f64>>("distance = 1m").is_err());
}

#[test]
fn enums_and_nested_arrays_round_trip() {
    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct Data {
        mode: Mode,
        data: Vec<Vec<i32>>,
        optional: Option<u32>,
    }
    for mode in [
        Mode::Fast,
        Mode::Pair(3, true),
        Mode::Named("custom".into()),
    ] {
        let value = Data {
            mode,
            data: vec![vec![1, -2], vec![]],
            optional: Some(8),
        };
        let source = to_string(&value).unwrap();
        assert_eq!(from_str::<Data>(&source).unwrap(), value, "{source}");
    }
}

#[test]
fn sensible_units_zero_negative_and_extremes() {
    for (value, source) in [
        (0., "0m"),
        (-1e-6, "-1um"),
        (0.02, "2cm"),
        (1200., "1.2km"),
        (1e-24, "1ym"),
        (1e24, "1Ym"),
    ] {
        assert_eq!(
            UnitVal::from_length(Length::new::<meter>(value))
                .to_source()
                .unwrap(),
            source
        );
    }
    assert_eq!(
        UnitVal::from_time(Time::new::<second>(7200.))
            .to_source()
            .unwrap(),
        "2h"
    );
    assert_eq!(
        UnitVal::from_time(Time::new::<second>(0.000001))
            .to_source()
            .unwrap(),
        "1us"
    );
    assert_eq!(
        UnitVal::from_time(Time::new::<crate::year>(2.))
            .to_source()
            .unwrap(),
        "2a"
    );
}

#[test]
fn errors_for_unrepresentable_values() {
    for value in [f64::NAN, f64::INFINITY] {
        assert!(to_string(&BTreeMap::from([("value", value)])).is_err());
    }
    assert!(to_string(&BTreeMap::from([("value", u64::MAX)])).is_err());
    assert!(to_string(&BTreeMap::from([("value", vec![None::<bool>])])).is_err());
    assert!(to_string(&BTreeMap::from([("value", "a\"b")])).is_err());
    assert!(to_string(&BTreeMap::from([("bad.key", 3)])).is_err());
    assert!(to_string(&3).is_err());
    assert!(from_str::<Sensor>("distance = ???").is_err());
}

#[test]
fn small_and_large_floats_round_trip() {
    for f in [1e-100, 1e100, -1e-30, -0.0, f64::MIN_POSITIVE, f64::MAX] {
        let value = BTreeMap::from([("value", f)]);
        assert_eq!(
            from_str::<BTreeMap<String, f64>>(&to_string(&value).unwrap()).unwrap()["value"],
            f
        );
    }
}

#[test]
fn quantity_round_trips_preserve_precision() {
    for magnitude in [1e-24, 1e-15, 1e-6, 0.0123456789, 1.0, 1234.56789, 1e24] {
        for sign in [-1.0, 1.0] {
            let value = sign * magnitude;
            let sensor = Sensor {
                distance: Length::new::<meter>(value),
                timeout: Time::new::<second>(value),
            };
            let source = to_string(&sensor).unwrap();
            let decoded: Sensor = from_str(&source).unwrap();
            assert!(
                (decoded.distance.get::<meter>() - value).abs() <= value.abs() * 1e-14,
                "{source}"
            );
            assert!(
                (decoded.timeout.get::<second>() - value).abs() <= value.abs() * 1e-14,
                "{source}"
            );
        }
    }
}

#[test]
fn ranges_defaults_and_strict_fields() {
    #[derive(Debug, Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Data {
        span: std::ops::Range<i32>,
        #[serde(default)]
        enabled: bool,
    }
    let data: Data = from_str("span = 1..4").unwrap();
    assert_eq!(data.span, 1..4);
    assert!(!data.enabled);
    assert!(from_str::<Data>("span = 1..4\nunknown = 1s").is_err());
    assert!(from_str::<BTreeMap<String, (u8, u8)>>("a = [1, 2, 3]").is_err());
    assert!(from_str::<Sensor>("distance = 1m\ndistance.child = 2m\ntimeout = 1s").is_err());
}

#[test]
fn serialization_produces_native_ast_values() {
    let sensor = Sensor {
        distance: Length::new::<micrometer>(1.),
        timeout: Time::new::<millisecond>(250.),
    };
    let ast = to_ast(&sensor).unwrap();
    let Statement::AssignLeft {
        lhs,
        rhs,
        annotations,
    } = &ast.vardefs[0];
    assert_eq!(lhs, &["distance".parse::<Var>().unwrap()]);
    assert_eq!(
        rhs,
        &[Rhs::Val(Val::UnitedVal(UnitVal::LengthVal(
            sensor.distance
        )))]
    );
    assert!(annotations.is_empty());
    assert_eq!(
        ast.to_source().unwrap(),
        "distance = 1um\ntimeout = 250ms\n"
    );
    assert_eq!(ast.to_source().unwrap(), to_string(&sensor).unwrap());

    let ast = to_ast(&BTreeMap::from([("values", vec![1, 2])])).unwrap();
    let Statement::AssignLeft { rhs, .. } = &ast.vardefs[0];
    assert!(matches!(rhs.as_slice(), [Rhs::Array(values)] if values.len() == 2));
}

#[test]
fn nodes_can_have_both_values_and_children() {
    #[derive(Serialize, Deserialize)]
    struct Resolution {
        #[serde(rename = "$value", with = "length")]
        value: Length,
        #[serde(rename = "type")]
        kind: String,
    }
    #[derive(Serialize, Deserialize)]
    struct Config {
        resolution: Resolution,
    }
    let source = "resolution = 50cm\nresolution.type = Multi\n";
    let config: Config = from_str(source).unwrap();
    assert_eq!(
        config.resolution.value,
        Length::new::<crate::centimeter>(50.)
    );
    let output = to_string(&config).unwrap();
    assert_eq!(
        crate::compile_code(&output)
            .unwrap()
            .vars
            .resolve("resolution")
            .unwrap(),
        crate::compile_code(source)
            .unwrap()
            .vars
            .resolve("resolution")
            .unwrap()
    );
    assert_eq!(
        from_str::<Config>(&output).unwrap().resolution.kind,
        "Multi"
    );
    assert!(from_str::<::serde::de::IgnoredAny>(source).is_ok());
}

#[test]
fn native_ranges_round_trip_in_all_forms() {
    #[derive(Debug, Serialize, Deserialize)]
    struct Quantity(#[serde(with = "length")] Length);
    #[derive(Serialize, Deserialize)]
    struct Config {
        range: Range<Quantity>,
    }
    for expression in ["1m..4m", "4m..", "..4m", ".."] {
        let source = format!("range = {expression}\n");
        let config: Config = from_str(&source).unwrap();
        let output = to_string(&config).unwrap();
        assert_eq!(output, source);
        let _: Config = from_str(&output).unwrap();
    }
    #[derive(Serialize, Deserialize)]
    struct Standard {
        #[serde(with = "range")]
        bounded: std::ops::Range<i32>,
        #[serde(with = "range")]
        open: std::ops::RangeFrom<i32>,
        #[serde(with = "range")]
        upper: std::ops::RangeTo<i32>,
    }
    let source = "bounded = 1..4\nopen = 2..\nupper = ..3\n";
    let config: Standard = from_str(source).unwrap();
    assert_eq!(to_string(&config).unwrap(), source);
    assert!(from_str::<Config>("range.start = 1m").is_err());
}

#[test]
fn document_preserves_source_and_appends_changed_values() {
    #[derive(Serialize, Deserialize)]
    struct Config {
        count: i32,
    }
    let source =
        "# @recommended\n# Keep this documentation\ncount = 3 # inline\nunknown = [1,\n 2]\n";
    let mut document: Document<Config> = from_str(source).unwrap();
    assert_eq!(to_string(&document).unwrap(), source);
    document.value.count = 5;
    let output = to_string(&document).unwrap();
    assert!(output.starts_with(source));
    assert!(output.contains("# @recommended\ncount = 5\n"));
    let compiled = crate::compile_code(&output).unwrap();
    assert_eq!(
        compiled.vars.resolve("count").unwrap(),
        Some(Rhs::Val(Val::NumVal(NumVal::Integer(5))))
    );
    assert!(compiled.annotations.contains_key("recommended"));
    assert_eq!(
        crate::compile_code(&compiled.to_string_filtered("recommended").unwrap())
            .unwrap()
            .vars
            .resolve("count")
            .unwrap(),
        Some(Rhs::Val(Val::NumVal(NumVal::Integer(5))))
    );
    assert!(compiled.vars.resolve("unknown").unwrap().is_some());
    let reloaded: Document<Config> = from_str(&output).unwrap();
    assert_eq!(to_string(&reloaded).unwrap(), output);
}

#[test]
fn document_rejects_silent_field_removal() {
    #[derive(Serialize, Deserialize)]
    struct Config {
        count: Option<i32>,
    }
    let mut document: Document<Config> = from_str("count = 3").unwrap();
    document.value.count = None;
    assert!(
        to_string(&document)
            .unwrap_err()
            .to_string()
            .contains("cannot remove")
    );
}

fn assert_same_values(original: &str, output: &str) {
    let original = crate::compile_code(original).unwrap();
    let output = crate::compile_code(output).unwrap();
    assert_eq!(original.vars.usage_vars().0, output.vars.usage_vars().0);
    for key in original.vars.usage_vars().0 {
        assert_eq!(
            original.vars.resolve(&key).unwrap(),
            output.vars.resolve(&key).unwrap(),
            "{key}"
        );
    }
}

#[test]
fn pelorus_default_survives_serde_with_all_source_metadata() {
    let source = include_str!("../tests/fixtures/pelorus_default.rl");
    let mut document: Document<Value> = from_str(source).unwrap();
    let output = to_string(&document).unwrap();
    assert_eq!(
        output, source,
        "comments, annotations, and formatting must be byte-identical"
    );
    assert_same_values(source, &output);
    // Also exercise actual serde serialization of every value, independently of
    // the retained source, including open ranges and value/namespace overlaps.
    let canonical = to_string(&document.value).unwrap();
    assert_same_values(source, &canonical);
    let decoded: Value = from_str(&canonical).unwrap();
    assert_same_values(source, &to_string(&decoded).unwrap());

    let Value::Map(root) = &mut document.value else {
        panic!()
    };
    let Value::Map(registration) = root.get_mut("registration").unwrap() else {
        panic!()
    };
    let Value::Map(map) = registration.get_mut("map").unwrap() else {
        panic!()
    };
    let Value::Map(resolution) = map.get_mut("resolution").unwrap() else {
        panic!()
    };
    resolution.insert(
        "$value".into(),
        Value::literal(Val::UnitedVal(UnitVal::from_length(Length::new::<
            crate::centimeter,
        >(25.)))),
    );
    let edited = to_string(&document).unwrap();
    assert!(edited.starts_with(source));
    assert!(edited.contains("# @recommended\nregistration.map.resolution = 25cm\n"));
    let original = crate::compile_code(source).unwrap();
    let edited = crate::compile_code(&edited).unwrap();
    assert_eq!(original.vars.usage_vars().0.len(), 143);
    for key in original.vars.usage_vars().0 {
        if key != "registration.map.resolution" {
            assert_eq!(
                original.vars.resolve(&key).unwrap(),
                edited.vars.resolve(&key).unwrap(),
                "{key}"
            );
        }
    }
    assert_eq!(
        edited.vars.resolve("registration.map.resolution").unwrap(),
        Some(Rhs::Val(Val::UnitedVal(UnitVal::from_length(
            Length::new::<crate::centimeter>(25.)
        ))))
    );
}

#[test]
fn from_evaluated_uses_values_instead_of_recompiling_source() {
    let mut evaluated =
        crate::compile_code("distance = 1mm\ndistance = 2mm\ntimeout = 20ms\n").unwrap();
    // The source is intentionally unusable. Deserialization must visit the AST.
    evaluated.source_text = "not valid ratslang and no files to read".into();
    let first: Sensor = crate::from_evaluated(&evaluated).unwrap();
    let repeated: Sensor = from_evaluated(&evaluated).unwrap();
    assert_eq!(first, repeated);
    assert_eq!(first.distance, Length::new::<crate::millimeter>(2.));
    assert_eq!(first.timeout, Time::new::<millisecond>(20.));
    assert!(
        from_evaluated::<Sensor>(&crate::compile_code("distance = 1s\ntimeout = 20ms").unwrap())
            .is_err()
    );
}

#[test]
fn from_evaluated_tracks_only_fields_serde_reads() {
    #[derive(Deserialize)]
    struct Config {
        section: Section,
    }
    #[derive(Deserialize)]
    struct Section {
        read: i32,
        optional: Option<i32>,
    }
    let evaluated = crate::compile_code(
        "section.read = 7\nsection.ignored = 20\nunused.child = 1\nunknown = 3\n",
    )
    .unwrap();
    assert!(evaluated.vars.usage_vars().1.is_empty());
    let config: Config = from_evaluated(&evaluated).unwrap();
    assert_eq!(config.section.read, 7);
    assert_eq!(config.section.optional, None);
    assert_eq!(evaluated.vars.usage_vars().1, vec!["section.read"]);

    #[derive(Debug, Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Strict {
        #[allow(dead_code)]
        count: i32,
    }
    let evaluated = crate::compile_code("count = 1\ntypo = 2").unwrap();
    assert!(
        from_evaluated::<Strict>(&evaluated)
            .unwrap_err()
            .to_string()
            .contains("unknown field")
    );
    assert_eq!(evaluated.vars.usage_vars().1, vec!["count"]);
}

#[test]
fn from_evaluated_keeps_initial_state() {
    let initial = std::collections::HashMap::from([(
        "distance".parse::<Var>().unwrap(),
        Rhs::Val(Val::UnitedVal(UnitVal::from_length(Length::new::<meter>(
            3.,
        )))),
    )]);
    let evaluated = crate::compile_code_with_state(
        "timeout = 20ms\n",
        &std::env::current_dir().unwrap(),
        Some(initial),
        std::io::sink(),
        false,
    )
    .unwrap();
    let config: Sensor = from_evaluated(&evaluated).unwrap();
    assert_eq!(config.distance, Length::new::<meter>(3.));
    assert_eq!(config.timeout, Time::new::<millisecond>(20.));
}

#[test]
fn namespace_defaults_fall_back_per_field_and_keep_usage_separate() {
    #[derive(Debug, Deserialize, PartialEq)]
    struct Settings {
        count: usize,
        enabled: bool,
    }
    let defaults =
        crate::compile_code("template.count = 3\ntemplate.enabled = true\ntemplate.ignored = 1\n")
            .unwrap();
    let user = crate::compile_code("cameras.front.count = 5\ncameras.front.typo = 2\n").unwrap();
    let settings: Settings = from_variables_with_defaults(
        &defaults.vars.filter_ns(&["template"]),
        &user.vars.filter_ns(&["cameras", "front"]),
    )
    .unwrap();
    assert_eq!(
        settings,
        Settings {
            count: 5,
            enabled: true
        }
    );
    assert_eq!(defaults.vars.usage_vars().1, vec!["template.enabled"]);
    assert_eq!(
        user.vars.usage_vars().1,
        vec!["cameras.front.count", "cameras.front.enabled"]
    );
}

#[test]
fn from_evaluated_does_not_reopen_includes_and_retains_document_source() {
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let directory =
        std::env::temp_dir().join(format!("ratslang-serde-{}-{unique}", std::process::id()));
    std::fs::create_dir(&directory).unwrap();
    let include = directory.join("included.rl");
    std::fs::write(&include, "distance = 2mm\ntimeout = 20ms\n").unwrap();
    let source = "# original comment\n= ./included.rl\n";
    let evaluated =
        crate::compile_code_with_state(source, &directory, None, std::io::sink(), false).unwrap();
    std::fs::remove_file(&include).unwrap();
    std::fs::remove_dir(&directory).unwrap();
    let config: Sensor = from_evaluated(&evaluated).unwrap();
    assert_eq!(config.distance, Length::new::<crate::millimeter>(2.));
    let document: Document<Sensor> = from_evaluated(&evaluated).unwrap();
    assert_eq!(document.value, config);
    assert_eq!(to_string(&document).unwrap(), source);
}

#[test]
fn annotated_enums_deserialize_bare_strings_and_paths() {
    #[derive(Debug, Deserialize, PartialEq)]
    enum Backend {
        #[serde(rename = "CPU")]
        Cpu,
        #[serde(rename = "Backend")]
        Named,
        #[serde(rename = "./cpu")]
        CpuPath,
    }
    #[derive(Deserialize)]
    struct Settings {
        backend: Backend,
    }
    for (source, expected) in [
        ("# @enum CPU, Backend\nbackend = CPU", Backend::Cpu),
        (
            "# @enum CPU, Backend\nbackend = \"Backend\"",
            Backend::Named,
        ),
        (
            "# @enum ./cpu, ./backend\nbackend = ./cpu",
            Backend::CpuPath,
        ),
    ] {
        assert_eq!(from_str::<Settings>(source).unwrap().backend, expected);
    }
    assert!(from_str::<Settings>("# @enum CPU, Backend\nbackend = Other").is_err());
}

#[test]
fn serde_defaults_enforce_constraints_on_user_overrides() {
    #[derive(Debug, Deserialize)]
    struct Settings {
        count: u32,
    }
    let defaults = crate::compile_code(
        "# @min 1\n# @max 10\ncount = 3\n# @enum CPU, Backend\nignored = CPU\n",
    )
    .unwrap();
    let user = crate::compile_code("# @max 20\ncount = 15\nignored = Other\n").unwrap();
    let settings: Settings = from_variables_with_defaults(&defaults.vars, &user.vars).unwrap();
    assert_eq!(settings.count, 15);
    assert_eq!(user.vars.usage_vars().1, vec!["count"]);
    assert!(defaults.vars.usage_vars().1.is_empty());
    for value in [0, 11] {
        let user = crate::compile_code(&format!("count = {value}\n")).unwrap();
        let error =
            from_variables_with_defaults::<Settings>(&defaults.vars, &user.vars).unwrap_err();
        assert!(error.to_string().contains("count"));
    }
}

#[test]
fn document_preserves_and_enforces_constraints_after_edits() {
    #[derive(Serialize, Deserialize)]
    struct Settings {
        count: u32,
    }
    let source = "# @recommended\n# @type integer\n# @min 1\n# @max 10\ncount = 3 # keep me\n";
    let mut document: Document<Settings> = from_str(source).unwrap();
    assert_eq!(to_string(&document).unwrap(), source);
    document.value.count = 8;
    let output = to_string(&document).unwrap();
    assert!(output.starts_with(source));
    let compiled = crate::compile_code(&output).unwrap();
    assert!(compiled.vars.constraints().unwrap().contains_key("count"));
    let selected = compiled.to_string_filtered("recommended").unwrap();
    assert_eq!(from_str::<Settings>(&selected).unwrap().count, 8);
    document.value.count = 11;
    assert!(
        to_string(&document)
            .unwrap_err()
            .to_string()
            .contains("count")
    );
}

#[test]
fn config_reader_preserves_usage_constraints_and_scalar_children() {
    let defaults = crate::compile_code("# @min 1\nsection.count = 3\nsection.enabled = true\nsection.enabled.extra = 5\nsection.ignored = 7\n").unwrap();
    let user = crate::compile_code("section.count = 4\nsection.typo = 9\n").unwrap();
    let reader = ConfigReader::new(&defaults.vars, &user.vars).unwrap();
    assert!(defaults.vars.usage_vars().1.is_empty());
    assert!(user.vars.usage_vars().1.is_empty());
    assert_eq!(reader.deserialize::<usize>("section.count").unwrap(), 4);
    assert!(reader.deserialize::<bool>("section.enabled").unwrap());
    assert_eq!(
        reader
            .deserialize::<usize>("section.enabled.extra")
            .unwrap(),
        5
    );
    assert_eq!(
        reader
            .deserialize::<Option<f64>>("section.optional")
            .unwrap(),
        None
    );
    assert!(reader.deserialize::<usize>("section.optional").is_err());
    assert!(!user.vars.usage_vars().1.contains(&"section.typo".into()));
    assert!(
        user.vars
            .usage_vars()
            .1
            .contains(&"section.optional".into())
    );
    let invalid = crate::compile_code("section.count = 0\n").unwrap();
    assert!(
        ConfigReader::new(&defaults.vars, &invalid.vars)
            .unwrap()
            .deserialize::<usize>("section.count")
            .is_err()
    );
}

#[test]
fn typed_f32_values_reject_overflow_during_deserialization() {
    #[derive(Debug, Deserialize)]
    struct Settings {
        value: f32,
    }
    let max = Val::NumVal(NumVal::Floating(f32::MAX as f64))
        .to_source()
        .unwrap();
    assert_eq!(
        from_str::<Settings>(&format!("value = {max}\n"))
            .unwrap()
            .value,
        f32::MAX
    );
    let overflow = Val::NumVal(NumVal::Floating(f32::MAX as f64 * 2.))
        .to_source()
        .unwrap();
    assert!(from_str::<Settings>(&format!("value = {overflow}\n")).is_err());
}

#[test]
fn quantity_wrappers_cover_collections_optional_values_matrices_and_open_ranges() {
    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct Config {
        lengths: Vec<Quantity<Length>>,
        matrix: [[Quantity<Length>; 2]; 2],
        timeout: Option<Quantity<Time>>,
        range: Range<Quantity<Length>>,
        times: Range<Quantity<Time>>,
    }
    let source = "lengths = [1um, 2cm]\nmatrix = [[1m, 2m], [3m, 4m]]\ntimeout = 5ms\nrange = ..2m\ntimes = 1ms..\n";
    let config: Config = from_str(source).unwrap();
    assert_eq!(config.matrix[1][0].get::<meter>(), 3.0);
    assert_eq!(
        config.timeout.unwrap().into_inner(),
        Time::new::<crate::millisecond>(5.0)
    );
    assert_eq!(
        from_str::<Config>(&to_string(&config).unwrap()).unwrap(),
        config
    );
    let no_timeout: Config = from_str(&source.replace("timeout = 5ms\n", "")).unwrap();
    assert!(no_timeout.timeout.is_none());
    assert!(!to_string(&no_timeout).unwrap().contains("timeout"));
    for bad in [
        source.replace("1um", "1s"),
        source.replace("5ms", "5"),
        source.replace("..2m", "..2s"),
    ] {
        assert!(from_str::<Config>(&bad).is_err(), "{bad}");
    }
}

#[test]
fn case_insensitive_derived_enums_keep_aliases_canonical_output_and_errors() {
    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    enum Backend {
        #[serde(rename = "CPU", alias = "host", alias = "single_core")]
        Cpu,
        #[serde(rename = "Backend")]
        Accelerator,
    }
    #[derive(Debug, Serialize, Deserialize)]
    struct Config {
        #[serde(with = "case_insensitive")]
        backend: Backend,
    }
    for spelling in ["cpu", "CpU", "HOST", "Single_Core", "\"hOsT\""] {
        let config: Config = from_str(&format!("backend = {spelling}\n")).unwrap();
        assert_eq!(config.backend, Backend::Cpu);
        assert_eq!(to_string(&config).unwrap(), "backend = \"CPU\"\n");
    }
    let error = from_str::<Config>("backend = typo\n")
        .unwrap_err()
        .to_string();
    assert!(error.contains("typo") && error.contains("CPU"), "{error}");
    let config: Config = from_str("backend = BACKEND\n").unwrap();
    assert_eq!(to_string(&config).unwrap(), "backend = \"Backend\"\n");
}

#[test]
fn required_user_fields_are_not_satisfied_by_defaults() {
    #[derive(Debug, Deserialize)]
    struct Settings {
        name: String,
    }
    let defaults = crate::compile_code("# @required\nname = \"\"\n").unwrap();
    let user = crate::compile_code("").unwrap();
    assert!(
        from_variables_with_defaults::<Settings>(&defaults.vars, &user.vars)
            .unwrap_err()
            .to_string()
            .contains("required")
    );
    let user = crate::compile_code("name = \"robot\"\n").unwrap();
    assert_eq!(
        from_variables_with_defaults::<Settings>(&defaults.vars, &user.vars)
            .unwrap()
            .name,
        "robot"
    );
}
#[test]
fn all_physical_dimensions_roundtrip_in_wrappers_arrays_and_ranges() {
    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct Settings {
        angle: Quantity<crate::Angle>,
        frequency: Quantity<crate::Frequency>,
        velocity: [Quantity<crate::Velocity>; 2],
        acceleration: Range<Quantity<crate::Acceleration>>,
    }
    let source = "# @type angle\n# @min 0deg\n# @max 180deg\nangle = 90deg\n# @type frequency\nfrequency = 2kHz\nvelocity = [36km/h, 5cm/s]\nacceleration = 1m/s2..2m/s^2\n";
    let config: Settings = from_str(source).unwrap();
    assert!((config.angle.get::<crate::degree>() - 90.).abs() < 1e-10);
    assert_eq!(config.frequency.get::<crate::hertz>(), 2000.);
    assert_eq!(config.velocity[0].get::<crate::meter_per_second>(), 10.);
    assert_eq!(
        from_str::<Settings>(&to_string(&config).unwrap()).unwrap(),
        config
    );
    for invalid in [
        source.replace("90deg", "200deg"),
        source.replace("2kHz", "2m"),
        source.replace("36km/h", "36m/s2"),
        source.replace("2m/s^2", "2m/s"),
    ] {
        assert!(from_str::<Settings>(&invalid).is_err(), "{invalid}");
    }
}

#[test]
fn mass_serde_wrappers_direct_fields_collections_and_ranges_roundtrip() {
    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct Settings {
        #[serde(with = "mass")]
        payload: crate::Mass,
        samples: Vec<Quantity<crate::Mass>>,
        limits: Range<Quantity<crate::Mass>>,
    }
    let settings: Settings =
        from_str("payload = 2kg\nsamples = [1g, 2mg, 3ug]\nlimits = 500g..1t\n").unwrap();
    assert_eq!(settings.payload.get::<crate::kilogram>(), 2.);
    assert_eq!(
        from_str::<Settings>(&to_string(&settings).unwrap()).unwrap(),
        settings
    );
    assert!(from_str::<Settings>("payload = 2m\nsamples = [1g]\nlimits = ..\n").is_err());
    use ::serde::Deserialize;
    let invalid = Value::Rhs(Rhs::Val(Val::NumVal(NumVal::Floating(f64::INFINITY))));
    assert!(f64::deserialize(invalid).is_err());
}
