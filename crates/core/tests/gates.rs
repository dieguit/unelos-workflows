use assert_fs::{fixture::PathCopy, TempDir};
use camino::{Utf8Path, Utf8PathBuf};
use std::fs;
use unelos_core::{
    approve, complete, confirm_outputs, load_project, next, reject, start, status, CurrentWork,
    ExecutionRecord, RunState,
};

fn project_copy() -> (TempDir, Utf8PathBuf) {
    let temp = TempDir::new().unwrap();
    let root = Utf8Path::from_path(temp.path()).unwrap().to_owned();
    temp.copy_from(
        Utf8Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/project"),
        &["**/*"],
    )
    .unwrap();
    (temp, root)
}

fn record(state: &RunState) -> &ExecutionRecord {
    match &state.current {
        CurrentWork::Task(record)
        | CurrentWork::Parallel(record)
        | CurrentWork::HumanGate(record) => record,
        CurrentWork::Finished { .. } => panic!("expected execution"),
    }
}

fn waiting(root: &Utf8Path, name: &str) -> RunState {
    let project = load_project(root, "implement-feature").unwrap();
    let state = start(&project, name, "Request").unwrap();
    fs::write(&record(&state).outputs[0].path, "plan").unwrap();
    confirm_outputs(root, "task-1").unwrap();
    complete(root, name, "task-1", None, None).unwrap()
}

fn state_path(root: &Utf8Path, name: &str) -> Utf8PathBuf {
    root.join(".unelos/runs").join(name).join("state.json")
}

fn assert_unchanged(root: &Utf8Path, name: &str, before: &[u8], state: &RunState) {
    assert_eq!(fs::read(state_path(root, name)).unwrap(), before);
    assert_eq!(status(root).unwrap(), *state);
    assert_eq!(
        fs::read(root.join(".unelos/active-run")).unwrap(),
        name.as_bytes()
    );
}

#[test]
fn rejection_persists_feedback_and_approval_resumes_loop_with_fresh_ids() {
    let (_temp, root) = project_copy();
    let waiting = waiting(&root, "gate-loop");
    let material = record(&waiting).inputs.clone();
    assert_eq!(next(&root).unwrap(), next(&root).unwrap());

    let revised = reject(
        &root,
        "gate-loop",
        "gate-2",
        "  Explain migration and rollback.  ",
    )
    .unwrap();
    assert_eq!(record(&revised).execution_id, "task-3");
    assert_eq!(record(&revised).outputs, material);
    let feedback = record(&revised).feedback.as_ref().unwrap();
    assert_eq!(feedback.source_node_id, "approve-plan");
    assert_eq!(feedback.source_execution_id, "gate-2");
    assert_eq!(feedback.outcome.as_deref(), Some("rejected"));
    assert_eq!(
        feedback.reason.as_deref(),
        Some("Explain migration and rollback.")
    );
    assert_eq!(feedback.artifacts, material);
    assert_eq!(status(&root).unwrap(), revised);
    assert_eq!(revised.history[1].work, waiting.current);
    assert_eq!(revised.history[1].outcome.as_deref(), Some("rejected"));
    assert_eq!(revised.history[1].reason, feedback.reason);
    assert!(next(&root).is_ok());

    let path = record(&revised).outputs[0].path.clone();
    assert!(complete(&root, "gate-loop", "task-3", None, None).is_err());
    fs::write(path, "revised plan").unwrap();
    confirm_outputs(&root, "task-3").unwrap();
    let gate = complete(&root, "gate-loop", "task-3", None, None).unwrap();
    assert_eq!(record(&gate).execution_id, "gate-4");

    let state_path = root.join(".unelos/runs/gate-loop/state.json");
    let before = fs::read(&state_path).unwrap();
    assert!(approve(&root, "gate-loop", "gate-2").is_err());
    assert!(reject(&root, "gate-loop", "gate-2", "old decision").is_err());
    assert_eq!(fs::read(&state_path).unwrap(), before);

    let approved = approve(&root, "gate-loop", "gate-4").unwrap();
    assert_eq!(record(&approved).execution_id, "task-5");
    let approval = record(&approved).feedback.as_ref().unwrap();
    assert_eq!(approval.source_node_id, "approve-plan");
    assert_eq!(approval.source_execution_id, "gate-4");
    assert_eq!(approval.outcome.as_deref(), Some("approved"));
    assert_eq!(approval.reason, None);
    assert_eq!(approval.artifacts, record(&gate).inputs);
    assert_eq!(approved.history[3].outcome.as_deref(), Some("approved"));
    assert_eq!(approved.history[3].reason, None);
    assert_eq!(status(&root).unwrap(), approved);
    assert_eq!(next(&root).unwrap(), next(&root).unwrap());
}

#[test]
fn invalid_reasons_and_wrong_work_kinds_do_not_mutate_saved_state() {
    for reason in ["", " \t\n", "\u{2003}\u{00a0}"] {
        let (_temp, root) = project_copy();
        waiting(&root, "bad-reason");
        let path = state_path(&root, "bad-reason");
        let before = fs::read(&path).unwrap();
        assert!(reject(&root, "bad-reason", "gate-2", reason).is_err());
        assert_unchanged(&root, "bad-reason", &before, &status(&root).unwrap());
    }

    let (_temp, root) = project_copy();
    let project = load_project(&root, "implement-feature").unwrap();
    start(&project, "not-gate", "Request").unwrap();
    let before = fs::read(state_path(&root, "not-gate")).unwrap();
    assert!(approve(&root, "not-gate", "task-1").is_err());
    assert!(reject(&root, "not-gate", "task-1", "reason").is_err());
    assert_unchanged(&root, "not-gate", &before, &status(&root).unwrap());

    let (_temp, root) = project_copy();
    fs::write(
        root.join(".unelos/workflows/implement-feature.yaml"),
        "name: implement-feature\nstart: parallel\nnodes:\n  parallel:\n    type: parallel\n    tasks:\n      - {task: create-plan, agent: planner}\n    next: done\n  done:\n    type: end\n",
    )
    .unwrap();
    let project = load_project(&root, "implement-feature").unwrap();
    start(&project, "not-gate-parallel", "Request").unwrap();
    let before = fs::read(state_path(&root, "not-gate-parallel")).unwrap();
    assert!(approve(&root, "not-gate-parallel", "task-1").is_err());
    assert!(reject(&root, "not-gate-parallel", "task-1", "reason").is_err());
    assert_unchanged(&root, "not-gate-parallel", &before, &status(&root).unwrap());
}

#[test]
fn stale_wrong_run_and_duplicate_decisions_leave_saved_state_unchanged() {
    let (_temp, root) = project_copy();
    let waiting = waiting(&root, "stale-gate");
    let path = state_path(&root, "stale-gate");
    let before = fs::read(&path).unwrap();
    assert!(approve(&root, "stale-gate", "gate-99").is_err());
    assert!(approve(&root, "other-run", "gate-2").is_err());
    assert_unchanged(&root, "stale-gate", &before, &waiting);
    let decided = approve(&root, "stale-gate", "gate-2").unwrap();
    let after = fs::read(&path).unwrap();
    assert!(reject(&root, "stale-gate", "gate-2", "duplicate").is_err());
    assert!(approve(&root, "stale-gate", "gate-2").is_err());
    assert_eq!(fs::read(path).unwrap(), after);
    assert_ne!(decided, waiting);
}

#[test]
fn decision_checks_gate_material_and_rejects_directories_instead_of_files() {
    for as_directory in [false, true] {
        for decision in ["approve", "reject"] {
            let (_temp, root) = project_copy();
            let gate = waiting(&root, "missing-gate-input");
            let artifact = record(&gate).inputs[0].path.clone();
            fs::remove_file(&artifact).unwrap();
            if as_directory {
                fs::create_dir(&artifact).unwrap();
            }
            let path = state_path(&root, "missing-gate-input");
            let before = fs::read(&path).unwrap();
            let result = if decision == "approve" {
                approve(&root, "missing-gate-input", "gate-2")
            } else {
                reject(&root, "missing-gate-input", "gate-2", "reason")
            };
            assert!(result.is_err(), "{decision} accepted missing gate material");
            assert_unchanged(&root, "missing-gate-input", &before, &gate);
        }
    }
}

#[test]
fn live_successor_and_gate_definition_are_validated_before_decision() {
    let (_temp, root) = project_copy();
    waiting(&root, "missing-skill-gate");
    let path = state_path(&root, "missing-skill-gate");
    let before = fs::read(&path).unwrap();
    fs::remove_file(root.join(".unelos/skills/verification/SKILL.md")).unwrap();
    assert!(approve(&root, "missing-skill-gate", "gate-2").is_err());
    assert_eq!(fs::read(&path).unwrap(), before);
    assert_eq!(record(&status(&root).unwrap()).execution_id, "gate-2");

    let (_temp, root) = project_copy();
    waiting(&root, "changed-gate");
    let workflow = root.join(".unelos/workflows/implement-feature.yaml");
    let original = fs::read_to_string(&workflow).unwrap();
    let changed = original.replace("inputs: [plan.md]", "inputs: []");
    assert_ne!(changed, original);
    fs::write(&workflow, changed).unwrap();
    let path = state_path(&root, "changed-gate");
    let before = fs::read(&path).unwrap();
    assert!(approve(&root, "changed-gate", "gate-2").is_err());
    assert_eq!(fs::read(path).unwrap(), before);

    for replacement in [
        "  approve-plan:\n    type: end\n",
        "  approve-plan:\n    task: create-plan\n    agent: planner\n    outputs: [plan.md]\n    next: implement\n",
    ] {
        let (_temp, root) = project_copy();
        waiting(&root, "removed-or-changed-gate");
        let workflow = root.join(".unelos/workflows/implement-feature.yaml");
        let original = fs::read_to_string(&workflow).unwrap();
        let gate = "  approve-plan:\n    type: human-gate\n    inputs: [plan.md]\n    next:\n      approved: implement\n      rejected: plan\n";
        let edited = original.replace(gate, replacement);
        assert_ne!(edited, original);
        fs::write(workflow, edited).unwrap();
        let path = state_path(&root, "removed-or-changed-gate");
        let before = fs::read(&path).unwrap();
        assert!(approve(&root, "removed-or-changed-gate", "gate-2").is_err());
        assert_eq!(fs::read(path).unwrap(), before);
    }
}

#[test]
fn gate_feedback_validation_matches_presented_artifacts_and_rejects_tampering() {
    let (_temp, root) = project_copy();
    let gate = waiting(&root, "feedback-validation");
    let rejected = reject(&root, "feedback-validation", "gate-2", "revise\nplan").unwrap();
    assert_eq!(
        record(&rejected)
            .feedback
            .as_ref()
            .unwrap()
            .reason
            .as_deref(),
        Some("revise\nplan")
    );

    let path = state_path(&root, "feedback-validation");
    let mut saved: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    saved["current"]["feedback"]["artifacts"] = serde_json::json!([]);
    fs::write(&path, serde_json::to_vec(&saved).unwrap()).unwrap();
    assert!(status(&root).is_err());
    assert!(next(&root).is_err());
    assert_ne!(gate, rejected);

    #[cfg(unix)]
    for linked in [false, true] {
        use std::os::unix::fs::symlink;
        let (_temp, root) = project_copy();
        waiting(&root, "unsafe-gate-feedback");
        reject(&root, "unsafe-gate-feedback", "gate-2", "revise").unwrap();
        let path = state_path(&root, "unsafe-gate-feedback");
        let mut saved: serde_json::Value =
            serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        let artifact_path = if linked {
            let target = root.join("outside.md");
            fs::write(&target, "outside").unwrap();
            let producer = root.join(".unelos/runs/unsafe-gate-feedback/other");
            fs::create_dir(&producer).unwrap();
            let link = producer.join("linked.md");
            symlink(target, &link).unwrap();
            link
        } else {
            root.join("outside.md")
        };
        let artifacts = serde_json::json!([{"name":"plan.md","path":artifact_path}]);
        saved["history"][1]["work"]["inputs"] = artifacts.clone();
        saved["current"]["feedback"]["artifacts"] = artifacts;
        fs::write(&path, serde_json::to_vec(&saved).unwrap()).unwrap();
        assert!(status(&root).is_err());
        assert!(next(&root).is_err());
    }
}

#[test]
fn finished_and_new_same_identity_run_cannot_replay_prior_gate_decision() {
    let (_temp, root) = project_copy();
    fs::write(
        root.join(".unelos/workflows/implement-feature.yaml"),
        "name: implement-feature\nstart: gate\nnodes:\n  gate:\n    type: human-gate\n    next:\n      approved: done\n      rejected: done\n  done:\n    type: end\n",
    )
    .unwrap();
    let project = load_project(&root, "implement-feature").unwrap();
    let finished = start(&project, "first-gate-run", "Request").unwrap();
    assert_eq!(record(&finished).execution_id, "gate-1");
    assert!(complete(&root, "first-gate-run", "gate-1", None, None).is_err());
    let finished = approve(&root, "first-gate-run", "gate-1").unwrap();
    assert!(finished.is_finished());
    assert!(approve(&root, "first-gate-run", "gate-1").is_err());
    assert!(reject(&root, "first-gate-run", "gate-1", "duplicate").is_err());

    let next_run = start(&project, "second-gate-run", "Request").unwrap();
    assert_eq!(record(&next_run).execution_id, "gate-1");
    let before = fs::read(state_path(&root, "second-gate-run")).unwrap();
    assert!(approve(&root, "first-gate-run", "gate-1").is_err());
    assert_eq!(
        fs::read(state_path(&root, "second-gate-run")).unwrap(),
        before
    );
}

#[test]
fn decision_prepares_only_declared_successor_and_preserves_blocked_output_path() {
    let (_temp, root) = project_copy();
    let workflow = root.join(".unelos/workflows/implement-feature.yaml");
    let original = fs::read_to_string(&workflow).unwrap();
    fs::write(
        &workflow,
        original.replace("approved: implement", "approved: done"),
    )
    .unwrap();
    waiting(&root, "live-approved-edge");
    let done = approve(&root, "live-approved-edge", "gate-2").unwrap();
    assert!(done.is_finished());
    assert_eq!(next(&root).unwrap(), next(&root).unwrap());

    let (_temp, root) = project_copy();
    waiting(&root, "blocked-successor");
    let blocked = root.join(".unelos/runs/blocked-successor/implement");
    fs::write(&blocked, "keep").unwrap();
    let path = state_path(&root, "blocked-successor");
    let before = fs::read(&path).unwrap();
    assert!(approve(&root, "blocked-successor", "gate-2").is_err());
    assert_eq!(fs::read(path).unwrap(), before);
    assert_eq!(fs::read(blocked).unwrap(), b"keep");
}

#[test]
fn decisions_support_gate_to_gate_and_gate_to_parallel_routes() {
    let (_temp, root) = project_copy();
    fs::write(
        root.join(".unelos/workflows/implement-feature.yaml"),
        "name: implement-feature\nstart: plan\nnodes:\n  plan:\n    task: create-plan\n    agent: planner\n    outputs: [plan.md]\n    next: gate-one\n  gate-one:\n    type: human-gate\n    inputs: [plan.md]\n    next: {approved: gate-two, rejected: gate-two}\n  gate-two:\n    type: human-gate\n    inputs: [plan.md]\n    next: {approved: done, rejected: done}\n  done:\n    type: end\n",
    )
    .unwrap();
    let project = load_project(&root, "implement-feature").unwrap();
    let state = start(&project, "gate-chain", "Request").unwrap();
    fs::write(&record(&state).outputs[0].path, "plan").unwrap();
    confirm_outputs(&root, "task-1").unwrap();
    let first = complete(&root, "gate-chain", "task-1", None, None).unwrap();
    let second = approve(&root, "gate-chain", "gate-2").unwrap();
    assert_eq!(record(&second).node_id, "gate-two");
    assert_eq!(record(&second).execution_id, "gate-3");
    let done = reject(&root, "gate-chain", "gate-3", "stop").unwrap();
    assert!(done.is_finished());
    assert_eq!(record(&first).node_id, "gate-one");
    assert_eq!(status(&root).unwrap(), done);

    let (_temp, root) = project_copy();
    fs::write(
        root.join(".unelos/workflows/implement-feature.yaml"),
        "name: implement-feature\nstart: plan\nnodes:\n  plan:\n    task: create-plan\n    agent: planner\n    outputs: [plan.md]\n    next: gate\n  gate:\n    type: human-gate\n    inputs: [plan.md]\n    next: {approved: parallel, rejected: parallel}\n  parallel:\n    type: parallel\n    tasks:\n      - {task: create-plan, agent: planner}\n    inputs: [plan.md]\n    outputs: [aggregate.md]\n    next: done\n  done:\n    type: end\n",
    )
    .unwrap();
    let project = load_project(&root, "implement-feature").unwrap();
    let state = start(&project, "gate-parallel", "Request").unwrap();
    fs::write(&record(&state).outputs[0].path, "plan").unwrap();
    confirm_outputs(&root, "task-1").unwrap();
    complete(&root, "gate-parallel", "task-1", None, None).unwrap();
    let parallel = approve(&root, "gate-parallel", "gate-2").unwrap();
    assert!(matches!(parallel.current, CurrentWork::Parallel(_)));
    assert_eq!(record(&parallel).execution_id, "task-3");
    assert!(next(&root).is_ok());
}

#[test]
fn feedback_includes_multiple_producer_artifacts_in_declared_order() {
    let (_temp, root) = project_copy();
    fs::write(
        root.join(".unelos/workflows/implement-feature.yaml"),
        "name: implement-feature\nstart: first\nnodes:\n  first:\n    task: create-plan\n    agent: planner\n    outputs: [first.md]\n    next: second\n  second:\n    task: create-plan\n    agent: planner\n    outputs: [second.md]\n    next: gate\n  gate:\n    type: human-gate\n    inputs: [first.md, second.md]\n    next: {approved: consume, rejected: consume}\n  consume:\n    task: create-plan\n    agent: planner\n    inputs: [first.md, second.md]\n    outputs: [final.md]\n    next: done\n  done:\n    type: end\n",
    )
    .unwrap();
    let project = load_project(&root, "implement-feature").unwrap();
    let mut state = start(&project, "multi-input-gate", "Request").unwrap();
    for id in ["task-1", "task-2"] {
        fs::write(&record(&state).outputs[0].path, id).unwrap();
        confirm_outputs(&root, id).unwrap();
        state = complete(&root, "multi-input-gate", id, None, None).unwrap();
    }
    let presented = record(&state).inputs.clone();
    assert_eq!(
        presented
            .iter()
            .map(|a| a.name.as_str())
            .collect::<Vec<_>>(),
        ["first.md", "second.md"]
    );
    let rejected = reject(&root, "multi-input-gate", "gate-3", "reviewed").unwrap();
    assert_eq!(
        record(&rejected).feedback.as_ref().unwrap().artifacts,
        presented
    );
    assert_eq!(rejected.history[2].outcome.as_deref(), Some("rejected"));
}

#[test]
fn missing_successor_producer_input_does_not_persist_decision() {
    let (_temp, root) = project_copy();
    let workflow = root.join(".unelos/workflows/implement-feature.yaml");
    let original = fs::read_to_string(&workflow).unwrap();
    let edited = original
        .replace("inputs: [plan.md]\n    outputs: [implementation-report.md]", "inputs: [phantom.md]\n    outputs: [implementation-report.md]")
        .replace("  done:\n    type: end\n", "  unused-producer:\n    task: create-plan\n    agent: planner\n    outputs: [phantom.md]\n    next: done\n  done:\n    type: end\n");
    assert_ne!(edited, original);
    fs::write(workflow, edited).unwrap();
    waiting(&root, "missing-successor-input");
    let path = state_path(&root, "missing-successor-input");
    let before = fs::read(&path).unwrap();
    assert!(approve(&root, "missing-successor-input", "gate-2").is_err());
    assert_eq!(fs::read(path).unwrap(), before);
}
