//! Shared application logic for the CLI and desktop adapters.

mod authoring_actions;
pub use authoring_actions::{make_mcp_global, make_skill_global, make_task_global};
mod context;
pub use authoring_actions::{
    duplicate_agent, duplicate_global_agent, duplicate_global_skill, duplicate_global_task,
    duplicate_global_workflow, duplicate_skill, duplicate_task, duplicate_workflow, remove_agent,
    remove_global_agent, remove_global_skill, remove_global_task, remove_global_workflow,
    remove_skill, remove_task, remove_workflow,
};
mod workflow_document;
pub use workflow_document::{
    create_global_workflow, create_workflow, get_global_workflow, get_workflow,
    save_global_workflow, save_workflow, validate_global_workflow, validate_workflow,
    workflow_warnings, IssueSeverity, StepCall, StepTask, WorkflowDocument, WorkflowDraft,
    WorkflowIssue, WorkflowRoute, WorkflowSettings, WorkflowStep, WorkflowWorker,
};
mod task_document;
pub use task_document::{
    get_global_task, get_task, save_global_task, save_task, save_task_with_outcome_renames,
    TaskDocument,
};

mod agent_document;
mod harness_build;
mod mcp_document;
mod mcp_install;
mod uninstall;
pub use uninstall::{uninstall_project, ProjectUninstall};
pub mod opencode_agents;
pub use agent_document::{
    agent_model_options, get_agent, get_global_agent, save_agent, save_global_agent, AgentDocument,
    AgentModelOptions, HarnessModelOptions,
};
pub use harness_build::{
    auto_build_harness, build_global_harness_agents, build_harness_config, get_project_settings,
    save_project_settings, HarnessBuild,
};
pub use mcp_document::{
    get_global_mcp, get_mcp, global_mcp_summaries, mcp_summaries, remove_global_mcp, remove_mcp,
    save_global_mcp, save_mcp, McpDocument, McpServer, McpSummary,
};
pub use mcp_install::{
    check_opencode_mcps, replace_opencode_mcps, uninstall_opencode_mcps, write_opencode_mcps,
};
mod model_mappings;
pub use model_mappings::{
    delete_model_mapping, list_model_mappings, save_model_mapping, MappedModel, ModelMappings,
};
pub mod global_config;
pub mod harness;
pub use harness::{
    harness_models, install_harness, list_harnesses, provision_cli, uninstall_harness,
    HarnessStatus, ModelInfo,
};
pub mod compose;
mod definitions;
pub use compose::{artifact_file_name, NodeSource};
mod execution;
mod graph;
mod import;
mod install;
pub use import::{
    apply_renames, commit_import, fetch_source, parse_source_spec, plan_import, FetchedSource,
    ImportConflict, ImportItem, ImportKind, ImportMcp, ImportOutcome, ImportPlan, ImportWarning,
    ImportWorkflow, SourceSpec,
};
mod project;
pub use project::{global_workflow_summaries, project_references, ProjectReferences, Reference};
mod runtime;
mod settings;
mod skill_document;
mod skills;
pub use skill_document::{
    create_global_skill, create_skill, get_global_skill, get_skill, save_global_skill, save_skill,
    SaveSkillResult, SkillDocument,
};
mod validation;
mod work;

pub use context::{
    build_task_packet, inspect_task, ArtifactReference, ContextError, TaskContext, TaskInspection,
    TaskPacket, TaskRuntimeInput, TransitionFeedback,
};
pub use definitions::*;
pub use execution::{
    parse_parallel_results, parse_worker_result, read_brief, render_execution_package,
    CompletionControl, ExecutionEntry, ExecutionError, ExecutionIdentity, ExecutionOperation,
    ExecutionPackage, ResumeContext, ResumeFile, RunCatalog, RunSummary, WorkerPrompt,
    WorkerResult, WorkerStatus,
};
pub use graph::{build_graph, GraphEdge, GraphNode, NodeKind, WorkflowGraph};
pub use install::{
    check_install_path, check_project_init, export_workflow_agents, initialize_project,
    install_workflow, write_install_file, AgentExport, InstallError, WorkflowInstall,
};
pub use project::{
    agent_summaries, global_agent_summaries, global_skill_summaries, global_task_summaries,
    list_workflow_descriptions, list_workflows, load_project, skill_summaries, task_summaries,
    workflow_summaries, AgentSummary, LoadedProject, ProjectLoadError, SkillSummary, TaskSummary,
    WorkflowSummary,
};
pub use runtime::{
    approve, approve_node, complete, complete_node, confirm_outputs, delete_run, execution_mutate,
    execution_new, execution_next, execution_resume, execution_runs, execution_start,
    execution_start_inline, get_run, get_run_workflow, list_runs, next, prepare_work,
    read_run_artifact, reject, reject_node, rename_run, start, start_workflow, status,
    verify_outputs, verify_work_outputs, CompletedExecution, CurrentWork, ExecutionRecord,
    InlineStep, RunDetails, RunListItem, RunState, RuntimeError,
};
pub use settings::{Settings, SettingsError};
pub use skills::{resolve_skills, ResolvedSkill, SkillResolveError};
pub use validation::{validate_project, ProjectValidationError};
pub use work::{
    build_work_packet, inspect_node, EndInspection, FinishedPacket, GateInspection, GatePacket,
    NodeInspection, ParallelInspection, ParallelPacket, TaskWorkPacket, WorkPacket,
    WorkRuntimeInput, WorkerInspection, WorkerPacket, WorkerReportBinding,
};
pub mod controller_mcp;
pub mod session_binding;
