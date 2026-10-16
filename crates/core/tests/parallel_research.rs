use assert_fs::TempDir;
use camino::{Utf8Path, Utf8PathBuf};
use unelos_core::*;

const WORKFLOW: &str = "parallel-research";

fn fixture() -> (TempDir, Utf8PathBuf) {
    let temp = TempDir::new().unwrap();
    let root = Utf8Path::from_path(temp.path()).unwrap().to_owned();
    install_workflow(
        &root,
        Some(&Utf8Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/parallel-research")),
        WORKFLOW,
    )
    .unwrap();
    (temp, root)
}

#[test]
fn example_installs_with_two_isolated_workers() {
    let (_temp, root) = fixture();
    let project = load_project(&root, WORKFLOW).unwrap();
    validate_project(&project).unwrap();
    let value = serde_json::to_value(inspect_node(&project, "parallel-research").unwrap()).unwrap();
    let workers = value["workers"].as_array().unwrap();
    assert_eq!(workers.len(), 2);
    assert_eq!(workers[0]["agent"], "backend-researcher");
    assert_eq!(workers[1]["agent"], "frontend-researcher");
    let skills = |i: usize| workers[i]["skills"].to_string();
    assert!(skills(0).contains("backend-research") && !skills(0).contains("frontend-research"));
    assert!(skills(1).contains("frontend-research") && !skills(1).contains("backend-research"));
}

/// Builds the parallel packet directly (no Brief/ledger involved) by starting at the parallel Node.
fn parallel_packet(root: &Utf8Path) -> WorkPacket {
    let mut project = load_project(root, WORKFLOW).unwrap();
    project.workflow.start = "parallel-research".into();
    if let NodeDefinition::Parallel(node) =
        project.workflow.nodes.get_mut("parallel-research").unwrap()
    {
        node.inputs.clear();
    }
    let state = start(&project, "parallel-check", "").unwrap();
    let CurrentWork::Parallel(record) = &state.current else {
        panic!("expected parallel")
    };
    build_work_packet(
        &project,
        WorkRuntimeInput::Parallel {
            input: TaskRuntimeInput {
                run_id: state.run_id.clone(),
                task_id: record.execution_id.clone(),
                node_id: record.node_id.clone(),
                request: state.request.clone(),
                inputs: record.inputs.clone(),
                outputs: record.outputs.clone(),
                feedback: record.feedback.clone(),
            },
            worker_reports: record.worker_reports.clone(),
            ledger_path: root.join(".unelos/runs/parallel-check/ledger.md"),
        },
    )
    .unwrap()
}

/// The `scope` Task packet (main execution), used to prove Tasks reject workerKey.
fn scope_packet(root: &Utf8Path) -> WorkPacket {
    let project = load_project(root, WORKFLOW).unwrap();
    let state = start(&project, "scope-check", "").unwrap();
    let CurrentWork::Task(record) = &state.current else {
        panic!("expected Task")
    };
    build_work_packet(
        &project,
        WorkRuntimeInput::Task {
            input: TaskRuntimeInput {
                run_id: state.run_id.clone(),
                task_id: record.execution_id.clone(),
                node_id: record.node_id.clone(),
                request: state.request.clone(),
                inputs: record.inputs.clone(),
                outputs: record.outputs.clone(),
                feedback: record.feedback.clone(),
            },
            ledger_path: root.join(".unelos/runs/scope-check/ledger.md"),
        },
    )
    .unwrap()
}

fn worker_json(expected: &ExecutionIdentity, key: &str, status: &str) -> serde_json::Value {
    let mut value = serde_json::to_value(expected).unwrap();
    value["kind"] = "parallel-worker".into();
    value["workerKey"] = key.into();
    value["status"] = status.into();
    value["summary"] = "Researched".into();
    value["changedPaths"] = serde_json::json!([]);
    value["verification"] = serde_json::json!(["Read code"]);
    value["progress"] = serde_json::json!([]);
    match status {
        "FINISHED" => value["outcome"] = serde_json::Value::Null,
        "NEEDS_CONTEXT" => {
            value["question"] = "Which API?".into();
            value["dependsOn"] = "Scope".into();
        }
        _ => value["reason"] = "Cannot read".into(),
    }
    value
}

fn frames(values: &[serde_json::Value]) -> String {
    values
        .iter()
        .map(|v| format!("<unelos-worker-result>{v}</unelos-worker-result>"))
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn parallel_package_has_one_isolated_prompt_per_worker() {
    let (_temp, root) = fixture();
    let packet = parallel_packet(&root);
    let WorkPacket::Parallel(parallel) = &packet else {
        panic!()
    };
    let reports: Vec<String> = parallel
        .workers
        .iter()
        .map(|w| w.outputs[0].path.to_string())
        .collect();
    let aggregate = parallel.outputs[0].path.to_string();
    let error = render_execution_package(Some(packet.clone()), vec![], None, None).unwrap_err();
    assert_eq!(error.code(), "missing_brief");
    let package =
        render_execution_package(Some(packet), vec![], Some("Persistence question"), None).unwrap();
    assert!(matches!(
        package.expected,
        Some(ExecutionIdentity::Parallel { .. })
    ));
    assert!(package.worker_prompt.is_none());
    assert_eq!(package.worker_prompts.len(), 2);
    assert_eq!(package.worker_prompts[0].key, "worker-1");
    assert_eq!(package.worker_prompts[0].agent, "backend-researcher");
    assert_eq!(package.worker_prompts[1].key, "worker-2");
    assert_eq!(package.worker_prompts[1].agent, "frontend-researcher");
    for (index, prompt) in package.worker_prompts.iter().enumerate() {
        assert!(prompt.prompt.contains(&reports[index]));
        assert!(!prompt.prompt.contains(&reports[1 - index]));
        assert!(!prompt.prompt.contains(&aggregate));
        assert!(!prompt.prompt.contains("ledgerPath"));
        assert!(!prompt.prompt.contains("controllerInstructions"));
        assert!(prompt.prompt.contains("parallel-worker"));
        assert!(prompt.prompt.contains("Persistence question"));
    }
    assert!(package.worker_prompts[0]
        .prompt
        .contains("Research the backend side"));
    assert!(!package.worker_prompts[0]
        .prompt
        .contains("Research the frontend side"));
    assert!(package.worker_prompts[0]
        .prompt
        .contains("# Backend research"));
    assert!(!package.worker_prompts[0]
        .prompt
        .contains("# Frontend research"));
}

#[test]
fn parallel_continuation_selects_one_worker() {
    let (_temp, root) = fixture();
    let packet = parallel_packet(&root);
    let expected = ExecutionIdentity::from_packet(&packet).unwrap();
    let control = |key: Option<&str>| ContinuationControl {
        expected: expected.clone(),
        worker_key: key.map(Into::into),
        answer: Some("Use the run list view".into()),
        progress: vec!["Read routes".into()],
    };
    let package = render_execution_package(
        Some(packet.clone()),
        vec![],
        Some("Persistence question"),
        Some(&control(Some("worker-2"))),
    )
    .unwrap();
    assert_eq!(package.worker_prompts.len(), 1);
    assert_eq!(package.worker_prompts[0].key, "worker-2");
    assert!(package.worker_prompts[0]
        .prompt
        .contains("Use the run list view"));
    for key in [None, Some("worker-9")] {
        let error = render_execution_package(
            Some(packet.clone()),
            vec![],
            Some("Persistence question"),
            Some(&control(key)),
        )
        .unwrap_err();
        assert_eq!(error.code(), "invalid_control");
    }
    let task = scope_packet(&root);
    let with_key = ContinuationControl {
        expected: ExecutionIdentity::from_packet(&task).unwrap(),
        worker_key: Some("worker-1".into()),
        answer: None,
        progress: vec![],
    };
    let error = render_execution_package(
        Some(task),
        vec![],
        Some("Persistence question"),
        Some(&with_key),
    )
    .unwrap_err();
    assert_eq!(error.code(), "invalid_control");
}

#[test]
fn parallel_result_parser_requires_one_finished_frame_per_key() {
    let expected = ExecutionIdentity::Parallel {
        run_id: "r-1".into(),
        node_id: "fan-out".into(),
        task_id: "task-4".into(),
    };
    let keys = vec!["worker-1".to_string(), "worker-2".to_string()];
    let one = worker_json(&expected, "worker-1", "FINISHED");
    let two = worker_json(&expected, "worker-2", "FINISHED");
    let results =
        parse_parallel_results(&frames(&[two.clone(), one.clone()]), &expected, &keys).unwrap();
    assert_eq!(results[0].worker_key.as_deref(), Some("worker-1"));
    assert_eq!(results.len(), 2);
    let mut outcome = two.clone();
    outcome["outcome"] = "passed".into();
    let mut serial = two.clone();
    serial["kind"] = "task".into();
    let mut stale = two.clone();
    stale["taskId"] = "task-3".into();
    for text in [
        frames(std::slice::from_ref(&one)),
        frames(&[one.clone(), one.clone()]),
        frames(&[one.clone(), worker_json(&expected, "worker-3", "FINISHED")]),
        frames(&[
            one.clone(),
            worker_json(&expected, "worker-2", "NEEDS_CONTEXT"),
        ]),
        frames(&[one.clone(), worker_json(&expected, "worker-2", "BLOCKED")]),
        frames(&[one.clone(), outcome]),
        frames(&[one.clone(), serial]),
        frames(&[one.clone(), stale]),
        "<unelos-worker-result>{</unelos-worker-result>".into(),
    ] {
        let error = parse_parallel_results(&text, &expected, &keys).unwrap_err();
        assert_eq!(error.code(), "invalid_worker_result", "{text}");
    }
    // A serial parser never accepts a parallel-worker frame.
    assert!(parse_worker_result(&frames(&[one]), &expected).is_err());
}

fn packet_json(package: &ExecutionPackage) -> serde_json::Value {
    serde_json::to_value(&package.packet).unwrap()
}

fn write_outputs(package: &ExecutionPackage) {
    for output in packet_json(package)["outputs"].as_array().unwrap() {
        std::fs::write(output["path"].as_str().unwrap(), "Useful section\n").unwrap();
    }
}

fn task_frame(package: &ExecutionPackage) -> ExecutionControl {
    let expected = package.expected.clone().unwrap();
    let mut value = serde_json::to_value(&expected).unwrap();
    for (k, v) in [("status", "FINISHED"), ("summary", "Done")] {
        value[k] = v.into();
    }
    value["changedPaths"] = serde_json::json!([]);
    value["verification"] = serde_json::json!(["Read"]);
    value["progress"] = serde_json::json!([]);
    value["outcome"] = serde_json::Value::Null;
    ExecutionControl {
        expected,
        result_text: Some(format!(
            "<unelos-worker-result>{value}</unelos-worker-result>"
        )),
        outcome: None,
        reason: None,
    }
}

fn parent_control(package: &ExecutionPackage, statuses: &[(&str, &str)]) -> ExecutionControl {
    let expected = package.expected.clone().unwrap();
    let values: Vec<_> = statuses
        .iter()
        .map(|(key, status)| worker_json(&expected, key, status))
        .collect();
    let text = frames(&values);
    ExecutionControl {
        expected,
        result_text: Some(text),
        outcome: None,
        reason: None,
    }
}

fn complete(
    root: &Utf8Path,
    control: &ExecutionControl,
) -> Result<ExecutionPackage, ExecutionError> {
    execution_mutate(root, "r-000001", ExecutionOperation::Complete, control)
}

fn state_bytes(root: &Utf8Path) -> Vec<u8> {
    std::fs::read(root.join(".unelos/runs/research-check/state.json")).unwrap()
}

/// Starts the run and completes `scope`, returning the parallel package.
fn parallel(root: &Utf8Path) -> ExecutionPackage {
    let scope = execution_start(root, WORKFLOW, Some("research-check")).unwrap();
    write_outputs(&scope);
    let ledger = packet_json(&scope)["ledgerPath"]
        .as_str()
        .unwrap()
        .to_owned();
    assert_eq!(
        complete(root, &task_frame(&scope)).unwrap_err().code(),
        "missing_brief"
    );
    std::fs::write(&ledger, "## Brief\nHow is a run persisted?\n").unwrap();
    complete(root, &task_frame(&scope)).unwrap()
}

fn write_worker_reports(package: &ExecutionPackage) {
    for worker in packet_json(package)["workers"].as_array().unwrap() {
        std::fs::write(worker["outputs"][0]["path"].as_str().unwrap(), "Findings\n").unwrap();
    }
}

#[test]
fn parallel_package_binds_flat_readable_paths() {
    let (_temp, root) = fixture();
    let package = parallel(&root);
    let packet = packet_json(&package);
    assert_eq!(packet["kind"], "parallel");
    assert_eq!(package.worker_prompts.len(), 2);
    let run = root.join(".unelos/runs/research-check/parallel-research");
    assert_eq!(
        packet["outputs"][0]["path"],
        run.join("research-summary.md").as_str()
    );
    assert_eq!(
        packet["workers"][0]["outputs"][0]["path"],
        run.join("backend-researcher/result.md").as_str()
    );
    assert_eq!(
        packet["workers"][1]["outputs"][0]["path"],
        run.join("frontend-researcher/result.md").as_str()
    );
}

#[test]
fn worker_question_continues_without_new_parent_identity() {
    let (_temp, root) = fixture();
    let package = parallel(&root);
    let control = ContinuationControl {
        expected: package.expected.clone().unwrap(),
        worker_key: Some("worker-2".into()),
        answer: Some("Focus on the run list view".into()),
        progress: vec!["Read src/App.tsx".into()],
    };
    let continued = execution_continue(&root, "r-000001", &control).unwrap();
    assert_eq!(continued.expected, package.expected);
    assert_eq!(continued.worker_prompts.len(), 1);
    assert!(continued.worker_prompts[0]
        .prompt
        .contains("Focus on the run list view"));
    assert_eq!(
        execution_next(&root, "r-000001").unwrap().expected,
        package.expected
    );
}

#[test]
fn partial_frames_never_complete_the_parent() {
    let (_temp, root) = fixture();
    let package = parallel(&root);
    write_outputs(&package);
    write_worker_reports(&package);
    let bytes = state_bytes(&root);
    for statuses in [
        vec![("worker-1", "FINISHED")],
        vec![("worker-1", "FINISHED"), ("worker-2", "NEEDS_CONTEXT")],
        vec![("worker-1", "FINISHED"), ("worker-2", "BLOCKED")],
        vec![("worker-1", "FINISHED"), ("worker-1", "FINISHED")],
    ] {
        let error = complete(&root, &parent_control(&package, &statuses)).unwrap_err();
        assert_eq!(error.code(), "invalid_worker_result", "{statuses:?}");
        assert_eq!(bytes, state_bytes(&root));
    }
    let mut with_outcome = parent_control(
        &package,
        &[("worker-1", "FINISHED"), ("worker-2", "FINISHED")],
    );
    with_outcome.outcome = Some("approved".into());
    assert_eq!(
        complete(&root, &with_outcome).unwrap_err().code(),
        "invalid_outcome"
    );
    assert_eq!(bytes, state_bytes(&root));
}

#[test]
fn missing_aggregate_or_worker_report_blocks_completion() {
    let (_temp, root) = fixture();
    let package = parallel(&root);
    let all = parent_control(
        &package,
        &[("worker-1", "FINISHED"), ("worker-2", "FINISHED")],
    );
    write_worker_reports(&package);
    assert_eq!(complete(&root, &all).unwrap_err().code(), "missing_output");
    write_outputs(&package);
    let frontend = packet_json(&package)["workers"][1]["outputs"][0]["path"]
        .as_str()
        .unwrap()
        .to_owned();
    std::fs::remove_file(&frontend).unwrap();
    assert_eq!(complete(&root, &all).unwrap_err().code(), "missing_output");
}

#[test]
fn gate_rejection_reenters_parallel_with_same_paths_and_new_identity() {
    let (_temp, root) = fixture();
    let first = parallel(&root);
    write_outputs(&first);
    write_worker_reports(&first);
    let synthesis = complete(
        &root,
        &parent_control(
            &first,
            &[("worker-2", "FINISHED"), ("worker-1", "FINISHED")],
        ),
    )
    .unwrap();
    let summary = packet_json(&first)["outputs"][0]["path"].clone();
    let inputs = packet_json(&synthesis)["inputs"].clone();
    assert!(inputs
        .as_array()
        .unwrap()
        .iter()
        .any(|i| i["path"] == summary));
    write_outputs(&synthesis);
    let gate = complete(&root, &task_frame(&synthesis)).unwrap();
    assert_eq!(gate.status, "waiting");
    let rejected = execution_mutate(
        &root,
        "r-000001",
        ExecutionOperation::Reject,
        &ExecutionControl {
            expected: gate.expected.clone().unwrap(),
            result_text: None,
            outcome: None,
            reason: Some("Cover error handling".into()),
        },
    )
    .unwrap();
    let again = packet_json(&rejected);
    assert_eq!(again["kind"], "parallel");
    assert_ne!(rejected.expected, first.expected);
    assert_eq!(again["outputs"], packet_json(&first)["outputs"]);
    assert_eq!(
        again["workers"][0]["outputs"],
        packet_json(&first)["workers"][0]["outputs"]
    );
    assert_eq!(
        again["workers"][1]["outputs"],
        packet_json(&first)["workers"][1]["outputs"]
    );
    assert!(rejected.worker_prompts[0]
        .prompt
        .contains("Cover error handling"));
    let stale = parent_control(
        &first,
        &[("worker-1", "FINISHED"), ("worker-2", "FINISHED")],
    );
    assert_eq!(
        complete(&root, &stale).unwrap_err().code(),
        "stale_execution"
    );
    let finished = complete(
        &root,
        &parent_control(
            &rejected,
            &[("worker-1", "FINISHED"), ("worker-2", "FINISHED")],
        ),
    )
    .unwrap();
    assert_eq!(packet_json(&finished)["nodeId"], "synthesis");
}
