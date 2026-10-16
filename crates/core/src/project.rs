use camino::{Utf8Path, Utf8PathBuf};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs, io,
};

use crate::{
    parse_agent_definition, parse_project_settings, parse_task_definition,
    parse_workflow_definition, DefinitionParseError, NodeDefinition, ProjectDefinitions,
    ProjectValidationError, TaskSelection, WorkflowDefinition,
};

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
                    cause: format!("unknown Task Definition {id:?}"),
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
    pub name: String,
    pub file: String,
    pub description: Option<String>,
    pub steps: usize,
    pub problem: Option<String>,
}

pub fn workflow_summaries(
    project_root: &Utf8Path,
) -> Result<Vec<WorkflowSummary>, ProjectLoadError> {
    let (root, names) = workflow_files(project_root)?;
    Ok(names
        .into_iter()
        .map(|name| {
            let file = format!("{name}.yaml");
            match load_project(&root, &name) {
                Ok(project) => WorkflowSummary {
                    problem: crate::validate_project(&project)
                        .err()
                        .map(|error| format!("{}: {}", error.field, error.cause)),
                    description: project.workflow.description.clone(),
                    steps: project
                        .workflow
                        .nodes
                        .values()
                        .filter(|node| !matches!(node, NodeDefinition::End))
                        .count(),
                    name,
                    file,
                },
                Err(error) => WorkflowSummary {
                    problem: Some(error.to_string()),
                    description: None,
                    steps: 0,
                    name,
                    file,
                },
            }
        })
        .collect())
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

fn read(path: &Utf8Path) -> Result<String, ProjectLoadError> {
    fs::read_to_string(path).map_err(|source| {
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
    let mut loaded = load_authoring(&authoring_root, workflow_name)?;
    loaded.project_root = project_root;
    Ok(loaded)
}

pub(crate) fn load_authoring(
    authoring_root: &Utf8Path,
    workflow_name: &str,
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
    let mut workflow =
        parse_workflow_definition(workflow_source.as_str(), &read(&workflow_source)?)?;
    if workflow.name != workflow_name {
        return Err(ProjectLoadError::NameMismatch {
            path: workflow_source,
            requested: workflow_name.to_owned(),
            declared: workflow.name,
        });
    }
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
            if let TaskSelection::Inline(task) = selection {
                resolve_instructions(
                    &authoring_root,
                    &workflow_source,
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
        let relative = Utf8PathBuf::from(directory).join(&id);
        let entry = bounded_path(root, &relative, &path, &id)?;
        if !entry.is_dir() {
            continue;
        }
        if !safe_id(&id) {
            return Err(diagnostic(&entry, "id", "invalid definition ID: use ASCII letters, digits, hyphen or underscore (1–128 bytes)"));
        }
        let source = bounded_path(root, &relative.join(filename), &entry, filename)?;
        sources.insert(id, source);
    }
    Ok(sources)
}

fn resolve_instructions(
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
    let path = bounded_path(root, &base.join(file), source, field)?;
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
