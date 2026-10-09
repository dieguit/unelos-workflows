use assert_fs::{fixture::PathCopy, TempDir};
use camino::{Utf8Path, Utf8PathBuf};
use unelos_core::*;

fn finished_control(package: &ExecutionPackage) -> ExecutionControl {
    let expected = package.expected.clone().unwrap();
    let mut result = serde_json::to_value(&expected).unwrap();
    result["status"] = "FINISHED".into();
    result["summary"] = "Inspected".into();
    result["changedPaths"] = serde_json::json!([]);
    result["verification"] = serde_json::json!(["Read report"]);
    result["progress"] = serde_json::json!([]);
    result["outcome"] = serde_json::Value::Null;
    ExecutionControl {
        expected,
        result_text: Some(format!(
            "<unelos-worker-result>{result}</unelos-worker-result>"
        )),
        outcome: None,
        reason: None,
    }
}

fn write_outputs(package: &ExecutionPackage) {
    let packet = serde_json::to_value(&package.packet).unwrap();
    for output in packet["outputs"].as_array().unwrap() {
        std::fs::write(output["path"].as_str().unwrap(), "Useful report section\n").unwrap();
    }
}

fn state_bytes(root: &Utf8Path) -> Vec<u8> {
    let state = status(root, "r-000001").unwrap();
    std::fs::read(
        root.join(".unelos/runs")
            .join(&state.name)
            .join("state.json"),
    )
    .unwrap()
}

fn advance(root: &Utf8Path, package: &ExecutionPackage) -> ExecutionPackage {
    write_outputs(package);
    execution_mutate(
        root,
        "r-000001",
        ExecutionOperation::Complete,
        &finished_control(package),
    )
    .unwrap()
}

fn gate(root: &Utf8Path) -> ExecutionPackage {
    let interview = execution_start(root, "page-change", Some("guard-check")).unwrap();
    let plan = advance(root, &interview);
    let packet = serde_json::to_value(&plan.packet).unwrap();
    std::fs::write(
        packet["ledgerPath"].as_str().unwrap(),
        "## Brief\nChange greeting\n",
    )
    .unwrap();
    advance(root, &plan)
}

#[test]
fn unnamed_start_derives_its_name_without_requesting_a_user_name() {
    let (_temp, root) = fixture();
    let fresh = execution_new(&root).unwrap();
    let package = execution_start(&root, "page-change", None).unwrap();
    for policy in [
        &fresh.controller_instructions,
        &package.controller_instructions,
    ] {
        assert!(policy.contains("derive a descriptive safe run name from the agreed goal"));
        assert!(policy.contains("Do not ask the user to name the run"));
        assert!(!policy.contains("Obtain the agreed goal and a descriptive name before starting"));
    }
    let run_id = package.run_id.as_deref().unwrap();
    let started = status(&root, run_id).unwrap();
    assert!(started.naming_pending);
    assert_eq!(
        serde_json::to_value(&package.packet).unwrap()["nodeId"],
        "interview"
    );
    let renamed = rename_run(&root, run_id, "change-greeting").unwrap();
    assert_eq!(renamed.run_id, started.run_id);
    assert!(!renamed.naming_pending);
    assert_eq!(
        execution_next(&root, run_id).unwrap().expected,
        package.expected
    );
}

#[test]
fn main_and_delegated_packages_deliver_mandatory_controller_completion_policy() {
    for delegated in [false, true] {
        let (_temp, root) = fixture();
        let package = render_execution_package(
            Some(packet(&root, delegated)),
            vec![],
            Some("Change greeting"),
            None,
        )
        .unwrap();
        assert_eq!(package.status, "running");
        assert!(package.controller_instructions.contains(
            "For every automated Task, main or delegated, you must call unelos_complete"
        ));
        assert!(package
            .controller_instructions
            .contains("Never hand completion back to the user or stop at a success summary"));
    }
}

#[test]
fn guarded_completion_returns_next_package_and_rejects_duplicate() {
    let (_temp, root) = fixture();
    let package = execution_start(&root, "page-change", Some("guard-check")).unwrap();
    write_outputs(&package);
    let control = finished_control(&package);
    let next_package =
        execution_mutate(&root, "r-000001", ExecutionOperation::Complete, &control).unwrap();
    assert_eq!(
        serde_json::to_value(next_package.packet).unwrap()["nodeId"],
        "plan"
    );
    let bytes = state_bytes(&root);
    assert!(execution_mutate(&root, "r-000001", ExecutionOperation::Complete, &control).is_err());
    assert_eq!(bytes, state_bytes(&root));
}

#[test]
fn rejected_completion_never_changes_confirmation_or_state() {
    let (_temp, root) = fixture();
    let package = execution_start(&root, "page-change", Some("guard-check")).unwrap();
    let bytes = state_bytes(&root);
    let control = finished_control(&package);
    assert!(execution_mutate(&root, "r-000001", ExecutionOperation::Complete, &control).is_err());
    assert_eq!(bytes, state_bytes(&root));
    let packet = serde_json::to_value(&package.packet).unwrap();
    let path = packet["outputs"][0]["path"].as_str().unwrap();
    std::fs::create_dir(path).unwrap();
    assert!(execution_mutate(&root, "r-000001", ExecutionOperation::Complete, &control).is_err());
    assert_eq!(bytes, state_bytes(&root));
    std::fs::remove_dir(path).unwrap();
    write_outputs(&package);
    for expected in [
        ExecutionIdentity::Task {
            run_id: "other".into(),
            node_id: "interview".into(),
            task_id: "task-1".into(),
        },
        ExecutionIdentity::Task {
            run_id: "r-000001".into(),
            node_id: "plan".into(),
            task_id: "task-1".into(),
        },
        ExecutionIdentity::Task {
            run_id: "r-000001".into(),
            node_id: "interview".into(),
            task_id: "task-2".into(),
        },
        ExecutionIdentity::HumanGate {
            run_id: "r-000001".into(),
            node_id: "interview".into(),
            activation_id: "task-1".into(),
        },
    ] {
        assert!(execution_mutate(
            &root,
            "r-000001",
            ExecutionOperation::Complete,
            &ExecutionControl {
                expected,
                ..control.clone()
            }
        )
        .is_err());
        assert_eq!(bytes, state_bytes(&root));
    }
    let invalid = ExecutionControl {
        outcome: Some("invented".into()),
        ..control.clone()
    };
    assert!(execution_mutate(&root, "r-000001", ExecutionOperation::Complete, &invalid).is_err());
    assert_eq!(bytes, state_bytes(&root));
    let mut value = serde_json::to_value(&control.expected).unwrap();
    value["status"] = "NEEDS_CONTEXT".into();
    value["summary"] = "Question".into();
    value["changedPaths"] = serde_json::json!([]);
    value["verification"] = serde_json::json!([]);
    value["progress"] = serde_json::json!([]);
    value["question"] = "Where?".into();
    value["dependsOn"] = "User".into();
    let question = ExecutionControl {
        result_text: Some(format!(
            "<unelos-worker-result>{value}</unelos-worker-result>"
        )),
        ..control
    };
    assert!(execution_mutate(&root, "r-000001", ExecutionOperation::Complete, &question).is_err());
    assert_eq!(bytes, state_bytes(&root));
}

#[test]
fn gate_reentry_rejects_old_activation_and_continuation_preserves_reports() {
    let (_temp, root) = fixture();
    let original_gate = gate(&root);
    let old = ExecutionControl {
        expected: original_gate.expected.unwrap(),
        result_text: None,
        outcome: None,
        reason: None,
    };
    let revised_plan = execution_mutate(
        &root,
        "r-000001",
        ExecutionOperation::Reject,
        &ExecutionControl {
            reason: Some("Add exact byte check".into()),
            ..old.clone()
        },
    )
    .unwrap();
    let new_gate = advance(&root, &revised_plan);
    assert_ne!(new_gate.expected, Some(old.expected.clone()));
    let bytes = state_bytes(&root);
    assert!(execution_mutate(&root, "r-000001", ExecutionOperation::Approve, &old).is_err());
    assert_eq!(bytes, state_bytes(&root));
    let implementation = execution_mutate(
        &root,
        "r-000001",
        ExecutionOperation::Approve,
        &ExecutionControl {
            expected: new_gate.expected.unwrap(),
            ..old
        },
    )
    .unwrap();
    write_outputs(&implementation);
    let before = state_bytes(&root);
    let packet = serde_json::to_value(&implementation.packet).unwrap();
    let ledger_path = packet["ledgerPath"].as_str().unwrap();
    let report_path = packet["outputs"][0]["path"].as_str().unwrap();
    let ledger = std::fs::read(ledger_path).unwrap();
    let report = std::fs::read(report_path).unwrap();
    let control = ContinuationControl {
        expected: implementation.expected.clone().unwrap(),
        answer: Some("Keep mobile".into()),
        progress: vec!["Read layout".into()],
    };
    let continued = execution_continue(&root, "r-000001", &control).unwrap();
    assert_eq!(continued.expected, implementation.expected);
    assert!(continued.worker_prompt.unwrap().contains("Keep mobile"));
    assert_eq!(before, state_bytes(&root));
    assert_eq!(ledger, std::fs::read(ledger_path).unwrap());
    assert_eq!(report, std::fs::read(report_path).unwrap());
    let review = advance(&root, &implementation);
    write_outputs(&review);
    let invalid = ExecutionControl {
        outcome: Some("invented".into()),
        ..finished_control(&review)
    };
    let bytes = state_bytes(&root);
    assert!(execution_mutate(&root, "r-000001", ExecutionOperation::Complete, &invalid).is_err());
    assert_eq!(bytes, state_bytes(&root));
    let mut correction = finished_control(&review);
    correction.outcome = Some("changes_requested".into());
    correction.result_text = Some(
        correction
            .result_text
            .unwrap()
            .replace("\"outcome\":null", "\"outcome\":\"changes_requested\""),
    );
    let reentered =
        execution_mutate(&root, "r-000001", ExecutionOperation::Complete, &correction).unwrap();
    assert_ne!(reentered.expected, implementation.expected);
    assert!(execution_continue(&root, "r-000001", &control).is_err());
}

#[test]
fn concurrent_duplicate_mutations_commit_once() {
    let (_temp, root) = fixture();
    let package = execution_start(&root, "page-change", Some("guard-check")).unwrap();
    write_outputs(&package);
    let control = finished_control(&package);
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
    let handles: Vec<_> = (0..2)
        .map(|_| {
            let root = root.clone();
            let control = control.clone();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                barrier.wait();
                execution_mutate(&root, "r-000001", ExecutionOperation::Complete, &control).is_ok()
            })
        })
        .collect();
    assert_eq!(
        handles
            .into_iter()
            .map(|h| h.join().unwrap())
            .filter(|ok| *ok)
            .count(),
        1
    );
    assert_eq!(status(&root, "r-000001").unwrap().history.len(), 1);
}

#[test]
fn concurrent_gate_approval_commits_once_and_missing_brief_never_advances() {
    let (_temp, root) = fixture();
    let package = gate(&root);
    let control = ExecutionControl {
        expected: package.expected.unwrap(),
        result_text: None,
        outcome: None,
        reason: None,
    };
    let ledger = root.join(".unelos/runs/guard-check/ledger.md");
    std::fs::write(&ledger, "No Brief").unwrap();
    let bytes = state_bytes(&root);
    assert!(execution_mutate(&root, "r-000001", ExecutionOperation::Approve, &control).is_err());
    assert_eq!(bytes, state_bytes(&root));
    std::fs::write(&ledger, "## Brief\nChange greeting\n").unwrap();
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
    let handles: Vec<_> = (0..2)
        .map(|_| {
            let root = root.clone();
            let control = control.clone();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                barrier.wait();
                execution_mutate(&root, "r-000001", ExecutionOperation::Approve, &control).is_ok()
            })
        })
        .collect();
    assert_eq!(
        handles
            .into_iter()
            .map(|h| h.join().unwrap())
            .filter(|ok| *ok)
            .count(),
        1
    );
    assert_eq!(status(&root, "r-000001").unwrap().history.len(), 3);
    let implementation = execution_next(&root, "r-000001").unwrap();
    let review = advance(&root, &implementation);
    write_outputs(&review);
    let mut passed = finished_control(&review);
    passed.outcome = Some("passed".into());
    passed.result_text = Some(
        passed
            .result_text
            .unwrap()
            .replace("\"outcome\":null", "\"outcome\":\"passed\""),
    );
    let finished =
        execution_mutate(&root, "r-000001", ExecutionOperation::Complete, &passed).unwrap();
    assert_eq!(finished.status, "finished");
    assert!(finished.expected.is_none());
    assert!(finished.worker_prompt.is_none());
    assert!(finished
        .controller_instructions
        .contains("Only when the returned package has status finished, explicitly tell the user"));
    assert!(finished
        .controller_instructions
        .contains("Task done; Unelos run finished"));
}

fn fixture() -> (TempDir, Utf8PathBuf) {
    let temp = TempDir::new().unwrap();
    temp.copy_from(
        Utf8Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/opencode-execution"),
        &["**/*"],
    )
    .unwrap();
    let root = Utf8Path::from_path(temp.path()).unwrap().to_owned();
    (temp, root)
}

fn packet(root: &Utf8Path, delegated: bool) -> WorkPacket {
    let mut project = load_project(root, "page-change").unwrap();
    let node_id = if delegated { "implement" } else { "interview" };
    project.workflow.start = node_id.into();
    if let NodeDefinition::Task(node) = project.workflow.nodes.get_mut(node_id).unwrap() {
        node.inputs.clear();
    }
    let state = start(&project, "prompt-check", "").unwrap();
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
            ledger_path: root.join(".unelos/runs/prompt-check/ledger.md"),
        },
    )
    .unwrap()
}

#[test]
fn delegated_prompt_contains_resolved_context_without_controller_metadata() {
    let (_temp, root) = fixture();
    let packet = packet(&root, true);
    let WorkPacket::Task(task) = &packet else {
        panic!()
    };
    let task_instructions = task.packet.context.task_instructions.clone();
    let skill = task.packet.context.skills[0].clone();
    let package =
        render_execution_package(Some(packet), vec![], Some("Change heading"), None).unwrap();
    let text = package.worker_prompt.unwrap();
    assert!(text.contains("Change heading"));
    assert!(text.contains("implementation-report.md"));
    let json: serde_json::Value = serde_json::from_str(
        text.split("Executor context:\n")
            .nth(1)
            .unwrap()
            .split("\n\nBrief:")
            .next()
            .unwrap(),
    )
    .unwrap();
    assert_eq!(json["taskInstructions"], task_instructions);
    assert_eq!(json["skills"][0], serde_json::to_value(skill).unwrap());
    assert!(json.get("ledgerPath").is_none());
    assert!(json.get("controllerInstructions").is_none());
    assert!(!text.contains("unelos-opencode-worker"));
}

#[test]
fn main_and_idle_packages_do_not_require_a_brief() {
    let (_temp, root) = fixture();
    let main = render_execution_package(Some(packet(&root, false)), vec![], None, None).unwrap();
    assert_eq!(main.status, "running");
    assert!(main.worker_prompt.is_none());
    let idle = render_execution_package(None, vec!["page-change".into()], None, None).unwrap();
    assert_eq!(idle.status, "idle");
    assert_eq!(idle.workflows, ["page-change"]);
    assert!(idle.expected.is_none());
}

#[test]
fn brief_is_exactly_one_nonempty_section_with_crlf_support() {
    assert_eq!(
        read_brief("## Brief\r\nKeep mobile\r\n## Progress\r\nDone").unwrap(),
        "Keep mobile"
    );
    for text in ["", "## Brief\n", "## Brief\nA\n## Brief\nB", "# Brief\nA"] {
        assert!(read_brief(text).is_err(), "{text}");
    }
}

#[test]
fn continuation_keeps_identity_and_rejects_stale_or_missing_brief() {
    let (_temp, root) = fixture();
    let packet = packet(&root, true);
    let expected = ExecutionIdentity::from_packet(&packet).unwrap();
    let control = ContinuationControl {
        expected: expected.clone(),
        answer: Some("Keep mobile".into()),
        progress: vec!["Layout inspected".into()],
    };
    let a = render_execution_package(
        Some(packet.clone()),
        vec![],
        Some("Heading"),
        Some(&control),
    )
    .unwrap();
    let b = render_execution_package(
        Some(packet.clone()),
        vec![],
        Some("Heading"),
        Some(&control),
    )
    .unwrap();
    assert_eq!(a.expected, Some(expected));
    assert_eq!(a.worker_prompt, b.worker_prompt);
    assert!(a.worker_prompt.unwrap().contains("Keep mobile"));
    assert!(render_execution_package(Some(packet.clone()), vec![], None, None).is_err());
    let stale = ContinuationControl {
        expected: ExecutionIdentity::Task {
            run_id: "r-000001".into(),
            node_id: "implement".into(),
            task_id: "task-later".into(),
        },
        ..control
    };
    assert!(render_execution_package(Some(packet), vec![], Some("Heading"), Some(&stale)).is_err());
}

#[test]
fn result_parser_rejects_wrong_identity_malformed_frames_and_empty_context() {
    let expected = ExecutionIdentity::Task {
        run_id: "r-1".into(),
        node_id: "implement".into(),
        task_id: "task-3".into(),
    };
    let valid = serde_json::json!({"kind":"task","runId":"r-1","nodeId":"implement","taskId":"task-3","status":"FINISHED","summary":"Checked","changedPaths":[],"verification":["Read report"],"progress":[],"outcome":null});
    let frame =
        |value: &serde_json::Value| format!("<unelos-worker-result>{value}</unelos-worker-result>");
    assert!(parse_worker_result(&frame(&valid), &expected).is_ok());
    for (key, value) in [
        ("taskId", serde_json::json!("task-4")),
        ("runId", serde_json::json!("r-2")),
        ("unexpected", serde_json::json!(true)),
        ("verification", serde_json::json!("not-array")),
    ] {
        let mut bad = valid.clone();
        bad[key] = value;
        assert!(parse_worker_result(&frame(&bad), &expected).is_err());
    }
    for text in [
        format!("{}{}", frame(&valid), frame(&valid)),
        "<unelos-worker-result>{</unelos-worker-result>".into(),
        "plain success".into(),
    ] {
        assert!(parse_worker_result(&text, &expected).is_err());
    }
    for (status, key) in [("NEEDS_CONTEXT", "question"), ("BLOCKED", "reason")] {
        let mut bad = valid.clone();
        bad["status"] = status.into();
        bad.as_object_mut().unwrap().remove("outcome");
        bad[key] = "".into();
        if status == "NEEDS_CONTEXT" {
            bad["dependsOn"] = "User".into();
        }
        assert!(parse_worker_result(&frame(&bad), &expected).is_err());
        bad[key] = "Required context".into();
        assert!(parse_worker_result(&frame(&bad), &expected).is_ok());
    }
}
