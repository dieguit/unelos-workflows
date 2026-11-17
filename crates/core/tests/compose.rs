use camino::Utf8PathBuf;
use std::fs;
use unelos_core::{artifact_file_name, load_project, NodeDefinition, ProjectLoadError, Transition};

fn project(files: &[(&str, &str)]) -> (assert_fs::TempDir, Utf8PathBuf) {
    let temp = assert_fs::TempDir::new().unwrap();
    let root = Utf8PathBuf::from_path_buf(temp.path().to_owned()).unwrap();
    fs::create_dir_all(root.join(".unelos/workflows")).unwrap();
    fs::write(root.join(".unelos/project.yaml"), "{}").unwrap();
    for (name, body) in files {
        fs::write(root.join(format!(".unelos/workflows/{name}.yaml")), body).unwrap();
    }
    (temp, root)
}

const CHILD: &str = "name: child
start: design
inputs: [brief.md]
nodes:
  design: {taskDefinition: {instructions: design}, inputs: [brief.md], outputs: [design.md], next: approve}
  approve: {type: human-gate, inputs: [design.md], next: {approved: build, rejected: design}}
  build: {taskDefinition: {instructions: build, outcomes: [all_done, give_up]}, outputs: [report.md], next: {all_done: done, give_up: abandoned}}
  done: {type: end}
  abandoned: {type: end}
";
const PARENT: &str = "name: parent
start: plan
nodes:
  plan: {taskDefinition: {instructions: plan}, outputs: [plan.md], next: frontend}
  frontend: {type: workflow, workflow: child, inputs: {brief.md: plan.md}, outputs: {report.md: frontend-report.md}, next: {done: review, abandoned: plan}}
  review: {type: human-gate, inputs: [frontend-report.md], next: {approved: end, rejected: frontend}}
  end: {type: end}
";

#[test]
fn dotted_node_prefix_collision_reports_child_output_source() {
    let parent = "name: parent
start: a
nodes:
  a: {taskDefinition: {instructions: test}, agent: worker, outputs: [x.md], next: a.b}
  a.b: {type: workflow, workflow: child, outputs: {report.md: x.md}, next: end}
  end: {type: end}
";
    let child = "name: child
start: task
nodes:
  task: {taskDefinition: {instructions: test}, agent: worker, outputs: [report.md], next: done}
  done: {type: end}
";
    let (_temp, root) = project(&[("parent", parent), ("child", child)]);
    fs::create_dir_all(root.join(".unelos/agents/worker")).unwrap();
    fs::write(
        root.join(".unelos/agents/worker/agent.yaml"),
        "instructions: test",
    )
    .unwrap();
    let p = load_project(&root, "parent").unwrap();
    let e = unelos_core::validate_project(&p).unwrap_err();
    assert!(e.cause.contains("conflicting producers"), "{e}");
    assert!(e.path.ends_with("child.yaml"), "{e}");
    assert_eq!(e.field, "nodes.task.outputs");
}

#[test]
fn declared_inputs_need_no_producer() {
    let (_temp, root) = project(&[("child", CHILD)]);
    let mut p = load_project(&root, "child").unwrap();
    p.definitions.agents.insert(
        "".into(),
        serde_json::from_value(serde_json::json!({"instructions": "test"})).unwrap(),
    );
    assert!(unelos_core::validate_project(&p).is_ok());
}

#[test]
fn declared_inputs_are_validated_even_when_unused() {
    for (input, cause) in [
        ("design.md", "also produced"),
        ("../bad", "invalid artifact"),
    ] {
        let child = CHILD.replace("inputs: [brief.md]", &format!("inputs: [{input}]"));
        let (_temp, root) = project(&[("child", &child)]);
        let mut p = load_project(&root, "child").unwrap();
        p.definitions.agents.insert(
            "".into(),
            serde_json::from_value(serde_json::json!({"instructions": "test"})).unwrap(),
        );
        let e = unelos_core::validate_project(&p).unwrap_err();
        assert_eq!(e.field, "inputs");
        assert!(e.cause.contains(cause), "{e}");
    }
}

#[test]
fn child_errors_name_child_file() {
    let (_temp, root) = project(&[("parent", PARENT), ("child", CHILD)]);
    let e = match load_project(&root, "parent") {
        Err(ProjectLoadError::Validation(e)) => e,
        Ok(p) => unelos_core::validate_project(&p).unwrap_err(),
        Err(e) => panic!("{e}"),
    };
    assert!(e.path.ends_with("child.yaml"), "{e}");
    assert_eq!(e.field, "nodes.design.agent");
}

#[test]
fn child_contract_cannot_hide_invalid_declaration() {
    let child = CHILD.replace("inputs: [brief.md]", "inputs: [design.md]");
    let parent = PARENT.replace("brief.md: plan.md", "design.md: plan.md");
    let (_temp, root) = project(&[("parent", &parent), ("child", &child)]);
    let mut p = load_project(&root, "parent").unwrap();
    p.definitions.agents.insert(
        "".into(),
        serde_json::from_value(serde_json::json!({"instructions": "test"})).unwrap(),
    );
    let e = unelos_core::validate_project(&p).unwrap_err();
    assert_eq!(e.field, "inputs");
    assert!(e.path.ends_with("child.yaml"));
    assert!(e.cause.contains("also produced"), "{e}");
}

#[test]
fn nested_child_semantic_errors_keep_innermost_source() {
    let mid = "name: mid\nstart: inner\nnodes:\n  inner: {type: workflow, workflow: child, inputs: {brief.md: plan.md}, next: {done: end, abandoned: end}}\n  end: {type: end}\ninputs: [plan.md]";
    let parent = PARENT
        .replace("workflow: child", "workflow: mid")
        .replace("brief.md: plan.md", "plan.md: plan.md")
        .replace("outputs: {report.md: frontend-report.md}, ", "")
        .replace("{done: review, abandoned: plan}", "review");
    for (child, expected) in [
        (CHILD.to_owned(), "nodes.design.agent"),
        (CHILD.replace("start: design", "start: absent"), "start"),
        (
            CHILD.replace("outputs: [design.md]", "outputs: [../bad]"),
            "nodes.design.outputs",
        ),
    ] {
        let (_temp, root) = project(&[("parent", &parent), ("mid", mid), ("child", &child)]);
        let mut p = load_project(&root, "parent").unwrap();
        if expected != "nodes.design.agent" {
            p.definitions.agents.insert(
                "".into(),
                serde_json::from_value(serde_json::json!({"instructions": "test"})).unwrap(),
            );
        }
        let e = unelos_core::validate_project(&p).unwrap_err();
        assert!(e.path.ends_with("child.yaml"), "{e}");
        assert_eq!(e.field, expected);
    }
}

#[test]
fn export_includes_child_agents() {
    let child = CHILD.replace("taskDefinition:", "agent: child-agent, taskDefinition:");
    let parent = PARENT.replace("taskDefinition:", "agent: parent-agent, taskDefinition:");
    let (_temp, root) = project(&[("parent", &parent), ("child", &child)]);
    for id in ["child-agent", "parent-agent"] {
        fs::create_dir_all(root.join(format!(".unelos/agents/{id}"))).unwrap();
        fs::write(
            root.join(format!(".unelos/agents/{id}/agent.yaml")),
            "instructions: test",
        )
        .unwrap();
    }
    let agents = unelos_core::export_workflow_agents(&root, "parent").unwrap();
    assert!(agents.iter().any(|a| a.id == "child-agent"));
}

#[test]
fn graph_uses_authoring_workflow() {
    let (_temp, root) = project(&[("parent", PARENT), ("child", CHILD)]);
    let p = load_project(&root, "parent").unwrap();
    let graph = unelos_core::build_graph(&p.authoring_workflow);
    let node = graph.nodes.iter().find(|n| n.id == "frontend").unwrap();
    assert_eq!(node.kind, unelos_core::NodeKind::Workflow);
    assert_eq!(serde_json::to_value(node).unwrap()["workflow"], "child");
    assert!(graph
        .edges
        .iter()
        .any(|e| e.from == "frontend" && e.outcome.as_deref() == Some("done")));
}

#[test]
fn summaries_list_calls() {
    let (_temp, root) = project(&[("parent", PARENT), ("child", CHILD)]);
    let summaries = unelos_core::workflow_summaries(&root).unwrap();
    let parent = summaries.iter().find(|s| s.name == "parent").unwrap();
    assert_eq!(parent.steps, Some(3));
    assert_eq!(
        serde_json::to_value(parent).unwrap()["calls"],
        serde_json::json!(["child"])
    );
}

#[test]
fn expands_child_nodes_edges_and_artifacts() {
    let (_temp, root) = project(&[("parent", PARENT), ("child", CHILD)]);
    let p = load_project(&root, "parent").unwrap();
    assert!(p.authoring_workflow.nodes.contains_key("frontend"));
    assert!(!p.workflow.nodes.contains_key("frontend"));
    assert!(!p.workflow.nodes.contains_key("frontend/done"));
    let NodeDefinition::Task(plan) = &p.workflow.nodes["plan"] else {
        panic!()
    };
    assert_eq!(plan.next, Transition::Single("frontend/design".into()));
    let NodeDefinition::Task(design) = &p.workflow.nodes["frontend/design"] else {
        panic!()
    };
    assert_eq!(design.inputs, ["plan.md"]);
    assert_eq!(design.outputs, ["frontend/design.md"]);
    let NodeDefinition::Task(build) = &p.workflow.nodes["frontend/build"] else {
        panic!()
    };
    assert_eq!(build.outputs, ["frontend-report.md"]);
    let Transition::Outcomes(routes) = &build.next else {
        panic!()
    };
    assert_eq!(routes["all_done"], "review");
    assert_eq!(routes["give_up"], "plan");
    let NodeDefinition::HumanGate(review) = &p.workflow.nodes["review"] else {
        panic!()
    };
    assert_eq!(review.next["rejected"], "frontend/design");
    assert_eq!(p.node_sources["frontend/design"].node_id, "design");
    assert!(p.node_sources["frontend/design"]
        .path
        .ends_with("child.yaml"));
}

#[test]
fn start_and_repeated_calls() {
    let text = PARENT.replace("start: plan", "start: frontend").replace(
        "  review:",
        "  second: {type: workflow, workflow: child, inputs: {brief.md: plan.md}, next: {done: end, abandoned: end}}\n  review:",
    );
    let (_temp, root) = project(&[("parent", &text), ("child", CHILD)]);
    let p = load_project(&root, "parent").unwrap();
    assert_eq!(p.workflow.start, "frontend/design");
    let NodeDefinition::Task(t) = &p.workflow.nodes["second/design"] else {
        panic!()
    };
    assert_eq!(t.outputs, ["second/design.md"]);
}

#[test]
fn nested_two_levels() {
    let (_temp, root) = project(&[
        ("outer", "name: outer\nstart: S\nnodes:\n  S: {type: workflow, workflow: mid, outputs: {e1.md: top.md}, next: done}\n  done: {type: end}"),
        ("mid", "name: mid\nstart: T\nnodes:\n  T: {type: workflow, workflow: leaf, outputs: {y.md: e1.md}, next: done}\n  done: {type: end}"),
        ("leaf", "name: leaf\nstart: m\nnodes:\n  m: {taskDefinition: {instructions: x}, outputs: [y.md], next: done}\n  done: {type: end}"),
    ]);
    let p = load_project(&root, "outer").unwrap();
    assert_eq!(p.workflow.start, "S/T/m");
    let NodeDefinition::Task(t) = &p.workflow.nodes["S/T/m"] else {
        panic!()
    };
    assert_eq!(t.outputs, ["top.md"]);
    assert_eq!(p.node_sources["S/T/m"].node_id, "m");
    assert!(p.node_sources["S/T/m"].path.ends_with("leaf.yaml"));
}

#[test]
fn artifact_file_name_strips_scope() {
    for (node, name, expected) in [
        ("S/T/m", "S/T/y", "y"),
        ("S/T/m", "S/e1", "e1"),
        ("S/n", "S/reports/a.md", "reports/a.md"),
        ("plan", "plan.md", "plan.md"),
        ("S/T/m", "other/x", "other/x"),
    ] {
        assert_eq!(artifact_file_name(node, name), expected);
    }
}

#[test]
fn composition_errors_are_field_diagnostics() {
    let cases = [
        (
            PARENT.replace("workflow: child", "workflow: parent"),
            CHILD.to_owned(),
            "workflow",
            "parent -> parent",
        ),
        (
            PARENT.replace("workflow: child", "workflow: missing"),
            CHILD.to_owned(),
            "workflow",
            "missing",
        ),
        (
            PARENT.replace("inputs: {brief.md: plan.md}", "inputs: {}"),
            CHILD.to_owned(),
            "inputs",
            "brief.md",
        ),
        (
            PARENT.replace("brief.md: plan.md", "unknown.md: plan.md"),
            CHILD.to_owned(),
            "inputs",
            "brief.md",
        ),
        (
            PARENT.replace(
                "report.md: frontend-report.md",
                "unknown.md: frontend-report.md",
            ),
            CHILD.to_owned(),
            "outputs",
            "unknown.md",
        ),
        (
            PARENT.replace("done: review", "wrong: review"),
            CHILD.to_owned(),
            "next",
            "outcomes",
        ),
        (
            PARENT.replace("{done: review, abandoned: plan}", "review"),
            CHILD.to_owned(),
            "next",
            "single",
        ),
        (
            PARENT.to_owned(),
            CHILD.replace("start: design", "start: done"),
            "workflow",
            "end",
        ),
        (
            PARENT.replace("outputs: [plan.md]", "outputs: [frontend/x.md]"),
            CHILD.to_owned(),
            "outputs",
            "frontend/",
        ),
        (
            PARENT.replace("  plan:", "  bad/id:"),
            CHILD.to_owned(),
            "workflow",
            "/",
        ),
        (
            PARENT.replace("report.md: frontend-report.md", "report.md: design.md"),
            CHILD.replace("outputs: [report.md]", "outputs: [report.md, design.md]"),
            "outputs",
            "file name",
        ),
    ];
    for (parent, child, field, cause) in cases {
        let (_temp, root) = project(&[("parent", &parent), ("child", &child)]);
        let ProjectLoadError::Validation(e) = load_project(&root, "parent").unwrap_err() else {
            panic!("expected validation")
        };
        assert!(e.field.ends_with(field), "{e}");
        assert!(e.cause.contains(cause), "{e}");
    }
}

#[test]
fn rejects_cycle_with_chain() {
    let child = "name: child\nstart: recurse\nnodes:\n  recurse: {type: workflow, workflow: parent, next: done}\n  done: {type: end}";
    let (_temp, root) = project(&[("parent", PARENT), ("child", child)]);
    let e = load_project(&root, "parent").unwrap_err();
    assert!(e.to_string().contains("parent -> child -> parent"), "{e}");
}

#[test]
fn reusable_fixtures_expand() {
    let root = camino::Utf8Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/compose");
    let p = load_project(&root, "fullstack").unwrap();
    assert!(p.workflow.nodes.contains_key("frontend/design"));
    let p = load_project(&root, "outer").unwrap();
    assert!(p.workflow.nodes.contains_key("S/T/m"));
}

#[test]
fn child_ids_use_same_rules_as_root_workflows() {
    let parent = PARENT.replace("workflow: child", "workflow: child.v2");
    let child = CHILD.replace("name: child", "name: child.v2");
    let (_temp, root) = project(&[("parent", &parent), ("child.v2", &child)]);
    assert!(load_project(&root, "child.v2").is_ok());
    assert!(load_project(&root, "parent").is_ok());
}

#[test]
fn rejects_unbindable_mapping_for_unused_child_input() {
    let parent = PARENT.replace("brief.md: plan.md", "brief.md: nonexistent.md");
    let child = CHILD.replace("inputs: [brief.md], ", "");
    let (_temp, root) = project(&[("parent", &parent), ("child", &child)]);
    let ProjectLoadError::Validation(e) = load_project(&root, "parent").unwrap_err() else {
        panic!("expected validation");
    };
    assert!(e.path.ends_with("parent.yaml"));
    assert_eq!(e.field, "nodes.frontend.inputs");
    assert!(e.cause.contains("nonexistent.md"), "{e}");
}

#[test]
fn unused_child_input_accepts_parent_contract_and_sibling_outputs() {
    let child = CHILD.replace("inputs: [brief.md], ", "");
    for parent in [
        PARENT.replace("start: plan", "inputs: [supplied.md]\nstart: plan")
            .replace("brief.md: plan.md", "brief.md: supplied.md"),
        PARENT.replace("brief.md: plan.md", "brief.md: sibling-report.md")
            + "  sibling: {type: workflow, workflow: child, inputs: {brief.md: plan.md}, outputs: {report.md: sibling-report.md}, next: {done: end, abandoned: end}}\n",
    ] {
        let (_temp, root) = project(&[("parent", &parent), ("child", &child)]);
        assert!(load_project(&root, "parent").is_ok());
    }
}

#[test]
fn rejects_unknown_parent_target_for_unused_child_end() {
    let parent = PARENT.replace("abandoned: plan", "abandoned: nonexistent");
    let child = CHILD.replace("give_up: abandoned", "give_up: done");
    let (_temp, root) = project(&[("parent", &parent), ("child", &child)]);
    let ProjectLoadError::Validation(e) = load_project(&root, "parent").unwrap_err() else {
        panic!("expected validation");
    };
    assert!(e.path.ends_with("parent.yaml"));
    assert_eq!(e.field, "nodes.frontend.next");
    assert!(e.cause.contains("nonexistent"), "{e}");
}

#[test]
fn rejects_unknown_parent_target_for_single_next() {
    let parent = PARENT.replace("{done: review, abandoned: plan}", "nonexistent");
    let child = CHILD
        .replace("give_up: abandoned", "give_up: done")
        .replace("  abandoned: {type: end}\n", "");
    let (_temp, root) = project(&[("parent", &parent), ("child", &child)]);
    let ProjectLoadError::Validation(e) = load_project(&root, "parent").unwrap_err() else {
        panic!("expected validation");
    };
    assert_eq!(e.field, "nodes.frontend.next");
    assert!(e.cause.contains("nonexistent"), "{e}");
}
