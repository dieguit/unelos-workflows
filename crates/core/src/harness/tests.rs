use super::*;
use std::sync::Mutex;
static ENV: Mutex<()> = Mutex::new(());

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
    use crate::{AgentModel, install::resolve_agent_model};
    let _lock = ENV.lock().unwrap();
    let temp = assert_fs::TempDir::new().unwrap();
    let root = Utf8Path::from_path(temp.path()).unwrap();
    let _env = Env::set(&[("UNELOS_CONFIG_DIR", root.as_str())]);
    let alias = AgentModel::Alias("smart".into());
    assert_eq!(resolve_agent_model(&alias).unwrap_err().code, "unmapped_model_alias");
    crate::save_model_mapping(Some("smart"), "smart", [("opencode".into(), "provider/model".into())].into()).unwrap();
    assert_eq!(resolve_agent_model(&alias).unwrap(), "provider/model");
    assert_eq!(resolve_agent_model(&AgentModel::Harness { harness: "claude".into(), model: "x".into() }).unwrap_err().code, "model_harness_mismatch");
}

#[test]
fn model_mapping_lifecycle() {
    use std::collections::BTreeMap;
    use crate::{delete_model_mapping, list_model_mappings, save_model_mapping};
    let _lock = ENV.lock().unwrap();
    let temp = assert_fs::TempDir::new().unwrap();
    let root = Utf8Path::from_path(temp.path()).unwrap();
    let _env = Env::set(&[("UNELOS_CONFIG_DIR", root.as_str())]);
    let defaults = list_model_mappings().unwrap();
    assert_eq!(defaults.keys().collect::<Vec<_>>(), ["balanced", "fast", "smart"]);
    assert!(root.join("models.json").is_file());
    for alias in ["smart", "balanced", "fast"] {
        delete_model_mapping(alias).unwrap();
    }
    assert!(list_model_mappings().unwrap().is_empty());
    let models = BTreeMap::from([("opencode".into(), "provider/model".into()), ("claude".into(), "".into())]);
    let saved = save_model_mapping(None, "smart", models.clone()).unwrap();
    assert_eq!(saved["smart"].len(), 1);
    assert_eq!(list_model_mappings().unwrap(), saved);
    assert!(save_model_mapping(None, "smart", models.clone()).is_err());
    assert!(save_model_mapping(Some("missing"), "new", models.clone()).is_err());
    for invalid in ["", "Smart", "-smart", "smart-", "smart..fast", "smart fast", "é"] {
        assert!(save_model_mapping(None, invalid, models.clone()).is_err());
    }
    for invalid in ["a b", " a", "a\n", "a\u{85}b"] {
        assert!(save_model_mapping(Some("smart"), "smart", BTreeMap::from([("opencode".into(), invalid.into())])).is_err());
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
    assert_eq!(fs::read_to_string(root.join("models.json")).unwrap(), "not json");
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
    assert!(!list_harnesses()[1].installable);
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
        .join("integrations/opencode/unelos/index.js")
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
    assert_eq!(harness_models("opencode").unwrap(), ["a/model", "b/model"]);
    fs::write(&bin, "#!/bin/sh\nwhile :; do :; done\n").unwrap();
    let start = Instant::now();
    assert_eq!(opencode::OpenCode.version(&bin), None);
    assert!(start.elapsed() < Duration::from_secs(4));
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
