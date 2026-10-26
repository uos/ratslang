use super::*;
#[test]
fn assignment_history_tracks_old_reference_values_and_does_not_mark_reads() {
    let config = compile_code("# µ\nx = 1\nx = 2\ny = [x, x]\nx = 3\n").unwrap();
    let before = config.vars.usage_vars();
    let explanation = config.vars.explain("y").unwrap().unwrap();
    assert_eq!(explanation.dependencies.len(), 1);
    assert_eq!(explanation.dependencies[0].value.to_source().unwrap(), "2");
    assert_eq!(
        explanation.dependencies[0].location.as_ref().unwrap().line,
        3
    );
    assert_eq!(explanation.value.to_source().unwrap(), "[2, 2]");
    let x = config.vars.explain("x").unwrap().unwrap();
    assert_eq!(x.history.len(), 3);
    assert_eq!(x.winner.location.unwrap().line, 5);
    assert_eq!(config.vars.usage_vars(), before);
    assert!(config.vars.explain("missing").unwrap().is_none());
}
#[test]
fn includes_namespaces_multia_assignments_and_filtered_views_keep_real_origins() {
    let folder = std::env::temp_dir().join(format!("ratslang-provenance-{}", std::process::id()));
    std::fs::create_dir_all(&folder).unwrap();
    std::fs::write(
        folder.join("inner.rl"),
        "# inner\na, b = 1m, 2s # EOF without newline",
    )
    .unwrap();
    std::fs::write(folder.join("outer.rl"), "nested {\n= ./inner.rl\n}\n").unwrap();
    let root = folder.join("root.rl");
    std::fs::write(&root, "sensor {\n= ./outer.rl\n}\ncopy = sensor.nested.a\n").unwrap();
    let config = compile_file(&root, None, None).unwrap();
    let explanation = config
        .vars
        .filter_ns(&["sensor", "nested"])
        .explain("a")
        .unwrap()
        .unwrap();
    assert_eq!(explanation.name, "sensor.nested.a");
    assert_eq!(explanation.winner.included_from.len(), 2);
    let source = explanation.winner.location.unwrap();
    assert_eq!(source.file, Some(folder.join("inner.rl")));
    assert_eq!(source.line, 2);
    assert_eq!(
        &std::fs::read_to_string(folder.join("inner.rl")).unwrap()[source.bytes],
        "a, b = 1m, 2s"
    );
    let copy = config.vars.explain("copy").unwrap().unwrap();
    assert_eq!(copy.dependencies[0].name, "sensor.nested.a");
    std::fs::remove_dir_all(folder).unwrap();
}
#[test]
fn file_slices_and_crlf_have_original_coordinates_and_initial_state_is_identified() {
    let path = std::env::temp_dir().join(format!("ratslang-coordinates-{}.rl", std::process::id()));
    let source = "# µ\r\nignored = 1\r\n  chosen = 2\r\n";
    std::fs::write(&path, source).unwrap();
    let result = compile_file(&path, Some(3), Some(3)).unwrap();
    let location = result
        .vars
        .explain("chosen")
        .unwrap()
        .unwrap()
        .winner
        .location
        .unwrap();
    assert_eq!((location.line, location.column), (3, 3));
    assert_eq!(&source[location.bytes], "chosen = 2");
    std::fs::remove_file(&path).unwrap();
    let result = compile_code_with_state(
        "y = x\n",
        Path::new("."),
        Some(HashMap::from([(
            "x".parse().unwrap(),
            Rhs::Val(Val::NumVal(NumVal::Integer(7))),
        )])),
        std::io::sink(),
        false,
    )
    .unwrap();
    assert!(result.vars.explain("y").unwrap().unwrap().dependencies[0].initial_state);
    assert!(
        result
            .vars
            .explain("x")
            .unwrap()
            .unwrap()
            .winner
            .initial_state
    );
}
use std::path::Path;
