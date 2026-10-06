//! Shared application logic for the CLI and desktop adapters.

mod context;
mod definitions;
mod graph;
mod project;
mod settings;
mod skills;
mod validation;
mod work;

pub use context::{
    build_task_packet, inspect_task, ArtifactReference, ContextError, TaskContext, TaskInspection,
    TaskPacket, TaskRuntimeInput, TransitionFeedback,
};
pub use definitions::*;
pub use graph::{build_graph, GraphEdge, GraphNode, NodeKind, WorkflowGraph};
pub use project::{load_project, LoadedProject, ProjectLoadError};
pub use settings::{Settings, SettingsError};
pub use skills::{resolve_skills, ResolvedSkill, SkillResolveError};
pub use validation::{validate_project, ProjectValidationError};
pub use work::{
    inspect_node, EndInspection, GateInspection, NodeInspection, ParallelInspection,
    WorkerInspection,
};
