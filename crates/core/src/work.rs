use camino::Utf8PathBuf;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::{
    context::{ordered_bindings, resolve_task_context},
    ArtifactReference, ContextError, Execution, LoadedProject, NodeDefinition, TaskContext,
    TaskInspection, TaskPacket, TaskRuntimeInput, TransitionFeedback,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkerReportBinding {
    pub key: String,
    pub report: ArtifactReference,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkRuntimeInput {
    Task {
        input: TaskRuntimeInput,
        ledger_path: Utf8PathBuf,
    },
    Parallel {
        input: TaskRuntimeInput,
        worker_reports: Vec<WorkerReportBinding>,
        ledger_path: Utf8PathBuf,
    },
    HumanGate {
        run_id: String,
        activation_id: String,
        node_id: String,
        artifacts: Vec<ArtifactReference>,
        ledger_path: Utf8PathBuf,
    },
    Finished {
        run_id: String,
        node_id: String,
        ledger_path: Utf8PathBuf,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskWorkPacket {
    #[serde(flatten)]
    pub packet: TaskPacket,
    pub ledger_path: Utf8PathBuf,
    pub controller_instructions: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkerPacket {
    pub key: String,
    #[serde(flatten)]
    pub context: TaskContext,
    pub run_id: String,
    pub parent_task_id: String,
    pub request: String,
    pub inputs: Vec<ArtifactReference>,
    pub outputs: Vec<ArtifactReference>,
    pub feedback: Option<TransitionFeedback>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ParallelPacket {
    pub kind: &'static str,
    pub run_id: String,
    pub task_id: String,
    pub node_id: String,
    pub request: String,
    pub inputs: Vec<ArtifactReference>,
    pub outputs: Vec<ArtifactReference>,
    pub feedback: Option<TransitionFeedback>,
    pub workers: Vec<WorkerPacket>,
    pub ledger_path: Utf8PathBuf,
    pub controller_instructions: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GatePacket {
    pub kind: &'static str,
    pub run_id: String,
    pub activation_id: String,
    pub node_id: String,
    pub artifacts: Vec<ArtifactReference>,
    pub decisions: Vec<String>,
    pub rejection_reason_required: bool,
    pub ledger_path: Utf8PathBuf,
    pub controller_instructions: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FinishedPacket {
    pub kind: &'static str,
    pub run_id: String,
    pub node_id: String,
    pub remaining_work: bool,
    pub ledger_path: Utf8PathBuf,
    pub controller_instructions: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub enum WorkPacket {
    Task(Box<TaskWorkPacket>),
    Parallel(ParallelPacket),
    HumanGate(GatePacket),
    Finished(FinishedPacket),
}

const LEDGER_DUTY: &str = "Only the controller updates the supplied run ledger: record progress, received decisions, and descriptions of artifact changes. The ledger is informational; use Unelos for current work, completion, and transitions. Never use the ledger to derive destinations. Read existing declared outputs before updating them in place; preserve relevant content and do not recreate the run tree.";
const OUTPUT_CONFIRMATION_DUTY: &str = "Before reporting future completion, explicitly confirm all parent outputs for the current Task ID through Unelos; unchanged reviewed outputs may be confirmed. Workers report produced or reviewed outputs to the controller. Workers never edit the ledger or authorize progression.";

pub fn build_work_packet(
    project: &LoadedProject,
    input: WorkRuntimeInput,
) -> Result<WorkPacket, ContextError> {
    match input {
        WorkRuntimeInput::Task { input, ledger_path } => {
            let packet = crate::build_task_packet(project, input)?;
            Ok(WorkPacket::Task(Box::new(TaskWorkPacket {
                packet,
                ledger_path,
                controller_instructions: format!("{LEDGER_DUTY} {OUTPUT_CONFIRMATION_DUTY}"),
            })))
        }
        WorkRuntimeInput::Parallel {
            input,
            worker_reports,
            ledger_path,
        } => {
            let node_id = input.node_id.as_str();
            if !matches!(
                project.workflow.nodes.get(node_id),
                Some(NodeDefinition::Parallel(_))
            ) {
                return Err(diagnostic(
                    project,
                    format!("nodes.{node_id}"),
                    "runtime input requires a parallel Node",
                ));
            }
            let NodeInspection::Parallel(inspection) = inspect_node(project, node_id)? else {
                unreachable!()
            };
            let inputs =
                ordered_bindings(project, node_id, "inputs", inspection.inputs, input.inputs)?;
            let outputs = ordered_bindings(
                project,
                node_id,
                "outputs",
                inspection.outputs,
                input.outputs,
            )?;
            let mut reports = BTreeMap::new();
            for binding in worker_reports {
                let key = binding.key.clone();
                if reports.insert(key.clone(), binding.report).is_some() {
                    return Err(worker_error(
                        project,
                        node_id,
                        &key,
                        "duplicate worker binding",
                    ));
                }
            }
            for key in reports.keys() {
                if !inspection.workers.iter().any(|worker| worker.key == *key) {
                    return Err(worker_error(
                        project,
                        node_id,
                        key,
                        "unknown worker binding",
                    ));
                }
            }
            let mut workers = Vec::with_capacity(inspection.workers.len());
            for worker in inspection.workers {
                let report = reports.remove(&worker.key).ok_or_else(|| {
                    worker_error(project, node_id, &worker.key, "missing worker binding")
                })?;
                if report.name != "result.md" {
                    return Err(worker_error(
                        project,
                        node_id,
                        &worker.key,
                        format!("report name {:?} must be result.md", report.name),
                    ));
                }
                if outputs.iter().any(|output| output.path == report.path)
                    || ledger_path == report.path
                {
                    return Err(worker_error(
                        project,
                        node_id,
                        &worker.key,
                        "report path collides with another output",
                    ));
                }
                if workers
                    .iter()
                    .any(|previous: &WorkerPacket| previous.outputs[0].path == report.path)
                {
                    return Err(worker_error(
                        project,
                        node_id,
                        &worker.key,
                        "duplicate report path",
                    ));
                }
                workers.push(WorkerPacket {
                    key: worker.key,
                    context: worker.context,
                    run_id: input.run_id.clone(),
                    parent_task_id: input.task_id.clone(),
                    request: input.request.clone(),
                    inputs: inputs.clone(),
                    outputs: vec![report],
                    feedback: input.feedback.clone(),
                });
            }
            Ok(WorkPacket::Parallel(ParallelPacket {
                kind: "parallel", run_id: input.run_id, task_id: input.task_id,
                node_id: input.node_id, request: input.request, inputs, outputs,
                feedback: input.feedback, workers, ledger_path,
                controller_instructions: format!("{LEDGER_DUTY} {OUTPUT_CONFIRMATION_DUTY} Dispatch every listed worker; relay questions and resume the same logical work. Wait for all reports, read them, then update aggregate outputs. A worker return or question never transitions the Node. Never complete a blocked or partial parent; report completion through Unelos."),
            }))
        }
        WorkRuntimeInput::HumanGate {
            run_id,
            activation_id,
            node_id,
            artifacts,
            ledger_path,
        } => {
            if !matches!(
                project.workflow.nodes.get(&node_id),
                Some(NodeDefinition::HumanGate(_))
            ) {
                return Err(diagnostic(
                    project,
                    format!("nodes.{node_id}"),
                    "runtime input requires a human-gate Node",
                ));
            }
            let NodeInspection::HumanGate(inspection) = inspect_node(project, &node_id)? else {
                unreachable!()
            };
            let artifacts =
                ordered_bindings(project, &node_id, "inputs", inspection.inputs, artifacts)?;
            Ok(WorkPacket::HumanGate(GatePacket {
                kind: "human-gate", run_id, activation_id, node_id, artifacts,
                decisions: inspection.decisions, rejection_reason_required: inspection.rejection_reason_required,
                ledger_path, controller_instructions: format!("{LEDGER_DUTY} Present the exact supplied artifacts and obtain an explicit user decision. Require a nonempty rejection reason. Record the decision through Unelos; ledger notes do not authorize progression."),
            }))
        }
        WorkRuntimeInput::Finished {
            run_id,
            node_id,
            ledger_path,
        } => {
            if !matches!(
                project.workflow.nodes.get(&node_id),
                Some(NodeDefinition::End)
            ) {
                return Err(diagnostic(
                    project,
                    format!("nodes.{node_id}"),
                    "runtime input requires an End Node",
                ));
            }
            Ok(WorkPacket::Finished(FinishedPacket {
                kind: "finished",
                run_id,
                node_id,
                remaining_work: false,
                ledger_path,
                controller_instructions: format!(
                    "{LEDGER_DUTY} Record only the final status reported by Unelos."
                ),
            }))
        }
    }
}

fn worker_error(
    project: &LoadedProject,
    node_id: &str,
    key: &str,
    cause: impl Into<String>,
) -> ContextError {
    diagnostic(
        project,
        format!("nodes.{node_id}.workers.{key}.outputs"),
        cause,
    )
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkerInspection {
    pub key: String,
    #[serde(flatten)]
    pub context: TaskContext,
    pub inputs: Vec<String>,
    pub outputs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ParallelInspection {
    pub kind: &'static str,
    pub node_id: String,
    pub inputs: Vec<String>,
    pub outputs: Vec<String>,
    pub workers: Vec<WorkerInspection>,
    pub controller_instructions: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GateInspection {
    pub kind: &'static str,
    pub node_id: String,
    pub inputs: Vec<String>,
    pub decisions: Vec<String>,
    pub rejection_reason_required: bool,
    pub controller_instructions: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EndInspection {
    pub kind: &'static str,
    pub node_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub enum NodeInspection {
    Task(Box<TaskInspection>),
    Parallel(ParallelInspection),
    HumanGate(GateInspection),
    End(EndInspection),
}

pub fn inspect_node(
    project: &LoadedProject,
    node_id: &str,
) -> Result<NodeInspection, ContextError> {
    let node = project
        .workflow
        .nodes
        .get(node_id)
        .ok_or_else(|| diagnostic(project, format!("nodes.{node_id}"), "unknown Node"))?;
    match node {
        NodeDefinition::Task(_) => crate::inspect_task(project, node_id)
            .map(|inspection| NodeInspection::Task(Box::new(inspection))),
        NodeDefinition::Parallel(parallel) => {
            if parallel.tasks.is_empty() {
                return Err(diagnostic(
                    project,
                    format!("nodes.{node_id}.tasks"),
                    "parallel node must declare at least one worker",
                ));
            }
            let workers = parallel
                .tasks
                .iter()
                .enumerate()
                .map(|(index, worker)| {
                    for (field, exists) in [
                        (
                            "task",
                            project
                                .definitions
                                .task_definitions
                                .contains_key(&worker.task),
                        ),
                        (
                            "agent",
                            project.definitions.agents.contains_key(&worker.agent),
                        ),
                    ] {
                        if !exists {
                            return Err(diagnostic(
                                project,
                                format!("nodes.{node_id}.tasks[{index}].{field}"),
                                format!("referenced {} is missing", field.to_ascii_uppercase()),
                            ));
                        }
                    }
                    Ok(WorkerInspection {
                        key: format!("worker-{}", index + 1),
                        context: resolve_task_context(
                            project,
                            node_id,
                            &worker.task,
                            &worker.agent,
                            Execution::Subagent,
                        )?,
                        inputs: parallel.inputs.clone(),
                        outputs: vec!["result.md".into()],
                    })
                })
                .collect::<Result<_, _>>()?;
            Ok(NodeInspection::Parallel(ParallelInspection {
                kind: "parallel",
                node_id: node_id.into(),
                inputs: parallel.inputs.clone(),
                outputs: parallel.outputs.clone(),
                workers,
                controller_instructions: "Dispatch every worker. Relay questions and answers between workers and the controller, wait for every required report, read all reports, and update every declared aggregate output in place while preserving relevant content. Record progress and artifact-change descriptions in the supplied ledger; only the controller writes ledger notes. A worker return or question never transitions the Node; do not advance until all reports are handled and completion is reported through Unelos.".into(),
            }))
        }
        NodeDefinition::HumanGate(gate) => {
            if gate.next.len() != 2
                || !gate.next.contains_key("approved")
                || !gate.next.contains_key("rejected")
            {
                return Err(diagnostic(
                    project,
                    format!("nodes.{node_id}.next"),
                    "human gate requires exactly approved and rejected routes",
                ));
            }
            Ok(NodeInspection::HumanGate(GateInspection {
                kind: "human-gate",
                node_id: node_id.into(),
                inputs: gate.inputs.clone(),
                decisions: vec!["approved".into(), "rejected".into()],
                rejection_reason_required: true,
                controller_instructions: "Present the declared logical artifacts for the human decision. Require a reason when rejected, record the explicit decision through Unelos, and record progress in the supplied ledger as controller-only notes. Ledger notes never authorize progression.".into(),
            }))
        }
        NodeDefinition::End => Ok(NodeInspection::End(EndInspection {
            kind: "end",
            node_id: node_id.into(),
        })),
    }
}

fn diagnostic(project: &LoadedProject, field: String, cause: impl Into<String>) -> ContextError {
    ContextError::Diagnostic {
        path: project.workflow_source.clone(),
        field,
        cause: cause.into(),
    }
}
