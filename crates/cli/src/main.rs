use camino::Utf8PathBuf;
use clap::{Parser, Subcommand};
use std::{
    io::{self, Write},
    process::ExitCode,
};
use unelos_core::{Settings, SettingsError};

#[derive(Parser)]
#[command(name = "unelos", version, about = "Unelos command-line tool")]
struct Cli {
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
    /// Inspect application settings.
    Config {
        #[command(subcommand)]
        command: ConfigCommand,
    },
}

#[derive(Subcommand)]
enum ConfigCommand {
    /// Show the effective settings.
    Show,
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
    #[error("{0}")]
    Settings(#[from] SettingsError),
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
            Self::Settings(_) => "invalid_settings",
            _ => "operational_failure",
        }
    }

    fn exit_code(&self) -> ExitCode {
        match self {
            Self::Settings(_) => ExitCode::from(2),
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
    match cli.command {
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
    }
    Ok(())
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
            _ => "Check the output destination and logging configuration.",
        };
        let diagnostic = miette::MietteDiagnostic::new(error.to_string())
            .with_code(error.code())
            .with_help(help);
        format!("{:?}", miette::Report::new(diagnostic))
    }
}
