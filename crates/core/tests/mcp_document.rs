use assert_fs::TempDir;
use camino::Utf8Path;
use serde_json::json;
use std::fs;
use unelos_core::{get_mcp, mcp_summaries, save_mcp, McpDocument};

fn document(value: serde_json::Value) -> McpDocument {
    serde_json::from_value(value).unwrap()
}

#[test]
fn roundtrip_validation_and_broken_summaries() {
    let temp = TempDir::new().unwrap();
    let root = Utf8Path::from_path(temp.path()).unwrap();
    for value in [
        json!({"id":"local","type":"local","command":["npx","server"],"environment":{"TOKEN":"{env:TOKEN}"}}),
        json!({"id":"remote","type":"remote","url":"https://example.com/mcp","headers":{"Authorization":"{env:TOKEN}"}}),
    ] {
        let mcp = document(value.clone());
        assert_eq!(save_mcp(root, None, mcp.clone()).unwrap(), mcp);
        assert_eq!(get_mcp(root, &mcp.id).unwrap(), mcp);
        assert_eq!(serde_json::to_value(&mcp).unwrap(), value);
        let yaml = fs::read_to_string(root.join(format!(".unelos/mcps/{}.yaml", mcp.id))).unwrap();
        assert!(!yaml.contains("id:"));
        assert!(save_mcp(root, None, mcp).is_err());
    }
    for value in [
        json!({"id":"bad","type":"local","command":[]}),
        json!({"id":"bad","type":"local","command":[""]}),
        json!({"id":"bad","type":"remote","url":""}),
        json!({"id":"bad","type":"remote","url":"file:///etc/passwd"}),
        json!({"id":"../bad","type":"remote","url":"https://example.com"}),
    ] {
        assert!(save_mcp(root, None, document(value)).is_err());
    }
    fs::write(root.join(".unelos/mcps/broken.yaml"), "type: [").unwrap();
    let rows = mcp_summaries(root).unwrap();
    assert_eq!(rows.len(), 3);
    assert!(rows[0].problem.is_some());
    assert_eq!(rows[1].target.as_deref(), Some("npx server"));
    assert_eq!(rows[2].kind.as_deref(), Some("remote"));
}

#[test]
fn rename_rewrites_only_agent_mcp_references_and_rejects_collisions() {
    let temp = TempDir::new().unwrap();
    let root = Utf8Path::from_path(temp.path()).unwrap();
    let mut mcp = document(json!({"id":"old","type":"local","command":["server"]}));
    save_mcp(root, None, mcp.clone()).unwrap();
    fs::create_dir_all(root.join(".unelos/agents/a")).unwrap();
    let file = root.join(".unelos/agents/a/agent.yaml");
    let text = "instructions: old\nskills: [old]\nmcps: [old, older] # old\n";
    fs::write(&file, text).unwrap();
    mcp.id = "new".into();
    save_mcp(root, Some("old"), mcp.clone()).unwrap();
    assert!(!root.join(".unelos/mcps/old.yaml").exists());
    assert_eq!(get_mcp(root, "new").unwrap(), mcp);
    assert_eq!(
        fs::read_to_string(&file).unwrap(),
        text.replace("mcps: [old,", "mcps: [\"new\",")
    );
    mcp.id = "taken".into();
    save_mcp(root, None, mcp.clone()).unwrap();
    assert!(save_mcp(root, Some("new"), mcp).is_err());
    fs::write(&file, "mcps: [").unwrap();
    let mut mcp = get_mcp(root, "new").unwrap();
    mcp.id = "next".into();
    assert!(save_mcp(root, Some("new"), mcp).is_err());
    assert!(get_mcp(root, "new").is_ok());
}
