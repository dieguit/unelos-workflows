use assert_fs::TempDir;
use camino::{Utf8Path, Utf8PathBuf};
use unelos_core::*;

fn fixture() -> (TempDir, Utf8PathBuf) {
    fn copy(from: &Utf8Path, to: &Utf8Path) {
        std::fs::create_dir_all(to).unwrap();
        for entry in std::fs::read_dir(from).unwrap() {
            let entry = entry.unwrap();
            let source = Utf8PathBuf::from_path_buf(entry.path()).unwrap();
            let target = to.join(source.file_name().unwrap());
            if source.is_dir() {
                copy(&source, &target);
            } else {
                std::fs::copy(source, target).unwrap();
            }
        }
    }
    let temp = TempDir::new().unwrap();
    let root = Utf8Path::from_path(temp.path()).unwrap().to_owned();
    copy(
        &Utf8Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/compose"),
        &root,
    );
    std::fs::create_dir_all(root.join(".unelos/agents/worker")).unwrap();
    std::fs::write(
        root.join(".unelos/agents/worker/agent.yaml"),
        "instructions: Work",
    )
    .unwrap();
    for entry in std::fs::read_dir(root.join(".unelos/workflows")).unwrap() {
        let path = entry.unwrap().path();
        let text = std::fs::read_to_string(&path).unwrap().replace(
            "taskDefinition:",
            "agent: worker, execution: main, taskDefinition:",
        );
        // Multiline task definitions need block fields, not flow separators.
        let text = text.replace(
            "    agent: worker, execution: main, taskDefinition:",
            "    agent: worker\n    execution: main\n    taskDefinition:",
        );
        std::fs::write(path, text).unwrap();
    }
    (temp, root)
}

fn packet(package: &ExecutionPackage) -> serde_json::Value {
    serde_json::to_value(&package.packet).unwrap()
}

fn act(
    root: &Utf8Path,
    package: &ExecutionPackage,
    operation: ExecutionOperation,
    outcome: Option<&str>,
) -> ExecutionPackage {
    let value = packet(package);
    if let Some(outputs) = value["outputs"].as_array() {
        for output in outputs {
            std::fs::write(
                output["path"].as_str().unwrap(),
                format!("Report {:?}\n", package.expected),
            )
            .unwrap();
        }
    }
    execution_mutate(
        root,
        "r-000001",
        operation,
        &CompletionControl {
            expected: package.expected.clone().unwrap(),
            summary: matches!(operation, ExecutionOperation::Complete).then(|| "Completed".into()),
            result_text: None,
            outcome: outcome.map(str::to_owned),
            reason: matches!(operation, ExecutionOperation::Reject)
                .then(|| "Revise the design".into()),
        },
    )
    .unwrap()
}

#[test]
fn runs_parent_through_child_gate_and_back() {
    let (_temp, root) = fixture();
    let plan = execution_start(&root, "fullstack", Some("composition")).unwrap();
    let design = act(&root, &plan, ExecutionOperation::Complete, None);
    let run = root.join(".unelos/runs/r-000001");
    assert_eq!(packet(&design)["nodeId"], "frontend/design");
    assert_eq!(
        packet(&design)["inputs"][0]["path"],
        run.join("plan/plan.md").as_str()
    );
    let path = run.join("frontend/design/design.md");
    assert_eq!(packet(&design)["outputs"][0]["path"], path.as_str());
    let gate = act(&root, &design, ExecutionOperation::Complete, None);
    assert_eq!(packet(&gate)["nodeId"], "frontend/approve");
    let rework = act(&root, &gate, ExecutionOperation::Reject, None);
    assert_eq!(packet(&rework)["nodeId"], "frontend/design");
    assert!(packet(&rework)["feedback"]
        .to_string()
        .contains("Revise the design"));
    let gate = act(&root, &rework, ExecutionOperation::Complete, None);
    let build = act(&root, &gate, ExecutionOperation::Approve, None);
    let review = act(
        &root,
        &build,
        ExecutionOperation::Complete,
        Some("all_done"),
    );
    assert_eq!(packet(&review)["nodeId"], "review");
    assert_eq!(
        packet(&review)["artifacts"][0]["path"],
        run.join("frontend/build/frontend-report.md").as_str()
    );
    let restart = act(&root, &review, ExecutionOperation::Reject, None);
    assert_eq!(packet(&restart)["nodeId"], "frontend/design");
    assert_eq!(packet(&restart)["outputs"][0]["path"], path.as_str());
    execution_next(&root, "r-000001").unwrap();
}

#[test]
fn nested_output_lands_in_scoped_folder() {
    let (_temp, root) = fixture();
    let work = execution_start(&root, "outer", Some("nested")).unwrap();
    assert_eq!(
        packet(&work)["outputs"][0]["path"],
        root.join(".unelos/runs/r-000001/S/T/m/top.md").as_str()
    );
    let done = act(&root, &work, ExecutionOperation::Complete, None);
    assert_eq!(packet(&done)["nodeId"], "done");
}

#[test]
fn child_edit_mid_run_is_incompatible_error() {
    let (_temp, root) = fixture();
    let plan = execution_start(&root, "fullstack", Some("edit")).unwrap();
    let design = act(&root, &plan, ExecutionOperation::Complete, None);
    act(&root, &design, ExecutionOperation::Complete, None);
    let path = root.join(".unelos/workflows/react-component.yaml");
    let text = std::fs::read_to_string(&path)
        .unwrap()
        .replace("next: approve", "next: renamed")
        .replace("  approve:", "  renamed:");
    std::fs::write(path, text).unwrap();
    let error = execution_next(&root, "r-000001").unwrap_err();
    assert!(matches!(
        error,
        ExecutionError::Runtime(RuntimeError::InvalidState(_))
    ));
    assert!(
        error
            .to_string()
            .contains("incompatible with live Workflow"),
        "{error}"
    );
}
