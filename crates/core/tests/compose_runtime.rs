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

// Runtime joins run paths per segment, so Windows paths mix `/` and `\`.
fn runs(root: &Utf8Path) -> Utf8PathBuf {
    root.join(".unelos/runs")
}

fn at(base: &Utf8Path, rel: &str) -> Utf8PathBuf {
    rel.split('/')
        .fold(base.to_owned(), |path, part| path.join(part))
}

#[test]
fn runs_parent_through_child_gate_and_back() {
    let (_temp, root) = fixture();
    let plan = execution_start(&root, "fullstack", Some("composition")).unwrap();
    let design = act(&root, &plan, ExecutionOperation::Complete, None);
    let run = runs(&root).join("r-000001");
    assert_eq!(packet(&design)["nodeId"], "frontend/design");
    assert_eq!(
        packet(&design)["inputs"][0]["path"],
        at(&run, "plan/plan.md").as_str()
    );
    let path = at(&run, "frontend/design/design.md");
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
        at(&run, "frontend/build/frontend-report.md").as_str()
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
        at(&runs(&root), "r-000001/S/T/m/top.md").as_str()
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

#[test]
fn standalone_child_marks_missing_input() {
    let (_temp, root) = fixture();
    let path = root.join(".unelos/workflows/react-component.yaml");
    let text = std::fs::read_to_string(&path)
        .unwrap()
        .replace("execution: main", "execution: subagent");
    std::fs::write(path, text).unwrap();
    let work = execution_start(&root, "react-component", None).unwrap();
    assert_eq!(
        packet(&work)["missingInputs"],
        serde_json::json!(["brief.md"])
    );
    assert_eq!(packet(&work)["inputs"], serde_json::json!([]));
    let prompt = work.worker_prompt.as_ref().unwrap();
    assert!(prompt.contains("Missing inputs (ask the coordinator for this content)"));
    assert!(prompt.contains("- brief.md"));
    assert!(!root.join(".unelos/runs/r-000001/design/brief.md").exists());
    let resumed = execution_next(&root, "r-000001").unwrap();
    assert_eq!(
        packet(&resumed)["missingInputs"],
        serde_json::json!(["brief.md"])
    );
}

#[test]
fn missing_input_reaches_gate_packet() {
    let (_temp, root) = fixture();
    let path = root.join(".unelos/workflows/react-component.yaml");
    let text = std::fs::read_to_string(&path)
        .unwrap()
        .replace("inputs: [design.md]", "inputs: [design.md, brief.md]");
    std::fs::write(path, text).unwrap();
    let work = execution_start(&root, "react-component", None).unwrap();
    let gate = act(&root, &work, ExecutionOperation::Complete, None);
    assert_eq!(
        packet(&gate)["missingInputs"],
        serde_json::json!(["brief.md"])
    );
    assert_eq!(packet(&gate)["artifacts"].as_array().unwrap().len(), 1);
    execution_next(&root, "r-000001").unwrap();
    act(&root, &gate, ExecutionOperation::Approve, None);
}

#[test]
fn parent_declared_input_propagates_into_child() {
    let (_temp, root) = fixture();
    let path = root.join(".unelos/workflows/fullstack.yaml");
    let text = std::fs::read_to_string(&path)
        .unwrap()
        .replace("start: plan", "start: frontend\ninputs: [brief.md]")
        .replace("brief.md: plan.md", "brief.md: brief.md");
    std::fs::write(path, text).unwrap();
    let work = execution_start(&root, "fullstack", None).unwrap();
    assert_eq!(packet(&work)["nodeId"], "frontend/design");
    assert_eq!(
        packet(&work)["missingInputs"],
        serde_json::json!(["brief.md"])
    );
    assert_eq!(packet(&work)["inputs"], serde_json::json!([]));
}

#[test]
fn embedded_input_still_binds() {
    let (_temp, root) = fixture();
    let plan = execution_start(&root, "fullstack", None).unwrap();
    let design = act(&root, &plan, ExecutionOperation::Complete, None);
    assert!(packet(&design).get("missingInputs").is_none());
    assert_eq!(packet(&design)["inputs"][0]["name"], "plan.md");
}

#[test]
fn many_to_one_inputs_bind_once_for_every_consumer() {
    for consumer in [
        "agent: worker\n    execution: main\n    taskDefinition: {instructions: Consume}\n    next: done",
        "type: human-gate\n    next: {approved: done, rejected: done}",
        "type: parallel\n    tasks:\n      - agent: worker\n        taskDefinition: {instructions: Consume}\n    next: done",
    ] {
        let (_temp, root) = fixture();
        std::fs::write(
            root.join(".unelos/workflows/child.yaml"),
            format!("name: child\nstart: consume\ninputs: [a.md, b.md, c.md]\nnodes:\n  consume:\n    inputs: [a.md, c.md, b.md]\n    {consumer}\n  done: {{type: end}}\n"),
        ).unwrap();
        std::fs::write(
            root.join(".unelos/workflows/parent.yaml"),
            "name: parent\nstart: produce\nnodes:\n  produce:\n    agent: worker\n    execution: main\n    taskDefinition: {instructions: Produce}\n    outputs: [x.md, y.md]\n    next: call\n  call:\n    type: workflow\n    workflow: child\n    inputs: {a.md: x.md, b.md: x.md, c.md: y.md}\n    next: done\n  done: {type: end}\n",
        ).unwrap();
        let producer = execution_start(&root, "parent", None).unwrap();
        let consumer = act(&root, &producer, ExecutionOperation::Complete, None);
        let value = packet(&consumer);
        let inputs = value.get("inputs").or_else(|| value.get("artifacts")).unwrap();
        assert_eq!(inputs.as_array().unwrap().len(), 2);
        assert_eq!(inputs[0]["name"], "x.md");
        assert_eq!(inputs[1]["name"], "y.md");
        assert_eq!(inputs[0]["path"], at(&runs(&root), "r-000001/produce/x.md").as_str());
        if let Some(workers) = value["workers"].as_array() {
            assert_eq!(workers[0]["inputs"], *inputs);
        }
        execution_next(&root, "r-000001").unwrap();
    }
}

#[test]
fn referenced_child_cannot_be_removed_during_parent_run() {
    let (_temp, root) = fixture();
    execution_start(&root, "fullstack", None).unwrap();
    let error = remove_workflow(&root, "react-component").unwrap_err();
    assert!(
        error.contains("Cannot remove Workflow \"react-component\": referenced by"),
        "{error}"
    );
    assert!(error.contains("fullstack.yaml"), "{error}");
    assert!(root.join(".unelos/workflows/react-component.yaml").exists());
    execution_next(&root, "r-000001").unwrap();
}

#[test]
fn missing_input_reaches_parallel_workers() {
    let (_temp, root) = fixture();
    std::fs::write(
        root.join(".unelos/workflows/parallel.yaml"),
        r#"
name: parallel
start: research
inputs: [brief.md]
nodes:
  research:
    type: parallel
    inputs: [brief.md]
    outputs: [report.md]
    tasks:
      - agent: worker
        taskDefinition: {instructions: Research}
    next: done
  done: {type: end}
"#,
    )
    .unwrap();
    let work = execution_start(&root, "parallel", None).unwrap();
    assert_eq!(
        packet(&work)["missingInputs"],
        serde_json::json!(["brief.md"])
    );
    assert_eq!(
        packet(&work)["workers"][0]["missingInputs"],
        serde_json::json!(["brief.md"])
    );
    assert_eq!(packet(&work)["workers"][0]["inputs"], serde_json::json!([]));
    assert!(work.worker_prompts[0]
        .prompt
        .contains("Missing inputs (ask the coordinator for this content)\n- brief.md"));
    execution_next(&root, "r-000001").unwrap();
}
