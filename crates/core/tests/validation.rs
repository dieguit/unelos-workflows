use camino::Utf8Path;
use unelos_core::{
    load_project, parse_workflow_definition, validate_project, LoadedProject, NodeDefinition,
    ParallelNode, Transition, WorkerReference,
};

fn fixture() -> LoadedProject {
    load_project(
        &Utf8Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/project"),
        "implement-feature",
    )
    .unwrap()
}

fn task<'a>(project: &'a mut LoadedProject, id: &str) -> &'a mut unelos_core::TaskNode {
    let NodeDefinition::Task(task) = project.workflow.nodes.get_mut(id).unwrap() else {
        panic!("expected task")
    };
    task
}

fn assert_error(project: &LoadedProject, field: &str, cause: &str) {
    let error = validate_project(project).unwrap_err();
    assert_eq!(error.path, project.workflow_source);
    assert_eq!(error.field, field);
    let rendered = error.to_string();
    assert!(
        rendered.contains(project.workflow_source.as_str()),
        "{rendered}"
    );
    assert!(rendered.contains(cause), "{rendered}");
}

#[test]
fn fixture_indexes_outputs_and_accepts_rework_loops() {
    let mut project = fixture();
    let producers = validate_project(&project).unwrap();
    assert_eq!(producers.len(), 3);
    assert_eq!(producers["plan.md"], "plan");
    assert_eq!(producers["implementation-report.md"], "implement");
    assert_eq!(producers["review.md"], "review");
    task(&mut project, "plan").next = Transition::Single("plan".into());
    assert!(validate_project(&project).is_ok());
}

#[test]
fn missing_entry_and_transition_targets_identify_the_reference() {
    let mut project = fixture();
    project.workflow.start = "missing".into();
    assert_error(&project, "start", "missing");
    let mut project = fixture();
    task(&mut project, "plan").next = Transition::Single("missing".into());
    assert_error(&project, "nodes.plan.next", "unknown target");
    let mut project = fixture();
    let Transition::Outcomes(routes) = &mut task(&mut project, "review").next else {
        unreachable!()
    };
    routes.insert("changes_requested".into(), "missing".into());
    assert_error(&project, "nodes.review.next.changes_requested", "missing");
    let mut project = fixture();
    let NodeDefinition::HumanGate(gate) = project.workflow.nodes.get_mut("approve-plan").unwrap()
    else {
        unreachable!()
    };
    gate.next.insert("rejected".into(), "missing".into());
    assert_error(
        &project,
        "nodes.approve-plan.next.rejected",
        "unknown target",
    );
}

#[test]
fn ordinary_task_references_must_resolve() {
    let mut project = fixture();
    task(&mut project, "review").task = "missing".into();
    assert_error(&project, "nodes.review.task", "unknown Task Definition");
    let mut project = fixture();
    task(&mut project, "review").agent = "missing".into();
    assert_error(&project, "nodes.review.agent", "unknown Agent");
}

fn parallel_project() -> LoadedProject {
    let mut project = fixture();
    project.workflow.nodes.insert(
        "implement".into(),
        NodeDefinition::Parallel(ParallelNode {
            tasks: vec![WorkerReference {
                task: "implement-plan".into(),
                agent: "engineer".into(),
            }],
            inputs: vec!["plan.md".into()],
            outputs: vec!["implementation-report.md".into()],
            next: Transition::Single("review".into()),
        }),
    );
    project
}

#[test]
fn parallel_workers_resolve_and_parent_artifacts_are_indexed() {
    let project = parallel_project();
    assert_eq!(
        validate_project(&project).unwrap()["implementation-report.md"],
        "implement"
    );
    for (field, cause) in [
        ("task", "unknown Task Definition"),
        ("agent", "unknown Agent"),
        ("tasks", "nonempty"),
    ] {
        let mut project = parallel_project();
        let NodeDefinition::Parallel(parallel) =
            project.workflow.nodes.get_mut("implement").unwrap()
        else {
            unreachable!()
        };
        match field {
            "task" => parallel.tasks[0].task = "missing".into(),
            "agent" => parallel.tasks[0].agent = "missing".into(),
            _ => parallel.tasks.clear(),
        }
        let reference = if field == "tasks" {
            "nodes.implement.tasks".into()
        } else {
            format!("nodes.implement.tasks[0].{field}")
        };
        assert_error(&project, &reference, cause);
    }
    let mut project = parallel_project();
    let NodeDefinition::Parallel(parallel) = project.workflow.nodes.get_mut("implement").unwrap()
    else {
        unreachable!()
    };
    parallel.next = Transition::Single("missing".into());
    assert_error(&project, "nodes.implement.next", "unknown target");
}

#[test]
fn task_outcome_routes_must_match_declared_outcomes() {
    for next in [
        Transition::Single("done".into()),
        Transition::Outcomes([("approved".into(), "done".into())].into()),
        Transition::Outcomes(
            [
                ("approved".into(), "done".into()),
                ("changes_requested".into(), "implement".into()),
                ("extra".into(), "done".into()),
            ]
            .into(),
        ),
    ] {
        let mut project = fixture();
        task(&mut project, "review").next = next;
        assert_error(&project, "nodes.review.next", "outcomes");
    }
    let mut project = fixture();
    task(&mut project, "plan").next = Transition::Outcomes([("ok".into(), "done".into())].into());
    assert_error(&project, "nodes.plan.next", "outcomes");
    let mut project = fixture();
    task(&mut project, "plan").next = Transition::Outcomes(Default::default());
    assert_error(&project, "nodes.plan.next", "nonempty");
    let mut project = fixture();
    project
        .definitions
        .task_definitions
        .get_mut("review-implementation")
        .unwrap()
        .outcomes = vec!["approved".into(), "approved".into()];
    assert_error(&project, "nodes.review.next", "outcomes");
}

#[test]
fn gates_require_approved_and_rejected_routes() {
    for routes in [
        [("approved".into(), "implement".into())].into(),
        [("rejected".into(), "plan".into())].into(),
        [
            ("approved".into(), "implement".into()),
            ("rejected".into(), "plan".into()),
            ("other".into(), "done".into()),
        ]
        .into(),
    ] {
        let mut project = fixture();
        let NodeDefinition::HumanGate(gate) =
            project.workflow.nodes.get_mut("approve-plan").unwrap()
        else {
            unreachable!()
        };
        gate.next = routes;
        assert_error(&project, "nodes.approve-plan.next", "approved");
    }
}

#[test]
fn artifacts_require_unique_producers_and_inputs_can_precede_producers() {
    let mut project = fixture();
    task(&mut project, "review").outputs.push("plan.md".into());
    assert_error(&project, "nodes.review.outputs", "producer");
    let mut project = fixture();
    task(&mut project, "plan").inputs.push("absent.md".into());
    assert_error(&project, "nodes.plan.inputs", "no producer");
    let mut project = fixture();
    task(&mut project, "plan").inputs.push("review.md".into());
    assert!(validate_project(&project).is_ok());
    let mut project = fixture();
    task(&mut project, "plan").outputs.push("plan.md".into());
    assert!(
        validate_project(&project).is_ok(),
        "same Node is still one producer"
    );
}

#[test]
fn artifact_names_are_safe_unambiguous_relative_paths() {
    for name in [
        "",
        ".",
        "..",
        "../escape.md",
        "nested/../escape.md",
        "/absolute.md",
        "C:/absolute.md",
        "C:relative.md",
        "\\\\server\\file",
        "nested\\..\\escape",
        "a\0b",
        "./plan.md",
        "nested//file.md",
        "nested/",
        "nested/./file.md",
    ] {
        for field in ["inputs", "outputs"] {
            let mut project = fixture();
            let plan = task(&mut project, "plan");
            if field == "inputs" {
                plan.inputs.push(name.into());
            } else {
                plan.outputs.push(name.into());
            }
            assert_error(&project, &format!("nodes.plan.{field}"), "relative");
        }
    }
    let mut project = fixture();
    task(&mut project, "plan")
        .outputs
        .push("reports/café plan.md".into());
    task(&mut project, "review")
        .inputs
        .push("reports/café plan.md".into());
    assert_eq!(
        validate_project(&project).unwrap()["reports/café plan.md"],
        "plan"
    );
}

#[test]
fn unused_definitions_do_not_resolve_skills_or_mcps() {
    let mut project = fixture();
    let mut unused = project.definitions.agents["engineer"].clone();
    unused.skills = vec!["missing-skill".into()];
    unused.mcps = vec!["missing-mcp".into()];
    project.definitions.agents.insert("unused".into(), unused);
    let mut unused = project.definitions.task_definitions["create-plan"].clone();
    unused.outcomes = vec!["unused-outcome".into()];
    project
        .definitions
        .task_definitions
        .insert("unused".into(), unused);
    assert!(validate_project(&project).is_ok());
}

#[test]
fn loading_still_rejects_invalid_node_combinations() {
    for node in [
        "{task: t, next: done}",
        "{task: t, agent: a}",
        "{type: parallel, next: done}",
        "{type: human-gate, execution: main, next: {approved: done, rejected: done}}",
        "{type: parallel, tasks: [{task: t, agent: a, execution: main}], next: done}",
        "{type: end, next: done}",
        "{type: end, task: t}",
    ] {
        let yaml = format!("name: test\nstart: n\nnodes:\n  n: {node}\n");
        assert!(parse_workflow_definition("invalid.yaml", &yaml).is_err());
    }
}
