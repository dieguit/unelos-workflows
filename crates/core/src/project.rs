use camino::{Utf8Path, Utf8PathBuf};
use std::{
    collections::{BTreeMap, BTreeSet},
    ffi::OsString,
    fs, io,
};

use crate::{
    parse_agent_definition, parse_project_settings, parse_task_definition,
    parse_workflow_definition, DefinitionParseError, NodeDefinition, ProjectDefinitions,
    ProjectValidationError, TaskSelection, WorkflowDefinition,
};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
pub struct Reference {
    pub kind: String,
    pub id: String,
    pub name: String,
}

#[derive(Debug, Default, serde::Serialize)]
pub struct ProjectReferences {
    pub agents: BTreeMap<String, Vec<Reference>>,
    pub tasks: BTreeMap<String, Vec<Reference>>,
    pub skills: BTreeMap<String, Vec<Reference>>,
    pub mcps: BTreeMap<String, Vec<Reference>>,
}

fn add_reference(map: &mut BTreeMap<String, Vec<Reference>>, id: &str, reference: &Reference) {
    if !id.is_empty() {
        map.entry(id.into()).or_default().push(reference.clone());
    }
}

impl ProjectReferences {
    fn task(&mut self, task: &TaskSelection, agent: &str, reference: &Reference) {
        add_reference(&mut self.agents, agent, reference);
        match task {
            TaskSelection::Named(id) => add_reference(&mut self.tasks, id, reference),
            TaskSelection::Inline(task) => {
                for skill in &task.skills {
                    add_reference(&mut self.skills, skill, reference);
                }
            }
        }
    }
}

pub fn project_references(root: &Utf8Path) -> Result<ProjectReferences, ProjectLoadError> {
    let (root, workflows) = workflow_files(root)?;
    let mut references = ProjectReferences::default();
    for agent in agent_summaries(&root)?
        .into_iter()
        .filter(|a| a.problem.is_none())
    {
        let reference = Reference {
            kind: "agent".into(),
            id: agent.name.clone(),
            name: agent.name,
        };
        for skill in agent.skills {
            add_reference(&mut references.skills, &skill, &reference);
        }
        for mcp in agent.mcps {
            add_reference(&mut references.mcps, &mcp, &reference);
        }
    }
    for task in task_summaries(&root)?
        .into_iter()
        .filter(|t| t.problem.is_none())
    {
        let reference = Reference {
            kind: "task".into(),
            id: task.name.clone(),
            name: task.name,
        };
        for skill in task.skills {
            add_reference(&mut references.skills, &skill, &reference);
        }
    }
    for id in workflows {
        let source = bounded_path(
            &root.join(".unelos"),
            &Utf8PathBuf::from(format!("workflows/{id}.yaml")),
            &root.join(".unelos"),
            "workflow",
        );
        let Ok(workflow) = source
            .and_then(|source| Ok(parse_workflow_definition(source.as_str(), &read(&source)?)?))
        else {
            continue;
        };
        let reference = Reference {
            kind: "workflow".into(),
            id,
            name: workflow.name,
        };
        for node in workflow.nodes.values() {
            match node {
                NodeDefinition::Task(task) => references.task(&task.task, &task.agent, &reference),
                NodeDefinition::Parallel(parallel) => {
                    for worker in &parallel.tasks {
                        references.task(&worker.task, &worker.agent, &reference);
                    }
                }
                _ => {}
            }
        }
    }
    for map in [
        &mut references.agents,
        &mut references.tasks,
        &mut references.skills,
        &mut references.mcps,
    ] {
        for entries in map.values_mut() {
            entries.sort_by(|a, b| a.name.cmp(&b.name).then_with(|| a.cmp(b)));
            entries.dedup();
        }
    }
    Ok(references)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadedProject {
    pub project_root: Utf8PathBuf,
    pub project_source: Utf8PathBuf,
    pub authoring_root: Utf8PathBuf,
    pub workflow_source: Utf8PathBuf,
    pub task_sources: BTreeMap<String, Utf8PathBuf>,
    pub agent_sources: BTreeMap<String, Utf8PathBuf>,
    pub instruction_dependencies: BTreeMap<Utf8PathBuf, BTreeSet<Utf8PathBuf>>,
    pub definitions: ProjectDefinitions,
    pub workflow: WorkflowDefinition,
    pub authoring_workflow: WorkflowDefinition,
    pub node_sources: BTreeMap<String, crate::NodeSource>,
    pub(crate) child_workflows: Vec<(
        Utf8PathBuf,
        WorkflowDefinition,
        BTreeMap<String, crate::NodeSource>,
    )>,
}

impl LoadedProject {
    pub(crate) fn selected_task<'a>(
        &'a self,
        selection: &'a crate::TaskSelection,
        field: &str,
    ) -> Result<(&'a crate::TaskDefinition, String), ProjectValidationError> {
        match selection {
            crate::TaskSelection::Named(id) => self
                .definitions
                .task_definitions
                .get(id)
                .map(|task| (task, id.clone()))
                .ok_or_else(|| ProjectValidationError {
                    path: self.workflow_source.clone(),
                    field: format!("{field}.task"),
                    cause: match id.strip_prefix("global/") {
                        Some(id) => format!("Global task {id:?} not found"),
                        None => format!("unknown Task Definition {id:?}"),
                    },
                }),
            crate::TaskSelection::Inline(task) => Ok((task, format!("{field}.taskDefinition"))),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ProjectLoadError {
    #[error("invalid Workflow name {name:?}: expected a nonempty filename stem")]
    InvalidWorkflowName { name: String },
    #[error("could not resolve project root {path}: {source}")]
    Root {
        path: Utf8PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("missing file {path}: {source}")]
    Missing {
        path: Utf8PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("could not read file {path}: {source}")]
    Read {
        path: Utf8PathBuf,
        #[source]
        source: io::Error,
    },
    #[error(transparent)]
    Parse(#[from] DefinitionParseError),
    #[error(transparent)]
    Validation(#[from] ProjectValidationError),
    #[error("Workflow {path} declares name {declared:?}, requested {requested:?}")]
    NameMismatch {
        path: Utf8PathBuf,
        requested: String,
        declared: String,
    },
}

/// List valid project Workflows in filename order without changing runtime state.
pub fn list_workflows(project_root: &Utf8Path) -> Result<Vec<String>, ProjectLoadError> {
    Ok(list_workflow_descriptions(project_root)?
        .into_keys()
        .collect())
}

pub(crate) fn list_workflow_descriptions(
    project_root: &Utf8Path,
) -> Result<BTreeMap<String, Option<String>>, ProjectLoadError> {
    let (root, names) = workflow_files(project_root)?;
    let mut workflows = BTreeMap::new();
    for name in names {
        let project = load_project(&root, &name)?;
        crate::validate_project(&project)?;
        workflows.insert(name, project.workflow.description);
    }
    Ok(workflows)
}

/// One row per `.unelos/workflows/*.yaml`; a broken Workflow yields `problem`
/// instead of failing the list. Only project-level problems return `Err`.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowSummary {
    pub calls: Vec<String>,
    pub name: String,
    pub file: String,
    pub description: Option<String>,
    /// `None` when the workflow file could not be loaded.
    pub steps: Option<usize>,
    pub problem: Option<String>,
}

pub fn workflow_summaries(
    project_root: &Utf8Path,
) -> Result<Vec<WorkflowSummary>, ProjectLoadError> {
    let (root, names) = workflow_files(project_root)?;
    let mut summaries: Vec<_> = names
        .into_iter()
        .map(|name| {
            let file = format!("{name}.yaml");
            match load_project(&root, &name) {
                Ok(project) => WorkflowSummary {
                    calls: project
                        .authoring_workflow
                        .nodes
                        .values()
                        .filter_map(|node| match node {
                            NodeDefinition::Workflow(call) => Some(call.workflow.clone()),
                            _ => None,
                        })
                        .collect::<BTreeSet<_>>()
                        .into_iter()
                        .collect(),
                    problem: crate::validate_project(&project)
                        .err()
                        .map(|error| format!("{}: {}", error.field, error.cause)),
                    description: project.workflow.description.clone(),
                    steps: Some(
                        project
                            .authoring_workflow
                            .nodes
                            .values()
                            .filter(|node| !matches!(node, NodeDefinition::End))
                            .count(),
                    ),
                    name,
                    file,
                },
                Err(error) => WorkflowSummary {
                    calls: Vec::new(),
                    problem: Some(error.to_string()),
                    description: None,
                    steps: None,
                    name,
                    file,
                },
            }
        })
        .collect();
    summaries.sort_by(|a, b| a.file.cmp(&b.file));
    Ok(summaries)
}

/// Resolved project root plus sorted Workflow names (file stems).
fn workflow_files(project_root: &Utf8Path) -> Result<(Utf8PathBuf, Vec<String>), ProjectLoadError> {
    let root = absolute_root(project_root).map_err(|source| ProjectLoadError::Root {
        path: project_root.to_owned(),
        source,
    })?;
    let authoring_root = root.join(".unelos");
    let definitions_path = bounded_path(
        &authoring_root,
        Utf8Path::new("project.yaml"),
        &authoring_root,
        "project",
    )?;
    parse_project_settings(definitions_path.as_str(), &read(&definitions_path)?)?;

    let directory = bounded_path(
        &authoring_root,
        Utf8Path::new("workflows"),
        &definitions_path,
        "workflows",
    )?;
    let entries = match fs::read_dir(&directory) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok((root, Vec::new())),
        Err(source) => {
            return Err(ProjectLoadError::Read {
                path: directory,
                source,
            })
        }
    };
    let mut names = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|source| ProjectLoadError::Read {
            path: directory.clone(),
            source,
        })?;
        let path =
            Utf8PathBuf::from_path_buf(entry.path()).map_err(|_| ProjectLoadError::Read {
                path: directory.clone(),
                source: io::Error::new(io::ErrorKind::InvalidData, "workflow path is not UTF-8"),
            })?;
        let file_type = entry.file_type().map_err(|source| ProjectLoadError::Read {
            path: path.clone(),
            source,
        })?;
        if path.extension() != Some("yaml") || !file_type.is_file() {
            continue;
        }
        let name = path
            .file_stem()
            .filter(|name| !name.is_empty())
            .ok_or_else(|| ProjectLoadError::InvalidWorkflowName {
                name: path.to_string(),
            })?
            .to_owned();
        names.push(name);
    }
    names.sort();
    Ok((root, names))
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct AgentSummary {
    pub name: String,
    pub model: Option<crate::AgentModel>,
    pub skills: Vec<String>,
    pub mcps: Vec<String>,
    /// Why this agent's definition could not be read; other fields are then empty.
    pub problem: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct TaskSummary {
    pub name: String,
    pub description: Option<String>,
    pub skills: Vec<String>,
    pub outcomes: Vec<String>,
    pub problem: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct SkillSummary {
    pub name: String,
    pub description: Option<String>,
    /// Why this skill's SKILL.md could not be read; other fields are then empty.
    pub problem: Option<String>,
}

/// Validate project settings, then summarize each `.unelos/<directory>/<id>` child in name
/// order. A child that fails becomes `broken(id, message)` instead of failing the list; only
/// project-level problems return `Err`. A missing directory yields no rows.
fn summarize_definitions<T>(
    project_root: &Utf8Path,
    directory: &str,
    summarize: impl Fn(&Utf8Path, &Utf8Path, &str) -> Result<T, ProjectLoadError>,
    broken: impl Fn(String, String) -> T,
) -> Result<Vec<T>, ProjectLoadError> {
    let root = absolute_root(project_root).map_err(|source| ProjectLoadError::Root {
        path: project_root.to_owned(),
        source,
    })?;
    let authoring_root = root.join(".unelos");
    let settings = bounded_path(
        &authoring_root,
        Utf8Path::new("project.yaml"),
        &authoring_root,
        "project",
    )?;
    parse_project_settings(settings.as_str(), &read(&settings)?)?;
    summarize_authoring(&authoring_root, directory, summarize, broken)
}

fn summarize_authoring<T>(
    authoring_root: &Utf8Path,
    directory: &str,
    summarize: impl Fn(&Utf8Path, &Utf8Path, &str) -> Result<T, ProjectLoadError>,
    broken: impl Fn(String, String) -> T,
) -> Result<Vec<T>, ProjectLoadError> {
    let path = bounded_path(
        authoring_root,
        Utf8Path::new(directory),
        authoring_root,
        directory,
    )?;
    let entries = match fs::read_dir(&path) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(source) => return Err(ProjectLoadError::Read { path, source }),
    };
    let candidates = agent_candidates(
        &path,
        entries.map(|entry| {
            entry.map(|entry| {
                (
                    entry.file_name(),
                    entry.file_type().map(|kind| kind.is_file()),
                )
            })
        }),
    )?;
    Ok(candidates
        .into_iter()
        .map(|(name, inspect)| {
            let Some(id) = name.to_str() else {
                return broken(
                    name.to_string_lossy().into_owned(),
                    "definition ID is not UTF-8".into(),
                );
            };
            inspect
                .map_err(|source| ProjectLoadError::Read {
                    path: path.join(id),
                    source,
                })
                .and_then(|()| summarize(authoring_root, &path, id))
                .unwrap_or_else(|error| broken(id.to_owned(), error.to_string()))
        })
        .collect())
}

/// Summarize each `.unelos/agents/<id>/agent.yaml`; a broken agent is a row with `problem`.
pub fn agent_summaries(project_root: &Utf8Path) -> Result<Vec<AgentSummary>, ProjectLoadError> {
    summarize_definitions(
        project_root,
        "agents",
        |authoring_root, directory, id| {
            let relative = Utf8PathBuf::from(format!("agents/{id}"));
            let entry = bounded_path(authoring_root, &relative, directory, id)?;
            if !safe_id(id) {
                return Err(diagnostic(&entry, "id", "invalid definition ID"));
            }
            let source = bounded_path(
                authoring_root,
                &Utf8PathBuf::from(format!("{relative}/agent.yaml")),
                &entry,
                "agent.yaml",
            )?;
            let mut agent = parse_agent_definition(source.as_str(), &read(&source)?)?;
            resolve_instructions(
                authoring_root,
                &source,
                "instructionsFile",
                &agent.instructions_file,
                &mut agent.instructions,
                &mut BTreeMap::new(),
            )?;
            Ok(AgentSummary {
                name: id.to_owned(),
                model: agent.model,
                skills: agent.skills,
                mcps: agent.mcps,
                problem: None,
            })
        },
        |name, problem| AgentSummary {
            name,
            model: None,
            skills: Vec::new(),
            mcps: Vec::new(),
            problem: Some(problem),
        },
    )
}

/// Summarize each `.unelos/tasks/<id>/task.yaml`; a broken task is a row with `problem`.
pub fn task_summaries(project_root: &Utf8Path) -> Result<Vec<TaskSummary>, ProjectLoadError> {
    summarize_definitions(project_root, "tasks", summarize_task, broken_task)
}

pub fn global_task_summaries() -> Result<Vec<TaskSummary>, String> {
    summarize_authoring(
        &crate::task_document::global_root()?,
        "tasks",
        summarize_task,
        broken_task,
    )
    .map_err(|e| e.to_string())
}

fn summarize_task(
    authoring_root: &Utf8Path,
    directory: &Utf8Path,
    id: &str,
) -> Result<TaskSummary, ProjectLoadError> {
    let relative = Utf8PathBuf::from(format!("tasks/{id}"));
    let entry = bounded_path(authoring_root, &relative, directory, id)?;
    if !safe_id(id) {
        return Err(diagnostic(&entry, "id", "invalid definition ID"));
    }
    let source = bounded_path(
        authoring_root,
        &Utf8PathBuf::from(format!("{relative}/task.yaml")),
        &entry,
        "task.yaml",
    )?;
    let mut task = parse_task_definition(source.as_str(), &read(&source)?)?;
    resolve_instructions(
        authoring_root,
        &source,
        "instructionsFile",
        &task.instructions_file,
        &mut task.instructions,
        &mut BTreeMap::new(),
    )?;
    Ok(TaskSummary {
        name: id.to_owned(),
        description: task.description,
        skills: task.skills,
        outcomes: task.outcomes,
        problem: None,
    })
}

fn broken_task(name: String, problem: String) -> TaskSummary {
    TaskSummary {
        name,
        description: None,
        skills: Vec::new(),
        outcomes: Vec::new(),
        problem: Some(problem),
    }
}

/// Summarize each `.unelos/skills/<id>/SKILL.md`; a broken skill is a row with `problem`.
/// IDs follow `resolve_skills`, so `safe_id` is not applied.
pub fn skill_summaries(project_root: &Utf8Path) -> Result<Vec<SkillSummary>, ProjectLoadError> {
    let root = absolute_root(project_root)
        .map_err(|source| ProjectLoadError::Root {
            path: project_root.to_owned(),
            source,
        })?
        .join(".unelos");
    let settings = bounded_path(&root, Utf8Path::new("project.yaml"), &root, "project")?;
    parse_project_settings(settings.as_str(), &read(&settings)?)?;
    skill_summaries_at(&root)
}

pub fn global_skill_summaries() -> Result<Vec<SkillSummary>, String> {
    skill_summaries_at(&crate::task_document::global_root()?).map_err(|e| e.to_string())
}

fn skill_summaries_at(authoring_root: &Utf8Path) -> Result<Vec<SkillSummary>, ProjectLoadError> {
    summarize_authoring(
        authoring_root,
        "skills",
        |authoring_root, directory, id| {
            let relative = Utf8PathBuf::from(format!("skills/{id}"));
            let entry = bounded_path(authoring_root, &relative, directory, id)?;
            let source = bounded_path(
                authoring_root,
                &Utf8PathBuf::from(format!("{relative}/SKILL.md")),
                &entry,
                "SKILL.md",
            )?;
            let description = skill_description(&read(&source)?)
                .map_err(|error| diagnostic(&source, "frontmatter", error))?;
            Ok(SkillSummary {
                name: id.to_owned(),
                description,
                problem: None,
            })
        },
        |name, problem| SkillSummary {
            name,
            description: None,
            problem: Some(problem),
        },
    )
}

/// `description` from a leading `---` YAML block; no block, empty block or no key → `None`.
fn skill_description(text: &str) -> Result<Option<String>, String> {
    let Some(rest) = text
        .strip_prefix("---\n")
        .or_else(|| text.strip_prefix("---\r\n"))
    else {
        return Ok(None);
    };
    let yaml = if rest.starts_with("---") {
        ""
    } else {
        &rest[..rest.find("\n---").ok_or("missing closing ---")?]
    };
    #[derive(serde::Deserialize)]
    struct Frontmatter {
        description: Option<String>,
    }
    let front: Option<Frontmatter> =
        serde_yaml_ng::from_str(yaml).map_err(|error| error.to_string())?;
    Ok(front
        .and_then(|front| front.description)
        .map(|description| description.trim().to_owned())
        .filter(|description| !description.is_empty()))
}

/// Non-file children to inspect, sorted by name. An iterator error is an unnamed readdir
/// failure, so the listing is untrustworthy and fails the directory. A child whose type
/// cannot be read (e.g. removed mid-scan) is kept with its error so only that agent fails.
fn agent_candidates(
    directory: &Utf8Path,
    entries: impl IntoIterator<Item = io::Result<(OsString, io::Result<bool>)>>,
) -> Result<Vec<(OsString, io::Result<()>)>, ProjectLoadError> {
    let mut candidates = Vec::new();
    for entry in entries {
        let (name, is_file) = entry.map_err(|source| ProjectLoadError::Read {
            path: directory.to_owned(),
            source,
        })?;
        match is_file {
            Ok(true) => {}
            Ok(false) => candidates.push((name, Ok(()))),
            Err(source) => candidates.push((name, Err(source))),
        }
    }
    candidates.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(candidates)
}

fn read(path: &Utf8Path) -> Result<String, ProjectLoadError> {
    // Stat first: opening a FIFO without a writer blocks, so only regular files are opened.
    fs::metadata(path)
        .and_then(|metadata| {
            if metadata.is_file() {
                fs::read_to_string(path)
            } else {
                Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "not a regular file",
                ))
            }
        })
        .map_err(|source| {
            if source.kind() == io::ErrorKind::NotFound {
                ProjectLoadError::Missing {
                    path: path.to_owned(),
                    source,
                }
            } else {
                ProjectLoadError::Read {
                    path: path.to_owned(),
                    source,
                }
            }
        })
}

/// Load only the selected authoring Workflow; references are not validated here.
/// Relative roots are resolved against the current directory once, at invocation.
pub fn load_project(
    project_root: &Utf8Path,
    workflow_name: &str,
) -> Result<LoadedProject, ProjectLoadError> {
    load_project_candidate(project_root, workflow_name, None)
}

pub(crate) fn load_project_candidate(
    project_root: &Utf8Path,
    workflow_name: &str,
    candidate: Option<&str>,
) -> Result<LoadedProject, ProjectLoadError> {
    let project_root = absolute_root(project_root).map_err(|source| ProjectLoadError::Root {
        path: project_root.to_owned(),
        source,
    })?;
    let authoring_root = project_root.join(".unelos");
    let settings = bounded_path(
        &authoring_root,
        Utf8Path::new("project.yaml"),
        &authoring_root.join("project.yaml"),
        "project",
    )?;
    parse_project_settings(settings.as_str(), &read(&settings)?)?;
    let mut loaded = load_authoring_candidate(&authoring_root, workflow_name, candidate)?;
    loaded.project_root = project_root;
    Ok(loaded)
}

pub(crate) fn load_authoring(
    authoring_root: &Utf8Path,
    workflow_name: &str,
) -> Result<LoadedProject, ProjectLoadError> {
    load_authoring_candidate(authoring_root, workflow_name, None)
}

fn load_authoring_candidate(
    authoring_root: &Utf8Path,
    workflow_name: &str,
    candidate: Option<&str>,
) -> Result<LoadedProject, ProjectLoadError> {
    if workflow_name.is_empty()
        || matches!(workflow_name, "." | "..")
        || workflow_name.contains(['/', '\\'])
        || Utf8Path::new(workflow_name).is_absolute()
    {
        return Err(ProjectLoadError::InvalidWorkflowName {
            name: workflow_name.to_owned(),
        });
    }
    let authoring_root =
        absolute_root(authoring_root).map_err(|source| ProjectLoadError::Root {
            path: authoring_root.to_owned(),
            source,
        })?;
    let project_source = authoring_root.join("project.yaml");
    let workflow_source = bounded_path(
        &authoring_root,
        &Utf8PathBuf::from(format!("workflows/{workflow_name}.yaml")),
        &project_source,
        "workflow",
    )?;
    let mut definitions = ProjectDefinitions {
        task_definitions: BTreeMap::new(),
        agents: BTreeMap::new(),
    };
    let mut task_sources = BTreeMap::new();
    let mut agent_sources = BTreeMap::new();
    let mut instruction_dependencies = BTreeMap::new();
    for (id, source) in definition_sources(&authoring_root, "tasks", "task.yaml")? {
        let mut task = parse_task_definition(source.as_str(), &read(&source)?)?;
        resolve_instructions(
            &authoring_root,
            &source,
            "instructionsFile",
            &task.instructions_file,
            &mut task.instructions,
            &mut instruction_dependencies,
        )?;
        task_sources.insert(id.clone(), source);
        definitions.task_definitions.insert(id, task);
    }
    for (id, source) in definition_sources(&authoring_root, "agents", "agent.yaml")? {
        let mut agent = parse_agent_definition(source.as_str(), &read(&source)?)?;
        resolve_instructions(
            &authoring_root,
            &source,
            "instructionsFile",
            &agent.instructions_file,
            &mut agent.instructions,
            &mut instruction_dependencies,
        )?;
        agent_sources.insert(id.clone(), source);
        definitions.agents.insert(id, agent);
    }
    let text = match candidate {
        Some(text) => text.to_owned(),
        None => read(&workflow_source)?,
    };
    let mut workflow = parse_workflow_definition(workflow_source.as_str(), &text)?;
    if workflow.name != workflow_name {
        return Err(ProjectLoadError::NameMismatch {
            path: workflow_source,
            requested: workflow_name.to_owned(),
            declared: workflow.name,
        });
    }
    let authoring_workflow = workflow.clone();
    let expanded = crate::compose::expand(&workflow_source, &workflow, &mut |id| {
        if id.is_empty()
            || matches!(id, "." | "..")
            || id.contains(['/', '\\'])
            || Utf8Path::new(id).is_absolute()
        {
            return Err(diagnostic(
                &workflow_source,
                "workflow",
                "invalid workflow ID",
            ));
        }
        let path = bounded_path(
            &authoring_root,
            &Utf8PathBuf::from(format!("workflows/{id}.yaml")),
            &workflow_source,
            "workflow",
        )?;
        let child = parse_workflow_definition(path.as_str(), &read(&path)?)?;
        if child.name != id {
            return Err(ProjectLoadError::NameMismatch {
                path,
                requested: id.to_owned(),
                declared: child.name,
            });
        }
        Ok((path, child))
    })?;
    workflow = expanded.workflow;
    let node_sources = expanded.node_sources;
    let child_workflows = expanded.children;
    for (id, node) in &mut workflow.nodes {
        let selections: Vec<_> = match node {
            NodeDefinition::Task(task) => vec![(
                format!("nodes.{id}.taskDefinition.instructionsFile"),
                &mut task.task,
            )],
            NodeDefinition::Parallel(parallel) => parallel
                .tasks
                .iter_mut()
                .enumerate()
                .map(|(index, worker)| {
                    (
                        format!("nodes.{id}.tasks[{index}].taskDefinition.instructionsFile"),
                        &mut worker.task,
                    )
                })
                .collect(),
            _ => Vec::new(),
        };
        for (field, selection) in selections {
            if let TaskSelection::Named(name) = selection {
                if let Some(id) = name.strip_prefix("global/") {
                    if !safe_id(id) {
                        return Err(diagnostic(
                            &workflow_source,
                            "task",
                            "invalid global task ID",
                        ));
                    }
                    if !definitions.task_definitions.contains_key(name) {
                        let root = crate::task_document::global_root()
                            .map_err(|e| diagnostic(&workflow_source, "task", e))?;
                        let source = bounded_path(
                            &root,
                            &Utf8PathBuf::from(format!("tasks/{id}/task.yaml")),
                            &workflow_source,
                            "task",
                        )?;
                        match read(&source) {
                            Ok(text) => {
                                let mut task = parse_task_definition(source.as_str(), &text)?;
                                // Global dependencies are not project-relative install inputs; runtime uses resolved text.
                                resolve_instructions(
                                    &root,
                                    &source,
                                    "instructionsFile",
                                    &task.instructions_file,
                                    &mut task.instructions,
                                    &mut BTreeMap::new(),
                                )?;
                                definitions.task_definitions.insert(name.clone(), task);
                                task_sources.insert(name.clone(), source);
                            }
                            Err(ProjectLoadError::Missing { .. }) => {}
                            Err(error) => return Err(error),
                        }
                    }
                }
            }
            if let TaskSelection::Inline(task) = selection {
                resolve_instructions(
                    &authoring_root,
                    &node_sources[id].path,
                    &field,
                    &task.instructions_file,
                    &mut task.instructions,
                    &mut instruction_dependencies,
                )?;
            }
        }
    }
    Ok(LoadedProject {
        project_root: authoring_root.clone(),
        authoring_root,
        project_source,
        workflow_source,
        task_sources,
        agent_sources,
        instruction_dependencies,
        definitions,
        workflow,
        authoring_workflow,
        node_sources,
        child_workflows,
    })
}

pub(crate) fn safe_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 128
        && id
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'-' | b'_'))
}

fn diagnostic(source: &Utf8Path, field: &str, cause: impl ToString) -> ProjectLoadError {
    ProjectValidationError {
        path: source.to_owned(),
        field: field.into(),
        cause: cause.to_string(),
    }
    .into()
}

/// Normalize relative paths once, then use the installer's shared symlink checks.
pub(crate) fn bounded_path(
    root: &Utf8Path,
    relative: &Utf8Path,
    source: &Utf8Path,
    field: &str,
) -> Result<Utf8PathBuf, ProjectLoadError> {
    if relative.is_absolute() || relative.as_str().contains(['\\', ':', '\0']) {
        return Err(diagnostic(
            source,
            field,
            "expected a relative authoring path",
        ));
    }
    let mut normalized = Utf8PathBuf::new();
    for component in relative.components() {
        match component {
            camino::Utf8Component::Normal(part) => normalized.push(part),
            camino::Utf8Component::CurDir => {}
            camino::Utf8Component::ParentDir if normalized.pop() => {}
            _ => return Err(diagnostic(source, field, "path escapes authoring root")),
        }
    }
    if fs::symlink_metadata(root).is_ok_and(|metadata| metadata.file_type().is_symlink()) {
        return Err(diagnostic(source, field, "symlinked authoring root"));
    }
    crate::check_install_path(root, &normalized)
        .map_err(|error| diagnostic(source, field, error))?;
    Ok(root.join(normalized))
}

fn definition_sources(
    root: &Utf8Path,
    directory: &str,
    filename: &str,
) -> Result<BTreeMap<String, Utf8PathBuf>, ProjectLoadError> {
    let path = bounded_path(root, Utf8Path::new(directory), root, directory)?;
    let entries = match fs::read_dir(&path) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(BTreeMap::new()),
        Err(source) => return Err(ProjectLoadError::Read { path, source }),
    };
    let mut paths = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|source| ProjectLoadError::Read {
            path: path.clone(),
            source,
        })?;
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| diagnostic(&path, directory, "definition ID is not UTF-8"))?;
        paths.push(name);
    }
    paths.sort();
    let mut sources = BTreeMap::new();
    for id in paths {
        let relative = Utf8PathBuf::from(format!("{directory}/{id}"));
        let entry = bounded_path(root, &relative, &path, &id)?;
        if !entry.is_dir() {
            continue;
        }
        if !safe_id(&id) {
            return Err(diagnostic(&entry, "id", "invalid definition ID: use ASCII letters, digits, hyphen or underscore (1–128 bytes)"));
        }
        let source = bounded_path(
            root,
            Utf8Path::new(&format!("{relative}/{filename}")),
            &entry,
            filename,
        )?;
        sources.insert(id, source);
    }
    Ok(sources)
}

pub(crate) fn resolve_instructions(
    root: &Utf8Path,
    source: &Utf8Path,
    field: &str,
    file: &Option<String>,
    text: &mut String,
    dependencies: &mut BTreeMap<Utf8PathBuf, BTreeSet<Utf8PathBuf>>,
) -> Result<(), ProjectLoadError> {
    let Some(file) = file else {
        return Ok(());
    };
    let base = source.parent().unwrap().strip_prefix(root).unwrap();
    // Validate the user-supplied path before joining: joining an absolute path discards the base.
    if Utf8Path::new(file).is_absolute() || file.contains(['\\', ':', '\0']) {
        return Err(diagnostic(
            source,
            field,
            "expected a relative instructionsFile path",
        ));
    }
    let relative = base
        .iter()
        .chain(std::iter::once(file.as_str()))
        .collect::<Vec<_>>()
        .join("/");
    let path = bounded_path(root, Utf8Path::new(&relative), source, field)?;
    let relative = path.strip_prefix(root).unwrap();
    if matches!(
        relative.components().next().map(|part| part.as_str()),
        Some(
            "runs"
                | "runtime.lock"
                | "run-sequence"
                | "rename.json"
                | "opencode"
                | "project.yaml"
                | "integration.json"
                | "active-run"
                | "README.md"
        )
    ) {
        return Err(diagnostic(source, field, "instructionsFile must reference authoring, not settings, integration, documentation or runtime state"));
    }
    *text = read(&path).map_err(|error| diagnostic(source, field, error))?;
    dependencies
        .entry(source.to_owned())
        .or_default()
        .insert(relative.to_owned());
    Ok(())
}

pub(crate) fn absolute_root(project_root: &Utf8Path) -> io::Result<Utf8PathBuf> {
    if project_root.is_absolute() {
        return Ok(project_root.to_owned());
    }
    let cwd = Utf8PathBuf::from_path_buf(std::env::current_dir()?).map_err(|_| {
        io::Error::new(io::ErrorKind::InvalidData, "current directory is not UTF-8")
    })?;
    Ok(cwd.join(project_root))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agent_candidates_isolate_child_metadata_failures() {
        let entries = vec![
            Ok((
                OsString::from("gone"),
                Err(io::Error::from(io::ErrorKind::NotFound)),
            )),
            Ok((OsString::from("file"), Ok(true))),
            Ok((OsString::from("agent"), Ok(false))),
        ];
        let candidates = agent_candidates(Utf8Path::new("agents"), entries).unwrap();
        assert_eq!(candidates.len(), 2);
        assert_eq!(candidates[0].0, "agent");
        assert!(candidates[0].1.is_ok());
        assert_eq!(candidates[1].0, "gone");
        assert_eq!(
            candidates[1].1.as_ref().unwrap_err().kind(),
            io::ErrorKind::NotFound
        );
    }

    #[test]
    fn agent_candidates_fail_directory_on_unnamed_iterator_error() {
        let entries = vec![Err(io::Error::other("readdir failed"))];
        assert!(agent_candidates(Utf8Path::new("agents"), entries).is_err());
    }
}
