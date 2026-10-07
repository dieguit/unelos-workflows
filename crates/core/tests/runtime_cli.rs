use assert_fs::{fixture::PathCopy, TempDir};
use camino::{Utf8Path, Utf8PathBuf};
use std::fs;
use unelos_core::{list_workflows, load_project, next, start, start_workflow, status};

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
    assert_eq!(status(&root).unwrap(), state);
    assert_eq!(next(&root).unwrap(), next(&root).unwrap());
}

#[test]
fn explicit_start_separates_id_and_name() {
    let (_temp, root) = project_copy();
    let state = start_workflow(&root, "implement-feature", Some("add-search")).unwrap();
    assert_eq!(state.name, "add-search");
    assert_ne!(state.run_id, state.name);
    assert!(!state.naming_pending);
    assert_eq!(status(&root).unwrap(), state);
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
    let saved = status(&root).unwrap();
    assert!(!saved.naming_pending);
    assert_eq!(saved.request, "Keep request");
}

#[test]
fn discovery_accepts_empty_workflows_and_ignores_non_yaml_files() {
    let (_temp, root) = project_copy();
    fs::write(root.join(".unelos/workflows/readme.txt"), "not a Workflow").unwrap();
    assert_eq!(list_workflows(&root).unwrap(), vec!["implement-feature"]);
    fs::remove_file(root.join(".unelos/workflows/implement-feature.yaml")).unwrap();
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
fn collisions_and_second_unfinished_start_leave_published_bytes_unchanged() {
    let (_temp, root) = project_copy();
    let state = start_workflow(&root, "implement-feature", Some("first")).unwrap();
    let pointer_path = root.join(".unelos/active-run");
    let state_path = root.join(".unelos/runs/first/state.json");
    let pointer = fs::read(&pointer_path).unwrap();
    let saved = fs::read(&state_path).unwrap();
    assert!(start_workflow(&root, "implement-feature", Some("second")).is_err());
    assert_eq!(fs::read(pointer_path).unwrap(), pointer);
    assert_eq!(fs::read(state_path).unwrap(), saved);
    assert_eq!(status(&root).unwrap(), state);

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
    fs::write(state_path, serde_json::to_vec(&saved).unwrap()).unwrap();
    let generated = start_workflow(&root, "finished", None).unwrap();
    assert_ne!(generated.run_id, "r-000001");

    let pointer = fs::read(root.join(".unelos/active-run")).unwrap();
    fs::write(root.join(".unelos/run-sequence"), "not-a-number").unwrap();
    assert!(start_workflow(&root, "finished", Some("third")).is_err());
    assert_eq!(fs::read(root.join(".unelos/active-run")).unwrap(), pointer);
    assert!(!root.join(".unelos/runs/third").exists());
}
