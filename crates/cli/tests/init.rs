use assert_cmd::cargo::cargo_bin_cmd;
use assert_fs::TempDir;
use serde_json::Value;
use std::{fs, path::Path, process::Output};

fn init(root: &Path) -> Output {
    cargo_bin_cmd!("unelos")
        .arg("--project")
        .arg(root)
        .args(["init", "--json"])
        .output()
        .unwrap()
}
fn write(root: &Path, name: &str, text: &str) {
    let path = root.join(name);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

#[test]
fn scaffold_only_and_idempotent() {
    let project = TempDir::new().unwrap();
    let output = init(project.path());
    assert!(output.status.success(), "{output:?}");
    let summary: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(summary["migrated"], false);
    assert!(project.path().join(".unelos/project.yaml").exists());
    assert!(!project.path().join("opencode.json").exists());
    assert!(!project.path().join(".unelos/opencode").exists());
    assert!(init(project.path()).status.success());
    cargo_bin_cmd!("unelos")
        .arg("--project")
        .arg(project.path())
        .args(["init", "--opencode", "v2"])
        .assert()
        .failure();
}

#[test]
fn migrates_only_our_registration_and_assets() {
    for (key, entry) in [
        (
            "plugins",
            "{\"package\":\"./.unelos/opencode/unelos\",\"options\":{\"binary\":\"old\"}}",
        ),
        ("plugin", "[\"./.unelos/opencode/plugin/index.js\",{}]"),
    ] {
        let project = TempDir::new().unwrap();
        let root = project.path();
        write(
            root,
            "opencode.jsonc",
            &format!("{{\n // keep\n \"{key}\":[\"other\",{entry}]\n}}"),
        );
        write(root, ".unelos/opencode/unelos/index.js", "old");
        write(root, ".unelos/opencode/unelos/custom.txt", "keep");
        write(root, ".unelos/integration.json", "{\"opencode\":\"v2\"}");
        write(root, ".opencode/commands/unelos.md", "custom");
        let output = init(root);
        assert!(output.status.success(), "{output:?}");
        let summary: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(summary["migrated"], true);
        assert!(summary["message"]
            .as_str()
            .unwrap()
            .contains("unelos harness install opencode"));
        let config = fs::read_to_string(root.join("opencode.jsonc")).unwrap();
        assert!(config.contains("// keep"));
        assert!(config.contains("other"));
        assert!(!config.contains(".unelos/opencode"));
        assert!(!root.join(".unelos/opencode/unelos/index.js").exists());
        assert!(!root.join(".unelos/integration.json").exists());
        assert_eq!(
            fs::read_to_string(root.join(".unelos/opencode/unelos/custom.txt")).unwrap(),
            "keep"
        );
        assert_eq!(
            fs::read_to_string(root.join(".opencode/commands/unelos.md")).unwrap(),
            "custom"
        );
        assert!(init(root).status.success());
        assert_eq!(
            fs::read_to_string(root.join("opencode.jsonc")).unwrap(),
            config
        );
    }
}

#[test]
fn malformed_config_fails_before_scaffold() {
    for text in [
        "[]",
        "{\"plugins\":false}",
        "{\"plugins\":[],\"plugins\":[]}",
    ] {
        let project = TempDir::new().unwrap();
        write(project.path(), "opencode.json", text);
        assert!(!init(project.path()).status.success());
        assert!(!project.path().join(".unelos/project.yaml").exists());
        assert_eq!(
            fs::read_to_string(project.path().join("opencode.json")).unwrap(),
            text
        );
    }
}
