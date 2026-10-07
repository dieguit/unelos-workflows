use assert_cmd::cargo::cargo_bin_cmd;
use assert_fs::{fixture::PathCopy, TempDir};
use serde_json::Value;
use std::fs;

fn project_copy() -> TempDir {
    let temp = TempDir::new().unwrap();
    temp.copy_from(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../core/tests/fixtures/project"),
        &["**/*"],
    )
    .unwrap();
    temp
}

fn command(root: &std::path::Path) -> assert_cmd::Command {
    let mut command = cargo_bin_cmd!("unelos");
    command.env_clear().arg("--project").arg(root).arg("--json");
    command
}

fn readable_command(root: &std::path::Path) -> assert_cmd::Command {
    let mut command = cargo_bin_cmd!("unelos");
    command.env_clear().arg("--project").arg(root);
    command
}

fn result(root: &std::path::Path, args: &[&str]) -> Value {
    let output = command(root).args(args).output().unwrap();
    assert!(output.status.success(), "{output:?}");
    assert!(output.stderr.is_empty(), "{output:?}");
    serde_json::from_slice(&output.stdout).unwrap()
}

#[test]
fn selection_start_status_next_and_node_completion() {
    let temp = project_copy();
    let root = temp.path();
    assert_eq!(result(root, &["status"])["status"], "idle");
    assert!(result(root, &["start"])["workflows"]
        .as_array()
        .unwrap()
        .contains(&Value::String("implement-feature".into())));
    assert!(!root.join(".unelos/active-run").exists());
    let summary = result(root, &["start", "implement-feature"]);
    assert_eq!(summary["namingPending"], true);
    assert_eq!(summary["current"]["nodeId"], "plan");
    let packet = result(root, &["next"]);
    assert_eq!(packet, result(root, &["next"]));
    let path = packet["outputs"][0]["path"].as_str().unwrap();
    fs::write(path, "plan").unwrap();
    assert_eq!(result(root, &["complete", "plan"])["status"], "waiting");
    assert_eq!(
        result(root, &["status"])["history"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn selection_validates_every_discovered_definition() {
    let temp = project_copy();
    let workflows = temp.path().join(".unelos/workflows");
    fs::write(
        workflows.join("broken.yaml"),
        "name: broken\nstart: task\nnodes:\n  task:\n    task: missing-definition\n    agent: planner\n    outputs: [out.md]\n    next: done\n  done:\n    type: end\n",
    )
    .unwrap();
    let output = command(temp.path()).arg("start").output().unwrap();
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    assert!(output.stdout.is_empty());
    let error: Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(error["error"]["code"], "project_load_failed");
    assert!(error["error"]["message"]
        .as_str()
        .unwrap()
        .contains("missing-definition"));
    assert!(!temp.path().join(".unelos/active-run").exists());
}

#[test]
fn workflow_selection_is_sorted_and_empty_selection_succeeds() {
    let temp = project_copy();
    let root = temp.path();
    fs::write(
        root.join(".unelos/workflows/another.yaml"),
        "name: another\nstart: done\nnodes:\n  done:\n    type: end\n",
    )
    .unwrap();
    assert_eq!(
        result(root, &["start"])["workflows"],
        serde_json::json!(["another", "implement-feature"])
    );
    fs::remove_file(root.join(".unelos/workflows/another.yaml")).unwrap();
    fs::remove_file(root.join(".unelos/workflows/implement-feature.yaml")).unwrap();
    assert_eq!(result(root, &["start"])["workflows"], serde_json::json!([]));
    assert!(!root.join(".unelos/active-run").exists());
}

#[test]
fn explicit_name_rename_and_mutations_preserve_identity() {
    let temp = project_copy();
    let root = temp.path();
    let started = result(
        root,
        &["start", "implement-feature", "--name", "fix-button"],
    );
    assert_eq!(started["name"], "fix-button");
    assert_eq!(started["namingPending"], false);
    let run_id = started["runId"].as_str().unwrap();
    let old_path = started["runDirectory"].as_str().unwrap().to_owned();
    let renamed = result(root, &["rename", "fix-save-button", "--run", run_id]);
    assert_eq!(renamed["runId"], run_id);
    assert_eq!(renamed["name"], "fix-save-button");
    assert!(!std::path::Path::new(&old_path).exists());

    let packet = result(root, &["next"]);
    assert_eq!(packet["kind"], "task");
    let instructions = packet["controllerInstructions"].as_str().unwrap();
    assert!(instructions.contains("Carry relevant conversation context"));
    assert!(instructions.contains("complete NODE"));
    assert!(!instructions.contains("--execution"));
    let state_path =
        std::path::Path::new(renamed["runDirectory"].as_str().unwrap()).join("state.json");
    let before = fs::read(&state_path).unwrap();
    for args in [vec!["complete", "wrong"], vec!["complete", "plan"]] {
        let output = command(root).args(args).output().unwrap();
        assert_eq!(output.status.code(), Some(2), "{output:?}");
        assert!(output.stdout.is_empty());
        let error: Value = serde_json::from_slice(&output.stderr).unwrap();
        assert!(matches!(
            error["error"]["code"].as_str(),
            Some("wrong_node" | "invalid_runtime_state")
        ));
        assert_eq!(fs::read(&state_path).unwrap(), before);
    }
    fs::write(packet["outputs"][0]["path"].as_str().unwrap(), "Plan").unwrap();
    assert_eq!(result(root, &["complete", "plan"])["status"], "waiting");
    let gate = result(root, &["next"]);
    assert_eq!(gate["kind"], "human-gate");
    assert!(gate["controllerInstructions"]
        .as_str()
        .unwrap()
        .contains("obtain an explicit user decision"));
    assert_eq!(
        result(
            root,
            &["reject", "approve-plan", "--reason", "Revise scope."]
        )["current"]["nodeId"],
        "plan"
    );
    let state = result(root, &["status"]);
    assert_eq!(state["history"].as_array().unwrap().len(), 2);
    assert_eq!(state["history"][1]["reason"], "Revise scope.");
    let feedback = result(root, &["next"])["feedback"].clone();
    assert_eq!(feedback["reason"], "Revise scope.");
    assert!(feedback["artifacts"][0]["path"]
        .as_str()
        .unwrap()
        .ends_with("/plan/plan.md"));
}

#[test]
fn cli_usage_errors_and_idle_next_are_distinct() {
    let temp = project_copy();
    let root = temp.path();
    let no_next = command(root).arg("next").output().unwrap();
    assert_eq!(no_next.status.code(), Some(2));
    assert!(no_next.stdout.is_empty());
    assert_eq!(
        serde_json::from_slice::<Value>(&no_next.stderr).unwrap()["error"]["code"],
        "no_active_run"
    );

    let missing_workflow = command(root)
        .args(["start", "--name", "named"])
        .output()
        .unwrap();
    assert_eq!(missing_workflow.status.code(), Some(2));
    assert!(std::str::from_utf8(&missing_workflow.stderr)
        .unwrap()
        .starts_with("error:"));
    assert!(serde_json::from_slice::<Value>(&missing_workflow.stderr).is_err());

    let help = command(root).arg("--help").output().unwrap();
    let help = String::from_utf8(help.stdout).unwrap();
    for command in [
        "start", "status", "next", "complete", "approve", "reject", "rename",
    ] {
        assert!(help.contains(command), "missing {command} in help");
    }
    for forbidden in ["request-file", "--execution", "complete --reason"] {
        assert!(!help.contains(forbidden), "unexpected {forbidden} in help");
    }

    result(root, &["start", "implement-feature"]);
    let packet = result(root, &["next"]);
    fs::write(packet["outputs"][0]["path"].as_str().unwrap(), "Plan").unwrap();
    result(root, &["complete", "plan"]);
    let output = readable_command(root).arg("status").output().unwrap();
    assert!(output.status.success(), "{output:?}");
    let readable = String::from_utf8(output.stdout).unwrap();
    assert!(readable.contains("History:"));
    assert!(readable.contains("plan (task)"));
    let packet = readable_command(root).arg("next").output().unwrap();
    assert!(packet.status.success(), "{packet:?}");
    let packet: Value = serde_json::from_slice(&packet.stdout).unwrap();
    assert_eq!(packet["kind"], "human-gate");
}

#[test]
fn rejection_correction_and_finished_state_survive_process_restarts() {
    let temp = project_copy();
    let root = temp.path();
    let started = result(root, &["start", "implement-feature"]);
    let run_id = started["runId"].as_str().unwrap().to_owned();
    let state_path =
        std::path::Path::new(started["runDirectory"].as_str().unwrap()).join("state.json");
    let unchanged = fs::read(&state_path).unwrap();
    let missing_output = command(root).args(["complete", "plan"]).output().unwrap();
    assert_eq!(missing_output.status.code(), Some(2));
    assert!(missing_output.stdout.is_empty());
    assert_eq!(fs::read(&state_path).unwrap(), unchanged);

    fn write_outputs(packet: &Value, text: &str) {
        for output in packet["outputs"].as_array().unwrap() {
            fs::write(output["path"].as_str().unwrap(), text).unwrap();
        }
    }

    let packet = result(root, &["next"]);
    write_outputs(&packet, "Plan v1");
    result(root, &["complete", "plan"]);
    let gate = result(root, &["next"]);
    assert_eq!(gate["kind"], "human-gate");
    let gate_state = fs::read(&state_path).unwrap();
    let gate_complete = command(root)
        .args(["complete", "approve-plan"])
        .output()
        .unwrap();
    assert_eq!(gate_complete.status.code(), Some(2));
    assert_eq!(fs::read(&state_path).unwrap(), gate_state);
    result(
        root,
        &[
            "reject",
            "approve-plan",
            "--reason",
            "Explain the rollback.",
        ],
    );
    let revised = result(root, &["next"]);
    assert_eq!(revised["feedback"]["reason"], "Explain the rollback.");
    assert!(revised["feedback"]["artifacts"][0]["path"]
        .as_str()
        .unwrap()
        .ends_with("/plan/plan.md"));
    write_outputs(&revised, "Plan v2 with rollback.");
    result(root, &["complete", "plan"]);
    result(root, &["approve", "approve-plan"]);

    let implementation = result(root, &["next"]);
    write_outputs(&implementation, "Implementation v1");
    result(root, &["complete", "implement"]);
    let review = result(root, &["next"]);
    write_outputs(&review, "Review asks for a correction");
    let review_state = fs::read(&state_path).unwrap();
    let invalid = command(root)
        .args(["complete", "review", "--outcome", "unknown"])
        .output()
        .unwrap();
    assert_eq!(invalid.status.code(), Some(2));
    assert_eq!(fs::read(&state_path).unwrap(), review_state);
    result(
        root,
        &["complete", "review", "--outcome", "changes_requested"],
    );
    let correction = result(root, &["next"]);
    assert_eq!(correction["nodeId"], "implement");
    write_outputs(&correction, "Implementation v2");
    result(root, &["complete", "implement"]);
    let final_review = result(root, &["next"]);
    write_outputs(&final_review, "Review approved");
    result(root, &["complete", "review", "--outcome", "approved"]);

    let finished = result(root, &["status"]);
    assert_eq!(finished["status"], "finished");
    assert_eq!(finished["runId"], run_id);
    assert_eq!(finished["history"].as_array().unwrap().len(), 8);
    assert_eq!(result(root, &["next"])["kind"], "finished");
}

#[test]
fn rejected_mutations_emit_structured_errors_without_changing_state() {
    let temp = project_copy();
    let root = temp.path();
    let started = result(root, &["start", "implement-feature"]);
    let state_path =
        std::path::Path::new(started["runDirectory"].as_str().unwrap()).join("state.json");
    fs::create_dir(root.join(".unelos/runs/collision")).unwrap();
    let before = fs::read(&state_path).unwrap();
    for (args, code) in [
        (
            vec![
                "rename",
                "../unsafe",
                "--run",
                started["runId"].as_str().unwrap(),
            ],
            "invalid_run_name",
        ),
        (
            vec!["rename", "good-name", "--run", "wrong-run"],
            "wrong_run",
        ),
        (
            vec![
                "rename",
                "collision",
                "--run",
                started["runId"].as_str().unwrap(),
            ],
            "run_name_collision",
        ),
        (vec!["approve", "plan"], "invalid_runtime_state"),
        (
            vec!["reject", "plan", "--reason", " "],
            "invalid_runtime_state",
        ),
        (
            vec!["complete", "review", "--outcome", "approved"],
            "wrong_node",
        ),
    ] {
        let output = command(root).args(&args).output().unwrap();
        assert_eq!(output.status.code(), Some(2), "{output:?}");
        assert!(output.stdout.is_empty(), "{output:?}");
        let error: Value = serde_json::from_slice(&output.stderr).unwrap();
        assert_eq!(error["error"]["code"], code, "{output:?}");
        assert_eq!(fs::read(&state_path).unwrap(), before);
    }
    for args in [
        vec!["complete", "plan", "--execution", "task-1"],
        vec!["complete", "plan", "--reason", "done"],
    ] {
        let output = command(root).args(args).output().unwrap();
        assert_eq!(output.status.code(), Some(2));
        assert!(std::str::from_utf8(&output.stderr)
            .unwrap()
            .starts_with("error:"));
    }
}

#[test]
fn simultaneous_node_completion_only_advances_once() {
    let temp = project_copy();
    let root = temp.path();
    let started = result(root, &["start", "implement-feature"]);
    let packet = result(root, &["next"]);
    fs::write(packet["outputs"][0]["path"].as_str().unwrap(), "Plan").unwrap();
    let children = (0..2)
        .map(|_| {
            std::process::Command::new(assert_cmd::cargo::cargo_bin!("unelos"))
                .env_clear()
                .args(["--project"])
                .arg(root)
                .args(["--json", "complete", "plan"])
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn()
                .unwrap()
        })
        .collect::<Vec<_>>();
    let outputs = children
        .into_iter()
        .map(|child| child.wait_with_output().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(
        outputs
            .iter()
            .filter(|output| output.status.success())
            .count(),
        1
    );
    assert_eq!(
        outputs
            .iter()
            .filter(|output| output.status.code() == Some(2))
            .count(),
        1
    );
    let status = result(root, &["status"]);
    assert_eq!(status["history"].as_array().unwrap().len(), 1);
    assert_eq!(status["current"]["nodeId"], "approve-plan");
    assert_eq!(status["runId"], started["runId"]);
}
