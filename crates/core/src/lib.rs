//! Shared application logic for the CLI and desktop adapters.

mod context;
mod definitions;
mod graph;
mod project;
mod runtime;
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
pub use project::{list_workflows, load_project, LoadedProject, ProjectLoadError};
pub use runtime::{
    approve, approve_node, complete, complete_node, confirm_outputs, next, prepare_work, reject,
    reject_node, rename_run, start, start_workflow, status, verify_outputs, verify_work_outputs,
    CompletedExecution, CurrentWork, ExecutionRecord, RunState, RuntimeError,
};
pub use settings::{Settings, SettingsError};
pub use skills::{resolve_skills, ResolvedSkill, SkillResolveError};
pub use validation::{validate_project, ProjectValidationError};
pub use work::{
    build_work_packet, inspect_node, EndInspection, FinishedPacket, GateInspection, GatePacket,
    NodeInspection, ParallelInspection, ParallelPacket, TaskWorkPacket, WorkPacket,
    WorkRuntimeInput, WorkerInspection, WorkerPacket, WorkerReportBinding,
};
