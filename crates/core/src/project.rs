use camino::{Utf8Path, Utf8PathBuf};
use std::{fs, io};

use crate::{
    parse_project_definitions, parse_workflow_definition, DefinitionParseError, ProjectDefinitions,
    WorkflowDefinition,
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
    #[error("Workflow {path} declares name {declared:?}, requested {requested:?}")]
    NameMismatch {
        path: Utf8PathBuf,
        requested: String,
        declared: String,
    },
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
    let project_root = if project_root.is_absolute() {
        project_root.to_owned()
    } else {
        let cwd = std::env::current_dir().map_err(|source| ProjectLoadError::Root {
            path: project_root.to_owned(),
            source,
        })?;
        let cwd = Utf8PathBuf::from_path_buf(cwd).map_err(|_| ProjectLoadError::Root {
            path: project_root.to_owned(),
            source: io::Error::new(io::ErrorKind::InvalidData, "current directory is not UTF-8"),
        })?;
        cwd.join(project_root)
    };
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
