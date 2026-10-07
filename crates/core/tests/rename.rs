use assert_fs::{fixture::PathCopy, TempDir};
use camino::{Utf8Path, Utf8PathBuf};
use std::fs;
use unelos_core::{
    complete, confirm_outputs, next, reject, rename_run, start_workflow, status, CurrentWork,
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

#[test]
fn rename_rebases_history_gate_feedback_and_current_work_references() {
    let (_temp, root) = project_copy();
    let started = start_workflow(&root, "implement-feature", Some("review-feature")).unwrap();
    let CurrentWork::Task(plan) = &started.current else {
        panic!("task");
    };
    fs::write(&plan.outputs[0].path, "plan").unwrap();
    confirm_outputs(&root, "task-1").unwrap();
    let gate = complete(&root, &started.run_id, "task-1", None, None).unwrap();
    let rejected = reject(&root, &started.run_id, "gate-2", "revise").unwrap();
    let renamed = rename_run(&root, &started.run_id, "updated-feature").unwrap();
    assert_eq!(renamed.history.len(), rejected.history.len());

    let old_prefix = root.join(".unelos/runs/review-feature").to_string();
    let new_prefix = root.join(".unelos/runs/updated-feature").to_string();
    let reference_paths = |work: &CurrentWork| {
        let (CurrentWork::Task(record)
        | CurrentWork::Parallel(record)
        | CurrentWork::HumanGate(record)) = work
        else {
            return Vec::new();
        };
        record
            .inputs
            .iter()
            .chain(&record.outputs)
            .chain(record.worker_reports.iter().map(|report| &report.report))
            .chain(
                record
                    .feedback
                    .iter()
                    .flat_map(|feedback| &feedback.artifacts),
            )
            .map(|reference| reference.path.to_string())
            .collect::<Vec<_>>()
    };
    for work in
        std::iter::once(&renamed.current).chain(renamed.history.iter().map(|entry| &entry.work))
    {
        for path in reference_paths(work) {
            assert!(path.starts_with(&new_prefix), "{path}");
            assert!(!path.starts_with(&old_prefix), "{path}");
        }
    }
    let feedback = match &renamed.current {
        CurrentWork::Task(record) => record.feedback.as_ref().unwrap(),
        _ => panic!("rejected work should return to task"),
    };
    assert_eq!(feedback.reason.as_deref(), Some("revise"));
    let (CurrentWork::HumanGate(old_gate), CurrentWork::HumanGate(new_gate)) =
        (&gate.current, &renamed.history[1].work)
    else {
        panic!("gate history");
    };
    assert_eq!(new_gate.execution_id, old_gate.execution_id);
    assert_eq!(new_gate.node_id, old_gate.node_id);
    assert_eq!(renamed.history[1].outcome.as_deref(), Some("rejected"));
    assert_eq!(status(&root).unwrap(), renamed);
}

#[test]
fn rename_rebases_every_repeated_parallel_worker_report() {
    let (_temp, root) = project_copy();
    fs::write(
        root.join(".unelos/workflows/implement-feature.yaml"),
        "name: implement-feature\nstart: parallel\nnodes:\n  parallel:\n    type: parallel\n    tasks:\n      - {task: create-plan, agent: planner}\n      - {task: create-plan, agent: planner}\n    next: done\n  done:\n    type: end\n",
    )
    .unwrap();
    let started = start_workflow(&root, "implement-feature", None).unwrap();
    let CurrentWork::Parallel(_) = &started.current else {
        panic!("parallel");
    };
    let renamed = rename_run(&root, &started.run_id, "parallel-run").unwrap();
    let CurrentWork::Parallel(parallel) = &renamed.current else {
        panic!("parallel");
    };
    assert_eq!(parallel.worker_reports.len(), 2);
    for report in &parallel.worker_reports {
        assert!(report
            .report
            .path
            .starts_with(root.join(".unelos/runs/parallel-run")));
    }
    let packet = next(&root).unwrap();
    let unelos_core::WorkPacket::Parallel(packet) = packet else {
        panic!("parallel packet");
    };
    assert_eq!(
        packet.workers[0].outputs[0].path,
        parallel.worker_reports[0].report.path
    );
    assert_eq!(
        packet.workers[1].outputs[0].path,
        parallel.worker_reports[1].report.path
    );
}

#[test]
fn rename_validates_name_id_collision_and_same_name_settlement() {
    let (_temp, root) = project_copy();
    let started = start_workflow(&root, "implement-feature", None).unwrap();
    assert!(rename_run(&root, "wrong-id", "safe-name").is_err());
    for name in ["", "../escape", "Unsafe", "a--b", "con"] {
        assert!(rename_run(&root, &started.run_id, name).is_err());
    }
    fs::create_dir(root.join(".unelos/runs/collision")).unwrap();
    assert!(rename_run(&root, &started.run_id, "collision").is_err());
    let settled = rename_run(&root, &started.run_id, &started.name).unwrap();
    assert!(!settled.naming_pending);
    assert_eq!(settled.run_id, started.run_id);
    let manual = rename_run(&root, &started.run_id, "manual-name").unwrap();
    assert_eq!(manual.name, "manual-name");
    assert_eq!(manual.run_id, started.run_id);
}

#[test]
fn legacy_state_without_naming_metadata_can_be_renamed() {
    let (_temp, root) = project_copy();
    let started = start_workflow(&root, "implement-feature", Some("legacy-run")).unwrap();
    let path = root.join(".unelos/runs/legacy-run/state.json");
    let mut saved: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    saved.as_object_mut().unwrap().remove("namingPending");
    fs::write(&path, serde_json::to_vec(&saved).unwrap()).unwrap();
    let renamed = rename_run(&root, &started.run_id, "renamed-legacy").unwrap();
    assert_eq!(renamed.name, "renamed-legacy");
    assert!(!renamed.naming_pending);
}

#[test]
fn renamed_finished_run_id_is_not_reused_after_sequence_reset() {
    let (_temp, root) = project_copy();
    fs::write(
        root.join(".unelos/workflows/implement-feature.yaml"),
        "name: implement-feature\nstart: done\nnodes:\n  done:\n    type: end\n",
    )
    .unwrap();

    let first = start_workflow(&root, "implement-feature", None).unwrap();
    assert!(matches!(first.current, CurrentWork::Finished { .. }));
    rename_run(&root, &first.run_id, "renamed-finished-run").unwrap();
    fs::remove_file(root.join(".unelos/run-sequence")).unwrap();

    let second = start_workflow(&root, "implement-feature", None).unwrap();
    assert!(matches!(second.current, CurrentWork::Finished { .. }));
    assert_ne!(second.run_id, first.run_id);
}

#[test]
fn rename_moves_reports_preserves_id_and_refreshes_packet_paths() {
    let (_temp, root) = project_copy();
    let started = start_workflow(&root, "implement-feature", None).unwrap();
    let CurrentWork::Task(record) = &started.current else {
        panic!("task");
    };
    fs::write(&record.outputs[0].path, "Keep these bytes").unwrap();
    let old_directory = root.join(".unelos/runs").join(&started.name);
    fs::write(
        old_directory.join("ledger.md"),
        "Controller prose stays unchanged",
    )
    .unwrap();
    let renamed = rename_run(&root, &started.run_id, "fix-save-button").unwrap();
    assert_eq!(renamed.run_id, started.run_id);
    assert_eq!(
        renamed.next_execution_sequence,
        started.next_execution_sequence
    );
    assert!(!renamed.naming_pending);
    assert!(!old_directory.exists());
    assert_eq!(status(&root).unwrap(), renamed);
    let packet = serde_json::to_value(next(&root).unwrap()).unwrap();
    let path = packet["outputs"][0]["path"].as_str().unwrap();
    assert!(std::path::Path::new(path).ends_with(
        std::path::Path::new("fix-save-button")
            .join("plan")
            .join("plan.md")
    ));
    assert_eq!(fs::read_to_string(path).unwrap(), "Keep these bytes");
    assert_eq!(
        fs::read_to_string(root.join(".unelos/runs/fix-save-button/ledger.md")).unwrap(),
        "Controller prose stays unchanged"
    );
}
