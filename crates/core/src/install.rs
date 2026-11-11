use crate::{Execution, LoadedProject, NodeDefinition, TaskSelection};
use camino::{Utf8Component, Utf8Path, Utf8PathBuf};
use serde::Serialize;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::{self, Write},
};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentExport {
    pub id: String,
    pub instructions: String,
    pub model: Option<String>,
    pub tools: Vec<String>,
    pub mcps: Vec<String>,
    pub main: bool,
    pub subagent: bool,
}

#[derive(Debug, Serialize)]
pub struct WorkflowInstall {
    pub workflow: String,
    pub agents: Vec<AgentExport>,
}

#[derive(Debug, thiserror::Error)]
#[error("{message}")]
pub struct InstallError {
    pub code: &'static str,
    pub message: String,
}

fn fail(code: &'static str, message: impl Into<String>) -> InstallError {
    InstallError {
        code,
        message: message.into(),
    }
}
fn io_error(error: io::Error) -> InstallError {
    fail("install_io_failed", error.to_string())
}

pub(crate) fn resolve_agent_model(model: &crate::AgentModel) -> Result<String, InstallError> {
    match model {
        crate::AgentModel::Alias(alias) => crate::list_model_mappings()?
            .get(alias).and_then(|models| models.get("opencode")).cloned()
            .ok_or_else(|| fail("unmapped_model_alias", format!("No OpenCode model mapped for alias '{alias}'"))),
        crate::AgentModel::Harness { harness, model } if harness == "opencode" => Ok(model.clone()),
        crate::AgentModel::Harness { harness, .. } => Err(fail("model_harness_mismatch", format!("Model targets {harness}, expected opencode"))),
    }
}

use crate::project::safe_id;

fn references(project: &LoadedProject) -> (BTreeSet<String>, BTreeMap<String, (bool, bool)>) {
    let mut tasks = BTreeSet::new();
    let mut agents = BTreeMap::new();
    let mut add = |task: &TaskSelection, agent: &str, execution: Execution| {
        if let TaskSelection::Named(id) = task {
            tasks.insert(id.clone());
        }
        let modes = agents.entry(agent.to_owned()).or_insert((false, false));
        match execution {
            Execution::Main => modes.0 = true,
            Execution::Subagent => modes.1 = true,
        };
    };
    for node in project.workflow.nodes.values() {
        match node {
            NodeDefinition::Task(node) => add(&node.task, &node.agent, node.execution),
            NodeDefinition::Parallel(node) => {
                for worker in &node.tasks {
                    add(&worker.task, &worker.agent, Execution::Subagent)
                }
            }
            _ => {}
        }
    }
    (tasks, agents)
}

fn export(project: &LoadedProject) -> Result<Vec<AgentExport>, InstallError> {
    let (_, references) = references(project);
    references
        .into_iter()
        .map(|(id, (main, subagent))| {
            if !safe_id(&id) {
                return Err(fail(
                    "unsupported_agent_id",
                    format!("Agent ID cannot be represented as a native profile: {id:?}"),
                ));
            }
            let agent = &project.definitions.agents[&id];
            if !agent.mcps.is_empty() {
                return Err(fail(
                    "unsupported_mcp",
                    format!(
                        "MCP requirements need explicit harness support: {}",
                        agent.mcps.join(", ")
                    ),
                ));
            }
            let model = agent.model.as_ref().map(resolve_agent_model).transpose()?;
            if let Some(model) = &model {
                let (provider, rest) = model
                    .split_once('/')
                    .ok_or_else(|| fail("invalid_model", "Expected provider/model[#variant]"))?;
                let parts: Vec<_> = rest.split('#').collect();
                if provider.is_empty()
                    || parts.len() > 2
                    || parts.iter().any(|s| s.is_empty())
                    || model.chars().any(|c| c.is_whitespace() || c == '\0')
                    || provider.contains('#')
                {
                    return Err(fail("invalid_model", "Expected provider/model[#variant]"));
                }
            }
            Ok(AgentExport {
                id,
                instructions: agent.instructions.clone(),
                model,
                tools: agent.tools.clone(),
                mcps: agent.mcps.clone(),
                main,
                subagent,
            })
        })
        .collect()
}

pub fn export_workflow_agents(
    root: &Utf8Path,
    workflow: &str,
) -> Result<Vec<AgentExport>, InstallError> {
    if !safe_id(workflow) {
        return Err(fail(
            "invalid_workflow_name",
            "Workflow must be a safe local identifier",
        ));
    }
    let project = load_install_source(root, workflow)?;
    crate::validate_project(&project).map_err(|e| fail("invalid_project", e.to_string()))?;
    export(&project)
}

fn load_install_source(root: &Utf8Path, workflow: &str) -> Result<LoadedProject, InstallError> {
    checked_path(root, Utf8Path::new(".unelos"))?;
    if root.join(".unelos").exists() {
        return crate::load_project(root, workflow)
            .map_err(|error| fail("invalid_project", error.to_string()));
    }
    let settings = checked_path(root, Utf8Path::new("project.yaml"))?;
    match fs::read_to_string(&settings) {
        Ok(text) => {
            crate::parse_project_settings(settings.as_str(), &text)
                .map_err(|error| fail("invalid_project", error.to_string()))?;
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(io_error(error)),
    }
    crate::project::load_authoring(root, workflow)
        .map_err(|error| fail("invalid_project", error.to_string()))
}

pub fn install_workflow(
    root: &Utf8Path,
    source: Option<&Utf8Path>,
    workflow: &str,
) -> Result<WorkflowInstall, InstallError> {
    if !safe_id(workflow) {
        return Err(fail(
            "invalid_workflow_name",
            "Workflow must be a safe local identifier",
        ));
    }
    let Some(source) = source else {
        let project = crate::load_project(root, workflow)
            .map_err(|error| fail("invalid_project", error.to_string()))?;
        crate::validate_project(&project)
            .map_err(|error| fail("invalid_project", error.to_string()))?;
        return Ok(WorkflowInstall {
            workflow: workflow.into(),
            agents: export(&project)?,
        });
    };
    let project = load_install_source(source, workflow)?;
    crate::validate_project(&project)
        .map_err(|error| fail("invalid_project", error.to_string()))?;
    let agents = export(&project)?;
    check_project_init(root)?;
    let mut files = BTreeMap::new();
    let authoring = &project.authoring_root;
    let (tasks, agent_ids) = references(&project);
    let mut selected_sources = BTreeSet::from([project.workflow_source.clone()]);
    for id in &tasks {
        if id.starts_with("global/") { continue; }
        collect_files(authoring, &Utf8PathBuf::from("tasks").join(id), &mut files)?;
        selected_sources.insert(project.task_sources[id].clone());
    }
    for id in agent_ids.keys() {
        collect_files(authoring, &Utf8PathBuf::from("agents").join(id), &mut files)?;
        selected_sources.insert(project.agent_sources[id].clone());
    }
    files.insert(
        Utf8PathBuf::from(format!("workflows/{workflow}.yaml")),
        fs::read(&project.workflow_source).map_err(io_error)?,
    );
    for source in selected_sources {
        if let Some(dependencies) = project.instruction_dependencies.get(&source) {
            for relative in dependencies {
                let mut components = relative.components();
                let registry = components.next().map(|part| part.as_str());
                let id = components
                    .next()
                    .map(|part| part.as_str())
                    .unwrap_or_default();
                if (registry == Some("tasks") && !tasks.contains(id))
                    || (registry == Some("agents") && !agent_ids.contains_key(id))
                    || (registry == Some("workflows")
                        && relative.extension() == Some("yaml")
                        && relative != &Utf8PathBuf::from(format!("workflows/{workflow}.yaml")))
                {
                    return Err(fail("invalid_project", format!("{source}: instructionsFile dependency {relative} belongs to unselected authoring; place shared instructions outside definition/workflow registries")));
                }
                files.insert(
                    relative.clone(),
                    fs::read(checked_path(authoring, relative)?).map_err(io_error)?,
                );
            }
        }
    }
    let mut skill_ids = Vec::new();
    for (node_id, node) in &project.workflow.nodes {
        let selections: Vec<_> = match node {
            NodeDefinition::Task(task) => vec![(&task.task, format!("nodes.{node_id}"))],
            NodeDefinition::Parallel(parallel) => parallel
                .tasks
                .iter()
                .enumerate()
                .map(|(index, worker)| (&worker.task, format!("nodes.{node_id}.tasks[{index}]")))
                .collect(),
            _ => Vec::new(),
        };
        for (selection, field) in selections {
            let (task, _) = project
                .selected_task(selection, &field)
                .map_err(|error| fail("invalid_project", error.to_string()))?;
            skill_ids.extend(task.skills.iter().cloned());
        }
    }
    skill_ids.extend(
        agent_ids
            .keys()
            .flat_map(|id| project.definitions.agents[id].skills.iter().cloned()),
    );
    let skills = crate::skills::resolve_authoring_skills(authoring, &skill_ids)
        .map_err(|e| fail("invalid_skill", e.to_string()))?;
    for skill in skills {
        collect_files(
            authoring,
            &Utf8PathBuf::from("skills").join(skill.id),
            &mut files,
        )?;
    }
    // Validate all imported destinations before any write; native generated profiles overwrite separately.
    for (relative, bytes) in &files {
        let relative = Utf8PathBuf::from(".unelos").join(relative);
        let path = checked_path(root, &relative)?;
        match fs::read(&path) {
            Ok(existing) if existing != *bytes => {
                return Err(fail(
                    "import_collision",
                    format!("Existing authoring differs: {relative}"),
                ))
            }
            Ok(_) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(io_error(error)),
        }
    }
    for (relative, bytes) in files {
        write_install_file(root, &Utf8PathBuf::from(".unelos").join(relative), &bytes)?;
    }
    initialize_project(root)?;
    Ok(WorkflowInstall {
        workflow: workflow.into(),
        agents,
    })
}

fn collect_files(
    root: &Utf8Path,
    relative: &Utf8Path,
    files: &mut BTreeMap<Utf8PathBuf, Vec<u8>>,
) -> Result<(), InstallError> {
    let path = checked_path(root, relative)?;
    for entry in fs::read_dir(path).map_err(io_error)? {
        let entry = entry.map_err(io_error)?;
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| fail("invalid_path", "Authoring resource is not UTF-8"))?;
        let relative = relative.join(name);
        let path = checked_path(root, &relative)?;
        let metadata = fs::symlink_metadata(&path).map_err(io_error)?;
        if metadata.is_dir() {
            collect_files(root, &relative, files)?;
        } else if metadata.is_file() {
            files.insert(relative, fs::read(path).map_err(io_error)?);
        } else {
            return Err(fail(
                "invalid_path",
                "Authoring resources must be ordinary files/directories",
            ));
        }
    }
    Ok(())
}

fn checked_path(root: &Utf8Path, relative: &Utf8Path) -> Result<Utf8PathBuf, InstallError> {
    let mut path = root.to_owned();
    for component in relative.components() {
        let Utf8Component::Normal(part) = component else {
            return Err(fail(
                "invalid_path",
                "Installation path must stay within the project",
            ));
        };
        path.push(part);
        match fs::symlink_metadata(&path) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                return Err(fail(
                    "invalid_path",
                    format!("Symlinked installation path: {path}"),
                ))
            }
            Ok(metadata) if path != root.join(relative) && !metadata.is_dir() => {
                return Err(fail(
                    "invalid_path",
                    format!("Installation parent is not a directory: {path}"),
                ));
            }
            Ok(_) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(io_error(error)),
        }
    }
    Ok(path)
}

/// Check a destination without creating or changing project files.
pub fn check_install_path(root: &Utf8Path, relative: &Utf8Path) -> Result<(), InstallError> {
    checked_path(root, relative).map(|_| ())
}

/// Validate authored scaffold destinations without changing project files.
pub fn check_project_init(root: &Utf8Path) -> Result<(), InstallError> {
    if !root.is_dir() {
        return Err(fail(
            "invalid_project",
            "Project must be an existing directory",
        ));
    }
    for relative in [
        ".unelos/project.yaml",
        ".unelos/workflows",
        ".unelos/skills",
        ".unelos/tasks",
        ".unelos/agents",
    ] {
        let path = checked_path(root, Utf8Path::new(relative))?;
        match fs::metadata(&path) {
            Ok(meta)
                if relative.ends_with(".yaml") && !meta.is_file()
                    || !relative.ends_with(".yaml") && !meta.is_dir() =>
            {
                return Err(fail(
                    "invalid_path",
                    format!("Wrong destination type: {path}"),
                ));
            }
            Ok(_) => {}
            Err(e) if e.kind() == io::ErrorKind::NotFound => {}
            Err(e) => return Err(io_error(e)),
        }
    }
    let path = root.join(".unelos/project.yaml");
    match fs::read_to_string(&path) {
        Ok(text) => {
            crate::parse_project_settings(path.as_str(), &text)
                .map_err(|e| fail("invalid_project", e.to_string()))?;
        }
        Err(e) if e.kind() == io::ErrorKind::NotFound => {}
        Err(e) => return Err(io_error(e)),
    }
    Ok(())
}

/// Create only missing shared authoring scaffold; preserve existing bytes.
pub fn initialize_project(root: &Utf8Path) -> Result<(), InstallError> {
    check_project_init(root)?;
    for relative in [
        ".unelos/workflows",
        ".unelos/skills",
        ".unelos/tasks",
        ".unelos/agents",
    ] {
        fs::create_dir_all(root.join(relative)).map_err(io_error)?;
    }
    if !root.join(".unelos/project.yaml").exists() {
        write_install_file(
            root,
            Utf8Path::new(".unelos/project.yaml"),
            b"# Project settings will live here.\n{}\n",
        )?;
    }
    Ok(())
}

/// Safe project-local installation transport shared by authoring and harness adapters.
pub fn write_install_file(
    root: &Utf8Path,
    relative: &Utf8Path,
    bytes: &[u8],
) -> Result<(), InstallError> {
    let path = checked_path(root, relative)?;
    let parent = path
        .parent()
        .ok_or_else(|| fail("invalid_path", "Missing installation parent"))?;
    fs::create_dir_all(parent).map_err(io_error)?;
    let temporary = parent.join(format!(
        ".{}.unelos-{}.tmp",
        path.file_name().unwrap(),
        std::process::id()
    ));
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)
        .map_err(io_error)?;
    let result = (|| {
        file.write_all(bytes)?;
        file.sync_all()?;
        drop(file);
        fs::rename(&temporary, &path)
    })()
    .map_err(io_error);
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}
