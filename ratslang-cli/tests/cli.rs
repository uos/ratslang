use std::{
    path::PathBuf,
    process::{Command, Output},
};
fn run(arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_ratslang"))
        .args(arguments)
        .output()
        .unwrap()
}
fn fixture(label: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!("ratslang-cli-{label}-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    root
}
#[test]
fn cli_checks_generates_explains_and_migrates_without_writing_by_default() {
    let root = fixture("workflow");
    let schema = root.join("default.rl");
    let user = root.join("user.rl");
    std::fs::write(&schema, "# Device name\n# @required\nname = \"\"\n# @renamed_to sensor.distance\nold = 1m\nsensor.distance = 1m\n").unwrap();
    std::fs::write(
        &user,
        "# @schema ./default.rl\nname = \"robot\"\nold = 25cm # Keep\n",
    )
    .unwrap();
    let path = user.to_str().unwrap();
    assert!(run(&["check", path]).status.success());
    assert!(!run(&["check", path, "--strict"]).status.success());
    let explain = run(&["explain", path, "old", "--set", "old=30cm"]);
    assert!(
        explain.status.success(),
        "{}",
        String::from_utf8_lossy(&explain.stderr)
    );
    assert!(String::from_utf8_lossy(&explain.stdout).contains("Winner [CLI]"));
    assert!(String::from_utf8_lossy(&explain.stdout).contains("Assignment history"));
    let before = std::fs::read_to_string(&user).unwrap();
    let migration = run(&["migrate", path]);
    assert!(
        migration.status.success(),
        "{}",
        String::from_utf8_lossy(&migration.stderr)
    );
    assert!(String::from_utf8_lossy(&migration.stdout).contains("sensor.distance = 25cm"));
    assert_eq!(std::fs::read_to_string(&user).unwrap(), before);
    assert!(run(&["migrate", path, "--write"]).status.success());
    assert!(run(&["check", path, "--strict"]).status.success());
    let generated = run(&["generate", schema.to_str().unwrap()]);
    assert!(generated.status.success());
    assert!(String::from_utf8_lossy(&generated.stdout).starts_with("# @schema "));
    assert!(String::from_utf8_lossy(&generated.stdout).contains("# Device name"));
    assert!(!String::from_utf8_lossy(&generated.stdout).contains("old ="));
    assert!(!String::from_utf8_lossy(&generated.stdout).contains("@required"));
    let url = "https://example.org/releases/download/v1.0.0/default.rl";
    let released = run(&["generate", schema.to_str().unwrap(), "--schema-url", url]);
    assert!(released.status.success());
    assert!(String::from_utf8_lossy(&released.stdout).starts_with(&format!("# @schema {url}\n")));
    assert!(!run(&["check", path, "--schema-url", url]).status.success());
    std::fs::write(&user, "# @schema ./default.rl\n").unwrap();
    assert!(!run(&["check", path]).status.success());
    assert!(
        run(&["check", path, "--set", "name=\"robot\""])
            .status
            .success()
    );
    std::fs::remove_dir_all(root).unwrap();
}
#[test]
fn cli_formats_without_fetching_schemas_and_returns_errors_for_bad_input() {
    let root = fixture("format");
    let user = root.join("user.rl");
    std::fs::write(
        &user,
        "# @schema https://example.org/default.rl\nsensor {\nx=1 # Keep\n}\n",
    )
    .unwrap();
    let output = run(&["fmt", user.to_str().unwrap(), "--write"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        std::fs::read_to_string(&user)
            .unwrap()
            .contains("  x = 1 # Keep")
    );
    assert!(!run(&["unknown", user.to_str().unwrap()]).status.success());
    assert!(
        !run(&["check", user.to_str().unwrap(), "--set", "= ./include.rl"])
            .status
            .success()
    );
    assert!(run(&["--help"]).status.success());
    assert!(String::from_utf8_lossy(&run(&["--version"]).stdout).contains("ratslang"));
    std::fs::remove_dir_all(root).unwrap();
}
