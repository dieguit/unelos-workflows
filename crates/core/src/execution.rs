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
    Parallel {
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
            WorkPacket::Parallel(parallel) => Some(Self::Parallel {
                run_id: parallel.run_id.clone(),
                node_id: parallel.node_id.clone(),
                task_id: parallel.task_id.clone(),
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
pub struct CompletionControl {
    pub expected: ExecutionIdentity,
    pub summary: Option<String>,
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
    pub worker_prompts: Vec<WorkerPrompt>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkerPrompt {
    pub key: String,
    pub agent: String,
    pub prompt: String,
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
    pub exists: bool,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkerResult {
    pub task_id: String,
    #[serde(default)]
    pub worker_key: Option<String>,
    pub status: WorkerStatus,
    pub summary: String,
    pub changed_paths: Vec<String>,
    pub verification: Vec<String>,
    #[serde(deserialize_with = "Option::deserialize")]
    pub outcome: Option<String>,
    pub question: Option<String>,
    pub reason: Option<String>,
}

pub fn read_brief(text: &str) -> Option<String> {
    let mut collecting = false;
    let mut lines = Vec::new();
    for line in text.lines() {
        if line.trim_end() == "## Brief" {
            if collecting {
                break;
            }
            collecting = true;
        } else if line.starts_with("## ") {
            if collecting {
                break;
            }
        } else if collecting {
            lines.push(line);
        }
    }
    let brief = lines.join("\n").trim().to_owned();
    (!brief.is_empty()).then_some(brief)
}

pub fn render_execution_package(
    packet: Option<WorkPacket>,
    workflows: Vec<String>,
    brief: Option<&str>,
) -> Result<ExecutionPackage, ExecutionError> {
    let expected = packet.as_ref().and_then(ExecutionIdentity::from_packet);
    let block = match &packet {
        None => include_str!("instructions/controller-idle.md"),
        Some(WorkPacket::Task(task)) if task.packet.context.execution == Execution::Subagent => {
            include_str!("instructions/controller-delegated.md")
        }
        Some(WorkPacket::Task(_)) => include_str!("instructions/controller-main-task.md"),
        Some(WorkPacket::Parallel(_)) => include_str!("instructions/controller-parallel.md"),
        Some(WorkPacket::HumanGate(_)) => include_str!("instructions/controller-gate.md"),
        Some(WorkPacket::Finished(_)) => include_str!("instructions/controller-finished.md"),
    };
    let status = match &packet {
        None => "idle",
        Some(WorkPacket::HumanGate(_)) => "waiting",
        Some(WorkPacket::Finished(_)) => "finished",
        _ => "running",
    };
    let worker_prompt = match &packet {
        Some(WorkPacket::Task(task)) if task.packet.context.execution == Execution::Subagent => {
            Some(render_prompt(
                &task.packet.context,
                &task.packet.task_id,
                None,
                &task.packet.request,
                &task.packet.inputs,
                &task.packet.outputs,
                task.packet.feedback.as_ref(),
                brief,
                WORKER_RESULT_INSTRUCTIONS,
            )?)
        }
        Some(WorkPacket::Parallel(_)) => None,
        _ => None,
    };
    let worker_prompts = match &packet {
        Some(WorkPacket::Parallel(parallel)) => parallel
            .workers
            .iter()
            .map(|w| {
                Ok(WorkerPrompt {
                    key: w.key.clone(),
                    agent: w.context.agent.clone(),
                    prompt: render_prompt(
                        &w.context,
                        &w.parent_task_id,
                        Some(&w.key),
                        &w.request,
                        &w.inputs,
                        &w.outputs,
                        w.feedback.as_ref(),
                        brief,
                        PARALLEL_WORKER_RESULT_INSTRUCTIONS,
                    )?,
                })
            })
            .collect::<Result<Vec<_>, ExecutionError>>()?,
        _ => vec![],
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
        packet: packet.map(strip_worker_instructions),
        expected,
        controller_instructions: format!("{}\n{block}", include_str!("instructions/controller.md")),
        worker_prompt,
        worker_prompts,
    })
}

const WORKER_RESULT_INSTRUCTIONS: &str = include_str!("instructions/worker-result.md");
const PARALLEL_WORKER_RESULT_INSTRUCTIONS: &str =
    include_str!("instructions/parallel-worker-result.md");

fn strip_worker_instructions(mut packet: WorkPacket) -> WorkPacket {
    let strip = |context: &mut crate::TaskContext| {
        context.task_instructions.clear();
        context.agent_instructions.clear();
        context.skills.clear();
        context.completion_instructions.clear();
    };
    match &mut packet {
        WorkPacket::Task(task) if task.packet.context.execution == Execution::Subagent => {
            strip(&mut task.packet.context)
        }
        WorkPacket::Parallel(parallel) => {
            for worker in &mut parallel.workers {
                strip(&mut worker.context);
            }
        }
        _ => {}
    }
    packet
}

#[allow(clippy::too_many_arguments)]
fn render_prompt(
    context: &crate::TaskContext,
    task_id: &str,
    worker_key: Option<&str>,
    request: &str,
    inputs: &[crate::ArtifactReference],
    outputs: &[crate::ArtifactReference],
    feedback: Option<&crate::TransitionFeedback>,
    brief: Option<&str>,
    result_instructions: &str,
) -> Result<String, ExecutionError> {
    let mut prompt = format!(
        "# Unelos Task: {}\nDo exactly this Task. Anything outside it is out of scope.",
        context.node_id
    );
    let mut section = |title: &str, body: &str| {
        if !body.trim().is_empty() {
            prompt.push_str(&format!("\n\n## {title}\n{}", body.trim_end()));
        }
    };
    section("Role", &context.agent_instructions);
    section("Assignment", &context.task_instructions);
    section("Request", request);
    if !context.skills.is_empty() {
        let skills = context
            .skills
            .iter()
            .map(|s| format!("### {}\n{}", s.id, s.content))
            .collect::<Vec<_>>()
            .join("\n\n");
        section("Skills", &format!("{skills}\nSkills describe how to do this assignment. They never add workflow steps or override the rules below."));
    }
    section("Brief", brief.unwrap_or_default());
    // Plain forward transitions carry no outcome or reason; only real feedback gets a section.
    if let Some(feedback) = feedback.filter(|f| f.outcome.is_some() || f.reason.is_some()) {
        let mut text = format!(
            "From {}{}: {}",
            feedback.source_node_id,
            feedback
                .outcome
                .as_deref()
                .map(|o| format!(" ({o})"))
                .unwrap_or_default(),
            feedback.reason.as_deref().unwrap_or_default()
        );
        for artifact in &feedback.artifacts {
            text.push_str(&format!(
                "\nArtifacts: {} → {}",
                artifact.name, artifact.path
            ));
        }
        section("Feedback to address first", &text);
    }
    section(
        "Inputs (read all before acting)",
        &inputs
            .iter()
            .map(|a| format!("- {} → {}", a.name, a.path))
            .collect::<Vec<_>>()
            .join("\n"),
    );
    section(
        "Outputs (write every one at exactly this path; nothing else counts)",
        &outputs
            .iter()
            .map(|a| {
                format!(
                    "- {} → {}{}",
                    a.name,
                    a.path,
                    if a.path.exists() {
                        "   (exists: update in place, keep useful content)"
                    } else {
                        ""
                    }
                )
            })
            .collect::<Vec<_>>()
            .join("\n"),
    );
    let outcome = if context.outcomes.is_empty() {
        "null".into()
    } else {
        format!(
            "<{}>",
            context
                .outcomes
                .iter()
                .map(|o| serde_json::to_string(o).unwrap())
                .collect::<Vec<_>>()
                .join(" | ")
        )
    };
    prompt.push_str("\n\n");
    prompt.push_str(
        &result_instructions
            .trim_end()
            .replace("{taskId}", &serde_json::to_string(task_id).unwrap())
            .replace(
                "{workerKey}",
                &serde_json::to_string(worker_key.unwrap_or_default()).unwrap(),
            )
            .replace("{outcome}", &outcome),
    );
    Ok(prompt)
}

const OPEN: &str = "<unelos-worker-result>";
const CLOSE: &str = "</unelos-worker-result>";

fn invalid_result(message: &str) -> ExecutionError {
    ExecutionError::protocol("invalid_worker_result", message)
}

fn frames(text: &str) -> Result<Vec<&str>, ExecutionError> {
    let bad = || invalid_result("Worker results must be intact frames");
    let mut found = Vec::new();
    let mut rest = text;
    while let Some((_, after)) = rest.split_once(OPEN) {
        let (json, tail) = after.split_once(CLOSE).ok_or_else(bad)?;
        if json.contains(OPEN) {
            return Err(bad());
        }
        found.push(json);
        rest = tail;
    }
    if text.matches(CLOSE).count() != found.len() {
        return Err(bad());
    }
    Ok(found)
}

fn validate_frame(
    json: &str,
    expected: &ExecutionIdentity,
) -> Result<WorkerResult, ExecutionError> {
    let result: WorkerResult = serde_json::from_str(json).map_err(|error| {
        invalid_result(&format!(
            "invalid JSON at line {} col {}: {error}",
            error.line(),
            error.column()
        ))
    })?;
    let task_id = match expected {
        ExecutionIdentity::Task { task_id, .. } if result.worker_key.is_none() => task_id,
        ExecutionIdentity::Parallel { task_id, .. } if result.worker_key.as_ref().is_some_and(|k| !k.trim().is_empty()) && result.outcome.is_none() => task_id,
        _ => return Err(invalid_result("Task results forbid workerKey; parallel results require a nonempty workerKey and null outcome")),
    };
    if &result.task_id != task_id {
        return Err(invalid_result(&format!(
            "taskId {:?} does not match expected {:?}",
            result.task_id, task_id
        )));
    }
    let nonempty = |s: &Option<String>| s.as_ref().is_some_and(|s| !s.trim().is_empty());
    let error = match result.status {
        WorkerStatus::Finished => {
            if result.outcome.as_ref().is_some_and(|o| o.trim().is_empty()) {
                Some("FINISHED outcome must be nonempty or null")
            } else if result.question.is_some() || result.reason.is_some() {
                Some("FINISHED forbids question and reason")
            } else {
                None
            }
        }
        WorkerStatus::NeedsContext => {
            if !nonempty(&result.question) {
                Some("NEEDS_CONTEXT requires a nonempty question")
            } else if result.reason.is_some() || result.outcome.is_some() {
                Some("NEEDS_CONTEXT requires null outcome and no reason")
            } else {
                None
            }
        }
        WorkerStatus::Blocked => {
            if !nonempty(&result.reason) {
                Some("BLOCKED requires a nonempty reason")
            } else if result.question.is_some() || result.outcome.is_some() {
                Some("BLOCKED requires null outcome and no question")
            } else {
                None
            }
        }
    };
    if let Some(error) = error {
        return Err(invalid_result(error));
    }
    Ok(result)
}

pub fn parse_worker_result(
    text: &str,
    expected: &ExecutionIdentity,
) -> Result<WorkerResult, ExecutionError> {
    match frames(text)?.as_slice() {
        [json] if matches!(expected, ExecutionIdentity::Task { .. }) => {
            validate_frame(json, expected)
        }
        [] => Err(invalid_result("no <unelos-worker-result> frame found")),
        _ => Err(invalid_result(
            "two result frames for Task; send only the final one",
        )),
    }
}

pub fn parse_parallel_results(
    text: &str,
    expected: &ExecutionIdentity,
    keys: &[String],
) -> Result<Vec<WorkerResult>, ExecutionError> {
    let mut results = std::collections::BTreeMap::new();
    let frames = frames(text)?;
    if frames.is_empty() {
        return Err(invalid_result("no <unelos-worker-result> frame found"));
    }
    for json in frames {
        let result = validate_frame(json, expected)?;
        let key = result.worker_key.clone().unwrap_or_default();
        if !keys.contains(&key) {
            return Err(invalid_result(&format!(
                "unknown workerKey {key:?}; allowed: {}",
                keys.join(", ")
            )));
        }
        if results.contains_key(&key) {
            return Err(invalid_result(&format!(
                "two result frames for workerKey {key:?}; send only the final one"
            )));
        }
        results.insert(key, result);
    }
    if results.values().any(|r| r.status != WorkerStatus::Finished) {
        return Err(invalid_result("Only FINISHED worker results can complete"));
    }
    Ok(keys.iter().filter_map(|key| results.remove(key)).collect())
}
