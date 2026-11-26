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
fn independent_unfinished_runs_do_not_redirect_reads_or_writes() {
    let (_temp, root) = fixture();
    let a = start_workflow(&root, "simple-plan-execute-review", Some("first-run")).unwrap();
    let b = start_workflow(&root, "simple-plan-execute-review", Some("second-run")).unwrap();
    let b_path = root.join(".unelos/runs").join(&b.run_id).join("state.json");
    let before_b = std::fs::read(&b_path).unwrap();
    assert_eq!(status(&root, &a.run_id).unwrap(), a);
    assert_eq!(status(&root, &b.run_id).unwrap(), b);
    let packet = serde_json::to_value(next(&root, &a.run_id).unwrap()).unwrap();
    for output in packet["outputs"].as_array().unwrap() {
        std::fs::write(output["path"].as_str().unwrap(), "Requirements A\n").unwrap();
    }
    complete_node(&root, &a.run_id, "interview", None).unwrap();
    assert_eq!(std::fs::read(&b_path).unwrap(), before_b);
    assert_eq!(status(&root, &b.run_id).unwrap(), b);
    assert_eq!(list_runs(&root).unwrap().len(), 2);
    assert!(!root.join(".unelos/active-run").exists());
}

#[test]
fn simultaneous_starts_allocate_distinct_ids() {
    let (_temp, root) = fixture();
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
    let handles: Vec<_> = ["first-run", "second-run"]
        .into_iter()
        .map(|name| {
            let root = root.clone();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                barrier.wait();
                start_workflow(&root, "simple-plan-execute-review", Some(name)).unwrap()
            })
        })
        .collect();
    let runs: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();
    assert_ne!(runs[0].run_id, runs[1].run_id);
    assert_eq!(list_runs(&root).unwrap().len(), 2);
}

#[test]
fn rename_keeps_identity_and_ignores_obsolete_pointer() {
    let (_temp, root) = fixture();
    let a = start_workflow(&root, "simple-plan-execute-review", Some("first-run")).unwrap();
    let b = start_workflow(&root, "simple-plan-execute-review", Some("second-run")).unwrap();
    std::fs::write(root.join(".unelos/active-run"), "../invalid-old-pointer").unwrap();
    let before_b =
        std::fs::read(root.join(".unelos/runs").join(&b.run_id).join("state.json")).unwrap();
    rename_run(&root, &a.run_id, "renamed-first").unwrap();
    assert_eq!(status(&root, &a.run_id).unwrap().name, "renamed-first");
    assert_eq!(status(&root, &b.run_id).unwrap(), b);
    assert_eq!(
        std::fs::read(root.join(".unelos/runs").join(&b.run_id).join("state.json")).unwrap(),
        before_b
    );
    assert_eq!(
        std::fs::read_to_string(root.join(".unelos/active-run")).unwrap(),
        "../invalid-old-pointer"
    );
}

#[test]
fn cross_run_callback_with_same_task_label_never_writes_either_run() {
    let (_temp, root) = fixture();
    let a = execution_start(&root, "simple-plan-execute-review", Some("first-run"), "").unwrap();
    let b = execution_start(&root, "simple-plan-execute-review", Some("second-run"), "").unwrap();
    let a_path = root
        .join(".unelos/runs")
        .join(a.run_id.as_deref().unwrap())
        .join("state.json");
    let b_path = root
        .join(".unelos/runs")
        .join(b.run_id.as_deref().unwrap())
        .join("state.json");
    let before = (
        std::fs::read(&a_path).unwrap(),
        std::fs::read(&b_path).unwrap(),
    );
    let expected = a.expected.unwrap();
    let control = CompletionControl {
        summary: Some("Inspected A".into()),
        expected,
        result_text: None,
        outcome: None,
        reason: None,
    };
    assert!(execution_mutate(
        &root,
        b.run_id.as_deref().unwrap(),
        ExecutionOperation::Complete,
        &control
    )
    .is_err());
    assert_eq!(
        (
            std::fs::read(a_path).unwrap(),
            std::fs::read(b_path).unwrap()
        ),
        before
    );
}

#[test]
fn duplicate_persisted_ids_fail_lookup_and_discovery() {
    let (_temp, root) = fixture();
    let a = start_workflow(&root, "simple-plan-execute-review", Some("first-run")).unwrap();
    let b = start_workflow(&root, "simple-plan-execute-review", Some("second-run")).unwrap();
    let path = root.join(".unelos/runs").join(&b.run_id).join("state.json");
    let mut saved: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    saved["runId"] = a.run_id.clone().into();
    std::fs::write(&path, serde_json::to_vec(&saved).unwrap()).unwrap();
    assert!(matches!(
        list_runs(&root),
        Err(RuntimeError::InvalidState(_))
    ));
    assert!(matches!(
        status(&root, &a.run_id),
        Err(RuntimeError::InvalidState(_))
    ));
}

#[test]
fn legacy_name_identity_survives_another_start_and_rename() {
    let (_temp, root) = fixture();
    let project = load_project(&root, "simple-plan-execute-review").unwrap();
    let legacy = start(&project, "legacy-name", "Legacy request").unwrap();
    let other = start_workflow(&root, "simple-plan-execute-review", Some("other-run")).unwrap();
    assert_eq!(status(&root, "legacy-name").unwrap(), legacy);
    let renamed = rename_run(&root, "legacy-name", "renamed-legacy").unwrap();
    assert_eq!(renamed.run_id, "legacy-name");
    assert_eq!(status(&root, "legacy-name").unwrap(), renamed);
    assert_eq!(status(&root, &other.run_id).unwrap(), other);
}

#[test]
fn corrupted_sibling_is_not_silently_skipped_for_a_valid_target() {
    for corruption in ["missing", "json", "name"] {
        let (_temp, root) = fixture();
        let a = start_workflow(&root, "simple-plan-execute-review", Some("first-run")).unwrap();
        let b = start_workflow(&root, "simple-plan-execute-review", Some("second-run")).unwrap();
        let path = root.join(".unelos/runs").join(&b.run_id).join("state.json");
        match corruption {
            "missing" => std::fs::remove_file(&path).unwrap(),
            "json" => std::fs::write(&path, "{").unwrap(),
            "name" => {
                let mut saved: serde_json::Value =
                    serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
                saved["runId"] = "wrong-directory".into();
                std::fs::write(&path, serde_json::to_vec(&saved).unwrap()).unwrap();
            }
            _ => unreachable!(),
        }
        assert!(list_runs(&root).is_err(), "accepted {corruption}");
        assert!(
            status(&root, &a.run_id).is_err(),
            "selected past {corruption}"
        );
    }
}
