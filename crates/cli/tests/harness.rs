use assert_cmd::cargo::cargo_bin_cmd;
use assert_fs::TempDir;
use serde_json::Value;

#[test]
fn harness_json_lifecycle_and_errors() {
    let temp = TempDir::new().unwrap();
    let run = |args: &[&str]| {
        cargo_bin_cmd!("unelos")
            .env("UNELOS_CONFIG_DIR", temp.path().join("unelos"))
            .env("OPENCODE_CONFIG_DIR", temp.path().join("opencode"))
            .env("PATH", temp.path())
            .env("HOME", temp.path())
            .env("XDG_CONFIG_HOME", temp.path())
            .args(args)
            .arg("--json")
            .output()
            .unwrap()
    };
    let list = run(&["harness", "list"]);
    assert!(list.status.success(), "{list:?}");
    let value: Value = serde_json::from_slice(&list.stdout).unwrap();
    assert_eq!(value.as_array().unwrap().len(), 12);
    assert_eq!(value[0]["id"], "opencode");
    let installed = run(&["harness", "install", "opencode"]);
    assert!(installed.status.success(), "{installed:?}");
    let value: Value = serde_json::from_slice(&installed.stdout).unwrap();
    assert_eq!(value[0]["installed"], true);
    assert!(run(&["harness", "install", "opencode"]).status.success());
    let removed = run(&["harness", "uninstall", "opencode"]);
    assert!(removed.status.success());
    let value: Value = serde_json::from_slice(&removed.stdout).unwrap();
    assert_eq!(value[0]["installed"], false);
    for args in [
        ["harness", "install", "unknown"],
        ["harness", "install", "claude"],
        ["harness", "models", "opencode"],
    ] {
        let output = run(&args);
        assert!(!output.status.success());
        let error: Value = serde_json::from_slice(&output.stderr).unwrap();
        assert!(error["error"]["code"].is_string());
    }
}
