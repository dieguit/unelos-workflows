use camino::{Utf8Path, Utf8PathBuf};
use std::{fs, io};

use crate::{
    parse_project_definitions, parse_workflow_definition, DefinitionParseError, ProjectDefinitions,
    ProjectValidationError, WorkflowDefinition,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadedProject {
    pub project_root: Utf8PathBuf,
    pub project_source: Utf8PathBuf,
    pub workflow_source: Utf8PathBuf,
    pub definitions: ProjectDefinitions,
    pub workflow: WorkflowDefinition,
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
    let root = absolute_root(project_root).map_err(|source| ProjectLoadError::Root {
        path: project_root.to_owned(),
        source,
    })?;
    let definitions_path = root.join(".unelos/project.yaml");
    parse_project_definitions(definitions_path.as_str(), &read(&definitions_path)?)?;

    let directory = root.join(".unelos/workflows");
    let entries = match fs::read_dir(&directory) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(source) => {
            return Err(ProjectLoadError::Read {
                path: directory,
                source,
            })
        }
    };
    let mut workflows = Vec::new();
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
        let project = load_project(&root, &name)?;
        crate::validate_project(&project)?;
        workflows.push(name);
    }
    workflows.sort();
    Ok(workflows)
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
    if workflow_name.is_empty()
        || matches!(workflow_name, "." | "..")
        || workflow_name.contains(['/', '\\'])
        || Utf8Path::new(workflow_name).is_absolute()
    {
        return Err(ProjectLoadError::InvalidWorkflowName {
            name: workflow_name.to_owned(),
        });
    }
    let project_root = absolute_root(project_root).map_err(|source| ProjectLoadError::Root {
        path: project_root.to_owned(),
        source,
    })?;
    let project_source = project_root.join(".unelos/project.yaml");
    let workflow_source = project_root.join(format!(".unelos/workflows/{workflow_name}.yaml"));
    let definitions = parse_project_definitions(project_source.as_str(), &read(&project_source)?)?;
    let workflow = parse_workflow_definition(workflow_source.as_str(), &read(&workflow_source)?)?;
    if workflow.name != workflow_name {
        return Err(ProjectLoadError::NameMismatch {
            path: workflow_source,
            requested: workflow_name.to_owned(),
            declared: workflow.name,
        });
    }
    Ok(LoadedProject {
        project_root,
        project_source,
        workflow_source,
        definitions,
        workflow,
    })
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
