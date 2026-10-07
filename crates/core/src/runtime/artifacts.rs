use super::{output_directories, store, CurrentWork, ExecutionRecord, RunState, RuntimeError};
use crate::{
    ArtifactReference, LoadedProject, NodeDefinition, TaskRuntimeInput, TransitionFeedback,
    WorkRuntimeInput, WorkerReportBinding,
};
use camino::Utf8Path;
use std::{collections::BTreeMap, fs};

pub(super) fn prepare_work(
    project: &LoadedProject,
    state: &RunState,
    node_id: &str,
    feedback: Option<TransitionFeedback>,
) -> Result<RunState, RuntimeError> {
    let producers = crate::validate_project(project)?;
    let node = project.workflow.nodes.get(node_id).ok_or_else(|| {
        RuntimeError::InvalidState(format!("Workflow Node {node_id:?} is missing"))
    })?;
    super::check_component(node_id, "Node")?;
    let run_dir = project.project_root.join(".unelos/runs").join(&state.name);
    store::validate_artifacts(state, &run_dir)?;
    let mut candidate = state.clone();
    candidate.current = match node {
        NodeDefinition::Task(task) => {
            let inputs = bind_inputs(state, &producers, node_id, &task.inputs)?;
            let outputs = super::allocate_outputs(&run_dir.join(node_id), &task.outputs)?;
            let execution_id = candidate.allocate_execution_id(false)?;
            CurrentWork::Task(ExecutionRecord {
                node_id: node_id.into(),
                execution_id,
                inputs,
                outputs,
                worker_reports: vec![],
                feedback,
                confirmed_execution_id: None,
            })
        }
        NodeDefinition::Parallel(parallel) => {
            let inputs = bind_inputs(state, &producers, node_id, &parallel.inputs)?;
            let node_dir = run_dir.join(node_id);
            let outputs = super::allocate_outputs(&node_dir, &parallel.outputs)?;
            let mut paths = outputs.clone();
            let mut counts = BTreeMap::new();
            for worker in &parallel.tasks {
                *counts.entry(worker.agent.as_str()).or_insert(0usize) += 1;
            }
            let mut reports = Vec::new();
            for (index, worker) in parallel.tasks.iter().enumerate() {
                super::check_component(&worker.agent, "Agent")?;
                let dir = if counts[worker.agent.as_str()] > 1 {
                    format!("{}-worker-{}", worker.agent, index + 1)
                } else {
                    worker.agent.clone()
                };
                super::check_component(&dir, "worker directory")?;
                let report = ArtifactReference {
                    name: "result.md".into(),
                    path: node_dir.join(dir).join("result.md"),
                };
                reports.push(WorkerReportBinding {
                    key: format!("worker-{}", index + 1),
                    report: report.clone(),
                });
                paths.push(report);
            }
            super::validate_paths(&paths)?;
            let execution_id = candidate.allocate_execution_id(false)?;
            CurrentWork::Parallel(ExecutionRecord {
                node_id: node_id.into(),
                execution_id,
                inputs,
                outputs,
                worker_reports: reports,
                feedback,
                confirmed_execution_id: None,
            })
        }
        NodeDefinition::HumanGate(gate) => {
            let inputs = bind_inputs(state, &producers, node_id, &gate.inputs)?;
            let execution_id = candidate.allocate_execution_id(true)?;
            CurrentWork::HumanGate(ExecutionRecord {
                node_id: node_id.into(),
                execution_id,
                inputs,
                outputs: vec![],
                worker_reports: vec![],
                feedback,
                confirmed_execution_id: None,
            })
        }
        NodeDefinition::End => CurrentWork::Finished {
            node_id: node_id.into(),
        },
    };
    Ok(candidate)
}

fn bind_inputs(
    state: &RunState,
    producers: &BTreeMap<String, String>,
    node: &str,
    names: &[String],
) -> Result<Vec<ArtifactReference>, RuntimeError> {
    names
        .iter()
        .map(|name| {
            let producer = producers.get(name).ok_or_else(|| {
                invalid_binding(node, name, "producer", "no output producer was found")
            })?;
            let execution = state
                .history
                .iter()
                .rev()
                .find_map(|entry| match &entry.work {
                    CurrentWork::Task(record) | CurrentWork::Parallel(record)
                        if &record.node_id == producer =>
                    {
                        Some(record)
                    }
                    _ => None,
                })
                .ok_or_else(|| {
                    invalid_binding(
                        node,
                        name,
                        producer,
                        "no completed producer execution is recorded",
                    )
                })?;
            let binding = execution
                .outputs
                .iter()
                .find(|output| &output.name == name)
                .ok_or_else(|| {
                    invalid_binding(
                        node,
                        name,
                        producer,
                        "latest completed execution has no recorded output",
                    )
                })?;
            require_regular_file(&binding.path).map_err(|cause| {
                invalid_binding(
                    node,
                    name,
                    producer,
                    &format!("recorded path {}: {cause}", binding.path),
                )
            })?;
            Ok(binding.clone())
        })
        .collect()
}

fn invalid_binding(node: &str, artifact: &str, producer: &str, cause: &str) -> RuntimeError {
    RuntimeError::InvalidState(format!(
        "cannot bind artifact {artifact:?} for Node {node:?} from producer {producer:?}: {cause}"
    ))
}

pub(super) fn require_regular_file(path: &Utf8Path) -> Result<(), String> {
    let metadata = fs::metadata(path).map_err(|error| error.to_string())?;
    if metadata.is_file() {
        Ok(())
    } else {
        Err("not a regular file".into())
    }
}

pub(super) fn prepare_work_directories(
    state: &RunState,
    run_dir: &Utf8Path,
) -> Result<(), RuntimeError> {
    let (node, outputs) = match &state.current {
        CurrentWork::Task(record) | CurrentWork::Parallel(record) => (
            record.node_id.as_str(),
            record
                .outputs
                .iter()
                .chain(record.worker_reports.iter().map(|r| &r.report))
                .cloned()
                .collect::<Vec<_>>(),
        ),
        CurrentWork::HumanGate(_) | CurrentWork::Finished { .. } => return Ok(()),
    };
    let node_dir = run_dir.join(node);
    for dir in output_directories(&node_dir, &outputs) {
        let relative = dir.strip_prefix(run_dir).map_err(|_| {
            RuntimeError::InvalidState(format!("artifact directory escapes run: {dir}"))
        })?;
        let mut path = run_dir.to_owned();
        for component in relative.components() {
            path.push(component.as_str());
            store::ensure_runtime_directory(&path)?;
        }
    }
    Ok(())
}

pub(super) fn runtime_input(
    state: &RunState,
    project: &LoadedProject,
) -> Result<WorkRuntimeInput, RuntimeError> {
    let ledger_path = project
        .project_root
        .join(".unelos/runs")
        .join(&state.name)
        .join("ledger.md");
    match &state.current {
        CurrentWork::Task(record) => Ok(WorkRuntimeInput::Task {
            input: task_input(state, record),
            ledger_path,
        }),
        CurrentWork::Parallel(record) => Ok(WorkRuntimeInput::Parallel {
            input: task_input(state, record),
            worker_reports: record.worker_reports.clone(),
            ledger_path,
        }),
        CurrentWork::HumanGate(record) => Ok(WorkRuntimeInput::HumanGate {
            run_id: state.run_id.clone(),
            activation_id: record.execution_id.clone(),
            node_id: record.node_id.clone(),
            artifacts: record.inputs.clone(),
            ledger_path,
        }),
        CurrentWork::Finished { node_id } => Ok(WorkRuntimeInput::Finished {
            run_id: state.run_id.clone(),
            node_id: node_id.clone(),
            ledger_path,
        }),
    }
}

fn task_input(state: &RunState, record: &ExecutionRecord) -> TaskRuntimeInput {
    TaskRuntimeInput {
        run_id: state.run_id.clone(),
        task_id: record.execution_id.clone(),
        node_id: record.node_id.clone(),
        request: state.request.clone(),
        inputs: record.inputs.clone(),
        outputs: record.outputs.clone(),
        feedback: record.feedback.clone(),
    }
}

pub(super) fn ensure_inputs_are_files(record: &ExecutionRecord) -> Result<(), RuntimeError> {
    for input in &record.inputs {
        require_regular_file(&input.path).map_err(|cause| {
            RuntimeError::InvalidState(format!(
                "saved input artifact {:?} at {} for Node {:?} is unavailable: {cause}",
                input.name, input.path, record.node_id
            ))
        })?;
    }
    Ok(())
}
