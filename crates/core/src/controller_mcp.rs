use crate::session_binding::{
    canonical, fail, io_error, PendingStart, SessionContext, SessionError, SessionService,
};
use crate::{CompletionControl, ExecutionOperation};
use rmcp::{model::*, service::RequestContext, ErrorData, RoleServer, ServerHandler};
use serde_json::{json, Value};

const TOOLS: [(&str, &str); 9] = [
    ("next", "Read the selected Unelos execution package"),
    ("start", "Start a fresh Unelos workflow and bind this session"),
    ("resume", "Bind the explicitly selected unfinished run and immediately continue using core resume context"),
    ("complete", "Complete the expected Unelos Task after inspection"),
    ("approve", "Record explicit approval of the expected human gate"),
    ("reject", "Reject the expected human gate with actionable feedback"),
    ("status", "Read selected Unelos status"),
    ("inspect", "Inspect a Unelos workflow or Node"),
    ("rename", "Name the selected Unelos run"),
];

fn object(properties: Value, required: &[&str]) -> Value {
    json!({"type":"object","properties":properties,"required":required,"additionalProperties":false})
}

pub fn tool_definitions() -> Vec<Tool> {
    let string = json!({"type":"string"});
    let identity = object(
        json!({"kind":{"type":"string","enum":["task","human-gate","parallel"]},"runId":string,"nodeId":string,"taskId":string,"activationId":string}),
        &["kind", "runId", "nodeId"],
    );
    TOOLS.iter().map(|(name, description)| {
        let mut schema = match *name {
            "start" => object(json!({"workflow":string,"name":string}), &["workflow"]),
            "resume" => object(json!({"runId":string}), &["runId"]),
            "complete" => object(json!({"expected":identity,
                "resultText":{"type":"string","description":"the worker's final message, verbatim (only newly finished workers for parallel; write aggregates before the last submission; empty once all receipts exist)"},
                "summary":{"type":"string","description":"One-paragraph summary for main Tasks"},
                "outcome":{"type":"string","description":"main Tasks only"}}), &["expected"]),
            "approve" => object(json!({"expected":identity}), &["expected"]),
            "reject" => object(json!({"expected":identity,"reason":string}), &["expected","reason"]),
            "inspect" => object(json!({"workflow":string,"node":string}), &["workflow"]),
            "rename" => object(json!({"name":string,"runId":string}), &["name","runId"]),
            _ => object(json!({}), &[]),
        };
        // Opaque on purpose: the harness hook injects it; a detailed schema invites the model to fill it.
        schema["properties"]["unelos_context"] =
            json!({"type":"object","description":"Injected by the harness. Never set it."});
        serde_json::from_value(json!({"name":name,"description":description,"inputSchema":schema}))
            .expect("static tool definition")
    }).collect()
}

pub fn harness_instructions(harness: &str) -> String {
    let prefix = if harness == "claude" {
        "mcp__plugin_unelos_unelos__"
    } else {
        "unelos_"
    };
    let names = TOOLS
        .iter()
        .map(|(name, _)| format!("{prefix}{name}"))
        .collect::<Vec<_>>()
        .join(", ");
    let workers = if harness == "claude" {
        "Native worker: Agent(subagent_type=nativeAgent, prompt=workerPrompt).\nParallel workers: in a single message issue one Agent(subagent_type=workerPrompts[i].nativeAgent, prompt=workerPrompts[i].prompt) call per workerPrompts entry before waiting; continue a worker via SendMessage to its agent id."
    } else {
        "Native worker: subagent(agent=nativeAgent, prompt=workerPrompt, sessionID for continuation).\nParallel workers: in one turn call subagent(agent=workerPrompts[i].nativeAgent, prompt=workerPrompts[i].prompt, background=true) for every entry before waiting; continue one worker with its sessionID."
    };
    format!("Native Unelos tools: {names}.\n{workers}")
}

pub fn native_package(mut value: Value, harness: &str) -> Value {
    if value.get("workerPrompt").is_some_and(|v| !v.is_null()) {
        if let Some(agent) = value["packet"]["agent"].as_str() {
            value["nativeAgent"] = json!(format!("unelos-{agent}"));
        }
    }
    if let Some(workers) = value["workerPrompts"].as_array_mut() {
        for worker in workers {
            if let Some(agent) = worker["agent"].as_str() {
                worker["nativeAgent"] = json!(format!("unelos-{agent}"));
            }
        }
    }
    value["harnessInstructions"] = json!(harness_instructions(harness));
    value
}

fn required<'a>(input: &'a Value, key: &str) -> Result<&'a str, SessionError> {
    input[key]
        .as_str()
        .ok_or_else(|| fail("invalid_control", format!("Missing string {key}")))
}

impl SessionService {
    pub fn dispatch(
        &self,
        tool: &str,
        input: &Value,
        meta: Option<&Value>,
    ) -> Result<Value, SessionError> {
        let context = SessionContext::parse(input.get("unelos_context"), meta)?;
        let lock = self.lock(&context)?;
        let mut binding = lock.read()?.filter(|b| b.enabled).ok_or_else(|| {
            fail(
                "controller_inactive",
                "Activate /unelos in the main session first",
            )
        })?;
        if canonical(&context.project)? != binding.project {
            return Err(fail(
                "project_changed",
                "Project changed; reactivate /unelos",
            ));
        }
        if binding.pending_start.is_some() {
            return Err(fail(
                "mutation_uncertain",
                "Explicitly reconcile with /unelos resume; never retry start",
            ));
        }
        let entry = matches!(tool, "start" | "resume");
        if entry {
            if binding.run_id.is_some()
                || binding.mode != if tool == "start" { "new" } else { "resume" }
            {
                return Err(fail(
                    "invalid_entry",
                    "Use /unelos for a fresh start or /unelos resume to select an existing run",
                ));
            }
        } else if tool != "inspect" {
            let run = binding
                .run_id
                .as_deref()
                .ok_or_else(|| fail("run_unbound", "Start or explicitly resume a run first"))?;
            if input
                .get("expected")
                .is_some_and(|expected| expected["runId"].as_str() != Some(run))
                || tool == "rename" && input["runId"].as_str() != Some(run)
            {
                return Err(fail(
                    "stale_execution",
                    "Input identity does not match this session's selected run",
                ));
            }
        }
        let root = &binding.project;
        let run = binding.run_id.as_deref().unwrap_or_default();
        let value = match tool {
            "next" => serde_json::to_value(crate::execution_next(root, run)?).map_err(io_error)?,
            "start" => {
                let workflow = required(input, "workflow")?;
                binding.pending_start = Some(PendingStart {
                    entry_id: binding.entry_id.clone(),
                    at: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs(),
                });
                lock.write(&binding)?;
                let result =
                    crate::execution_start(&binding.project, workflow, input["name"].as_str());
                match result {
                    Ok(package) => serde_json::to_value(package).map_err(|_| fail("mutation_uncertain", "Start response unavailable; explicitly reconcile with /unelos resume; never retry start"))?,
                    Err(error) => {
                        // execution_start wraps every publish failure as mutation_uncertain.
                        if error.code() != "mutation_uncertain" {
                            binding.pending_start = None;
                            lock.write(&binding)?;
                        }
                        return Err(error);
                    }
                }
            }
            "resume" => {
                serde_json::to_value(crate::execution_resume(root, required(input, "runId")?)?)
                    .map_err(io_error)?
            }
            "complete" | "approve" | "reject" => {
                let control: CompletionControl = serde_json::from_value(json!({
                    "expected":input["expected"],"resultText":input["resultText"],
                    "summary":input["summary"],"outcome":input["outcome"],"reason":input["reason"]
                }))
                .map_err(|e| fail("invalid_control", e))?;
                let operation = match tool {
                    "approve" => ExecutionOperation::Approve,
                    "reject" => ExecutionOperation::Reject,
                    _ => ExecutionOperation::Complete,
                };
                serde_json::to_value(crate::execution_mutate(root, run, operation, &control)?)
                    .map_err(io_error)?
            }
            "status" => {
                let state = crate::status(root, run)?;
                run_summary(root, &state, true)?
            }
            "rename" => {
                let state = crate::rename_run(root, run, required(input, "name")?)?;
                run_summary(root, &state, false)?
            }
            "inspect" => {
                let project = crate::load_project(root, required(input, "workflow")?)
                    .map_err(|e| fail("project_load_failed", e))?;
                crate::validate_project(&project).map_err(|e| fail("invalid_project", e))?;
                if let Some(node) = input["node"].as_str() {
                    serde_json::to_value(
                        crate::inspect_node(&project, node)
                            .map_err(|e| fail("context_resolution_failed", e))?,
                    )
                    .map_err(io_error)?
                } else {
                    let graph = crate::build_graph(&project.authoring_workflow);
                    let nodes = project
                        .authoring_workflow
                        .nodes
                        .iter()
                        .map(|(id, node)| {
                            let mut value = serde_json::to_value(node).map_err(io_error)?;
                            value["id"] = json!(id);
                            Ok(value)
                        })
                        .collect::<Result<Vec<_>, SessionError>>()?;
                    json!({"name":project.workflow.name,"start":graph.start,"nodes":nodes,"edges":graph.edges})
                }
            }
            _ => return Err(fail("invalid_tool", "Unknown Unelos tool")),
        };
        if entry {
            let run = value["runId"].as_str().filter(|id| !id.trim().is_empty())
                .ok_or_else(|| fail(if tool == "start" { "mutation_uncertain" } else { "invalid_control" }, "No selected run ID; explicitly reconcile with /unelos resume; never retry start"))?;
            if tool == "resume" && Some(run) != input["runId"].as_str() {
                return Err(fail("invalid_control", "Resume returned a different run"));
            }
            binding.run_id = Some(run.into());
            binding.pending_start = None;
            lock.write(&binding).map_err(|error| if tool == "start" {
                fail("mutation_uncertain", "Binding failed; explicitly reconcile with /unelos resume; never retry start")
            } else { error })?;
        }
        Ok(native_package(value, &context.harness))
    }
}

fn run_summary(
    root: &camino::Utf8Path,
    state: &crate::RunState,
    history: bool,
) -> Result<Value, SessionError> {
    use crate::CurrentWork;
    let (status, kind, node) = match &state.current {
        CurrentWork::Task(record) => ("running", "task", record.node_id.as_str()),
        CurrentWork::Parallel(record) => ("running", "parallel", record.node_id.as_str()),
        CurrentWork::HumanGate(record) => ("waiting", "human-gate", record.node_id.as_str()),
        CurrentWork::Finished { node_id } => ("finished", "finished", node_id.as_str()),
    };
    let mut value = json!({"runId":state.run_id,"name":state.name,"namingPending":state.naming_pending,
        "workflowName":state.workflow_name,"runDirectory":root.join(".unelos/runs").join(&state.run_id),
        "status":status,"current":{"kind":kind,"nodeId":node}});
    if history {
        value["history"] = serde_json::to_value(&state.history).map_err(io_error)?;
    }
    Ok(value)
}

pub fn error_envelope(error: &SessionError) -> Value {
    json!({"error":{"code":error.code(),"message":error.to_string()}})
}

#[derive(Clone)]
pub struct ControllerMcp {
    pub sessions: SessionService,
}

pub async fn serve_stdio() -> Result<(), SessionError> {
    use rmcp::ServiceExt;
    let server = ControllerMcp {
        sessions: SessionService::discover()?,
    };
    server
        .serve(rmcp::transport::stdio())
        .await
        .map_err(|e| fail("mcp_failed", e))?
        .waiting()
        .await
        .map_err(|e| fail("mcp_failed", e))?;
    Ok(())
}

impl ServerHandler for ControllerMcp {
    fn get_info(&self) -> ServerConfig {
        let mut config = ServerConfig::default();
        config.capabilities = ServerCapabilities::builder().enable_tools().build();
        config.server_info.name = "unelos".into();
        config
    }
    async fn list_tools(
        &self,
        _: Option<PaginatedRequestParams>,
        _: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, ErrorData> {
        Ok(ListToolsResult {
            tools: tool_definitions(),
            ..Default::default()
        })
    }
    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        _: RequestContext<RoleServer>,
    ) -> Result<CallToolResponse, ErrorData> {
        if !TOOLS.iter().any(|(name, _)| *name == request.name) {
            return Err(ErrorData::invalid_params("Unknown Unelos tool", None));
        }
        let sessions = self.sessions.clone();
        let result = tokio::task::spawn_blocking(move || {
            let input = Value::Object(request.arguments.unwrap_or_default());
            let meta = request
                .meta
                .map(|meta| serde_json::to_value(meta).expect("metadata is JSON"));
            sessions.dispatch(&request.name, &input, meta.as_ref())
        })
        .await;
        let result = match result {
            Ok(Ok(value)) => CallToolResult::success(vec![ContentBlock::text(value.to_string())]),
            Ok(Err(error)) => CallToolResult::error(vec![ContentBlock::text(error_envelope(&error).to_string())]),
            Err(_) => CallToolResult::error(vec![ContentBlock::text(error_envelope(&fail("mutation_uncertain", "Request interrupted; explicitly reconcile with /unelos resume; never retry start")).to_string())]),
        };
        Ok(result.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn harness_packages_map_native_agents_and_parallel_dispatch() {
        for harness in ["opencode", "claude"] {
            let value = native_package(
                json!({"workerPrompt":"work","packet":{"agent":"builder"},
                "workerPrompts":[{"agent":"reviewer","prompt":"review"}]}),
                harness,
            );
            assert_eq!(value["nativeAgent"], "unelos-builder");
            assert_eq!(value["workerPrompts"][0]["nativeAgent"], "unelos-reviewer");
            let instructions = value["harnessInstructions"].as_str().unwrap();
            if harness == "claude" {
                assert!(
                    instructions.contains("Agent(subagent_type=nativeAgent, prompt=workerPrompt)")
                );
                assert!(instructions.contains("single message"));
                assert!(instructions.contains("SendMessage"));
            } else {
                assert_eq!(instructions, "Native Unelos tools: unelos_next, unelos_start, unelos_resume, unelos_complete, unelos_approve, unelos_reject, unelos_status, unelos_inspect, unelos_rename.\nNative worker: subagent(agent=nativeAgent, prompt=workerPrompt, sessionID for continuation).\nParallel workers: in one turn call subagent(agent=workerPrompts[i].nativeAgent, prompt=workerPrompts[i].prompt, background=true) for every entry before waiting; continue one worker with its sessionID.");
            }
        }
    }
}
