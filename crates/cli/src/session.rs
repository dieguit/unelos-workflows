use crate::{Cli, Error};
use clap::Subcommand;
use serde_json::{json, Value};
use std::io::{self, Read, Write};
use unelos_core::session_binding::{SessionContext, SessionService};

#[derive(Subcommand)]
pub(crate) enum SessionCommand {
    Activate {
        #[arg(long, value_parser = ["claude", "opencode"])]
        harness: String,
        #[arg(long)]
        session: String,
        #[arg(long, value_parser = ["new", "resume", "off"], conflicts_with = "request")]
        mode: Option<String>,
        #[arg(long, conflicts_with = "mode")]
        request: Option<String>,
    },
}

#[derive(Subcommand)]
pub(crate) enum HookCommand {
    Claude {
        #[command(subcommand)]
        command: ClaudeHook,
    },
}

#[derive(Subcommand)]
pub(crate) enum ClaudeHook {
    PreToolUse,
    SessionStart,
}

pub(crate) fn activate(cli: &Cli, command: &SessionCommand) -> Result<(), Error> {
    let SessionCommand::Activate {
        harness,
        session,
        mode,
        request,
    } = command;
    let project = if cli.project.is_absolute() {
        cli.project.clone()
    } else {
        camino::Utf8PathBuf::from_path_buf(std::env::current_dir()?)
            .map_err(|_| invalid("Project must be UTF-8"))?
            .join(&cli.project)
    };
    let context = SessionContext {
        harness: harness.clone(),
        session_id: session.clone(),
        project,
        role: "controller".into(),
    };
    let service = SessionService::discover()?;
    let value = match request {
        Some(text) => service.activate_request(&context, text)?,
        None => service.activate(&context, mode.as_deref().unwrap_or("new"))?,
    };
    crate::emit(&value, cli.json, serde_json::to_string_pretty(&value)?)
}

fn invalid(message: &str) -> Error {
    unelos_core::ExecutionError::Protocol {
        code: "context_missing",
        message: message.into(),
    }
    .into()
}

fn hook_context(input: &Value) -> Result<SessionContext, Error> {
    let project = std::env::var("CLAUDE_PROJECT_DIR")
        .ok()
        .filter(|s| !s.is_empty())
        .or_else(|| input["cwd"].as_str().map(str::to_owned))
        .ok_or_else(|| invalid("Claude project unavailable"))?;
    Ok(SessionContext {
        harness: "claude".into(),
        session_id: input["session_id"]
            .as_str()
            .ok_or_else(|| invalid("Claude session unavailable"))?
            .into(),
        project: project.into(),
        role: if input.get("agent_id").is_some() {
            "worker"
        } else {
            "controller"
        }
        .into(),
    })
}

pub(crate) fn hook(command: &HookCommand) -> Result<(), Error> {
    let mut bytes = Vec::new();
    io::stdin().take(1024 * 1024 + 1).read_to_end(&mut bytes)?;
    if bytes.len() > 1024 * 1024 {
        return Err(invalid("Hook input exceeds 1 MiB"));
    }
    let input: Value = serde_json::from_slice(&bytes)?;
    let HookCommand::Claude { command } = command;
    let value = match command {
        ClaudeHook::PreToolUse => {
            if !input["tool_name"]
                .as_str()
                .is_some_and(|s| s.starts_with("mcp__plugin_unelos_unelos__"))
            {
                return Ok(());
            }
            let context = hook_context(&input)?;
            let mut updated = input["tool_input"]
                .as_object()
                .cloned()
                .ok_or_else(|| invalid("Claude tool_input must be an object"))?;
            updated.insert("unelos_context".into(), serde_json::to_value(context)?);
            json!({"hookSpecificOutput":{"hookEventName":"PreToolUse","permissionDecision":"allow","updatedInput":updated}})
        }
        ClaudeHook::SessionStart => {
            if !matches!(input["source"].as_str(), Some("compact" | "resume"))
                || input.get("agent_id").is_some()
            {
                return Ok(());
            }
            let context = hook_context(&input)?;
            let service = SessionService::discover()?;
            let Some(binding) = service.binding(&context)?.filter(|b| b.enabled) else {
                return Ok(());
            };
            let project = std::fs::canonicalize(&context.project)?;
            if project != binding.project.as_std_path() {
                return Ok(());
            }
            let recovery = if binding.pending_start.is_some() {
                "A pendingStart is unresolved. Explicitly reconcile via /unelos-resume; never retry start."
            } else {
                "Call mcp__plugin_unelos_unelos__next to recover."
            };
            json!({"hookSpecificOutput":{"hookEventName":"SessionStart","additionalContext":format!(
                "Unelos controller enabled. Bound run: {}. {}",
                binding.run_id.as_deref().unwrap_or("(none; explicitly start or resume a run)"),
                recovery
            )}})
        }
    };
    writeln!(io::stdout().lock(), "{value}")?;
    Ok(())
}
