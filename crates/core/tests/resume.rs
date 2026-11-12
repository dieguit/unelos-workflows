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
fn fresh_entry_delivers_descriptions_and_zero_based_opt_out_instructions() {
    let (_temp, root) = fixture();
    std::fs::write(
        root.join(".unelos/workflows/aaa.yaml"),
        "name: aaa\ndescription: A short workflow\nstart: end\nnodes: {end: {type: end}}\n",
    )
    .unwrap();
    std::fs::write(
        root.join(".unelos/workflows/bbb.yaml"),
        "name: bbb\nstart: end\nnodes: {end: {type: end}}\n",
    )
    .unwrap();
    let package = serde_json::to_value(execution_new(&root).unwrap()).unwrap();
    assert_eq!(package["workflows"][0], "aaa");
    assert_eq!(package["workflows"][1], "bbb");
    assert_eq!(package["workflowDescriptions"]["aaa"], "A short workflow");
    assert!(package["workflowDescriptions"].get("bbb").is_none());
    let instructions = package["controllerInstructions"].as_str().unwrap();
    assert!(instructions.contains("0. Continue without Unelos"));
    assert!(instructions.contains("Accept a number, a name"));
    assert!(instructions.contains("requestedAction"));
    assert!(!root.join(".unelos/runs").exists());
}

#[test]
fn fresh_entry_never_selects_history_and_resume_is_read_only() {
    let (_temp, root) = fixture();
    let a = execution_start(&root, "simple-plan-execute-review", Some("first-run")).unwrap();
    let b = execution_start(&root, "simple-plan-execute-review", Some("second-run")).unwrap();
    let a_id = a.run_id.as_deref().unwrap();
    let b_id = b.run_id.as_deref().unwrap();
    let ledger = root.join(".unelos/runs").join(a_id).join("ledger.md");
    std::fs::write(
        &ledger,
        "## Brief\nChange A\n\n## Progress\nInterview pending\n",
    )
    .unwrap();
    let state_a = root.join(".unelos/runs").join(a_id).join("state.json");
    let before = std::fs::read(&state_a).unwrap();
    let fresh = execution_new(&root).unwrap();
    assert!(fresh.run_id.is_none());
    assert!(fresh.packet.is_none());
    assert!(fresh.resume_context.is_none());
    assert_eq!(serde_json::to_value(&fresh).unwrap()["entryMode"], "new");
    let catalog = execution_runs(&root).unwrap();
    assert_eq!(catalog.runs.len(), 2);
    assert_eq!(catalog.controller_instructions, format!("{}\n{}", include_str!("../src/instructions/controller.md"), include_str!("../src/instructions/controller-resume.md")));
    let resumed = execution_resume(&root, a_id).unwrap();
    assert_eq!(resumed.controller_instructions, format!("{}\n{}\n{}", include_str!("../src/instructions/controller.md"), include_str!("../src/instructions/controller-main-task.md"), include_str!("../src/instructions/controller-resume.md")));
    assert_eq!(resumed.run_id.as_deref(), Some(a_id));
    assert!(resumed.resume_context.as_ref().unwrap().ledger.exists);
    assert_eq!(
        serde_json::to_value(&resumed).unwrap()["entryMode"],
        "resume"
    );
    assert_eq!(std::fs::read(state_a).unwrap(), before);
    assert_eq!(
        execution_next(&root, b_id).unwrap().run_id.as_deref(),
        Some(b_id)
    );
}

#[test]
fn finished_run_is_addressable_but_not_resumable() {
    let (_temp, root) = fixture();
    let path = root.join(".unelos/workflows/simple-plan-execute-review.yaml");
    let definition = std::fs::read_to_string(&path).unwrap();
    std::fs::write(
        path,
        definition.replace("start: interview", "start: finish"),
    )
    .unwrap();
    let finished = execution_start(&root, "simple-plan-execute-review", None).unwrap();
    assert_eq!(finished.status, "finished");
    assert!(execution_runs(&root).unwrap().runs.is_empty());
    assert_eq!(
        execution_resume(&root, finished.run_id.as_deref().unwrap())
            .unwrap_err()
            .code(),
        "run_finished"
    );
    let fresh = execution_new(&root).unwrap();
    assert!(fresh.packet.is_none());
    assert_eq!(fresh.workflows, vec!["simple-plan-execute-review"]);
}

#[test]
fn fresh_entry_does_not_read_corrupt_run_storage_or_write_runtime() {
    let (_temp, root) = fixture();
    let run = root.join(".unelos/runs/broken");
    std::fs::create_dir_all(&run).unwrap();
    std::fs::write(run.join("state.json"), "{").unwrap();
    let new = execution_new(&root).unwrap();
    assert_eq!(new.workflows, vec!["simple-plan-execute-review"]);
    assert!(!root.join(".unelos/runtime.lock").exists());
}

fn finish_main(root: &Utf8Path, state: &RunState) -> RunState {
    let CurrentWork::Task(record) = &state.current else {
        panic!("expected main task")
    };
    for output in &record.outputs {
        std::fs::write(&output.path, format!("Report for {}", record.node_id)).unwrap();
    }
    complete_node(root, &state.run_id, &record.node_id, None).unwrap()
}

#[test]
fn empty_output_is_distinct_from_missing_and_missing_input_blocks_dispatch() {
    let (_temp, root) = fixture();
    let state = start_workflow(&root, "simple-plan-execute-review", None).unwrap();
    let CurrentWork::Task(record) = &state.current else {
        panic!()
    };
    let path = &record.outputs[0].path;
    std::fs::write(path, "").unwrap();
    let resumed = execution_resume(&root, &state.run_id).unwrap();
    assert!(resumed.resume_context.unwrap().reports[0].exists);
    std::fs::remove_file(path).unwrap();
    assert!(!execution_resume(&root, &state.run_id)
        .unwrap()
        .resume_context
        .unwrap()
        .reports[0]
        .exists);
    let plan = finish_main(&root, &state);
    std::fs::remove_file(path).unwrap();
    let before = std::fs::read(
        root.join(".unelos/runs")
            .join(&plan.run_id)
            .join("state.json"),
    )
    .unwrap();
    let resumed = execution_resume(&root, &plan.run_id).unwrap();
    assert_eq!(resumed.status, "needs-context");
    assert!(resumed.worker_prompt.is_none());
    assert!(resumed.expected.is_some());
    assert!(resumed
        .resume_context
        .unwrap()
        .issues
        .iter()
        .any(|issue| issue.contains(path.as_str())));
    assert_eq!(
        std::fs::read(
            root.join(".unelos/runs")
                .join(&plan.run_id)
                .join("state.json")
        )
        .unwrap(),
        before
    );
}

#[test]
fn resume_rejects_oversize_or_invalid_utf8_without_mutation() {
    for (bytes, code) in [
        (vec![b'x'; 8 * 1024 * 1024 + 1], "resume_context_too_large"),
        (vec![255], "invalid_resume_context"),
    ] {
        let (_temp, root) = fixture();
        let state = start_workflow(&root, "simple-plan-execute-review", None).unwrap();
        let CurrentWork::Task(record) = &state.current else {
            panic!()
        };
        std::fs::write(&record.outputs[0].path, &bytes).unwrap();
        let path = root
            .join(".unelos/runs")
            .join(&state.run_id)
            .join("state.json");
        let before = std::fs::read(&path).unwrap();
        let resumed = execution_resume(&root, &state.run_id).unwrap();
        assert!(resumed.resume_context.unwrap().reports[0].exists);
        let _ = code;
        assert_eq!(std::fs::read(path).unwrap(), before);
    }
}

#[cfg(unix)]
#[test]
fn resume_rejects_external_report_symlink_without_touching_target() {
    let (_temp, root) = fixture();
    let state = start_workflow(&root, "simple-plan-execute-review", None).unwrap();
    let CurrentWork::Task(record) = &state.current else {
        panic!()
    };
    let external = root.join("external.md");
    std::fs::write(&external, "untouched").unwrap();
    std::os::unix::fs::symlink(&external, &record.outputs[0].path).unwrap();
    assert!(execution_resume(&root, &state.run_id).is_err());
    assert_eq!(std::fs::read_to_string(external).unwrap(), "untouched");
}

#[test]
fn resume_gate_preserves_activation_and_never_approves() {
    let (_temp, root) = fixture();
    let state = start_workflow(&root, "simple-plan-execute-review", None).unwrap();
    let state = finish_main(&root, &state);
    let gate = finish_main(&root, &state);
    let path = root
        .join(".unelos/runs")
        .join(&gate.run_id)
        .join("state.json");
    let before = std::fs::read(&path).unwrap();
    let resumed = execution_resume(&root, &gate.run_id).unwrap();
    assert_eq!(resumed.status, "waiting");
    assert_eq!(
        resumed.expected,
        execution_next(&root, &gate.run_id).unwrap().expected
    );
    assert!(resumed.worker_prompt.is_none());
    assert_eq!(std::fs::read(path).unwrap(), before);
}

#[test]
fn delegated_resume_keeps_prompt_without_brief_or_ledger() {
    let (_temp, root) = fixture();
    let state = start_workflow(&root, "simple-plan-execute-review", None).unwrap();
    let state = finish_main(&root, &state);
    let state = finish_main(&root, &state);
    let ledger = root
        .join(".unelos/runs")
        .join(&state.run_id)
        .join("ledger.md");
    std::fs::write(&ledger, "## Brief\nMake the requested change\n").unwrap();
    let state = approve_node(&root, &state.run_id, "approve-plan").unwrap();
    let CurrentWork::Task(record) = &state.current else {
        panic!()
    };
    std::fs::write(&record.outputs[0].path, "Previous implementation progress").unwrap();
    let current = execution_next(&root, &state.run_id).unwrap();
    let resumed = execution_resume(&root, &state.run_id).unwrap();
    assert_eq!(resumed.worker_prompt, current.worker_prompt);
    assert_eq!(resumed.expected, current.expected);
    assert!(serde_json::to_string(&resumed.resume_context)
        .unwrap()
        .contains(record.outputs[0].path.as_str()));
    std::fs::write(&ledger, "No Brief").unwrap();
    let without_brief = execution_resume(&root, &state.run_id).unwrap();
    assert_eq!(without_brief.status, "running");
    assert!(without_brief.worker_prompt.is_some());
    assert_eq!(without_brief.expected, current.expected);
    std::fs::remove_file(ledger).unwrap();
    let missing = execution_resume(&root, &state.run_id).unwrap();
    assert_eq!(missing.status, "running");
    assert_eq!(missing.expected, current.expected);
    assert!(missing.worker_prompt.is_some());
    assert!(missing.resume_context.unwrap().issues.is_empty());
    assert_eq!(status(&root, &state.run_id).unwrap(), state);
}
