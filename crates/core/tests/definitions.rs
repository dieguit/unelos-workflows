use unelos_core::{
    parse_project_definitions, parse_workflow_definition, Execution, NodeDefinition, Transition,
};

#[test]
fn project_preserves_authored_values_and_defaults() {
    let project = parse_project_definitions(
        ".unelos/project.yaml",
        include_str!("fixtures/definitions/project.yaml"),
    )
    .unwrap();
    let task = &project.task_definitions["implement-plan"];
    assert_eq!(
        task.description.as_deref(),
        Some("Implement the supplied plan.")
    );
    assert_eq!(task.instructions, "Read the plan.\nVerify the changes.\n");
    assert_eq!(task.skills, ["planning", "verification"]);
    assert_eq!(task.outcomes, ["approved", "changes_requested"]);
    let agent = &project.agents["engineer"];
    assert_eq!(agent.instructions, "Follow project conventions.");
    assert_eq!(agent.skills, ["rust", "verification"]);
    assert_eq!(agent.model.as_deref(), Some("vendor/model#variant"));
    assert_eq!(agent.mcps, ["github", "local"]);
    assert_eq!(agent.tools, ["read", "shell"]);
    let task = &project.task_definitions["minimal"];
    assert!(task.description.is_none());
    assert!(task.skills.is_empty() && task.outcomes.is_empty());
    let agent = &project.agents["minimal"];
    assert!(agent.model.is_none());
    assert!(agent.skills.is_empty() && agent.mcps.is_empty() && agent.tools.is_empty());
    let empty = parse_project_definitions("empty", "{}").unwrap();
    assert!(empty.task_definitions.is_empty() && empty.agents.is_empty());
}

#[test]
fn project_rejects_malformed_or_discarded_configuration() {
    for (yaml, detail) in [
        ("taskDefinitons: {}", "taskDefinitons"),
        (
            "taskDefinitions: {x: {instructions: work, skilss: []}}",
            "skilss",
        ),
        ("agents: {x: {instructions: work, tool: []}}", "tool"),
        ("taskDefinitions: {x: {skills: []}}", "instructions"),
        ("agents: {x: {skills: []}}", "instructions"),
        ("agents: {x: {instructions: work, skills: null}}", ""),
        ("agents: {x: {instructions: work, mcps: github}}", ""),
        ("agents: {x: {instructions: work, tools: [12]}}", ""),
        ("agents: {x: {instructions: work, model: false}}", ""),
        ("taskDefinitions: {x: {instructions: null}}", ""),
        ("taskDefinitions: {x: {instructions: 42}}", ""),
        ("agents: null", ""),
        ("agents: {}\nagents: {}", ""),
        (
            "agents: {x: {instructions: first}, x: {instructions: second}}",
            "",
        ),
        ("{}\n---\nagents: {}", ""),
        ("agents: [", ""),
    ] {
        let error = match parse_project_definitions("broken-project.yaml", yaml) {
            Ok(_) => panic!("accepted invalid YAML: {yaml}"),
            Err(error) => error,
        };
        let rendered = error.to_string();
        assert!(rendered.contains("broken-project.yaml"), "{rendered}");
        assert!(rendered.contains(detail), "{rendered}");
    }
}

#[test]
fn project_rejects_non_string_definition_ids() {
    for yaml in [
        "taskDefinitions: {12: {instructions: Work}}",
        "taskDefinitions: {true: {instructions: Work}}",
        "agents: {12: {instructions: Work}}",
        "agents: {true: {instructions: Work}}",
    ] {
        assert!(
            parse_project_definitions("broken-project.yaml", yaml).is_err(),
            "accepted non-string definition ID: {yaml}"
        );
    }
}

#[test]
fn workflow_parses_all_node_kinds_and_transition_forms() {
    let workflow = parse_workflow_definition(
        "workflow.yaml",
        include_str!("fixtures/definitions/workflow.yaml"),
    )
    .unwrap();
    assert_eq!(workflow.name, "implement-feature");
    assert_eq!(workflow.start, "plan");
    assert_eq!(workflow.nodes.len(), 6);
    let NodeDefinition::Task(plan) = &workflow.nodes["plan"] else {
        panic!("task expected")
    };
    assert_eq!(plan.task, "create-plan");
    assert_eq!(plan.agent, "planner");
    assert_eq!(plan.execution, Execution::Main);
    assert!(plan.inputs.is_empty());
    assert_eq!(plan.outputs, ["plan.md"]);
    assert_eq!(plan.next, Transition::Single("approve-plan".into()));
    let NodeDefinition::Task(work) = &workflow.nodes["implement"] else {
        panic!("task expected")
    };
    assert_eq!(work.execution, Execution::Subagent);
    assert_eq!(work.inputs, ["plan.md"]);
    assert_eq!(work.outputs, ["implementation-report.md"]);
    let NodeDefinition::HumanGate(gate) = &workflow.nodes["approve-plan"] else {
        panic!("gate expected")
    };
    assert_eq!(gate.inputs, ["plan.md"]);
    assert_eq!(gate.next["approved"], "implement");
    assert_eq!(gate.next["rejected"], "plan");
    let NodeDefinition::Parallel(parallel) = &workflow.nodes["parallel-check"] else {
        panic!("parallel expected")
    };
    assert_eq!(parallel.inputs, ["implementation-report.md"]);
    assert_eq!(parallel.outputs, ["checks.md"]);
    assert_eq!(parallel.tasks[0].task, "check-api");
    assert_eq!(parallel.tasks[0].agent, "backend");
    assert_eq!(parallel.tasks[1].task, "check-ui");
    assert_eq!(parallel.tasks[1].agent, "frontend");
    assert_eq!(parallel.next, Transition::Single("review".into()));
    let NodeDefinition::Task(review) = &workflow.nodes["review"] else {
        panic!("task expected")
    };
    assert_eq!(review.execution, Execution::Main);
    assert!(review.outputs.is_empty());
    let Transition::Outcomes(routes) = &review.next else {
        panic!("routes expected")
    };
    assert_eq!(routes["approved"], "done");
    assert_eq!(routes["changes_requested"], "implement");
    assert_eq!(workflow.nodes["done"], NodeDefinition::End);
}

#[test]
fn workflow_requires_top_level_fields_and_rejects_typos() {
    for (yaml, field) in [
        ("start: x\nnodes: {}", "name"),
        ("name: x\nnodes: {}", "start"),
        ("name: x\nstart: x", "nodes"),
        ("name: x\nstart: x\nnodes: {}\nnode: {}", "node"),
    ] {
        let error = parse_workflow_definition("bad-workflow.yaml", yaml)
            .unwrap_err()
            .to_string();
        assert!(
            error.contains("bad-workflow.yaml") && error.contains(field),
            "{error}"
        );
    }
}

#[test]
fn workflow_rejects_invalid_node_configuration() {
    for (node, detail) in [
        ("{task: t, agent: a, next: x, type: mystery}", "mystery"),
        ("{agent: a, next: x}", "type"),
        (
            "{task: t, agent: a, next: x, execution: background}",
            "background",
        ),
        ("{task: t, next: x}", "agent"),
        ("{task: t, agent: a}", "next"),
        ("{task: t, agent: a, next: x, ouputs: []}", "ouputs"),
        ("{type: parallel, tasks: [{task: t}], next: x}", "agent"),
        (
            "{type: parallel, tasks: [{task: t, agent: a, execution: main}], next: x}",
            "execution",
        ),
        ("{type: human-gate, next: x}", ""),
        ("{type: human-gate, next: {}, outputs: []}", "outputs"),
        ("{type: end, task: t}", "task"),
        ("{type: end, execution: main}", "execution"),
        ("{type: end, inputs: []}", "inputs"),
        ("{type: end, outputs: []}", "outputs"),
        ("{type: end, next: x}", "next"),
        ("{task: t, agent: a, next: [x]}", "next"),
        ("{task: t, agent: a, next: {approved: [x]}}", "next"),
        ("{task: t, agent: a, next: 12}", "next"),
        ("{task: t, agent: a, next: x, inputs: null}", ""),
        ("{task: t, agent: a, next: x, outputs: [false]}", ""),
        ("{type: parallel, tasks: null, next: x}", ""),
        ("{type: null, task: t, agent: a, next: x}", ""),
    ] {
        let yaml = format!("name: test\nstart: x\nnodes:\n  x: {node}\n");
        let error = match parse_workflow_definition("bad-node.yaml", &yaml) {
            Ok(_) => panic!("accepted node: {node}"),
            Err(error) => error.to_string(),
        };
        assert!(error.contains("bad-node.yaml"), "{error}");
        assert!(error.contains(detail), "{error}");
    }
}

#[test]
fn workflow_defaults_do_not_perform_semantic_validation() {
    let yaml = "name: ''\nstart: absent\nnodes:\n  p: {type: parallel, tasks: [], next: {}}\n  g: {type: human-gate, next: {custom: absent}}";
    let workflow = parse_workflow_definition("structural-only", yaml).unwrap();
    let NodeDefinition::Parallel(p) = &workflow.nodes["p"] else {
        panic!("parallel expected")
    };
    assert!(p.inputs.is_empty() && p.outputs.is_empty() && p.tasks.is_empty());
    let NodeDefinition::HumanGate(g) = &workflow.nodes["g"] else {
        panic!("gate expected")
    };
    assert!(g.inputs.is_empty());
    assert_eq!(g.next["custom"], "absent");
}

#[test]
fn workflow_rejects_non_string_and_duplicate_map_keys() {
    for yaml in [
        "name: x\nstart: n\nnodes: {12: {type: end}}",
        "name: x\nstart: n\nnodes: {n: {task: t, agent: a, next: {12: done}}}",
        "name: x\nstart: n\nnodes: {n: {task: t, agent: a, next: {same: one, same: two}}}",
        "name: x\nstart: n\nnodes: {n: {type: human-gate, next: {same: one, same: two}}}",
        "name: x\nstart: n\nnodes: {n: {type: end}, n: {type: end}}",
        "name: x\nstart: n\nnodes: {n: {task: first, task: second, agent: a, next: x}}",
    ] {
        assert!(
            parse_workflow_definition("bad-workflow.yaml", yaml).is_err(),
            "accepted malformed key: {yaml}"
        );
    }
}

#[test]
fn workflow_rejects_non_string_worker_fields_and_non_list_workers() {
    for node in [
        "{type: parallel, tasks: [{task: 12, agent: a}], next: x}",
        "{type: parallel, tasks: [{task: t, agent: false}], next: x}",
        "{type: parallel, tasks: {task: t, agent: a}, next: x}",
        "{type: parallel, tasks: [{task: t, agent: a}, null], next: x}",
    ] {
        let yaml = format!("name: x\nstart: n\nnodes:\n  n: {node}\n");
        assert!(
            parse_workflow_definition("bad-workflow.yaml", &yaml).is_err(),
            "accepted malformed worker: {node}"
        );
    }
}
