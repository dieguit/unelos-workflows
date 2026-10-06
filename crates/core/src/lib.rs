//! Shared application logic for the CLI and desktop adapters.

mod definitions;
mod project;
mod settings;

pub use definitions::*;
pub use project::{load_project, LoadedProject, ProjectLoadError};
pub use settings::{Settings, SettingsError};
