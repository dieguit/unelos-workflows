use crate::{Execution, RuntimeError, WorkPacket};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum ExecutionIdentity {
    Task {
        run_id: String,
        node_id: String,
        task_id: String,
    },
    HumanGate {
        run_id: String,
        node_id: String,
        activation_id: String,
    },
}

impl ExecutionIdentity {
    pub fn from_packet(packet: &WorkPacket) -> Option<Self> {
        match packet {
            WorkPacket::Task(task) => Some(Self::Task {
                run_id: task.packet.run_id.clone(),
                node_id: task.packet.context.node_id.clone(),
                task_id: task.packet.task_id.clone(),
            }),
            WorkPacket::HumanGate(gate) => Some(Self::HumanGate {
                run_id: gate.run_id.clone(),
                node_id: gate.node_id.clone(),
                activation_id: gate.activation_id.clone(),
            }),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExecutionControl {
    pub expected: ExecutionIdentity,
    pub result_text: Option<String>,
    pub outcome: Option<String>,
    pub reason: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionPackage {
    pub run_id: Option<String>,
    pub entry_mode: Option<ExecutionEntry>,
    pub resume_context: Option<ResumeContext>,
    pub status: String,
    pub workflows: Vec<String>,
    pub workflow_descriptions: std::collections::BTreeMap<String, String>,
    pub packet: Option<WorkPacket>,
    pub expected: Option<ExecutionIdentity>,
    pub controller_instructions: String,
    pub worker_prompt: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ExecutionEntry {
    New,
    Resume,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunSummary {
    pub run_id: String,
    pub name: String,
    pub workflow_name: String,
    pub status: String,
    pub node_id: String,
    pub expected: Option<ExecutionIdentity>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunCatalog {
    pub entry_mode: ExecutionEntry,
    pub controller_instructions: String,
    pub runs: Vec<RunSummary>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResumeFile {
    pub path: camino::Utf8PathBuf,
    pub content: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResumeContext {
    pub state: crate::RunState,
    pub ledger: ResumeFile,
    pub reports: Vec<ResumeFile>,
    pub issues: Vec<String>,
}

#[derive(Clone, Copy, Debug)]
pub enum ExecutionOperation {
    Complete,
    Approve,
    Reject,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ContinuationControl {
    pub expected: ExecutionIdentity,
    pub answer: Option<String>,
    pub progress: Vec<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum ExecutionError {
    #[error(transparent)]
    Runtime(#[from] RuntimeError),
    #[error("{message}")]
    Protocol { code: &'static str, message: String },
}

impl ExecutionError {
    pub(crate) fn protocol(code: &'static str, message: impl Into<String>) -> Self {
        Self::Protocol {
            code,
            message: message.into(),
        }
    }

    pub(crate) fn after_commit(_: Self) -> Self {
        Self::protocol(
            "mutation_uncertain",
            "State committed but response unavailable; read current state, never blindly retry",
        )
    }

    pub fn code(&self) -> &'static str {
        match self {
            Self::Protocol { code, .. } => code,
            Self::Runtime(
                RuntimeError::StaleExecution { .. }
                | RuntimeError::StaleRun { .. }
                | RuntimeError::WrongNode { .. },
            ) => "stale_execution",
            Self::Runtime(_) => "execution_failed",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum WorkerStatus {
    Finished,
    NeedsContext,
    Blocked,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkerResult {
    pub kind: String,
    pub run_id: String,
    pub node_id: String,
    pub task_id: String,
    pub status: WorkerStatus,
    pub summary: String,
    pub changed_paths: Vec<String>,
    pub verification: Vec<String>,
    pub progress: Vec<String>,
    pub outcome: Option<String>,
    pub question: Option<String>,
    pub depends_on: Option<String>,
    pub reason: Option<String>,
}

pub fn read_brief(text: &str) -> Result<String, ExecutionError> {
    let mut found = false;
    let mut collecting = false;
    let mut lines = Vec::new();
    for line in text.lines() {
        if line.trim_end() == "## Brief" {
            if found {
                return Err(ExecutionError::protocol(
                    "missing_brief",
                    "Record exactly one agreed Brief",
                ));
            }
            found = true;
            collecting = true;
        } else if line.starts_with("## ") {
            collecting = false;
        } else if collecting {
            lines.push(line);
        }
    }
    let brief = lines.join("\n").trim().to_owned();
    if brief.is_empty() {
        return Err(ExecutionError::protocol(
            "missing_brief",
            "Record one nonempty agreed Brief",
        ));
    }
    Ok(brief)
}

pub fn render_execution_package(
    packet: Option<WorkPacket>,
    workflows: Vec<String>,
    brief: Option<&str>,
    continuation: Option<&ContinuationControl>,
) -> Result<ExecutionPackage, ExecutionError> {
    let expected = packet.as_ref().and_then(ExecutionIdentity::from_packet);
    if let Some(control) = continuation {
        if expected.as_ref() != Some(&control.expected) {
            return Err(ExecutionError::protocol(
                "stale_execution",
                "Continuation Task is no longer current",
            ));
        }
        if control.answer.as_ref().is_some_and(|s| s.trim().is_empty()) {
            return Err(ExecutionError::protocol(
                "invalid_control",
                "Continuation answer must be nonempty or null",
            ));
        }
    }
    let status = match &packet {
        None => "idle",
        Some(WorkPacket::HumanGate(_)) => "waiting",
        Some(WorkPacket::Finished(_)) => "finished",
        _ => "running",
    };
    let worker_prompt = match &packet {
        Some(WorkPacket::Task(task)) if task.packet.context.execution == Execution::Subagent => {
            let brief = brief.filter(|s| !s.trim().is_empty()).ok_or_else(|| {
                ExecutionError::protocol(
                    "missing_brief",
                    "Record one agreed Brief before delegation",
                )
            })?;
            let context = serde_json::to_string_pretty(&task.packet).map_err(|_| {
                ExecutionError::protocol("invalid_packet", "Cannot render execution context")
            })?;
            let continuation = serde_json::json!({
                "answer": continuation.and_then(|c| c.answer.as_ref()),
                "progress": continuation.map(|c| c.progress.as_slice()).unwrap_or(&[]),
            });
            let result_instructions = WORKER_RESULT_INSTRUCTIONS.trim_end();
            Some(format!("Executor context:\n{context}\n\nBrief:\n{brief}\n\nContinuation context:\n{continuation}\n\n{result_instructions}"))
        }
        _ if continuation.is_some() => {
            return Err(ExecutionError::protocol(
                "unsupported_execution",
                "Only delegated serial Tasks accept worker continuations",
            ))
        }
        _ => None,
    };
    Ok(ExecutionPackage {
        entry_mode: None,
        resume_context: None,
        run_id: packet.as_ref().map(|packet| match packet {
            WorkPacket::Task(task) => task.packet.run_id.clone(),
            WorkPacket::HumanGate(gate) => gate.run_id.clone(),
            WorkPacket::Parallel(parallel) => parallel.run_id.clone(),
            WorkPacket::Finished(finished) => finished.run_id.clone(),
        }),
        status: status.into(),
        workflows,
        workflow_descriptions: Default::default(),
        packet,
        expected,
        controller_instructions: include_str!("instructions/controller.md").into(),
        worker_prompt,
    })
}

const WORKER_RESULT_INSTRUCTIONS: &str = include_str!("instructions/worker-result.md");

pub fn parse_worker_result(
    text: &str,
    expected: &ExecutionIdentity,
) -> Result<WorkerResult, ExecutionError> {
    let invalid = || {
        ExecutionError::protocol(
            "invalid_worker_result",
            "Worker result must be one intact frame for the expected Task",
        )
    };
    let open = "<unelos-worker-result>";
    let close = "</unelos-worker-result>";
    if text.matches(open).count() != 1 || text.matches(close).count() != 1 {
        return Err(invalid());
    }
    let json = text
        .split_once(open)
        .ok_or_else(invalid)?
        .1
        .split_once(close)
        .ok_or_else(invalid)?
        .0;
    let value: serde_json::Value = serde_json::from_str(json).map_err(|_| invalid())?;
    let result: WorkerResult = serde_json::from_value(value.clone()).map_err(|_| invalid())?;
    let identity = ExecutionIdentity::Task {
        run_id: result.run_id.clone(),
        node_id: result.node_id.clone(),
        task_id: result.task_id.clone(),
    };
    if &identity != expected || result.kind != "task" {
        return Err(invalid());
    }
    let nonempty = |s: &Option<String>| s.as_ref().is_some_and(|s| !s.trim().is_empty());
    let valid = match result.status {
        WorkerStatus::Finished => {
            value.get("outcome").is_some()
                && (result.outcome.is_none() || nonempty(&result.outcome))
                && result.question.is_none()
                && result.depends_on.is_none()
                && result.reason.is_none()
        }
        WorkerStatus::NeedsContext => {
            nonempty(&result.question)
                && nonempty(&result.depends_on)
                && result.reason.is_none()
                && result.outcome.is_none()
        }
        WorkerStatus::Blocked => {
            nonempty(&result.reason)
                && result.question.is_none()
                && result.depends_on.is_none()
                && result.outcome.is_none()
        }
    };
    if !valid {
        return Err(invalid());
    }
    Ok(result)
}
