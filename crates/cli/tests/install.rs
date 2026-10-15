use assert_cmd::cargo::cargo_bin_cmd;
use assert_fs::{fixture::PathCopy, TempDir};
use serde_json::Value;
use std::fs;

fn source() -> TempDir {
    let temp = TempDir::new().unwrap();
    temp.copy_from(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../core/tests/fixtures/simple-plan-execute-review"),
        &["**/*"],
    )
    .unwrap();
    // Exercise Windows checkout line endings on every platform.
    let path = temp.path().join("agents/simple-implementer/agent.yaml");
    let text = fs::read_to_string(&path).unwrap().replace("\r\n", "\n");
    fs::write(path, text.replace('\n', "\r\n")).unwrap();
    temp
}

fn install(target: &TempDir, source: Option<&TempDir>, version: &str) -> std::process::Output {
    let mut command = cargo_bin_cmd!("unelos");
    command.arg("--project").arg(target.path()).args([
        "install",
        "simple-plan-execute-review",
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
    let (front, body) = profile(&target, "simple-implementer");
    assert_eq!(front["mode"], "subagent");
    assert!(front.get("model").is_none());
    assert!(front.get("permissions").is_none());
    assert!(body.contains("Follow existing project conventions"));
    assert_eq!(profile(&target, "simple-controller").0["mode"], "primary");
    assert!(!target.path().join(".unelos/active-run").exists());
    fs::write(
        target
            .path()
            .join(".opencode/agents/unelos-simple-implementer.md"),
        "User edit to generated file",
    )
    .unwrap();
    assert!(install(&target, None, "v2").status.success());
    assert_eq!(profile(&target, "simple-implementer"), (front, body));
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
        let path = source.path().join("agents/simple-implementer/agent.yaml");
        let text = fs::read_to_string(&path)
            .unwrap()
            .replace("\r\n", "\n")
            .replace(
                "instructionsFile:",
                "model: provider/vendor/model#high\ntools: [read-file]\ninstructionsFile:",
            );
        fs::write(path, text).unwrap();
        let output = install(&target, Some(&source), version);
        assert!(output.status.success(), "{output:?}");
        let (front, body) = profile(&target, "simple-implementer");
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
    let path = source.path().join("agents/simple-implementer/agent.yaml");
    let original = fs::read_to_string(&path).unwrap().replace("\r\n", "\n");
    fs::write(
        &path,
        original.replace("instructionsFile:", "mcps: [browser]\ninstructionsFile:"),
    )
    .unwrap();
    assert!(!install(&target, Some(&source), "v2").status.success());
    assert!(!target
        .path()
        .join(".opencode/agents/unelos-simple-implementer.md")
        .exists());
    fs::write(&path, &original).unwrap();
    assert!(install(&target, Some(&source), "v2").status.success());
    let target_path = target.path().join(".unelos/project.yaml");
    let before = fs::read(&target_path).unwrap();
    fs::write(
        &path,
        original.replace(
            "instructionsFile: instructions.md",
            "instructions: Different source instructions",
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
        .join(".opencode/agents/unelos-simple-implementer.md");
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
            .join(".opencode/agents/unelos-simple-implementer.md"),
    )
    .unwrap();
    assert!(!install(&target, Some(&source), "v2").status.success());
    assert!(!target.path().join(".unelos/project.yaml").exists());
    assert!(!target
        .path()
        .join(".unelos/workflows/simple-plan-execute-review.yaml")
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
            "simple-plan-execute-review",
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

#[test]
fn initialized_install_infers_native_version_and_explicit_override_preserves_metadata() {
    let target = TempDir::new().unwrap();
    let source = source();
    let authored = source.path().join("agents/simple-implementer/agent.yaml");
    let text = fs::read_to_string(&authored)
        .unwrap()
        .replace("\r\n", "\n")
        .replace(
            "instructionsFile:",
            "model: provider/model#high\ninstructionsFile:",
        );
    fs::write(authored, text).unwrap();
    cargo_bin_cmd!("unelos")
        .arg("--project")
        .arg(target.path())
        .arg("init")
        .assert()
        .success();
    cargo_bin_cmd!("unelos")
        .arg("--project")
        .arg(target.path())
        .args(["install", "simple-plan-execute-review", "--source"])
        .arg(source.path())
        .assert()
        .success();
    assert_eq!(
        profile(&target, "simple-implementer").0["model"],
        "provider/model#high"
    );
    let before = fs::read(target.path().join(".unelos/integration.json")).unwrap();
    assert!(install(&target, None, "v1").status.success());
    assert_eq!(
        profile(&target, "simple-implementer").0["model"],
        "provider/model"
    );
    assert_eq!(profile(&target, "simple-implementer").0["variant"], "high");
    assert_eq!(
        fs::read(target.path().join(".unelos/integration.json")).unwrap(),
        before
    );
}

#[test]
fn uninitialized_install_without_version_stays_core_only() {
    let target = TempDir::new().unwrap();
    let source = source();
    cargo_bin_cmd!("unelos")
        .arg("--project")
        .arg(target.path())
        .args(["install", "simple-plan-execute-review", "--source"])
        .arg(source.path())
        .assert()
        .success();
    assert!(target
        .path()
        .join(".unelos/workflows/simple-plan-execute-review.yaml")
        .exists());
    assert!(!target.path().join(".opencode").exists());
}

#[test]
fn broken_recorded_integration_fails_before_workflow_import() {
    for missing in [
        ".unelos/opencode/unelos/index.js",
        "opencode.json",
        ".opencode/commands/unelos.md",
    ] {
        let target = TempDir::new().unwrap();
        let source = source();
        cargo_bin_cmd!("unelos")
            .arg("--project")
            .arg(target.path())
            .args(["init", "--opencode", "v1"])
            .assert()
            .success();
        fs::remove_file(target.path().join(missing)).unwrap();
        let output = cargo_bin_cmd!("unelos")
            .arg("--project")
            .arg(target.path())
            .args(["install", "simple-plan-execute-review", "--source"])
            .arg(source.path())
            .arg("--json")
            .output()
            .unwrap();
        assert!(!output.status.success(), "accepted missing {missing}");
        assert!(String::from_utf8(output.stderr)
            .unwrap()
            .contains("unelos init"));
        assert!(!target
            .path()
            .join(".unelos/workflows/simple-plan-execute-review.yaml")
            .exists());
    }
}

#[test]
fn invalid_metadata_cannot_silently_disable_native_generation() {
    for explicit in [false, true] {
        let target = TempDir::new().unwrap();
        let source = source();
        fs::create_dir(target.path().join(".unelos")).unwrap();
        fs::write(
            target.path().join(".unelos/integration.json"),
            "{\"opencode\":\"bad\"}",
        )
        .unwrap();
        let mut cmd = cargo_bin_cmd!("unelos");
        cmd.arg("--project")
            .arg(target.path())
            .args(["install", "simple-plan-execute-review", "--source"])
            .arg(source.path());
        if explicit {
            cmd.args(["--opencode", "v2"]);
        }
        cmd.assert().failure();
        assert!(!target
            .path()
            .join(".unelos/workflows/simple-plan-execute-review.yaml")
            .exists());
    }
}
