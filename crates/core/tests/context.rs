use assert_fs::TempDir;
use camino::Utf8Path;
use std::fs;
use unelos_core::{
    build_task_packet, inspect_task, load_project, ArtifactReference, NodeDefinition, ParallelNode,
    SkillResolveError, TaskRuntimeInput, Transition, TransitionFeedback,
};

fn fixture_project() -> unelos_core::LoadedProject {
    let root = Utf8Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/project");
    load_project(&root, "implement-feature").unwrap()
}

fn runtime_project() -> (TempDir, unelos_core::LoadedProject) {
    let temp = TempDir::new().unwrap();
    let source = Utf8Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/project");
    let root = Utf8Path::from_path(temp.path()).unwrap().join("project");
    for path in [
        ".unelos/project.yaml",
        ".unelos/workflows/implement-feature.yaml",
        ".unelos/tasks/create-plan/task.yaml",
        ".unelos/tasks/implement-plan/task.yaml",
        ".unelos/tasks/review-implementation/task.yaml",
        ".unelos/agents/planner/agent.yaml",
        ".unelos/agents/engineer/agent.yaml",
        ".unelos/agents/reviewer/agent.yaml",
    ] {
        fs::create_dir_all(root.join(path).parent().unwrap()).unwrap();
        fs::copy(source.join(path), root.join(path)).unwrap();
    }
    let mut project = load_project(&root, "implement-feature").unwrap();
    for task in project.definitions.task_definitions.values_mut() {
        task.skills.clear();
    }
    for agent in project.definitions.agents.values_mut() {
        agent.skills.clear();
    }
    (temp, project)
}

fn runtime_input(root: &Utf8Path) -> TaskRuntimeInput {
    TaskRuntimeInput {
        run_id: "run-1".into(),
        task_id: "task-3".into(),
        node_id: "implement".into(),
        request: "Implement naïve café.\nSecond line.".into(),
        inputs: vec![ArtifactReference {
            name: "plan.md".into(),
            path: root.join(".unelos/runs/run-1/tasks/task-1/outputs/plan.md"),
        }],
        outputs: vec![ArtifactReference {
            name: "implementation-report.md".into(),
            path: root.join(".unelos/runs/run-1/tasks/task-3/outputs/report.md"),
        }],
        feedback: Some(TransitionFeedback {
            source_node_id: "review".into(),
            source_execution_id: "task-2".into(),
            outcome: Some("changes_requested".into()),
            reason: Some("Café said \"redo\".\nAgain.".into()),
            artifacts: vec![ArtifactReference {
                name: "review.md".into(),
                path: root.join("review.md"),
            }],
        }),
    }
}

#[test]
fn resolves_the_selected_task_into_deterministic_inspection_json() {
    let mut project = fixture_project();
    let task = project
        .definitions
        .task_definitions
        .get_mut("implement-plan")
        .unwrap();
    task.instructions = "Task text, verbatim.\n".into();
    task.outcomes = vec!["done".into(), "retry".into()];
    let agent = project.definitions.agents.get_mut("engineer").unwrap();
    agent.skills = vec!["rust".into(), "verification".into()];
    agent.instructions = "Agent text, separate.\n".into();
    agent.model = Some(unelos_core::AgentModel::Alias("opaque-model".into()));
    agent.mcps = vec!["opaque-mcp".into()];
    agent.tools = vec!["opaque-tool".into()];

    let inspection = inspect_task(&project, "implement").unwrap();
    let value = serde_json::to_value(&inspection).unwrap();
    assert_eq!(value["kind"], "task");
    assert_eq!(value["nodeId"], "implement");
    assert_eq!(value["taskDefinition"], "implement-plan");
    assert_eq!(value["agent"], "engineer");
    assert_eq!(value["execution"], "subagent");
    assert_eq!(value["taskInstructions"], "Task text, verbatim.\n");
    assert_eq!(value["agentInstructions"], "Agent text, separate.\n");
    assert_eq!(value["skills"][0]["id"], "verification");
    assert_eq!(value["skills"][1]["id"], "rust");
    assert_eq!(value["skills"].as_array().unwrap().len(), 2);
    for skill in &inspection.context.skills {
        let expected = project.project_root.join(".unelos/skills").join(&skill.id);
        assert_eq!(skill.directory, expected);
        assert_eq!(
            value["skills"][if skill.id == "verification" { 0 } else { 1 }]["directory"],
            expected.as_str()
        );
        assert_eq!(
            skill.content,
            fs::read_to_string(expected.join("SKILL.md")).unwrap()
        );
        assert_eq!(
            value["skills"][if skill.id == "verification" { 0 } else { 1 }]["content"],
            skill.content
        );
    }
    assert_eq!(value["model"], "opaque-model");
    assert_eq!(value["mcps"], serde_json::json!(["opaque-mcp"]));
    assert_eq!(value["tools"], serde_json::json!(["opaque-tool"]));
    assert_eq!(value["inputs"], serde_json::json!(["plan.md"]));
    assert_eq!(
        value["outputs"],
        serde_json::json!(["implementation-report.md"])
    );
    assert_eq!(value["outcomes"], serde_json::json!(["done", "retry"]));
    assert_eq!(
        value["completionInstructions"],
        "Read any existing declared outputs and update them in place, preserving relevant content. Produce every declared output before completing this current Task; report the changes to the controller. Do not edit the run ledger."
    );
    assert!(value.get("runId").is_none());
    assert!(value.get("taskId").is_none());
    assert_eq!(
        serde_json::to_vec(&inspection).unwrap(),
        serde_json::to_vec(&inspect_task(&project, "implement").unwrap()).unwrap()
    );

    let plan = serde_json::to_value(inspect_task(&project, "plan").unwrap()).unwrap();
    assert_eq!(plan["execution"], "main");

    let review = serde_json::to_value(inspect_task(&project, "review").unwrap()).unwrap();
    assert_eq!(
        review["outcomes"],
        serde_json::json!(["approved", "changes_requested"])
    );
}

#[test]
fn resolves_only_selected_skills_and_does_not_load_unused_agents_skills() {
    let mut project = fixture_project();
    project
        .definitions
        .agents
        .get_mut("engineer")
        .unwrap()
        .skills
        .clear();
    project.definitions.agents.insert(
        "unused".into(),
        unelos_core::AgentDefinition {
            instructions_file: None,
            instructions: String::new(),
            skills: vec!["missing".into()],
            mcps: vec![],
            tools: vec![],
            model: None,
        },
    );
    let value = serde_json::to_value(inspect_task(&project, "implement").unwrap()).unwrap();
    assert_eq!(value["skills"].as_array().unwrap().len(), 1);
    assert_eq!(value["skills"][0]["id"], "verification");
}

#[test]
fn selected_missing_skill_reports_id_and_native_expected_path() {
    let mut project = fixture_project();
    project
        .definitions
        .task_definitions
        .get_mut("implement-plan")
        .unwrap()
        .skills
        .push("missing-selected".into());
    let expected = project
        .project_root
        .join(".unelos/skills")
        .join("missing-selected")
        .join("SKILL.md");
    let error = inspect_task(&project, "implement").unwrap_err();
    assert!(error.to_string().contains("missing-selected"));
    assert!(error.to_string().contains(expected.as_str()));
    assert!(
        matches!(error, unelos_core::ContextError::Skill(SkillResolveError::Missing { id, path, .. }) if id == "missing-selected" && path == expected)
    );
}

#[test]
fn reports_unknown_unsupported_and_missing_references_without_panicking() {
    let mut project = fixture_project();
    let error = inspect_task(&project, "missing").unwrap_err();
    assert!(
        matches!(error, unelos_core::ContextError::Diagnostic { path, field, .. }
        if path == project.workflow_source && field == "nodes.missing")
    );
    for node in ["approve-plan", "done"] {
        let error = inspect_task(&project, node).unwrap_err();
        assert!(
            matches!(error, unelos_core::ContextError::Diagnostic { path, field, .. }
            if path == project.workflow_source && field == format!("nodes.{node}"))
        );
    }
    project.workflow.nodes.insert(
        "parallel".into(),
        NodeDefinition::Parallel(ParallelNode {
            tasks: vec![],
            inputs: vec![],
            outputs: vec![],
            next: Transition::Single("done".into()),
        }),
    );
    let error = inspect_task(&project, "parallel").unwrap_err();
    assert!(
        matches!(error, unelos_core::ContextError::Diagnostic { path, field, .. }
        if path == project.workflow_source && field == "nodes.parallel")
    );

    let mut project = fixture_project();
    if let NodeDefinition::Task(task) = project.workflow.nodes.get_mut("implement").unwrap() {
        task.task = "missing-task".into();
    }
    assert!(inspect_task(&project, "implement")
        .unwrap_err()
        .to_string()
        .contains("missing-task"));
    let mut project = fixture_project();
    if let NodeDefinition::Task(task) = project.workflow.nodes.get_mut("implement").unwrap() {
        task.agent = "missing-agent".into();
    }
    assert!(inspect_task(&project, "implement")
        .unwrap_err()
        .to_string()
        .contains("missing-agent"));
}

#[test]
fn builds_deterministic_runtime_packet_without_allocating_artifacts() {
    let (_temp, project) = runtime_project();
    let input = runtime_input(&project.project_root);
    let packet = build_task_packet(&project, input.clone()).unwrap();
    let value = serde_json::to_value(&packet).unwrap();
    assert_eq!(value["kind"], "task");
    assert_eq!(value["runId"], "run-1");
    assert_eq!(value["taskId"], "task-3");
    assert_eq!(value["nodeId"], "implement");
    assert_eq!(value["request"], input.request);
    assert_eq!(value["inputs"][0]["name"], "plan.md");
    assert_eq!(value["inputs"][0]["path"], input.inputs[0].path.as_str());
    assert_eq!(value["outputs"][0]["name"], "implementation-report.md");
    assert_eq!(value["outputs"][0]["path"], input.outputs[0].path.as_str());
    assert_eq!(value["feedback"]["sourceNodeId"], "review");
    assert_eq!(value["feedback"]["sourceExecutionId"], "task-2");
    assert_eq!(value["feedback"]["outcome"], "changes_requested");
    assert_eq!(value["feedback"]["reason"], "Café said \"redo\".\nAgain.");
    assert_eq!(value["feedback"]["artifacts"][0]["name"], "review.md");
    assert_eq!(
        value["feedback"]["artifacts"][0]["path"],
        input.feedback.as_ref().unwrap().artifacts[0].path.as_str()
    );
    assert_eq!(
        serde_json::to_vec(&packet).unwrap(),
        serde_json::to_vec(&build_task_packet(&project, input.clone()).unwrap()).unwrap()
    );
    assert!(!project.project_root.join(".unelos/runs").exists());
    assert!(!input.outputs[0].path.exists());

    let mut changed = input;
    changed.run_id = "run-2".into();
    changed.task_id = "task-9".into();
    changed.inputs[0].path = project.project_root.join("opaque/input");
    changed.outputs[0].path = project.project_root.join("opaque/output");
    let changed = serde_json::to_value(build_task_packet(&project, changed).unwrap()).unwrap();
    assert_eq!(changed["runId"], "run-2");
    assert_eq!(changed["taskId"], "task-9");
    assert_eq!(
        changed["inputs"][0]["path"],
        project.project_root.join("opaque/input").as_str()
    );
    assert_eq!(
        changed["outputs"][0]["path"],
        project.project_root.join("opaque/output").as_str()
    );
}

#[test]
fn validates_exact_bindings_and_serializes_them_in_declaration_order() {
    let (_temp, mut project) = runtime_project();
    let mut input = runtime_input(&project.project_root);
    input.node_id = "review".into();
    input.inputs = ["implementation-report.md", "plan.md"]
        .into_iter()
        .map(|name| ArtifactReference {
            name: name.into(),
            path: project.project_root.join(name),
        })
        .collect();
    input.outputs = vec![ArtifactReference {
        name: "review.md".into(),
        path: project.project_root.join("review.md"),
    }];
    let value = serde_json::to_value(build_task_packet(&project, input.clone()).unwrap()).unwrap();
    assert_eq!(value["inputs"][0]["name"], "plan.md");
    assert_eq!(value["inputs"][1]["name"], "implementation-report.md");

    for (bindings, side, name) in [
        (vec![], "inputs", "plan.md"),
        (
            vec![("plan.md", "one"), ("plan.md", "two")],
            "inputs",
            "plan.md",
        ),
        (
            vec![
                ("extra.md", "extra"),
                ("implementation-report.md", "report"),
            ],
            "inputs",
            "extra.md",
        ),
    ] {
        let mut invalid = input.clone();
        invalid.inputs = bindings
            .into_iter()
            .map(|(name, path)| ArtifactReference {
                name: name.into(),
                path: project.project_root.join(path),
            })
            .collect();
        let error = build_task_packet(&project, invalid)
            .unwrap_err()
            .to_string();
        assert!(
            error.contains("review") && error.contains(side) && error.contains(name),
            "{error}"
        );
    }
    for (bindings, side, name) in [
        (vec![], "outputs", "review.md"),
        (
            vec![("review.md", "one"), ("review.md", "two")],
            "outputs",
            "review.md",
        ),
        (vec![("extra.md", "extra")], "outputs", "extra.md"),
    ] {
        let mut invalid = input.clone();
        invalid.outputs = bindings
            .into_iter()
            .map(|(name, path)| ArtifactReference {
                name: name.into(),
                path: project.project_root.join(path),
            })
            .collect();
        let error = build_task_packet(&project, invalid)
            .unwrap_err()
            .to_string();
        assert!(
            error.contains("review") && error.contains(side) && error.contains(name),
            "{error}"
        );
    }

    let mut plan = runtime_input(&project.project_root);
    plan.node_id = "plan".into();
    plan.inputs.clear();
    plan.outputs.clear();
    if let NodeDefinition::Task(node) = project.workflow.nodes.get_mut("plan").unwrap() {
        node.inputs.clear();
        node.outputs.clear();
    }
    assert!(build_task_packet(&project, plan.clone()).is_ok());
    plan.inputs.push(ArtifactReference {
        name: "unexpected".into(),
        path: project.project_root.join("unexpected"),
    });
    let error = build_task_packet(&project, plan).unwrap_err().to_string();
    assert!(error.contains("plan") && error.contains("inputs") && error.contains("unexpected"));
}

#[test]
fn absent_feedback_and_reason_serialize_as_null() {
    let (_temp, project) = runtime_project();
    let mut input = runtime_input(&project.project_root);
    input.feedback = None;
    let value = serde_json::to_value(build_task_packet(&project, input.clone()).unwrap()).unwrap();
    assert_eq!(value["feedback"], serde_json::Value::Null);
    input.feedback = Some(TransitionFeedback {
        source_node_id: "gate".into(),
        source_execution_id: "opaque-id".into(),
        outcome: Some("approved".into()),
        reason: None,
        artifacts: vec![],
    });
    let value = serde_json::to_value(build_task_packet(&project, input).unwrap()).unwrap();
    assert_eq!(value["feedback"]["reason"], serde_json::Value::Null);
}

#[test]
fn feedback_outcome_accepts_old_string_null_and_missing_values() {
    let json = r#"{"sourceNodeId":"review","sourceExecutionId":"task-2","outcome":"changes_requested","reason":null,"artifacts":[]}"#;
    let feedback: TransitionFeedback = serde_json::from_str(json).unwrap();
    assert_eq!(feedback.outcome.as_deref(), Some("changes_requested"));

    for json in [
        r#"{"sourceNodeId":"review","sourceExecutionId":"task-2","outcome":null,"reason":null,"artifacts":[]}"#,
        r#"{"sourceNodeId":"review","sourceExecutionId":"task-2","reason":null,"artifacts":[]}"#,
    ] {
        let feedback: TransitionFeedback = serde_json::from_str(json).unwrap();
        assert_eq!(feedback.outcome, None);
    }
}
