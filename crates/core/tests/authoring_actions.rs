use camino::{Utf8Path, Utf8PathBuf};
use std::fs;
use unelos_core::*;

fn fixture() -> (assert_fs::TempDir, Utf8PathBuf) {
    let temp = assert_fs::TempDir::new().unwrap();
    let root = Utf8PathBuf::from_path_buf(temp.path().to_owned()).unwrap();
    for dir in [
        "skills/skill/resources",
        "agents/agent/resources",
        "workflows",
        "tasks",
    ] {
        fs::create_dir_all(root.join(".unelos").join(dir)).unwrap();
    }
    fs::write(
        root.join(".unelos/project.yaml"),
        include_str!("fixtures/project/.unelos/project.yaml"),
    )
    .unwrap();
    write(
        &root,
        "skills/skill/SKILL.md",
        "---\nname: skill\ndescription: Original\n---\nBody\n",
    );
    write(&root, "skills/skill/resources/data", "skill data");
    write(
        &root,
        "agents/agent/agent.yaml",
        "instructionsFile: resources/prompt.md\n",
    );
    write(&root, "agents/agent/resources/prompt.md", "agent prompt");
    write(
        &root,
        "workflows/flow.yaml",
        "name: flow\ndescription: test\nstart: done\nnodes:\n  done:\n    type: end\n",
    );
    (temp, root)
}
fn write(root: &Utf8Path, path: &str, text: &str) {
    fs::write(root.join(".unelos").join(path), text).unwrap();
}

#[test]
fn task_copies_and_named_workflow_references() {
    let (_temp, root) = fixture();
    fs::create_dir_all(root.join(".unelos/tasks/task")).unwrap();
    write(
        &root,
        "tasks/task/task.yaml",
        "instructionsFile: prompt.md\n",
    );
    write(&root, "tasks/task/prompt.md", "prompt");
    duplicate_task(&root, "task", "copy").unwrap();
    assert_eq!(get_task(&root, "copy").unwrap().instructions, "prompt");
    assert_eq!(
        fs::read(root.join(".unelos/tasks/task/task.yaml")).unwrap(),
        fs::read(root.join(".unelos/tasks/copy/task.yaml")).unwrap()
    );
    remove_task(&root, "copy").unwrap();
    for node in [
        "    task: task\n    agent: agent\n    next: done\n",
        "    type: parallel\n    tasks:\n      - task: task\n        agent: agent\n    next: done\n",
    ] {
        write(&root, "workflows/flow.yaml", &format!("name: flow\nstart: work\nnodes:\n  work:\n{node}  done:\n    type: end\n"));
        assert!(remove_task(&root, "task").unwrap_err().contains("Cannot remove Task \"task\": referenced by"));
    }
}

#[test]
fn copies_full_trees_and_workflow_identity_without_changing_source() {
    let (_temp, root) = fixture();
    let original = fs::read(root.join(".unelos/workflows/flow.yaml")).unwrap();
    duplicate_skill(&root, "skill", "Skill ü copy").unwrap();
    duplicate_agent(&root, "agent", "agent-copy").unwrap();
    duplicate_workflow(&root, "flow", "flow-copy").unwrap();
    for (old, new) in [
        ("skills/skill", "skills/Skill ü copy"),
        ("agents/agent", "agents/agent-copy"),
    ] {
        let file = if old.starts_with("skills") {
            "resources/data"
        } else {
            "resources/prompt.md"
        };
        assert_eq!(
            fs::read(root.join(".unelos").join(old).join(file)).unwrap(),
            fs::read(root.join(".unelos").join(new).join(file)).unwrap()
        );
        let file = if old.starts_with("skills") {
            "SKILL.md"
        } else {
            "agent.yaml"
        };
        assert_eq!(
            fs::read(root.join(".unelos").join(old).join(file)).unwrap(),
            fs::read(root.join(".unelos").join(new).join(file)).unwrap()
        );
    }
    let copied = load_project(&root, "flow-copy").unwrap();
    assert_eq!(copied.workflow.name, "flow-copy");
    assert_eq!(
        copied.workflow.nodes,
        load_project(&root, "flow").unwrap().workflow.nodes
    );
    assert_eq!(
        fs::read(root.join(".unelos/workflows/flow.yaml")).unwrap(),
        original
    );
    assert!(duplicate_skill(&root, "skill", "Skill ü copy").is_err());
    assert!(duplicate_agent(&root, "agent", "agent-copy").is_err());
    assert!(duplicate_workflow(&root, "flow", "flow-copy").is_err());
    for bad in ["../outside", "", "/outside"] {
        assert!(duplicate_skill(&root, "skill", bad).is_err());
        assert!(duplicate_agent(&root, "agent", bad).is_err());
        assert!(duplicate_workflow(&root, "flow", bad).is_err());
        assert!(remove_skill(&root, bad).is_err());
        assert!(remove_agent(&root, bad).is_err());
        assert!(remove_workflow(&root, bad).is_err());
    }
    remove_skill(&root, "Skill ü copy").unwrap();
    remove_agent(&root, "agent-copy").unwrap();
    remove_workflow(&root, "flow-copy").unwrap();
}

#[test]
fn references_inline_parallel_tasks_agents_and_external_instructions_block_removal() {
    let (_temp, root) = fixture();
    write(
        &root,
        "agents/agent/agent.yaml",
        "instructions: prompt\nskills: [skill]\n",
    );
    assert!(remove_skill(&root, "skill")
        .unwrap_err()
        .contains("referenced"));
    write(&root, "agents/agent/agent.yaml", "instructions: prompt\n");
    fs::create_dir_all(root.join(".unelos/tasks/task")).unwrap();
    write(
        &root,
        "tasks/task/task.yaml",
        "instructions: prompt\nskills: [skill]\n",
    );
    assert!(remove_skill(&root, "skill").is_err());
    write(&root, "tasks/task/task.yaml", "instructions: prompt\n");
    for node in [
        "    taskDefinition:\n      instructions: prompt\n      skills: [skill]\n    agent: agent\n    next: done\n",
        "    type: parallel\n    tasks:\n      - taskDefinition:\n          instructions: prompt\n          skills: [skill]\n        agent: agent\n    next: done\n",
    ] {
        write(&root, "workflows/flow.yaml", &format!("name: flow\nstart: work\nnodes:\n  work:\n{node}  done:\n    type: end\n"));
        assert!(remove_skill(&root, "skill").unwrap_err().contains("referenced"));
        assert!(remove_agent(&root, "agent").unwrap_err().contains("referenced"));
    }
    write(
        &root,
        "workflows/flow.yaml",
        "name: flow\nstart: done\nnodes:\n  done:\n    type: end\n",
    );
    write(
        &root,
        "tasks/task/task.yaml",
        "instructionsFile: ../../agents/agent/resources/prompt.md\n",
    );
    assert!(remove_agent(&root, "agent")
        .unwrap_err()
        .contains("instructions"));
    write(
        &root,
        "tasks/task/task.yaml",
        "instructionsFile: ../../skills/skill/resources/data\n",
    );
    assert!(remove_skill(&root, "skill").is_err());
}

#[test]
fn finished_runs_do_not_block_workflow_removal() {
    let (_temp, root) = fixture();
    start(&load_project(&root, "flow").unwrap(), "stored", "request").unwrap();
    let state_path = root.join(".unelos/runs/stored/state.json");
    let mut state: serde_json::Value =
        serde_json::from_slice(&fs::read(&state_path).unwrap()).unwrap();
    state["current"] = serde_json::json!({"kind": "finished", "nodeId": "done"});
    fs::write(&state_path, serde_json::to_vec(&state).unwrap()).unwrap();
    remove_workflow(&root, "flow").unwrap();
}

#[test]
fn run_and_malformed_authoring_fail_closed_but_unreferenced_bad_definition_can_be_removed() {
    let (_temp, root) = fixture();
    let project = load_project(&root, "flow").unwrap();
    start(&project, "stored", "request").unwrap();
    // The fixture flow starts at End, so force the stored run back to in-progress work.
    let state_path = root.join(".unelos/runs/stored/state.json");
    let mut state: serde_json::Value =
        serde_json::from_slice(&fs::read(&state_path).unwrap()).unwrap();
    state["current"] = serde_json::json!({"kind": "task", "nodeId": "done", "executionId": "task-1", "inputs": [], "outputs": [], "workerReports": [], "feedback": null});
    state["nextExecutionSequence"] = 2.into();
    fs::write(&state_path, serde_json::to_vec(&state).unwrap()).unwrap();
    assert!(remove_workflow(&root, "flow")
        .unwrap_err()
        .contains("in-progress runs"));
    fs::write(root.join(".unelos/runs/stored/state.json"), "invalid json").unwrap();
    assert!(remove_workflow(&root, "flow")
        .unwrap_err()
        .contains("stored runs"));
    assert!(remove_agent(&root, "agent").is_err());
    assert!(remove_skill(&root, "skill").is_err());
    fs::remove_dir_all(root.join(".unelos/runs")).unwrap();
    write(&root, "agents/agent/agent.yaml", "bad: [");
    assert!(remove_skill(&root, "skill").is_err());
    assert!(remove_workflow(&root, "flow").is_err());
    remove_agent(&root, "agent").unwrap();
    write(&root, "workflows/flow.yaml", "broken: [");
    assert!(remove_skill(&root, "skill").is_err());
    remove_workflow(&root, "flow").unwrap();
    remove_skill(&root, "skill").unwrap();
}

#[cfg(unix)]
#[test]
fn duplicates_preserve_executable_resources_and_directory_permissions() {
    use std::os::unix::fs::PermissionsExt;
    let (_temp, root) = fixture();
    for (kind, id, duplicate) in [
        (
            "skills",
            "skill",
            duplicate_skill as fn(&Utf8Path, &str, &str) -> Result<(), String>,
        ),
        (
            "agents",
            "agent",
            duplicate_agent as fn(&Utf8Path, &str, &str) -> Result<(), String>,
        ),
    ] {
        let source = root.join(".unelos").join(kind).join(id);
        fs::write(source.join("resources/script.sh"), "#!/bin/sh\nexit 0\n").unwrap();
        fs::set_permissions(
            source.join("resources/script.sh"),
            fs::Permissions::from_mode(0o751),
        )
        .unwrap();
        fs::set_permissions(source.join("resources"), fs::Permissions::from_mode(0o750)).unwrap();
        fs::set_permissions(&source, fs::Permissions::from_mode(0o700)).unwrap();
        duplicate(&root, id, "copy").unwrap();
        let target = root.join(".unelos").join(kind).join("copy");
        for relative in ["", "resources", "resources/script.sh"] {
            assert_eq!(
                fs::metadata(target.join(relative))
                    .unwrap()
                    .permissions()
                    .mode()
                    & 0o777,
                fs::metadata(source.join(relative))
                    .unwrap()
                    .permissions()
                    .mode()
                    & 0o777
            );
        }
    }
}

#[cfg(unix)]
#[test]
fn symlinks_and_dangling_collisions_never_touch_outside() {
    let (_temp, root) = fixture();
    let outside = assert_fs::TempDir::new().unwrap();
    fs::write(outside.path().join("data"), "outside").unwrap();
    for (source, destination) in [
        ("skills/skill", "skills/copy"),
        ("agents/agent", "agents/copy"),
        ("workflows/flow.yaml", "workflows/copy.yaml"),
    ] {
        std::os::unix::fs::symlink(
            outside.path().join("missing"),
            root.join(".unelos").join(destination),
        )
        .unwrap();
        let result = if source.starts_with("skills") {
            duplicate_skill(&root, "skill", "copy")
        } else if source.starts_with("agents") {
            duplicate_agent(&root, "agent", "copy")
        } else {
            duplicate_workflow(&root, "flow", "copy")
        };
        assert!(result.is_err());
        fs::remove_file(root.join(".unelos").join(destination)).unwrap();
    }
    std::os::unix::fs::symlink(
        outside.path(),
        root.join(".unelos/skills/skill/resources/link"),
    )
    .unwrap();
    assert!(duplicate_skill(&root, "skill", "copy").is_err());
    assert!(remove_skill(&root, "skill").is_err());
    assert_eq!(
        fs::read_to_string(outside.path().join("data")).unwrap(),
        "outside"
    );
    assert!(root.join(".unelos/skills/skill/SKILL.md").exists());
}
