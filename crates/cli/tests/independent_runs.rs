use assert_cmd::cargo::cargo_bin_cmd;
use assert_fs::{fixture::PathCopy, TempDir};
use serde_json::Value;

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
fn result(root: &std::path::Path, args: &[&str]) -> Value {
    let output = command(root).args(args).output().unwrap();
    assert!(output.status.success(), "{output:?}");
    assert!(output.stderr.is_empty(), "{output:?}");
    serde_json::from_slice(&output.stdout).unwrap()
}
#[test]
fn cli_requires_explicit_target_and_supports_two_unfinished_runs() {
    let temp = project_copy();
    let root = temp.path();
    let a = result(
        root,
        &[
            "execution",
            "start",
            "implement-feature",
            "--name",
            "first-run",
        ],
    );
    let b = result(
        root,
        &[
            "execution",
            "start",
            "implement-feature",
            "--name",
            "second-run",
        ],
    );
    let a_id = a["runId"].as_str().unwrap();
    let b_id = b["runId"].as_str().unwrap();
    assert_ne!(a_id, b_id);
    command(root).args(["execution", "next"]).assert().failure();
    command(root).arg("status").assert().failure();
    assert_eq!(
        result(root, &["execution", "next", "--run", a_id])["runId"],
        a["runId"]
    );
    assert_eq!(
        result(root, &["execution", "resume", "--run", b_id])["runId"],
        b["runId"]
    );
    assert!(result(root, &["execution", "new"])["runId"].is_null());
    assert_eq!(
        result(root, &["execution", "runs"])["runs"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
}

#[test]
fn cross_run_stdin_is_rejected_and_only_the_selected_run_advances() {
    let temp = project_copy();
    let root = temp.path();
    let a = result(
        root,
        &[
            "execution",
            "start",
            "implement-feature",
            "--name",
            "first-run",
        ],
    );
    let b = result(
        root,
        &[
            "execution",
            "start",
            "implement-feature",
            "--name",
            "second-run",
        ],
    );
    let a_id = a["runId"].as_str().unwrap();
    let b_id = b["runId"].as_str().unwrap();
    let a_path = root.join(format!(".unelos/runs/{a_id}/state.json"));
    let b_path = root.join(format!(".unelos/runs/{b_id}/state.json"));
    let before = (
        std::fs::read(&a_path).unwrap(),
        std::fs::read(&b_path).unwrap(),
    );
    std::fs::write(
        a["packet"]["outputs"][0]["path"].as_str().unwrap(),
        "Plan A",
    )
    .unwrap();
    let control =
        serde_json::json!({"expected":a["expected"],"summary":"Inspected plan A"}).to_string();
    let output = command(root)
        .args(["execution", "complete", "--run", b_id])
        .write_stdin(control.as_str())
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert_eq!(
        serde_json::from_slice::<Value>(&output.stderr).unwrap()["error"]["code"],
        "stale_execution"
    );
    assert_eq!(
        (
            std::fs::read(&a_path).unwrap(),
            std::fs::read(&b_path).unwrap()
        ),
        before
    );
    let output = command(root)
        .args(["execution", "complete", "--run", a_id])
        .write_stdin(control)
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        serde_json::from_slice::<Value>(&output.stdout).unwrap()["runId"],
        a_id
    );
    assert_ne!(std::fs::read(&a_path).unwrap(), before.0);
    assert_eq!(std::fs::read(&b_path).unwrap(), before.1);
}

#[test]
fn unknown_selectors_and_finished_resume_are_structured_errors() {
    let temp = project_copy();
    let root = temp.path();
    let output = command(root)
        .args(["status", "--run", "unknown"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert_eq!(
        serde_json::from_slice::<Value>(&output.stderr).unwrap()["error"]["code"],
        "unknown_run"
    );
    std::fs::write(
        root.join(".unelos/workflows/implement-feature.yaml"),
        "name: implement-feature\nstart: done\nnodes:\n  done:\n    type: end\n",
    )
    .unwrap();
    let finished = result(root, &["execution", "start", "implement-feature"]);
    assert!(finished["runId"].is_string());
    let output = command(root)
        .args([
            "execution",
            "resume",
            "--run",
            finished["runId"].as_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert_eq!(
        serde_json::from_slice::<Value>(&output.stderr).unwrap()["error"]["code"],
        "run_finished"
    );
    assert!(result(root, &["execution", "runs"])["runs"]
        .as_array()
        .unwrap()
        .is_empty());
    for args in [
        vec!["execution", "new", "--run", "unknown"],
        vec!["start", "--run", "unknown"],
    ] {
        command(root).args(args).assert().failure();
    }
}
