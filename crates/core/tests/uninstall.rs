use camino::Utf8Path;
use std::fs;
use unelos_core::uninstall_project;

fn put(root: &Utf8Path, path: &str, text: &str) {
    let path = root.join(path);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

#[test]
fn removes_both_harnesses_and_broken_authoring() {
    let temp = tempfile::tempdir().unwrap();
    let root = Utf8Path::from_path(temp.path()).unwrap();
    for path in [
        ".unelos/workflows/broken.yaml",
        ".opencode/agents/unelos-stale.md",
        ".claude/agents/unelos-edited.md",
        ".claude/agents/mine.md",
        "source.txt",
    ] {
        put(root, path, "keep");
    }
    let result = uninstall_project(root).unwrap();
    assert_eq!(result.removed.len(), 3);
    assert!(!root.join(".unelos").exists());
    assert!(!root.join(".opencode/agents/unelos-stale.md").exists());
    assert!(!root.join(".claude/agents/unelos-edited.md").exists());
    assert_eq!(
        fs::read(root.join(".claude/agents/mine.md")).unwrap(),
        b"keep"
    );
    assert_eq!(fs::read(root.join("source.txt")).unwrap(), b"keep");
    assert!(uninstall_project(root).unwrap().removed.is_empty());
}

#[test]
fn cleans_orphan_agents_without_unelos() {
    let temp = tempfile::tempdir().unwrap();
    let root = Utf8Path::from_path(temp.path()).unwrap();
    put(root, ".claude/agents/unelos-old.md", "");
    assert_eq!(uninstall_project(root).unwrap().removed.len(), 1);
    assert!(!root.join(".unelos").exists());
}

#[test]
fn ignores_unowned_config() {
    for state in [None, Some("{}"), Some(r#"{"empty":{}}"#)] {
        for kind in 0..4 {
            let temp = tempfile::tempdir().unwrap();
            let root = Utf8Path::from_path(temp.path()).unwrap();
            put(root, ".unelos/sentinel", "keep");
            if let Some(state) = state {
                put(root, ".unelos/opencode-mcps.json", state);
            }
            match kind {
                0 => put(root, "opencode.json", "broken"),
                1 => {
                    put(root, "opencode.json", "broken");
                    put(root, "opencode.jsonc", "broken");
                }
                2 => fs::create_dir(root.join("opencode.json")).unwrap(),
                _ => {
                    #[cfg(unix)]
                    std::os::unix::fs::symlink("missing", root.join("opencode.json")).unwrap();
                }
            }
            uninstall_project(root).unwrap();
            assert!(!root.join(".unelos").exists());
            if kind < 2 {
                assert_eq!(fs::read(root.join("opencode.json")).unwrap(), b"broken");
            }
            if kind == 2 {
                assert!(root.join("opencode.json").is_dir());
            }
        }
    }
}

const STATE: &str = r#"{"workflow":{"owned":{"type":"local","command":["owned"]},"edited":{"type":"local","command":["old"]}}}"#;

#[test]
fn cleans_owned_mcps_preserving_user_edits() {
    let temp = tempfile::tempdir().unwrap();
    let root = Utf8Path::from_path(temp.path()).unwrap();
    put(root, ".unelos/opencode-mcps.json", STATE);
    put(
        root,
        "opencode.json",
        r#"{"mcp":{"owned":{"type":"local","command":["owned"],"environment":{},"enabled":true},"edited":{"type":"local","command":["new"],"environment":{},"enabled":true}},"tools":{"owned_*":false,"edited_*":true}}"#,
    );
    uninstall_project(root).unwrap();
    let value: serde_json::Value =
        serde_json::from_slice(&fs::read(root.join("opencode.json")).unwrap()).unwrap();
    assert!(value["mcp"].get("owned").is_none());
    assert_eq!(value["mcp"]["edited"]["command"][0], "new");
    assert_eq!(value["tools"]["edited_*"], true);
}

#[test]
fn does_not_recreate_missing_config() {
    let temp = tempfile::tempdir().unwrap();
    let root = Utf8Path::from_path(temp.path()).unwrap();
    put(root, ".unelos/opencode-mcps.json", STATE);
    uninstall_project(root).unwrap();
    assert!(!root.join("opencode.json").exists());
}

#[test]
fn rejects_invalid_ownership_before_deleting() {
    for state in ["broken", "[]"] {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8Path::from_path(temp.path()).unwrap();
        put(root, ".unelos/opencode-mcps.json", state);
        put(root, ".opencode/agents/unelos-good.md", "keep");
        assert!(uninstall_project(root).is_err());
        assert!(root.join(".opencode/agents/unelos-good.md").exists());
    }
}

#[cfg(unix)]
#[test]
fn refuses_nonregular_ownership_and_owned_config_without_reading() {
    for path in [".unelos/opencode-mcps.json", "opencode.json"] {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8Path::from_path(temp.path()).unwrap();
        put(root, ".unelos/opencode-mcps.json", STATE);
        put(root, ".claude/agents/unelos-good.md", "keep");
        let target = root.join(path);
        if target.exists() {
            fs::remove_file(&target).unwrap();
        }
        fs::create_dir(&target).unwrap();
        let error = uninstall_project(root).unwrap_err();
        assert_eq!(error.code, "invalid_path");
        assert!(root.join(".claude/agents/unelos-good.md").exists());
    }
}

#[test]
fn nonempty_ownership_validates_config_before_deleting() {
    for ambiguous in [false, true] {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8Path::from_path(temp.path()).unwrap();
        put(root, ".unelos/opencode-mcps.json", STATE);
        put(root, ".opencode/agents/unelos-good.md", "keep");
        put(root, "opencode.json", "broken");
        if ambiguous {
            put(root, "opencode.jsonc", "{}");
        }
        assert!(uninstall_project(root).is_err());
        assert!(root.join(".opencode/agents/unelos-good.md").exists());
        assert!(root.join(".unelos").exists());
    }
}

#[cfg(unix)]
#[test]
fn preflights_all_targets_before_deleting() {
    for path in [
        ".unelos",
        ".opencode",
        ".opencode/agents",
        ".claude",
        ".claude/agents",
        ".claude/agents/unelos-bad.md",
    ] {
        let temp = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let root = Utf8Path::from_path(temp.path()).unwrap();
        put(root, ".opencode/agents/unelos-good.md", "keep");
        let target = root.join(path);
        if target.exists() {
            fs::remove_dir_all(&target).unwrap();
        }
        fs::create_dir_all(target.parent().unwrap()).unwrap();
        std::os::unix::fs::symlink(outside.path(), &target).unwrap();
        fs::write(outside.path().join("sentinel"), "keep").unwrap();
        assert!(uninstall_project(root).is_err(), "{path}");
        assert_eq!(fs::read(outside.path().join("sentinel")).unwrap(), b"keep");
        if !path.starts_with(".opencode") {
            assert!(root.join(".opencode/agents/unelos-good.md").exists());
        }
    }
    let temp = tempfile::tempdir().unwrap();
    let root = Utf8Path::from_path(temp.path()).unwrap();
    put(root, ".opencode/agents/unelos-good.md", "keep");
    fs::create_dir_all(root.join(".claude/agents/unelos-directory.md")).unwrap();
    assert!(uninstall_project(root).is_err());
    assert!(root.join(".opencode/agents/unelos-good.md").exists());
}

#[cfg(unix)]
#[test]
fn nested_symlink_does_not_delete_target() {
    let temp = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let root = Utf8Path::from_path(temp.path()).unwrap();
    put(root, ".unelos/sentinel", "");
    fs::write(outside.path().join("sentinel"), "keep").unwrap();
    std::os::unix::fs::symlink(outside.path(), root.join(".unelos/link")).unwrap();
    fs::create_dir_all(root.join(".claude/agents")).unwrap();
    std::os::unix::fs::symlink(outside.path(), root.join(".claude/agents/mine.md")).unwrap();
    uninstall_project(root).unwrap();
    assert_eq!(fs::read(outside.path().join("sentinel")).unwrap(), b"keep");
    assert!(root.join(".claude/agents/mine.md").is_symlink());
}
