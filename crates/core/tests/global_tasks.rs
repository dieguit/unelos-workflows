use camino::Utf8Path;
use std::{fs, process::Command};
use unelos_core::*;

#[test]
fn global_task_lifecycle_and_workflow_loading() {
    if std::env::var_os("UNELOS_GLOBAL_TASK_TEST_CHILD").is_none() {
        let temp = assert_fs::TempDir::new().unwrap();
        let status = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "global_task_lifecycle_and_workflow_loading"])
            .env("UNELOS_GLOBAL_TASK_TEST_CHILD", "1")
            .env("UNELOS_CONFIG_DIR", temp.path())
            .status()
            .unwrap();
        assert!(status.success());
        return;
    }
    assert!(global_task_summaries().unwrap().is_empty());
    let task = TaskDocument {
        id: "x".into(),
        description: Some("shared".into()),
        instructions: "prompt".into(),
        skills: vec![],
        outcomes: vec![],
    };
    save_global_task(None, task).unwrap();
    assert_eq!(get_global_task("x").unwrap().instructions, "prompt");
    assert_eq!(
        global_task_summaries().unwrap()[0].description.as_deref(),
        Some("shared")
    );
    assert!(get_global_task("../escape").is_err());
    let global = std::path::PathBuf::from(std::env::var_os("UNELOS_CONFIG_DIR").unwrap());
    fs::write(
        global.join("tasks/x/task.yaml"),
        "instructionsFile: prompt.md\n",
    )
    .unwrap();
    fs::write(global.join("tasks/x/prompt.md"), "external").unwrap();
    let mut task = get_global_task("x").unwrap();
    task.instructions = "updated".into();
    save_global_task(Some("x"), task).unwrap();
    assert!(fs::read_to_string(global.join("tasks/x/task.yaml"))
        .unwrap()
        .contains("instructionsFile: prompt.md"));
    duplicate_global_task("x", "copy").unwrap();
    assert_eq!(get_global_task("copy").unwrap().instructions, "updated");
    remove_global_task("copy").unwrap();

    let temp = assert_fs::TempDir::new().unwrap();
    let root = Utf8Path::from_path(temp.path()).unwrap();
    fs::create_dir_all(root.join(".unelos/workflows")).unwrap();
    fs::create_dir_all(root.join(".unelos/agents/agent")).unwrap();
    fs::write(
        root.join(".unelos/project.yaml"),
        include_str!("fixtures/project/.unelos/project.yaml"),
    )
    .unwrap();
    fs::write(
        root.join(".unelos/agents/agent/agent.yaml"),
        "instructions: agent\n",
    )
    .unwrap();
    for node in [
        "    task: global/x\n    agent: agent\n    next: done\n",
        "    type: parallel\n    tasks:\n      - task: global/x\n        agent: agent\n    next: done\n",
    ] {
        fs::write(root.join(".unelos/workflows/flow.yaml"), format!("name: flow\nstart: work\nnodes:\n  work:\n{node}  done:\n    type: end\n")).unwrap();
        let project = load_project(root, "flow").unwrap();
        validate_project(&project).unwrap();
        assert_eq!(project.definitions.task_definitions["global/x"].instructions, "updated");
        assert!(project.instruction_dependencies.is_empty());
    }
    remove_global_task("x").unwrap();
    let project = load_project(root, "flow").unwrap();
    assert!(validate_project(&project)
        .unwrap_err()
        .to_string()
        .contains("Global task \"x\" not found"));
}
