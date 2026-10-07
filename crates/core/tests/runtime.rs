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

#[test]
fn rejects_saved_external_output_before_returning_a_packet() {
    let (_temp, root) = project_copy();
    let project = load_project(&root, "implement-feature").unwrap();
    start(&project, "tampered", "request").unwrap();
    let state_path = root.join(".unelos/runs/tampered/state.json");
    let mut state: serde_json::Value =
        serde_json::from_slice(&fs::read(&state_path).unwrap()).unwrap();
    state["current"]["outputs"][0]["path"] = root.join("outside.md").to_string().into();
    fs::write(&state_path, serde_json::to_vec(&state).unwrap()).unwrap();
    let tampered = fs::read(&state_path).unwrap();

    assert!(next(&root).is_err());
    assert!(status(&root).is_err());
    assert_eq!(fs::read(state_path).unwrap(), tampered);
}

#[cfg(unix)]
#[test]
fn rejects_literal_backslash_symlinked_output_without_mutating_state() {
    use std::os::unix::fs::symlink;

    let (_temp, root) = project_copy();
    let project = load_project(&root, "implement-feature").unwrap();
    start(&project, "backslash", "request").unwrap();
    let state_path = root.join(".unelos/runs/backslash/state.json");
    let outside = root.join("outside.md");
    fs::write(&outside, "outside").unwrap();
    let escaped = root.join(".unelos/runs/backslash/plan/outside\\redirect.md");
    symlink(&outside, &escaped).unwrap();
    let mut state: serde_json::Value =
        serde_json::from_slice(&fs::read(&state_path).unwrap()).unwrap();
    state["current"]["outputs"][0]["path"] = escaped.to_string().into();
    fs::write(&state_path, serde_json::to_vec(&state).unwrap()).unwrap();
    let tampered = fs::read(&state_path).unwrap();

    assert!(status(&root).is_err());
    assert!(next(&root).is_err());
    assert_eq!(fs::read(&state_path).unwrap(), tampered);
    assert_eq!(fs::read(outside).unwrap(), b"outside");

    state["current"]["outputs"][0]["path"] = root
        .join(".unelos/runs/backslash/plan/../outside.md")
        .to_string()
        .into();
    fs::write(&state_path, serde_json::to_vec(&state).unwrap()).unwrap();
    let traversed = fs::read(&state_path).unwrap();
    assert!(status(&root).is_err());
    assert!(next(&root).is_err());
    assert_eq!(fs::read(state_path).unwrap(), traversed);
}

#[test]
fn rejects_saved_external_worker_report_before_returning_a_packet() {
    let (_temp, root) = project_copy();
    fs::write(
        root.join(".unelos/workflows/implement-feature.yaml"),
        "name: implement-feature\nstart: parallel\nnodes:\n  parallel:\n    type: parallel\n    tasks:\n      - {task: create-plan, agent: planner}\n    next: done\n  done:\n    type: end\n",
    )
    .unwrap();
    let project = load_project(&root, "implement-feature").unwrap();
    start(&project, "tampered-worker", "request").unwrap();
    let state_path = root.join(".unelos/runs/tampered-worker/state.json");
    let mut state: serde_json::Value =
        serde_json::from_slice(&fs::read(&state_path).unwrap()).unwrap();
    state["current"]["workerReports"][0]["report"]["path"] =
        root.join("outside.md").to_string().into();
    fs::write(&state_path, serde_json::to_vec(&state).unwrap()).unwrap();
    let tampered = fs::read(&state_path).unwrap();

    assert!(next(&root).is_err());
    assert_eq!(fs::read(state_path).unwrap(), tampered);
}

#[test]
fn validates_saved_feedback_and_history_paths_but_allows_producer_inputs() {
    let (_temp, root) = project_copy();
    let project = load_project(&root, "implement-feature").unwrap();
    start(&project, "saved-history", "request").unwrap();
    let state_path = root.join(".unelos/runs/saved-history/state.json");
    let original: serde_json::Value =
        serde_json::from_slice(&fs::read(&state_path).unwrap()).unwrap();
    let producer_path = root
        .join(".unelos/runs/saved-history/producer/result.md")
        .to_string();
    let external_path = root.join("outside.md").to_string();

    let mut valid = original.clone();
    valid["current"]["inputs"] = serde_json::json!([{"name":"result.md","path":producer_path}]);
    valid["current"]["feedback"] = serde_json::json!({
        "sourceNodeId":"producer", "sourceExecutionId":"task-1", "outcome":"approved",
        "reason":null, "artifacts":[{"name":"result.md","path":root.join(".unelos/runs/saved-history/producer/result.md").to_string()}]
    });
    fs::write(&state_path, serde_json::to_vec(&valid).unwrap()).unwrap();
    assert!(status(&root).is_ok());

    for path in ["feedback", "history"] {
        let mut invalid = original.clone();
        if path == "feedback" {
            invalid["current"]["feedback"] = serde_json::json!({
                "sourceNodeId":"producer", "sourceExecutionId":"task-1", "outcome":"approved",
                "reason":null, "artifacts":[{"name":"result.md","path":external_path}]
            });
        } else {
            let mut completed = original["current"].clone();
            completed["outputs"][0]["path"] = external_path.clone().into();
            invalid["history"] =
                serde_json::json!([{"work":completed,"outcome":null,"reason":null}]);
        }
        fs::write(&state_path, serde_json::to_vec(&invalid).unwrap()).unwrap();
        assert!(status(&root).is_err(), "accepted external {path} path");
    }
}

#[cfg(unix)]
#[test]
fn rejects_symlinked_pointer_state_artifact_and_ledger_paths() {
    use std::os::unix::fs::symlink;

    for linked in ["pointer", "state", "artifact", "ledger"] {
        let (_temp, root) = project_copy();
        let project = load_project(&root, "implement-feature").unwrap();
        start(&project, "linked", "request").unwrap();
        let runtime = root.join(".unelos");
        let run = runtime.join("runs/linked");
        match linked {
            "pointer" => {
                fs::rename(runtime.join("active-run"), root.join("pointer-target")).unwrap();
                symlink(root.join("pointer-target"), runtime.join("active-run")).unwrap();
            }
            "state" => {
                fs::rename(run.join("state.json"), root.join("state-target")).unwrap();
                symlink(root.join("state-target"), run.join("state.json")).unwrap();
            }
            "artifact" => {
                fs::rename(run.join("plan"), root.join("plan-target")).unwrap();
                symlink(root.join("plan-target"), run.join("plan")).unwrap();
            }
            "ledger" => symlink(root.join("outside-ledger"), run.join("ledger.md")).unwrap(),
            _ => unreachable!(),
        }
        assert!(status(&root).is_err(), "accepted symlinked {linked}");
    }
}

#[test]
fn rejects_aliased_worker_suffix_and_nested_output_collisions() {
    let (_temp, root) = project_copy();
    fs::write(
        root.join(".unelos/workflows/implement-feature.yaml"),
        "name: implement-feature\nstart: parallel\nnodes:\n  parallel:\n    type: parallel\n    tasks:\n      - {task: create-plan, agent: planner}\n      - {task: create-plan, agent: planner}\n      - {task: create-plan, agent: planner-worker-1}\n    next: done\n  done:\n    type: end\n",
    )
    .unwrap();
    let project = load_project(&root, "implement-feature").unwrap();
    assert!(start(&project, "alias", "request").is_err());
    assert!(!root.join(".unelos/active-run").exists());

    let (_temp, root) = project_copy();
    fs::write(
        root.join(".unelos/workflows/implement-feature.yaml"),
        "name: implement-feature\nstart: parallel\nnodes:\n  parallel:\n    type: parallel\n    tasks:\n      - {task: create-plan, agent: planner}\n    outputs: [planner]\n    next: done\n  done:\n    type: end\n",
    )
    .unwrap();
    let project = load_project(&root, "implement-feature").unwrap();
    assert!(start(&project, "nested", "request").is_err());
    assert!(!root.join(".unelos/active-run").exists());
}

#[test]
fn rejects_existing_run_path_and_unsafe_runtime_components_without_publication() {
    for (workflow, expected) in [
        (
            "name: implement-feature\nstart: ../escape\nnodes:\n  ../escape:\n    type: end\n",
            "unsafe-node",
        ),
        (
            "name: implement-feature\nstart: parallel\nnodes:\n  parallel:\n    type: parallel\n    tasks:\n      - {task: create-plan, agent: ../escape}\n    next: done\n  done:\n    type: end\n",
            "unsafe-agent",
        ),
    ] {
        let (_temp, root) = project_copy();
        fs::write(root.join(".unelos/workflows/implement-feature.yaml"), workflow).unwrap();
        if expected == "unsafe-agent" {
            let project_path = root.join(".unelos/project.yaml");
            let project_yaml = fs::read_to_string(&project_path).unwrap();
            fs::write(
                project_path,
                project_yaml.replace(
                    "agents:\n",
                    "agents:\n  \"../escape\":\n    instructions: unsafe\n",
                ),
            )
            .unwrap();
        }
        let project = load_project(&root, "implement-feature").unwrap();
        let error = start(&project, expected, "request").unwrap_err();
        assert!(error.to_string().contains(if expected == "unsafe-node" {
            "unsafe Node"
        } else {
            "unsafe Agent"
        }));
        assert!(!root.join(".unelos/active-run").exists());
        assert!(!root.join(format!(".unelos/runs/{expected}")).exists());
    }

    let (_temp, root) = project_copy();
    let project = load_project(&root, "implement-feature").unwrap();
    fs::create_dir_all(root.join(".unelos/runs/existing")).unwrap();
    fs::write(root.join(".unelos/runs/existing/keep"), "untouched").unwrap();
    assert!(start(&project, "existing", "request").is_err());
    assert_eq!(
        fs::read(root.join(".unelos/runs/existing/keep")).unwrap(),
        b"untouched"
    );
    assert!(!root.join(".unelos/active-run").exists());
}

#[cfg(unix)]
#[test]
fn start_rejects_existing_symlinked_run_path_without_touching_target() {
    use std::os::unix::fs::symlink;

    let (_temp, root) = project_copy();
    let project = load_project(&root, "implement-feature").unwrap();
    let target = root.join("keep");
    fs::create_dir(&target).unwrap();
    fs::write(target.join("marker"), "untouched").unwrap();
    fs::create_dir_all(root.join(".unelos/runs")).unwrap();
    symlink(&target, root.join(".unelos/runs/linked-run")).unwrap();

    assert!(start(&project, "linked-run", "request").is_err());
    assert_eq!(fs::read(target.join("marker")).unwrap(), b"untouched");
    assert!(!root.join(".unelos/active-run").exists());
}
