use assert_cmd::cargo::cargo_bin_cmd;
use assert_fs::prelude::*;
use serde_json::{json, Value};
use std::process::Output;

fn command() -> assert_cmd::Command {
    let mut command = cargo_bin_cmd!("unelos");
    command.env_clear();
    command
}

fn settings(output: Output) -> Value {
    assert!(output.status.success(), "{output:?}");
    serde_json::from_slice(&output.stdout).unwrap()
}

#[test]
fn help_identifies_cli() {
    let output = command().arg("--help").output().unwrap();
    assert!(output.status.success());
    assert!(String::from_utf8(output.stdout).unwrap().contains("config"));
    assert!(output.stderr.is_empty());
}

#[test]
fn version_identifies_cli() {
    let output = command().arg("--version").output().unwrap();
    assert!(output.status.success());
    assert_eq!(output.stdout, b"unelos 0.1.0\n");
    assert!(output.stderr.is_empty());
}

#[test]
fn json_settings_default() {
    let output = command()
        .args(["--json", "config", "show"])
        .output()
        .unwrap();
    assert!(output.stderr.is_empty());
    insta::assert_json_snapshot!(settings(output), @r#"
    {
      "log_level": "warn"
    }
    "#);
}

#[test]
fn human_settings_default() {
    let output = command().args(["config", "show"]).output().unwrap();
    assert!(output.status.success());
    assert_eq!(output.stdout, b"log_level = \"warn\"\n");
    assert!(output.stderr.is_empty());
}

#[test]
fn settings_precedence_and_global_flags() {
    let file = assert_fs::NamedTempFile::new("settings.toml").unwrap();
    file.write_str("log_level = 'error'").unwrap();
    let file_only = command()
        .arg("--config")
        .arg(file.path())
        .args(["--json", "config", "show"])
        .output()
        .unwrap();
    assert_eq!(settings(file_only), json!({"log_level": "error"}));
    let environment = command()
        .arg("--config")
        .arg(file.path())
        .env("UNELOS_LOG_LEVEL", "info")
        .args(["--json", "config", "show"])
        .output()
        .unwrap();
    assert_eq!(settings(environment), json!({"log_level": "info"}));
    let explicit = command()
        .arg("--config")
        .arg(file.path())
        .env("UNELOS_LOG_LEVEL", "info")
        .args(["config", "show", "--json", "--log-level", "debug"])
        .output()
        .unwrap();
    assert_eq!(settings(explicit), json!({"log_level": "debug"}));
}

#[test]
fn invalid_clap_input_is_usage_error_even_in_json_mode() {
    for args in [
        vec!["--json", "unknown"],
        vec!["--json", "--log-level", "verbose", "config", "show"],
    ] {
        let output = command().args(args).output().unwrap();
        assert_eq!(output.status.code(), Some(2));
        assert!(output.stdout.is_empty());
        assert!(String::from_utf8(output.stderr).unwrap().contains("error:"));
    }
}

fn assert_settings_error(output: Output) -> Value {
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    assert!(output.stdout.is_empty());
    assert!(!output.stderr.contains(&0x1b));
    assert!(!String::from_utf8_lossy(&output.stderr).contains("do-not-leak"));
    let value: Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(value["error"]["code"], "invalid_settings");
    assert!(value["error"]["message"].as_str().unwrap().len() > 10);
    value
}

#[test]
fn missing_explicit_config_is_error() {
    let directory = assert_fs::TempDir::new().unwrap();
    let output = command()
        .arg("--config")
        .arg(directory.path().join("missing.toml"))
        .args(["--json", "config", "show"])
        .output()
        .unwrap();
    assert_settings_error(output);
}

#[test]
fn explicit_config_does_not_search_alternate_filenames() {
    let directory = assert_fs::TempDir::new().unwrap();
    directory
        .child("settings.toml")
        .write_str("log_level = 'warn'")
        .unwrap();
    let output = command()
        .arg("--config")
        .arg(directory.path().join("settings"))
        .args(["--json", "config", "show"])
        .output()
        .unwrap();
    assert_settings_error(output);
}

#[test]
fn non_utf8_config_contents_are_rejected() {
    let file = assert_fs::NamedTempFile::new("settings.toml").unwrap();
    file.write_binary(b"log_level = 'warn'\n#\xff").unwrap();
    let output = command()
        .arg("--config")
        .arg(file.path())
        .args(["--json", "config", "show"])
        .output()
        .unwrap();
    assert_settings_error(output);
}

#[test]
fn invalid_file_contents_are_not_leaked() {
    for input in [
        "log_level = 'do-not-leak",
        "token = 'do-not-leak'",
        "log_level = 'do-not-leak'",
    ] {
        let file = assert_fs::NamedTempFile::new("settings.toml").unwrap();
        file.write_str(input).unwrap();
        let output = command()
            .arg("--config")
            .arg(file.path())
            .args(["--json", "config", "show"])
            .output()
            .unwrap();
        assert_settings_error(output);
        let human = command()
            .arg("--config")
            .arg(file.path())
            .args(["config", "show"])
            .output()
            .unwrap();
        assert_eq!(human.status.code(), Some(2));
        assert!(human.stdout.is_empty());
        assert!(!String::from_utf8_lossy(&human.stderr).contains("do-not-leak"));
    }
}

#[test]
fn invalid_environment_is_settings_error() {
    let output = command()
        .env("UNELOS_LOG_LEVEL", "do-not-leak")
        .args(["--json", "config", "show"])
        .output()
        .unwrap();
    let error = assert_settings_error(output);
    insta::assert_json_snapshot!(error, @r#"
    {
      "error": {
        "code": "invalid_settings",
        "message": "log_level must be off, error, warn, info, debug, or trace"
      }
    }
    "#);
}

#[test]
fn explicit_flag_overrides_invalid_environment() {
    let output = command()
        .env("UNELOS_LOG_LEVEL", "do-not-leak")
        .args(["--json", "--log-level", "off", "config", "show"])
        .output()
        .unwrap();
    assert_eq!(settings(output), json!({"log_level": "off"}));
}

#[test]
fn human_diagnostic_has_safe_message_and_actionable_help() {
    let output = command()
        .env("UNELOS_LOG_LEVEL", "do-not-leak")
        .args(["config", "show"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    let text = String::from_utf8(output.stderr).unwrap();
    assert!(text.contains("invalid_settings"));
    assert!(text.contains("UNELOS_LOG_LEVEL"));
    assert!(!text.contains("do-not-leak"));
    // Snapshot the message independently of terminal-specific glyphs/wrapping.
    let message = text
        .lines()
        .find(|line| line.contains("log_level must"))
        .unwrap();
    insta::assert_snapshot!(message.trim().trim_start_matches(['×', 'x', ' ']), @"log_level must be off, error, warn, info, debug, or trace");
}

#[test]
fn json_operational_events_stay_on_stderr() {
    let output = command()
        .args(["--json", "--log-level", "debug", "config", "show"])
        .output()
        .unwrap();
    assert!(output.status.success());
    assert_eq!(
        serde_json::from_slice::<Value>(&output.stdout).unwrap(),
        json!({"log_level": "debug"})
    );
    assert!(!output.stderr.contains(&0x1b));
    let logs = String::from_utf8(output.stderr).unwrap();
    assert!(!logs.is_empty());
    for line in logs.lines() {
        let event: Value = serde_json::from_str(line).unwrap();
        assert_eq!(event["level"], "DEBUG");
    }
}

#[cfg(target_os = "linux")]
#[test]
fn output_io_failure_is_operational_error() {
    use std::{
        fs::OpenOptions,
        process::{Command, Stdio},
    };
    let sink = OpenOptions::new().write(true).open("/dev/full").unwrap();
    let output = Command::new(assert_cmd::cargo::cargo_bin!("unelos"))
        .env_clear()
        .args(["--json", "config", "show"])
        .stdout(Stdio::from(sink))
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    let error: Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(
        error,
        json!({"error": {"code": "operational_failure", "message": "could not write command output"}})
    );
}

#[cfg(unix)]
#[test]
fn non_utf8_path_is_usage_error() {
    use std::{ffi::OsString, os::unix::ffi::OsStringExt};
    let output = command()
        .arg("--config")
        .arg(OsString::from_vec(vec![0xff]))
        .args(["config", "show"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    assert!(!String::from_utf8_lossy(&output.stderr).contains("panicked"));
}

#[cfg(unix)]
#[test]
fn closed_stdout_pipe_is_quiet_success() {
    use std::process::{Command, Stdio};
    let mut child = Command::new(assert_cmd::cargo::cargo_bin!("unelos"))
        .env_clear()
        .args(["config", "show"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    drop(child.stdout.take());
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success(), "{output:?}");
    assert!(output.stderr.is_empty());
}
