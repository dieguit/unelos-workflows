use assert_fs::TempDir;
use camino::{Utf8Path, Utf8PathBuf};
use unelos_core::*;

fn fixture() -> (TempDir, Utf8PathBuf) {
    let temp = TempDir::new().unwrap();
    let root = Utf8Path::from_path(temp.path()).unwrap().to_owned();
    install_workflow(
        &root,
        Some(
            &Utf8Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures/simple-plan-execute-review"),
        ),
        "simple-plan-execute-review",
    )
    .unwrap();
    (temp, root)
}

#[test]
fn lists_all_statuses_without_live_workflow_and_tracks_updates() {
    let (_temp, root) = fixture();
    let active = start_workflow(&root, "simple-plan-execute-review", None).unwrap();
    assert!(active.started_at.is_some());
    assert_eq!(active.started_at, active.updated_at);
    let mut gate = start_workflow(&root, "simple-plan-execute-review", None).unwrap();
    for _ in 0..2 {
        let CurrentWork::Task(record) = &gate.current else {
            panic!()
        };
        for output in &record.outputs {
            std::fs::write(&output.path, "report").unwrap();
        }
        gate = complete_node(&root, &gate.run_id, &record.node_id, None).unwrap();
    }
    let workflow = root.join(".unelos/workflows/simple-plan-execute-review.yaml");
    let definition = std::fs::read_to_string(&workflow).unwrap();
    std::fs::write(
        &workflow,
        definition.replace("start: interview", "start: finish"),
    )
    .unwrap();
    let finished = start_workflow(&root, "simple-plan-execute-review", None).unwrap();
    let renamed = rename_run(&root, &active.run_id, "renamed").unwrap();
    assert_eq!(renamed.started_at, active.started_at);
    assert!(renamed.updated_at.unwrap() >= active.updated_at.unwrap());
    std::fs::remove_file(&workflow).unwrap();
    let runs = list_runs(&root).unwrap();
    assert_eq!(runs.len(), 3);
    for (id, status, node) in [
        (&active.run_id, "running", "interview"),
        (&gate.run_id, "waiting", "approve-plan"),
        (&finished.run_id, "finished", "finish"),
    ] {
        let row = runs.iter().find(|r| &r.run_id == id).unwrap();
        assert_eq!(row.status, status);
        assert_eq!(row.node_id, node);
    }
    assert!(runs.windows(2).all(|r| r[0].updated_at > r[1].updated_at
        || (r[0].updated_at == r[1].updated_at && r[0].run_id < r[1].run_id)));
}

#[test]
fn artifacts_are_scoped_and_delete_removes_the_run() {
    let (_temp, root) = fixture();
    let state = start_workflow(&root, "simple-plan-execute-review", None).unwrap();
    let CurrentWork::Task(record) = &state.current else {
        panic!()
    };
    let path = &record.outputs[0].path;
    assert!(read_run_artifact(&root, &state.run_id, path).is_err());
    assert!(!get_run(&root, &state.run_id)
        .unwrap()
        .existing_artifact_paths
        .contains(&path.to_string()));
    std::fs::write(path, "report").unwrap();
    let details = get_run(&root, &state.run_id).unwrap();
    assert!(details.existing_artifact_paths.contains(&path.to_string()));
    let json = serde_json::to_value(&details).unwrap();
    assert_eq!(json["runId"], state.run_id);
    assert!(json.get("state").is_none());
    assert!(json["existingArtifactPaths"].is_array());
    assert!(std::fs::read_to_string(
        root.join(".unelos/runs")
            .join(&state.run_id)
            .join("state.json")
    )
    .unwrap()
    .find("existingArtifactPaths")
    .is_none());
    assert_eq!(
        read_run_artifact(&root, &state.run_id, path).unwrap(),
        "report"
    );
    assert!(read_run_artifact(
        &root,
        &state.run_id,
        &root.join(".unelos/workflows/simple-plan-execute-review.yaml")
    )
    .is_err());
    assert!(delete_run(&root, "../outside").is_err());
    let state_path = root
        .join(".unelos/runs")
        .join(&state.run_id)
        .join("state.json");
    let mut old = serde_json::to_value(&state).unwrap();
    old.as_object_mut().unwrap().remove("startedAt");
    old.as_object_mut().unwrap().remove("updatedAt");
    std::fs::write(&state_path, serde_json::to_vec(&old).unwrap()).unwrap();
    assert!(get_run(&root, &state.run_id)
        .unwrap()
        .state
        .started_at
        .is_none());
    delete_run(&root, &state.run_id).unwrap();
    assert!(!state_path.parent().unwrap().exists());
}

#[cfg(unix)]
#[test]
fn artifact_symlinks_are_rejected() {
    let (_temp, root) = fixture();
    let state = start_workflow(&root, "simple-plan-execute-review", None).unwrap();
    let CurrentWork::Task(record) = &state.current else {
        panic!()
    };
    std::os::unix::fs::symlink(
        root.join(".unelos/workflows/simple-plan-execute-review.yaml"),
        &record.outputs[0].path,
    )
    .unwrap();
    assert!(read_run_artifact(&root, &state.run_id, &record.outputs[0].path).is_err());
    assert!(get_run(&root, &state.run_id).is_err());
}

#[test]
fn non_regular_and_unreadable_artifacts_cannot_be_read() {
    let (_temp, root) = fixture();
    let state = start_workflow(&root, "simple-plan-execute-review", None).unwrap();
    let CurrentWork::Task(record) = &state.current else {
        panic!()
    };
    let path = &record.outputs[0].path;
    std::fs::create_dir(path).unwrap();
    assert!(!get_run(&root, &state.run_id)
        .unwrap()
        .existing_artifact_paths
        .contains(&path.to_string()));
    assert!(read_run_artifact(&root, &state.run_id, path).is_err());
    std::fs::remove_dir(path).unwrap();
    std::fs::write(path, [0xff]).unwrap();
    assert!(read_run_artifact(&root, &state.run_id, path).is_err());
}
