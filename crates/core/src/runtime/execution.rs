use super::{artifacts, completion, gates, packet_for_state, store, CurrentWork, RunState};
use crate::{
    parse_parallel_results, parse_worker_result, read_brief, render_execution_package,
    CompletionControl, ExecutionError, ExecutionIdentity, ExecutionOperation, ExecutionPackage,
    LoadedProject, RuntimeError, WorkPacket, WorkerStatus,
};
use camino::Utf8Path;

fn project_for(root: &Utf8Path, state: &RunState) -> Result<LoadedProject, ExecutionError> {
    let project = super::load_run_project(root, state)?;
    crate::validate_project(&project).map_err(RuntimeError::from)?;
    Ok(project)
}

pub(super) fn package_from_state(
    project: &LoadedProject,
    state: &RunState,
) -> Result<ExecutionPackage, ExecutionError> {
    let packet = packet_for_state(project, state)?;
    let path = project
        .project_root
        .join(".unelos/runs")
        .join(&state.run_id)
        .join("ledger.md");
    let brief = match std::fs::read_to_string(&path) {
        Ok(text) => read_brief(&text),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => return Err(super::path_error(&path, error).into()),
    };
    let mut package = render_execution_package(Some(packet), vec![], brief.as_deref())?;
    filter_receipts(&mut package, state);
    Ok(package)
}

pub(super) fn filter_receipts(package: &mut ExecutionPackage, state: &RunState) {
    if let CurrentWork::Parallel(record) = &state.current {
        package
            .worker_prompts
            .retain(|prompt| !record.worker_receipts.contains_key(&prompt.key));
    }
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
    package_from_state(&project_for(root, &state)?, &state)
}

pub fn execution_start(
    root: &Utf8Path,
    workflow: &str,
    name: Option<&str>,
    request: &str,
) -> Result<ExecutionPackage, ExecutionError> {
    let project = crate::load_project(root, workflow).map_err(RuntimeError::from)?;
    start_project(root, name, request, |_| Ok((project, None)))
}

pub(super) fn start_project(
    root: &Utf8Path,
    name: Option<&str>,
    request: &str,
    load: impl FnOnce(&str) -> Result<(LoadedProject, Option<(String, String)>), ExecutionError>,
) -> Result<ExecutionPackage, ExecutionError> {
    let name = name.map(super::slugify_run_name).transpose()?;
    let store = store::RunStore::open(root)?;
    store.ensure_start_available(name.as_deref())?;
    let run_id = store.allocate_run_id()?;
    let (project, inline) = load(&run_id)?;
    crate::validate_project(&project).map_err(RuntimeError::from)?;
    let mut state = super::prepare_start_state(
        &project,
        &run_id,
        name.as_deref().unwrap_or(&run_id),
        request,
        name.is_none(),
    )?;
    if let Some((label, _)) = &inline {
        state.inline = true;
        state.workflow_name.clone_from(label);
    }
    let package = package_from_state(&project, &state)?;
    store
        .publish(&state, |run| {
            if let Some((_, text)) = inline {
                let path = run.join("workflow.yaml");
                store::atomic_write(&path, text.as_bytes())?;
            }
            artifacts::prepare_work_directories(&state, run)
        })
        .map_err(|error| ExecutionError::after_commit(error.into()))?;
    Ok(package)
}

fn execution_mutate_inner(
    root: &Utf8Path,
    run_id: &str,
    operation: ExecutionOperation,
    control: &CompletionControl,
) -> Result<ExecutionPackage, ExecutionError> {
    let store = store::RunStore::open(root)?;
    let mut state = store.run(run_id)?;
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
            let text = || {
                control.result_text.as_deref().ok_or_else(|| {
                    ExecutionError::protocol(
                        "invalid_worker_result",
                        "Completion requires the worker's final message, verbatim",
                    )
                })
            };
            let outcome = match packet_for_state(&project, &state)? {
                WorkPacket::Task(task) => {
                    let outcome = if task.packet.context.execution == crate::Execution::Subagent {
                        let result = parse_worker_result(text()?, &control.expected)?;
                        if result.status != WorkerStatus::Finished {
                            return Err(ExecutionError::protocol(
                                "invalid_worker_result",
                                "Only FINISHED work can complete",
                            ));
                        }
                        result.outcome
                    } else {
                        if control.result_text.is_some() {
                            return Err(ExecutionError::protocol(
                                "invalid_control",
                                "Main Tasks use summary, not resultText",
                            ));
                        }
                        control.outcome.clone()
                    };
                    let outcomes = &task.packet.context.outcomes;
                    if match &outcome {
                        Some(outcome) => !outcomes.contains(outcome),
                        None => !outcomes.is_empty(),
                    } {
                        return Err(ExecutionError::protocol(
                            "invalid_outcome",
                            format!(
                                "outcome {:?} is not declared; allowed: {}",
                                outcome,
                                if outcomes.is_empty() {
                                    "null".into()
                                } else {
                                    outcomes.join(", ")
                                }
                            ),
                        ));
                    }
                    outcome
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
                    let results = if control
                        .result_text
                        .as_deref()
                        .unwrap_or("")
                        .trim()
                        .is_empty()
                        && record.worker_receipts.len() == keys.len()
                    {
                        vec![]
                    } else {
                        parse_parallel_results(text()?, &control.expected, &keys)?
                    };
                    for result in &results {
                        let key = result.worker_key.as_ref().unwrap();
                        if record.worker_receipts.contains_key(key) {
                            return Err(ExecutionError::protocol(
                                "invalid_worker_result",
                                format!("workerKey {key:?} already has a receipt"),
                            ));
                        }
                        let report = record
                            .worker_reports
                            .iter()
                            .find(|r| &r.key == key)
                            .unwrap();
                        artifacts::require_nonempty_file(&report.report.path).map_err(|cause| {
                            ExecutionError::protocol(
                                "missing_output",
                                format!("{}: {cause}", report.report.path),
                            )
                        })?;
                    }
                    let CurrentWork::Parallel(record) = &mut state.current else {
                        unreachable!()
                    };
                    for result in results {
                        record
                            .worker_receipts
                            .insert(result.worker_key.clone().unwrap(), result);
                    }
                    record.invalid_worker_results = 0;
                    // Save receipts even when the aggregates still need writing.
                    store.update(&state)?;
                    let CurrentWork::Parallel(record) = &state.current else {
                        unreachable!()
                    };
                    if record.worker_receipts.len() != keys.len() {
                        return package_from_state(&project, &state);
                    }
                    None
                }
                _ => unreachable!(),
            };
            let record = match &state.current {
                CurrentWork::Task(r) | CurrentWork::Parallel(r) => r,
                _ => unreachable!(),
            };
            for output in record
                .outputs
                .iter()
                .chain(record.worker_reports.iter().map(|r| &r.report))
            {
                artifacts::require_nonempty_file(&output.path).map_err(|cause| {
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
                outcome.as_deref(),
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
            if control.result_text.is_some()
                || control.outcome.is_some()
                || control.summary.is_some()
            {
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
    let package = package_from_state(&project, &candidate)?;
    artifacts::prepare_work_directories(
        &candidate,
        &project
            .project_root
            .join(".unelos/runs")
            .join(&candidate.run_id),
    )?;
    store
        .update(&candidate)
        .map_err(|error| ExecutionError::after_commit(error.into()))?;
    Ok(package)
}

pub fn execution_mutate(
    root: &Utf8Path,
    run_id: &str,
    operation: ExecutionOperation,
    control: &CompletionControl,
) -> Result<ExecutionPackage, ExecutionError> {
    let result = execution_mutate_inner(root, run_id, operation, control);
    if let Err(ExecutionError::Protocol {
        code: "invalid_worker_result",
        message,
    }) = &result
    {
        let store = store::RunStore::open(root)?;
        let mut state = store.run(run_id)?;
        assert_identity(&state, &control.expected)?;
        if let CurrentWork::Task(record) | CurrentWork::Parallel(record) = &mut state.current {
            record.invalid_worker_results = record.invalid_worker_results.saturating_add(1);
            let stop = record.invalid_worker_results >= 3;
            store.update(&state)?;
            if stop {
                return Err(ExecutionError::protocol(
                    "invalid_worker_result",
                    format!("{message}. Stop and tell the user."),
                ));
            }
        }
    }
    if result
        .as_ref()
        .err()
        .is_some_and(|error| error.code() != "invalid_worker_result")
    {
        let store = store::RunStore::open(root)?;
        let mut state = store.run(run_id)?;
        if assert_identity(&state, &control.expected).is_ok() {
            if let CurrentWork::Task(record) | CurrentWork::Parallel(record) = &mut state.current {
                if record.invalid_worker_results != 0 {
                    record.invalid_worker_results = 0;
                    store.update(&state)?;
                }
            }
        }
    }
    result
}
