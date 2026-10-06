use assert_fs::TempDir;
use camino::{Utf8Path, Utf8PathBuf};
use std::fs;
use unelos_core::{
    build_work_packet, inspect_node, inspect_task, load_project, ArtifactReference, ContextError,
    NodeDefinition, TaskRuntimeInput, TransitionFeedback, WorkPacket, WorkRuntimeInput,
    WorkerReportBinding,
};

fn project() -> (TempDir, unelos_core::LoadedProject) {
    let temp = TempDir::new().unwrap();
    let root = Utf8Path::from_path(temp.path()).unwrap();
    fs::create_dir_all(root.join(".unelos/workflows")).unwrap();
    fs::write(
        root.join(".unelos/project.yaml"),
        "taskDefinitions:\n  plan:\n    instructions: Write the plan.\n  backend:\n    instructions: Research backend.\n    skills: [backend]\n  frontend:\n    instructions: Research frontend.\n    skills: [frontend]\nagents:\n  planner:\n    instructions: Plan carefully.\n  backend-researcher:\n    instructions: Backend context only.\n    skills: [backend]\n    model: example/backend\n    mcps: [backend-mcp]\n    tools: [read-file]\n  frontend-researcher:\n    instructions: Frontend context only.\n    tools: [read-file]\n",
    )
    .unwrap();
    fs::write(
        root.join(".unelos/workflows/research-feature.yaml"),
        "name: research-feature\nstart: plan\nnodes:\n  plan:\n    task: plan\n    agent: planner\n    outputs: [plan.md]\n    next: parallel-research\n  parallel-research:\n    type: parallel\n    inputs: [plan.md]\n    tasks:\n      - task: backend\n        agent: backend-researcher\n      - task: frontend\n        agent: frontend-researcher\n    outputs: [research-summary.md]\n    next: approve-plan\n  approve-plan:\n    type: human-gate\n    inputs: [plan.md, research-summary.md]\n    next:\n      approved: done\n      rejected: parallel-research\n  done:\n    type: end\n",
    )
    .unwrap();
    for (name, marker) in [
        ("backend", "BACKEND_ONLY_MARKER\n"),
        ("frontend", "FRONTEND_ONLY_MARKER\n"),
    ] {
        let skill = root.join(format!(".unelos/skills/{name}/SKILL.md"));
        fs::create_dir_all(skill.parent().unwrap()).unwrap();
        fs::write(skill, marker).unwrap();
    }
    let project = load_project(root, "research-feature").unwrap();
    (temp, project)
}

#[test]
fn inspects_parallel_workers_with_isolated_context_and_stable_keys() {
    let (_temp, project) = project();
    let inspected = inspect_node(&project, "parallel-research").unwrap();
    let value = serde_json::to_value(&inspected).unwrap();
    assert_eq!(value["kind"], "parallel");
    assert_eq!(value["inputs"], serde_json::json!(["plan.md"]));
    assert_eq!(value["outputs"], serde_json::json!(["research-summary.md"]));
    assert_eq!(value["workers"][0]["key"], "worker-1");
    assert_eq!(value["workers"][1]["key"], "worker-2");
    assert_eq!(value["workers"][0]["execution"], "subagent");
    assert_eq!(value["workers"][0]["skills"].as_array().unwrap().len(), 1);
    assert!(value["workers"][0]
        .to_string()
        .contains("BACKEND_ONLY_MARKER"));
    assert!(!value["workers"][0]
        .to_string()
        .contains("FRONTEND_ONLY_MARKER"));
    assert!(value["workers"][1]
        .to_string()
        .contains("FRONTEND_ONLY_MARKER"));
    assert!(!value["workers"][1]
        .to_string()
        .contains("BACKEND_ONLY_MARKER"));
    assert_eq!(
        value["workers"][0]["outputs"],
        serde_json::json!(["result.md"])
    );
    for field in ["runId", "taskId", "request", "feedback", "ledgerPath"] {
        assert!(value.get(field).is_none());
        assert!(value["workers"][0].get(field).is_none());
    }
    assert_eq!(
        serde_json::to_vec(&inspected).unwrap(),
        serde_json::to_vec(&inspect_node(&project, "parallel-research").unwrap()).unwrap()
    );
}

#[test]
fn inspects_gate_and_end_without_runtime_identity_and_allows_duplicate_workers() {
    let (_temp, mut project) = project();
    let gate = serde_json::to_value(inspect_node(&project, "approve-plan").unwrap()).unwrap();
    assert_eq!(gate["kind"], "human-gate");
    assert_eq!(
        gate["inputs"],
        serde_json::json!(["plan.md", "research-summary.md"])
    );
    assert_eq!(
        gate["decisions"],
        serde_json::json!(["approved", "rejected"])
    );
    assert_eq!(gate["rejectionReasonRequired"], true);
    for field in ["runId", "taskId", "agent", "taskDefinition"] {
        assert!(gate.get(field).is_none());
    }
    let end = serde_json::to_value(inspect_node(&project, "done").unwrap()).unwrap();
    assert_eq!(end["kind"], "end");
    for field in ["runId", "taskId"] {
        assert!(end.get(field).is_none());
    }

    if let NodeDefinition::Parallel(parallel) =
        project.workflow.nodes.get_mut("parallel-research").unwrap()
    {
        parallel.tasks[1] = parallel.tasks[0].clone();
    }
    let duplicated =
        serde_json::to_value(inspect_node(&project, "parallel-research").unwrap()).unwrap();
    assert_eq!(duplicated["workers"][0]["key"], "worker-1");
    assert_eq!(duplicated["workers"][1]["key"], "worker-2");
}

#[test]
fn reports_invalid_node_worker_gate_and_selected_skill_context() {
    let (_temp, mut loaded) = project();
    let error = inspect_node(&loaded, "missing").unwrap_err();
    assert!(matches!(error, ContextError::Diagnostic { path, field, .. }
        if path == loaded.workflow_source && field == "nodes.missing"));

    if let NodeDefinition::Parallel(parallel) =
        loaded.workflow.nodes.get_mut("parallel-research").unwrap()
    {
        parallel.tasks[0].task = "missing-task".into();
    }
    let error = inspect_node(&loaded, "parallel-research").unwrap_err();
    assert!(error.to_string().contains("tasks[0].task"));

    let (_temp, mut loaded) = project();
    if let NodeDefinition::Parallel(parallel) =
        loaded.workflow.nodes.get_mut("parallel-research").unwrap()
    {
        parallel.tasks[0].agent = "missing-agent".into();
    }
    assert!(inspect_node(&loaded, "parallel-research")
        .unwrap_err()
        .to_string()
        .contains("tasks[0].agent"));

    let (_temp, mut loaded) = project();
    if let NodeDefinition::Parallel(parallel) =
        loaded.workflow.nodes.get_mut("parallel-research").unwrap()
    {
        parallel.tasks.clear();
    }
    assert!(inspect_node(&loaded, "parallel-research")
        .unwrap_err()
        .to_string()
        .contains("at least one"));

    let (_temp, mut loaded) = project();
    if let NodeDefinition::HumanGate(gate) = loaded.workflow.nodes.get_mut("approve-plan").unwrap()
    {
        gate.next.remove("rejected");
    }
    assert!(inspect_node(&loaded, "approve-plan")
        .unwrap_err()
        .to_string()
        .contains("approved"));

    let (_temp, loaded) = project();
    fs::remove_file(loaded.project_root.join(".unelos/skills/backend/SKILL.md")).unwrap();
    assert!(inspect_node(&loaded, "parallel-research")
        .unwrap_err()
        .to_string()
        .contains("backend"));
    assert!(inspect_node(&loaded, "approve-plan").is_ok());
    assert!(inspect_node(&loaded, "done").is_ok());
}

#[test]
fn task_node_inspection_reuses_task_inspection_shape() {
    let (_temp, project) = project();
    let inspected = inspect_node(&project, "plan").unwrap();
    assert_eq!(
        serde_json::to_value(&inspected).unwrap(),
        serde_json::to_value(inspect_task(&project, "plan").unwrap()).unwrap()
    );
    assert_eq!(
        String::from_utf8(serde_json::to_vec(&inspected).unwrap())
            .unwrap()
            .matches("\"kind\":")
            .count(),
        1
    );
}

fn runtime_input() -> (TempDir, unelos_core::LoadedProject, WorkRuntimeInput) {
    let (temp, project) = project();
    let run_root = project.project_root.join(".unelos/runs/change café button");
    let binding = |name: &str, path: &str| ArtifactReference {
        name: name.into(),
        path: run_root.join(path),
    };
    let feedback = TransitionFeedback {
        source_node_id: "approve-plan".into(),
        source_execution_id: "gate-previous".into(),
        outcome: "rejected".into(),
        reason: Some("Explain the risks.".into()),
        artifacts: vec![binding(
            "research-summary.md",
            "parallel-research/research-summary.md",
        )],
    };
    let input = WorkRuntimeInput::Parallel {
        input: TaskRuntimeInput {
            run_id: "change-button-color".into(),
            task_id: "execution-2".into(),
            node_id: "parallel-research".into(),
            request: "Change café button color.".into(),
            inputs: vec![binding("plan.md", "plan/plan.md")],
            outputs: vec![binding(
                "research-summary.md",
                "parallel-research/research-summary.md",
            )],
            feedback: Some(feedback),
        },
        worker_reports: vec![
            WorkerReportBinding {
                key: "worker-2".into(),
                report: binding(
                    "result.md",
                    "parallel-research/frontend-researcher/result.md",
                ),
            },
            WorkerReportBinding {
                key: "worker-1".into(),
                report: binding(
                    "result.md",
                    "parallel-research/backend-researcher/result.md",
                ),
            },
        ],
        ledger_path: run_root.join("ledger.md"),
    };
    (temp, project, input)
}

#[test]
fn composes_deterministic_parallel_workers_with_explicit_paths_and_parent_identity() {
    let (_temp, project, input) = runtime_input();
    let packet = build_work_packet(&project, input.clone()).unwrap();
    let value = serde_json::to_value(&packet).unwrap();
    let run_root = project.project_root.join(".unelos/runs/change café button");
    let feedback = match &input {
        WorkRuntimeInput::Parallel { input, .. } => input.feedback.as_ref().unwrap(),
        _ => unreachable!(),
    };
    assert_eq!(value["kind"], "parallel");
    assert_eq!(value["workers"][0]["key"], "worker-1");
    assert_eq!(value["workers"][0]["parentTaskId"], "execution-2");
    assert!(value["workers"][0].get("taskId").is_none());
    assert!(value["workers"][0].get("ledgerPath").is_none());
    assert_eq!(value["workers"][0]["request"], "Change café button color.");
    assert_eq!(
        value["workers"][0]["feedback"],
        serde_json::to_value(feedback).unwrap()
    );
    assert_eq!(value["inputs"][0]["name"], "plan.md");
    assert_eq!(value["outputs"][0]["name"], "research-summary.md");
    assert_eq!(
        value["workers"][0]["outputs"][0]["path"],
        run_root
            .join("parallel-research/backend-researcher/result.md")
            .as_str()
    );
    let instructions = value["controllerInstructions"].as_str().unwrap();
    for duty in [
        "Only the controller updates the supplied run ledger",
        "Dispatch every listed worker",
        "relay questions",
        "Wait for all reports",
        "read them",
        "Never complete a blocked or partial parent",
        "Never use the ledger to derive destinations",
    ] {
        assert!(instructions.contains(duty), "missing {duty}");
    }
    assert_eq!(
        serde_json::to_vec(&packet).unwrap(),
        serde_json::to_vec(&build_work_packet(&project, input.clone()).unwrap()).unwrap()
    );
    assert!(!run_root.exists());
    if let WorkRuntimeInput::Parallel {
        input,
        worker_reports,
        ledger_path,
    } = input
    {
        let reports = worker_reports.clone();
        let mut changed = input.clone();
        changed.task_id = "execution-3".into();
        let rebound = build_work_packet(
            &project,
            WorkRuntimeInput::Parallel {
                input: changed,
                worker_reports,
                ledger_path: ledger_path.clone(),
            },
        )
        .unwrap();
        let rebound = serde_json::to_value(rebound).unwrap();
        assert_eq!(rebound["workers"][0]["parentTaskId"], "execution-3");
        assert_eq!(
            rebound["workers"][0]["outputs"],
            value["workers"][0]["outputs"]
        );
        assert_eq!(reports.len(), 2);
    }
}

#[test]
fn rejects_invalid_worker_report_bindings_and_parent_artifact_bindings() {
    let (_temp, project, input) = runtime_input();
    let WorkRuntimeInput::Parallel {
        input: task,
        worker_reports,
        ledger_path,
    } = input
    else {
        unreachable!()
    };
    for (reports, expected) in [
        (worker_reports[..1].to_vec(), "worker-1"),
        (
            {
                let mut r = worker_reports.clone();
                r[1].key = "worker-2".into();
                r
            },
            "worker-2",
        ),
        (
            {
                let mut r = worker_reports.clone();
                r[1].key = "worker-99".into();
                r
            },
            "worker-99",
        ),
        (
            {
                let mut r = worker_reports.clone();
                r[0].report.name = "summary.md".into();
                r
            },
            "summary.md",
        ),
        (
            {
                let mut r = worker_reports.clone();
                r[1].report.path = r[0].report.path.clone();
                r
            },
            "worker-2",
        ),
        (
            {
                let mut r = worker_reports.clone();
                r[1].report.path = task.outputs[0].path.clone();
                r
            },
            "worker-1",
        ),
        (
            {
                let mut r = worker_reports.clone();
                r[1].report.path = ledger_path.clone();
                r
            },
            "worker-1",
        ),
    ] {
        let error = build_work_packet(
            &project,
            WorkRuntimeInput::Parallel {
                input: task.clone(),
                worker_reports: reports,
                ledger_path: ledger_path.clone(),
            },
        )
        .unwrap_err()
        .to_string();
        assert!(error.contains("nodes.parallel-research.workers"), "{error}");
        assert!(error.contains(expected), "{error}");
    }
    for (side, bindings, name) in [
        ("inputs", vec![], "plan.md"),
        (
            "inputs",
            vec![
                ArtifactReference {
                    name: "plan.md".into(),
                    path: Utf8PathBuf::from("a"),
                },
                ArtifactReference {
                    name: "plan.md".into(),
                    path: Utf8PathBuf::from("b"),
                },
            ],
            "plan.md",
        ),
        (
            "inputs",
            vec![ArtifactReference {
                name: "extra.md".into(),
                path: Utf8PathBuf::from("extra"),
            }],
            "extra.md",
        ),
        ("outputs", vec![], "research-summary.md"),
        (
            "outputs",
            vec![ArtifactReference {
                name: "extra.md".into(),
                path: Utf8PathBuf::from("extra"),
            }],
            "extra.md",
        ),
        (
            "outputs",
            vec![
                ArtifactReference {
                    name: "research-summary.md".into(),
                    path: Utf8PathBuf::from("a"),
                },
                ArtifactReference {
                    name: "research-summary.md".into(),
                    path: Utf8PathBuf::from("b"),
                },
            ],
            "research-summary.md",
        ),
    ] {
        let mut changed = task.clone();
        if side == "inputs" {
            changed.inputs = bindings;
        } else {
            changed.outputs = bindings;
        }
        let error = build_work_packet(
            &project,
            WorkRuntimeInput::Parallel {
                input: changed,
                worker_reports: worker_reports.clone(),
                ledger_path: ledger_path.clone(),
            },
        )
        .unwrap_err()
        .to_string();
        assert!(
            error.contains(&format!("nodes.parallel-research.{side}.{name}")),
            "{error}"
        );
    }
}

#[test]
fn composes_task_gate_and_finished_variants_with_controller_contracts() {
    let (_temp, project, parallel) = runtime_input();
    let WorkRuntimeInput::Parallel {
        input: parent,
        ledger_path,
        ..
    } = parallel
    else {
        unreachable!()
    };
    let root = project.project_root.join(".unelos/runs/gate");
    let gate = build_work_packet(
        &project,
        WorkRuntimeInput::HumanGate {
            run_id: "run-1".into(),
            activation_id: "activation-2".into(),
            node_id: "approve-plan".into(),
            artifacts: vec![
                ArtifactReference {
                    name: "research-summary.md".into(),
                    path: root.join("summary"),
                },
                ArtifactReference {
                    name: "plan.md".into(),
                    path: root.join("plan"),
                },
            ],
            ledger_path: ledger_path.clone(),
        },
    )
    .unwrap();
    let gate_json = serde_json::to_value(&gate).unwrap();
    assert_eq!(gate_json["kind"], "human-gate");
    assert_eq!(gate_json["runId"], "run-1");
    assert_eq!(gate_json["activationId"], "activation-2");
    assert_eq!(
        gate_json["artifacts"][0]["path"],
        root.join("plan").as_str()
    );
    assert_eq!(
        gate_json["decisions"],
        serde_json::json!(["approved", "rejected"])
    );
    assert_eq!(gate_json["rejectionReasonRequired"], true);
    assert!(gate_json.get("taskId").is_none() && gate_json.get("agent").is_none());
    let gate_instructions = gate_json["controllerInstructions"].as_str().unwrap();
    assert!(gate_instructions.contains("Only the controller updates the supplied run ledger"));
    assert!(gate_instructions.contains("nonempty rejection reason"));
    assert!(gate_instructions.contains("through Unelos"));

    let finished = build_work_packet(
        &project,
        WorkRuntimeInput::Finished {
            run_id: "run-1".into(),
            node_id: "done".into(),
            ledger_path: ledger_path.clone(),
        },
    )
    .unwrap();
    let finished_json = serde_json::to_value(&finished).unwrap();
    assert_eq!(finished_json["kind"], "finished");
    assert_eq!(finished_json["remainingWork"], false);
    assert_eq!(finished_json["nodeId"], "done");
    assert!(finished_json.get("taskId").is_none() && finished_json.get("executor").is_none());
    assert!(finished_json["controllerInstructions"]
        .as_str()
        .unwrap()
        .contains("final status reported by Unelos"));
    assert_ne!(gate_json["kind"], finished_json["kind"]);

    let task = build_work_packet(
        &project,
        WorkRuntimeInput::Task {
            input: TaskRuntimeInput {
                run_id: parent.run_id,
                task_id: "task-plan".into(),
                node_id: "plan".into(),
                request: "Plan it".into(),
                inputs: vec![],
                outputs: vec![ArtifactReference {
                    name: "plan.md".into(),
                    path: root.join("plan"),
                }],
                feedback: None,
            },
            ledger_path: ledger_path.clone(),
        },
    )
    .unwrap();
    let task_json = serde_json::to_value(&task).unwrap();
    assert_eq!(task_json["kind"], "task");
    assert_eq!(task_json["taskId"], "task-plan");
    assert!(task_json.get("controllerInstructions").is_some());
    assert_eq!(task_json["feedback"], serde_json::Value::Null);
    assert_eq!(
        serde_json::to_vec(&task)
            .unwrap()
            .windows(b"\"kind\":".len())
            .filter(|w| *w == b"\"kind\":")
            .count(),
        1
    );
    assert!(matches!(task, WorkPacket::Task(_)));
    assert_eq!(task_json["runId"], "change-button-color");
    assert_eq!(task_json["request"], "Plan it");
    assert_eq!(task_json["outputs"][0]["name"], "plan.md");
    assert_eq!(task_json["taskDefinition"], "plan");
    assert!(task_json["controllerInstructions"]
        .as_str()
        .unwrap()
        .contains("Read existing declared outputs"));
    assert_eq!(
        serde_json::to_value(inspect_node(&project, "done").unwrap()).unwrap()["kind"],
        "end"
    );
    assert_eq!(
        serde_json::to_value(inspect_node(&project, "approve-plan").unwrap()).unwrap()["kind"],
        gate_json["kind"]
    );
}

#[test]
fn repeated_worker_references_keep_separate_keys_and_report_paths() {
    let (_temp, mut project, input) = runtime_input();
    if let NodeDefinition::Parallel(parallel) =
        project.workflow.nodes.get_mut("parallel-research").unwrap()
    {
        parallel.tasks[1] = parallel.tasks[0].clone();
    }
    let WorkRuntimeInput::Parallel {
        input,
        worker_reports,
        ledger_path,
    } = input
    else {
        unreachable!()
    };
    let packet = serde_json::to_value(
        build_work_packet(
            &project,
            WorkRuntimeInput::Parallel {
                input,
                worker_reports,
                ledger_path,
            },
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(packet["workers"][0]["key"], "worker-1");
    assert_eq!(packet["workers"][1]["key"], "worker-2");
    assert_eq!(
        packet["workers"][0]["taskDefinition"],
        packet["workers"][1]["taskDefinition"]
    );
    assert_ne!(
        packet["workers"][0]["outputs"][0]["path"],
        packet["workers"][1]["outputs"][0]["path"]
    );
}

#[test]
fn runtime_variants_reject_wrong_or_unknown_nodes_and_gate_binding_errors() {
    let (_temp, project, parallel) = runtime_input();
    let WorkRuntimeInput::Parallel {
        input,
        worker_reports,
        ledger_path,
    } = parallel
    else {
        unreachable!()
    };
    for node_id in ["plan", "unknown"] {
        let mut wrong = input.clone();
        wrong.node_id = node_id.into();
        let error = build_work_packet(
            &project,
            WorkRuntimeInput::Parallel {
                input: wrong,
                worker_reports: worker_reports.clone(),
                ledger_path: ledger_path.clone(),
            },
        )
        .unwrap_err()
        .to_string();
        assert!(error.contains(&format!("nodes.{node_id}")), "{error}");
    }
    let bad_gate = WorkRuntimeInput::HumanGate {
        run_id: "r".into(),
        activation_id: "a".into(),
        node_id: "approve-plan".into(),
        artifacts: vec![],
        ledger_path: ledger_path.clone(),
    };
    assert!(build_work_packet(&project, bad_gate)
        .unwrap_err()
        .to_string()
        .contains("nodes.approve-plan.inputs.plan.md"));
    for node_id in ["done", "unknown"] {
        let error = build_work_packet(
            &project,
            WorkRuntimeInput::HumanGate {
                run_id: "r".into(),
                activation_id: "a".into(),
                node_id: node_id.into(),
                artifacts: vec![],
                ledger_path: ledger_path.clone(),
            },
        )
        .unwrap_err()
        .to_string();
        assert!(error.contains(&format!("nodes.{node_id}")), "{error}");
    }
    for node_id in ["parallel-research", "unknown"] {
        let error = build_work_packet(
            &project,
            WorkRuntimeInput::Finished {
                run_id: "r".into(),
                node_id: node_id.into(),
                ledger_path: ledger_path.clone(),
            },
        )
        .unwrap_err()
        .to_string();
        assert!(error.contains(&format!("nodes.{node_id}")), "{error}");
    }
    assert!(matches!(
        build_work_packet(
            &project,
            WorkRuntimeInput::Task {
                input: TaskRuntimeInput {
                    run_id: "r".into(),
                    task_id: "t".into(),
                    node_id: "parallel-research".into(),
                    request: String::new(),
                    inputs: vec![],
                    outputs: vec![],
                    feedback: None
                },
                ledger_path: ledger_path.clone(),
            }
        ),
        Err(ContextError::Diagnostic { .. })
    ));
    let mut unknown_task = TaskRuntimeInput {
        run_id: "r".into(),
        task_id: "t".into(),
        node_id: "unknown".into(),
        request: String::new(),
        inputs: vec![],
        outputs: vec![],
        feedback: None,
    };
    assert!(build_work_packet(
        &project,
        WorkRuntimeInput::Task {
            input: unknown_task.clone(),
            ledger_path: ledger_path.clone(),
        }
    )
    .unwrap_err()
    .to_string()
    .contains("nodes.unknown"));
    unknown_task.node_id = "approve-plan".into();
    assert!(build_work_packet(
        &project,
        WorkRuntimeInput::Task {
            input: unknown_task,
            ledger_path,
        }
    )
    .is_err());
}
