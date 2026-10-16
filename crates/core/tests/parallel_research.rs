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
