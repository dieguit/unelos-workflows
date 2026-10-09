use assert_cmd::cargo::cargo_bin_cmd;
use assert_fs::{fixture::PathCopy, TempDir};
use serde_json::Value;
use std::fs;

fn source() -> TempDir {
    let temp = TempDir::new().unwrap();
    temp.copy_from(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/opencode-execution"),
        &["**/*"],
    )
    .unwrap();
    // Exercise Windows checkout line endings on every platform.
    let path = temp.path().join(".unelos/project.yaml");
    let text = fs::read_to_string(&path).unwrap().replace("\r\n", "\n");
    fs::write(path, text.replace('\n', "\r\n")).unwrap();
    temp
}

fn install(target: &TempDir, source: Option<&TempDir>, version: &str) -> std::process::Output {
    let mut command = cargo_bin_cmd!("unelos");
    command.arg("--project").arg(target.path()).args([
        "install",
        "page-change",
        "--opencode",
        version,
        "--json",
    ]);
    if let Some(source) = source {
        command.arg("--source").arg(source.path());
    }
    command.output().unwrap()
}

fn profile(target: &TempDir, id: &str) -> (Value, String) {
    let text = fs::read_to_string(
        target
            .path()
            .join(format!(".opencode/agents/unelos-{id}.md")),
    )
    .unwrap();
    let mut parts = text.splitn(3, "---\n");
    parts.next();
    let front: Value = serde_yaml_ng::from_str(parts.next().unwrap()).unwrap();
    (front, parts.next().unwrap().into())
}

#[test]
fn workflow_install_generates_native_agents_and_reinstall_overwrites_edits() {
    let target = TempDir::new().unwrap();
    let source = source();
    fs::create_dir_all(target.path().join(".opencode/agents")).unwrap();
    fs::write(
        target.path().join(".opencode/agents/custom.md"),
        "User agent",
    )
    .unwrap();
    fs::write(target.path().join("opencode.json"), "{\"other\":true}").unwrap();
    let output = install(&target, Some(&source), "v2");
    assert!(output.status.success(), "{output:?}");
    let (front, body) = profile(&target, "t16-engineer");
    assert_eq!(front["mode"], "subagent");
    assert!(front.get("model").is_none());
    assert!(front.get("permissions").is_none());
    assert!(body.contains("Follow existing page conventions"));
    assert_eq!(profile(&target, "t16-controller").0["mode"], "primary");
    assert!(!target.path().join(".unelos/active-run").exists());
    fs::write(
        target
            .path()
            .join(".opencode/agents/unelos-t16-engineer.md"),
        "User edit to generated file",
    )
    .unwrap();
    assert!(install(&target, None, "v2").status.success());
    assert_eq!(profile(&target, "t16-engineer"), (front, body));
    assert_eq!(
        fs::read_to_string(target.path().join(".opencode/agents/custom.md")).unwrap(),
        "User agent"
    );
    assert_eq!(
        fs::read_to_string(target.path().join("opencode.json")).unwrap(),
        "{\"other\":true}"
    );
    assert!(install(&target, Some(&source), "v2").status.success());
}

#[test]
fn native_models_variants_and_tool_requirements_are_not_permission_grants() {
    for version in ["v1", "v2"] {
        let source = source();
        let target = TempDir::new().unwrap();
        let path = source.path().join(".unelos/project.yaml");
        let text = fs::read_to_string(&path)
            .unwrap()
            .replace("\r\n", "\n")
            .replace(
                "  t16-engineer:\n",
                "  t16-engineer:\n    model: provider/vendor/model#high\n    tools: [read-file]\n",
            );
        fs::write(path, text).unwrap();
        let output = install(&target, Some(&source), version);
        assert!(output.status.success(), "{output:?}");
        let (front, body) = profile(&target, "t16-engineer");
        assert_eq!(
            front["model"],
            if version == "v2" {
                "provider/vendor/model#high"
            } else {
                "provider/vendor/model"
            }
        );
        if version == "v1" {
            assert_eq!(front["variant"], "high");
        }
        assert!(front.get("permission").is_none());
        assert!(front.get("permissions").is_none());
        assert!(body.contains("read-file"));
    }
}

#[test]
fn unsupported_requirements_and_source_collisions_are_explicit() {
    let target = TempDir::new().unwrap();
    let source = source();
    let path = source.path().join(".unelos/project.yaml");
    let original = fs::read_to_string(&path).unwrap().replace("\r\n", "\n");
    fs::write(
        &path,
        original.replace(
            "  t16-engineer:\n",
            "  t16-engineer:\n    mcps: [browser]\n",
        ),
    )
    .unwrap();
    assert!(!install(&target, Some(&source), "v2").status.success());
    assert!(!target
        .path()
        .join(".opencode/agents/unelos-t16-engineer.md")
        .exists());
    fs::write(&path, &original).unwrap();
    assert!(install(&target, Some(&source), "v2").status.success());
    let target_path = target.path().join(".unelos/project.yaml");
    let before = fs::read(&target_path).unwrap();
    fs::write(
        &path,
        original.replace(
            "Follow existing page conventions",
            "Different source instructions",
        ),
    )
    .unwrap();
    assert!(!install(&target, Some(&source), "v2").status.success());
    assert_eq!(fs::read(target_path).unwrap(), before);
}

#[cfg(unix)]
#[test]
fn generated_agent_symlinks_do_not_overwrite_external_files() {
    let target = TempDir::new().unwrap();
    let source = source();
    let outside = TempDir::new().unwrap();
    assert!(install(&target, Some(&source), "v2").status.success());
    let external = outside.path().join("secret");
    fs::write(&external, "Preserve").unwrap();
    let path = target
        .path()
        .join(".opencode/agents/unelos-t16-engineer.md");
    fs::remove_file(&path).unwrap();
    std::os::unix::fs::symlink(&external, path).unwrap();
    assert!(!install(&target, None, "v2").status.success());
    assert_eq!(fs::read_to_string(external).unwrap(), "Preserve");
}

#[cfg(unix)]
#[test]
fn unsafe_native_destination_does_not_partially_import_a_workflow() {
    let target = TempDir::new().unwrap();
    let source = source();
    let outside = TempDir::new().unwrap();
    fs::create_dir_all(target.path().join(".opencode/agents")).unwrap();
    let external = outside.path().join("secret");
    fs::write(&external, "Preserve").unwrap();
    std::os::unix::fs::symlink(
        &external,
        target
            .path()
            .join(".opencode/agents/unelos-t16-engineer.md"),
    )
    .unwrap();
    assert!(!install(&target, Some(&source), "v2").status.success());
    assert!(!target.path().join(".unelos/project.yaml").exists());
    assert!(!target
        .path()
        .join(".unelos/workflows/page-change.yaml")
        .exists());
    assert_eq!(fs::read_to_string(external).unwrap(), "Preserve");
}

#[test]
fn profile_regeneration_preserves_authoring_and_active_run_bytes() {
    let target = TempDir::new().unwrap();
    let source = source();
    assert!(install(&target, Some(&source), "v2").status.success());
    cargo_bin_cmd!("unelos")
        .arg("--project")
        .arg(target.path())
        .args([
            "execution",
            "start",
            "page-change",
            "--name",
            "keep-run",
            "--json",
        ])
        .assert()
        .success();
    let state = target.path().join(".unelos/runs/keep-run/state.json");
    let authoring = target.path().join(".unelos/project.yaml");
    let state_before = fs::read(&state).unwrap();
    let authoring_before = fs::read(&authoring).unwrap();
    assert!(install(&target, None, "v1").status.success());
    assert_eq!(fs::read(state).unwrap(), state_before);
    assert_eq!(fs::read(authoring).unwrap(), authoring_before);
}
