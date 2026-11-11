use camino::{Utf8Path, Utf8PathBuf};
use serde::{Deserialize, Serialize};
use serde_yaml_ng::Value;
use std::fs;

#[derive(Debug, Serialize, Deserialize)]
pub struct WorkflowSettings {
    pub name: String,
    pub description: Option<String>,
    pub start: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WorkflowRoute {
    pub outcome: Option<String>,
    pub target: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WorkflowWorker {
    pub title: String,
    pub agent: String,
}

#[derive(Debug, Serialize)]
pub struct WorkflowIssue {
    pub message: String,
    pub step: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowStep {
    pub id: String,
    #[serde(default)]
    pub original_id: Option<String>,
    #[serde(default)]
    pub task: Option<StepTask>,
    #[serde(default)]
    pub label: String,
    pub kind: String,
    #[serde(default)]
    pub title: Option<String>,
    pub agent: Option<String>,
    pub execution: Option<crate::Execution>,
    pub routes: Vec<WorkflowRoute>,
    #[serde(default)]
    pub workers: Vec<WorkflowWorker>,
    pub inputs: Vec<String>,
    pub outputs: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StepTask {
    pub source: String,
    pub id: Option<String>,
    pub description: Option<String>,
    pub instructions: String,
    pub skills: Vec<String>,
    pub outcomes: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WorkflowDraft {
    #[serde(flatten)]
    pub settings: WorkflowSettings,
    pub steps: Vec<WorkflowStep>,
}

#[derive(Debug, Serialize)]
pub struct WorkflowDocument {
    pub id: String,
    #[serde(flatten)]
    pub settings: WorkflowSettings,
    pub steps: Vec<WorkflowStep>,
    pub issues: Vec<WorkflowIssue>,
}

fn source(root: &Utf8Path, id: &str) -> Result<Utf8PathBuf, String> {
    if id.is_empty() || matches!(id, "." | "..") || id.contains(['/', '\\']) || Utf8Path::new(id).is_absolute() { return Err(format!("Invalid Workflow ID {id:?}")); }
    let authoring = crate::project::absolute_root(root).map_err(|e| e.to_string())?.join(".unelos");
    crate::project::bounded_path(&authoring, &Utf8PathBuf::from(format!("workflows/{id}.yaml")), &authoring, "workflow").map_err(|e| e.to_string())
}

fn issues(root: &Utf8Path, id: &str, text: Option<&str>) -> Vec<WorkflowIssue> {
    let result = crate::project::load_project_candidate(root, id, text)
        .map_err(|e| e.to_string())
        .and_then(|project| crate::validate_project(&project).map(|_| ()).map_err(|e| e.to_string()));
    let workflow = text.and_then(|text| crate::parse_workflow_definition(id, text).ok());
    result.err().into_iter().map(|message| {
        let step = workflow.as_ref().and_then(|workflow| {
            message.split(": ").find_map(|field| {
                let field = field.strip_prefix("nodes.")?;
                workflow.nodes.keys().filter(|id| field == id.as_str() || field.strip_prefix(id.as_str()).is_some_and(|rest| rest.starts_with('.') || rest.starts_with('[')))
                    .max_by_key(|id| id.len()).cloned()
            })
        });
        WorkflowIssue { message: user_message(&message), step }
    }).collect()
}

/// Drops the file path and YAML position from loader errors and capitalizes the rest.
fn user_message(message: &str) -> String {
    let message = message.split_once(".yaml: ").map_or(message, |(_, rest)| rest);
    let message = message.rfind(" at line ").map_or(message, |i| &message[..i]);
    let message = message.strip_prefix("nodes: ").unwrap_or(message);
    let message = message.replace("missing field `", "missing required field `");
    let mut chars = message.chars();
    chars.next().map_or(String::new(), |c| c.to_uppercase().chain(chars).collect::<String>() + if message.ends_with('.') { "" } else { "." })
}

#[cfg(test)]
mod tests {
    #[test]
    fn user_message_hides_paths_and_positions() {
        assert_eq!(
            super::user_message("/home/x/.unelos/workflows/a.yaml: nodes: missing field `agent` at line 5 column 3"),
            "Missing required field `agent`."
        );
        assert_eq!(super::user_message("start step missing"), "Start step missing.");
    }
}

fn title(task: &crate::TaskSelection) -> String {
    match task {
        crate::TaskSelection::Named(id) => id.clone(),
        crate::TaskSelection::Inline(task) => task.description.clone().unwrap_or_else(|| "Inline task".into()),
    }
}

fn routes(next: &crate::Transition) -> Vec<WorkflowRoute> {
    match next {
        crate::Transition::Single(target) => vec![WorkflowRoute { outcome: None, target: target.clone() }],
        crate::Transition::Outcomes(outcomes) => outcome_routes(outcomes),
    }
}

fn outcome_routes(outcomes: &indexmap::IndexMap<String, String>) -> Vec<WorkflowRoute> {
    outcomes.iter().map(|(outcome, target)| WorkflowRoute { outcome: Some(outcome.clone()), target: target.clone() }).collect()
}

fn document(root: &Utf8Path, id: &str, text: &str) -> Result<WorkflowDocument, String> {
    let workflow = crate::parse_workflow_definition(id, text).map_err(|e| e.to_string())?;
    let project = crate::project::load_project_candidate(root, id, Some(text)).ok();
    let steps = workflow.nodes.iter().map(|(id, node)| {
        let node = project.as_ref().and_then(|p| p.workflow.nodes.get(id)).unwrap_or(node);
        let mut step = WorkflowStep { id: id.clone(), original_id: Some(id.clone()), task: None, label: id.clone(), kind: String::new(), title: None, agent: None, execution: None, routes: vec![], workers: vec![], inputs: vec![], outputs: vec![] };
        step.kind = match node {
            crate::NodeDefinition::Task(task) => {
                let (source, task_id, definition) = match &task.task {
                    crate::TaskSelection::Named(id) => ("named", Some(id.clone()), project.as_ref().and_then(|p| p.definitions.task_definitions.get(id))),
                    crate::TaskSelection::Inline(definition) => ("inline", None, Some(definition)),
                };
                step.task = Some(StepTask { source: source.into(), id: task_id, description: definition.and_then(|d| d.description.clone()), instructions: definition.map(|d| d.instructions.clone()).unwrap_or_default(), skills: definition.map(|d| d.skills.clone()).unwrap_or_default(), outcomes: definition.map(|d| d.outcomes.clone()).unwrap_or_default() });
                step.title = Some(title(&task.task));
                step.agent = Some(task.agent.clone());
                step.execution = Some(task.execution);
                step.routes = routes(&task.next);
                step.inputs = task.inputs.clone();
                step.outputs = task.outputs.clone();
                "task"
            }
            crate::NodeDefinition::Parallel(parallel) => {
                step.workers = parallel.tasks.iter().map(|worker| WorkflowWorker { title: title(&worker.task), agent: worker.agent.clone() }).collect();
                step.routes = routes(&parallel.next);
                step.inputs = parallel.inputs.clone();
                step.outputs = parallel.outputs.clone();
                "parallel"
            }
            crate::NodeDefinition::HumanGate(gate) => {
                step.routes = outcome_routes(&gate.next);
                step.inputs = gate.inputs.clone();
                "human-gate"
            }
            crate::NodeDefinition::End => "end",
        }.into();
        step
    }).collect();
    Ok(WorkflowDocument { id: id.into(), settings: WorkflowSettings { name: workflow.name, description: workflow.description, start: workflow.start }, steps, issues: issues(root, id, Some(text)) })
}

pub fn get_workflow(root: &Utf8Path, id: &str) -> Result<WorkflowDocument, String> {
    let text = fs::read_to_string(source(root, id)?).map_err(|e| e.to_string())?;
    document(root, id, &text)
}

fn candidate(root: &Utf8Path, id: &str, draft: &WorkflowDraft) -> Result<(Utf8PathBuf, String), String> {
    let settings = &draft.settings;
    let source = source(root, id)?;
    let text = fs::read_to_string(&source).map_err(|e| e.to_string())?;
    let mut value: Value = serde_yaml_ng::from_str(&text).map_err(|e| e.to_string())?;
    let fields = value.as_mapping_mut().ok_or("Workflow must be a mapping")?;
    let old_nodes = fields.get("nodes").and_then(Value::as_mapping).cloned().unwrap_or_default();
    let loaded = document(root, id, &text).ok();
    let mut nodes = serde_yaml_ng::Mapping::new();
    for step in &draft.steps {
        let mut node = step.original_id.as_ref().and_then(|id| old_nodes.get(Value::String(id.clone()))).and_then(Value::as_mapping).cloned().unwrap_or_default();
        if matches!(step.kind.as_str(), "task" | "human-gate") {
            node.insert("type".into(), Value::String(step.kind.clone()));
            set_list(&mut node, "inputs", &step.inputs);
        }
        if step.kind == "parallel" && loaded.as_ref().and_then(|doc| doc.steps.iter().find(|s| s.original_id == step.original_id)).is_none_or(|old| old.inputs != step.inputs) {
            set_list(&mut node, "inputs", &step.inputs);
        }
        if step.kind == "task" {
            set_optional(&mut node, "agent", step.agent.as_ref().map(|s| Value::String(s.clone())));
            set_optional(&mut node, "execution", step.execution.map(|e| serde_yaml_ng::to_value(e).unwrap()));
            set_list(&mut node, "outputs", &step.outputs);
            if let Some(task) = &step.task {
                if task.source == "named" {
                    node.remove("taskDefinition");
                    set_optional(&mut node, "task", task.id.as_ref().map(|s| Value::String(s.clone())));
                } else {
                    node.remove("task");
                    let mut definition = node.get("taskDefinition").and_then(Value::as_mapping).cloned().unwrap_or_default();
                    let old_instructions = loaded.as_ref().and_then(|doc| doc.steps.iter().find(|s| s.original_id == step.original_id)).and_then(|s| s.task.as_ref()).map(|t| t.instructions.as_str());
                    if old_instructions != Some(task.instructions.as_str()) { definition.remove("instructionsFile"); }
                    if !definition.contains_key("instructionsFile") { definition.insert("instructions".into(), Value::String(task.instructions.clone())); }
                    definition.insert("skills".into(), serde_yaml_ng::to_value(&task.skills).unwrap());
                    definition.insert("outcomes".into(), serde_yaml_ng::to_value(&task.outcomes).unwrap());
                    set_optional(&mut definition, "description", task.description.as_ref().map(|s| Value::String(s.clone())));
                    node.insert("taskDefinition".into(), Value::Mapping(definition));
                }
            } else { node.remove("task"); node.remove("taskDefinition"); }
        }
        if step.kind != "end" {
            let next = if step.routes.is_empty() { None }
                else if step.kind != "human-gate" && step.routes.len() == 1 && step.routes[0].outcome.is_none() { Some(Value::String(step.routes[0].target.clone())) }
                else { Some(Value::Mapping(step.routes.iter().map(|r| (r.outcome.clone().map(Value::String).unwrap_or(Value::Null), Value::String(r.target.clone()))).collect())) };
            set_optional(&mut node, "next", next);
        }
        nodes.insert(Value::String(step.id.clone()), Value::Mapping(node));
    }
    fields.insert("nodes".into(), Value::Mapping(nodes));
    fields.insert(Value::String("name".into()), Value::String(settings.name.clone()));
    fields.insert(Value::String("description".into()), settings.description.clone().map(Value::String).unwrap_or(Value::Null));
    fields.insert(Value::String("start".into()), Value::String(settings.start.clone()));
    Ok((source, serde_yaml_ng::to_string(&value).map_err(|e| e.to_string())?))
}

fn set_optional(fields: &mut serde_yaml_ng::Mapping, key: &str, value: Option<Value>) {
    if let Some(value) = value { fields.insert(key.into(), value); } else { fields.remove(key); }
}

fn set_list(fields: &mut serde_yaml_ng::Mapping, key: &str, values: &[String]) {
    set_optional(fields, key, (!values.is_empty()).then(|| serde_yaml_ng::to_value(values).unwrap()));
}

fn step_rename_issue(root: &Utf8Path, id: &str, draft: &WorkflowDraft) -> Option<WorkflowIssue> {
    let step = draft.steps.iter().find(|step| step.original_id.as_ref().is_some_and(|old| old != &step.id))?;
    let result = crate::list_runs(root).map_err(|e| format!("Cannot safely rename: could not inspect stored runs: {e}"))
        // Finished runs keep the old id in their history; only in-progress runs need the step to exist.
        .and_then(|runs| if runs.iter().any(|run| run.workflow_name == id && !run.is_finished()) { Err("Finish or remove in-progress runs of this workflow before renaming a step".into()) } else { Ok(()) });
    result.err().map(|message| WorkflowIssue { message, step: Some(step.id.clone()) })
}

pub fn validate_workflow(root: &Utf8Path, id: &str, draft: &WorkflowDraft) -> Result<Vec<WorkflowIssue>, String> {
    let settings = &draft.settings;
    let (_, text) = candidate(root, id, draft)?;
    if let Some(issue) = step_rename_issue(root, id, draft) { return Ok(vec![issue]); }
    if let Err(message) = rename_target(root, id, &settings.name) { return Ok(vec![WorkflowIssue { message, step: None }]); }
    Ok(issues(root, &settings.name, Some(&text)))
}

fn rename_target(root: &Utf8Path, id: &str, name: &str) -> Result<Utf8PathBuf, String> {
    if name != id {
        if !crate::project::safe_id(name) { return Err("Name must contain 1–128 ASCII letters, digits, underscores or hyphens".into()); }
        let target = source(root, name)?;
        if target.try_exists().map_err(|e| e.to_string())? { return Err(format!("Workflow {name:?} already exists")); }
        // shortcut: run creation is not locked with authoring, add a shared lock before supporting concurrent edits/execution.
        let runs = crate::list_runs(root).map_err(|e| format!("Cannot safely rename: could not inspect stored runs: {e}"))?;
        // Finished runs keep the old workflow name; only in-progress runs need the file to stay put.
        if runs.iter().any(|run| run.workflow_name == id && !run.is_finished()) { return Err("Finish or remove in-progress runs of this workflow before renaming it".into()); }
        Ok(target)
    } else { source(root, id) }
}

pub fn save_workflow(root: &Utf8Path, id: &str, draft: &WorkflowDraft) -> Result<WorkflowDocument, String> {
    let settings = &draft.settings;
    let (source, text) = candidate(root, id, draft)?;
    if let Some(issue) = step_rename_issue(root, id, draft) { return Err(issue.message); }
    let target = rename_target(root, id, &settings.name)?;
    let document = document(root, &settings.name, &text)?;
    if !document.issues.is_empty() { return Err(document.issues.iter().map(|issue| issue.message.as_str()).collect::<Vec<_>>().join("\n")); }
    if target == source {
        crate::skill_document::atomic_write(&source, &text)?;
    } else {
        let staging = source.with_extension("unelos-rename");
        fs::OpenOptions::new().write(true).create_new(true).open(&staging).map_err(|e| e.to_string())?;
        let result = (|| {
            crate::skill_document::atomic_write(&staging, &text)?;
            // shortcut: a crash can leave both identities, use a journal if crash-recoverable renames are needed.
            // Publish without overwriting a target created after validation.
            fs::hard_link(&staging, &target).map_err(|e| e.to_string())?;
            if let Err(error) = fs::remove_file(&source) {
                fs::remove_file(&target).map_err(|rollback| format!("{error}; rollback: {rollback}"))?;
                return Err(error.to_string());
            }
            Ok(())
        })();
        let _ = fs::remove_file(staging);
        result?;
    }
    Ok(document)
}
