use assert_fs::{fixture::PathCopy, TempDir};
use camino::Utf8Path;
use std::fs;
use unelos_core::{complete, confirm_outputs, load_project, next, start, status, CurrentWork};

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

fn normalize_fixture_text(text: &str) -> String {
    text.replace("\r\n", "\n")
}

fn read_fixture_text(path: impl AsRef<std::path::Path>) -> String {
    normalize_fixture_text(&fs::read_to_string(path).unwrap())
}

fn replace_fixture_text(text: &str, from: &str, to: &str) -> String {
    let text = normalize_fixture_text(text);
    assert!(text.contains(from), "fixture edit did not match: {from:?}");
    text.replace(from, to)
}

#[test]
fn multiline_fixture_edits_match_lf_and_crlf() {
    let source = read_fixture_text(
        Utf8Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/project/.unelos/workflows/implement-feature.yaml"),
    );
    let node = "  plan:\n    task: create-plan\n    agent: planner\n    outputs: [plan.md]\n    next: approve-plan\n";
    let inputs = "inputs: [plan.md]\n    outputs: [implementation-report.md]";

    for source in [source.clone(), source.replace('\n', "\r\n")] {
        let edited = replace_fixture_text(&source, node, "  plan:\n    type: human-gate\n");
        let edited = replace_fixture_text(
            &edited,
            inputs,
            "inputs: [phantom.md]\n    outputs: [implementation-report.md]",
        );

        assert!(edited.contains("  plan:\n    type: human-gate\n"));
        assert!(edited.contains("inputs: [phantom.md]\n    outputs: [implementation-report.md]"));
        assert!(!edited.contains(node));
    }
}

#[test]
fn completion_requires_confirmation_and_enters_the_waiting_gate() {
    let (_temp, root) = project_copy();
    let project = load_project(&root, "implement-feature").unwrap();
    let state = start(&project, "completion-run", "Change button color").unwrap();
    let CurrentWork::Task(plan) = &state.current else {
        panic!("expected plan")
    };
    fs::write(&plan.outputs[0].path, "plan").unwrap();
    assert!(complete(&root, &state.run_id, &plan.execution_id, None, None).is_err());
    confirm_outputs(&root, "completion-run", &plan.execution_id).unwrap();

    let completed = complete(&root, &state.run_id, &plan.execution_id, None, None).unwrap();

    assert!(matches!(completed.current, CurrentWork::HumanGate(_)));
    assert_eq!(completed.history.len(), 1);
    assert_eq!(
        outputs(&completed.history[0].work)[0].path,
        plan.outputs[0].path
    );
    let packet = serde_json::to_value(next(&root, "completion-run").unwrap()).unwrap();
    assert_eq!(packet["feedback"]["outcome"], serde_json::Value::Null);
    assert_eq!(status(&root, "completion-run").unwrap(), completed);
}

fn confirm_current(root: &Utf8Path, state: &unelos_core::RunState, contents: &str) {
    let record = match &state.current {
        CurrentWork::Task(record) | CurrentWork::Parallel(record) => record,
        _ => panic!("expected task work"),
    };
    for output in &record.outputs {
        fs::write(&output.path, format!("{contents} {}", record.execution_id)).unwrap();
    }
    confirm_outputs(root, &state.run_id, &record.execution_id).unwrap();
}

fn execution(state: &unelos_core::RunState) -> &unelos_core::ExecutionRecord {
    match &state.current {
        CurrentWork::Task(record) | CurrentWork::Parallel(record) => record,
        _ => panic!("expected task work"),
    }
}

#[test]
fn completion_routes_a_correction_loop_with_fresh_ids_and_feedback() {
    let (_temp, root) = project_copy();
    let workflow = root.join(".unelos/workflows/implement-feature.yaml");
    let source = read_fixture_text(&workflow);
    fs::write(
        &workflow,
        replace_fixture_text(&source, "next: approve-plan", "next: implement"),
    )
    .unwrap();
    let project = load_project(&root, "implement-feature").unwrap();
    let mut state = start(&project, "completion-loop", "Improve feature").unwrap();

    confirm_current(&root, &state, "plan");
    state = complete(&root, &state.run_id, "task-1", None, None).unwrap();
    assert_eq!(execution(&state).execution_id, "task-2");
    let implementation_path = execution(&state).outputs[0].path.clone();
    assert_eq!(
        execution(&state).inputs[0].path,
        outputs(&state.history[0].work)[0].path
    );
    confirm_current(&root, &state, "implementation v1");
    state = complete(&root, &state.run_id, "task-2", None, None).unwrap();
    assert_eq!(execution(&state).execution_id, "task-3");
    let review_path = execution(&state).outputs[0].path.clone();
    confirm_current(&root, &state, "changes requested");
    state = complete(
        &root,
        &state.run_id,
        "task-3",
        Some("changes_requested"),
        Some("Please revise"),
    )
    .unwrap();
    assert_eq!(execution(&state).execution_id, "task-4");
    assert_eq!(execution(&state).outputs[0].path, implementation_path);
    assert!(execution(&state).confirmed_execution_id.is_none());
    let feedback = execution(&state).feedback.as_ref().unwrap();
    assert_eq!(feedback.source_node_id, "review");
    assert_eq!(feedback.source_execution_id, "task-3");
    assert_eq!(feedback.outcome.as_deref(), Some("changes_requested"));
    assert_eq!(feedback.reason.as_deref(), Some("Please revise"));
    assert_eq!(
        feedback.artifacts[0].path,
        outputs(&state.history[2].work)[0].path
    );

    confirm_outputs(&root, &state.run_id, "task-4").unwrap();
    let error = complete(&root, &state.run_id, "task-4", None, None).unwrap_err().to_string();
    assert!(error.contains("Unchanged outputs"));
    assert!(error.contains("record in a report how the feedback was addressed"));
    confirm_current(&root, &state, "implementation v1");
    state = complete(&root, &state.run_id, "task-4", None, None).unwrap();
    assert_eq!(execution(&state).execution_id, "task-5");
    assert_eq!(execution(&state).outputs[0].path, review_path);
    assert_eq!(execution(&state).inputs[1].path, implementation_path);
    confirm_current(&root, &state, "approved");
    state = complete(&root, &state.run_id, "task-5", Some("approved"), None).unwrap();
    assert!(state.is_finished());
    assert_eq!(state.history.len(), 5);
    assert_eq!(
        state
            .history
            .iter()
            .map(|entry| match &entry.work {
                CurrentWork::Task(record) | CurrentWork::Parallel(record) => {
                    record.execution_id.as_str()
                }
                _ => panic!("expected completed task"),
            })
            .collect::<Vec<_>>(),
        ["task-1", "task-2", "task-3", "task-4", "task-5"]
    );
    assert_eq!(status(&root, "completion-loop").unwrap(), state);
    assert_eq!(status(&root, "completion-loop").unwrap(), state);
    let finished = serde_json::to_value(next(&root, "completion-loop").unwrap()).unwrap();
    assert_eq!(finished["kind"], "finished");
    assert_eq!(
        start(&project, "completion-after", "Another request")
            .unwrap()
            .run_id,
        "completion-after"
    );
}

#[test]
fn invalid_identity_and_outcome_completions_leave_state_bytes_unchanged() {
    let (_temp, root) = project_copy();
    let project = load_project(&root, "implement-feature").unwrap();
    let state = start(&project, "completion-run", "Request").unwrap();
    confirm_current(&root, &state, "plan");
    let state_path = root.join(".unelos/runs/completion-run/state.json");
    let before = fs::read(&state_path).unwrap();

    for (run, id, outcome) in [
        ("other-run", "task-1", None),
        ("completion-run", "task-99", None),
        ("completion-run", "task-1", Some("approved")),
        ("completion-run", "task-1", Some("")),
    ] {
        assert!(complete(&root, run, id, outcome, None).is_err());
        assert_eq!(fs::read(&state_path).unwrap(), before);
    }
}

#[test]
fn outcome_routes_require_an_exact_declared_label() {
    let (_temp, root) = project_copy();
    let workflow = root.join(".unelos/workflows/implement-feature.yaml");
    let source = read_fixture_text(&workflow);
    fs::write(
        &workflow,
        replace_fixture_text(&source, "next: approve-plan", "next: implement"),
    )
    .unwrap();
    let project = load_project(&root, "implement-feature").unwrap();
    let state = start(&project, "outcome-run", "Request").unwrap();
    confirm_current(&root, &state, "plan");
    let state = complete(&root, &state.run_id, "task-1", None, None).unwrap();
    confirm_current(&root, &state, "implementation");
    let state = complete(&root, &state.run_id, "task-2", None, None).unwrap();
    confirm_current(&root, &state, "review");
    let state_path = root.join(".unelos/runs/outcome-run/state.json");
    let before = fs::read(&state_path).unwrap();
    for outcome in [None, Some("unknown"), Some("")] {
        assert!(complete(&root, &state.run_id, "task-3", outcome, None).is_err());
        assert_eq!(fs::read(&state_path).unwrap(), before);
    }
    let completed = complete(&root, &state.run_id, "task-3", Some("approved"), None).unwrap();
    assert_eq!(completed.history[2].outcome.as_deref(), Some("approved"));
}

#[test]
fn completion_rejects_matching_gate_activation_and_duplicate_progression() {
    let (_temp, root) = project_copy();
    let project = load_project(&root, "implement-feature").unwrap();
    let state = start(&project, "gate-completion", "Request").unwrap();
    confirm_current(&root, &state, "plan");
    let waiting = complete(&root, &state.run_id, "task-1", None, None).unwrap();
    let CurrentWork::HumanGate(gate) = &waiting.current else {
        panic!("expected gate")
    };
    let gate_id = gate.execution_id.clone();
    let state_path = root.join(".unelos/runs/gate-completion/state.json");
    let before = fs::read(&state_path).unwrap();
    assert!(complete(&root, &state.run_id, &gate_id, None, None).is_err());
    assert!(complete(&root, &state.run_id, "task-1", None, None).is_err());
    assert_eq!(fs::read(&state_path).unwrap(), before);
}

#[test]
fn finished_and_reentered_runs_reject_duplicate_or_old_run_completion() {
    let (_temp, root) = project_copy();
    let project = load_project(&root, "implement-feature").unwrap();
    let state = start(&project, "finished-completion", "Request").unwrap();
    confirm_current(&root, &state, "plan");
    fs::write(
        root.join(".unelos/workflows/implement-feature.yaml"),
        replace_fixture_text(
            &read_fixture_text(root.join(".unelos/workflows/implement-feature.yaml")),
            "next: approve-plan",
            "next: done",
        ),
    )
    .unwrap();
    let finished = complete(&root, &state.run_id, "task-1", None, None).unwrap();
    assert!(finished.is_finished());
    assert!(complete(&root, &state.run_id, "task-1", None, None).is_err());

    let later = start(&project, "later-completion", "Fresh request").unwrap();
    let error = complete(&root, &state.run_id, "task-1", None, None).unwrap_err();
    assert!(matches!(
        error,
        unelos_core::RuntimeError::StaleExecution { current: None, .. }
    ));
    assert_eq!(status(&root, &state.run_id).unwrap(), finished);
    assert_eq!(later, status(&root, "later-completion").unwrap());
}

#[test]
fn compatible_live_next_edit_is_used_and_unconfirmed_or_missing_outputs_fail() {
    let (_temp, root) = project_copy();
    let project = load_project(&root, "implement-feature").unwrap();
    let state = start(&project, "live-next", "Request").unwrap();
    let CurrentWork::Task(plan) = &state.current else {
        panic!("expected plan")
    };
    fs::write(&plan.outputs[0].path, "plan").unwrap();
    assert!(complete(&root, &state.run_id, "task-1", None, None).is_err());
    confirm_outputs(&root, "live-next", "task-1").unwrap();
    fs::write(
        root.join(".unelos/workflows/implement-feature.yaml"),
        replace_fixture_text(
            &read_fixture_text(root.join(".unelos/workflows/implement-feature.yaml")),
            "next: approve-plan",
            "next: done",
        ),
    )
    .unwrap();
    let completed = complete(&root, &state.run_id, "task-1", None, None).unwrap();
    assert!(completed.is_finished());
}

#[test]
fn confirmed_completion_rechecks_that_outputs_remain_regular_files() {
    let (_temp, root) = project_copy();
    let project = load_project(&root, "implement-feature").unwrap();
    let state = start(&project, "missing-output", "Request").unwrap();
    confirm_current(&root, &state, "plan");
    let output = execution(&state).outputs[0].path.clone();
    fs::remove_file(&output).unwrap();
    let state_path = root.join(".unelos/runs/missing-output/state.json");
    let before = fs::read(&state_path).unwrap();
    assert!(complete(&root, &state.run_id, "task-1", None, None).is_err());
    assert_eq!(fs::read(&state_path).unwrap(), before);
    fs::create_dir(&output).unwrap();
    assert!(complete(&root, &state.run_id, "task-1", None, None).is_err());
    assert_eq!(fs::read(state_path).unwrap(), before);
}

#[test]
fn incompatible_live_bindings_and_missing_successor_skill_do_not_advance() {
    let original = read_fixture_text(
        Utf8Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/project/.unelos/workflows/implement-feature.yaml"),
    );
    for edited in [
        replace_fixture_text(&original, "outputs: [plan.md]", "outputs: [changed.md]"),
        replace_fixture_text(
            &original,
            "  plan:\n    task: create-plan\n    agent: planner\n    outputs: [plan.md]\n    next: approve-plan\n",
            "  plan:\n    type: human-gate\n    next:\n      approved: implement\n      rejected: implement\n",
        ),
    ] {
        let (_temp, root) = project_copy();
        let project = load_project(&root, "implement-feature").unwrap();
        let state = start(&project, "incompatible", "Request").unwrap();
        confirm_current(&root, &state, "plan");
        let path = root.join(".unelos/runs/incompatible/state.json");
        let before = fs::read(&path).unwrap();
        fs::write(root.join(".unelos/workflows/implement-feature.yaml"), edited).unwrap();
        assert!(complete(&root, &state.run_id, "task-1", None, None).is_err());
        assert_eq!(fs::read(path).unwrap(), before);
    }

    let (_temp, root) = project_copy();
    let workflow = root.join(".unelos/workflows/implement-feature.yaml");
    fs::write(
        &workflow,
        replace_fixture_text(&original, "next: approve-plan", "next: implement"),
    )
    .unwrap();
    let project = load_project(&root, "implement-feature").unwrap();
    let state = start(&project, "changed-input", "Request").unwrap();
    confirm_current(&root, &state, "plan");
    let state = complete(&root, &state.run_id, "task-1", None, None).unwrap();
    confirm_current(&root, &state, "implementation");
    let path = root.join(".unelos/runs/changed-input/state.json");
    let before = fs::read(&path).unwrap();
    fs::write(
        &workflow,
        replace_fixture_text(
            &replace_fixture_text(&original, "next: approve-plan", "next: implement"),
            "inputs: [plan.md]\n    outputs: [implementation-report.md]",
            "inputs: []\n    outputs: [implementation-report.md]",
        ),
    )
    .unwrap();
    assert!(complete(&root, &state.run_id, "task-2", None, None).is_err());
    assert_eq!(fs::read(path).unwrap(), before);

    let (_temp, root) = project_copy();
    let workflow = root.join(".unelos/workflows/implement-feature.yaml");
    fs::write(
        &workflow,
        replace_fixture_text(&original, "next: approve-plan", "next: implement"),
    )
    .unwrap();
    let project = load_project(&root, "implement-feature").unwrap();
    let state = start(&project, "missing-skill", "Request").unwrap();
    confirm_current(&root, &state, "plan");
    let path = root.join(".unelos/runs/missing-skill/state.json");
    let before = fs::read(&path).unwrap();
    fs::remove_file(root.join(".unelos/skills/verification/SKILL.md")).unwrap();
    assert!(complete(&root, &state.run_id, "task-1", None, None).is_err());
    assert_eq!(fs::read(path).unwrap(), before);
}

#[test]
fn parallel_completion_requires_confirmed_aggregate_and_worker_reports() {
    let (_temp, root) = project_copy();
    fs::write(
        root.join(".unelos/workflows/implement-feature.yaml"),
        "name: implement-feature\nstart: parallel\nnodes:\n  parallel:\n    type: parallel\n    tasks:\n      - {task: create-plan, agent: planner}\n    outputs: [aggregate.md]\n    next: done\n  done:\n    type: end\n",
    )
    .unwrap();
    let project = load_project(&root, "implement-feature").unwrap();
    let state = start(&project, "parallel-completion", "Request").unwrap();
    let CurrentWork::Parallel(record) = &state.current else {
        panic!("expected parallel work")
    };
    let report = record.worker_reports[0].report.path.clone();
    let state_path = root.join(".unelos/runs/parallel-completion/state.json");
    let before = fs::read(&state_path).unwrap();
    assert!(confirm_outputs(&root, "parallel-completion", "task-1").is_err());
    assert_eq!(fs::read(&state_path).unwrap(), before);
    fs::write(&record.outputs[0].path, "aggregate").unwrap();
    assert!(!report.exists());
    confirm_outputs(&root, "parallel-completion", "task-1").unwrap();
    let confirmed = fs::read(&state_path).unwrap();
    assert!(complete(&root, &state.run_id, "task-1", None, None).is_err());
    assert_eq!(fs::read(&state_path).unwrap(), confirmed);
    fs::write(&report, "worker report").unwrap();
    assert!(complete(&root, &state.run_id, "task-1", None, None)
        .unwrap()
        .is_finished());
}

#[test]
fn missing_successor_producer_input_fails_without_persisting_completion() {
    let (_temp, root) = project_copy();
    let workflow = root.join(".unelos/workflows/implement-feature.yaml");
    let source = read_fixture_text(&workflow);
    let source = replace_fixture_text(&source, "next: approve-plan", "next: implement");
    let source = replace_fixture_text(
        &source,
        "inputs: [plan.md]\n    outputs: [implementation-report.md]",
        "inputs: [phantom.md]\n    outputs: [implementation-report.md]",
    );
    let source = replace_fixture_text(
        &source,
        "  done:\n    type: end\n",
        "  unvisited:\n    task: create-plan\n    agent: planner\n    outputs: [phantom.md]\n    next: done\n  done:\n    type: end\n",
    );
    fs::write(&workflow, source).unwrap();
    let project = load_project(&root, "implement-feature").unwrap();
    let state = start(&project, "missing-input", "Request").unwrap();
    confirm_current(&root, &state, "plan");
    let state_path = root.join(".unelos/runs/missing-input/state.json");
    let before = fs::read(&state_path).unwrap();
    assert!(complete(&root, &state.run_id, "task-1", None, None).is_err());
    assert_eq!(fs::read(state_path).unwrap(), before);
}

#[test]
fn successor_directory_failure_does_not_advance_state() {
    let (_temp, root) = project_copy();
    let workflow = root.join(".unelos/workflows/implement-feature.yaml");
    let source = read_fixture_text(&workflow);
    fs::write(
        &workflow,
        replace_fixture_text(&source, "next: approve-plan", "next: implement"),
    )
    .unwrap();
    let project = load_project(&root, "implement-feature").unwrap();
    let state = start(&project, "directory-failure", "Request").unwrap();
    confirm_current(&root, &state, "plan");
    let state_path = root.join(".unelos/runs/directory-failure/state.json");
    let before = fs::read(&state_path).unwrap();
    fs::write(
        root.join(".unelos/runs/directory-failure/implement"),
        "keep",
    )
    .unwrap();
    assert!(complete(&root, &state.run_id, "task-1", None, None).is_err());
    assert_eq!(fs::read(state_path).unwrap(), before);
    assert_eq!(
        fs::read(root.join(".unelos/runs/directory-failure/implement")).unwrap(),
        b"keep"
    );
}

#[cfg(unix)]
#[test]
fn successor_directory_symlink_failure_does_not_write_outside_run() {
    use std::os::unix::fs::symlink;

    let (_temp, root) = project_copy();
    let workflow = root.join(".unelos/workflows/implement-feature.yaml");
    let source = read_fixture_text(&workflow);
    fs::write(
        &workflow,
        replace_fixture_text(&source, "next: approve-plan", "next: implement"),
    )
    .unwrap();
    let project = load_project(&root, "implement-feature").unwrap();
    let state = start(&project, "directory-symlink", "Request").unwrap();
    confirm_current(&root, &state, "plan");
    let state_path = root.join(".unelos/runs/directory-symlink/state.json");
    let before = fs::read(&state_path).unwrap();
    let outside = root.join("outside");
    fs::create_dir(&outside).unwrap();
    symlink(
        &outside,
        root.join(".unelos/runs/directory-symlink/implement"),
    )
    .unwrap();
    assert!(complete(&root, &state.run_id, "task-1", None, None).is_err());
    assert_eq!(fs::read(state_path).unwrap(), before);
    assert_eq!(fs::read_dir(outside).unwrap().count(), 0);
}

fn outputs(work: &CurrentWork) -> &[unelos_core::ArtifactReference] {
    match work {
        CurrentWork::Task(record) | CurrentWork::Parallel(record) => &record.outputs,
        _ => &[],
    }
}
