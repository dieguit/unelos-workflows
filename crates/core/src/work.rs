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
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub missing_inputs: Vec<String>,
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
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub missing_inputs: Vec<String>,
    pub outputs: Vec<ArtifactReference>,
    pub feedback: Option<TransitionFeedback>,
    pub workers: Vec<WorkerPacket>,
    pub ledger_path: Utf8PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GatePacket {
    pub kind: &'static str,
    pub run_id: String,
    pub activation_id: String,
    pub node_id: String,
    pub artifacts: Vec<ArtifactReference>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub missing_inputs: Vec<String>,
    pub ledger_path: Utf8PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FinishedPacket {
    pub kind: &'static str,
    pub run_id: String,
    pub node_id: String,
    pub ledger_path: Utf8PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub enum WorkPacket {
    Task(Box<TaskWorkPacket>),
    Parallel(ParallelPacket),
    HumanGate(GatePacket),
    Finished(FinishedPacket),
}

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
                    missing_inputs: input.missing_inputs.clone(),
                    outputs: vec![report],
                    feedback: input.feedback.clone(),
                });
            }
            Ok(WorkPacket::Parallel(ParallelPacket {
                kind: "parallel",
                run_id: input.run_id,
                task_id: input.task_id,
                node_id: input.node_id,
                request: input.request,
                inputs,
                missing_inputs: input.missing_inputs,
                outputs,
                feedback: input.feedback,
                workers,
                ledger_path,
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
            let missing_inputs = crate::runtime::missing_inputs(project, &node_id, &artifacts);
            Ok(WorkPacket::HumanGate(GatePacket {
                kind: "human-gate",
                run_id,
                activation_id,
                node_id,
                artifacts,
                missing_inputs,
                ledger_path,
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
                ledger_path,
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
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GateInspection {
    pub kind: &'static str,
    pub node_id: String,
    pub inputs: Vec<String>,
    pub decisions: Vec<String>,
    pub rejection_reason_required: bool,
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
                .map(
                    |(index, worker)| -> Result<WorkerInspection, ContextError> {
                        Ok(WorkerInspection {
                            key: format!("worker-{}", index + 1),
                            context: resolve_task_context(
                                project,
                                node_id,
                                &format!("nodes.{node_id}.tasks[{index}]"),
                                &worker.task,
                                &worker.agent,
                                Execution::Subagent,
                            )?,
                            inputs: parallel.inputs.clone(),
                            outputs: vec!["result.md".into()],
                        })
                    },
                )
                .collect::<Result<_, _>>()?;
            Ok(NodeInspection::Parallel(ParallelInspection {
                kind: "parallel",
                node_id: node_id.into(),
                inputs: parallel.inputs.clone(),
                outputs: parallel.outputs.clone(),
                workers,
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
            }))
        }
        NodeDefinition::Workflow(_) => Err(diagnostic(
            project,
            format!("nodes.{node_id}"),
            "workflow steps must be expanded before inspection",
        )),
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
