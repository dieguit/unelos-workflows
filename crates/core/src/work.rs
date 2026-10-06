use serde::Serialize;

use crate::{
    context::resolve_task_context, ContextError, Execution, LoadedProject, NodeDefinition,
    TaskContext, TaskInspection,
};

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
