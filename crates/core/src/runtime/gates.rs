use super::{artifacts, packet_for_state, store, CompletedExecution, CurrentWork, RunState};
use crate::{LoadedProject, NodeDefinition, RuntimeError, TransitionFeedback};
use camino::Utf8Path;

pub fn approve(
    project_root: &Utf8Path,
    run_id: &str,
    expected_gate_activation: &str,
) -> Result<RunState, RuntimeError> {
    decide(
        project_root,
        run_id,
        expected_gate_activation,
        "approved",
        None,
    )
}

pub fn reject(
    project_root: &Utf8Path,
    run_id: &str,
    expected_gate_activation: &str,
    reason: &str,
) -> Result<RunState, RuntimeError> {
    let reason = reason.trim();
    if reason.is_empty() {
        return Err(RuntimeError::InvalidState(
            "rejection reason must not be empty".into(),
        ));
    }
    decide(
        project_root,
        run_id,
        expected_gate_activation,
        "rejected",
        Some(reason),
    )
}

fn decide(
    project_root: &Utf8Path,
    run_id: &str,
    expected_gate_activation: &str,
    outcome: &str,
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
    let project = crate::load_project(project_root, &state.workflow_name)?;
    crate::validate_project(&project)?;
    let candidate = prepare_decision(&project, &state, expected_gate_activation, outcome, reason)?;
    artifacts::prepare_work_directories(
        &candidate,
        &project.project_root.join(".unelos/runs").join(&state.run_id),
    )?;
    store.update(&candidate)?;
    store.run(&candidate.run_id)
}

pub(super) fn prepare_decision(
    project: &LoadedProject,
    state: &RunState,
    expected_gate_activation: &str,
    outcome: &str,
    reason: Option<&str>,
) -> Result<RunState, RuntimeError> {
    state.expect_execution(expected_gate_activation)?;
    let record = match &state.current {
        CurrentWork::HumanGate(record) => record,
        _ => {
            return Err(RuntimeError::InvalidState(
                "decision requires current human-gate work".into(),
            ))
        }
    };
    packet_for_state(project, state)?;
    let gate = match project.workflow.nodes.get(&record.node_id) {
        Some(NodeDefinition::HumanGate(gate)) => gate,
        _ => {
            return Err(super::mismatch(
                &record.node_id,
                "current Node kind changed",
            ))
        }
    };
    let target = gate.next.get(outcome).ok_or_else(|| {
        RuntimeError::InvalidState(format!(
            "gate {:?} has no declared {outcome:?} successor",
            record.node_id
        ))
    })?;
    let feedback = TransitionFeedback {
        source_node_id: record.node_id.clone(),
        source_execution_id: record.execution_id.clone(),
        outcome: Some(outcome.into()),
        reason: reason.map(str::to_owned),
        artifacts: record.inputs.clone(),
    };
    let mut candidate = state.clone();
    candidate.history.push(CompletedExecution {
        work: state.current.clone(),
        outcome: Some(outcome.into()),
        reason: reason.map(str::to_owned),
    });
    let candidate = artifacts::prepare_work(project, &candidate, target, Some(feedback))?;
    crate::build_work_packet(project, artifacts::runtime_input(&candidate, project)?)?;
    Ok(candidate)
}
