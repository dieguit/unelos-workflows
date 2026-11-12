use assert_fs::{fixture::PathCopy, TempDir};
use camino::Utf8Path;
use std::{fs, sync::Mutex};
use unelos_core::*;

static ENV: Mutex<()> = Mutex::new(());
struct Env(Vec<(String, Option<std::ffi::OsString>)>);
impl Env {
    fn set(values: &[(&str, &str)]) -> Self {
        Self(
            values
                .iter()
                .map(|(key, value)| {
                    let old = std::env::var_os(key);
                    std::env::set_var(key, value);
                    (key.to_string(), old)
                })
                .collect(),
        )
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
fn rebuild_merges_prunes_preserves_and_replaces_mcp_ownership() {
    let _lock = ENV.lock().unwrap();
    let temp = TempDir::new().unwrap();
    let root = Utf8Path::from_path(temp.path()).unwrap();
    let _env = Env::set(&[
        ("UNELOS_CONFIG_DIR", root.join("global").as_str()),
        ("OPENCODE_CONFIG_DIR", root.join("global-opencode").as_str()),
        ("HOME", root.as_str()),
        ("XDG_CONFIG_HOME", root.as_str()),
        ("PATH", root.join("bins").as_str()),
    ]);
    assert!(build_harness_config(root).unwrap().harnesses.is_empty());
    fs::create_dir(root.join(".unelos")).unwrap();
    let authoring = assert_fs::fixture::ChildPath::new(root.join(".unelos"));
    authoring
        .copy_from(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures/simple-plan-execute-review"),
            &["**/*"],
        )
        .unwrap();
    initialize_project(root).unwrap();
    let workflow = root.join(".unelos/workflows/simple-plan-execute-review.yaml");
    let text = fs::read_to_string(&workflow).unwrap();
    let second = root.join(".unelos/workflows/second.yaml");
    fs::write(
        &second,
        text.replace("name: simple-plan-execute-review", "name: second")
            .replace("execution: main", "execution: subagent"),
    )
    .unwrap();
    let agent = root.join(".unelos/agents/simple-implementer/agent.yaml");
    let original = fs::read_to_string(&agent).unwrap();
    fs::write(&agent, format!("{original}\nmcps: [test-server]\n")).unwrap();
    fs::create_dir_all(root.join(".unelos/mcps")).unwrap();
    save_mcp(
        root,
        None,
        McpDocument {
            id: "test-server".into(),
            server: McpServer::Local {
                command: vec!["server".into()],
                environment: Default::default(),
            },
        },
    )
    .unwrap();
    let cli = root.join("cli");
    fs::write(&cli, "test executable").unwrap();
    harness::install_harness("opencode", &cli).unwrap();
    fs::create_dir_all(root.join(".opencode/agents")).unwrap();
    let foreign = root.join(".opencode/agents/custom.md");
    fs::write(&foreign, "keep").unwrap();
    let built = build_harness_config(root).unwrap();
    assert_eq!(built.harnesses, ["opencode"]);
    assert_eq!(
        built
            .written
            .iter()
            .filter(|p| p.extension() == Some("md"))
            .count(),
        3
    );
    let controller = root.join(".opencode/agents/unelos-simple-controller.md");
    assert!(fs::read_to_string(&controller)
        .unwrap()
        .contains("mode: all"));
    let before = fs::metadata(&controller).unwrap().modified().unwrap();
    assert!(build_harness_config(root).unwrap().written.is_empty());
    assert_eq!(
        before,
        fs::metadata(&controller).unwrap().modified().unwrap()
    );
    fs::write(
        root.join(".unelos/workflows/broken.yaml"),
        "invalid: workflow",
    )
    .unwrap();
    let built = build_harness_config(root).unwrap();
    assert_eq!(built.skipped.len(), 1);
    assert!(built.skipped[0].starts_with("broken: "));
    assert!(built.written.is_empty() && built.removed.is_empty());
    assert_eq!(
        before,
        fs::metadata(&controller).unwrap().modified().unwrap()
    );
    fs::remove_file(root.join(".unelos/workflows/broken.yaml")).unwrap();
    fs::remove_file(&second).unwrap();
    fs::write(
        &workflow,
        text.replace("agent: simple-reviewer", "agent: simple-implementer"),
    )
    .unwrap();
    let built = build_harness_config(root).unwrap();
    assert_eq!(
        built.removed,
        [root.join(".opencode/agents/unelos-simple-reviewer.md")]
    );
    assert_eq!(fs::read_to_string(&foreign).unwrap(), "keep");
    let state: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(root.join(".unelos/opencode-mcps.json")).unwrap())
            .unwrap();
    assert!(state.get("second").is_none());
    assert!(state["simple-plan-execute-review"]
        .get("test-server")
        .is_some());
    fs::remove_dir_all(root.join(".unelos/workflows")).unwrap();
    build_harness_config(root).unwrap();
    let config: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(root.join("opencode.json")).unwrap()).unwrap();
    assert!(config["mcp"].get("test-server").is_none());
    save_project_settings(
        root,
        &ProjectSettings {
            auto_build_harness: Some(false),
        },
    )
    .unwrap();
    assert_eq!(
        get_project_settings(root).unwrap().auto_build_harness,
        Some(false)
    );
    assert!(auto_build_harness(root).unwrap().is_none());
}
