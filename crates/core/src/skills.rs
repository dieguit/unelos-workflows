use camino::{Utf8Path, Utf8PathBuf};
use std::{collections::HashSet, fs, io};

use crate::project::absolute_root;

#[derive(Debug, Clone, PartialEq, Eq)]
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
    let mut seen = HashSet::new();
    let mut skills = Vec::new();
    for id in ordered_ids {
        let id = id.as_ref();
        if id.is_empty() || matches!(id, "." | "..") || id.contains(['/', '\\', ':', '\0']) {
            return Err(SkillResolveError::InvalidId { id: id.to_owned() });
        }
        if !seen.insert(id) {
            continue;
        }
        let directory = project_root.join(".unelos/skills").join(id);
        let path = directory.join("SKILL.md");
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
