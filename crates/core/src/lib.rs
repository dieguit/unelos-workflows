//! Shared application logic for the CLI and desktop adapters.

mod definitions;
mod project;
mod settings;
mod skills;
mod validation;

pub use definitions::*;
pub use project::{load_project, LoadedProject, ProjectLoadError};
pub use settings::{Settings, SettingsError};
pub use skills::{resolve_skills, ResolvedSkill, SkillResolveError};
pub use validation::{validate_project, ProjectValidationError};
