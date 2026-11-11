use camino::{Utf8Path, Utf8PathBuf};
use serde::{Deserialize, Serialize};
use std::{collections::{BTreeMap, BTreeSet}, fs};
use crate::TaskDefinition;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskDocument {
    pub id: String,
    pub instructions: String,
    pub skills: Vec<String>,
    pub description: Option<String>,
    pub outcomes: Vec<String>,
}

fn paths(root: &Utf8Path, id: &str) -> Result<(Utf8PathBuf, Utf8PathBuf), String> {
    if !crate::project::safe_id(id) { return Err(format!("Invalid Task ID {id:?}")); }
    let source = crate::project::bounded_path(&root, &Utf8PathBuf::from(format!("tasks/{id}/task.yaml")), &root, "task").map_err(|e| e.to_string())?;
    Ok((root.to_owned(), source))
}

fn load(root: &Utf8Path, source: &Utf8Path) -> Result<(TaskDefinition, BTreeMap<Utf8PathBuf, std::collections::BTreeSet<Utf8PathBuf>>), String> {
    let text = fs::read_to_string(source).map_err(|e| e.to_string())?;
    let mut task = crate::parse_task_definition(source.as_str(), &text).map_err(|e| e.to_string())?;
    let mut dependencies = BTreeMap::new();
    crate::project::resolve_instructions(root, source, "instructionsFile", &task.instructions_file, &mut task.instructions, &mut dependencies).map_err(|e| e.to_string())?;
    Ok((task, dependencies))
}

fn get_at(root: &Utf8Path, id: &str) -> Result<TaskDocument, String> {
    let (root, source) = paths(root, id)?;
    let (task, _) = load(&root, &source)?;
    Ok(TaskDocument { id: id.into(), instructions: task.instructions, skills: task.skills, description: task.description, outcomes: task.outcomes })
}

// Replace one scalar only when reparsing proves that no other YAML value changed.
fn replace_scalar(text: &str, old: &str, new: &str, expected: &serde_yaml_ng::Value) -> Result<String, String> {
    let replacement = serde_json::to_string(new).map_err(|e| e.to_string())?;
    for (start, _) in text.match_indices(old) {
        let end = start + old.len();
        let mut spans = vec![(start, end)];
        if start > 0 && end < text.len() {
            let bytes = text.as_bytes();
            if matches!(bytes[start - 1], b'\'' | b'"') && bytes[end] == bytes[start - 1] {
                spans.insert(0, (start - 1, end + 1));
            }
        }
        for (start, end) in spans {
            let mut candidate = text.to_owned();
            candidate.replace_range(start..end, &replacement);
            if serde_yaml_ng::from_str::<serde_yaml_ng::Value>(&candidate).ok().as_ref() == Some(expected) {
                return Ok(candidate);
            }
        }
    }
    Err("Cannot safely rewrite outcome key in this YAML formatting; repair the workflow manually".into())
}

fn rename_route(value: &mut serde_yaml_ng::Value, node: &str, old: &str, new: &str) -> Result<(), String> {
    use serde_yaml_ng::{Mapping, Value};
    let routes = value.get_mut("nodes").and_then(|v| v.get_mut(node)).and_then(|v| v.get_mut("next"))
        .and_then(Value::as_mapping_mut).ok_or("Expected outcome routes")?;
    let old_routes = std::mem::take(routes);
    let mut updated = Mapping::new();
    for (key, destination) in old_routes {
        let key = if key.as_str() == Some(old) { Value::from(new) } else { key };
        if updated.insert(key, destination).is_some() { return Err("Outcome route collision".into()); }
    }
    *routes = updated;
    Ok(())
}

fn workflow_edits(project: &Utf8Path, root: &Utf8Path, id: Option<&str>, old: &[String], new: &[String], renames: &BTreeMap<String, String>) -> Result<Vec<(Utf8PathBuf, String, String)>, String> {
    let old_set: BTreeSet<_> = old.iter().cloned().collect();
    let new_set: BTreeSet<_> = new.iter().cloned().collect();
    if old_set.len() != old.len() || new_set.len() != new.len() { return Err("Duplicate task outcomes".into()); }
    if renames.iter().any(|(from, to)| !old_set.contains(from) || !new_set.contains(to)) {
        return Err("Outcome renames must map existing old outcomes to new outcomes".into());
    }
    let mapped: BTreeSet<_> = old.iter().map(|key| renames.get(key).unwrap_or(key)).collect();
    if mapped.len() != old.len() { return Err("Outcome renames collide; specify an unambiguous mapping".into()); }
    let Some(id) = id else { if renames.is_empty() { return Ok(vec![]); } else { return Err("Cannot rename outcomes of a new task".into()); } };
    if old_set == new_set && renames.iter().all(|(a, b)| a == b) { return Ok(vec![]); }
    let folder = crate::project::bounded_path(root, Utf8Path::new("workflows"), root, "workflow").map_err(|e| e.to_string())?;
    if !folder.try_exists().map_err(|e| e.to_string())? { return Ok(vec![]); }
    let mut paths = Vec::new();
    for entry in fs::read_dir(folder).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let name = entry.file_name().into_string().map_err(|_| "Non-UTF-8 workflow path")?;
        if !name.ends_with(".yaml") { continue; }
        let path = crate::project::bounded_path(root, &Utf8PathBuf::from(format!("workflows/{name}")), root, "workflow").map_err(|e| e.to_string())?;
        if !path.is_file() { return Err(format!("{path}: workflow is not a regular file")); }
        paths.push(path);
    }
    paths.sort();
    let mut edits = Vec::new();
    for path in paths {
        let original = fs::read_to_string(&path).map_err(|e| format!("{path}: {e}"))?;
        let workflow = crate::parse_workflow_definition(path.as_str(), &original).map_err(|e| e.to_string())?;
        let mut changes = Vec::new();
        for (node_id, node) in &workflow.nodes {
            let crate::NodeDefinition::Task(node) = node else { continue };
            if !matches!(&node.task, crate::TaskSelection::Named(name) if name == id) { continue; }
            let routes: BTreeSet<_> = match &node.next {
                crate::Transition::Single(_) if old.is_empty() => BTreeSet::new(),
                crate::Transition::Outcomes(routes) => routes.keys().cloned().collect(),
                _ => return Err(format!("{path}: node {node_id}: existing routes do not match task outcomes; repair workflow first")),
            };
            let updated: BTreeSet<_> = routes.iter().map(|key| renames.get(key).unwrap_or(key).clone()).collect();
            if routes != old_set || updated != new_set || (new.is_empty() != matches!(&node.next, crate::Transition::Single(_))) {
                return Err(format!("{path}: node {node_id}: outcome changes require manual workflow route repair; no destinations can be guessed or removed"));
            }
            for key in &routes {
                if let Some(target) = renames.get(key).filter(|target| *target != key) { changes.push((node_id.clone(), key.clone(), target.clone())); }
            }
        }
        if changes.is_empty() { continue; }
        let mut text = original.clone();
        let mut value: serde_yaml_ng::Value = serde_yaml_ng::from_str(&text).map_err(|e| e.to_string())?;
        let mut temporary = Vec::new();
        for (index, (node, old, new)) in changes.iter().enumerate() {
            let mut temp = format!("__unelos_outcome_rename_{index}__");
            while text.contains(&temp) { temp.push('_'); }
            rename_route(&mut value, node, old, &temp)?;
            text = replace_scalar(&text, old, &temp, &value).map_err(|e| format!("{path}: {e}"))?;
            temporary.push((node, temp, new));
        }
        for (node, temp, new) in temporary {
            rename_route(&mut value, node, &temp, new)?;
            text = replace_scalar(&text, &temp, new, &value).map_err(|e| format!("{path}: {e}"))?;
        }
        crate::parse_workflow_definition(path.as_str(), &text).map_err(|e| e.to_string())?;
        edits.push((path, original, text));
    }
    if !edits.is_empty() {
        // shortcut: run creation is not locked with authoring, add a shared lock before supporting concurrent edits/execution.
        let runs = crate::list_runs(project).map_err(|e| format!("Cannot inspect stored runs: {e}"))?;
        for (path, _, _) in &edits {
            let id = path.file_stem().ok_or("Invalid workflow filename")?;
            if runs.iter().any(|run| run.workflow_name == id && !run.is_finished()) {
                return Err(format!("{path}: finish or remove in-progress runs before changing routes"));
            }
        }
    }
    Ok(edits)
}

fn save_at(root: &Utf8Path, original_id: Option<&str>, task: TaskDocument, project: Option<&Utf8Path>, renames: &BTreeMap<String, String>) -> Result<TaskDocument, String> {
    let (authoring, source) = paths(root, &task.id)?;
    let (instructions_file, dependencies, old_outcomes) = if let Some(id) = original_id {
        if id != task.id { return Err("Task IDs are read-only".into()); }
        let (old, dependencies) = load(&authoring, &source)?;
        (old.instructions_file, dependencies, old.outcomes)
    } else {
        if source.parent().unwrap().exists() { return Err(format!("Task '{}' already exists", task.id)); }
        (None, BTreeMap::new(), vec![])
    };
    let edits = if let Some(project) = project {
        workflow_edits(project, root, original_id, &old_outcomes, &task.outcomes, renames)?
    } else { vec![] };
    let definition = TaskDefinition { instructions: task.instructions.clone(), instructions_file, skills: task.skills, description: task.description, outcomes: task.outcomes };
    let yaml = serde_yaml_ng::to_string(&definition).map_err(|e| e.to_string())?;
    crate::parse_task_definition(source.as_str(), &yaml).map_err(|e| e.to_string())?;
    let instruction_path = dependencies.get(&source).and_then(|paths| paths.first()).map(|path| authoring.join(path));
    let original = instruction_path.as_ref().map(fs::read_to_string).transpose().map_err(|e| e.to_string())?;
    if instruction_path.as_ref().is_some_and(|path| path == &source || edits.iter().any(|(workflow, _, _)| workflow == path)) {
        return Err("instructionsFile conflicts with a file being updated".into());
    }
    let old_source = if source.exists() { Some(fs::read_to_string(&source).map_err(|e| e.to_string())?) } else { None };
    fs::create_dir_all(source.parent().unwrap()).map_err(|e| e.to_string())?;
    let mut writes = Vec::new();
    if let Some(path) = instruction_path { writes.push((path, original, task.instructions.clone())); }
    writes.push((source.clone(), old_source, yaml));
    writes.extend(edits.into_iter().map(|(path, old, new)| (path, Some(old), new)));
    let mut written = 0;
    for (path, _, content) in &writes {
        if let Err(error) = crate::skill_document::atomic_write(path, content) {
            let mut failures = Vec::new();
            for (path, original, _) in writes[..written].iter().rev() {
                let result = match original { Some(text) => crate::skill_document::atomic_write(path, text), None => fs::remove_file(path).map_err(|e| e.to_string()) };
                if let Err(e) = result { failures.push(e); }
            }
            return Err(format!("{error}; rollback errors: {failures:?}"));
        }
        written += 1;
    }
    get_at(root, &task.id)
}

pub(crate) fn global_root() -> Result<Utf8PathBuf, String> {
    crate::project::absolute_root(&crate::global_config::config_dir().map_err(|e| e.to_string())?).map_err(|e| e.to_string())
}
pub fn get_task(root: &Utf8Path, id: &str) -> Result<TaskDocument, String> {
    get_at(&crate::project::absolute_root(root).map_err(|e| e.to_string())?.join(".unelos"), id)
}
pub fn save_task(root: &Utf8Path, original_id: Option<&str>, task: TaskDocument) -> Result<TaskDocument, String> {
    save_task_with_outcome_renames(root, original_id, task, BTreeMap::new())
}
pub fn save_task_with_outcome_renames(root: &Utf8Path, original_id: Option<&str>, task: TaskDocument, renames: BTreeMap<String, String>) -> Result<TaskDocument, String> {
    let root = crate::project::absolute_root(root).map_err(|e| e.to_string())?;
    save_at(&root.join(".unelos"), original_id, task, Some(&root), &renames)
}
pub fn get_global_task(id: &str) -> Result<TaskDocument, String> { get_at(&global_root()?, id) }
pub fn save_global_task(original_id: Option<&str>, task: TaskDocument) -> Result<TaskDocument, String> { save_at(&global_root()?, original_id, task, None, &BTreeMap::new()) }
