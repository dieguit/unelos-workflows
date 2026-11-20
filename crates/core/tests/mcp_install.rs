use assert_fs::TempDir;
use camino::Utf8Path;
use std::{collections::BTreeMap, fs};
use unelos_core::{
    check_opencode_mcps, uninstall_opencode_mcps, write_opencode_mcps, AgentExport, McpServer,
};

fn agent(id: &str) -> AgentExport {
    AgentExport {
        id: "agent".into(),
        instructions: String::new(),
        model: None,
        effort: None,
        tools: vec![],
        mcps: vec![id.into()],
        subagent: true,
        mcp_servers: BTreeMap::from([(
            id.into(),
            McpServer::Local {
                command: vec!["server".into()],
                environment: BTreeMap::new(),
            },
        )]),
    }
}

#[test]
fn workflow_ownership_uninstall_collisions_and_ambiguity() {
    let temp = TempDir::new().unwrap();
    let root = Utf8Path::from_path(temp.path()).unwrap();
    write_opencode_mcps(root, "one", &[agent("shared"), agent("only_one")]).unwrap();
    write_opencode_mcps(root, "two", &[agent("shared")]).unwrap();
    uninstall_opencode_mcps(root, "one").unwrap();
    let read = || {
        serde_json::from_str::<serde_json::Value>(
            &fs::read_to_string(root.join("opencode.json")).unwrap(),
        )
        .unwrap()
    };
    let value = read();
    assert!(value["mcp"].get("only_one").is_none());
    assert!(value["mcp"].get("shared").is_some());
    let mut edited = value.clone();
    edited["mcp"]["shared"]["command"] = serde_json::json!(["user"]);
    fs::write(
        root.join("opencode.json"),
        serde_json::to_string(&edited).unwrap(),
    )
    .unwrap();
    assert_eq!(
        check_opencode_mcps(root, "two", &[agent("shared")])
            .unwrap_err()
            .code,
        "mcp_collision"
    );
    assert_eq!(read(), edited);
    fs::write(
        root.join("opencode.json"),
        serde_json::to_string(&value).unwrap(),
    )
    .unwrap();
    uninstall_opencode_mcps(root, "two").unwrap();
    assert!(read()["mcp"].get("shared").is_none());
    fs::write(root.join("opencode.jsonc"), "{}").unwrap();
    assert_eq!(
        check_opencode_mcps(root, "two", &[agent("shared")])
            .unwrap_err()
            .code,
        "ambiguous_opencode_config"
    );
}
