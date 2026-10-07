use super::{
    artifacts, completion, gates, store, CurrentWork, LoadedProject, RunState, RuntimeError,
};
use camino::Utf8Path;

pub fn complete_node(
    root: &Utf8Path,
    node: &str,
    outcome: Option<&str>,
) -> Result<RunState, RuntimeError> {
    let (store, project, state) = open_current(root, node)?;
    let mut confirmed = state.clone();
    let execution_id = match &mut confirmed.current {
        CurrentWork::Task(record) | CurrentWork::Parallel(record) => {
            record.confirmed_execution_id = Some(record.execution_id.clone());
            record.execution_id.clone()
        }
        _ => {
            return Err(RuntimeError::InvalidState(
                "completion requires current task work".into(),
            ))
        }
    };
    let candidate =
        completion::prepare_completion(&project, &confirmed, &execution_id, outcome, None)?;
    commit(&store, &project, &state, candidate)
}

pub fn approve_node(root: &Utf8Path, node: &str) -> Result<RunState, RuntimeError> {
    decide_node(root, node, "approved", None)
}

pub fn reject_node(root: &Utf8Path, node: &str, reason: &str) -> Result<RunState, RuntimeError> {
    let reason = reason.trim();
    if reason.is_empty() {
        return Err(RuntimeError::InvalidState(
            "rejection reason must not be empty".into(),
        ));
    }
    decide_node(root, node, "rejected", Some(reason))
}

fn decide_node(
    root: &Utf8Path,
    node: &str,
    outcome: &str,
    reason: Option<&str>,
) -> Result<RunState, RuntimeError> {
    let (store, project, state) = open_current(root, node)?;
    let activation = match &state.current {
        CurrentWork::HumanGate(record) => record.execution_id.clone(),
        _ => {
            return Err(RuntimeError::InvalidState(
                "decision requires current human-gate work".into(),
            ))
        }
    };
    let candidate = gates::prepare_decision(&project, &state, &activation, outcome, reason)?;
    commit(&store, &project, &state, candidate)
}

fn open_current(
    root: &Utf8Path,
    node: &str,
) -> Result<(store::RunStore, LoadedProject, RunState), RuntimeError> {
    let store = store::RunStore::open(root)?;
    let state = store.active()?;
    let current_node = match &state.current {
        CurrentWork::Task(record)
        | CurrentWork::Parallel(record)
        | CurrentWork::HumanGate(record) => &record.node_id,
        CurrentWork::Finished { node_id } => node_id,
    };
    if current_node != node {
        return Err(RuntimeError::WrongNode {
            expected: node.into(),
            current: current_node.clone(),
        });
    }
    let project = crate::load_project(root, &state.workflow_name)?;
    crate::validate_project(&project)?;
    Ok((store, project, state))
}

fn commit(
    store: &store::RunStore,
    project: &LoadedProject,
    state: &RunState,
    candidate: RunState,
) -> Result<RunState, RuntimeError> {
    artifacts::prepare_work_directories(
        &candidate,
        &project.project_root.join(".unelos/runs").join(&state.name),
    )?;
    store.update(&candidate)?;
    Ok(candidate)
}
