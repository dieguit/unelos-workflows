use assert_fs::{fixture::PathCopy, TempDir};
use camino::Utf8Path;
use std::fs;
use unelos_core::{
    confirm_outputs, load_project, next, prepare_work, start, status, verify_outputs,
    verify_work_outputs, ArtifactReference, CompletedExecution, CurrentWork, TransitionFeedback,
};

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

fn text(path: &Utf8Path) -> String {
    fs::read_to_string(path).unwrap().replace("\r\n", "\n")
}

#[test]
fn binds_latest_recorded_producer_and_preserves_feedback_and_caller_state() {
    let (_temp, root) = project_copy();
    let project = load_project(&root, "implement-feature").unwrap();
    let mut state = start(&project, "binding-run", "Change color").unwrap();
    let before = state.clone();
    assert!(prepare_work(&project, &state, "implement", None).is_err());
    assert_eq!(state, before);

    let CurrentWork::Task(plan) = &state.current else {
        panic!("expected plan")
    };
    let plan_path = plan.outputs[0].path.clone();
    fs::write(&plan_path, "Plan").unwrap();
    state.history.push(CompletedExecution {
        work: state.current.clone(),
        outcome: None,
        reason: None,
    });
    let before_preparation = state.clone();
    let candidate = prepare_work(&project, &state, "implement", None).unwrap();
    let CurrentWork::Task(work) = &candidate.current else {
        panic!("expected implement")
    };
    assert_eq!(work.execution_id, "task-2");
    assert_eq!(work.inputs[0].path, plan_path);
    assert!(work.outputs[0]
        .path
        .ends_with("implement/implementation-report.md"));
    assert_eq!(fs::read(&plan_path).unwrap(), b"Plan");
    assert!(!work.outputs[0].path.exists());
    assert_eq!(state, before_preparation);

    let feedback = TransitionFeedback {
        source_node_id: "plan".into(),
        source_execution_id: "task-1".into(),
        outcome: "rejected".into(),
        reason: Some("Add detail".into()),
        artifacts: vec![ArtifactReference {
            name: "plan.md".into(),
            path: plan_path.clone(),
        }],
    };
    let corrected = prepare_work(&project, &state, "implement", Some(feedback.clone())).unwrap();
    let CurrentWork::Task(corrected) = corrected.current else {
        panic!("expected task")
    };
    assert_eq!(corrected.feedback, Some(feedback));
}

#[test]
fn preparation_requires_the_latest_completed_binding_to_be_a_file() {
    let (_temp, root) = project_copy();
    let project = load_project(&root, "implement-feature").unwrap();
    let mut state = start(&project, "missing-latest", "request").unwrap();
    let CurrentWork::Task(plan) = &state.current else {
        panic!("expected plan")
    };
    let first = plan.outputs[0].clone();
    fs::write(&first.path, "old").unwrap();
    state.history.push(CompletedExecution {
        work: CurrentWork::Task(plan.clone()),
        outcome: None,
        reason: None,
    });
    let mut newer = plan.clone();
    newer.outputs[0].path = root.join(".unelos/runs/missing-latest/plan/new-plan.md");
    fs::create_dir_all(newer.outputs[0].path.parent().unwrap()).unwrap();
    fs::write(&newer.outputs[0].path, "new").unwrap();
    state.history.push(CompletedExecution {
        work: CurrentWork::Task(newer.clone()),
        outcome: None,
        reason: None,
    });
    assert!(prepare_work(&project, &state, "implement", None).is_ok());
    fs::remove_file(&newer.outputs[0].path).unwrap();
    fs::write(root.join("plan.md"), "unrelated").unwrap();
    assert!(prepare_work(&project, &state, "implement", None).is_err());
    fs::create_dir(&newer.outputs[0].path).unwrap();
    assert!(prepare_work(&project, &state, "implement", None).is_err());
}

#[test]
fn gate_and_end_preparation_allocate_as_expected() {
    let (_temp, root) = project_copy();
    let workflow = root.join(".unelos/workflows/implement-feature.yaml");
    let original = text(&workflow);
    fs::write(&workflow, original).unwrap();
    let project = load_project(&root, "implement-feature").unwrap();
    let mut state = start(&project, "gate-binding", "request").unwrap();
    let CurrentWork::Task(plan) = &state.current else {
        panic!("expected plan")
    };
    fs::write(&plan.outputs[0].path, "plan").unwrap();
    state.history.push(CompletedExecution {
        work: state.current.clone(),
        outcome: None,
        reason: None,
    });
    let gate = prepare_work(&project, &state, "approve-plan", None).unwrap();
    let CurrentWork::HumanGate(gate) = gate.current else {
        panic!("expected gate")
    };
    assert_eq!(gate.execution_id, "gate-2");
    assert_eq!(gate.inputs[0].path, plan.outputs[0].path);

    let end = prepare_work(&project, &state, "done", None).unwrap();
    assert!(matches!(end.current, CurrentWork::Finished { .. }));
    assert_eq!(end.next_execution_sequence, state.next_execution_sequence);
}

#[test]
fn next_keeps_frozen_inputs_and_feedback_and_rejects_missing_bound_files() {
    let (_temp, root) = project_copy();
    let project = load_project(&root, "implement-feature").unwrap();
    let mut state = start(&project, "frozen-binding", "request").unwrap();
    let CurrentWork::Task(plan) = &state.current else {
        panic!("expected plan")
    };
    let original = plan.outputs[0].clone();
    fs::write(&original.path, "initial plan").unwrap();
    state.history.push(CompletedExecution {
        work: state.current.clone(),
        outcome: None,
        reason: None,
    });
    let feedback = TransitionFeedback {
        source_node_id: "plan".into(),
        source_execution_id: "task-1".into(),
        outcome: "rejected".into(),
        reason: Some("Clarify design".into()),
        artifacts: vec![original.clone()],
    };
    let state = prepare_work(&project, &state, "implement", Some(feedback.clone())).unwrap();
    let state_path = root.join(".unelos/runs/frozen-binding/state.json");
    fs::write(&state_path, serde_json::to_vec(&state).unwrap()).unwrap();

    let first = serde_json::to_value(next(&root).unwrap()).unwrap();
    assert_eq!(first["feedback"]["reason"], "Clarify design");
    assert_eq!(serde_json::to_value(next(&root).unwrap()).unwrap(), first);
    fs::write(&original.path, "edited report bytes").unwrap();
    assert_eq!(
        serde_json::to_value(next(&root).unwrap()).unwrap()["feedback"],
        first["feedback"]
    );

    let mut newer = match &state.current {
        CurrentWork::Task(record) => record.clone(),
        _ => unreachable!(),
    };
    newer.node_id = "plan".into();
    newer.execution_id = "task-99".into();
    newer.outputs[0].path = root.join(".unelos/runs/frozen-binding/plan/newer.md");
    let mut saved = state.clone();
    saved.history.push(CompletedExecution {
        work: CurrentWork::Task(newer),
        outcome: None,
        reason: None,
    });
    fs::write(&state_path, serde_json::to_vec(&saved).unwrap()).unwrap();
    let repeated = serde_json::to_value(next(&root).unwrap()).unwrap();
    assert_eq!(repeated["inputs"][0]["path"], original.path.as_str());

    fs::remove_file(&original.path).unwrap();
    assert!(next(&root).is_err());
    assert_eq!(state.current, saved.current);
    assert_eq!(
        feedback,
        match &saved.current {
            CurrentWork::Task(record) => record.feedback.clone().unwrap(),
            _ => unreachable!(),
        }
    );
}

#[test]
fn next_rejects_changed_live_input_declarations_without_rebinding() {
    let (_temp, root) = project_copy();
    let project = load_project(&root, "implement-feature").unwrap();
    let mut state = start(&project, "changed-input", "request").unwrap();
    let CurrentWork::Task(plan) = &state.current else {
        panic!("expected plan")
    };
    fs::write(&plan.outputs[0].path, "plan").unwrap();
    state.history.push(CompletedExecution {
        work: state.current.clone(),
        outcome: None,
        reason: None,
    });
    let state = prepare_work(&project, &state, "implement", None).unwrap();
    let state_path = root.join(".unelos/runs/changed-input/state.json");
    let bytes = serde_json::to_vec(&state).unwrap();
    fs::write(&state_path, &bytes).unwrap();

    let workflow = root.join(".unelos/workflows/implement-feature.yaml");
    let changed = text(&workflow).replace("inputs: [plan.md]", "inputs: []");
    fs::write(workflow, changed).unwrap();
    assert!(next(&root).is_err());
    assert_eq!(fs::read(state_path).unwrap(), bytes);
}

#[cfg(unix)]
#[test]
fn preparation_rejects_external_and_symlinked_history_input_paths() {
    use std::os::unix::fs::symlink;

    for symlinked in [false, true] {
        let (_temp, root) = project_copy();
        let project = load_project(&root, "implement-feature").unwrap();
        let mut state = start(&project, "unsafe-binding", "request").unwrap();
        let CurrentWork::Task(mut plan) = state.current.clone() else {
            panic!("expected plan")
        };
        let binding = if symlinked {
            let outside = root.join("outside.md");
            fs::write(&outside, "outside").unwrap();
            let path = root.join(".unelos/runs/unsafe-binding/plan/link.md");
            symlink(outside, &path).unwrap();
            path
        } else {
            root.join("outside.md")
        };
        plan.outputs[0].path = binding;
        state.history.push(CompletedExecution {
            work: CurrentWork::Task(plan),
            outcome: None,
            reason: None,
        });
        assert!(prepare_work(&project, &state, "implement", None).is_err());
    }
}

#[test]
fn confirms_outputs_only_for_current_execution_and_rechecks_files() {
    let (_temp, root) = project_copy();
    let project = load_project(&root, "implement-feature").unwrap();
    let state = start(&project, "confirm-run", "request").unwrap();

    assert!(verify_outputs(&root, "task-1").is_err());
    assert!(confirm_outputs(&root, "task-1").is_err());
    let path = root.join(".unelos/runs/confirm-run/plan/plan.md");
    fs::write(&path, "").unwrap();
    let confirmed = confirm_outputs(&root, "task-1").unwrap();
    assert!(verify_outputs(&root, "task-1").is_ok());
    verify_work_outputs(&root, &confirmed, "task-1").unwrap();
    assert_eq!(confirm_outputs(&root, "task-1").unwrap(), confirmed);
    assert!(confirm_outputs(&root, "task-99").is_err());
    let state_path = root.join(".unelos/runs/confirm-run/state.json");
    let confirmed_bytes = fs::read(&state_path).unwrap();
    fs::remove_file(&path).unwrap();
    fs::create_dir(&path).unwrap();
    assert!(verify_outputs(&root, "task-1").is_err());
    assert!(confirm_outputs(&root, "task-1").is_err());
    assert_eq!(fs::read(&state_path).unwrap(), confirmed_bytes);

    let unconfirmed = state;
    assert!(verify_work_outputs(&root, &unconfirmed, "task-1").is_err());
}

#[test]
fn old_state_defaults_to_unconfirmed_and_receipts_must_match() {
    let (_temp, root) = project_copy();
    let project = load_project(&root, "implement-feature").unwrap();
    start(&project, "old-state", "request").unwrap();
    let state_path = root.join(".unelos/runs/old-state/state.json");
    let mut saved: serde_json::Value =
        serde_json::from_slice(&fs::read(&state_path).unwrap()).unwrap();
    saved["current"]
        .as_object_mut()
        .unwrap()
        .remove("confirmedExecutionId");
    fs::write(&state_path, serde_json::to_vec(&saved).unwrap()).unwrap();
    assert!(
        matches!(status(&root).unwrap().current, CurrentWork::Task(record) if record.confirmed_execution_id.is_none())
    );
    assert!(verify_outputs(&root, "task-1").is_err());

    saved["current"]["confirmedExecutionId"] = "task-99".into();
    fs::write(&state_path, serde_json::to_vec(&saved).unwrap()).unwrap();
    assert!(status(&root).is_err());
}

#[test]
fn rework_resets_confirmation_even_when_it_reuses_the_output_path() {
    let (_temp, root) = project_copy();
    let project = load_project(&root, "implement-feature").unwrap();
    let mut state = start(&project, "reconfirm-run", "request").unwrap();
    let CurrentWork::Task(record) = &state.current else {
        panic!("expected task")
    };
    let output = record.outputs[0].path.clone();
    fs::write(&output, "reviewed unchanged").unwrap();
    let confirmed = confirm_outputs(&root, "task-1").unwrap();
    assert!(matches!(confirmed.current, CurrentWork::Task(_)));
    state.history.push(CompletedExecution {
        work: confirmed.current.clone(),
        outcome: None,
        reason: None,
    });

    let rework = prepare_work(&project, &state, "plan", None).unwrap();
    let CurrentWork::Task(record) = &rework.current else {
        panic!("expected task")
    };
    assert_eq!(record.execution_id, "task-2");
    assert_eq!(record.outputs[0].path, output);
    assert_eq!(record.confirmed_execution_id, None);
    assert!(verify_work_outputs(&root, &rework, "task-2").is_err());
}

#[test]
fn confirmation_is_atomic_for_all_outputs_and_required_even_for_empty_outputs() {
    let (_temp, root) = project_copy();
    fs::write(root.join(".unelos/workflows/implement-feature.yaml"), "name: implement-feature\nstart: plan\nnodes:\n  plan:\n    task: create-plan\n    agent: planner\n    outputs: [one.md, two.md]\n    next: done\n  done:\n    type: end\n").unwrap();
    let project = load_project(&root, "implement-feature").unwrap();
    let started = start(&project, "atomic-confirm", "request").unwrap();
    let CurrentWork::Task(record) = &started.current else {
        panic!("expected task")
    };
    fs::write(&record.outputs[0].path, "one").unwrap();
    assert!(confirm_outputs(&root, "task-1").is_err());
    assert!(
        matches!(status(&root).unwrap().current, CurrentWork::Task(record) if record.confirmed_execution_id.is_none())
    );
    fs::write(&record.outputs[1].path, "two").unwrap();
    confirm_outputs(&root, "task-1").unwrap();
    assert!(verify_outputs(&root, "task-1").is_ok());

    let (_temp, root) = project_copy();
    fs::write(root.join(".unelos/workflows/implement-feature.yaml"), "name: implement-feature\nstart: plan\nnodes:\n  plan:\n    task: create-plan\n    agent: planner\n    outputs: []\n    next: done\n  done:\n    type: end\n").unwrap();
    let project = load_project(&root, "implement-feature").unwrap();
    start(&project, "empty-confirm", "request").unwrap();
    assert!(verify_outputs(&root, "task-1").is_err());
    assert!(confirm_outputs(&root, "task-1").is_ok());
    assert!(verify_outputs(&root, "task-1").is_ok());
}

#[test]
fn parallel_verification_requires_only_parent_aggregate_outputs() {
    let (_temp, root) = project_copy();
    fs::write(root.join(".unelos/workflows/implement-feature.yaml"), "name: implement-feature\nstart: parallel\nnodes:\n  parallel:\n    type: parallel\n    tasks:\n      - {task: create-plan, agent: planner}\n    outputs: [aggregate.md]\n    next: done\n  done:\n    type: end\n").unwrap();
    let project = load_project(&root, "implement-feature").unwrap();
    let state = start(&project, "aggregate-only", "request").unwrap();
    let CurrentWork::Parallel(record) = &state.current else {
        panic!("expected parallel")
    };
    fs::write(&record.outputs[0].path, "aggregate").unwrap();
    assert!(!record.worker_reports[0].report.path.exists());
    confirm_outputs(&root, "task-1").unwrap();
    assert!(verify_outputs(&root, "task-1").is_ok());
    fs::remove_file(&record.outputs[0].path).unwrap();
    assert!(verify_outputs(&root, "task-1").is_err());
}

#[test]
fn confirmation_rejects_gates_and_finished_work() {
    for (workflow, name) in [
        ("name: implement-feature\nstart: gate\nnodes:\n  gate:\n    type: human-gate\n    next: {approved: done, rejected: done}\n  done:\n    type: end\n", "gate-confirm"),
        ("name: implement-feature\nstart: done\nnodes:\n  done:\n    type: end\n", "end-confirm"),
    ] {
        let (_temp, root) = project_copy();
        fs::write(root.join(".unelos/workflows/implement-feature.yaml"), workflow).unwrap();
        let project = load_project(&root, "implement-feature").unwrap();
        let state = start(&project, name, "request").unwrap();
        assert!(confirm_outputs(&root, if state.is_finished() { "task-1" } else { "gate-1" }).is_err());
    }
}
