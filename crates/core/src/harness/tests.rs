use super::*;
use std::sync::Mutex;
static ENV: Mutex<()> = Mutex::new(());

#[test]
fn global_opencode_mcp_ownership_preserves_user_config() {
    let _lock = ENV.lock().unwrap();
    let temp = assert_fs::TempDir::new().unwrap();
    let root = Utf8Path::from_path(temp.path()).unwrap();
    let unelos = root.join("unelos");
    let opencode = root.join("opencode");
    let _env = Env::set(&[
        ("UNELOS_CONFIG_DIR", unelos.as_str()),
        ("OPENCODE_CONFIG_DIR", opencode.as_str()),
        ("HOME", root.as_str()),
        ("PATH", root.join("empty").as_str()),
    ]);
    fs::create_dir_all(&opencode).unwrap();
    let path = opencode.join("opencode.jsonc");
    let source = root.join("cli");
    fs::write(&source, "binary").unwrap();
    let user_line = r#"    "user": { "type": "local", "command": ["user-command"] }"#;
    let baseline = format!(
        "{{\n  // Keep this comment\n  \"mcp\": {{\n{user_line}\n  }},\n  \"tools\": {{}}\n}}\n"
    );
    fs::write(&path, &baseline).unwrap();
    install_harness("opencode", &source).unwrap();
    let installed = fs::read_to_string(&path).unwrap();
    let server = crate::McpServer::Local {
        command: vec!["fixture-command".into()],
        environment: Default::default(),
    };
    let definition = unelos.join("agents").join("probe");
    fs::create_dir_all(&definition).unwrap();
    let agent_file = definition.join("agent.yaml");
    let profile = opencode.join("agents").join("unelos-global-probe.md");
    let receipt = unelos.join("opencode-mcps.json");
    let add_mcp = |id: &str| {
        crate::save_global_mcp(
            None,
            crate::McpDocument {
                id: id.to_owned(),
                server: server.clone(),
            },
        )
        .unwrap();
    };
    add_mcp("probe");
    fs::write(&agent_file, "instructions: probe\nmcps: [global/probe]\n").unwrap();
    let build = crate::build_global_harness_agents().unwrap();
    assert!(build.skipped.is_empty(), "{:?}", build.skipped);
    let configured = fs::read_to_string(&path).unwrap();
    assert!(configured.contains(user_line));
    assert!(configured.contains("// Keep this comment"));
    assert!(configured.contains("\"enabled\": true"));
    assert!(configured.contains("\"probe_*\": false"));
    assert!(fs::read_to_string(&profile)
        .unwrap()
        .contains("probe_*: true"));
    assert!(receipt.is_file());
    assert!(crate::build_global_harness_agents()
        .unwrap()
        .written
        .is_empty());
    assert_eq!(fs::read_to_string(&path).unwrap(), configured);

    fs::remove_dir_all(&definition).unwrap();
    crate::build_global_harness_agents().unwrap();
    assert_eq!(fs::read_to_string(&path).unwrap(), installed);
    assert!(!receipt.exists());
    fs::create_dir_all(&definition).unwrap();
    add_mcp("user");
    let identical = installed.replace(
        user_line,
        r#"    "user": { "type": "local", "command": ["fixture-command"], "environment": {}, "enabled": true }"#,
    ).replace("\"tools\": {}", "\"tools\": {\"user_*\": false}");
    fs::write(&path, &identical).unwrap();
    fs::write(&agent_file, "instructions: probe\nmcps: [global/user]\n").unwrap();
    let build = crate::build_global_harness_agents().unwrap();
    assert!(build.skipped.is_empty(), "{:?}", build.skipped);
    assert_eq!(fs::read_to_string(&path).unwrap(), identical);
    assert!(!receipt.exists());
    assert!(fs::read_to_string(&profile)
        .unwrap()
        .contains("user_*: true"));
    fs::write(&path, &installed).unwrap();
    fs::write(
        &agent_file,
        "instructions: probe\nmcps: [global/probe, global/user]\n",
    )
    .unwrap();
    let build = crate::build_global_harness_agents().unwrap();
    assert!(build.skipped.iter().any(|s| s.contains("mcp.user")));
    assert!(!fs::read_to_string(&profile).unwrap().contains("user_*"));
    let configured = fs::read_to_string(&path).unwrap();
    let modified = configured.replace("fixture-command", "user-edited");
    fs::write(&path, &modified).unwrap();
    let build = crate::build_global_harness_agents().unwrap();
    assert!(build
        .skipped
        .iter()
        .any(|s| s.contains("released ownership")));
    assert_eq!(fs::read_to_string(&path).unwrap(), modified);
    assert!(!fs::read_to_string(&profile).unwrap().contains("probe_*"));

    fs::write(&path, "{invalid").unwrap();
    let build = crate::build_global_harness_agents().unwrap();
    assert!(build
        .skipped
        .iter()
        .any(|s| s.starts_with("opencode globals:")));
    assert_eq!(fs::read_to_string(&path).unwrap(), "{invalid");
    fs::write(&path, &modified).unwrap();
    let duplicate = opencode.join("opencode.json");
    fs::write(&duplicate, "{}\n").unwrap();
    assert!(crate::build_global_harness_agents()
        .unwrap()
        .skipped
        .iter()
        .any(|s| s.contains("Both opencode.json and opencode.jsonc exist")));
    assert_eq!(fs::read_to_string(&path).unwrap(), modified);
    fs::remove_file(duplicate).unwrap();
    uninstall_harness("opencode").unwrap();
    let final_config = fs::read_to_string(&path).unwrap();
    assert!(final_config.contains("user-edited"));
    assert!(final_config.contains(user_line));
    assert!(!final_config.contains("probe_*"));
    assert!(!receipt.exists());
    assert!(!opencode.join("opencode.json").exists());
}

#[test]
#[cfg(unix)]
fn bundled_globals_install_update_preserve_and_remove() {
    let _lock = ENV.lock().unwrap();
    let temp = assert_fs::TempDir::new().unwrap();
    let root = Utf8Path::from_path(temp.path()).unwrap();
    let fake = fake_claude(root);
    let config = root.join("unelos");
    let _env = Env::set(&[
        ("UNELOS_CONFIG_DIR", config.as_str()),
        ("CLAUDE_CONFIG_DIR", root.join("claude").as_str()),
        ("UNELOS_CLAUDE_BIN", fake.as_str()),
        ("OPENCODE_CONFIG_DIR", root.join("opencode").as_str()),
        ("HOME", root.as_str()),
        ("PATH", root.join("empty").as_str()),
    ]);
    let source = root.join("source");
    fs::write(&source, "binary").unwrap();
    install_harness("opencode", &source).unwrap();
    let opencode_agents = root.join("opencode").join("agents");
    let author = opencode_agents.join("unelos-global-workflow-author.md");
    let analyst = opencode_agents.join("unelos-global-workflow-analyst.md");
    assert!(author.is_file());
    assert!(analyst.is_file());
    fs::write(&author, "user edits").unwrap();
    let initial: std::collections::BTreeMap<String, String> =
        serde_json::from_slice(&fs::read(config.join("bundled.json")).unwrap()).unwrap();
    assert_eq!(initial.len(), 21);
    for relative in initial.keys() {
        assert!(relative
            .split('/')
            .fold(config.clone(), |p, s| p.join(s))
            .is_file());
    }
    uninstall_harness("opencode").unwrap();
    assert!(!analyst.exists());
    assert_eq!(fs::read_to_string(&author).unwrap(), "user edits");
    install_harness("claude", &source).unwrap();
    let claude_author = root
        .join("claude")
        .join("agents")
        .join("unelos-global-workflow-author.md");
    assert!(claude_author.is_file());
    uninstall_harness("claude").unwrap();
    assert!(!claude_author.exists());
    let unowned = config
        .join("agents")
        .join("workflow-author")
        .join("instructions.md");
    fs::create_dir_all(unowned.parent().unwrap()).unwrap();
    fs::write(&unowned, "pre-existing").unwrap();
    install_harness("opencode", &source).unwrap();
    assert_eq!(fs::read_to_string(&unowned).unwrap(), "pre-existing");
    let manifest_path = config.join("bundled.json");
    let mut manifest: std::collections::BTreeMap<String, String> =
        serde_json::from_slice(&fs::read(&manifest_path).unwrap()).unwrap();
    assert_eq!(manifest.len(), 20);
    for relative in manifest.keys() {
        let file = relative.split('/').fold(config.clone(), |p, s| p.join(s));
        assert!(file.is_file(), "{file}");
    }
    // Simulate the bytes and recorded hash from an older bundled release.
    use sha2::{Digest, Sha256};
    let workflow = config.join("workflows").join("workflow-design.yaml");
    fs::write(&workflow, "older release").unwrap();
    manifest.insert(
        "workflows/workflow-design.yaml".into(),
        format!("{:x}", Sha256::digest(b"older release")),
    );
    fs::write(&manifest_path, serde_json::to_vec(&manifest).unwrap()).unwrap();
    install_harness("opencode", &source).unwrap();
    assert_ne!(fs::read_to_string(&workflow).unwrap(), "older release");
    assert_eq!(fs::read_to_string(&unowned).unwrap(), "pre-existing");
    fs::write(&workflow, "user changes").unwrap();
    install_harness("claude", &source).unwrap();
    assert_eq!(fs::read_to_string(&workflow).unwrap(), "user changes");
    uninstall_harness("opencode").unwrap();
    assert!(manifest_path.exists());
    let skill = config.join("skills").join("unelos-workflow-design");
    assert!(skill.exists());
    uninstall_harness("claude").unwrap();
    assert!(!manifest_path.exists());
    assert!(!skill.exists());
    assert_eq!(fs::read_to_string(&workflow).unwrap(), "user changes");
    assert_eq!(fs::read_to_string(&unowned).unwrap(), "pre-existing");
}

#[cfg(unix)]
fn fake_claude(root: &Utf8Path) -> Utf8PathBuf {
    use std::os::unix::fs::PermissionsExt;
    let binary = root.join("fake-claude");
    fs::write(&binary, r#"#!/bin/sh
set -eu
/bin/mkdir -p "$CLAUDE_CONFIG_DIR"
echo "$*" >> "$CLAUDE_CONFIG_DIR/calls"
case "$*" in
  "plugin marketplace list --json")
    if [ -f "$CLAUDE_CONFIG_DIR/market" ]; then echo '[{"name":"unelos-local"}]'; else echo '[]'; fi ;;
  "plugin list --json")
    if [ -f "$CLAUDE_CONFIG_DIR/plugin" ]; then echo '[{"id":"unelos@unelos-local"}]'; else echo '[]'; fi ;;
  "plugin marketplace add "*) : > "$CLAUDE_CONFIG_DIR/market" ;;
  "plugin marketplace update "*|"plugin update "*) ;;
  "plugin install "*|"plugin enable "*)
    : > "$CLAUDE_CONFIG_DIR/plugin"
    echo '{"enabledPlugins":{"unelos@unelos-local":true}}' > "$CLAUDE_CONFIG_DIR/settings.json" ;;
  "plugin uninstall "*)
    /bin/rm -f "$CLAUDE_CONFIG_DIR/plugin"
    echo '{}' > "$CLAUDE_CONFIG_DIR/settings.json" ;;
  "plugin marketplace remove "*) /bin/rm -f "$CLAUDE_CONFIG_DIR/market" ;;
  *) exit 1 ;;
esac
"#).unwrap();
    fs::set_permissions(&binary, fs::Permissions::from_mode(0o755)).unwrap();
    binary
}

#[test]
#[cfg(unix)]
fn claude_install_uninstall_with_fake_cli() {
    let _lock = ENV.lock().unwrap();
    let temp = assert_fs::TempDir::new().unwrap();
    // The provisioned binary path is canonical (macOS temp dirs live behind /var -> /private/var).
    let root = &Utf8PathBuf::from_path_buf(fs::canonicalize(temp.path()).unwrap()).unwrap();
    let fake = fake_claude(root);
    let _env = Env::set(&[
        ("UNELOS_CONFIG_DIR", root.join("unelos").as_str()),
        ("CLAUDE_CONFIG_DIR", root.join("claude").as_str()),
        ("UNELOS_CLAUDE_BIN", fake.as_str()),
        ("OPENCODE_CONFIG_DIR", root.join("opencode").as_str()),
        ("HOME", root.as_str()),
        ("PATH", root.join("empty").as_str()),
    ]);
    // A broken OpenCode config must not prevent Claude installation.
    fs::create_dir_all(root.join("opencode")).unwrap();
    fs::write(root.join("opencode/opencode.json"), "invalid").unwrap();
    let source = root.join("unelos source");
    fs::write(&source, "binary").unwrap();
    install_harness("claude", &source).unwrap();
    let cx = context().unwrap();
    let record = claude::Claude.integration(&cx).unwrap().unwrap();
    assert_eq!(record.files.len(), 8);
    let mcp: serde_json::Value = serde_json::from_slice(
        &fs::read(root.join("unelos/integrations/claude/unelos/.mcp.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(mcp["mcpServers"]["unelos"]["command"], cx.binary.as_str());
    install_harness("claude", &source).unwrap();
    let calls = fs::read_to_string(root.join("claude/calls")).unwrap();
    assert!(calls
        .lines()
        .any(|line| line == "plugin marketplace update unelos-local"));
    assert!(calls
        .lines()
        .any(|line| line == "plugin update unelos@unelos-local --scope user"));
    assert_eq!(
        calls
            .lines()
            .filter(|s| s.starts_with("plugin marketplace add "))
            .count(),
        1
    );
    assert_eq!(
        calls
            .lines()
            .filter(|s| s.starts_with("plugin install "))
            .count(),
        1
    );
    fs::write(
        root.join("claude/settings.json"),
        r#"{"enabledPlugins":{"unelos@unelos-local":false}}"#,
    )
    .unwrap();
    assert!(claude::Claude.integration(&cx).unwrap().is_none());
    claude::Claude.uninstall(&cx).unwrap();
    assert!(!global_config::read_state().unwrap().contains_key("claude"));
    assert!(record.files.iter().all(|path| !path.exists()));
    assert!(cx.binary.exists());
}

#[test]
#[cfg(unix)]
fn failed_workflow_export_preserves_existing_profiles() {
    let _lock = ENV.lock().unwrap();
    let temp = assert_fs::TempDir::new().unwrap();
    let root = Utf8Path::from_path(temp.path()).unwrap();
    let fake = fake_claude(root);
    let _env = Env::set(&[
        ("UNELOS_CONFIG_DIR", root.join("unelos").as_str()),
        ("CLAUDE_CONFIG_DIR", root.join("claude").as_str()),
        ("UNELOS_CLAUDE_BIN", fake.as_str()),
        ("OPENCODE_CONFIG_DIR", root.join("opencode").as_str()),
        ("HOME", root.as_str()),
        ("PATH", root.join("empty").as_str()),
    ]);
    let source = root.join("source");
    fs::write(&source, "binary").unwrap();
    install_harness("claude", &source).unwrap();
    crate::install_workflow(
        root,
        Some(
            &Utf8Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures/simple-plan-execute-review"),
        ),
        "simple-plan-execute-review",
    )
    .unwrap();
    fs::create_dir_all(root.join(".unelos/workflows")).unwrap();
    fs::write(root.join(".unelos/workflows/broken.yaml"), "invalid").unwrap();
    fs::create_dir_all(root.join(".claude/agents")).unwrap();
    let profile = root.join(".claude/agents/unelos-existing.md");
    fs::write(&profile, "keep").unwrap();
    let valid = root.join(".claude/agents/unelos-simple-controller.md");
    fs::write(&valid, "outdated").unwrap();
    let result = crate::build_harness_config(root).unwrap();
    assert!(!result.skipped.is_empty());
    assert!(result.removed.is_empty());
    assert_eq!(fs::read_to_string(profile).unwrap(), "keep");
    assert!(!result.written.contains(&valid));
    assert_eq!(fs::read_to_string(valid).unwrap(), "outdated");
}

#[test]
fn claude_agent_model_mapping_and_mcp_list() {
    use crate::{AgentExport, AgentModel, McpServer};
    let _lock = ENV.lock().unwrap();
    let temp = assert_fs::TempDir::new().unwrap();
    let root = Utf8Path::from_path(temp.path()).unwrap();
    let _env = Env::set(&[("UNELOS_CONFIG_DIR", root.join("global").as_str())]);
    crate::save_model_mapping(
        Some("smart"),
        "smart",
        [("claude".into(), "opus".into())].into(),
    )
    .unwrap();
    let mut agent = AgentExport {
        id: "test".into(),
        instructions: "Do work".into(),
        model: Some(AgentModel::Alias("smart".into())),
        effort: None,
        tools: vec![],
        mcps: vec!["local".into(), "remote".into()],
        mcp_servers: [
            (
                "local".into(),
                McpServer::Local {
                    command: vec!["server".into(), "--flag".into()],
                    environment: [("TOKEN".into(), "secret".into())].into(),
                },
            ),
            (
                "remote".into(),
                McpServer::Remote {
                    url: "https://example.com/mcp".into(),
                    headers: [("Authorization".into(), "Bearer token".into())].into(),
                },
            ),
        ]
        .into(),
        main: true,
        subagent: true,
    };
    let path = root.join(".claude/agents/unelos-test.md");
    for (model, expected) in [
        (agent.model.clone(), Some("opus")),
        (
            Some(AgentModel::Harness {
                harness: "claude".into(),
                model: "sonnet".into(),
            }),
            Some("sonnet"),
        ),
        (
            Some(AgentModel::Harness {
                harness: "opencode".into(),
                model: "p/m".into(),
            }),
            None,
        ),
        (None, None),
    ] {
        agent.model = model;
        claude::Claude
            .build_project(
                root,
                &[("workflow".into(), vec![agent.clone()])].into(),
                &[],
                true,
            )
            .unwrap();
        let text = fs::read_to_string(&path).unwrap();
        let front: serde_json::Value =
            serde_yaml_ng::from_str(text.split("---").nth(1).unwrap()).unwrap();
        assert_eq!(front["name"], "unelos-test");
        assert_eq!(front.get("model").and_then(|v| v.as_str()), expected);
        let servers = front["mcpServers"].as_array().unwrap();
        assert_eq!(servers[0]["local"]["type"], "stdio");
        assert_eq!(servers[0]["local"]["command"], "server");
        assert_eq!(servers[0]["local"]["args"][0], "--flag");
        assert_eq!(servers[0]["local"]["env"]["TOKEN"], "secret");
        assert_eq!(servers[1]["remote"]["type"], "http");
        assert_eq!(
            servers[1]["remote"]["headers"]["Authorization"],
            "Bearer token"
        );
    }
    assert_eq!(
        claude::Claude.list_models().unwrap(),
        ["fable", "opus", "sonnet", "haiku"].map(|id| ModelInfo {
            id: id.into(),
            efforts: ["low", "medium", "high", "xhigh", "max"]
                .map(str::to_owned)
                .into(),
        })
    );
}

#[test]
fn adapter_write_failure_restores_existing_files() {
    let temp = assert_fs::TempDir::new().unwrap();
    let root = Utf8Path::from_path(temp.path()).unwrap();
    fs::create_dir_all(root.join(".claude/agents")).unwrap();
    crate::harness_build::write_agents(
        root,
        ".claude/agents",
        vec![
            (".claude/agents/unelos-a.md".into(), "original a".into()),
            (".claude/agents/unelos-b.md".into(), "original b".into()),
        ],
        vec![],
        true,
    )
    .unwrap();
    fs::write(root.join(".claude/agents/unelos-stale.md"), "stale").unwrap();
    fs::write(
        root.join(format!(
            ".claude/agents/.unelos-b.md.unelos-{}.tmp",
            std::process::id()
        )),
        "blocked",
    )
    .unwrap();
    assert!(crate::harness_build::write_agents(
        root,
        ".claude/agents",
        vec![
            (".claude/agents/unelos-a.md".into(), "new a".into()),
            (".claude/agents/unelos-b.md".into(), "new b".into()),
        ],
        vec![],
        true
    )
    .is_err());
    assert_eq!(
        fs::read_to_string(root.join(".claude/agents/unelos-a.md")).unwrap(),
        "original a"
    );
    assert_eq!(
        fs::read_to_string(root.join(".claude/agents/unelos-b.md")).unwrap(),
        "original b"
    );
    assert_eq!(
        fs::read_to_string(root.join(".claude/agents/unelos-stale.md")).unwrap(),
        "stale"
    );
}

#[test]
#[cfg(unix)]
fn builds_both_harnesses_and_unmapped_claude_keeps_files() {
    use assert_fs::fixture::PathCopy;
    let _lock = ENV.lock().unwrap();
    let temp = assert_fs::TempDir::new().unwrap();
    let root = Utf8Path::from_path(temp.path()).unwrap();
    let fake = fake_claude(root);
    let _env = Env::set(&[
        ("UNELOS_CONFIG_DIR", root.join("global").as_str()),
        ("OPENCODE_CONFIG_DIR", root.join("opencode").as_str()),
        ("CLAUDE_CONFIG_DIR", root.join("claude").as_str()),
        ("UNELOS_CLAUDE_BIN", fake.as_str()),
        ("HOME", root.as_str()),
        ("PATH", root.join("empty").as_str()),
    ]);
    fs::create_dir(root.join(".unelos")).unwrap();
    assert_fs::fixture::ChildPath::new(root.join(".unelos"))
        .copy_from(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures/simple-plan-execute-review"),
            &["**/*"],
        )
        .unwrap();
    crate::initialize_project(root).unwrap();
    let source = root.join("cli");
    fs::write(&source, "binary").unwrap();
    install_harness("opencode", &source).unwrap();
    install_harness("claude", &source).unwrap();
    let mut legacy_copies = Vec::new();
    for harness in ["opencode", "claude"] {
        let directory = root.join(format!(".{harness}")).join("agents");
        fs::create_dir_all(&directory).unwrap();
        let name = "unelos-global-workflow-author.md";
        let source = root.join(harness).join("agents").join(name);
        let copy = directory.join(name);
        let text = fs::read_to_string(source).unwrap();
        fs::write(&copy, text.replace("mode: all", "mode: subagent")).unwrap();
        legacy_copies.push(copy);
    }
    crate::save_model_mapping(
        Some("smart"),
        "smart",
        [
            ("opencode".into(), "p/m".into()),
            ("claude".into(), "opus".into()),
        ]
        .into(),
    )
    .unwrap();
    let source_agent = root.join(".unelos/agents/simple-implementer/agent.yaml");
    let original = fs::read_to_string(&source_agent).unwrap();
    fs::write(&source_agent, format!("{original}\nmodel: smart\n")).unwrap();
    let built = crate::build_harness_config(root).unwrap();
    assert_eq!(built.harnesses, ["opencode", "claude"]);
    assert!(built.skipped.is_empty());
    assert_eq!(built.removed, legacy_copies);
    for harness in ["opencode", "claude"] {
        let receipt = root
            .join(format!(".{harness}"))
            .join("agents")
            .join(".unelos-owned-agents.json");
        fs::remove_file(&receipt).unwrap();
        crate::build_harness_config(root).unwrap();
        assert!(receipt.is_file());
    }
    let claude_agent = root.join(".claude/agents/unelos-simple-implementer.md");
    let before = fs::read(&claude_agent).unwrap();
    let stale = root.join(".claude/agents/unelos-stale.md");
    fs::write(&stale, "keep").unwrap();
    crate::save_model_mapping(
        Some("smart"),
        "smart",
        [("opencode".into(), "p/new".into())].into(),
    )
    .unwrap();
    let built = crate::build_harness_config(root).unwrap();
    assert_eq!(built.harnesses, ["opencode", "claude"]);
    assert_eq!(built.skipped.len(), 1);
    assert!(built.skipped[0].starts_with("claude: No Claude model mapped"));
    assert_eq!(fs::read(&claude_agent).unwrap(), before);
    assert_eq!(fs::read_to_string(stale).unwrap(), "keep");
    assert!(
        fs::read_to_string(root.join(".opencode/agents/unelos-simple-implementer.md"))
            .unwrap()
            .contains("p/new")
    );
}

struct Env(Vec<(String, Option<std::ffi::OsString>)>);
impl Env {
    fn set(values: &[(&str, &str)]) -> Self {
        let saved = values
            .iter()
            .map(|(key, value)| {
                let old = std::env::var_os(key);
                std::env::set_var(key, value);
                (key.to_string(), old)
            })
            .collect();
        Self(saved)
    }
}
impl Drop for Env {
    fn drop(&mut self) {
        for (key, value) in &self.0 {
            if let Some(value) = value {
                std::env::set_var(key, value);
            } else {
                std::env::remove_var(key);
            }
        }
    }
}

#[test]
fn semver_extraction() {
    assert_eq!(
        parse_version("OpenCode v1.2.3-beta.1 (build)"),
        Some("1.2.3-beta.1".into())
    );
    assert_eq!(parse_version("claude 2.12.8"), Some("2.12.8".into()));
    assert_eq!(parse_version("not a version 1.2"), None);
}

#[test]
fn agent_models_resolve_global_aliases_for_opencode() {
    use crate::{harness::opencode::resolve_agent_model, AgentModel};
    let _lock = ENV.lock().unwrap();
    let temp = assert_fs::TempDir::new().unwrap();
    let root = Utf8Path::from_path(temp.path()).unwrap();
    let _env = Env::set(&[("UNELOS_CONFIG_DIR", root.as_str())]);
    let alias = AgentModel::Alias("smart".into());
    assert_eq!(
        resolve_agent_model(&alias, None).unwrap_err().code,
        "unmapped_model_alias"
    );
    crate::save_model_mapping(
        Some("smart"),
        "smart",
        [("opencode".into(), "provider/model#high".into())].into(),
    )
    .unwrap();
    assert_eq!(
        resolve_agent_model(&alias, None).unwrap(),
        "provider/model#high"
    );
    assert_eq!(
        resolve_agent_model(&alias, Some("default")).unwrap(),
        "provider/model"
    );
    assert_eq!(
        resolve_agent_model(
            &AgentModel::Harness {
                harness: "claude".into(),
                model: "x".into()
            },
            None
        )
        .unwrap_err()
        .code,
        "model_harness_mismatch"
    );
}

#[test]
fn resolve_effort_matrix() {
    use crate::{model_mappings::resolve_harness_model, AgentModel, MappedModel};
    let _lock = ENV.lock().unwrap();
    let temp = assert_fs::TempDir::new().unwrap();
    let root = Utf8Path::from_path(temp.path()).unwrap();
    let _env = Env::set(&[("UNELOS_CONFIG_DIR", root.as_str())]);
    crate::save_model_mapping(
        Some("smart"),
        "smart",
        [
            ("opencode".into(), "p/m#high".into()),
            ("claude".into(), "opus#max".into()),
        ]
        .into(),
    )
    .unwrap();
    let alias = AgentModel::Alias("smart".into());
    let direct = AgentModel::Harness {
        harness: "claude".into(),
        model: "sonnet".into(),
    };
    for (model, effort, harness, expected) in [
        (&alias, None, "opencode", Some("p/m#high")),
        (&alias, None, "claude", Some("opus#max")),
        (&alias, Some("low"), "claude", Some("opus#low")),
        (&alias, Some("default"), "opencode", Some("p/m")),
        (&alias, Some("default"), "claude", Some("opus")),
        (&direct, None, "claude", Some("sonnet")),
        (&direct, Some("high"), "claude", Some("sonnet#high")),
        (&direct, Some("default"), "claude", Some("sonnet")),
        (&direct, Some("high"), "opencode", None),
    ] {
        assert_eq!(
            resolve_harness_model(model, effort, harness).unwrap(),
            expected.map(MappedModel::from)
        );
    }
    for (harness, name) in [("opencode", "OpenCode"), ("claude", "Claude")] {
        let err =
            resolve_harness_model(&AgentModel::Alias("missing".into()), None, harness).unwrap_err();
        assert_eq!(err.code, "unmapped_model_alias");
        assert_eq!(
            err.message,
            format!("No {name} model mapped for alias 'missing'")
        );
    }
}

fn effort_agent(model: crate::AgentModel, effort: Option<&str>) -> crate::AgentExport {
    crate::AgentExport {
        id: "effort".into(),
        instructions: "Work".into(),
        model: Some(model),
        effort: effort.map(str::to_owned),
        tools: vec![],
        mcps: vec![],
        mcp_servers: Default::default(),
        main: true,
        subagent: true,
    }
}

#[test]
fn opencode_renders_effort_v1_v2() {
    use crate::{
        opencode_agents::{render_agents, Version},
        AgentModel,
    };
    for (model, effort, expected) in [
        ("p/m", Some("max"), "p/m#max"),
        ("p/m", None, "p/m"),
        ("p/m", Some("default"), "p/m"),
        ("p/m#high", None, "p/m#high"),
    ] {
        let agent = effort_agent(
            AgentModel::Harness {
                harness: "opencode".into(),
                model: model.into(),
            },
            effort,
        );
        for version in [Version::V1, Version::V2] {
            let rendered = render_agents(version, std::slice::from_ref(&agent)).unwrap();
            let front: serde_json::Value =
                serde_yaml_ng::from_str(rendered[0].1.split("---").nth(1).unwrap()).unwrap();
            let (base, variant) = expected
                .split_once('#')
                .map_or((expected, None), |(m, v)| (m, Some(v)));
            assert_eq!(
                front["model"],
                if version == Version::V2 {
                    expected
                } else {
                    base
                }
            );
            assert_eq!(
                front.get("variant").and_then(|v| v.as_str()),
                if version == Version::V1 {
                    variant
                } else {
                    None
                }
            );
        }
    }
    for effort in ["max", "default"] {
        let agent = effort_agent(
            AgentModel::Harness {
                harness: "opencode".into(),
                model: "p/m#high".into(),
            },
            Some(effort),
        );
        assert_eq!(
            render_agents(Version::V2, std::slice::from_ref(&agent))
                .unwrap_err()
                .code,
            "invalid_model"
        );
        assert_eq!(
            crate::opencode_agents::check_agents(Utf8Path::new("."), &[agent])
                .unwrap_err()
                .code,
            "invalid_model"
        );
    }
}

#[test]
fn claude_renders_effort() {
    use crate::AgentModel;
    let _lock = ENV.lock().unwrap();
    let temp = assert_fs::TempDir::new().unwrap();
    let root = Utf8Path::from_path(temp.path()).unwrap();
    let _env = Env::set(&[("UNELOS_CONFIG_DIR", root.join("global").as_str())]);
    crate::save_model_mapping(
        Some("smart"),
        "smart",
        [("claude".into(), "opus#max".into())].into(),
    )
    .unwrap();
    for (effort, expected) in [
        (None, Some("max")),
        (Some("default"), None),
        (Some("low"), Some("low")),
        (Some("medium"), Some("medium")),
        (Some("high"), Some("high")),
        (Some("xhigh"), Some("xhigh")),
    ] {
        let agent = effort_agent(AgentModel::Alias("smart".into()), effort);
        claude::Claude
            .build_project(root, &[("workflow".into(), vec![agent])].into(), &[], true)
            .unwrap();
        let text = fs::read_to_string(root.join(".claude/agents/unelos-effort.md")).unwrap();
        let front: serde_json::Value =
            serde_yaml_ng::from_str(text.split("---").nth(1).unwrap()).unwrap();
        assert_eq!(front["model"], "opus");
        assert_eq!(front.get("effort").and_then(|v| v.as_str()), expected);
    }
    let path = root.join(".claude/agents/unelos-effort.md");
    let before = fs::read(&path).unwrap();
    let agent = effort_agent(AgentModel::Alias("smart".into()), Some("none"));
    assert_eq!(
        claude::Claude
            .build_project(root, &[("workflow".into(), vec![agent])].into(), &[], true)
            .unwrap_err()
            .code,
        "invalid_effort"
    );
    assert_eq!(fs::read(path).unwrap(), before);
}

#[test]
fn model_mapping_lifecycle() {
    use crate::{delete_model_mapping, list_model_mappings, save_model_mapping};
    use std::collections::BTreeMap;
    let _lock = ENV.lock().unwrap();
    let temp = assert_fs::TempDir::new().unwrap();
    let root = Utf8Path::from_path(temp.path()).unwrap();
    let _env = Env::set(&[("UNELOS_CONFIG_DIR", root.as_str())]);
    let defaults = list_model_mappings().unwrap();
    assert_eq!(
        defaults.keys().collect::<Vec<_>>(),
        ["balanced", "fast", "smart"]
    );
    assert!(root.join("models.json").is_file());
    for alias in ["smart", "balanced", "fast"] {
        delete_model_mapping(alias).unwrap();
    }
    assert!(list_model_mappings().unwrap().is_empty());
    let models = BTreeMap::from([
        ("opencode".into(), "provider/model".into()),
        ("claude".into(), "".into()),
    ]);
    let saved = save_model_mapping(None, "smart", models.clone()).unwrap();
    assert_eq!(saved["smart"].len(), 1);
    assert_eq!(list_model_mappings().unwrap(), saved);
    let json: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(root.join("models.json")).unwrap()).unwrap();
    assert!(json["smart"]["opencode"].get("effort").is_none());
    for effort in ["a b", "x#y"] {
        let err = save_model_mapping(
            Some("smart"),
            "smart",
            BTreeMap::from([(
                "opencode".into(),
                crate::MappedModel {
                    model: "p/m".into(),
                    effort: Some(effort.into()),
                },
            )]),
        )
        .unwrap_err();
        assert_eq!(err.code, "invalid_effort");
    }
    assert_eq!(
        save_model_mapping(
            Some("smart"),
            "smart",
            BTreeMap::from([(
                "opencode".into(),
                crate::MappedModel {
                    model: "p/m#high".into(),
                    effort: None
                }
            )])
        )
        .unwrap_err()
        .code,
        "invalid_model"
    );
    let normalized = save_model_mapping(
        Some("smart"),
        "smart",
        BTreeMap::from([(
            "opencode".into(),
            crate::MappedModel {
                model: "p/m".into(),
                effort: Some("".into()),
            },
        )]),
    )
    .unwrap();
    assert_eq!(normalized["smart"]["opencode"].effort, None);
    for claude in ["opus", "sonnet"] {
        let updated = save_model_mapping(
            Some("smart"),
            "smart",
            BTreeMap::from([
                ("opencode".into(), crate::MappedModel::from("p/m#high")),
                ("claude".into(), claude.into()),
            ]),
        )
        .unwrap();
        assert_eq!(updated["smart"]["opencode"].effort.as_deref(), Some("high"));
    }
    save_model_mapping(Some("smart"), "smart", models.clone()).unwrap();
    assert!(save_model_mapping(None, "smart", models.clone()).is_err());
    assert!(save_model_mapping(Some("missing"), "new", models.clone()).is_err());
    for invalid in [
        "",
        "Smart",
        "-smart",
        "smart-",
        "smart..fast",
        "smart fast",
        "é",
    ] {
        assert!(save_model_mapping(None, invalid, models.clone()).is_err());
    }
    for invalid in ["a b", " a", "a\n", "a\u{85}b"] {
        assert!(save_model_mapping(
            Some("smart"),
            "smart",
            BTreeMap::from([("opencode".into(), invalid.into())])
        )
        .is_err());
    }
    assert_eq!(list_model_mappings().unwrap(), saved);
    save_model_mapping(None, "fast", BTreeMap::new()).unwrap();
    assert!(save_model_mapping(Some("smart"), "fast", models.clone()).is_err());
    save_model_mapping(Some("smart"), "smart", models.clone()).unwrap();
    let renamed = save_model_mapping(Some("smart"), "smart-fast_v2.0", models).unwrap();
    assert!(!renamed.contains_key("smart"));
    assert!(renamed.contains_key("smart-fast_v2.0"));
    assert!(delete_model_mapping("missing").is_err());
    delete_model_mapping("fast").unwrap();
    assert!(delete_model_mapping("smart-fast_v2.0").unwrap().is_empty());
    fs::write(root.join("models.json"), "not json").unwrap();
    assert!(list_model_mappings().is_err());
    assert!(save_model_mapping(None, "new", BTreeMap::new()).is_err());
    assert_eq!(
        fs::read_to_string(root.join("models.json")).unwrap(),
        "not json"
    );
}

#[test]
fn legacy_model_mappings_load() {
    let _lock = ENV.lock().unwrap();
    let temp = assert_fs::TempDir::new().unwrap();
    let root = Utf8Path::from_path(temp.path()).unwrap();
    let _env = Env::set(&[("UNELOS_CONFIG_DIR", root.as_str())]);
    let bytes = br#"{"smart":{"opencode":"p/m#high","claude":"opus"},"fast":{"claude":{"model":"haiku","effort":"low"}}}"#;
    fs::write(root.join("models.json"), bytes).unwrap();
    let mappings = crate::list_model_mappings().unwrap();
    assert_eq!(
        mappings["smart"]["opencode"],
        crate::MappedModel {
            model: "p/m".into(),
            effort: Some("high".into())
        }
    );
    assert_eq!(
        mappings["smart"]["claude"],
        crate::MappedModel {
            model: "opus".into(),
            effort: None
        }
    );
    assert_eq!(mappings["fast"]["claude"].effort.as_deref(), Some("low"));
    assert_eq!(fs::read(root.join("models.json")).unwrap(), bytes);
}

#[test]
fn global_lifecycle() {
    let _lock = ENV.lock().unwrap();
    let temp = assert_fs::TempDir::new().unwrap();
    let root = Utf8Path::from_path(temp.path()).unwrap();
    let global = root.join("unelos");
    let oc = root.join("opencode");
    let bins = root.join("bins");
    fs::create_dir_all(&bins).unwrap();
    let _env = Env::set(&[
        ("UNELOS_CONFIG_DIR", global.as_str()),
        ("OPENCODE_CONFIG_DIR", oc.as_str()),
        ("HOME", root.as_str()),
        ("XDG_CONFIG_HOME", root.as_str()),
        ("PATH", bins.as_str()),
        ("CLAUDE_CONFIG_DIR", root.join("claude").as_str()),
        ("CODEX_HOME", root.join("codex").as_str()),
        ("PI_CODING_AGENT_DIR", root.join("pi").as_str()),
        ("COPILOT_HOME", root.join("copilot").as_str()),
        ("CURSOR_CONFIG_DIR", root.join("cursor").as_str()),
        ("KIMI_CODE_HOME", root.join("kimi").as_str()),
        ("QWEN_HOME", root.join("qwen").as_str()),
    ]);
    let statuses = list_harnesses();
    assert_eq!(
        statuses.iter().map(|s| s.id.as_str()).collect::<Vec<_>>(),
        [
            "opencode", "claude", "codex", "pi", "copilot", "cursor", "droid", "kimi", "qwen",
            "gemini", "amp", "cline"
        ]
    );
    assert!(statuses.iter().all(|s| !s.detected && !s.installed));
    fs::create_dir(root.join("claude")).unwrap();
    assert!(list_harnesses()[1].detected);
    assert!(list_harnesses()[1].installable);
    assert!(install_harness("claude", &root.join("missing")).is_err());
    assert!(harness_models("unknown").is_err());
    fs::create_dir_all(&oc).unwrap();
    let config = oc.join("opencode.jsonc");
    let original = "{\n // keep me\n \"model\": \"p/m\",\n \"plugins\": [\"foreign\", {\"package\":\"other\",\"options\":{\"a\":1}},],\n}\n";
    fs::write(&config, original).unwrap();
    let src = root.join("cli");
    fs::write(&src, b"test executable").unwrap();
    assert!(global_config::config_dir().unwrap().starts_with(root));
    assert!(opencode::OpenCode.config_dir().unwrap().starts_with(root));
    let statuses = install_harness("opencode", &src).unwrap();
    assert!(statuses[0].installed);
    let bytes = fs::read(&config).unwrap();
    let state = fs::read(global.join("harnesses.json")).unwrap();
    let mtime = fs::metadata(&config).unwrap().modified().unwrap();
    install_harness("opencode", &src).unwrap();
    assert_eq!(fs::read(&config).unwrap(), bytes);
    assert_eq!(fs::read(global.join("harnesses.json")).unwrap(), state);
    assert_eq!(fs::metadata(&config).unwrap().modified().unwrap(), mtime);
    assert!(String::from_utf8(bytes).unwrap().contains("// keep me"));
    let stable = provision_cli(&src).unwrap();
    let before = fs::metadata(&stable).unwrap().modified().unwrap();
    assert_eq!(provision_cli(&src).unwrap(), stable);
    assert_eq!(fs::metadata(&stable).unwrap().modified().unwrap(), before);
    let mut records = global_config::read_state().unwrap();
    records.get_mut("opencode").unwrap().version = "0.0.0".into();
    global_config::write_state(&records).unwrap();
    assert!(list_harnesses()[0].update_available);
    let legacy = global.join("integrations/opencode/unelos/index.js");
    fs::write(&legacy, "old plugin").unwrap();
    let mut records = global_config::read_state().unwrap();
    let files = &mut records.get_mut("opencode").unwrap().files;
    files.retain(|file| !file.ends_with("index.ts"));
    files.push(legacy.clone());
    global_config::write_state(&records).unwrap();
    install_harness("opencode", &src).unwrap();
    assert!(!legacy.exists());
    assert!(global
        .join("integrations/opencode/unelos/index.ts")
        .exists());
    assert!(!global_config::read_state().unwrap()["opencode"]
        .files
        .contains(&legacy));
    let foreign = global.join("integrations/opencode/unelos/foreign.txt");
    fs::write(&foreign, "keep").unwrap();
    let mut records = global_config::read_state().unwrap();
    records
        .get_mut("opencode")
        .unwrap()
        .files
        .push(foreign.clone());
    global_config::write_state(&records).unwrap();
    assert!(uninstall_harness("opencode").is_err());
    assert!(foreign.exists());
    records.get_mut("opencode").unwrap().files.pop();
    global_config::write_state(&records).unwrap();
    assert!(!uninstall_harness("opencode").unwrap()[0].installed);
    assert!(foreign.exists());
    assert!(stable.exists());
    assert!(!global
        .join("integrations/opencode/unelos/index.ts")
        .exists());
    let text = fs::read_to_string(&config).unwrap();
    assert!(text.contains("// keep me"));
    let value: serde_json::Value =
        jsonc_parser::parse_to_serde_value(&text, &Default::default()).unwrap();
    assert_eq!(
        value["plugins"],
        serde_json::json!(["foreign", {"package":"other","options":{"a":1}}])
    );
    uninstall_harness("opencode").unwrap();
    assert_eq!(fs::read_to_string(&config).unwrap(), text);
    install_harness("opencode", &src).unwrap();
    let current = global.join("integrations/opencode/unelos/index.ts");
    fs::rename(&current, &legacy).unwrap();
    let mut records = global_config::read_state().unwrap();
    for file in &mut records.get_mut("opencode").unwrap().files {
        if *file == current {
            *file = legacy.clone();
        }
    }
    global_config::write_state(&records).unwrap();
    uninstall_harness("opencode").unwrap();
    assert!(!legacy.exists());
    fs::write(oc.join("opencode.json"), "{}").unwrap();
    assert!(install_harness("opencode", &src).is_err());
}

#[cfg(unix)]
#[test]
fn path_detection_version_models_and_timeout() {
    use std::os::unix::fs::PermissionsExt;
    let _lock = ENV.lock().unwrap();
    let temp = assert_fs::TempDir::new().unwrap();
    let root = Utf8Path::from_path(temp.path()).unwrap();
    let _env = Env::set(&[
        ("PATH", root.as_str()),
        ("HOME", root.as_str()),
        ("UNELOS_CONFIG_DIR", root.as_str()),
        ("OPENCODE_CONFIG_DIR", root.join("config").as_str()),
    ]);
    let bin = root.join("opencode");
    fs::write(&bin, "#!/bin/sh\nif [ \"$1\" = models ]; then printf 'a/model\\nb/model\\n'; else echo 'opencode v2.3.4'; fi\n").unwrap();
    fs::set_permissions(&bin, fs::Permissions::from_mode(0o755)).unwrap();
    assert_eq!(list_harnesses()[0].version.as_deref(), Some("2.3.4"));
    assert_eq!(
        harness_models("opencode").unwrap(),
        ["a/model", "b/model"].map(|id| ModelInfo {
            id: id.into(),
            efforts: vec![],
        })
    );
    fs::write(&bin, format!("#!/bin/sh\nif [ \"$1\" = api ] && [ \"$2\" = model.list ]; then printf '%s' '{}'; else exit 1; fi\n", MODEL_LIST)).unwrap();
    assert_eq!(harness_models("opencode").unwrap(), expected_model_list());
    fs::write(&bin, "#!/bin/sh\nwhile :; do :; done\n").unwrap();
    let start = Instant::now();
    assert_eq!(opencode::OpenCode.version(&bin), None);
    assert!(start.elapsed() < Duration::from_secs(4));
}

const MODEL_LIST: &str = r#"{"location":"x","data":[{"providerID":"openai","id":"gpt","variants":[{"id":"none"},{"id":"high"}]},{"providerID":"anthropic","id":"opus","variants":[]}]}"#;

fn expected_model_list() -> Vec<ModelInfo> {
    vec![
        ModelInfo {
            id: "anthropic/opus".into(),
            efforts: vec![],
        },
        ModelInfo {
            id: "openai/gpt".into(),
            efforts: vec!["none".into(), "high".into()],
        },
    ]
}

#[test]
fn parse_opencode_model_list_reads_variants() {
    assert_eq!(
        opencode::parse_opencode_model_list(MODEL_LIST),
        Some(expected_model_list())
    );
    for invalid in [
        "not json",
        "{}",
        r#"{"data":[{"providerID":"p","id":"m"}]}"#,
        r#"{"data":[{"providerID":"p","id":"m","variants":[{}]}]}"#,
    ] {
        assert_eq!(opencode::parse_opencode_model_list(invalid), None);
    }
    assert_eq!(
        opencode::parse_opencode_model_list(r#"{"data":[]}"#),
        Some(vec![])
    );
}

#[test]
fn claude_models_have_static_efforts() {
    let models = harness_models("claude").unwrap();
    assert_eq!(
        models.iter().map(|m| m.id.as_str()).collect::<Vec<_>>(),
        ["fable", "opus", "sonnet", "haiku"]
    );
    for model in models {
        assert_eq!(model.efforts, ["low", "medium", "high", "xhigh", "max"]);
    }
}

#[test]
fn config_precedence() {
    let _lock = ENV.lock().unwrap();
    let _env = Env::set(&[
        ("UNELOS_CONFIG_DIR", "/chosen"),
        ("XDG_CONFIG_HOME", "/xdg"),
        ("HOME", "/home/test"),
    ]);
    assert_eq!(global_config::config_dir().unwrap(), "/chosen");
    std::env::remove_var("UNELOS_CONFIG_DIR");
    assert_eq!(global_config::config_dir().unwrap(), "/xdg/unelos");
    std::env::remove_var("XDG_CONFIG_HOME");
    assert_eq!(
        global_config::config_dir().unwrap(),
        "/home/test/.config/unelos"
    );
}
