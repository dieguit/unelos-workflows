use camino::{Utf8Path, Utf8PathBuf};
use std::{fs, process::Command};
use unelos_core::*;

#[test]
fn global_agent_authoring_instructions_and_validation() {
    if std::env::var_os("UNELOS_GLOBAL_AGENT_CHILD").is_none() {
        let config = assert_fs::TempDir::new().unwrap();
        let status = Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "global_agent_authoring_instructions_and_validation",
            ])
            .env("UNELOS_GLOBAL_AGENT_CHILD", "1")
            .env("UNELOS_CONFIG_DIR", config.path())
            .status()
            .unwrap();
        assert!(status.success());
        return;
    }
    assert!(global_agent_summaries().unwrap().is_empty());
    let document = AgentDocument {
        id: "shared".into(),
        instructions: "shared".into(),
        skills: Vec::new(),
        mcps: Vec::new(),
        tools: Vec::new(),
        model: None,
        effort: None,
    };
    save_global_agent(None, document).unwrap();
    let config =
        Utf8PathBuf::from_path_buf(std::env::var_os("UNELOS_CONFIG_DIR").unwrap().into()).unwrap();
    let directory = config.join("agents").join("shared");
    let source = directory.join("agent.yaml");
    fs::write(&source, "instructionsFile: prompt.md\n").unwrap();
    fs::write(directory.join("prompt.md"), "external prompt\r\n").unwrap();
    assert_eq!(
        get_global_agent("shared")
            .unwrap()
            .instructions
            .replace("\r\n", "\n"),
        "external prompt\n"
    );
    let mut agent = get_global_agent("shared").unwrap();
    agent.instructions = "updated".into();
    save_global_agent(Some("shared"), agent).unwrap();
    assert!(fs::read_to_string(&source)
        .unwrap()
        .contains("instructionsFile"));
    duplicate_global_agent("shared", "copy").unwrap();
    assert_eq!(get_global_agent("copy").unwrap().instructions, "updated");
    remove_global_agent("copy").unwrap();
    for field in ["skills", "mcps"] {
        let mut agent = get_global_agent("shared").unwrap();
        if field == "skills" {
            agent.skills.push("local".into());
        } else {
            agent.mcps.push("local".into());
        }
        assert!(save_global_agent(Some("shared"), agent)
            .unwrap_err()
            .contains(field));
    }
    fs::write(&source, "instructionsFile: ../../../outside.md\n").unwrap();
    assert!(get_global_agent("shared").unwrap_err().contains("escapes"));
    assert!(global_agent_summaries().unwrap()[0].problem.is_some());
    assert!(get_global_agent("../outside").is_err());
    let project = assert_fs::TempDir::new().unwrap();
    let root = Utf8Path::from_path(project.path()).unwrap();
    fs::create_dir_all(root.join(".unelos").join("workflows")).unwrap();
    fs::write(root.join(".unelos").join("project.yaml"), "{}").unwrap();
    fs::write(&source, "instructions: global\n").unwrap();
    fs::write(root.join(".unelos").join("workflows").join("local.yaml"), "name: local\nstart: work\nnodes:\n  work:\n    agent: global/shared\n    taskDefinition:\n      instructions: local task\n    next: end\n  end:\n    type: end\n").unwrap();
    validate_project(&load_project(root, "local").unwrap()).unwrap();
    fs::write(&source, "instructions: global\nskills: [local]\n").unwrap();
    assert!(load_project(root, "local")
        .unwrap_err()
        .to_string()
        .contains("skills[0]"));
}
