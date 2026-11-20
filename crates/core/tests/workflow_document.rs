use assert_fs::{fixture::PathCopy, TempDir};
use camino::{Utf8Path, Utf8PathBuf};
use std::fs;
use unelos_core::{
    get_workflow, save_workflow, start_workflow, validate_workflow, Execution, IssueSeverity,
    WorkflowDocument, WorkflowDraft,
};

fn fixture() -> (TempDir, Utf8PathBuf) {
    let temp = TempDir::new().unwrap();
    temp.copy_from(
        Utf8Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/project"),
        &["**/*"],
    )
    .unwrap();
    let root = Utf8PathBuf::from_path_buf(temp.path().to_owned()).unwrap();
    (temp, root)
}

#[test]
fn creates_workflows_with_free_ids_and_requires_an_agent_before_save() {
    let (_temp, root) = fixture();
    let id = unelos_core::create_workflow(&root).unwrap();
    assert_eq!(id, "new-workflow");
    let text = fs::read_to_string(root.join(".unelos/workflows/new-workflow.yaml")).unwrap();
    unelos_core::parse_workflow_definition(&id, &text).unwrap();
    let document = get_workflow(&root, &id).unwrap();
    assert_eq!(document.issues.len(), 1);
    assert!(document.issues[0].message.contains("Pick an agent"));
    assert_eq!(document.issues[0].step.as_deref(), Some("new-task"));
    let project = unelos_core::load_project(&root, &id).unwrap();
    assert!(unelos_core::validate_project(&project).is_err());
    assert!(unelos_core::workflow_summaries(&root).is_ok());
    let mut value = draft(document);
    assert!(save_workflow(&root, &id, &value).is_err());
    value
        .steps
        .iter_mut()
        .find(|step| step.id == "new-task")
        .unwrap()
        .agent = Some("engineer".into());
    assert!(save_workflow(&root, &id, &value)
        .unwrap()
        .issues
        .iter()
        .all(|issue| issue.severity == IssueSeverity::Warning));
    assert_eq!(
        unelos_core::create_workflow(&root).unwrap(),
        "new-workflow-2"
    );
    assert_eq!(
        unelos_core::create_workflow(&root).unwrap(),
        "new-workflow-3"
    );
    assert_eq!(
        fs::read_to_string(root.join(".unelos/workflows/new-workflow-2.yaml"))
            .unwrap()
            .lines()
            .next(),
        Some("name: new-workflow-2")
    );
}

fn parallel_fixture() -> (TempDir, Utf8PathBuf) {
    let (temp, root) = fixture();
    fs::write(
        root.join(".unelos/workflows/implement-feature.yaml"),
        r#"name: implement-feature
description: null
start: workers
nodes:
  workers:
    type: parallel
    tasks:
      - task: implement-plan
        agent: engineer
      - taskDefinition:
          description: Review work
          instructions: Review carefully
          skills: [code-review]
        agent: reviewer
    outputs: [report.md]
    next: done
  done:
    type: end
"#,
    )
    .unwrap();
    (temp, root)
}

#[test]
fn parallel_roundtrip_preserves_yaml() {
    let (_temp, root) = parallel_fixture();
    let path = root.join(".unelos/workflows/implement-feature.yaml");
    let before: serde_yaml_ng::Value =
        serde_yaml_ng::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    let doc = get_workflow(&root, "implement-feature").unwrap();
    assert!(!doc.steps[0].workers[0].task.instructions.is_empty());
    save_workflow(&root, "implement-feature", &draft(doc)).unwrap();
    let after: serde_yaml_ng::Value =
        serde_yaml_ng::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(before, after);
}

#[test]
fn parallel_worker_edits_and_outputs_persist() {
    let (_temp, root) = parallel_fixture();
    let mut value = draft(get_workflow(&root, "implement-feature").unwrap());
    let worker = &mut value.steps[0].workers[1];
    worker.task.description = Some("Changed description".into());
    worker.task.instructions = "Changed instructions".into();
    worker.task.skills = vec!["rust".into()];
    worker.agent = "engineer".into();
    value.steps[0].outputs = vec!["changed.md".into()];
    save_workflow(&root, "implement-feature", &value).unwrap();
    let loaded = get_workflow(&root, "implement-feature").unwrap();
    assert_eq!(
        serde_json::to_value(&loaded.steps[0].workers).unwrap(),
        serde_json::to_value(&value.steps[0].workers).unwrap()
    );
    assert_eq!(loaded.steps[0].outputs, ["changed.md"]);
}

#[test]
fn parallel_worker_instructions_file_is_preserved_until_edited() {
    let (_temp, root) = parallel_fixture();
    let path = root.join(".unelos/workflows/implement-feature.yaml");
    fs::write(root.join(".unelos/workflows/review.md"), "Review carefully").unwrap();
    fs::write(
        &path,
        fs::read_to_string(&path).unwrap().replace(
            "instructions: Review carefully",
            "instructionsFile: review.md",
        ),
    )
    .unwrap();
    let mut value = draft(get_workflow(&root, "implement-feature").unwrap());
    value.steps[0].workers[1].task.description = Some("Updated".into());
    save_workflow(&root, "implement-feature", &value).unwrap();
    assert!(fs::read_to_string(&path)
        .unwrap()
        .contains("instructionsFile: review.md"));
    value.steps[0].workers[1].task.instructions = "New instructions".into();
    save_workflow(&root, "implement-feature", &value).unwrap();
    assert!(!fs::read_to_string(&path)
        .unwrap()
        .contains("instructionsFile"));
    assert_eq!(
        get_workflow(&root, "implement-feature").unwrap().steps[0].workers[1]
            .task
            .instructions,
        "New instructions"
    );
}

#[test]
fn parallel_workers_can_switch_to_named_and_be_added_or_removed() {
    let (_temp, root) = parallel_fixture();
    let mut value = draft(get_workflow(&root, "implement-feature").unwrap());
    value.steps[0].workers[1].task.source = "named".into();
    value.steps[0].workers[1].task.id = Some("implement-plan".into());
    let saved = save_workflow(&root, "implement-feature", &value).unwrap();
    assert_eq!(
        saved.steps[0].workers[1].task.id.as_deref(),
        Some("implement-plan")
    );
    let text = fs::read_to_string(root.join(".unelos/workflows/implement-feature.yaml")).unwrap();
    assert!(!text.contains("taskDefinition"));
    let mut value = draft(saved);
    value.steps[0].workers.push(unelos_core::WorkflowWorker {
        task: unelos_core::StepTask {
            source: "inline".into(),
            id: None,
            description: None,
            instructions: "Another task".into(),
            skills: vec![],
            outcomes: vec![],
        },
        agent: "engineer".into(),
    });
    assert_eq!(
        save_workflow(&root, "implement-feature", &value)
            .unwrap()
            .steps[0]
            .workers
            .len(),
        3
    );
    value.steps[0].workers.remove(0);
    assert_eq!(
        save_workflow(&root, "implement-feature", &value)
            .unwrap()
            .steps[0]
            .workers
            .len(),
        2
    );
}

#[test]
fn rich_steps_preserve_authored_order_through_save() {
    let (_temp, root) = fixture();
    let path = root.join(".unelos/workflows/implement-feature.yaml");
    let text = fs::read_to_string(&path)
        .unwrap()
        .replace("\r\n", "\n")
        .replace(
            "    type: human-gate\n",
            "    type: human-gate\n    description: Approve the plan?\n",
        )
        .replace(
            "      approved: implement\n      rejected: plan",
            "      rejected: plan\n      approved: implement",
        )
        .replace(
            "      approved: done\n      changes_requested: implement",
            "      changes_requested: implement\n      approved: done",
        );
    fs::write(&path, text).unwrap();
    let doc = get_workflow(&root, "implement-feature").unwrap();
    assert_eq!(
        doc.steps
            .iter()
            .map(|step| step.id.as_str())
            .collect::<Vec<_>>(),
        ["plan", "approve-plan", "implement", "review", "done"]
    );
    let saved = save_workflow(&root, "implement-feature", &draft(doc)).unwrap();
    let loaded = get_workflow(&root, "implement-feature").unwrap();
    assert_eq!(
        serde_json::to_value(&saved.steps).unwrap(),
        serde_json::to_value(&loaded.steps).unwrap()
    );
    assert_eq!(loaded.steps[0].title.as_deref(), Some("create-plan"));
    assert_eq!(loaded.steps[0].routes[0].outcome, None);
    assert_eq!(loaded.steps[0].routes[0].target, "approve-plan");
    assert_eq!(loaded.steps[1].title.as_deref(), Some("Approve the plan?"));
    assert_eq!(
        loaded.steps[1].routes[0].outcome.as_deref(),
        Some("rejected")
    );
    assert_eq!(
        loaded.steps[3].routes[0].outcome.as_deref(),
        Some("changes_requested")
    );
    assert!(loaded.steps[4].routes.is_empty());
}

#[test]
fn parallel_workers_inline_titles_and_scoped_issues() {
    let (_temp, root) = fixture();
    let path = root.join(".unelos/workflows/implement-feature.yaml");
    fs::write(
        &path,
        r#"name: implement-feature
start: review.extra
nodes:
  review.extra:
    taskDefinition:
      description: Inline description
      instructions: Do work
    agent: missing
    next: workers
  workers:
    type: parallel
    tasks:
      - task: implement-plan
        agent: engineer
      - taskDefinition:
          instructions: Do work
        agent: planner
    inputs: [plan.md]
    outputs: [report.md]
    next: done
  done:
    type: end
"#,
    )
    .unwrap();
    let doc = get_workflow(&root, "implement-feature").unwrap();
    assert_eq!(doc.steps[0].title.as_deref(), Some("Inline description"));
    assert_eq!(
        doc.steps[1].workers[0].task.id.as_deref(),
        Some("implement-plan")
    );
    assert_eq!(doc.steps[1].workers[1].task.source, "inline");
    assert_eq!(doc.steps[1].workers[1].agent, "planner");
    assert_eq!(doc.steps[1].inputs, ["plan.md"]);
    assert_eq!(doc.steps[1].outputs, ["report.md"]);
    assert_eq!(doc.issues[0].step.as_deref(), Some("review.extra"));
    let mut settings = draft(doc);
    settings.settings.start = "missing".into();
    assert!(
        validate_workflow(&root, "implement-feature", &settings).unwrap()[0]
            .step
            .is_none()
    );
}

#[test]
fn settings_roundtrip_preserves_nonsettings_and_bridge_shape() {
    let (_temp, root) = fixture();
    let id = "implement-feature";
    let path = root.join(format!(".unelos/workflows/{id}.yaml"));
    let before: serde_yaml_ng::Value =
        serde_yaml_ng::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    let doc = get_workflow(&root, id).unwrap();
    let mut settings = draft(doc);
    settings.settings.description = Some(String::new());
    assert!(validate_workflow(&root, id, &settings).unwrap().is_empty());
    let saved = save_workflow(&root, id, &settings).unwrap();
    assert_eq!(saved.settings.description.as_deref(), Some(""));
    assert_eq!(
        get_workflow(&root, id)
            .unwrap()
            .settings
            .description
            .as_deref(),
        Some("")
    );
    let after: serde_yaml_ng::Value =
        serde_yaml_ng::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    for (key, value) in before.as_mapping().unwrap() {
        if !matches!(
            key.as_str(),
            Some("name" | "description" | "start" | "nodes")
        ) {
            assert_eq!(&after[key], value);
        }
    }
    let json = serde_json::to_value(saved).unwrap();
    assert_eq!(json["id"], id);
    assert!(json["name"].is_string());
    assert!(json["steps"][0]["id"].is_string());
    assert!(json["steps"][0]["label"].is_string());
    assert!(json["issues"].as_array().unwrap().is_empty());
    assert!(json.get("settings").is_none());
}

#[test]
fn invalid_candidate_and_existing_invalid_graph_never_write() {
    let (_temp, root) = fixture();
    let id = "implement-feature";
    let path = root.join(format!(".unelos/workflows/{id}.yaml"));
    let mut settings = draft(get_workflow(&root, id).unwrap());
    let original = fs::read(&path).unwrap();
    settings.settings.start = "missing".into();
    assert!(!validate_workflow(&root, id, &settings).unwrap().is_empty());
    assert!(save_workflow(&root, id, &settings).is_err());
    assert_eq!(fs::read(&path).unwrap(), original);
    settings.settings.start = "plan".into();
    let text = String::from_utf8(original)
        .unwrap()
        .replace("start: plan", "start: missing");
    fs::write(&path, &text).unwrap();
    let doc = get_workflow(&root, id).unwrap();
    assert!(!doc.issues.is_empty());
    assert!(save_workflow(&root, id, &draft(doc)).is_err());
    assert_eq!(fs::read_to_string(&path).unwrap(), text);
    settings.settings.name = "../different".into();
    assert!(save_workflow(&root, id, &settings).is_err());
    assert_eq!(fs::read_to_string(&path).unwrap(), text);
    assert!(get_workflow(&root, "../implement-feature").is_err());
    assert!(save_workflow(&root, "../implement-feature", &settings).is_err());
}

#[test]
fn rename_preserves_nodes_and_never_clobbers() {
    let (_temp, root) = fixture();
    let id = "implement-feature";
    let old = root.join(format!(".unelos/workflows/{id}.yaml"));
    let original = fs::read(&old).unwrap();
    let before = unelos_core::load_project(&root, id).unwrap().workflow;
    let mut settings = draft(get_workflow(&root, id).unwrap());
    settings.settings.name = "renamed".into();
    let new = root.join(".unelos/workflows/renamed.yaml");
    fs::write(&new, "existing").unwrap();
    assert!(save_workflow(&root, id, &settings).is_err());
    assert_eq!(fs::read_to_string(&new).unwrap(), "existing");
    assert_eq!(fs::read(&old).unwrap(), original);
    fs::remove_file(&new).unwrap();
    let staging_temp = old.with_extension("unelos-tmp");
    fs::write(&staging_temp, "occupied").unwrap();
    assert!(save_workflow(&root, id, &settings).is_err());
    assert_eq!(fs::read(&old).unwrap(), original);
    assert!(!new.exists());
    assert!(!old.with_extension("unelos-rename").exists());
    fs::remove_file(staging_temp).unwrap();
    assert!(validate_workflow(&root, id, &settings).unwrap().is_empty());
    let saved = save_workflow(&root, id, &settings).unwrap();
    assert_eq!(saved.id, "renamed");
    assert!(!old.exists());
    let after = unelos_core::load_project(&root, "renamed")
        .unwrap()
        .workflow;
    assert_eq!(after.name, "renamed");
    assert_eq!(before.nodes, after.nodes);
}

#[test]
fn stored_run_blocks_rename_but_not_metadata_edits_and_bad_store_fails_closed() {
    let (_temp, root) = fixture();
    let id = "implement-feature";
    let project = unelos_core::load_project(&root, id).unwrap();
    unelos_core::start(&project, "existing-run", "request").unwrap();
    let path = root.join(format!(".unelos/workflows/{id}.yaml"));
    let original = fs::read(&path).unwrap();
    let state_path = root.join(".unelos/runs/existing-run/state.json");
    let state = fs::read(&state_path).unwrap();
    let mut settings = draft(get_workflow(&root, id).unwrap());
    settings.settings.name = "renamed".into();
    assert!(save_workflow(&root, id, &settings)
        .unwrap_err()
        .contains("in-progress runs"));
    assert_eq!(fs::read(&path).unwrap(), original);
    assert_eq!(fs::read(&state_path).unwrap(), state);
    fs::write(&state_path, "bad json").unwrap();
    assert!(save_workflow(&root, id, &settings)
        .unwrap_err()
        .contains("could not inspect"));
    settings.settings.name = id.into();
    settings.settings.description = None;
    assert!(save_workflow(&root, id, &settings).is_ok());
    // A finished run no longer blocks the workflow rename.
    let mut finished: serde_json::Value = serde_json::from_slice(&state).unwrap();
    finished["current"] = serde_json::json!({"kind": "finished", "nodeId": "done"});
    fs::write(&state_path, serde_json::to_vec(&finished).unwrap()).unwrap();
    settings.settings.name = "renamed".into();
    assert!(save_workflow(&root, id, &settings).is_ok());
}

#[test]
fn unchanged_historical_name_is_accepted() {
    let (_temp, root) = fixture();
    let old = root.join(".unelos/workflows/implement-feature.yaml");
    let new = root.join(".unelos/workflows/historical.name.yaml");
    let text = fs::read_to_string(&old)
        .unwrap()
        .replace("name: implement-feature", "name: historical.name");
    fs::write(&new, text).unwrap();
    let settings = draft(get_workflow(&root, "historical.name").unwrap());
    assert!(save_workflow(&root, "historical.name", &settings).is_ok());
}

#[cfg(unix)]
#[test]
fn symlink_escape_and_atomic_write_failure_preserve_source() {
    let (_temp, root) = fixture();
    let outside = TempDir::new().unwrap();
    let id = "implement-feature";
    let path = root.join(format!(".unelos/workflows/{id}.yaml"));
    let original = fs::read(&path).unwrap();
    let settings = draft(get_workflow(&root, id).unwrap());
    fs::write(path.with_extension("unelos-tmp"), "occupied").unwrap();
    assert!(save_workflow(&root, id, &settings).is_err());
    assert_eq!(fs::read(&path).unwrap(), original);
    fs::remove_file(path.with_extension("unelos-tmp")).unwrap();
    fs::write(outside.path().join("outside.yaml"), &original).unwrap();
    fs::remove_file(&path).unwrap();
    std::os::unix::fs::symlink(outside.path().join("outside.yaml"), &path).unwrap();
    assert!(save_workflow(&root, id, &settings).is_err());
    assert_eq!(
        fs::read(outside.path().join("outside.yaml")).unwrap(),
        original
    );
}

fn draft(doc: WorkflowDocument) -> WorkflowDraft {
    WorkflowDraft {
        settings: doc.settings,
        steps: doc.steps,
    }
}

#[test]
fn main_agent_settings_warn_without_blocking_save_or_start() {
    let (_temp, root) = fixture();
    let id = "implement-feature";
    let initial = get_workflow(&root, id).unwrap();
    assert!(initial.issues.is_empty());
    let step = initial
        .steps
        .iter()
        .find(|step| step.kind == "task")
        .unwrap();
    let agent = root
        .join(".unelos")
        .join("agents")
        .join(step.agent.as_ref().unwrap())
        .join("agent.yaml");
    let original = fs::read_to_string(&agent).unwrap();
    let mut yaml: serde_yaml_ng::Value = serde_yaml_ng::from_str(&original).unwrap();
    yaml["model"] = "smart".into();
    fs::write(agent, serde_yaml_ng::to_string(&yaml).unwrap()).unwrap();
    let mut draft = draft(initial);
    let first = draft
        .steps
        .iter_mut()
        .find(|step| step.kind == "task")
        .unwrap();
    first.execution = Some(Execution::Main);
    let step_id = first.id.clone();
    let issues = validate_workflow(&root, id, &draft).unwrap();
    assert!(issues
        .iter()
        .any(|issue| issue.severity == IssueSeverity::Warning
            && issue.step.as_deref() == Some(&step_id)
            && issue.message.contains("sets model")));
    assert!(save_workflow(&root, id, &draft)
        .unwrap()
        .issues
        .iter()
        .any(|issue| issue.severity == IssueSeverity::Warning));
    start_workflow(&root, id, Some("warning-run")).unwrap();
}

fn workflow_call_fixture() -> (TempDir, Utf8PathBuf) {
    let (temp, root) = fixture();
    fs::write(
        root.join(".unelos/workflows/react-component.yaml"),
        r#"name: react-component
start: build
inputs: [brief.md]
nodes:
  build:
    agent: engineer
    taskDefinition: {instructions: Build it.}
    inputs: [brief.md]
    outputs: [report.md]
    next: done
  done: {type: end}
"#,
    )
    .unwrap();
    fs::write(
        root.join(".unelos/workflows/composite.yaml"),
        r#"name: composite
start: frontend
inputs: [plan.md]
nodes:
  frontend:
    type: workflow
    workflow: react-component
    inputs: {brief.md: plan.md}
    outputs: {report.md: frontend-report.md}
    next: done
  done: {type: end}
"#,
    )
    .unwrap();
    (temp, root)
}

#[test]
fn workflow_step_round_trips() {
    let (_temp, root) = workflow_call_fixture();
    let doc = get_workflow(&root, "composite").unwrap();
    let json = serde_json::to_value(&doc).unwrap();
    assert_eq!(json["inputs"], serde_json::json!(["plan.md"]));
    assert_eq!(
        json["steps"][0]["call"],
        serde_json::json!({
            "workflow": "react-component",
            "inputs": {"brief.md": "plan.md"},
            "outputs": {"report.md": "frontend-report.md"}
        })
    );
    assert_eq!(doc.steps[0].kind, "workflow");
    assert_eq!(doc.steps[0].inputs, ["plan.md"]);
    assert_eq!(doc.steps[0].outputs, ["frontend-report.md"]);
    let mut json = serde_json::to_value(draft(doc)).unwrap();
    json["inputs"] = serde_json::json!(["brief.md"]);
    json["steps"][0]["call"]["inputs"]["brief.md"] = "brief.md".into();
    let value: WorkflowDraft = serde_json::from_value(json).unwrap();
    let saved = save_workflow(&root, "composite", &value).unwrap();
    assert!(saved
        .issues
        .iter()
        .all(|issue| issue.severity == IssueSeverity::Warning));
    let text = fs::read_to_string(root.join(".unelos/workflows/composite.yaml")).unwrap();
    let parsed = unelos_core::parse_workflow_definition("composite", &text).unwrap();
    assert_eq!(parsed.inputs, ["brief.md"]);
    let unelos_core::NodeDefinition::Workflow(call) = &parsed.nodes["frontend"] else {
        panic!("expected workflow call");
    };
    assert_eq!(call.workflow, "react-component");
    assert_eq!(call.inputs["brief.md"], "brief.md");
    assert_eq!(call.outputs["report.md"], "frontend-report.md");
    assert_eq!(call.next, unelos_core::Transition::Single("done".into()));
    let loaded = get_workflow(&root, "composite").unwrap();
    assert_eq!(loaded.steps[0].inputs, ["brief.md"]);
    assert_eq!(loaded.steps[0].outputs, ["frontend-report.md"]);

    let mut json = serde_json::to_value(draft(loaded)).unwrap();
    json["steps"][0]["routes"] = serde_json::json!([{"outcome":"done","target":"done"}]);
    json["steps"][0]["call"]["outputs"] = serde_json::json!({});
    save_workflow(&root, "composite", &serde_json::from_value(json).unwrap()).unwrap();
    let text = fs::read_to_string(root.join(".unelos/workflows/composite.yaml")).unwrap();
    let yaml: serde_yaml_ng::Value = serde_yaml_ng::from_str(&text).unwrap();
    assert!(yaml["nodes"]["frontend"].get("outputs").is_none());
    assert_eq!(yaml["nodes"]["frontend"]["next"]["done"], "done");
}

#[test]
fn workflow_step_issue_attached() {
    let (_temp, root) = workflow_call_fixture();
    let mut json = serde_json::to_value(draft(get_workflow(&root, "composite").unwrap())).unwrap();
    json["steps"][0]["call"] = serde_json::json!({
        "workflow": "unknown-child", "inputs": {}, "outputs": {}
    });
    let issues =
        validate_workflow(&root, "composite", &serde_json::from_value(json).unwrap()).unwrap();
    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].step.as_deref(), Some("frontend"));
    assert!(issues[0].message.contains("unknown-child"));
}

#[test]
fn workflow_document_preserves_authored_edges_into_calls() {
    let (_temp, root) = workflow_call_fixture();
    let path = root.join(".unelos/workflows/composite.yaml");
    let text = fs::read_to_string(&path).unwrap().replace(
        "start: frontend",
        "start: review",
    ).replace(
        "nodes:\n",
        "nodes:\n  review:\n    type: human-gate\n    next: {approved: frontend, rejected: frontend}\n",
    );
    fs::write(&path, text).unwrap();
    let doc = get_workflow(&root, "composite").unwrap();
    assert_eq!(doc.steps[0].routes[0].target, "frontend");
    save_workflow(&root, "composite", &draft(doc)).unwrap();
    let parsed =
        unelos_core::parse_workflow_definition("composite", &fs::read_to_string(path).unwrap())
            .unwrap();
    let unelos_core::NodeDefinition::HumanGate(gate) = &parsed.nodes["review"] else {
        panic!("expected gate");
    };
    assert_eq!(gate.next["approved"], "frontend");
}

#[test]
fn existing_workflow_unchanged() {
    let (_temp, root) = fixture();
    let example = Utf8Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/superpowers-dev");
    let target = assert_fs::fixture::ChildPath::new(root.join(".unelos"));
    target.copy_from(&example, &["**/*"]).unwrap();
    let path = root.join(".unelos/workflows/superpowers-dev.yaml");
    let before = unelos_core::parse_workflow_definition(
        "superpowers-dev",
        &fs::read_to_string(&path).unwrap(),
    )
    .unwrap();
    let doc = get_workflow(&root, "superpowers-dev").unwrap();
    save_workflow(&root, "superpowers-dev", &draft(doc)).unwrap();
    let after = unelos_core::parse_workflow_definition(
        "superpowers-dev",
        &fs::read_to_string(path).unwrap(),
    )
    .unwrap();
    assert_eq!(before, after);
}

#[test]
fn insert_inline_task_and_rename_step_roundtrip() {
    let (_temp, root) = fixture();
    let id = "implement-feature";
    let mut value = draft(get_workflow(&root, id).unwrap());
    let mut inserted: unelos_core::WorkflowStep = serde_json::from_value(serde_json::json!({
        "id": "check", "kind": "task", "agent": "planner", "execution": "main",
        "routes": [{"outcome": null, "target": "approve-plan"}], "inputs": [], "outputs": [],
        "task": {"source": "inline", "id": null, "description": "Check plan", "instructions": "Check the plan", "skills": [], "outcomes": []}
    })).unwrap();
    assert!(inserted.original_id.is_none());
    inserted.label = "ignored".into();
    value.steps[0].routes[0].target = "check".into();
    value.steps.insert(1, inserted);
    save_workflow(&root, id, &value).unwrap();
    let doc = get_workflow(&root, id).unwrap();
    assert_eq!(
        doc.steps.iter().map(|s| s.id.as_str()).collect::<Vec<_>>(),
        [
            "plan",
            "check",
            "approve-plan",
            "implement",
            "review",
            "done"
        ]
    );
    assert_eq!(doc.steps[0].routes[0].target, "check");
    assert_eq!(doc.steps[1].routes[0].target, "approve-plan");
    assert_eq!(doc.steps[1].original_id.as_deref(), Some("check"));
    assert_eq!(
        doc.steps[1].task.as_ref().unwrap().instructions,
        "Check the plan"
    );
    let mut value = draft(doc);
    value.steps[1].id = "verify".into();
    value.steps[0].routes[0].target = "verify".into();
    assert!(validate_workflow(&root, id, &value).unwrap().is_empty());
    let saved = save_workflow(&root, id, &value).unwrap();
    assert_eq!(saved.steps[1].original_id.as_deref(), Some("verify"));
}

#[test]
fn named_tasks_resolve_and_incomplete_tasks_return_issues() {
    let (_temp, root) = fixture();
    let id = "implement-feature";
    let mut value = draft(get_workflow(&root, id).unwrap());
    let task = value.steps[3].task.as_ref().unwrap();
    assert_eq!(task.source, "named");
    assert_eq!(task.id.as_deref(), Some("review-implementation"));
    assert_eq!(task.outcomes, ["approved", "changes_requested"]);
    assert!(!task.instructions.is_empty());
    value.steps[0].agent = None;
    assert!(!validate_workflow(&root, id, &value).unwrap().is_empty());
    value.steps[0].agent = Some("planner".into());
    value.steps[0].routes.clear();
    assert!(!validate_workflow(&root, id, &value).unwrap().is_empty());
}

#[test]
fn step_rename_is_blocked_by_stored_runs() {
    let (_temp, root) = fixture();
    let id = "implement-feature";
    unelos_core::start(
        &unelos_core::load_project(&root, id).unwrap(),
        "existing-run",
        "request",
    )
    .unwrap();
    let mut value = draft(get_workflow(&root, id).unwrap());
    value.steps[0].id = "planning".into();
    value.settings.start = "planning".into();
    value.steps[1].routes[1].target = "planning".into();
    let path = root.join(".unelos/workflows/implement-feature.yaml");
    let original = fs::read(&path).unwrap();
    let issues = validate_workflow(&root, id, &value).unwrap();
    assert_eq!(issues[0].step.as_deref(), Some("planning"));
    assert!(issues[0].message.contains("in-progress runs"));
    assert!(save_workflow(&root, id, &value)
        .unwrap_err()
        .contains("in-progress runs"));
    assert_eq!(fs::read(&path).unwrap(), original);
    // A finished run no longer blocks step renames.
    let state_path = root.join(".unelos/runs/existing-run/state.json");
    let mut state: serde_json::Value =
        serde_json::from_slice(&fs::read(&state_path).unwrap()).unwrap();
    state["current"] = serde_json::json!({"kind": "finished", "nodeId": "done"});
    fs::write(&state_path, serde_json::to_vec(&state).unwrap()).unwrap();
    assert!(validate_workflow(&root, id, &value).unwrap().is_empty());
    assert!(save_workflow(&root, id, &value).is_ok());
}

#[test]
fn parallel_and_end_raw_yaml_are_preserved_except_next() {
    let (_temp, root) = fixture();
    let path = root.join(".unelos/workflows/implement-feature.yaml");
    fs::write(&path, "name: implement-feature\nstart: workers\nnodes:\n  workers:\n    type: parallel\n    tasks:\n      - taskDefinition:\n          instructions: Work\n        agent: engineer\n    next: done\n  done:\n    type: end\n").unwrap();
    let before: serde_yaml_ng::Value =
        serde_yaml_ng::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    let mut value = draft(get_workflow(&root, "implement-feature").unwrap());
    value.steps[0].routes[0].target = "finish".into();
    value.steps[1].id = "finish".into();
    let saved = save_workflow(&root, "implement-feature", &value).unwrap();
    assert_eq!(saved.steps[0].routes[0].target, "finish");
    let after: serde_yaml_ng::Value =
        serde_yaml_ng::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    let mut expected = before["nodes"]["workers"].clone();
    expected["next"] = "finish".into();
    assert_eq!(after["nodes"]["workers"], expected);
    assert_eq!(after["nodes"]["finish"], before["nodes"]["done"]);
}

#[test]
fn removed_producer_input_is_removed_from_parallel_on_save() {
    let (_temp, root) = fixture();
    let id = "implement-feature";
    let path = root.join(format!(".unelos/workflows/{id}.yaml"));
    fs::write(&path, "name: implement-feature\nstart: plan\nnodes:\n  plan:\n    task: create-plan\n    agent: planner\n    outputs: [plan.md]\n    next: workers\n  workers:\n    type: parallel\n    tasks:\n      - task: implement-plan\n        agent: engineer\n    inputs: [plan.md]\n    outputs: [report.md]\n    next: done\n  done:\n    type: end\n").unwrap();
    let before: serde_yaml_ng::Value =
        serde_yaml_ng::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    let mut value = draft(get_workflow(&root, id).unwrap());
    value.steps.remove(0);
    value.settings.start = "workers".into();
    value.steps[0].inputs.clear();
    assert!(validate_workflow(&root, id, &value).unwrap().is_empty());
    save_workflow(&root, id, &value).unwrap();
    assert!(get_workflow(&root, id).unwrap().steps[0].inputs.is_empty());
    let after: serde_yaml_ng::Value =
        serde_yaml_ng::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    let mut expected = before["nodes"]["workers"].clone();
    expected.as_mapping_mut().unwrap().remove("inputs");
    assert_eq!(after["nodes"]["workers"], expected);
}
