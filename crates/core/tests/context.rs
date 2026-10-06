use camino::Utf8Path;
use std::fs;
use unelos_core::{
    inspect_task, load_project, NodeDefinition, ParallelNode, SkillResolveError, Transition,
};

fn fixture_project() -> unelos_core::LoadedProject {
    let root = Utf8Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/project");
    load_project(&root, "implement-feature").unwrap()
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
    agent.model = Some("opaque-model".into());
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
        "Produce the declared outputs before completing this current Task."
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
