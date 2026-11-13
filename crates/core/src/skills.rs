use camino::{Utf8Path, Utf8PathBuf};
use serde::Serialize;
use std::{collections::HashSet, fs, io};

use crate::project::absolute_root;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedSkill {
    pub id: String,
    pub directory: Utf8PathBuf,
    pub content: String,
}

#[derive(Debug, thiserror::Error)]
pub enum SkillResolveError {
    #[error("invalid Skill ID {id:?}: expected a nonempty local directory identifier")]
    InvalidId { id: String },
    #[error("could not resolve project root {path}: {source}")]
    Root {
        path: Utf8PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("missing Skill {id:?} file {path}: {source}")]
    Missing {
        id: String,
        path: Utf8PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("could not read Skill {id:?} file {path}: {source}")]
    Read {
        id: String,
        path: Utf8PathBuf,
        #[source]
        source: io::Error,
    },
}

/// Read only explicitly selected local Skills, preserving first-occurrence order.
/// Content includes frontmatter; resources are accessible through `directory`, not injected.
/// Relative roots are resolved against the current directory once, at invocation.
pub fn resolve_skills(
    project_root: &Utf8Path,
    ordered_ids: &[impl AsRef<str>],
) -> Result<Vec<ResolvedSkill>, SkillResolveError> {
    let project_root = absolute_root(project_root).map_err(|source| SkillResolveError::Root {
        path: project_root.to_owned(),
        source,
    })?;
    resolve_authoring_skills(&project_root.join(".unelos"), ordered_ids)
}

pub(crate) fn resolve_authoring_skills(
    authoring_root: &Utf8Path,
    ordered_ids: &[impl AsRef<str>],
) -> Result<Vec<ResolvedSkill>, SkillResolveError> {
    let mut seen = HashSet::new();
    let mut skills = Vec::new();
    for id in ordered_ids {
        let id = id.as_ref();
        let local_id = id.strip_prefix("global/").unwrap_or(id);
        if crate::skill_document::valid_id(local_id).is_err() {
            return Err(SkillResolveError::InvalidId { id: id.to_owned() });
        }
        if !seen.insert(id) {
            continue;
        }
        let global_root;
        let root = if id.starts_with("global/") {
            global_root = crate::task_document::global_root().map_err(|error| SkillResolveError::Root {
                path: authoring_root.to_owned(),
                source: io::Error::other(error),
            })?;
            &global_root
        } else {
            authoring_root
        };
        let directory = root.join("skills").join(local_id);
        let path = directory.join("SKILL.md");
        crate::check_install_path(
            root,
            &Utf8PathBuf::from("skills").join(local_id).join("SKILL.md"),
        )
        .map_err(|error| SkillResolveError::Read {
            id: id.into(),
            path: path.clone(),
            source: io::Error::new(io::ErrorKind::InvalidInput, error),
        })?;
        let content = fs::read_to_string(&path).map_err(|source| {
            if source.kind() == io::ErrorKind::NotFound {
                SkillResolveError::Missing {
                    id: id.to_owned(),
                    path,
                    source,
                }
            } else {
                SkillResolveError::Read {
                    id: id.to_owned(),
                    path,
                    source,
                }
            }
        })?;
        skills.push(ResolvedSkill {
            id: id.to_owned(),
            directory,
            content,
        });
    }
    Ok(skills)
}
