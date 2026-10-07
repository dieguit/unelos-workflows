use camino::{Utf8Path, Utf8PathBuf};
use serde::{Deserialize, Serialize};

use crate::{
    ArtifactReference, ContextError, ProjectLoadError, ProjectValidationError, TransitionFeedback,
    WorkerReportBinding,
};

mod store;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunState {
    pub run_id: String,
    pub name: String,
    pub workflow_name: String,
    pub request: String,
    pub current: CurrentWork,
    pub history: Vec<CompletedExecution>,
    pub next_execution_sequence: u64,
}

impl RunState {
    pub fn is_finished(&self) -> bool {
        matches!(self.current, CurrentWork::Finished { .. })
    }

    // Task 2 wires this internal allocator into public runtime operations.
    #[allow(dead_code)]
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
    pub feedback: Option<TransitionFeedback>,
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
    #[error("no active run in {path}")]
    NoActiveRun { path: Utf8PathBuf },
    #[error("run {name:?} is already active")]
    ActiveRun { name: String },
    #[error("run {name:?} already exists")]
    Collision { name: String },
    #[error("invalid persisted run state: {0}")]
    InvalidState(String),
    #[error("stale execution {expected:?}; current execution is {current:?}")]
    StaleExecution {
        expected: String,
        current: Option<String>,
    },
}

// Task 2 wires these private storage helpers into public runtime operations.
#[allow(dead_code)]
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

#[allow(dead_code)]
pub(crate) fn path_error(path: &Utf8Path, source: std::io::Error) -> RuntimeError {
    RuntimeError::Io {
        path: path.to_owned(),
        source,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state() -> RunState {
        RunState {
            run_id: "qa-run".into(),
            name: "qa-run".into(),
            workflow_name: "implement-feature".into(),
            request: "Change color".into(),
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
            feedback: None,
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
            feedback: None,
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
