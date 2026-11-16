use assert_cmd::cargo::cargo_bin_cmd;
use assert_fs::TempDir;
use serde_json::Value;
use std::{fs, path::Path, process::Output};

fn write(root: &Path, name: &str) {
    let path = root.join(name);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, b"keep").unwrap();
}

fn uninstall(root: &Path, args: &[&str]) -> Output {
    let home = TempDir::new().unwrap();
    cargo_bin_cmd!("unelos")
        .env("HOME", home.path())
        .env("XDG_CONFIG_HOME", home.path())
        .arg("--project")
        .arg(root)
        .arg("uninstall")
        .args(args)
        .output()
        .unwrap()
}

#[test]
fn yes_removes_only_selected_project() {
    let target = TempDir::new().unwrap();
    let other = TempDir::new().unwrap();
    for root in [target.path(), other.path()] {
        for name in [
            ".unelos/broken.yaml",
            ".claude/agents/unelos-stale.md",
            "source.txt",
            ".claude/agents/mine.md",
        ] {
            write(root, name);
        }
    }
    let output = uninstall(target.path(), &["--yes"]);
    assert!(output.status.success(), "{output:?}");
    assert!(!target.path().join(".unelos").exists());
    assert!(!target
        .path()
        .join(".claude/agents/unelos-stale.md")
        .exists());
    assert_eq!(fs::read(target.path().join("source.txt")).unwrap(), b"keep");
    assert!(target.path().join(".claude/agents/mine.md").exists());
    assert!(other.path().join(".unelos/broken.yaml").exists());
    assert!(other.path().join(".claude/agents/unelos-stale.md").exists());
}

#[test]
fn json_requires_yes_without_mutation() {
    let target = TempDir::new().unwrap();
    write(target.path(), ".unelos/data");
    let output = uninstall(target.path(), &["--json"]);
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    let value: Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(value["error"]["code"], "uninstall_confirmation_required");
    assert!(target.path().join(".unelos/data").exists());
}

#[test]
fn noninteractive_without_yes_refuses() {
    let target = TempDir::new().unwrap();
    write(target.path(), ".unelos/data");
    let output = uninstall(target.path(), &[]);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("uninstall_confirmation_required"));
    assert!(target.path().join(".unelos/data").exists());
}

#[test]
fn yes_json_reports_removed_paths() {
    let target = TempDir::new().unwrap();
    write(target.path(), ".unelos/data");
    let output = uninstall(target.path(), &["--yes", "--json"]);
    assert!(output.status.success(), "{output:?}");
    assert!(output.stderr.is_empty());
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        value["removed"],
        serde_json::json!([target.path().canonicalize().unwrap().join(".unelos")])
    );
}

#[test]
fn repeat_is_noop() {
    let target = TempDir::new().unwrap();
    write(target.path(), ".unelos/data");
    assert!(uninstall(target.path(), &["--yes"]).status.success());
    let output = uninstall(target.path(), &["--yes", "--json"]);
    assert!(output.status.success(), "{output:?}");
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["removed"], serde_json::json!([]));
}

#[test]
fn unsafe_target_is_json_error() {
    let target = TempDir::new().unwrap();
    write(target.path(), ".unelos/data");
    write(target.path(), ".claude/agents/unelos-wrong.md/data");
    let output = uninstall(target.path(), &["--yes", "--json"]);
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    let value: Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(value["error"]["code"], "uninstall_failed");
    assert!(target.path().join(".unelos/data").exists());
}
