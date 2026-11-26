use assert_fs::TempDir;
use camino::{Utf8Path, Utf8PathBuf};
use unelos_core::*;

const WORKFLOW: &str = "council";

fn fixture() -> (TempDir, Utf8PathBuf) {
    static CONFIG: std::sync::OnceLock<TempDir> = std::sync::OnceLock::new();
    CONFIG.get_or_init(|| {
        let config = TempDir::new().unwrap();
        std::env::set_var("UNELOS_CONFIG_DIR", config.path());
        config
    });
    let temp = TempDir::new().unwrap();
    let root = Utf8Path::from_path(temp.path()).unwrap().to_owned();
    install_workflow(
        &root,
        Some(&Utf8Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/council")),
        WORKFLOW,
    )
    .unwrap();
    (temp, root)
}

#[test]
fn example_installs_with_three_isolated_workers() {
    let (_temp, root) = fixture();
    let project = load_project(&root, WORKFLOW).unwrap();
    validate_project(&project).unwrap();
    let value = serde_json::to_value(inspect_node(&project, "council").unwrap()).unwrap();
    let workers = value["workers"].as_array().unwrap();
    assert_eq!(workers.len(), 3);
    assert_eq!(workers[0]["agent"], "councillor-alpha");
    assert_eq!(workers[1]["agent"], "councillor-beta");
    assert_eq!(workers[2]["agent"], "councillor-gamma");
    for worker in workers {
        assert!(worker["skills"]
            .to_string()
            .contains("council-deliberation"));
    }
}

/// Builds the parallel packet directly (no Brief/ledger involved) by starting at the parallel Node.
fn parallel_packet(root: &Utf8Path) -> WorkPacket {
    let mut project = load_project(root, WORKFLOW).unwrap();
    project.workflow.start = "council".into();
    if let NodeDefinition::Parallel(node) = project.workflow.nodes.get_mut("council").unwrap() {
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
                missing_inputs: vec![],
                run_id: state.run_id.clone(),
                task_id: record.execution_id.clone(),
                node_id: record.node_id.clone(),
                request: state.request.clone(),
                inputs: record.inputs.clone(),
                outputs: record.outputs.clone(),
                feedback: record.feedback.clone(),
            },
            worker_reports: record.worker_reports.clone(),
            ledger_path: root
                .join(".unelos")
                .join("runs")
                .join("parallel-check")
                .join("ledger.md"),
        },
    )
    .unwrap()
}

fn worker_json(expected: &ExecutionIdentity, key: &str, status: &str) -> serde_json::Value {
    let mut value = serde_json::json!({"taskId": serde_json::to_value(expected).unwrap()["taskId"], "outcome": null});
    value["workerKey"] = key.into();
    value["status"] = status.into();
    value["summary"] = "Researched".into();
    value["changedPaths"] = serde_json::json!([]);
    value["verification"] = serde_json::json!(["Read code"]);
    match status {
        "FINISHED" => value["outcome"] = serde_json::Value::Null,
        "NEEDS_CONTEXT" => {
            value["question"] = "Which API?".into();
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
fn parallel_first_start_renders_worker_prompts_without_brief() {
    let (_temp, root) = fixture();
    let path = root.join(".unelos/workflows/council.yaml");
    let yaml = std::fs::read_to_string(&path)
        .unwrap()
        .replace("\r\n", "\n")
        .replace("start: assignment", "start: council")
        .replace("    inputs: [question.md]\n", "");
    std::fs::write(path, yaml).unwrap();
    let package = execution_start(&root, WORKFLOW, None, "").unwrap();
    assert_eq!(package.worker_prompts.len(), 3);
    assert!(package
        .worker_prompts
        .iter()
        .all(|p| !p.prompt.contains("\n\nBrief:")));
    let serialized = serde_json::to_value(&package).unwrap();
    assert!(serialized["packet"].get("controllerInstructions").is_none());
    assert!(!serialized.to_string().contains("complete NODE"));
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
    let without_brief = render_execution_package(Some(packet.clone()), vec![], None).unwrap();
    assert_eq!(without_brief.worker_prompts.len(), 3);
    assert!(without_brief
        .worker_prompts
        .iter()
        .all(|p| !p.prompt.contains("\n\nBrief:")));
    let package =
        render_execution_package(Some(packet), vec![], Some("Persistence question")).unwrap();
    assert!(matches!(
        package.expected,
        Some(ExecutionIdentity::Parallel { .. })
    ));
    assert!(package.worker_prompt.is_none());
    assert_eq!(package.worker_prompts.len(), 3);
    assert_eq!(package.worker_prompts[0].key, "worker-1");
    assert_eq!(package.worker_prompts[0].agent, "councillor-alpha");
    assert_eq!(package.worker_prompts[1].key, "worker-2");
    assert_eq!(package.worker_prompts[1].agent, "councillor-beta");
    assert_eq!(package.worker_prompts[2].key, "worker-3");
    assert_eq!(package.worker_prompts[2].agent, "councillor-gamma");
    for (index, prompt) in package.worker_prompts.iter().enumerate() {
        assert!(prompt.prompt.contains(&reports[index]));
        for (other, report) in reports.iter().enumerate() {
            if other != index {
                assert!(!prompt.prompt.contains(report));
            }
        }
        assert!(!prompt.prompt.contains(&aggregate));
        assert!(!prompt.prompt.contains("ledgerPath"));
        assert!(!prompt.prompt.contains("controllerInstructions"));
        assert!(prompt.prompt.contains("Write only your report"));
        assert!(prompt.prompt.contains("Persistence question"));
    }
    for prompt in &package.worker_prompts {
        assert!(prompt.prompt.contains("Answer question.md independently"));
        assert!(prompt.prompt.contains("# Council deliberation"));
    }
    for (index, focus) in [
        "Focus: correctness",
        "Focus: architecture",
        "Focus: simplicity",
    ]
    .iter()
    .enumerate()
    {
        for (other, prompt) in package.worker_prompts.iter().enumerate() {
            assert_eq!(prompt.prompt.contains(focus), index == other);
        }
    }
}

#[test]
fn parallel_result_parser_accepts_finished_subsets() {
    let expected = ExecutionIdentity::Parallel {
        run_id: "r-1".into(),
        node_id: "fan-out".into(),
        task_id: "task-4".into(),
    };
    let keys = vec![
        "worker-1".to_string(),
        "worker-2".to_string(),
        "worker-3".to_string(),
    ];
    let one = worker_json(&expected, "worker-1", "FINISHED");
    let two = worker_json(&expected, "worker-2", "FINISHED");
    let three = worker_json(&expected, "worker-3", "FINISHED");
    let results = parse_parallel_results(
        &frames(&[two.clone(), three, one.clone()]),
        &expected,
        &keys,
    )
    .unwrap();
    assert_eq!(results[0].worker_key.as_deref(), Some("worker-1"));
    assert_eq!(results.len(), 3);
    assert_eq!(
        parse_parallel_results(&frames(std::slice::from_ref(&one)), &expected, &keys)
            .unwrap()
            .len(),
        1
    );
    assert!(
        parse_parallel_results(&frames(&[one.clone(), one.clone()]), &expected, &keys)
            .unwrap_err()
            .to_string()
            .contains("two result frames for workerKey \"worker-1\"; send only the final one")
    );
    let mut outcome = two.clone();
    outcome["outcome"] = "passed".into();
    let mut serial = two.clone();
    serial["kind"] = "task".into();
    let mut stale = two.clone();
    stale["taskId"] = "task-3".into();
    for text in [
        frames(&[one.clone(), one.clone()]),
        frames(&[one.clone(), worker_json(&expected, "worker-4", "FINISHED")]),
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

fn task_frame(package: &ExecutionPackage) -> CompletionControl {
    let expected = package.expected.clone().unwrap();
    let mut value =
        serde_json::json!({"taskId": serde_json::to_value(&expected).unwrap()["taskId"]});
    for (k, v) in [("status", "FINISHED"), ("summary", "Done")] {
        value[k] = v.into();
    }
    value["changedPaths"] = serde_json::json!([]);
    value["verification"] = serde_json::json!(["Read"]);
    value["outcome"] = serde_json::Value::Null;
    CompletionControl {
        summary: Some("Done".into()),
        expected,
        result_text: package
            .worker_prompt
            .as_ref()
            .map(|_| format!("<unelos-worker-result>{value}</unelos-worker-result>")),
        outcome: None,
        reason: None,
    }
}

fn parent_control(package: &ExecutionPackage, statuses: &[(&str, &str)]) -> CompletionControl {
    let expected = package.expected.clone().unwrap();
    let values: Vec<_> = statuses
        .iter()
        .map(|(key, status)| worker_json(&expected, key, status))
        .collect();
    let text = frames(&values);
    CompletionControl {
        summary: None,
        expected,
        result_text: Some(text),
        outcome: None,
        reason: None,
    }
}

fn complete(
    root: &Utf8Path,
    control: &CompletionControl,
) -> Result<ExecutionPackage, ExecutionError> {
    execution_mutate(root, "r-000001", ExecutionOperation::Complete, control)
}

/// Starts the run and completes `assignment`, returning the parallel package.
fn parallel(root: &Utf8Path) -> ExecutionPackage {
    let scope = execution_start(root, WORKFLOW, Some("research-check"), "").unwrap();
    write_outputs(&scope);
    let ledger = packet_json(&scope)["ledgerPath"]
        .as_str()
        .unwrap()
        .to_owned();
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
    for worker in packet["workers"].as_array().unwrap() {
        for field in [
            "taskInstructions",
            "agentInstructions",
            "skills",
            "completionInstructions",
        ] {
            assert!(worker.get(field).is_none(), "{field}");
        }
    }
    assert_eq!(packet["kind"], "parallel");
    assert_eq!(package.worker_prompts.len(), 3);
    let run = root.join(".unelos/runs/r-000001/council");
    assert_eq!(
        Utf8Path::new(packet["outputs"][0]["path"].as_str().unwrap()),
        run.join("council-responses.md")
    );
    assert_eq!(
        Utf8Path::new(packet["workers"][0]["outputs"][0]["path"].as_str().unwrap()),
        run.join("councillor-alpha/result.md")
    );
    assert_eq!(
        Utf8Path::new(packet["workers"][1]["outputs"][0]["path"].as_str().unwrap()),
        run.join("councillor-beta/result.md")
    );
    assert_eq!(
        Utf8Path::new(packet["workers"][2]["outputs"][0]["path"].as_str().unwrap()),
        run.join("councillor-gamma/result.md")
    );
}

#[test]
fn partial_frames_never_complete_the_parent() {
    let (_temp, root) = fixture();
    let package = parallel(&root);
    write_outputs(&package);
    write_worker_reports(&package);
    let first = parent_control(&package, &[("worker-1", "FINISHED")]);
    let remaining = complete(&root, &first).unwrap();
    assert_eq!(remaining.status, "running");
    assert_eq!(remaining.worker_prompts.len(), 2);
    assert_eq!(remaining.worker_prompts[0].key, "worker-2");
    let resumed = unelos_core::execution_resume(&root, "r-000001").unwrap();
    assert_eq!(resumed.worker_prompts.len(), 2);
    assert_eq!(resumed.expected, package.expected);
    for attempt in 1..=3 {
        let error = complete(&root, &first).unwrap_err();
        assert!(error.to_string().contains("worker-1"));
        assert_eq!(
            error.to_string().contains("Stop and tell the user."),
            attempt == 3
        );
    }
    let next = complete(
        &root,
        &parent_control(
            &remaining,
            &[("worker-2", "FINISHED"), ("worker-3", "FINISHED")],
        ),
    )
    .unwrap();
    assert_eq!(packet_json(&next)["nodeId"], "synthesis");
}

#[test]
fn missing_aggregate_or_worker_report_blocks_completion() {
    let (_temp, root) = fixture();
    let package = parallel(&root);
    let all = parent_control(
        &package,
        &[
            ("worker-1", "FINISHED"),
            ("worker-2", "FINISHED"),
            ("worker-3", "FINISHED"),
        ],
    );
    write_worker_reports(&package);
    assert_eq!(complete(&root, &all).unwrap_err().code(), "missing_output");
    write_outputs(&package);
    let frontend = packet_json(&package)["workers"][1]["outputs"][0]["path"]
        .as_str()
        .unwrap()
        .to_owned();
    std::fs::remove_file(&frontend).unwrap();
    assert_eq!(
        complete(&root, &parent_control(&package, &[]))
            .unwrap_err()
            .code(),
        "missing_output"
    );
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
            &[
                ("worker-2", "FINISHED"),
                ("worker-1", "FINISHED"),
                ("worker-3", "FINISHED"),
            ],
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
        &CompletionControl {
            summary: None,
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
    assert_eq!(
        again["workers"][2]["outputs"],
        packet_json(&first)["workers"][2]["outputs"]
    );
    assert!(rejected.worker_prompts[0]
        .prompt
        .contains("Cover error handling"));
    let stale = parent_control(
        &first,
        &[
            ("worker-1", "FINISHED"),
            ("worker-2", "FINISHED"),
            ("worker-3", "FINISHED"),
        ],
    );
    assert_eq!(
        complete(&root, &stale).unwrap_err().code(),
        "stale_execution"
    );
    let report = again["outputs"][0]["path"].as_str().unwrap();
    std::fs::write(report, "Addressed error handling feedback").unwrap();
    let finished = complete(
        &root,
        &parent_control(
            &rejected,
            &[
                ("worker-1", "FINISHED"),
                ("worker-2", "FINISHED"),
                ("worker-3", "FINISHED"),
            ],
        ),
    )
    .unwrap();
    assert_eq!(packet_json(&finished)["nodeId"], "synthesis");
}

#[test]
fn parallel_package_carries_concurrent_dispatch_policy() {
    let (_temp, root) = fixture();
    let policy = parallel(&root).controller_instructions;
    for phrase in [
        "before waiting for any",
        "Never run them yourself",
        "read every report",
        "only their final messages",
    ] {
        assert!(policy.contains(phrase), "{phrase}");
    }
}
