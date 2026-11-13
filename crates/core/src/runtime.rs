use camino::{Utf8Path, Utf8PathBuf};
use serde::{Deserialize, Serialize};

use crate::{
    ArtifactReference, ContextError, LoadedProject, NodeDefinition, ProjectLoadError,
    ProjectValidationError, TaskRuntimeInput, TransitionFeedback, WorkPacket, WorkRuntimeInput,
    WorkerReportBinding,
};

mod artifacts;
mod commands;
mod completion;
mod execution;
mod gates;
mod rename;
mod resume;
mod runs_ui;
mod store;
pub use runs_ui::{delete_run, get_run, list_runs, read_run_artifact, RunDetails, RunListItem};

pub use commands::{approve_node, complete_node, reject_node};
pub use completion::complete;
pub use execution::{execution_mutate, execution_next, execution_start};
pub use gates::{approve, reject};
pub use rename::rename_run;
pub use resume::{execution_new, execution_resume, execution_runs};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunState {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub started_at: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<u64>,
    pub run_id: String,
    pub name: String,
    pub workflow_name: String,
    pub request: String,
    #[serde(default)]
    pub naming_pending: bool,
    pub current: CurrentWork,
    pub history: Vec<CompletedExecution>,
    pub next_execution_sequence: u64,
}

impl RunState {
    pub fn is_finished(&self) -> bool {
        matches!(self.current, CurrentWork::Finished { .. })
    }

    pub(crate) fn allocate_execution_id(&mut self, gate: bool) -> Result<String, RuntimeError> {
        let sequence = self.next_execution_sequence;
        self.next_execution_sequence = sequence
            .checked_add(1)
            .ok_or_else(|| RuntimeError::InvalidState("execution sequence overflow".into()))?;
        Ok(format!("{}-{sequence}", if gate { "gate" } else { "task" }))
    }

    pub fn expect_execution(&self, expected: &str) -> Result<(), RuntimeError> {
        let current = match &self.current {
            CurrentWork::Task(record) | CurrentWork::Parallel(record) => &record.execution_id,
            CurrentWork::HumanGate(record) => &record.execution_id,
            CurrentWork::Finished { .. } => {
                return Err(RuntimeError::StaleExecution {
                    expected: expected.into(),
                    current: None,
                })
            }
        };
        if current != expected {
            return Err(RuntimeError::StaleExecution {
                expected: expected.into(),
                current: Some(current.clone()),
            });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase"
)]
pub enum CurrentWork {
    Task(ExecutionRecord),
    Parallel(ExecutionRecord),
    HumanGate(ExecutionRecord),
    Finished { node_id: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionRecord {
    pub node_id: String,
    pub execution_id: String,
    pub inputs: Vec<ArtifactReference>,
    pub outputs: Vec<ArtifactReference>,
    pub worker_reports: Vec<WorkerReportBinding>,
    #[serde(default)]
    pub output_digests: std::collections::BTreeMap<Utf8PathBuf, Option<String>>,
    #[serde(default)]
    pub worker_receipts: std::collections::BTreeMap<String, crate::WorkerResult>,
    #[serde(default)]
    pub invalid_worker_results: u32,
    pub feedback: Option<TransitionFeedback>,
    #[serde(default)]
    pub confirmed_execution_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompletedExecution {
    pub work: CurrentWork,
    pub outcome: Option<String>,
    pub reason: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum RuntimeError {
    #[error(transparent)]
    Project(#[from] ProjectLoadError),
    #[error(transparent)]
    Validation(#[from] ProjectValidationError),
    #[error(transparent)]
    Context(#[from] ContextError),
    #[error("filesystem error at {path}: {source}")]
    Io {
        path: Utf8PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("invalid JSON at {path}: {source}")]
    Json {
        path: Utf8PathBuf,
        #[source]
        source: serde_json::Error,
    },
    #[error("invalid run name {name:?}: expected lowercase ASCII letters/digits separated by hyphens, max 64 characters")]
    InvalidName { name: String },
    #[error("unknown run ID {run_id:?}; select an existing run explicitly")]
    UnknownRun { run_id: String },
    #[error("duplicate persisted run ID {run_id:?}")]
    DuplicateRunId { run_id: String },
    #[error("run {name:?} already exists")]
    Collision { name: String },
    #[error("invalid persisted run state: {0}")]
    InvalidState(String),
    #[error("stale execution {expected:?}; current execution is {current:?}")]
    StaleExecution {
        expected: String,
        current: Option<String>,
    },
    #[error("stale run {expected:?}; current run is {current:?}")]
    StaleRun { expected: String, current: String },
    #[error("wrong Node {expected:?}; current Node is {current:?}")]
    WrongNode { expected: String, current: String },
}

pub(crate) fn valid_run_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 64
        && name.split('-').all(|part| {
            !part.is_empty()
                && part
                    .bytes()
                    .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
        })
        && !matches!(
            name,
            "con"
                | "prn"
                | "aux"
                | "nul"
                | "com1"
                | "com2"
                | "com3"
                | "com4"
                | "com5"
                | "com6"
                | "com7"
                | "com8"
                | "com9"
                | "lpt1"
                | "lpt2"
                | "lpt3"
                | "lpt4"
                | "lpt5"
                | "lpt6"
                | "lpt7"
                | "lpt8"
                | "lpt9"
        )
}

pub(crate) fn slugify_run_name(input: &str) -> Result<String, RuntimeError> {
    let mut name = input
        .to_ascii_lowercase()
        .split(|c: char| !c.is_ascii_lowercase() && !c.is_ascii_digit())
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    name.truncate(64);
    name = name.trim_end_matches('-').into();
    if name.is_empty() {
        return Err(RuntimeError::InvalidName { name: input.into() });
    }
    if !valid_run_name(&name) {
        name.push_str("-run");
    }
    Ok(name)
}

pub(crate) fn path_error(path: &Utf8Path, source: std::io::Error) -> RuntimeError {
    RuntimeError::Io {
        path: path.to_owned(),
        source,
    }
}

pub fn start(project: &LoadedProject, name: &str, request: &str) -> Result<RunState, RuntimeError> {
    let name = slugify_run_name(name)?;
    crate::validate_project(project)?;
    let store = store::RunStore::open(&project.project_root)?;
    store.ensure_start_available(Some(&name))?;
    start_prepared(project, &name, &name, request, false, &store)
}

pub fn start_workflow(
    root: &Utf8Path,
    workflow: &str,
    name: Option<&str>,
) -> Result<RunState, RuntimeError> {
    let name = name.map(slugify_run_name).transpose()?;
    let project = crate::load_project(root, workflow)?;
    crate::validate_project(&project)?;
    let store = store::RunStore::open(&project.project_root)?;
    store.ensure_start_available(name.as_deref())?;
    let run_id = store.allocate_run_id()?;
    let run_name = name.as_deref().unwrap_or(&run_id);
    start_prepared(&project, &run_id, run_name, "", name.is_none(), &store)
}

fn start_prepared(
    project: &LoadedProject,
    run_id: &str,
    name: &str,
    request: &str,
    naming_pending: bool,
    store: &store::RunStore,
) -> Result<RunState, RuntimeError> {
    let state = prepare_start_state(project, run_id, name, request, naming_pending)?;
    store.publish(&state, |run| {
        artifacts::prepare_work_directories(&state, run)
    })?;
    store.run(run_id)
}

fn prepare_start_state(
    project: &LoadedProject,
    run_id: &str,
    name: &str,
    request: &str,
    naming_pending: bool,
) -> Result<RunState, RuntimeError> {
    let node_id = &project.workflow.start;
    check_component(node_id, "Node")?;
    let state = RunState {
        started_at: None,
        updated_at: None,
        run_id: run_id.into(),
        name: name.into(),
        workflow_name: project.workflow.name.clone(),
        request: request.into(),
        naming_pending,
        current: CurrentWork::Finished {
            node_id: node_id.clone(),
        },
        history: vec![],
        next_execution_sequence: 1,
    };
    let state = artifacts::prepare_work(project, &state, node_id, None)?;
    crate::build_work_packet(project, artifacts::runtime_input(&state, project)?)?;
    Ok(state)
}

pub fn prepare_work(
    project: &LoadedProject,
    state: &RunState,
    node_id: &str,
    feedback: Option<TransitionFeedback>,
) -> Result<RunState, RuntimeError> {
    artifacts::prepare_work(project, state, node_id, feedback)
}

pub(crate) fn stored_runs(project_root: &Utf8Path) -> Result<Vec<RunState>, RuntimeError> {
    store::RunStore::open(project_root)?.runs()
}

pub fn status(project_root: &Utf8Path, run_id: &str) -> Result<RunState, RuntimeError> {
    store::RunStore::open(project_root)?.run(run_id)
}

pub fn confirm_outputs(
    project_root: &Utf8Path,
    run_id: &str,
    expected_execution_id: &str,
) -> Result<RunState, RuntimeError> {
    let store = store::RunStore::open(project_root)?;
    let mut state = store.run(run_id)?;
    state.expect_execution(expected_execution_id)?;
    let record = match &mut state.current {
        CurrentWork::Task(record) | CurrentWork::Parallel(record) => record,
        _ => {
            return Err(RuntimeError::InvalidState(
                "output confirmation requires current task work".into(),
            ))
        }
    };
    for output in &record.outputs {
        artifacts::require_nonempty_file(&output.path).map_err(|cause| {
            RuntimeError::InvalidState(format!(
                "declared output {:?} at {} is unavailable: {cause}",
                output.name, output.path
            ))
        })?;
    }
    record.confirmed_execution_id = Some(expected_execution_id.into());
    store.update(&state)?;
    store.run(&state.run_id)
}

pub fn verify_outputs(
    project_root: &Utf8Path,
    run_id: &str,
    expected_execution_id: &str,
) -> Result<(), RuntimeError> {
    let store = store::RunStore::open(project_root)?;
    let state = store.run(run_id)?;
    verify_work_outputs(project_root, &state, expected_execution_id)
}

/// Verifies caller-supplied prepared state, not a persisted progression operation.
pub fn verify_work_outputs(
    project_root: &Utf8Path,
    state: &RunState,
    expected_execution_id: &str,
) -> Result<(), RuntimeError> {
    let root = crate::project::absolute_root(project_root)
        .map_err(|source| path_error(project_root, source))?;
    let run_path = root.join(".unelos/runs").join(&state.run_id);
    store::validate_state(state)?;
    store::validate_artifacts(state, &run_path)?;
    state.expect_execution(expected_execution_id)?;
    let record = match &state.current {
        CurrentWork::Task(record) | CurrentWork::Parallel(record) => record,
        _ => {
            return Err(RuntimeError::InvalidState(
                "output verification requires current task work".into(),
            ))
        }
    };
    if record.confirmed_execution_id.as_deref() != Some(expected_execution_id) {
        return Err(RuntimeError::InvalidState(format!(
            "outputs for execution {expected_execution_id:?} have not been confirmed"
        )));
    }
    for output in record
        .outputs
        .iter()
        .chain(record.worker_reports.iter().map(|r| &r.report))
    {
        artifacts::require_nonempty_file(&output.path).map_err(|cause| {
            RuntimeError::InvalidState(format!(
                "declared output {:?} at {} is unavailable: {cause}",
                output.name, output.path
            ))
        })?;
    }
    artifacts::verify_freshness(record)?;
    Ok(())
}

pub fn next(project_root: &Utf8Path, run_id: &str) -> Result<WorkPacket, RuntimeError> {
    let store = store::RunStore::open(project_root)?;
    let state = store.run(run_id)?;
    let project = crate::load_project(project_root, &state.workflow_name)?;
    crate::validate_project(&project)?;
    packet_for_state(&project, &state)
}

fn packet_for_state(project: &LoadedProject, state: &RunState) -> Result<WorkPacket, RuntimeError> {
    packet_for_snapshot(project, state, true)
}

fn packet_for_snapshot(
    project: &LoadedProject,
    state: &RunState,
    require_inputs: bool,
) -> Result<WorkPacket, RuntimeError> {
    let record = match &state.current {
        CurrentWork::Task(record)
        | CurrentWork::Parallel(record)
        | CurrentWork::HumanGate(record) => record,
        CurrentWork::Finished { node_id } => {
            let input = WorkRuntimeInput::Finished {
                run_id: state.run_id.clone(),
                node_id: node_id.clone(),
                ledger_path: project
                    .project_root
                    .join(".unelos/runs")
                    .join(&state.run_id)
                    .join("ledger.md"),
            };
            return Ok(crate::build_work_packet(project, input)?);
        }
    };
    if require_inputs {
        artifacts::ensure_inputs_are_files(record)?;
    }
    let node = project
        .workflow
        .nodes
        .get(&record.node_id)
        .ok_or_else(|| mismatch(&record.node_id, "current Node was removed"))?;
    let ledger_path = project
        .project_root
        .join(".unelos/runs")
        .join(&state.run_id)
        .join("ledger.md");
    let input = match (&state.current, node) {
        (CurrentWork::Task(_), NodeDefinition::Task(task)) => {
            if !same_input_names(&task.inputs, &record.inputs) {
                return Err(mismatch(&record.node_id, "task input declarations changed"));
            }
            WorkRuntimeInput::Task {
                input: task_input(
                    state,
                    &record.node_id,
                    &record.execution_id,
                    record.inputs.clone(),
                    record.outputs.clone(),
                    record.feedback.clone(),
                ),
                ledger_path,
            }
        }
        (CurrentWork::Parallel(_), NodeDefinition::Parallel(parallel)) => {
            if !same_input_names(&parallel.inputs, &record.inputs) {
                return Err(mismatch(
                    &record.node_id,
                    "parallel input declarations changed",
                ));
            }
            if record.worker_reports.len() != parallel.tasks.len() {
                return Err(mismatch(&record.node_id, "parallel worker count changed"));
            }
            let reports = record.worker_reports.clone();
            for (index, binding) in reports.iter().enumerate() {
                if binding.key != format!("worker-{}", index + 1) {
                    return Err(mismatch(&record.node_id, "parallel worker keys changed"));
                }
            }
            let aggregate_count = parallel.outputs.len();
            if record.outputs.len() != aggregate_count {
                return Err(mismatch(
                    &record.node_id,
                    "parallel output bindings changed",
                ));
            }
            WorkRuntimeInput::Parallel {
                input: task_input(
                    state,
                    &record.node_id,
                    &record.execution_id,
                    record.inputs.clone(),
                    record.outputs.clone(),
                    record.feedback.clone(),
                ),
                worker_reports: reports,
                ledger_path,
            }
        }
        (CurrentWork::HumanGate(_), NodeDefinition::HumanGate(gate)) => {
            if !same_input_names(&gate.inputs, &record.inputs) {
                return Err(mismatch(&record.node_id, "gate input declarations changed"));
            }
            WorkRuntimeInput::HumanGate {
                run_id: state.run_id.clone(),
                activation_id: record.execution_id.clone(),
                node_id: record.node_id.clone(),
                artifacts: record.inputs.clone(),
                ledger_path,
            }
        }
        _ => return Err(mismatch(&record.node_id, "current Node kind changed")),
    };
    let packet = crate::build_work_packet(project, input)?;
    if let (CurrentWork::Parallel(_), WorkPacket::Parallel(packet)) = (&state.current, &packet) {
        if packet.workers.len() != record.worker_reports.len() {
            return Err(mismatch(
                &record.node_id,
                "parallel worker bindings changed",
            ));
        }
        for (worker, saved) in packet.workers.iter().zip(&record.worker_reports) {
            if worker.key != saved.key || worker.outputs[0] != saved.report {
                return Err(mismatch(
                    &record.node_id,
                    "parallel worker artifact bindings changed",
                ));
            }
        }
    }
    Ok(packet)
}

fn mismatch(node: &str, cause: &str) -> RuntimeError {
    RuntimeError::InvalidState(format!(
        "saved work for Node {node:?} is incompatible with live Workflow: {cause}"
    ))
}
fn same_input_names(names: &[String], bindings: &[ArtifactReference]) -> bool {
    names.len() == bindings.len()
        && names
            .iter()
            .zip(bindings)
            .all(|(name, binding)| name == &binding.name)
}
fn task_input(
    state: &RunState,
    node: &str,
    task_id: &str,
    inputs: Vec<ArtifactReference>,
    outputs: Vec<ArtifactReference>,
    feedback: Option<TransitionFeedback>,
) -> TaskRuntimeInput {
    TaskRuntimeInput {
        run_id: state.run_id.clone(),
        task_id: task_id.into(),
        node_id: node.into(),
        request: state.request.clone(),
        inputs,
        outputs,
        feedback,
    }
}
fn allocate_outputs(
    dir: &Utf8Path,
    names: &[String],
) -> Result<Vec<ArtifactReference>, RuntimeError> {
    names
        .iter()
        .map(|name| {
            for part in name.split('/') {
                check_component(part, "artifact output")?;
            }
            Ok(ArtifactReference {
                name: name.clone(),
                path: dir.join(name),
            })
        })
        .collect::<Result<Vec<_>, RuntimeError>>()
        .and_then(|outputs| {
            validate_paths(&outputs)?;
            Ok(outputs)
        })
}
fn check_component(component: &str, source: &str) -> Result<(), RuntimeError> {
    let upper = component
        .split('.')
        .next()
        .unwrap_or(component)
        .to_ascii_uppercase();
    let device = ["CON", "PRN", "AUX", "NUL"].contains(&upper.as_str())
        || (upper.len() == 4
            && ["COM", "LPT"].iter().any(|p| {
                upper.starts_with(p)
                    && upper.as_bytes()[3].is_ascii_digit()
                    && upper.as_bytes()[3] != b'0'
            }));
    if component.is_empty()
        || matches!(component, "." | "..")
        || component.ends_with(['.', ' '])
        || component
            .chars()
            .any(|c| c.is_control() || "<>:\"|?*\\/".contains(c))
        || device
    {
        return Err(RuntimeError::InvalidState(format!(
            "unsafe {source} path component {component:?}"
        )));
    }
    Ok(())
}
fn validate_paths(paths: &[ArtifactReference]) -> Result<(), RuntimeError> {
    let mut seen: Vec<String> = Vec::new();
    for path in paths {
        let normalized = path.path.as_str().to_lowercase().replace('\\', "/");
        if seen.iter().any(|old| {
            old == &normalized
                || old.starts_with(&(normalized.clone() + "/"))
                || normalized.starts_with(&(old.clone() + "/"))
        }) {
            return Err(RuntimeError::InvalidState(format!(
                "colliding artifact path {}",
                path.path
            )));
        }
        seen.push(normalized);
    }
    Ok(())
}
fn output_directories(base: &Utf8Path, outputs: &[ArtifactReference]) -> Vec<Utf8PathBuf> {
    let mut dirs = if outputs.is_empty() {
        Vec::new()
    } else {
        vec![base.to_owned()]
    };
    for output in outputs {
        let parent = output.path.parent().unwrap();
        let mut current = base.to_owned();
        if let Ok(relative) = parent.strip_prefix(base) {
            for component in relative.components() {
                current.push(component.as_str());
                dirs.push(current.clone());
            }
        }
    }
    dirs.sort();
    dirs.dedup();
    dirs
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state() -> RunState {
        RunState {
            started_at: None,
            updated_at: None,
            run_id: "qa-run".into(),
            name: "qa-run".into(),
            workflow_name: "implement-feature".into(),
            request: "Change color".into(),
            naming_pending: false,
            current: CurrentWork::Finished {
                node_id: "done".into(),
            },
            history: vec![],
            next_execution_sequence: 1,
        }
    }

    #[test]
    fn execution_ids_allocate_monotonically_and_roundtrip() {
        let mut state = state();
        assert_eq!(state.allocate_execution_id(false).unwrap(), "task-1");
        assert_eq!(state.allocate_execution_id(false).unwrap(), "task-2");
        assert_eq!(state.allocate_execution_id(true).unwrap(), "gate-3");
        let bytes = serde_json::to_vec(&state).unwrap();
        assert!(std::str::from_utf8(&bytes)
            .unwrap()
            .contains("\"nodeId\":\"done\""));
        assert_eq!(serde_json::from_slice::<RunState>(&bytes).unwrap(), state);
    }

    #[test]
    fn current_execution_must_match_expected_identity() {
        let mut state = state();
        state.current = CurrentWork::Task(ExecutionRecord {
            node_id: "plan".into(),
            execution_id: "task-2".into(),
            inputs: vec![],
            outputs: vec![],
            worker_reports: vec![],
            output_digests: Default::default(),
            worker_receipts: Default::default(),
            invalid_worker_results: 0,
            feedback: None,
            confirmed_execution_id: None,
        });
        assert!(state.expect_execution("task-2").is_ok());
        assert!(state.expect_execution("task-1").is_err());
        state.current = CurrentWork::Finished {
            node_id: "done".into(),
        };
        assert!(state.expect_execution("task-2").is_err());
    }

    #[test]
    fn execution_sequence_overflow_is_an_error() {
        let mut state = state();
        state.next_execution_sequence = u64::MAX;
        assert!(state.allocate_execution_id(false).is_err());
    }

    #[test]
    fn finished_state_and_run_name_rules_are_derived_and_portable() {
        let mut state = state();
        assert!(state.is_finished());
        state.current = CurrentWork::Task(ExecutionRecord {
            node_id: "plan".into(),
            execution_id: "task-1".into(),
            inputs: vec![],
            outputs: vec![],
            worker_reports: vec![],
            output_digests: Default::default(),
            worker_receipts: Default::default(),
            invalid_worker_results: 0,
            feedback: None,
            confirmed_execution_id: None,
        });
        assert!(!state.is_finished());
        let max_name = "a".repeat(64);
        for valid in ["qa-run", "a1", max_name.as_str()] {
            assert!(valid_run_name(valid));
        }
        for invalid in ["", "../bad", "Upper", "a--b", "con", "nul"] {
            assert!(!valid_run_name(invalid));
        }
    }
}
