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

fn inline_fixture() -> (TempDir, Utf8PathBuf) {
    let (temp, root) = fixture();
    let workflows = root.join(".unelos/workflows");
    std::fs::write(workflows.join("a.yaml"), "name: a\nstart: write\nnodes:\n  write:\n    agent: worker\n    taskDefinition: {instructions: Write, outcomes: [ok, quit]}\n    outputs: [summary.md, notes.md]\n    next: {ok: done, quit: abandoned}\n  done: {type: end}\n  abandoned: {type: end}\n").unwrap();
    std::fs::write(workflows.join("b.yaml"), "name: b\nstart: read\ninputs: [summary.md, extra.md, missing.md]\nnodes:\n  read:\n    agent: worker\n    taskDefinition: {instructions: Read}\n    inputs: [summary.md, extra.md, missing.md]\n    next: done\n  done: {type: end}\n").unwrap();
    (temp, root)
}

fn inline_steps(yaml: &str) -> Vec<InlineStep> {
    serde_yaml_ng::from_str(yaml).unwrap()
}

#[test]
fn run_workflow_uses_the_saved_inline_graph_or_the_named_document() {
    let (_temp, root) = inline_fixture();
    let named = start(&load_project(&root, "a").unwrap(), "named", "").unwrap();
    assert_eq!(
        serde_json::to_value(get_run_workflow(&root, &named.run_id).unwrap()).unwrap(),
        serde_json::to_value(get_workflow(&root, "a").unwrap()).unwrap()
    );

    let steps = inline_steps("- workflow: a\n- gate: Continue?\n- workflow: a\n");
    execution_start_inline(&root, &steps, None).unwrap();
    let run_id = list_runs(&root)
        .unwrap()
        .into_iter()
        .find(|run| run.run_id != named.run_id)
        .unwrap()
        .run_id;
    std::fs::remove_file(root.join(".unelos/workflows/a.yaml")).unwrap();
    let doc = get_run_workflow(&root, &run_id).unwrap();
    assert_eq!(doc.settings.start, "a");
    assert_eq!(doc.steps[1].title.as_deref(), Some("Continue?"));
    assert_eq!(
        doc.steps
            .iter()
            .map(|step| (step.id.as_str(), step.kind.as_str()))
            .collect::<Vec<_>>(),
        [
            ("a", "workflow"),
            ("gate-1", "human-gate"),
            ("a-2", "workflow"),
            ("done", "end"),
            ("stopped", "end"),
        ]
    );
}

#[test]
fn inline_synthesizes_routes_and_binds_latest_outputs_across_a_gate() {
    let (_temp, root) = inline_fixture();
    let steps = inline_steps(
        "- workflow: a\n  stopOn: [abandoned]\n- workflow: a\n- gate: Open the PR?\n- workflow: b\n  inputs: {extra.md: a/notes.md}\n",
    );
    let first = execution_start_inline(&root, &steps, None).unwrap();
    let path = runs(&root).join("r-000001/workflow.yaml");
    let saved = std::fs::read_to_string(path).unwrap();
    let definition = parse_workflow_definition("workflow.yaml", &saved).unwrap();
    assert_eq!(
        definition
            .nodes
            .keys()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        ["a", "a-2", "gate-1", "b", "done", "stopped"]
    );
    let NodeDefinition::Workflow(call) = &definition.nodes["a"] else {
        panic!()
    };
    let Transition::Outcomes(routes) = &call.next else {
        panic!()
    };
    assert_eq!(routes["abandoned"], "stopped");
    assert_eq!(routes["done"], "a-2");
    let second = act(&root, &first, ExecutionOperation::Complete, Some("ok"));
    let gate = act(&root, &second, ExecutionOperation::Complete, Some("ok"));
    assert_eq!(packet(&gate)["nodeId"], "gate-1");
    assert_eq!(packet(&gate)["description"], "Open the PR?");
    assert_eq!(
        packet(&execution_resume(&root, "r-000001").unwrap()),
        packet(&gate)
    );
    let catalog = execution_runs(&root).unwrap();
    assert_eq!(catalog.runs[0].workflow_name, "a → a → gate → b");
    let b = act(&root, &gate, ExecutionOperation::Approve, None);
    assert_eq!(packet(&b)["inputs"][0], packet(&second)["outputs"][0]);
    assert_eq!(packet(&b)["inputs"][1], packet(&first)["outputs"][1]);
    assert_eq!(
        packet(&b)["missingInputs"],
        serde_json::json!(["missing.md"])
    );
    assert_eq!(
        packet(&execution_next(&root, "r-000001").unwrap()),
        packet(&b)
    );

    let child = root.join(".unelos/workflows/b.yaml");
    let text = std::fs::read_to_string(&child)
        .unwrap()
        .replace("start: read", "start: changed")
        .replace("  read:", "  changed:");
    std::fs::write(child, text).unwrap();
    for error in [
        execution_resume(&root, "r-000001").unwrap_err(),
        execution_next(&root, "r-000001").unwrap_err(),
    ] {
        assert!(
            error
                .to_string()
                .contains("incompatible with live Workflow"),
            "{error}"
        );
    }
}

#[test]
fn inline_gate_can_be_first_and_rejection_stops() {
    let (_temp, root) = inline_fixture();
    let steps = inline_steps("- gate: Start?\n- workflow: a");
    let gate = execution_start_inline(&root, &steps, None).unwrap();
    assert_eq!(packet(&gate)["nodeId"], "gate-1");
    let stopped = act(&root, &gate, ExecutionOperation::Reject, None);
    assert_eq!(packet(&stopped)["nodeId"], "stopped");
}

#[test]
fn inline_validates_step_references_before_starting() {
    let (_temp, root) = inline_fixture();
    for (yaml, diagnostic) in [
        ("- gate: Start?", "requires at least one workflow"),
        ("- workflow: not-here", "cannot load workflow"),
        ("- workflow: a\n  stopOn: [typo]", "unknown stopOn outcome"),
        (
            "- workflow: b\n  inputs: {typo: summary.md}",
            "unknown input",
        ),
        (
            "- workflow: b\n  inputs: {summary.md: a/summary.md}",
            "no earlier output",
        ),
    ] {
        let error = execution_start_inline(&root, &inline_steps(yaml), None).unwrap_err();
        assert!(error.to_string().contains(diagnostic), "{error}");
    }
    let run = execution_start_inline(&root, &[InlineStep::workflow("a")], None).unwrap();
    assert_eq!(packet(&run)["nodeId"], "a/write");
    let (_temp, root) = inline_fixture();
    execution_start(&root, "a", None).unwrap();
    let state = runs(&root).join("r-000001/state.json");
    let mut json: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&state).unwrap()).unwrap();
    json.as_object_mut().unwrap().remove("inline");
    assert!(!serde_json::from_value::<RunState>(json).unwrap().inline);
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
