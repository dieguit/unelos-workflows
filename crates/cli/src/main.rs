use camino::Utf8PathBuf;
use clap::{Parser, Subcommand};
use std::{
    io::{self, Read, Write},
    process::ExitCode,
};
use unelos_core::{
    approve_node, build_graph, complete_node, execution_mutate, execution_new, execution_next,
    execution_resume, execution_runs, execution_start, inspect_node, list_workflows, load_project,
    next, reject_node, rename_run, start_workflow, status, validate_project, CompletionControl,
    ContextError, ExecutionError, ExecutionOperation, ProjectLoadError, ProjectValidationError,
    RuntimeError, Settings, SettingsError,
};

mod init;
mod opencode;
mod runtime_output;

#[derive(Parser)]
#[command(name = "unelos", version, about = "Unelos command-line tool")]
struct Cli {
    /// Load a project from this directory.
    #[arg(long, global = true, default_value = ".")]
    project: Utf8PathBuf,
    /// Load settings from this TOML file (no automatic discovery).
    #[arg(long, global = true)]
    config: Option<Utf8PathBuf>,
    /// Emit command results and application errors as JSON.
    #[arg(long, global = true)]
    json: bool,
    /// Override the operational log level.
    #[arg(long, global = true, value_parser = ["off", "error", "warn", "info", "debug", "trace"])]
    log_level: Option<String>,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Initialize the project scaffold (integrations are installed globally).
    Init,
    /// Detect harnesses and manage global integrations.
    Harness {
        #[command(subcommand)]
        command: HarnessCommand,
    },
    /// Install a Workflow and optionally generate native OpenCode agents.
    Install {
        workflow: String,
        #[arg(long)]
        source: Option<Utf8PathBuf>,
        #[arg(long, value_enum)]
        opencode: Option<opencode::Version>,
    },
    /// Core-owned execution packages and identity-guarded progression.
    Execution {
        #[command(subcommand)]
        command: ExecutionCommand,
    },
    /// Inspect application settings.
    Config {
        #[command(subcommand)]
        command: ConfigCommand,
    },
    /// Inspect a normalized workflow or one resolved Node context.
    Inspect {
        workflow: String,
        /// Resolve a Task, parallel, human-gate, or end Node context.
        #[arg(long)]
        node: Option<String>,
    },
    /// List available Workflows, or start one and return its summary.
    Start {
        workflow: Option<String>,
        /// Set a descriptive run name at creation.
        #[arg(long, requires = "workflow")]
        name: Option<String>,
    },
    /// Show selected run status and history.
    Status {
        #[command(flatten)]
        target: RunSelector,
    },
    /// Read the current work packet without advancing it.
    Next {
        #[command(flatten)]
        target: RunSelector,
    },
    /// Confirm current Task/parallel outputs and advance the named Node.
    Complete {
        #[command(flatten)]
        target: RunSelector,
        node: String,
        /// Outcome for an outcome-map Node.
        #[arg(long)]
        outcome: Option<String>,
    },
    /// Approve the current human gate.
    Approve {
        node: String,
        #[command(flatten)]
        target: RunSelector,
    },
    /// Reject the current human gate with feedback.
    Reject {
        #[command(flatten)]
        target: RunSelector,
        node: String,
        #[arg(long, required = true)]
        reason: String,
    },
    /// Rename the selected run without changing its immutable run ID.
    Rename {
        name: String,
        #[arg(long = "run", required = true)]
        run_id: String,
    },
}

#[derive(Subcommand)]
enum ConfigCommand {
    /// Show the effective settings.
    Show,
}

#[derive(Subcommand)]
enum HarnessCommand {
    List,
    Install { id: String },
    Uninstall { id: String },
    Models { id: String },
}

#[derive(Subcommand)]
enum ExecutionCommand {
    New,
    Runs,
    Resume {
        #[command(flatten)]
        target: RunSelector,
    },
    Next {
        #[command(flatten)]
        target: RunSelector,
    },
    Start {
        workflow: String,
        #[arg(long)]
        name: Option<String>,
    },
    Complete {
        #[command(flatten)]
        target: RunSelector,
    },
    Approve {
        #[command(flatten)]
        target: RunSelector,
    },
    Reject {
        #[command(flatten)]
        target: RunSelector,
    },
}

#[derive(clap::Args)]
struct RunSelector {
    #[arg(long = "run")]
    run_id: String,
}

#[tokio::main]
async fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(&cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(Error::Output(error)) if error.kind() == io::ErrorKind::BrokenPipe => ExitCode::SUCCESS,
        Err(error) => {
            let rendered = render_error(&error, cli.json);
            match writeln!(io::stderr().lock(), "{rendered}") {
                Ok(()) => error.exit_code(),
                Err(error) if error.kind() == io::ErrorKind::BrokenPipe => ExitCode::SUCCESS,
                Err(_) => ExitCode::FAILURE,
            }
        }
    }
}

#[derive(Debug, thiserror::Error)]
enum Error {
    #[error(transparent)]
    Install(#[from] unelos_core::InstallError),
    #[error(transparent)]
    Execution(#[from] ExecutionError),
    #[error("{0}")]
    Settings(#[from] SettingsError),
    #[error("{0}")]
    ProjectLoad(#[from] ProjectLoadError),
    #[error("{0}")]
    ProjectValidation(#[from] ProjectValidationError),
    #[error("{0}")]
    Context(#[from] ContextError),
    #[error(transparent)]
    Runtime(#[from] RuntimeError),
    #[error("could not render runtime result: {0}")]
    RuntimeOutput(#[source] io::Error),
    #[error("could not write command output")]
    Output(#[from] io::Error),
    #[error("could not serialize command output")]
    Serialize(#[from] serde_json::Error),
    #[error("could not initialize operational logging")]
    Logging(#[source] Box<dyn std::error::Error + Send + Sync>),
}

impl Error {
    fn code(&self) -> &'static str {
        match self {
            Self::Install(error) => error.code,
            Self::Execution(error) => error.code(),
            Self::Settings(_) => "invalid_settings",
            Self::ProjectLoad(_) => "project_load_failed",
            Self::ProjectValidation(_) => "invalid_project",
            Self::Context(_) => "context_resolution_failed",
            Self::Runtime(error) => match error {
                RuntimeError::WrongNode { .. } => "wrong_node",
                RuntimeError::UnknownRun { .. } => "unknown_run",
                RuntimeError::InvalidName { .. } => "invalid_run_name",
                RuntimeError::DuplicateRunId { .. } => "duplicate_run_id",
                RuntimeError::Collision { .. } => "run_name_collision",
                RuntimeError::StaleRun { .. } => "wrong_run",
                RuntimeError::InvalidState(_) => "invalid_runtime_state",
                RuntimeError::Io { .. } => "runtime_io_failed",
                RuntimeError::Json { .. } => "invalid_runtime_json",
                RuntimeError::Project(_) => "project_load_failed",
                RuntimeError::Validation(_) => "invalid_project",
                RuntimeError::Context(_) => "context_resolution_failed",
                RuntimeError::StaleExecution { .. } => "wrong_run",
            },
            Self::RuntimeOutput(_) => "runtime_io_failed",
            _ => "operational_failure",
        }
    }

    fn exit_code(&self) -> ExitCode {
        match self {
            Self::Settings(_)
            | Self::ProjectLoad(_)
            | Self::ProjectValidation(_)
            | Self::Context(_) => ExitCode::from(2),
            Self::Runtime(RuntimeError::Io { .. } | RuntimeError::Json { .. }) => ExitCode::FAILURE,
            Self::Runtime(_) => ExitCode::from(2),
            _ => ExitCode::FAILURE,
        }
    }
}

fn run(cli: &Cli) -> Result<(), Error> {
    let settings = Settings::load(cli.config.as_deref(), cli.log_level.as_deref())?;
    let subscriber = tracing_subscriber::fmt()
        .with_env_filter(settings.log_level.as_str())
        .with_writer(io::stderr)
        .with_ansi(false);
    if cli.json {
        subscriber.json().try_init().map_err(Error::Logging)?;
    } else {
        subscriber.try_init().map_err(Error::Logging)?;
    }
    match &cli.command {
        Command::Harness { command } => {
            let value = match command {
                HarnessCommand::List => serde_json::to_value(unelos_core::list_harnesses())?,
                HarnessCommand::Install { id } => {
                    let src =
                        Utf8PathBuf::from_path_buf(std::env::current_exe()?).map_err(|_| {
                            unelos_core::InstallError {
                                code: "invalid_path",
                                message: "CLI path must be UTF-8".into(),
                            }
                        })?;
                    serde_json::to_value(unelos_core::install_harness(id, &src)?)?
                }
                HarnessCommand::Uninstall { id } => {
                    serde_json::to_value(unelos_core::uninstall_harness(id)?)?
                }
                HarnessCommand::Models { id } => {
                    serde_json::to_value(unelos_core::harness_models(id)?)?
                }
            };
            emit(&value, cli.json, serde_json::to_string_pretty(&value)?)?;
        }
        Command::Init => {
            let value = init::initialize(&cli.project)?;
            emit(&value, cli.json, value["message"].as_str().unwrap().into())?;
        }
        Command::Install {
            workflow,
            source,
            opencode,
        } => {
            let selected = init::install_version(&cli.project, *opencode)?;
            if selected.is_some() {
                let agents = unelos_core::export_workflow_agents(
                    source.as_deref().unwrap_or(&cli.project),
                    workflow,
                )?;
                opencode::check_agents(&cli.project, &agents)?;
                unelos_core::check_opencode_mcps(&cli.project, workflow, &agents)?;
            }
            let installed =
                unelos_core::install_workflow(&cli.project, source.as_deref(), workflow)?;
            let native_agents = match selected {
                Some(version) => {
                    unelos_core::write_opencode_mcps(&cli.project, workflow, &installed.agents)?;
                    opencode::write_agents(&cli.project, version, &installed.agents)?
                }
                None => Vec::new(),
            };
            let value = serde_json::json!({"workflow":installed.workflow,"agents":installed.agents,"nativeAgents":native_agents});
            emit(
                &value,
                cli.json,
                format!("Installed workflow {}", installed.workflow),
            )?;
        }
        Command::Execution { command } => {
            if matches!(command, ExecutionCommand::Runs) {
                let value = serde_json::to_value(execution_runs(&cli.project)?)?;
                return emit(&value, cli.json, runtime_output::readable_packet(&value));
            }
            let mutating = matches!(
                command,
                ExecutionCommand::Start { .. }
                    | ExecutionCommand::Complete { .. }
                    | ExecutionCommand::Approve { .. }
                    | ExecutionCommand::Reject { .. }
            );
            let package = match command {
                ExecutionCommand::New => execution_new(&cli.project)?,
                ExecutionCommand::Resume { target } => {
                    execution_resume(&cli.project, &target.run_id)?
                }
                ExecutionCommand::Next { target } => execution_next(&cli.project, &target.run_id)?,
                ExecutionCommand::Start { workflow, name } => {
                    execution_start(&cli.project, workflow, name.as_deref())?
                }
                operation => {
                    let (operation, target) = match operation {
                        ExecutionCommand::Complete { target } => {
                            (ExecutionOperation::Complete, target)
                        }
                        ExecutionCommand::Approve { target } => {
                            (ExecutionOperation::Approve, target)
                        }
                        ExecutionCommand::Reject { target } => (ExecutionOperation::Reject, target),
                        _ => unreachable!(),
                    };
                    execution_mutate(
                        &cli.project,
                        &target.run_id,
                        operation,
                        &read_control::<CompletionControl>()?,
                    )?
                }
            };
            let value = serde_json::to_value(&package).map_err(|error| {
                if mutating {
                    mutation_response_error(package.run_id.as_deref())
                } else {
                    Error::Serialize(error)
                }
            })?;
            emit(&value, cli.json, runtime_output::readable(&value)).map_err(|error| {
                if mutating {
                    mutation_response_error(package.run_id.as_deref())
                } else {
                    error
                }
            })?;
        }
        Command::Config {
            command: ConfigCommand::Show,
        } => {
            tracing::debug!("showing effective settings");
            let output = if cli.json {
                serde_json::to_string(&settings)?
            } else {
                format!("log_level = {:?}", settings.log_level)
            };
            writeln!(io::stdout().lock(), "{output}")?;
        }
        Command::Inspect { workflow, node } => {
            let project = load_project(&cli.project, workflow)?;
            validate_project(&project)?;
            let result = if let Some(node) = node {
                serde_json::to_value(inspect_node(&project, node)?)?
            } else {
                let graph = build_graph(&project.workflow);
                let nodes = project
                    .workflow
                    .nodes
                    .iter()
                    .map(|(id, node)| {
                        let mut value = serde_json::to_value(node)?;
                        value
                            .as_object_mut()
                            .expect("Node serializes as an object")
                            .insert("id".into(), serde_json::json!(id));
                        Ok(value)
                    })
                    .collect::<Result<Vec<_>, serde_json::Error>>()?;
                serde_json::json!({
                    "name": project.workflow.name,
                    "start": graph.start,
                    "nodes": nodes,
                    "edges": graph.edges,
                })
            };
            let output = if cli.json {
                serde_json::to_string(&result)?
            } else {
                serde_json::to_string_pretty(&result)?
            };
            writeln!(io::stdout().lock(), "{output}")?;
        }
        Command::Start { workflow, name } => {
            let value = match workflow {
                None => serde_json::json!({"workflows": list_workflows(&cli.project)?}),
                Some(workflow) => runtime_output::summary(
                    &cli.project,
                    &start_workflow(&cli.project, workflow, name.as_deref())?,
                )
                .map_err(Error::RuntimeOutput)?,
            };
            emit(&value, cli.json, runtime_output::readable(&value))?;
        }
        Command::Status { target } => {
            let state = status(&cli.project, &target.run_id)?;
            let value =
                runtime_output::status(&cli.project, &state).map_err(Error::RuntimeOutput)?;
            emit(&value, cli.json, runtime_output::readable(&value))?;
        }
        Command::Next { target } => {
            let packet = next(&cli.project, &target.run_id)?;
            let value = serde_json::to_value(packet)?;
            emit(&value, cli.json, runtime_output::readable_packet(&value))?;
        }
        Command::Complete {
            node,
            outcome,
            target,
        } => {
            let state = complete_node(&cli.project, &target.run_id, node, outcome.as_deref())?;
            let value =
                runtime_output::summary(&cli.project, &state).map_err(Error::RuntimeOutput)?;
            emit(&value, cli.json, runtime_output::readable(&value))?;
        }
        Command::Approve { node, target } => {
            let state = approve_node(&cli.project, &target.run_id, node)?;
            let value =
                runtime_output::summary(&cli.project, &state).map_err(Error::RuntimeOutput)?;
            emit(&value, cli.json, runtime_output::readable(&value))?;
        }
        Command::Reject {
            node,
            reason,
            target,
        } => {
            let state = reject_node(&cli.project, &target.run_id, node, reason)?;
            let value =
                runtime_output::summary(&cli.project, &state).map_err(Error::RuntimeOutput)?;
            emit(&value, cli.json, runtime_output::readable(&value))?;
        }
        Command::Rename { name, run_id } => {
            let state = rename_run(&cli.project, run_id, name)?;
            let value =
                runtime_output::summary(&cli.project, &state).map_err(Error::RuntimeOutput)?;
            emit(&value, cli.json, runtime_output::readable(&value))?;
        }
    }
    Ok(())
}

fn emit(value: &serde_json::Value, json: bool, readable: String) -> Result<(), Error> {
    let output = if json {
        serde_json::to_string(value)?
    } else {
        readable
    };
    writeln!(io::stdout().lock(), "{output}")?;
    Ok(())
}

fn mutation_response_error(run_id: Option<&str>) -> Error {
    ExecutionError::Protocol {
        code: "mutation_uncertain",
        message: format!("State committed but response unavailable; reconcile with execution next --run {}; never blindly retry", run_id.unwrap_or("RUN_ID")),
    }
    .into()
}

fn read_control<T: serde::de::DeserializeOwned>() -> Result<T, Error> {
    let invalid = || {
        Error::Execution(ExecutionError::Protocol {
            code: "invalid_control",
            message: "Provide one valid execution control JSON object (maximum 1 MiB)".into(),
        })
    };
    let mut bytes = Vec::new();
    io::stdin()
        .take(1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| invalid())?;
    if bytes.len() > 1024 * 1024 {
        return Err(invalid());
    }
    serde_json::from_slice(&bytes).map_err(|_| invalid())
}

fn render_error(error: &Error, json: bool) -> String {
    if json {
        serde_json::json!({
            "error": { "code": error.code(), "message": error.to_string() }
        })
        .to_string()
    } else {
        // Render a sanitized diagnostic: config's source chain can contain secrets.
        let help = match error {
            Error::Settings(_) => {
                "Check --config, UNELOS_LOG_LEVEL, and --log-level. Use --help for usage."
            }
            Error::ProjectLoad(_) => "Check --project and the reported authoring source.",
            Error::ProjectValidation(_) => {
                "Check --project and the reported authoring source and field."
            }
            Error::Context(_) => {
                "Check the selected Node, Task Definition, Agent, and selected Skill in the reported authoring source."
            }
            _ => "Check the output destination and logging configuration.",
        };
        let diagnostic = miette::MietteDiagnostic::new(error.to_string())
            .with_code(error.code())
            .with_help(help);
        format!("{:?}", miette::Report::new(diagnostic))
    }
}
