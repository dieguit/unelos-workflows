use camino::{Utf8Path, Utf8PathBuf};
use serde_json::{json, Value};
use std::io;
use unelos_core::{CurrentWork, RunState};

pub(crate) fn summary(root: &Utf8Path, state: &RunState) -> Result<Value, io::Error> {
    let root = if root.is_absolute() {
        root.to_owned()
    } else {
        let cwd = Utf8PathBuf::from_path_buf(std::env::current_dir()?).map_err(|_| {
            io::Error::new(io::ErrorKind::InvalidData, "working directory is not UTF-8")
        })?;
        cwd.join(root)
    };
    let (status, kind, node) = match &state.current {
        CurrentWork::Task(record) => ("running", "task", record.node_id.as_str()),
        CurrentWork::Parallel(record) => ("running", "parallel", record.node_id.as_str()),
        CurrentWork::HumanGate(record) => ("waiting", "human-gate", record.node_id.as_str()),
        CurrentWork::Finished { node_id } => ("finished", "finished", node_id.as_str()),
    };
    Ok(json!({
        "runId": state.run_id, "name": state.name, "namingPending": state.naming_pending,
        "workflowName": state.workflow_name,
        "runDirectory": root.join(".unelos/runs").join(&state.name),
        "status": status, "current": { "kind": kind, "nodeId": node },
    }))
}

pub(crate) fn status(root: &Utf8Path, state: &RunState) -> Result<Value, io::Error> {
    let mut value = summary(root, state)?;
    value["history"] = serde_json::to_value(&state.history)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    Ok(value)
}

pub(crate) fn readable(value: &Value) -> String {
    if value.get("entryMode").is_some() {
        return readable_packet(value);
    }
    if let Some(workflows) = value["workflows"].as_array() {
        return if workflows.is_empty() {
            "No Workflows found.".into()
        } else {
            format!(
                "Available Workflows:\n{}",
                workflows
                    .iter()
                    .filter_map(Value::as_str)
                    .collect::<Vec<_>>()
                    .join("\n")
            )
        };
    }
    if value["status"] == "idle" {
        return "No selected run.".into();
    }
    let mut text = format!(
        "{}: {} ({})\nWorkflow: {}\nDirectory: {}\nCurrent: {} ({})",
        value["status"].as_str().unwrap_or("run"),
        value["name"].as_str().unwrap_or_default(),
        value["runId"].as_str().unwrap_or_default(),
        value["workflowName"].as_str().unwrap_or_default(),
        value["runDirectory"].as_str().unwrap_or_default(),
        value["current"]["nodeId"].as_str().unwrap_or_default(),
        value["current"]["kind"].as_str().unwrap_or_default(),
    );
    if let Some(history) = value["history"].as_array() {
        text.push_str("\nHistory:");
        for entry in history {
            let work = &entry["work"];
            text.push_str(&format!(
                "\n- {} ({})",
                work["nodeId"].as_str().unwrap_or_default(),
                work["kind"].as_str().unwrap_or_default(),
            ));
            if let Some(outcome) = entry["outcome"].as_str() {
                text.push_str(&format!(" — {outcome}"));
            }
            if let Some(reason) = entry["reason"].as_str() {
                text.push_str(&format!(" — {reason}"));
            }
        }
    }
    text
}

pub(crate) fn readable_packet(value: &Value) -> String {
    serde_json::to_string_pretty(value).expect("JSON value always serializes")
}
