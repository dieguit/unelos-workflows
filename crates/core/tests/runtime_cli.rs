use assert_fs::{fixture::PathCopy, TempDir};
use camino::{Utf8Path, Utf8PathBuf};
use std::fs;
use unelos_core::{
    approve_node, complete_node, list_workflows, load_project, next, reject_node, start,
    start_workflow, status, CurrentWork,
};

fn project_copy() -> (TempDir, Utf8PathBuf) {
    let temp = TempDir::new().unwrap();
    temp.copy_from(
        Utf8Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/project"),
        &["**/*"],
    )
    .unwrap();
    let root = Utf8Path::from_path(temp.path()).unwrap().to_owned();
    (temp, root)
}

fn progression_bytes(root: &Utf8Path, name: &str) -> (Vec<u8>, Vec<u8>) {
    (
        fs::read(root.join(".unelos/runs").join(name).join("state.json")).unwrap(),
        fs::read(root.join(".unelos/runs").join(name).join("state.json")).unwrap(),
    )
}

fn assert_progression_unchanged(root: &Utf8Path, name: &str, before: &(Vec<u8>, Vec<u8>)) {
    assert_eq!(progression_bytes(root, name), *before);
}

#[test]
fn discovery_does_not_allocate_a_run() {
    let (_temp, root) = project_copy();
    let workflows = list_workflows(&root).unwrap();
    assert!(workflows.contains(&"implement-feature".to_owned()));
    assert!(workflows.windows(2).all(|pair| pair[0] < pair[1]));
    assert!(!root.join(".unelos/active-run").exists());
    assert!(!root.join(".unelos/runs").exists());
}

#[test]
fn unnamed_start_persists_pending_name_and_empty_request() {
    let (_temp, root) = project_copy();
    let state = start_workflow(&root, "implement-feature", None).unwrap();
    assert_eq!(state.name, state.run_id);
    assert!(state.run_id.starts_with("r-"));
    assert!(state.naming_pending);
    assert!(state.request.is_empty());
    assert_eq!(status(&root, &state.run_id).unwrap(), state);
    assert_eq!(
        next(&root, &state.run_id).unwrap(),
        next(&root, &state.run_id).unwrap()
    );
}

#[test]
fn explicit_start_separates_id_and_name() {
    let (_temp, root) = project_copy();
    let state = start_workflow(&root, "implement-feature", Some("add-search")).unwrap();
    assert_eq!(state.name, "add-search");
    assert_ne!(state.run_id, state.name);
    assert!(!state.naming_pending);
    assert_eq!(status(&root, &state.run_id).unwrap(), state);
}

#[test]
fn legacy_state_without_naming_metadata_is_already_named() {
    let (_temp, root) = project_copy();
    let project = load_project(&root, "implement-feature").unwrap();
    start(&project, "legacy", "Keep request").unwrap();
    let path = root.join(".unelos/runs/legacy/state.json");
    let mut value: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    value.as_object_mut().unwrap().remove("namingPending");
    fs::write(&path, serde_json::to_vec(&value).unwrap()).unwrap();
    let saved = status(&root, "legacy").unwrap();
    assert!(!saved.naming_pending);
    assert_eq!(saved.request, "Keep request");
}

#[test]
fn discovery_accepts_empty_workflows_and_ignores_non_yaml_files() {
    let (_temp, root) = project_copy();
    fs::write(root.join(".unelos/workflows/readme.txt"), "not a Workflow").unwrap();
    assert_eq!(
        list_workflows(&root).unwrap(),
        vec!["completion-loop", "implement-feature", "research-feature"]
    );
    for name in ["completion-loop", "implement-feature", "research-feature"] {
        fs::remove_file(root.join(format!(".unelos/workflows/{name}.yaml"))).unwrap();
    }
    assert!(list_workflows(&root).unwrap().is_empty());
}

#[test]
fn discovery_rejects_malformed_mismatched_and_invalid_workflows() {
    for (filename, contents) in [
        ("broken.yaml", "name: ["),
        (
            "mismatch.yaml",
            &include_str!("fixtures/project/.unelos/workflows/implement-feature.yaml")
                .replace("name: implement-feature", "name: other"),
        ),
        (
            "invalid.yaml",
            "name: invalid\nstart: missing\nnodes:\n  done:\n    type: end\n",
        ),
    ] {
        let (_temp, root) = project_copy();
        fs::write(root.join(".unelos/workflows").join(filename), contents).unwrap();
        assert!(list_workflows(&root).is_err(), "accepted {filename}");
    }
}

#[test]
fn starts_after_finish_allocate_distinct_ids() {
    let (_temp, root) = project_copy();
    fs::write(
        root.join(".unelos/workflows/finished.yaml"),
        "name: finished\nstart: done\nnodes:\n  done:\n    type: end\n",
    )
    .unwrap();
    let a = start_workflow(&root, "finished", Some("first")).unwrap();
    let b = start_workflow(&root, "finished", Some("second")).unwrap();
    assert_ne!(a.run_id, b.run_id);
}

#[test]
fn collisions_and_second_unfinished_start_preserve_the_first_run() {
    let (_temp, root) = project_copy();
    let state = start_workflow(&root, "implement-feature", Some("first")).unwrap();
    let state_path = root
        .join(".unelos/runs")
        .join(&state.run_id)
        .join("state.json");
    let saved = fs::read(&state_path).unwrap();
    assert!(start_workflow(&root, "implement-feature", Some("second")).is_ok());
    assert_eq!(fs::read(state_path).unwrap(), saved);
    assert_eq!(status(&root, &state.run_id).unwrap(), state);

    let (_temp, root) = project_copy();
    fs::create_dir_all(root.join(".unelos/runs/taken")).unwrap();
    assert!(start_workflow(&root, "implement-feature", Some("taken")).is_err());
    assert!(!root.join(".unelos/run-sequence").exists());
    assert!(!root.join(".unelos/active-run").exists());
}

#[test]
fn allocation_skips_legacy_generated_id_and_corrupt_counter_does_not_publish() {
    let (_temp, root) = project_copy();
    fs::write(
        root.join(".unelos/workflows/finished.yaml"),
        "name: finished\nstart: done\nnodes:\n  done:\n    type: end\n",
    )
    .unwrap();
    let legacy = load_project(&root, "finished").unwrap();
    start(&legacy, "legacy", "legacy").unwrap();
    let state_path = root.join(".unelos/runs/legacy/state.json");
    let mut saved: serde_json::Value =
        serde_json::from_slice(&fs::read(&state_path).unwrap()).unwrap();
    saved["runId"] = "r-000001".into();
    fs::write(&state_path, serde_json::to_vec(&saved).unwrap()).unwrap();
    fs::rename(
        root.join(".unelos/runs/legacy"),
        root.join(".unelos/runs/r-000001"),
    )
    .unwrap();
    let generated = start_workflow(&root, "finished", None).unwrap();
    assert_ne!(generated.run_id, "r-000001");

    let before = fs::read(
        root.join(".unelos/runs")
            .join(&generated.run_id)
            .join("state.json"),
    )
    .unwrap();
    fs::write(root.join(".unelos/run-sequence"), "not-a-number").unwrap();
    assert!(start_workflow(&root, "finished", Some("third")).is_err());
    assert_eq!(
        fs::read(
            root.join(".unelos/runs")
                .join(&generated.run_id)
                .join("state.json")
        )
        .unwrap(),
        before
    );
    assert!(!root.join(".unelos/runs/third").exists());
}

#[test]
fn node_completion_confirms_outputs_and_waits_for_gate() {
    let (_temp, root) = project_copy();
    let started = start_workflow(&root, "implement-feature", None).unwrap();
    let CurrentWork::Task(record) = &started.current else {
        panic!("task");
    };
    let before = progression_bytes(&root, &started.run_id);
    assert!(complete_node(&root, &started.run_id, "plan", None).is_err());
    assert_progression_unchanged(&root, &started.run_id, &before);
    fs::write(&record.outputs[0].path, "plan").unwrap();
    let gate = complete_node(&root, &started.run_id, "plan", None).unwrap();
    assert!(matches!(gate.current, CurrentWork::HumanGate(_)));
    assert_eq!(gate.history.len(), 1);
}

#[test]
fn rejected_completion_does_not_persist_output_confirmation() {
    let (_temp, root) = project_copy();
    let started = start_workflow(&root, "implement-feature", None).unwrap();
    let CurrentWork::Task(record) = &started.current else {
        panic!("task");
    };
    fs::write(&record.outputs[0].path, "plan").unwrap();
    let before = progression_bytes(&root, &started.run_id);
    assert!(complete_node(&root, &started.run_id, "plan", Some("approved")).is_err());
    assert_progression_unchanged(&root, &started.run_id, &before);
    assert!(matches!(
        complete_node(&root, &started.run_id, "implement", None),
        Err(unelos_core::RuntimeError::WrongNode { expected, current })
            if expected == "implement" && current == "plan"
    ));
    assert_progression_unchanged(&root, &started.run_id, &before);
    assert_eq!(status(&root, &started.run_id).unwrap(), started);
}

#[test]
fn node_completion_accepts_parallel_work() {
    let (_temp, root) = project_copy();
    fs::write(
        root.join(".unelos/workflows/parallel.yaml"),
        "name: parallel\nstart: parallel\nnodes:\n  parallel:\n    type: parallel\n    tasks:\n      - {task: create-plan, agent: planner}\n    outputs: [summary.md]\n    next: done\n  done:\n    type: end\n",
    )
    .unwrap();
    let started = start_workflow(&root, "parallel", None).unwrap();
    let CurrentWork::Parallel(record) = &started.current else {
        panic!("parallel")
    };
    fs::write(&record.outputs[0].path, "summary").unwrap();
    let before = progression_bytes(&root, &started.run_id);
    assert!(complete_node(&root, &started.run_id, "parallel", None).is_err());
    assert_progression_unchanged(&root, &started.run_id, &before);
    fs::write(&record.worker_reports[0].report.path, "worker result").unwrap();
    let finished = complete_node(&root, &started.run_id, "parallel", None).unwrap();
    assert!(finished.is_finished());
    assert_eq!(finished.history.len(), 1);
}

#[test]
fn node_operations_follow_feedback_loop_and_reject_invalid_mutations_atomically() {
    let (_temp, root) = project_copy();
    let mut state = start_workflow(&root, "implement-feature", Some("task")).unwrap();

    let CurrentWork::Task(plan) = &state.current else {
        panic!("plan task")
    };
    let plan_path = plan.outputs[0].path.clone();
    let before = progression_bytes(&root, "r-000001");
    assert!(complete_node(&root, &state.run_id, "implement", None).is_err());
    assert_progression_unchanged(&root, "r-000001", &before);
    assert!(complete_node(&root, &state.run_id, "plan", None).is_err());
    assert_progression_unchanged(&root, "r-000001", &before);
    fs::create_dir_all(&plan_path).unwrap();
    assert!(complete_node(&root, &state.run_id, "plan", None).is_err());
    assert_progression_unchanged(&root, "r-000001", &before);
    fs::remove_dir(&plan_path).unwrap();

    fs::write(&plan_path, "plan").unwrap();
    state = complete_node(&root, &state.run_id, "plan", None).unwrap();
    let gate_before = progression_bytes(&root, "r-000001");
    assert!(matches!(state.current, CurrentWork::HumanGate(_)));
    assert!(complete_node(&root, &state.run_id, "approve-plan", None).is_err());
    assert_eq!(status(&root, &state.run_id).unwrap(), state);
    assert_progression_unchanged(&root, "r-000001", &gate_before);
    fs::remove_file(&plan_path).unwrap();
    assert!(approve_node(&root, &state.run_id, "approve-plan").is_err());
    assert_eq!(status(&root, &state.run_id).unwrap(), state);
    assert_progression_unchanged(&root, "r-000001", &gate_before);
    fs::write(&plan_path, "plan").unwrap();
    for reason in ["  ", "\u{2003}"] {
        assert!(reject_node(&root, &state.run_id, "approve-plan", reason).is_err());
        assert_eq!(status(&root, &state.run_id).unwrap(), state);
        assert_progression_unchanged(&root, "r-000001", &gate_before);
    }
    assert!(approve_node(&root, &state.run_id, "plan").is_err());
    assert_progression_unchanged(&root, "r-000001", &gate_before);
    state = reject_node(&root, &state.run_id, "approve-plan", "  revise plan  ").unwrap();
    assert_eq!(
        state.history.last().unwrap().reason.as_deref(),
        Some("revise plan")
    );
    let CurrentWork::Task(revised) = &state.current else {
        panic!("revised plan")
    };
    assert_eq!(revised.outputs[0].path, plan_path);
    assert_eq!(
        revised.feedback.as_ref().unwrap().reason.as_deref(),
        Some("revise plan")
    );
    fs::write(&plan_path, "revised plan").unwrap();
    complete_node(&root, &state.run_id, "plan", None).unwrap();
    state = approve_node(&root, &state.run_id, "approve-plan").unwrap();
    let CurrentWork::Task(implement) = &state.current else {
        panic!("implementation")
    };
    let report_path = implement.outputs[0].path.clone();
    fs::write(&report_path, "implementation").unwrap();
    state = complete_node(&root, &state.run_id, "implement", None).unwrap();
    let CurrentWork::Task(review) = &state.current else {
        panic!("review")
    };
    let review_path = review.outputs[0].path.clone();
    let missing_output_before = progression_bytes(&root, "r-000001");
    assert!(complete_node(&root, &state.run_id, "review", None).is_err());
    assert_progression_unchanged(&root, "r-000001", &missing_output_before);
    fs::write(&review_path, "review").unwrap();
    let invalid_outcome_before = progression_bytes(&root, "r-000001");
    assert!(complete_node(&root, &state.run_id, "review", None).is_err());
    assert_progression_unchanged(&root, "r-000001", &invalid_outcome_before);
    assert!(complete_node(&root, &state.run_id, "review", Some("unknown")).is_err());
    assert_progression_unchanged(&root, "r-000001", &invalid_outcome_before);
    assert_eq!(status(&root, &state.run_id).unwrap(), state);
    state = complete_node(&root, &state.run_id, "review", Some("changes_requested")).unwrap();
    let CurrentWork::Task(implement) = &state.current else {
        panic!("implementation retry")
    };
    assert_eq!(implement.outputs[0].path, report_path);
    assert_eq!(status(&root, &state.run_id).unwrap(), state);
    fs::write(&report_path, "revised implementation").unwrap();
    state = complete_node(&root, &state.run_id, "implement", None).unwrap();
    let CurrentWork::Task(review) = &state.current else {
        panic!("review retry")
    };
    assert_eq!(review.outputs[0].path, review_path);
    state = complete_node(&root, &state.run_id, "review", Some("approved")).unwrap();
    assert!(state.is_finished());
    assert_eq!(state.history.len(), 8);
    assert_eq!(fs::read_to_string(review_path).unwrap(), "review");
    let saved = progression_bytes(&root, "r-000001");
    assert!(complete_node(&root, &state.run_id, "review", Some("approved")).is_err());
    assert_progression_unchanged(&root, "r-000001", &saved);
}
