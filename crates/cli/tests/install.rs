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
                "model: {harness: opencode, model: provider/vendor/model#high}\ntools: [read-file]\ninstructionsFile:",
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
fn missing_mcp_and_source_collisions_are_explicit() {
    let target = TempDir::new().unwrap();
    let source = source();
    let path = source.path().join("agents/simple-implementer/agent.yaml");
    let original = fs::read_to_string(&path).unwrap().replace("\r\n", "\n");
    fs::write(
        &path,
        original.replace("instructionsFile:", "mcps: [browser]\ninstructionsFile:"),
    )
    .unwrap();
    let missing = install(&target, Some(&source), "v2");
    assert!(!missing.status.success());
    assert!(
        String::from_utf8_lossy(&missing.stderr).contains("missing_mcp"),
        "{missing:?}"
    );
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

#[test]
fn mcps_project_with_scoped_tools_and_cleanup_without_clobbering_user_config() {
    let target = TempDir::new().unwrap();
    let source = source();
    let path = source.path().join("agents/simple-implementer/agent.yaml");
    let text = fs::read_to_string(&path).unwrap().replace("\r\n", "\n");
    fs::write(
        &path,
        text.replace(
            "instructionsFile:",
            "mcps: [browser, web]\ninstructionsFile:",
        ),
    )
    .unwrap();
    fs::create_dir_all(source.path().join("mcps")).unwrap();
    fs::write(
        source.path().join("mcps/browser.yaml"),
        "type: local\ncommand: [npx, browser]\nenvironment: {TOKEN: '{env:TOKEN}'}\n",
    )
    .unwrap();
    fs::write(
        source.path().join("mcps/web.yaml"),
        "type: remote\nurl: https://example.com/mcp\nheaders: {Authorization: '{env:TOKEN}'}\n",
    )
    .unwrap();
    let config = target.path().join("opencode.jsonc");
    fs::write(&config, "{\n// preserve me\n\"mcp\": {\"custom\": {\"type\":\"remote\",\"url\":\"https://custom.test\"}},\n\"tools\":{\"custom_*\":true},\n}\n").unwrap();
    let output = install(&target, Some(&source), "v2");
    assert!(output.status.success(), "{output:?}");
    let front = profile(&target, "simple-implementer").0;
    assert_eq!(front["tools"]["browser_*"], true);
    assert_eq!(front["tools"]["web_*"], true);
    assert!(profile(&target, "simple-controller")
        .0
        .get("tools")
        .is_none());
    let read_config = || {
        let text = fs::read_to_string(&config).unwrap();
        assert!(text.contains("// preserve me"));
        let text = text
            .lines()
            .filter(|line| !line.trim_start().starts_with("//"))
            .collect::<Vec<_>>()
            .join("\n");
        serde_yaml_ng::from_str::<Value>(&text).unwrap()
    };
    let value = read_config();
    assert_eq!(value["mcp"]["browser"]["enabled"], true);
    assert_eq!(
        value["mcp"]["browser"]["command"],
        serde_json::json!(["npx", "browser"])
    );
    assert_eq!(
        value["mcp"]["web"]["headers"]["Authorization"],
        "{env:TOKEN}"
    );
    assert_eq!(value["tools"]["browser_*"], false);
    assert!(install(&target, None, "v2").status.success());
    let installed_agent = target
        .path()
        .join(".unelos/agents/simple-implementer/agent.yaml");
    fs::write(&installed_agent, text).unwrap();
    assert!(install(&target, None, "v2").status.success());
    let value = read_config();
    assert!(value["mcp"].get("browser").is_none());
    assert!(value["tools"].get("browser_*").is_none());
    assert_eq!(value["tools"]["custom_*"], true);
    assert!(value["mcp"].get("custom").is_some());
    let original = fs::read_to_string(&installed_agent).unwrap();
    fs::write(
        &installed_agent,
        original.replace("instructionsFile:", "mcps: [browser]\ninstructionsFile:"),
    )
    .unwrap();
    fs::write(
        &config,
        "{\"mcp\":{\"browser\":{\"type\":\"remote\",\"url\":\"https://user.test\"}}}",
    )
    .unwrap();
    let before = fs::read(&config).unwrap();
    assert!(!install(&target, None, "v2").status.success());
    assert_eq!(fs::read(&config).unwrap(), before);
}

#[test]
fn global_mcps_use_bare_keys_and_reject_project_collisions() {
    let target = TempDir::new().unwrap();
    let source = source();
    let global = TempDir::new().unwrap();
    fs::create_dir_all(global.path().join("mcps")).unwrap();
    fs::write(
        global.path().join("mcps/browser.yaml"),
        "type: remote\nurl: https://example.com/mcp\n",
    )
    .unwrap();
    let path = source.path().join("agents/simple-implementer/agent.yaml");
    let text = fs::read_to_string(&path).unwrap().replace("\r\n", "\n");
    fs::write(
        &path,
        text.replace(
            "instructionsFile:",
            "mcps: [global/browser]\ninstructionsFile:",
        ),
    )
    .unwrap();
    let run = || {
        cargo_bin_cmd!("unelos")
            .env("UNELOS_CONFIG_DIR", global.path())
            .arg("--project")
            .arg(target.path())
            .args([
                "install",
                "simple-plan-execute-review",
                "--opencode",
                "v2",
                "--json",
                "--source",
            ])
            .arg(source.path())
            .output()
            .unwrap()
    };
    let output = run();
    assert!(output.status.success(), "{output:?}");
    let config: Value =
        serde_json::from_slice(&fs::read(target.path().join("opencode.json")).unwrap()).unwrap();
    assert_eq!(config["mcp"]["browser"]["url"], "https://example.com/mcp");
    assert!(config["mcp"].get("global/browser").is_none());
    assert_eq!(
        profile(&target, "simple-implementer").0["tools"]["browser_*"],
        true
    );
    assert!(!target.path().join(".unelos/mcps/browser.yaml").exists());
    fs::create_dir_all(source.path().join("mcps")).unwrap();
    fs::write(
        source.path().join("mcps/browser.yaml"),
        "type: remote\nurl: https://example.com/mcp\n",
    )
    .unwrap();
    fs::write(
        &path,
        text.replace(
            "instructionsFile:",
            "mcps: [browser, global/browser]\ninstructionsFile:",
        ),
    )
    .unwrap();
    let output = run();
    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("mcp_collision"),
        "{output:?}"
    );
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
    let state = target.path().join(".unelos/runs/r-000001/state.json");
    let authoring = target.path().join(".unelos/project.yaml");
    let state_before = fs::read(&state).unwrap();
    let authoring_before = fs::read(&authoring).unwrap();
    assert!(install(&target, None, "v1").status.success());
    assert_eq!(fs::read(state).unwrap(), state_before);
    assert_eq!(fs::read(authoring).unwrap(), authoring_before);
}

#[test]
fn initialized_install_uses_explicit_native_version_without_integration_metadata() {
    let target = TempDir::new().unwrap();
    let source = source();
    let authored = source.path().join("agents/simple-implementer/agent.yaml");
    let text = fs::read_to_string(&authored)
        .unwrap()
        .replace("\r\n", "\n")
        .replace(
            "instructionsFile:",
            "model: {harness: opencode, model: provider/model#high}\ninstructionsFile:",
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
        .args([
            "install",
            "simple-plan-execute-review",
            "--opencode",
            "v2",
            "--source",
        ])
        .arg(source.path())
        .assert()
        .success();
    assert_eq!(
        profile(&target, "simple-implementer").0["model"],
        "provider/model#high"
    );
    assert!(!target.path().join(".unelos/integration.json").exists());
    assert!(install(&target, None, "v1").status.success());
    assert_eq!(
        profile(&target, "simple-implementer").0["model"],
        "provider/model"
    );
    assert_eq!(profile(&target, "simple-implementer").0["variant"], "high");
    assert!(!target.path().join(".unelos/integration.json").exists());
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
fn legacy_recorded_version_still_selects_native_agents() {
    for version in ["v1", "v2"] {
        let target = TempDir::new().unwrap();
        let source = source();
        cargo_bin_cmd!("unelos")
            .arg("--project")
            .arg(target.path())
            .arg("init")
            .assert()
            .success();
        fs::write(
            target.path().join(".unelos/integration.json"),
            format!("{{\"opencode\":\"{version}\"}}"),
        )
        .unwrap();
        let output = cargo_bin_cmd!("unelos")
            .arg("--project")
            .arg(target.path())
            .args(["install", "simple-plan-execute-review", "--source"])
            .arg(source.path())
            .arg("--json")
            .output()
            .unwrap();
        assert!(output.status.success(), "{output:?}");
        assert!(target
            .path()
            .join(".opencode/agents/unelos-simple-implementer.md")
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
