use assert_fs::TempDir;
use camino::{Utf8Path, Utf8PathBuf};
use unelos_core::*;

fn finished_control(package: &ExecutionPackage) -> CompletionControl {
    let expected = package.expected.clone().unwrap();
    let mut result =
        serde_json::json!({"taskId": serde_json::to_value(&expected).unwrap()["taskId"]});
    result["status"] = "FINISHED".into();
    result["summary"] = "Inspected".into();
    result["changedPaths"] = serde_json::json!([]);
    result["verification"] = serde_json::json!(["Read report"]);
    result["outcome"] = serde_json::Value::Null;
    CompletionControl {
        summary: Some("Inspected".into()),
        expected,
        result_text: package
            .worker_prompt
            .as_ref()
            .map(|_| format!("<unelos-worker-result>{result}</unelos-worker-result>")),
        outcome: None,
        reason: None,
    }
}

fn write_outputs(package: &ExecutionPackage) {
    let packet = serde_json::to_value(&package.packet).unwrap();
    for output in packet["outputs"].as_array().unwrap() {
        std::fs::write(
            output["path"].as_str().unwrap(),
            format!("Useful report section {:?}\n", package.expected),
        )
        .unwrap();
    }
}

fn state_bytes(root: &Utf8Path) -> Vec<u8> {
    let state = status(root, "r-000001").unwrap();
    std::fs::read(
        root.join(".unelos/runs")
            .join(&state.run_id)
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
    let interview =
        execution_start(root, "simple-plan-execute-review", Some("guard-check"), "").unwrap();
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
    let package = execution_start(&root, "simple-plan-execute-review", None, "").unwrap();
    assert!(fresh
        .controller_instructions
        .contains("Never ask the user to name the run"));
    assert!(!fresh.controller_instructions.contains("unelos_complete"));
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
        )
        .unwrap();
        assert_eq!(package.status, "running");
        let block = if delegated {
            include_str!("../src/instructions/controller-delegated.md")
        } else {
            include_str!("../src/instructions/controller-main-task.md")
        };
        assert_eq!(
            package.controller_instructions,
            format!(
                "{}\n{block}",
                include_str!("../src/instructions/controller.md")
            )
        );
    }
}

#[test]
fn every_state_delivers_only_its_policy_block() {
    let (_temp, root) = fixture();
    let (_delegated_temp, delegated_root) = fixture();
    let cases = [
        (None, "idle"),
        (Some(packet(&root, false)), "main-task"),
        (Some(packet(&delegated_root, true)), "delegated"),
        (
            Some(WorkPacket::Parallel(ParallelPacket {
                missing_inputs: vec![],
                kind: "parallel",
                run_id: "run".into(),
                task_id: "task".into(),
                node_id: "parallel".into(),
                request: String::new(),
                inputs: vec![],
                outputs: vec![],
                feedback: None,
                workers: vec![],
                ledger_path: "ledger.md".into(),
            })),
            "parallel",
        ),
        (
            Some(WorkPacket::HumanGate(GatePacket {
                description: None,
                missing_inputs: vec![],
                kind: "human-gate",
                run_id: "run".into(),
                activation_id: "gate".into(),
                node_id: "gate".into(),
                artifacts: vec![],
                ledger_path: "ledger.md".into(),
            })),
            "gate",
        ),
        (
            Some(WorkPacket::Finished(FinishedPacket {
                kind: "finished",
                run_id: "run".into(),
                node_id: "done".into(),
                ledger_path: "ledger.md".into(),
            })),
            "finished",
        ),
    ];
    for (packet, state) in cases {
        let package = render_execution_package(packet, vec![], None).unwrap();
        let block = std::fs::read_to_string(
            Utf8Path::new(env!("CARGO_MANIFEST_DIR"))
                .join(format!("src/instructions/controller-{state}.md")),
        )
        .unwrap();
        assert_eq!(
            package.controller_instructions,
            format!(
                "{}\n{block}",
                include_str!("../src/instructions/controller.md")
            )
        );
        let text = serde_json::to_string(&package).unwrap();
        for forbidden in [
            "unelos_continue",
            "complete NODE",
            "Task done; Unelos run finished",
        ] {
            assert!(!text.contains(forbidden), "{state}: {forbidden}");
        }
        if state == "idle" {
            assert!(!package.controller_instructions.contains("unelos_complete"));
        }
        if state == "gate" {
            assert!(!package.controller_instructions.contains("workerPrompt"));
            assert!(!package.controller_instructions.contains("FINISHED"));
        }
    }
}

#[test]
fn guarded_completion_returns_next_package_and_rejects_duplicate() {
    let (_temp, root) = fixture();
    let package =
        execution_start(&root, "simple-plan-execute-review", Some("guard-check"), "").unwrap();
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
    let package =
        execution_start(&root, "simple-plan-execute-review", Some("guard-check"), "").unwrap();
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
            &CompletionControl {
                summary: None,
                expected,
                ..control.clone()
            }
        )
        .is_err());
        assert_eq!(bytes, state_bytes(&root));
    }
    let invalid = CompletionControl {
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
    let question = CompletionControl {
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
    let old = CompletionControl {
        summary: None,
        expected: original_gate.expected.unwrap(),
        result_text: None,
        outcome: None,
        reason: None,
    };
    let revised_plan = execution_mutate(
        &root,
        "r-000001",
        ExecutionOperation::Reject,
        &CompletionControl {
            reason: Some("Add exact byte check".into()),
            ..old.clone()
        },
    )
    .unwrap();
    assert_eq!(
        serde_json::to_value(&revised_plan.packet).unwrap()["nodeId"],
        "plan"
    );
    let new_gate = advance(&root, &revised_plan);
    assert_ne!(new_gate.expected, Some(old.expected.clone()));
    let bytes = state_bytes(&root);
    assert!(execution_mutate(&root, "r-000001", ExecutionOperation::Approve, &old).is_err());
    assert_eq!(bytes, state_bytes(&root));
    let implementation = execution_mutate(
        &root,
        "r-000001",
        ExecutionOperation::Approve,
        &CompletionControl {
            summary: None,
            expected: new_gate.expected.unwrap(),
            ..old
        },
    )
    .unwrap();
    write_outputs(&implementation);
    let implementation_context = serde_json::to_value(&implementation.packet).unwrap();
    assert_eq!(implementation_context["nodeId"], "implement");
    assert_eq!(implementation_context["execution"], "subagent");
    assert_eq!(implementation_context["agent"], "simple-implementer");
    let before = state_bytes(&root);
    let packet = serde_json::to_value(&implementation.packet).unwrap();
    let ledger_path = packet["ledgerPath"].as_str().unwrap();
    let report_path = packet["outputs"][0]["path"].as_str().unwrap();
    let ledger = std::fs::read(ledger_path).unwrap();
    let report = std::fs::read(report_path).unwrap();
    let refreshed = execution_next(&root, "r-000001").unwrap();
    assert_eq!(refreshed.expected, implementation.expected);
    assert_eq!(before, state_bytes(&root));
    assert_eq!(ledger, std::fs::read(ledger_path).unwrap());
    assert_eq!(report, std::fs::read(report_path).unwrap());
    let review = advance(&root, &implementation);
    let review_context = serde_json::to_value(&review.packet).unwrap();
    assert_eq!(review_context["nodeId"], "review");
    assert_eq!(review_context["execution"], "subagent");
    assert_eq!(review_context["agent"], "simple-reviewer");
    write_outputs(&review);
    let invalid = CompletionControl {
        result_text: finished_control(&review)
            .result_text
            .map(|s| s.replace("\"outcome\":null", "\"outcome\":\"invented\"")),
        ..finished_control(&review)
    };
    let bytes = state_bytes(&root);
    assert!(execution_mutate(&root, "r-000001", ExecutionOperation::Complete, &invalid).is_err());
    assert_eq!(bytes, state_bytes(&root));
    let mut correction = finished_control(&review);
    correction.outcome = Some("ignored-for-delegated-work".into());
    correction.result_text = Some(
        correction
            .result_text
            .unwrap()
            .replace("\"outcome\":null", "\"outcome\":\"changes_requested\""),
    );
    let reentered =
        execution_mutate(&root, "r-000001", ExecutionOperation::Complete, &correction).unwrap();
    assert_eq!(
        serde_json::to_value(&reentered.packet).unwrap()["nodeId"],
        "implement"
    );
    assert_ne!(reentered.expected, implementation.expected);
}

#[test]
fn concurrent_duplicate_mutations_commit_once() {
    let (_temp, root) = fixture();
    let package =
        execution_start(&root, "simple-plan-execute-review", Some("guard-check"), "").unwrap();
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
fn concurrent_gate_approval_commits_once_without_brief() {
    let (_temp, root) = fixture();
    let package = gate(&root);
    let control = CompletionControl {
        summary: None,
        expected: package.expected.unwrap(),
        result_text: None,
        outcome: None,
        reason: None,
    };
    let ledger = root.join(".unelos/runs/r-000001/ledger.md");
    std::fs::write(&ledger, "No Brief").unwrap();
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
        .contains("The run is finished. Tell the user it is done"));
}

fn fixture() -> (TempDir, Utf8PathBuf) {
    let temp = TempDir::new().unwrap();
    let root = Utf8Path::from_path(temp.path()).unwrap().to_owned();
    unelos_core::install_workflow(
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

fn packet(root: &Utf8Path, delegated: bool) -> WorkPacket {
    let mut project = load_project(root, "simple-plan-execute-review").unwrap();
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
                missing_inputs: vec![],
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
    // Skill forwarding is tested independently of the example's dependencies.
    let definitions = root.join(".unelos/workflows/simple-plan-execute-review.yaml");
    let text = std::fs::read_to_string(&definitions)
        .unwrap()
        .replace("\r\n", "\n")
        .replace(
            "  implement:\n    taskDefinition:\n",
            "  implement:\n    taskDefinition:\n      skills: [test-context]\n",
        );
    std::fs::write(definitions, text).unwrap();
    let directory = root.join(".unelos/skills/test-context");
    std::fs::create_dir_all(&directory).unwrap();
    std::fs::write(directory.join("SKILL.md"), "---\nname: test-context\ndescription: Test context forwarding.\n---\nUse the supplied context.\n").unwrap();
    let packet = packet(&root, true);
    let WorkPacket::Task(task) = &packet else {
        panic!()
    };
    let task_instructions = task.packet.context.task_instructions.clone();
    let skill = task.packet.context.skills[0].clone();
    let package = render_execution_package(Some(packet), vec![], Some("Change heading")).unwrap();
    let text = package.worker_prompt.unwrap();
    assert!(text.contains("Change heading"));
    assert!(text.contains("implementation-report.md"));
    assert!(text.contains(&task_instructions));
    assert!(text.contains(&skill.content));
    assert!(text.starts_with("# Unelos Task: implement\n"));
    let json = serde_json::to_value(&package.packet).unwrap();
    for field in [
        "taskInstructions",
        "agentInstructions",
        "skills",
        "completionInstructions",
    ] {
        assert!(json.get(field).is_none(), "{field}");
    }
    assert!(!text.contains("unelos-opencode-worker"));
}

#[test]
fn main_and_idle_packages_do_not_require_a_brief() {
    let (_temp, root) = fixture();
    let main = render_execution_package(Some(packet(&root, false)), vec![], None).unwrap();
    assert_eq!(main.status, "running");
    assert!(main.worker_prompt.is_none());
    let idle =
        render_execution_package(None, vec!["simple-plan-execute-review".into()], None).unwrap();
    assert_eq!(idle.status, "idle");
    assert_eq!(idle.workflows, ["simple-plan-execute-review"]);
    assert!(idle.expected.is_none());
}

#[test]
fn markdown_prompt_includes_only_nonempty_sections_and_marks_existing_outputs() {
    let (_temp, root) = fixture();
    let mut work = packet(&root, true);
    let WorkPacket::Task(task) = &mut work else {
        panic!()
    };
    task.packet.request = "Keep the public API".into();
    let path = task.packet.outputs[0].path.clone();
    std::fs::write(&path, "Existing useful content").unwrap();
    let prompt = render_execution_package(Some(work.clone()), vec![], Some("Agreed goal"))
        .unwrap()
        .worker_prompt
        .unwrap();
    assert!(prompt.contains("## Request\nKeep the public API"));
    assert!(prompt.contains("## Brief\nAgreed goal"));
    assert!(prompt.contains("(exists: update in place, keep useful content)"));
    let WorkPacket::Task(task) = &mut work else {
        panic!()
    };
    task.packet.request.clear();
    let prompt = render_execution_package(Some(work), vec![], None)
        .unwrap()
        .worker_prompt
        .unwrap();
    assert!(!prompt.contains("## Request"));
    assert!(!prompt.contains("## Brief"));
    assert!(!prompt.contains("Continuation context"));
}

#[test]
fn delegated_first_start_renders_worker_prompt_without_brief() {
    let (_temp, root) = fixture();
    std::fs::write(
        root.join(".unelos/workflows/simple-plan-execute-review.yaml"),
        "name: simple-plan-execute-review\nstart: work\nnodes:\n  work:\n    taskDefinition:\n      instructions: Do the work.\n    agent: simple-implementer\n    execution: subagent\n    next: done\n  done:\n    type: end\n",
    ).unwrap();
    let package = execution_start(&root, "simple-plan-execute-review", None, "").unwrap();
    assert!(package.worker_prompt.is_some());
    assert!(!package
        .worker_prompt
        .as_ref()
        .unwrap()
        .contains("\n\nBrief:"));
    let serialized = serde_json::to_value(&package).unwrap();
    assert!(serialized["packet"].get("controllerInstructions").is_none());
    assert!(!serialized.to_string().contains("complete NODE"));
}

#[test]
fn brief_is_optional_first_section_with_crlf_support() {
    assert_eq!(
        read_brief("## Brief\r\nKeep mobile\r\n## Progress\r\nDone").unwrap(),
        "Keep mobile"
    );
    assert_eq!(read_brief("## Brief\nA\n## Brief\nB").as_deref(), Some("A"));
    assert_eq!(read_brief("## Brief\n## Brief\nB"), None);
    for text in ["", "## Brief\n", "# Brief\nA"] {
        assert!(read_brief(text).is_none(), "{text}");
    }
}

#[test]
fn result_parser_rejects_wrong_identity_malformed_frames_and_empty_context() {
    let expected = ExecutionIdentity::Task {
        run_id: "r-1".into(),
        node_id: "implement".into(),
        task_id: "task-3".into(),
    };
    let valid = serde_json::json!({"taskId":"task-3","status":"FINISHED","summary":"Checked","changedPaths":[],"verification":["Read report"],"outcome":null});
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
        bad[key] = "".into();
        assert!(parse_worker_result(&frame(&bad), &expected).is_err());
        bad[key] = "Required context".into();
        assert!(parse_worker_result(&frame(&bad), &expected).is_ok());
    }
}

#[test]
fn result_errors_name_the_failed_check_and_reject_duplicate_keys() {
    let expected = ExecutionIdentity::Task {
        run_id: "r-1".into(),
        node_id: "work".into(),
        task_id: "task-5".into(),
    };
    let valid = r#"{"taskId":"task-5","status":"FINISHED","outcome":null,"summary":"Done","changedPaths":[],"verification":[]}"#;
    let frame = |json: &str| {
        format!("surrounding text\n<unelos-worker-result>{json}</unelos-worker-result>\nmore text")
    };
    assert!(parse_worker_result(&frame(valid), &expected).is_ok());
    for (json, message) in [
        (
            valid.replace("task-5", "task-3"),
            "taskId \"task-3\" does not match expected \"task-5\"",
        ),
        (
            valid.replace("\"taskId\":", "\"taskId\":\"first\",\"taskId\":"),
            "duplicate field `taskId`",
        ),
        (
            valid.replace("\"summary\":", "\"notes\":true,\"summary\":"),
            "unknown field `notes`",
        ),
        (
            valid.replace("FINISHED", "NEEDS_CONTEXT"),
            "NEEDS_CONTEXT requires a nonempty question",
        ),
        (
            valid.replace("\"outcome\":null,", ""),
            "missing field `outcome`",
        ),
        ("{".into(), "invalid JSON at line"),
    ] {
        let error = parse_worker_result(&frame(&json), &expected).unwrap_err();
        assert!(error.to_string().contains(message), "{error}");
    }
    assert!(parse_worker_result("done", &expected)
        .unwrap_err()
        .to_string()
        .contains("no <unelos-worker-result> frame found"));
}
