use crate::{Execution, LoadedProject, NodeDefinition, ProjectDefinitions};
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

fn safe_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 128
        && id
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'-' | b'_'))
}

fn references(project: &LoadedProject) -> (BTreeSet<String>, BTreeMap<String, (bool, bool)>) {
    let mut tasks = BTreeSet::new();
    let mut agents = BTreeMap::new();
    let mut add = |task: &str, agent: &str, execution: Execution| {
        tasks.insert(task.to_owned());
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
            if let Some(model) = &agent.model {
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
                model: agent.model.clone(),
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
    checked_path(root, Utf8Path::new(".unelos/project.yaml"))?;
    checked_path(
        root,
        &Utf8PathBuf::from(format!(".unelos/workflows/{workflow}.yaml")),
    )?;
    let project =
        crate::load_project(root, workflow).map_err(|e| fail("invalid_project", e.to_string()))?;
    crate::validate_project(&project).map_err(|e| fail("invalid_project", e.to_string()))?;
    export(&project)
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
        return Ok(WorkflowInstall {
            workflow: workflow.into(),
            agents: export_workflow_agents(root, workflow)?,
        });
    };
    let agents = export_workflow_agents(source, workflow)?;
    let project = crate::load_project(source, workflow)
        .map_err(|e| fail("invalid_project", e.to_string()))?;
    let project_path = checked_path(root, Utf8Path::new(".unelos/project.yaml"))?;
    let mut definitions = match fs::read_to_string(&project_path) {
        Ok(text) => crate::parse_project_definitions(project_path.as_str(), &text)
            .map_err(|e| fail("invalid_project", e.to_string()))?,
        Err(error) if error.kind() == io::ErrorKind::NotFound => ProjectDefinitions {
            task_definitions: BTreeMap::new(),
            agents: BTreeMap::new(),
        },
        Err(error) => return Err(io_error(error)),
    };
    let (tasks, agent_ids) = references(&project);
    for id in &tasks {
        let value = &project.definitions.task_definitions[id];
        if definitions
            .task_definitions
            .get(id)
            .is_some_and(|existing| existing != value)
        {
            return Err(fail(
                "import_collision",
                format!("Existing Task definition differs: {id}"),
            ));
        }
        definitions
            .task_definitions
            .insert(id.clone(), value.clone());
    }
    for id in agent_ids.keys() {
        let value = &project.definitions.agents[id];
        if definitions
            .agents
            .get(id)
            .is_some_and(|existing| existing != value)
        {
            return Err(fail(
                "import_collision",
                format!("Existing Agent definition differs: {id}"),
            ));
        }
        definitions.agents.insert(id.clone(), value.clone());
    }
    let workflow_relative = Utf8PathBuf::from(format!(".unelos/workflows/{workflow}.yaml"));
    let mut files = vec![(
        workflow_relative,
        fs::read(&project.workflow_source).map_err(io_error)?,
    )];
    let skill_ids: Vec<_> = tasks
        .iter()
        .flat_map(|id| project.definitions.task_definitions[id].skills.iter())
        .chain(
            agent_ids
                .keys()
                .flat_map(|id| project.definitions.agents[id].skills.iter()),
        )
        .cloned()
        .collect();
    let skills = crate::resolve_skills(source, &skill_ids)
        .map_err(|e| fail("invalid_skill", e.to_string()))?;
    for skill in skills {
        collect_files(
            source,
            &Utf8PathBuf::from(format!(".unelos/skills/{}", skill.id)),
            &mut files,
        )?;
    }
    // Validate all imported destinations before any write; native generated profiles overwrite separately.
    for (relative, bytes) in &files {
        let path = checked_path(root, relative)?;
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
    let yaml = serde_yaml_ng::to_string(&definitions)
        .map_err(|e| fail("invalid_project", e.to_string()))?;
    for (relative, bytes) in files {
        write_install_file(root, &relative, &bytes)?;
    }
    write_install_file(root, Utf8Path::new(".unelos/project.yaml"), yaml.as_bytes())?;
    Ok(WorkflowInstall {
        workflow: workflow.into(),
        agents,
    })
}

fn collect_files(
    root: &Utf8Path,
    relative: &Utf8Path,
    files: &mut Vec<(Utf8PathBuf, Vec<u8>)>,
) -> Result<(), InstallError> {
    let path = checked_path(root, relative)?;
    for entry in fs::read_dir(path).map_err(io_error)? {
        let entry = entry.map_err(io_error)?;
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| fail("invalid_path", "Skill resource is not UTF-8"))?;
        let relative = relative.join(name);
        let path = checked_path(root, &relative)?;
        let metadata = fs::symlink_metadata(&path).map_err(io_error)?;
        if metadata.is_dir() {
            collect_files(root, &relative, files)?;
        } else if metadata.is_file() {
            files.push((relative, fs::read(path).map_err(io_error)?));
        } else {
            return Err(fail(
                "invalid_path",
                "Skill resources must be ordinary files/directories",
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
