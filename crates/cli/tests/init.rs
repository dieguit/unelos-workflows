use assert_cmd::cargo::cargo_bin_cmd;
use assert_fs::TempDir;
use serde_json::{json, Value};
use std::{fs, path::Path, process::Output};

fn init(root: &Path, args: &[&str]) -> Output {
    cargo_bin_cmd!("unelos")
        .arg("--project")
        .arg(root)
        .args(["init", "--json"])
        .args(args)
        .output()
        .unwrap()
}

fn write(root: &Path, path: &str, text: &str) {
    let path = root.join(path);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

fn value(root: &Path, path: &str) -> Value {
    jsonc_parser::parse_to_serde_value(
        &fs::read_to_string(root.join(path)).unwrap(),
        &jsonc_parser::ParseOptions::default(),
    )
    .unwrap()
}

#[test]
fn fresh_init_bundles_plugin_and_records_version() {
    let project = TempDir::new().unwrap();
    let root = project.path().join("project café space");
    fs::create_dir(&root).unwrap();
    let output = init(&root, &[]);
    assert!(output.status.success(), "{output:?}");
    let summary: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(summary["opencode"], "v2");
    let config = value(&root, "opencode.json");
    assert_eq!(config["plugins"][0]["package"], "./.unelos/opencode/unelos");
    let binary = config["plugins"][0]["options"]["binary"].as_str().unwrap();
    assert!(Path::new(binary).is_absolute());
    assert_eq!(
        fs::canonicalize(binary).unwrap(),
        fs::canonicalize(assert_cmd::cargo::cargo_bin!("unelos")).unwrap()
    );
    assert_eq!(
        value(&root, ".unelos/integration.json"),
        json!({"opencode":"v2"})
    );
    for file in ["package.json", "index.js", "v1.js", "commands/unelos.md"] {
        assert_eq!(
            fs::read(root.join(".unelos/opencode/unelos").join(file)).unwrap(),
            fs::read(
                Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("../../integrations/opencode")
                    .join(file)
            )
            .unwrap()
        );
    }
    assert!(root.join(".unelos/workflows").is_dir());
    assert!(root.join(".unelos/skills").is_dir());
    assert!(root.join(".unelos/tasks").is_dir());
    assert!(root.join(".unelos/agents").is_dir());
    assert!(!root.join(".unelos/runs").exists());
    assert!(!root.join(".unelos/bin").exists());
    cargo_bin_cmd!("unelos")
        .arg("--project")
        .arg(&root)
        .arg("start")
        .assert()
        .success();
}

#[test]
fn rerun_preserves_authoring_config_agents_and_reports_and_repairs_assets() {
    let project = TempDir::new().unwrap();
    let root = project.path();
    let preserved = [
        (".unelos/project.yaml", "# authored\n{}\n"),
        (".unelos/skills/custom/SKILL.md", "custom skill"),
        (".unelos/workflows/custom.yaml", "custom workflow"),
        (".unelos/runs/a/report.md", "report"),
        (".unelos/active-run", "a"),
        (".opencode/agents/custom.md", "custom agent"),
    ];
    for (path, text) in preserved {
        write(root, path, text);
    }
    write(
        root,
        "opencode.jsonc",
        "{\n// preserve\n\"model\":\"p/m\",\"plugins\":[\"other\",],\n}",
    );
    assert!(init(root, &[]).status.success());
    let config = fs::read(root.join("opencode.jsonc")).unwrap();
    assert!(init(root, &[]).status.success());
    assert_eq!(fs::read(root.join("opencode.jsonc")).unwrap(), config);
    fs::remove_file(root.join(".unelos/opencode/unelos/index.js")).unwrap();
    assert!(init(root, &[]).status.success());
    assert!(root.join(".unelos/opencode/unelos/index.js").is_file());
    for (path, text) in preserved {
        assert_eq!(fs::read_to_string(root.join(path)).unwrap(), text);
    }
    assert!(fs::read_to_string(root.join("opencode.jsonc"))
        .unwrap()
        .contains("// preserve"));
    assert_eq!(value(root, "opencode.jsonc")["plugins"][0], "other");
    fs::remove_file(root.join("opencode.jsonc")).unwrap();
    assert!(init(root, &[]).status.success());
    assert!(root.join("opencode.json").exists());
}

#[test]
fn config_locations_resolve_package_relative_to_config() {
    for path in [
        "opencode.json",
        "opencode.jsonc",
        ".opencode/opencode.json",
        ".opencode/opencode.jsonc",
    ] {
        let project = TempDir::new().unwrap();
        write(project.path(), path, "{\"plugins\":[\"other\"]}");
        let output = init(project.path(), &[]);
        assert!(output.status.success(), "{path}: {output:?}");
        let config = value(project.path(), path);
        let package = config["plugins"][1]["package"].as_str().unwrap();
        assert_eq!(
            fs::canonicalize(project.path().join(path).parent().unwrap().join(package)).unwrap(),
            fs::canonicalize(project.path().join(".unelos/opencode/unelos")).unwrap()
        );
    }
}

#[test]
fn v1_defaults_survive_rerun_and_switch_preserves_edited_commands() {
    let project = TempDir::new().unwrap();
    let root = project.path();
    assert!(init(root, &["--opencode", "v1"]).status.success());
    assert!(root.join(".opencode/commands/unelos.md").exists());
    assert!(init(root, &[]).status.success());
    assert_eq!(value(root, ".unelos/integration.json")["opencode"], "v1");
    assert_eq!(
        value(root, "opencode.json")["plugin"][0][0],
        "./.unelos/opencode/unelos/index.js"
    );
    write(root, ".opencode/commands/unelos.md", "my command");
    assert!(init(root, &["--opencode", "v2"]).status.success());
    assert_eq!(
        fs::read_to_string(root.join(".opencode/commands/unelos.md")).unwrap(),
        "my command"
    );
    let config = fs::read(root.join("opencode.json")).unwrap();
    assert!(!init(root, &["--opencode", "v1"]).status.success());
    assert_eq!(fs::read(root.join("opencode.json")).unwrap(), config);
}

#[test]
fn generated_v1_command_removed_on_switch() {
    let project = TempDir::new().unwrap();
    assert!(init(project.path(), &["--opencode", "v1"]).status.success());
    assert!(init(project.path(), &["--opencode", "v2"]).status.success());
    assert!(!project.path().join(".opencode/commands/unelos.md").exists());
}

#[test]
fn invalid_inputs_fail_without_partial_installation() {
    for (path, text) in [
        ("opencode.json", "[]"),
        ("opencode.json", "{\"plugins\":false}"),
        (".unelos/project.yaml", "invalid: ["),
        (".unelos/integration.json", "{\"opencode\":\"v3\"}"),
        (".unelos/workflows", "not a directory"),
        (".unelos/tasks", "not a directory"),
        (".unelos/agents", "not a directory"),
        (".unelos/opencode/unelos", "not a directory"),
        (".opencode/commands/unelos.md", "edited command"),
    ] {
        let project = TempDir::new().unwrap();
        write(project.path(), path, text);
        let output = init(project.path(), &["--opencode", "v1"]);
        assert!(!output.status.success(), "accepted {path}: {output:?}");
        assert_eq!(fs::read_to_string(project.path().join(path)).unwrap(), text);
        assert!(!project
            .path()
            .join(".unelos/opencode/unelos/index.js")
            .exists());
        if path != "opencode.json" {
            assert!(!project.path().join("opencode.json").exists());
        }
    }
    let project = TempDir::new().unwrap();
    assert!(!init(&project.path().join("missing"), &[]).status.success());
    assert!(!project.path().join("missing").exists());
}

#[test]
fn ambiguous_configs_and_version_conflicts_require_explicit_resolution() {
    let project = TempDir::new().unwrap();
    write(project.path(), "opencode.json", "{}");
    write(project.path(), ".opencode/opencode.jsonc", "{}");
    assert!(!init(project.path(), &[]).status.success());
    assert!(!project.path().join(".unelos").exists());
    fs::remove_file(project.path().join(".opencode/opencode.jsonc")).unwrap();
    write(
        project.path(),
        "opencode.json",
        "{\"plugin\":[\"./.unelos/opencode/thin-plugin/index.js\"]}",
    );
    write(
        project.path(),
        ".unelos/integration.json",
        "{\"opencode\":\"v2\"}",
    );
    assert!(!init(project.path(), &[]).status.success());
    assert!(init(project.path(), &["--opencode", "v2"]).status.success());
    assert_eq!(value(project.path(), "opencode.json")["plugin"], json!([]));
}

#[cfg(unix)]
#[test]
fn symlinked_destinations_cannot_modify_external_files() {
    for path in [
        "opencode.json",
        ".unelos/project.yaml",
        ".unelos/integration.json",
        ".unelos/skills",
        ".unelos/tasks",
        ".unelos/agents",
        ".unelos/opencode/unelos",
        ".opencode/commands/unelos.md",
    ] {
        let project = TempDir::new().unwrap();
        let outside = TempDir::new().unwrap();
        let external = outside.path().join("external");
        fs::write(&external, "preserve").unwrap();
        let destination = project.path().join(path);
        fs::create_dir_all(destination.parent().unwrap()).unwrap();
        std::os::unix::fs::symlink(&external, &destination).unwrap();
        assert!(
            !init(project.path(), &["--opencode", "v1"]).status.success(),
            "accepted {path}"
        );
        assert_eq!(fs::read_to_string(external).unwrap(), "preserve");
        assert!(!project
            .path()
            .join(".unelos/opencode/unelos/index.js")
            .exists());
    }
}
