use assert_fs::TempDir;
use camino::Utf8Path;
use std::fs;
use unelos_core::{inspect_node, inspect_task, load_project, ContextError, NodeDefinition};

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
