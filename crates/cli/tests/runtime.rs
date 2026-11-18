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
fn start_lists_and_starts_global_workflows() {
    let temp = project_copy();
    let global = TempDir::new().unwrap();
    fs::create_dir_all(global.path().join("workflows")).unwrap();
    fs::create_dir_all(global.path().join("agents").join("shared")).unwrap();
    fs::write(
        global
            .path()
            .join("agents")
            .join("shared")
            .join("agent.yaml"),
        "instructions: shared\n",
    )
    .unwrap();
    fs::write(global.path().join("workflows").join("shared.yaml"), "name: shared\nstart: work\nnodes:\n  work:\n    agent: global/shared\n    taskDefinition:\n      instructions: global task\n    next: end\n  end:\n    type: end\n").unwrap();
    let output = command(temp.path())
        .env("UNELOS_CONFIG_DIR", global.path())
        .arg("start")
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    let list: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(list["workflows"]
        .as_array()
        .unwrap()
        .contains(&Value::String("global/shared".into())));
    let output = command(temp.path())
        .env("UNELOS_CONFIG_DIR", global.path())
        .args(["start", "global/shared"])
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    let run: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(run["workflowName"], "global/shared");
}

#[test]
fn execution_packages_and_guarded_stdin_controls_work_without_external_next() {
    let temp = project_copy();
    let root = temp.path();
    assert_eq!(result(root, &["execution", "new"])["status"], "idle");
    let package = result(
        root,
        &[
            "execution",
            "start",
            "implement-feature",
            "--name",
            "native-start",
        ],
    );
    assert_eq!(package["packet"]["nodeId"], "plan");
    let run_id = package["runId"].as_str().unwrap();
    let expected = package["expected"].clone();
    fs::write(
        package["packet"]["outputs"][0]["path"].as_str().unwrap(),
        "Plan",
    )
    .unwrap();
    let control = serde_json::json!({"expected":expected,"summary":"Inspected"});
    let output = command(root)
        .args(["execution", "complete", "--run", run_id])
        .write_stdin(control.to_string())
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    let next: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(next["packet"]["kind"], "human-gate");
    assert_eq!(next["packet"], result(root, &["next", "--run", run_id]));
    let rejected = command(root)
        .args(["execution", "complete", "--run", run_id])
        .write_stdin(control.to_string())
        .output()
        .unwrap();
    assert!(!rejected.status.success());
    let error: Value = serde_json::from_slice(&rejected.stderr).unwrap();
    assert_eq!(error["error"]["code"], "stale_execution");
    assert_eq!(
        result(root, &["status", "--run", run_id])["history"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn invalid_execution_stdin_never_mutates_state() {
    let temp = project_copy();
    let root = temp.path();
    let package = result(
        root,
        &[
            "execution",
            "start",
            "implement-feature",
            "--name",
            "bad-input",
        ],
    );
    let state = root.join(".unelos/runs/r-000001/state.json");
    let run_id = package["runId"].as_str().unwrap();
    let before = fs::read(&state).unwrap();
    for text in [
        "not-json".to_owned(),
        "{}".to_owned(),
        "{} {}".to_owned(),
        "x".repeat(1024 * 1024 + 1),
        serde_json::json!({"expected":package["expected"],"unexpected":true}).to_string(),
    ] {
        let output = command(root)
            .args(["execution", "complete", "--run", run_id])
            .write_stdin(text)
            .output()
            .unwrap();
        assert!(!output.status.success());
        assert_eq!(fs::read(&state).unwrap(), before);
    }
    for operation in ["approve", "reject"] {
        let output = command(root)
            .args(["execution", operation, "--run", run_id])
            .write_stdin("{}")
            .output()
            .unwrap();
        assert!(!output.status.success());
        assert_eq!(fs::read(&state).unwrap(), before);
    }
}

#[test]
fn lost_mutation_response_reports_uncertainty_and_does_not_retry() {
    use std::{
        io::Write,
        process::{Command as Process, Stdio},
    };
    let temp = project_copy();
    let root = temp.path();
    let package = result(
        root,
        &[
            "execution",
            "start",
            "implement-feature",
            "--name",
            "lost-response",
        ],
    );
    fs::write(
        package["packet"]["outputs"][0]["path"].as_str().unwrap(),
        "Plan",
    )
    .unwrap();
    let input = serde_json::json!({"expected":package["expected"],"summary":"Inspected"});
    let run_id = package["runId"].as_str().unwrap();
    let program = cargo_bin_cmd!("unelos").get_program().to_owned();
    let mut child = Process::new(program)
        .arg("--project")
        .arg(root)
        .args(["execution", "complete", "--json", "--run", run_id])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    drop(child.stdout.take());
    child
        .stdin
        .take()
        .unwrap()
        .write_all(input.to_string().as_bytes())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(!output.status.success());
    let error: Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(error["error"]["code"], "mutation_uncertain");
    assert!(error["error"]["message"].as_str().unwrap().contains(run_id));
    assert_eq!(
        result(root, &["status", "--run", run_id])["history"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn selection_start_status_next_and_node_completion() {
    let temp = project_copy();
    let root = temp.path();
    command(root).arg("status").assert().failure();
    assert!(result(root, &["start"])["workflows"]
        .as_array()
        .unwrap()
        .contains(&Value::String("implement-feature".into())));
    assert!(!root.join(".unelos/active-run").exists());
    let summary = result(root, &["start", "implement-feature"]);
    let run_id = summary["runId"].as_str().unwrap();
    assert_eq!(summary["namingPending"], true);
    assert_eq!(summary["current"]["nodeId"], "plan");
    let packet = result(root, &["next", "--run", run_id]);
    assert_eq!(packet, result(root, &["next", "--run", run_id]));
    let path = packet["outputs"][0]["path"].as_str().unwrap();
    fs::write(path, "plan").unwrap();
    assert_eq!(
        result(root, &["complete", "plan", "--run", run_id])["status"],
        "waiting"
    );
    assert_eq!(
        result(root, &["status", "--run", run_id])["history"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn gate_rejections_and_reads_preserve_published_progression() {
    let temp = project_copy();
    let root = temp.path();
    let started = result(root, &["start", "implement-feature"]);
    let run_id = started["runId"].as_str().unwrap();
    let state_path =
        std::path::Path::new(started["runDirectory"].as_str().unwrap()).join("state.json");
    let bytes = || fs::read(&state_path).unwrap();

    let task_before = bytes();
    result(root, &["status", "--run", run_id]);
    let packet = result(root, &["next", "--run", run_id]);
    assert_eq!(packet, result(root, &["next", "--run", run_id]));
    assert_eq!(bytes(), task_before);

    let plan_path = packet["outputs"][0]["path"].as_str().unwrap();
    fs::write(plan_path, "Plan").unwrap();
    result(root, &["complete", "plan", "--run", run_id]);
    let gate_before = bytes();
    result(root, &["status", "--run", run_id]);
    let gate = result(root, &["next", "--run", run_id]);
    assert_eq!(gate, result(root, &["next", "--run", run_id]));
    assert_eq!(bytes(), gate_before);

    for reason in ["", " ", "\u{2003}"] {
        let output = command(root)
            .args([
                "reject",
                "approve-plan",
                "--reason",
                reason,
                "--run",
                run_id,
            ])
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(2), "{output:?}");
        assert!(output.stdout.is_empty());
        let error: Value = serde_json::from_slice(&output.stderr).unwrap();
        assert_eq!(error["error"]["code"], "invalid_runtime_state");
        assert!(error["error"]["message"]
            .as_str()
            .unwrap()
            .contains("reason"));
        assert_eq!(bytes(), gate_before);
    }

    fs::remove_file(plan_path).unwrap();
    for args in [
        vec!["approve", "approve-plan", "--run", run_id],
        vec![
            "reject",
            "approve-plan",
            "--reason",
            "Explain rollback.",
            "--run",
            run_id,
        ],
    ] {
        let output = command(root).args(args).output().unwrap();
        assert_eq!(output.status.code(), Some(2), "{output:?}");
        assert!(output.stdout.is_empty());
        let error: Value = serde_json::from_slice(&output.stderr).unwrap();
        assert_eq!(error["error"]["code"], "invalid_runtime_state");
        assert!(error["error"]["message"]
            .as_str()
            .unwrap()
            .contains(plan_path));
        assert_eq!(bytes(), gate_before);
    }
    fs::write(plan_path, "Plan").unwrap();
    assert_eq!(
        result(root, &["approve", "approve-plan", "--run", run_id])["current"]["nodeId"],
        "implement"
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
        serde_json::json!([
            "another",
            "completion-loop",
            "implement-feature",
            "research-feature"
        ])
    );
    for name in [
        "another",
        "completion-loop",
        "implement-feature",
        "research-feature",
    ] {
        fs::remove_file(root.join(format!(".unelos/workflows/{name}.yaml"))).unwrap();
    }
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
    assert!(std::path::Path::new(&old_path).exists());
    assert_eq!(renamed["runDirectory"], started["runDirectory"]);

    let packet = result(root, &["next", "--run", run_id]);
    assert_eq!(packet["kind"], "task");
    assert!(packet.get("controllerInstructions").is_none());
    assert!(!packet.to_string().contains("complete NODE"));
    let state_path =
        std::path::Path::new(renamed["runDirectory"].as_str().unwrap()).join("state.json");
    let before = fs::read(&state_path).unwrap();
    for args in [
        vec!["complete", "wrong", "--run", run_id],
        vec!["complete", "plan", "--run", run_id],
    ] {
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
    assert_eq!(
        result(root, &["complete", "plan", "--run", run_id])["status"],
        "waiting"
    );
    let gate = result(root, &["next", "--run", run_id]);
    assert_eq!(gate["kind"], "human-gate");
    assert!(gate.get("controllerInstructions").is_none());
    assert_eq!(
        result(
            root,
            &[
                "reject",
                "approve-plan",
                "--reason",
                "Revise scope.",
                "--run",
                run_id
            ]
        )["current"]["nodeId"],
        "plan"
    );
    let state = result(root, &["status", "--run", run_id]);
    assert_eq!(state["history"].as_array().unwrap().len(), 2);
    assert_eq!(state["history"][1]["reason"], "Revise scope.");
    let feedback = result(root, &["next", "--run", run_id])["feedback"].clone();
    assert_eq!(feedback["reason"], "Revise scope.");
    assert!(
        std::path::Path::new(feedback["artifacts"][0]["path"].as_str().unwrap())
            .ends_with(std::path::Path::new("plan").join("plan.md"))
    );
}

#[test]
fn cli_usage_errors_and_idle_next_are_distinct() {
    let temp = project_copy();
    let root = temp.path();
    let no_next = command(root).arg("next").output().unwrap();
    assert_eq!(no_next.status.code(), Some(2));
    assert!(no_next.stdout.is_empty());
    assert!(String::from_utf8(no_next.stderr).unwrap().contains("--run"));

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

    let started = result(root, &["start", "implement-feature"]);
    let run_id = started["runId"].as_str().unwrap();
    let packet = result(root, &["next", "--run", run_id]);
    fs::write(packet["outputs"][0]["path"].as_str().unwrap(), "Plan").unwrap();
    result(root, &["complete", "plan", "--run", run_id]);
    let output = readable_command(root)
        .args(["status", "--run", run_id])
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    let readable = String::from_utf8(output.stdout).unwrap();
    assert!(readable.contains("History:"));
    assert!(readable.contains("plan (task)"));
    let packet = readable_command(root)
        .args(["next", "--run", run_id])
        .output()
        .unwrap();
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
    let missing_output = command(root)
        .args(["complete", "plan", "--run", &run_id])
        .output()
        .unwrap();
    assert_eq!(missing_output.status.code(), Some(2));
    assert!(missing_output.stdout.is_empty());
    assert_eq!(fs::read(&state_path).unwrap(), unchanged);

    fn write_outputs(packet: &Value, text: &str) {
        for output in packet["outputs"].as_array().unwrap() {
            fs::write(output["path"].as_str().unwrap(), text).unwrap();
        }
    }

    let packet = result(root, &["next", "--run", &run_id]);
    write_outputs(&packet, "Plan v1");
    result(root, &["complete", "plan", "--run", &run_id]);
    let gate = result(root, &["next", "--run", &run_id]);
    assert_eq!(gate["kind"], "human-gate");
    let gate_state = fs::read(&state_path).unwrap();
    let gate_complete = command(root)
        .args(["complete", "approve-plan", "--run", &run_id])
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
            "--run",
            &run_id,
        ],
    );
    let revised = result(root, &["next", "--run", &run_id]);
    assert_eq!(revised["feedback"]["reason"], "Explain the rollback.");
    assert!(std::path::Path::new(
        revised["feedback"]["artifacts"][0]["path"]
            .as_str()
            .unwrap()
    )
    .ends_with(std::path::Path::new("plan").join("plan.md")));
    write_outputs(&revised, "Plan v2 with rollback.");
    result(root, &["complete", "plan", "--run", &run_id]);
    result(root, &["approve", "approve-plan", "--run", &run_id]);

    let implementation = result(root, &["next", "--run", &run_id]);
    write_outputs(&implementation, "Implementation v1");
    result(root, &["complete", "implement", "--run", &run_id]);
    let review = result(root, &["next", "--run", &run_id]);
    write_outputs(&review, "Review asks for a correction");
    let review_state = fs::read(&state_path).unwrap();
    let invalid = command(root)
        .args([
            "complete",
            "review",
            "--outcome",
            "unknown",
            "--run",
            &run_id,
        ])
        .output()
        .unwrap();
    assert_eq!(invalid.status.code(), Some(2));
    assert_eq!(fs::read(&state_path).unwrap(), review_state);
    result(
        root,
        &[
            "complete",
            "review",
            "--outcome",
            "changes_requested",
            "--run",
            &run_id,
        ],
    );
    let correction = result(root, &["next", "--run", &run_id]);
    assert_eq!(correction["nodeId"], "implement");
    write_outputs(&correction, "Implementation v2");
    result(root, &["complete", "implement", "--run", &run_id]);
    let final_review = result(root, &["next", "--run", &run_id]);
    write_outputs(&final_review, "Review approved");
    result(
        root,
        &[
            "complete",
            "review",
            "--outcome",
            "approved",
            "--run",
            &run_id,
        ],
    );

    let finished_bytes = fs::read(&state_path).unwrap();
    let finished = result(root, &["status", "--run", &run_id]);
    assert_eq!(finished["status"], "finished");
    assert_eq!(finished["runId"], run_id);
    assert_eq!(finished["history"].as_array().unwrap().len(), 8);
    let finished_packet = result(root, &["next", "--run", &run_id]);
    assert_eq!(finished_packet["kind"], "finished");
    assert_eq!(finished_packet, result(root, &["next", "--run", &run_id]));
    assert_eq!(fs::read(&state_path).unwrap(), finished_bytes);
    assert!(!root.join(".unelos/active-run").exists());
}

#[test]
fn rejected_mutations_emit_structured_errors_without_changing_state() {
    let temp = project_copy();
    let root = temp.path();
    let started = result(root, &["start", "implement-feature"]);
    let run_id = started["runId"].as_str().unwrap();
    let state_path =
        std::path::Path::new(started["runDirectory"].as_str().unwrap()).join("state.json");
    result(root, &["start", "implement-feature", "--name", "collision"]);
    let before = fs::read(&state_path).unwrap();
    for (args, code) in [
        (
            vec!["rename", "!!!", "--run", started["runId"].as_str().unwrap()],
            "invalid_run_name",
        ),
        (
            vec!["rename", "good-name", "--run", "wrong-run"],
            "unknown_run",
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
        (
            vec!["approve", "plan", "--run", run_id],
            "invalid_runtime_state",
        ),
        (
            vec!["reject", "plan", "--reason", " ", "--run", run_id],
            "invalid_runtime_state",
        ),
        (
            vec![
                "complete",
                "review",
                "--outcome",
                "approved",
                "--run",
                run_id,
            ],
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
        vec!["complete", "plan", "--execution", "task-1", "--run", run_id],
        vec!["complete", "plan", "--reason", "done", "--run", run_id],
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
    let run_id = started["runId"].as_str().unwrap();
    let packet = result(root, &["next", "--run", run_id]);
    fs::write(packet["outputs"][0]["path"].as_str().unwrap(), "Plan").unwrap();
    let children = (0..2)
        .map(|_| {
            std::process::Command::new(assert_cmd::cargo::cargo_bin!("unelos"))
                .env_clear()
                .args(["--project"])
                .arg(root)
                .args(["--json", "complete", "plan", "--run", run_id])
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
    let status = result(root, &["status", "--run", run_id]);
    assert_eq!(status["history"].as_array().unwrap().len(), 1);
    assert_eq!(status["current"]["nodeId"], "approve-plan");
    assert_eq!(status["runId"], started["runId"]);
}
