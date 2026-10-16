use super::{artifacts, completion, gates, packet_for_state, store, CurrentWork, RunState};
use crate::{
    parse_parallel_results, parse_worker_result, read_brief, render_execution_package,
    ContinuationControl, ExecutionControl, ExecutionError, ExecutionIdentity, ExecutionOperation,
    ExecutionPackage, LoadedProject, RuntimeError, WorkPacket, WorkerStatus,
};
use camino::Utf8Path;

fn project_for(root: &Utf8Path, state: &RunState) -> Result<LoadedProject, ExecutionError> {
    let project = crate::load_project(root, &state.workflow_name).map_err(RuntimeError::from)?;
    crate::validate_project(&project).map_err(RuntimeError::from)?;
    Ok(project)
}

fn package_from_state(
    project: &LoadedProject,
    state: &RunState,
    continuation: Option<&ContinuationControl>,
) -> Result<ExecutionPackage, ExecutionError> {
    let packet = packet_for_state(project, state)?;
    let brief = if packet.requires_brief() {
        let path = project
            .project_root
            .join(".unelos/runs")
            .join(&state.name)
            .join("ledger.md");
        let text = std::fs::read_to_string(&path).map_err(|_| {
            ExecutionError::protocol(
                "missing_brief",
                "Record one agreed Brief in the supplied ledger before delegation",
            )
        })?;
        Some(read_brief(&text)?)
    } else {
        None
    };
    render_execution_package(Some(packet), vec![], brief.as_deref(), continuation)
}

fn assert_identity(state: &RunState, expected: &ExecutionIdentity) -> Result<(), ExecutionError> {
    let valid = match (&state.current, expected) {
        (
            CurrentWork::Task(record),
            ExecutionIdentity::Task {
                run_id,
                node_id,
                task_id,
            },
        ) => {
            run_id == &state.run_id && node_id == &record.node_id && task_id == &record.execution_id
        }
        (
            CurrentWork::HumanGate(record),
            ExecutionIdentity::HumanGate {
                run_id,
                node_id,
                activation_id,
            },
        ) => {
            run_id == &state.run_id
                && node_id == &record.node_id
                && activation_id == &record.execution_id
        }
        (
            CurrentWork::Parallel(record),
            ExecutionIdentity::Parallel {
                run_id,
                node_id,
                task_id,
            },
        ) => {
            run_id == &state.run_id && node_id == &record.node_id && task_id == &record.execution_id
        }
        _ => false,
    };
    if !valid {
        return Err(ExecutionError::protocol(
            "stale_execution",
            "Expected execution is no longer current; read state, never blindly retry",
        ));
    }
    Ok(())
}

pub fn execution_next(root: &Utf8Path, run_id: &str) -> Result<ExecutionPackage, ExecutionError> {
    let store = store::RunStore::open(root)?;
    let state = store.run(run_id)?;
    package_from_state(&project_for(root, &state)?, &state, None)
}

pub fn execution_start(
    root: &Utf8Path,
    workflow: &str,
    name: Option<&str>,
) -> Result<ExecutionPackage, ExecutionError> {
    if let Some(name) = name {
        if !super::valid_run_name(name) {
            return Err(RuntimeError::InvalidName { name: name.into() }.into());
        }
    }
    let project = crate::load_project(root, workflow).map_err(RuntimeError::from)?;
    crate::validate_project(&project).map_err(RuntimeError::from)?;
    let store = store::RunStore::open(root)?;
    store.ensure_start_available(name)?;
    let run_id = store.allocate_run_id()?;
    let state = super::prepare_start_state(
        &project,
        &run_id,
        name.unwrap_or(&run_id),
        "",
        name.is_none(),
    )?;
    let package = package_from_state(&project, &state, None)?;
    store
        .publish(&state, |run| {
            artifacts::prepare_work_directories(&state, run)
        })
        .map_err(|error| ExecutionError::after_commit(error.into()))?;
    Ok(package)
}

pub fn execution_continue(
    root: &Utf8Path,
    run_id: &str,
    control: &ContinuationControl,
) -> Result<ExecutionPackage, ExecutionError> {
    let store = store::RunStore::open(root)?;
    let state = store.run(run_id)?;
    assert_identity(&state, &control.expected)?;
    package_from_state(&project_for(root, &state)?, &state, Some(control))
}

pub fn execution_mutate(
    root: &Utf8Path,
    run_id: &str,
    operation: ExecutionOperation,
    control: &ExecutionControl,
) -> Result<ExecutionPackage, ExecutionError> {
    let store = store::RunStore::open(root)?;
    let state = store.run(run_id)?;
    assert_identity(&state, &control.expected)?;
    let project = project_for(root, &state)?;
    let candidate = match operation {
        ExecutionOperation::Complete => {
            let (CurrentWork::Task(record) | CurrentWork::Parallel(record)) = &state.current else {
                return Err(ExecutionError::protocol(
                    "invalid_control",
                    "Completion requires a Task",
                ));
            };
            if control.reason.is_some() {
                return Err(ExecutionError::protocol(
                    "invalid_control",
                    "Task completion cannot carry gate feedback",
                ));
            }
            let text = control.result_text.as_deref().ok_or_else(|| {
                ExecutionError::protocol(
                    "invalid_worker_result",
                    "Completion requires an inspected result",
                )
            })?;
            match packet_for_state(&project, &state)? {
                WorkPacket::Task(task) => {
                    let result = parse_worker_result(text, &control.expected)?;
                    if result.status != WorkerStatus::Finished {
                        return Err(ExecutionError::protocol(
                            "invalid_worker_result",
                            "Only FINISHED work can complete",
                        ));
                    }
                    let outcomes = &task.packet.context.outcomes;
                    if result.outcome != control.outcome
                        || match &control.outcome {
                            Some(outcome) => !outcomes.contains(outcome),
                            None => !outcomes.is_empty(),
                        }
                    {
                        return Err(ExecutionError::protocol(
                            "invalid_outcome",
                            "Select exactly one declared outcome, or none for ordinary work",
                        ));
                    }
                }
                WorkPacket::Parallel(parallel) => {
                    if control.outcome.is_some() {
                        return Err(ExecutionError::protocol(
                            "invalid_outcome",
                            "Parallel parents have no outcomes",
                        ));
                    }
                    let keys: Vec<String> =
                        parallel.workers.iter().map(|w| w.key.clone()).collect();
                    parse_parallel_results(text, &control.expected, &keys)?;
                }
                _ => unreachable!(),
            }
            for output in record
                .outputs
                .iter()
                .chain(record.worker_reports.iter().map(|r| &r.report))
            {
                artifacts::require_regular_file(&output.path).map_err(|cause| {
                    ExecutionError::protocol(
                        "missing_output",
                        format!("Required output unavailable: {} ({cause})", output.path),
                    )
                })?;
            }
            let mut confirmed = state.clone();
            if let CurrentWork::Task(record) | CurrentWork::Parallel(record) =
                &mut confirmed.current
            {
                record.confirmed_execution_id = Some(record.execution_id.clone());
            }
            completion::prepare_completion(
                &project,
                &confirmed,
                &record.execution_id,
                control.outcome.as_deref(),
                None,
            )?
        }
        ExecutionOperation::Approve | ExecutionOperation::Reject => {
            let CurrentWork::HumanGate(record) = &state.current else {
                return Err(ExecutionError::protocol(
                    "invalid_control",
                    "Decision requires a human gate",
                ));
            };
            if control.result_text.is_some() || control.outcome.is_some() {
                return Err(ExecutionError::protocol(
                    "invalid_control",
                    "Gate decisions cannot carry Task results/outcomes",
                ));
            }
            let (outcome, reason) = match operation {
                ExecutionOperation::Approve if control.reason.is_none() => ("approved", None),
                ExecutionOperation::Reject => (
                    "rejected",
                    Some(
                        control
                            .reason
                            .as_deref()
                            .filter(|s| !s.trim().is_empty())
                            .ok_or_else(|| {
                                ExecutionError::protocol(
                                    "invalid_control",
                                    "Rejection requires actionable feedback",
                                )
                            })?
                            .trim(),
                    ),
                ),
                _ => {
                    return Err(ExecutionError::protocol(
                        "invalid_control",
                        "Approval cannot carry a rejection reason",
                    ))
                }
            };
            gates::prepare_decision(&project, &state, &record.execution_id, outcome, reason)?
        }
    };
    // Resolve before publication: missing Brief/context must not partially advance state.
    let package = package_from_state(&project, &candidate, None)?;
    artifacts::prepare_work_directories(
        &candidate,
        &project
            .project_root
            .join(".unelos/runs")
            .join(&candidate.name),
    )?;
    store
        .update(&candidate)
        .map_err(|error| ExecutionError::after_commit(error.into()))?;
    Ok(package)
}
