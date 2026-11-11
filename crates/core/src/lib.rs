//! Shared application logic for the CLI and desktop adapters.

mod context;
mod authoring_actions;
pub use authoring_actions::{duplicate_global_task, remove_global_task, duplicate_task, remove_task, duplicate_agent, duplicate_skill, duplicate_workflow, remove_agent, remove_skill, remove_workflow};
mod workflow_document;
pub use workflow_document::{get_workflow, validate_workflow, save_workflow, WorkflowDocument, WorkflowDraft, StepTask, WorkflowSettings, WorkflowStep, WorkflowRoute, WorkflowWorker, WorkflowIssue};
mod task_document;
pub use task_document::{get_global_task, save_global_task, get_task, save_task, save_task_with_outcome_renames, TaskDocument};

mod agent_document;
pub use agent_document::{get_agent, save_agent, agent_model_options, AgentDocument, AgentModelOptions, HarnessModelOptions};
mod model_mappings;
pub use model_mappings::{delete_model_mapping, list_model_mappings, save_model_mapping, ModelMappings};
pub mod global_config;
pub mod harness;
pub use harness::{
    harness_models, install_harness, list_harnesses, provision_cli, uninstall_harness, HarnessStatus,
};
mod definitions;
mod execution;
mod graph;
mod install;
mod project;
mod runtime;
mod settings;
mod skills;
mod skill_document;
pub use skill_document::{get_skill, save_skill, SaveSkillResult, SkillDocument};
mod validation;
mod work;

pub use context::{
    build_task_packet, inspect_task, ArtifactReference, ContextError, TaskContext, TaskInspection,
    TaskPacket, TaskRuntimeInput, TransitionFeedback,
};
pub use definitions::*;
pub use execution::{
    parse_parallel_results, parse_worker_result, read_brief, render_execution_package,
    ContinuationControl, ExecutionControl, ExecutionEntry, ExecutionError, ExecutionIdentity,
    ExecutionOperation, ExecutionPackage, ResumeContext, ResumeFile, RunCatalog, RunSummary,
    WorkerPrompt, WorkerResult, WorkerStatus,
};
pub use graph::{build_graph, GraphEdge, GraphNode, NodeKind, WorkflowGraph};
pub use install::{
    check_install_path, check_project_init, export_workflow_agents, initialize_project,
    install_workflow, write_install_file, AgentExport, InstallError, WorkflowInstall,
};
pub use project::{
    global_task_summaries, task_summaries, TaskSummary, agent_summaries, list_workflows, load_project, skill_summaries, workflow_summaries,
    AgentSummary, LoadedProject, ProjectLoadError, SkillSummary,
    WorkflowSummary,
};
pub use runtime::{
    approve, approve_node, complete, complete_node, confirm_outputs, execution_continue,
    execution_mutate, execution_new, execution_next, execution_resume, execution_runs,
    execution_start, list_runs, next, prepare_work, reject, reject_node, rename_run, start,
    start_workflow, status, verify_outputs, verify_work_outputs, CompletedExecution, CurrentWork,
    ExecutionRecord, RunState, RuntimeError,
};
pub use settings::{Settings, SettingsError};
pub use skills::{resolve_skills, ResolvedSkill, SkillResolveError};
pub use validation::{validate_project, ProjectValidationError};
pub use work::{
    build_work_packet, inspect_node, EndInspection, FinishedPacket, GateInspection, GatePacket,
    NodeInspection, ParallelInspection, ParallelPacket, TaskWorkPacket, WorkPacket,
    WorkRuntimeInput, WorkerInspection, WorkerPacket, WorkerReportBinding,
};
