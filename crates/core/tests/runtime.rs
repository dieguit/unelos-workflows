use assert_fs::{fixture::PathCopy, TempDir};
use camino::Utf8Path;
use std::fs;
use unelos_core::{load_project, next, start, status};

fn project_copy() -> (TempDir, camino::Utf8PathBuf) {
    let temp = TempDir::new().unwrap();
    let root = Utf8Path::from_path(temp.path()).unwrap().to_owned();
    temp.copy_from(
        Utf8Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/project"),
        &["**/*"],
    )
    .unwrap();
    (temp, root)
}

#[test]
fn starts_and_recovers_a_live_task_packet() {
    let (_temp, root) = project_copy();
    let project = load_project(&root, "implement-feature").unwrap();
    let state = start(&project, "change-button-color", "Change the button color.").unwrap();
    assert_eq!(state, status(&root).unwrap());
    let a = serde_json::to_value(next(&root).unwrap()).unwrap();
    let b = serde_json::to_value(next(&root).unwrap()).unwrap();
    assert_eq!(a, b);
    assert_eq!(a["taskId"], "task-1");
    assert_eq!(a["request"], "Change the button color.");
    let output = Utf8Path::new(a["outputs"][0]["path"].as_str().unwrap());
    assert!(output.ends_with("plan/plan.md"));
    assert!(!root
        .join(".unelos/runs/change-button-color/plan/plan.md")
        .exists());
    assert!(start(&project, "another-run", "Another request").is_err());
}

#[test]
fn live_task_and_skill_edits_change_packet_without_changing_saved_state() {
    let (_temp, root) = project_copy();
    let workflow = root.join(".unelos/workflows/implement-feature.yaml");
    let yaml = fs::read_to_string(&workflow)
        .unwrap()
        .replace("start: plan", "start: implement");
    fs::write(
        &workflow,
        yaml.replace("    inputs: [plan.md]\n", "    inputs: []\n"),
    )
    .unwrap();
    let project = load_project(&root, "implement-feature").unwrap();
    let state = start(&project, "live-edit", "request").unwrap();
    let state_path = root.join(".unelos/runs/live-edit/state.json");
    let bytes = fs::read(&state_path).unwrap();
    let mut definitions = fs::read_to_string(root.join(".unelos/project.yaml")).unwrap();
    definitions = definitions.replace(
        "Implement the supplied plan and report verification.",
        "Changed task live.",
    );
    definitions = definitions.replace(
        "Follow existing project conventions.",
        "Changed agent live.",
    );
    fs::write(root.join(".unelos/project.yaml"), definitions).unwrap();
    fs::write(
        root.join(".unelos/skills/verification/SKILL.md"),
        "changed skill live",
    )
    .unwrap();
    let packet = serde_json::to_value(next(&root).unwrap()).unwrap();
    assert_eq!(packet["taskInstructions"], "Changed task live.");
    assert_eq!(packet["agentInstructions"], "Changed agent live.");
    assert!(packet.to_string().contains("changed skill live"));
    assert_eq!(fs::read(state_path).unwrap(), bytes);
    assert_eq!(status(&root).unwrap(), state);
}

#[test]
fn incompatible_live_edits_error_without_rewriting_state_or_paths() {
    let (_temp, root) = project_copy();
    let workflow = root.join(".unelos/workflows/implement-feature.yaml");
    let yaml = fs::read_to_string(&workflow)
        .unwrap()
        .replace("start: plan", "start: implement")
        .replace("    inputs: [plan.md]\n", "    inputs: []\n");
    fs::write(&workflow, yaml).unwrap();
    let project = load_project(&root, "implement-feature").unwrap();
    start(&project, "incompatible", "request").unwrap();
    let state_path = root.join(".unelos/runs/incompatible/state.json");
    let saved = fs::read(&state_path).unwrap();
    fs::remove_file(root.join(".unelos/skills/verification/SKILL.md")).unwrap();
    assert!(next(&root).is_err());
    assert_eq!(fs::read(&state_path).unwrap(), saved);
    assert_eq!(status(&root).unwrap().request, "request");
}

#[test]
fn removed_or_changed_current_task_definition_errors_without_mutating_state() {
    let original = fs::read_to_string(
        Utf8Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/project/.unelos/workflows/implement-feature.yaml"),
    )
    .unwrap();
    let node = "  plan:\n    task: create-plan\n    agent: planner\n    outputs: [plan.md]\n    next: approve-plan\n";
    for edit in [
        original.replace(node, ""),
        original.replace(
            node,
            "  plan:\n    type: human-gate\n    next:\n      approved: done\n      rejected: done\n",
        ),
        original.replace("outputs: [plan.md]", "outputs: [changed.md]"),
    ] {
        let (_temp, root) = project_copy();
        let project = load_project(&root, "implement-feature").unwrap();
        start(&project, "changed-live", "request").unwrap();
        let path = root.join(".unelos/runs/changed-live/state.json");
        let saved = fs::read(&path).unwrap();
        fs::write(root.join(".unelos/workflows/implement-feature.yaml"), edit).unwrap();
        assert!(next(&root).is_err());
        assert_eq!(fs::read(path).unwrap(), saved);
        assert_eq!(status(&root).unwrap().request, "request");
    }
}

#[test]
fn starts_input_free_gate_and_finished_nodes_and_allows_new_run_after_end() {
    for (node_yaml, expected) in [
        (
            "    type: human-gate\n    next:\n      approved: done\n      rejected: done\n",
            "gate-1",
        ),
        ("    type: end\n", "finished"),
    ] {
        let (_temp, root) = project_copy();
        fs::write(root.join(".unelos/workflows/implement-feature.yaml"), format!("name: implement-feature\nstart: plan\nnodes:\n  plan:\n{node_yaml}  done:\n    type: end\n")).unwrap();
        let project = load_project(&root, "implement-feature").unwrap();
        let state = start(&project, "variant", "request").unwrap();
        if expected == "finished" {
            assert!(state.is_finished());
            assert_eq!(
                serde_json::to_value(next(&root).unwrap()).unwrap()["kind"],
                "finished"
            );
            assert!(start(&project, "after-end", "again").unwrap().is_finished());
        } else {
            assert_eq!(
                match state.current {
                    unelos_core::CurrentWork::HumanGate(ref r) => r.execution_id.as_str(),
                    _ => "",
                },
                expected
            );
        }
    }
}

#[test]
fn rejects_input_nodes_and_unsafe_names_without_publishing() {
    let (_temp, root) = project_copy();
    let project = load_project(&root, "implement-feature").unwrap();
    for name in ["", "../escape", "UPPER", "con", &"a".repeat(65)] {
        assert!(start(&project, name, "request").is_err(), "{name:?}");
    }
    let workflow = root.join(".unelos/workflows/implement-feature.yaml");
    let yaml = fs::read_to_string(&workflow)
        .unwrap()
        .replace("start: plan", "start: implement");
    fs::write(&workflow, yaml).unwrap();
    let project = load_project(&root, "implement-feature").unwrap();
    assert!(start(&project, "inputs", "request").is_err());
    assert!(!root.join(".unelos/active-run").exists());
}

#[test]
fn parallel_workers_get_stable_distinct_report_paths() {
    let (_temp, root) = project_copy();
    fs::write(
        root.join(".unelos/workflows/implement-feature.yaml"),
        "name: implement-feature\nstart: parallel\nnodes:\n  parallel:\n    type: parallel\n    tasks:\n      - {task: create-plan, agent: planner}\n      - {task: create-plan, agent: planner}\n    outputs: [aggregate.md]\n    next: done\n  done:\n    type: end\n",
    )
    .unwrap();
    let project = load_project(&root, "implement-feature").unwrap();
    let state = start(&project, "parallel-run", "request").unwrap();
    assert!(matches!(
        state.current,
        unelos_core::CurrentWork::Parallel(_)
    ));
    let packet = serde_json::to_value(next(&root).unwrap()).unwrap();
    assert_eq!(packet["workers"][0]["key"], "worker-1");
    assert_eq!(packet["workers"][1]["key"], "worker-2");
    assert!(packet["workers"][0]["outputs"][0]["path"]
        .as_str()
        .unwrap()
        .ends_with("planner-worker-1/result.md"));
    assert!(packet["workers"][1]["outputs"][0]["path"]
        .as_str()
        .unwrap()
        .ends_with("planner-worker-2/result.md"));
    assert!(root
        .join(".unelos/runs/parallel-run/parallel/planner-worker-1")
        .is_dir());
    assert_eq!(serde_json::to_value(next(&root).unwrap()).unwrap(), packet);
}

#[test]
fn rejects_aggregate_worker_path_collisions_before_publishing() {
    let (_temp, root) = project_copy();
    fs::write(
        root.join(".unelos/workflows/implement-feature.yaml"),
        "name: implement-feature\nstart: parallel\nnodes:\n  parallel:\n    type: parallel\n    tasks:\n      - {task: create-plan, agent: planner}\n      - {task: create-plan, agent: planner}\n    outputs: [planner-worker-1/result.md]\n    next: done\n  done:\n    type: end\n",
    )
    .unwrap();
    let project = load_project(&root, "implement-feature").unwrap();
    assert!(start(&project, "collision", "request").is_err());
    assert!(!root.join(".unelos/active-run").exists());
}

#[test]
fn added_parallel_worker_does_not_reallocate_saved_work() {
    let (_temp, root) = project_copy();
    let workflow = root.join(".unelos/workflows/implement-feature.yaml");
    let parallel = "name: implement-feature\nstart: parallel\nnodes:\n  parallel:\n    type: parallel\n    tasks:\n      - {task: create-plan, agent: planner}\n      - {task: create-plan, agent: reviewer}\n    next: done\n  done:\n    type: end\n";
    fs::write(&workflow, parallel).unwrap();
    let project = load_project(&root, "implement-feature").unwrap();
    start(&project, "worker-edit", "request").unwrap();
    let path = root.join(".unelos/runs/worker-edit/state.json");
    let saved = fs::read(&path).unwrap();
    fs::write(
        &workflow,
        parallel.replace(
            "    next: done",
            "      - {task: create-plan, agent: planner}\n    next: done",
        ),
    )
    .unwrap();
    assert!(next(&root).is_err());
    assert_eq!(fs::read(path).unwrap(), saved);
    assert_eq!(status(&root).unwrap().request, "request");
}

#[test]
fn status_does_not_need_valid_live_definitions() {
    let (_temp, root) = project_copy();
    let project = load_project(&root, "implement-feature").unwrap();
    let state = start(&project, "change-color", "request").unwrap();
    fs::write(root.join(".unelos/project.yaml"), "invalid: [").unwrap();
    assert_eq!(status(&root).unwrap(), state);
}
