use super::{artifacts, packet_for_state, store, verify_work_outputs};
use crate::{
    CompletedExecution, CurrentWork, LoadedProject, NodeDefinition, RunState, RuntimeError,
    Transition, TransitionFeedback,
};
use camino::Utf8Path;

pub fn complete(
    project_root: &Utf8Path,
    run_id: &str,
    expected_execution_id: &str,
    outcome: Option<&str>,
    reason: Option<&str>,
) -> Result<RunState, RuntimeError> {
    let store = store::RunStore::open(project_root)?;
    let state = store.run(run_id)?;
    if state.run_id != run_id {
        return Err(RuntimeError::StaleRun {
            expected: run_id.into(),
            current: state.run_id,
        });
    }
    let project = super::load_run_project(project_root, &state)?;
    crate::validate_project(&project)?;
    let candidate = prepare_completion(&project, &state, expected_execution_id, outcome, reason)?;
    artifacts::prepare_work_directories(
        &candidate,
        &project
            .project_root
            .join(".unelos/runs")
            .join(&state.run_id),
    )?;
    store.update(&candidate)?;
    store.run(&candidate.run_id)
}

pub(super) fn prepare_completion(
    project: &LoadedProject,
    state: &RunState,
    expected_execution_id: &str,
    outcome: Option<&str>,
    reason: Option<&str>,
) -> Result<RunState, RuntimeError> {
    state.expect_execution(expected_execution_id)?;
    let record = match &state.current {
        CurrentWork::Task(record) | CurrentWork::Parallel(record) => record,
        _ => {
            return Err(RuntimeError::InvalidState(
                "completion requires current task work".into(),
            ))
        }
    };
    packet_for_state(project, state)?;
    verify_work_outputs(&project.project_root, state, expected_execution_id)?;
    let node = project
        .workflow
        .nodes
        .get(&record.node_id)
        .ok_or_else(|| super::mismatch(&record.node_id, "current Node was removed"))?;
    let transition = match (&state.current, node) {
        (CurrentWork::Task(_), NodeDefinition::Task(task)) => &task.next,
        (CurrentWork::Parallel(_), NodeDefinition::Parallel(parallel)) => &parallel.next,
        _ => {
            return Err(super::mismatch(
                &record.node_id,
                "current Node kind changed",
            ))
        }
    };
    let target = match transition {
        Transition::Single(target) if outcome.is_none() => target,
        Transition::Single(_) => {
            return Err(RuntimeError::InvalidState(
                "single-successor work does not accept an outcome".into(),
            ))
        }
        Transition::Outcomes(routes) => {
            outcome.and_then(|label| routes.get(label)).ok_or_else(|| {
                RuntimeError::InvalidState(format!(
                    "Node {:?} requires one declared outcome from {:?}; received {:?}",
                    record.node_id,
                    routes.keys().collect::<Vec<_>>(),
                    outcome,
                ))
            })?
        }
    };
    let feedback = TransitionFeedback {
        source_node_id: record.node_id.clone(),
        source_execution_id: record.execution_id.clone(),
        outcome: outcome.map(str::to_owned),
        reason: reason.map(str::to_owned),
        artifacts: record.outputs.clone(),
    };
    let mut candidate = state.clone();
    candidate.history.push(CompletedExecution {
        work: state.current.clone(),
        outcome: outcome.map(str::to_owned),
        reason: reason.map(str::to_owned),
    });
    let candidate = artifacts::prepare_work(project, &candidate, target, Some(feedback))?;
    crate::build_work_packet(project, artifacts::runtime_input(&candidate, project)?)?;
    Ok(candidate)
}
