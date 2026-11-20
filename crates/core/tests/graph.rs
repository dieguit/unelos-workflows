use camino::Utf8Path;
use unelos_core::{
    build_graph, load_project, validate_project, GraphEdge, GraphNode, LoadedProject,
    NodeDefinition, NodeKind, ParallelNode, Transition, WorkerReference,
};

fn fixture() -> LoadedProject {
    load_project(
        &Utf8Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/project"),
        "implement-feature",
    )
    .unwrap()
}

fn edge(from: &str, to: &str, outcome: Option<&str>) -> GraphEdge {
    GraphEdge {
        from: from.into(),
        to: to.into(),
        outcome: outcome.map(str::to_owned),
    }
}

#[test]
fn fixture_has_exact_topology_and_serialization() {
    let project = fixture();
    validate_project(&project).unwrap();
    let graph = build_graph(&project.workflow);
    assert_eq!(graph.start, "plan");
    assert_eq!(
        graph.nodes,
        vec![
            GraphNode {
                id: "approve-plan".into(),
                kind: NodeKind::HumanGate,
                workflow: None,
            },
            GraphNode {
                id: "done".into(),
                kind: NodeKind::End,
                workflow: None,
            },
            GraphNode {
                id: "implement".into(),
                kind: NodeKind::Task,
                workflow: None,
            },
            GraphNode {
                id: "plan".into(),
                kind: NodeKind::Task,
                workflow: None,
            },
            GraphNode {
                id: "review".into(),
                kind: NodeKind::Task,
                workflow: None,
            },
        ]
    );
    assert_eq!(
        graph.edges,
        vec![
            edge("approve-plan", "implement", Some("approved")),
            edge("approve-plan", "plan", Some("rejected")),
            edge("implement", "review", None),
            edge("plan", "approve-plan", None),
            edge("review", "done", Some("approved")),
            edge("review", "implement", Some("changes_requested")),
        ]
    );
    assert_eq!(
        graph.edges.iter().filter(|e| e.to == "implement").count(),
        2
    );
    assert!(!graph.edges.iter().any(|e| e.from == "done"));
    let value = serde_json::to_value(&graph).unwrap();
    assert_eq!(
        value,
        serde_json::json!({
            "start": "plan",
            "nodes": [
                {"id":"approve-plan","kind":"human-gate"},
                {"id":"done","kind":"end"},
                {"id":"implement","kind":"task"},
                {"id":"plan","kind":"task"},
                {"id":"review","kind":"task"}
            ],
            "edges": [
                {"from":"approve-plan","to":"implement","outcome":"approved"},
                {"from":"approve-plan","to":"plan","outcome":"rejected"},
                {"from":"implement","to":"review","outcome":null},
                {"from":"plan","to":"approve-plan","outcome":null},
                {"from":"review","to":"done","outcome":"approved"},
                {"from":"review","to":"implement","outcome":"changes_requested"}
            ]
        })
    );
    let mut reordered = project.workflow.clone();
    reordered.nodes = reordered.nodes.into_iter().rev().collect();
    assert_eq!(
        serde_json::to_string(&graph).unwrap(),
        serde_json::to_string(&build_graph(&reordered)).unwrap()
    );
}

#[test]
fn artifacts_do_not_create_edges_and_all_declared_nodes_survive() {
    let mut project = fixture();
    validate_project(&project).unwrap();
    let baseline = build_graph(&project.workflow);
    let NodeDefinition::Task(plan) = project.workflow.nodes.get_mut("plan").unwrap() else {
        panic!()
    };
    plan.inputs.push("review.md".into());
    plan.outputs.push("extra.md".into());
    validate_project(&project).unwrap();
    assert_eq!(build_graph(&project.workflow), baseline);
    let NodeDefinition::Task(plan) = project.workflow.nodes.get_mut("plan").unwrap() else {
        panic!()
    };
    plan.next = Transition::Single("plan".into());
    project
        .workflow
        .nodes
        .insert("unreachable".into(), NodeDefinition::End);
    validate_project(&project).unwrap();
    let graph = build_graph(&project.workflow);
    assert!(graph.edges.contains(&edge("plan", "plan", None)));
    assert!(graph.nodes.contains(&GraphNode {
        id: "unreachable".into(),
        kind: NodeKind::End,
        workflow: None,
    }));
}

#[test]
fn parallel_stays_one_node_for_single_and_labeled_routes() {
    let mut project = fixture();
    project.workflow.nodes.insert(
        "implement".into(),
        NodeDefinition::Parallel(ParallelNode {
            tasks: vec![
                WorkerReference {
                    task: "implement-plan".into(),
                    agent: "engineer".into(),
                },
                WorkerReference {
                    task: "create-plan".into(),
                    agent: "planner".into(),
                },
            ],
            inputs: vec!["plan.md".into()],
            outputs: vec!["implementation-report.md".into()],
            next: Transition::Single("review".into()),
        }),
    );
    validate_project(&project).unwrap();
    let graph = build_graph(&project.workflow);
    assert_eq!(graph.nodes.len(), 5);
    assert_eq!(
        graph
            .nodes
            .iter()
            .map(|node| node.id.as_str())
            .collect::<Vec<_>>(),
        vec!["approve-plan", "done", "implement", "plan", "review"]
    );
    let single_route_nodes = graph.nodes.clone();
    assert!(graph.nodes.contains(&GraphNode {
        id: "implement".into(),
        kind: NodeKind::Parallel,
        workflow: None,
    }));
    assert!(graph.edges.contains(&edge("implement", "review", None)));
    assert_eq!(
        serde_json::to_value(NodeKind::Parallel).unwrap(),
        "parallel"
    );
    let NodeDefinition::Parallel(parallel) = project.workflow.nodes.get_mut("implement").unwrap()
    else {
        panic!()
    };
    parallel.next =
        Transition::Outcomes([("z".into(), "review".into()), ("a".into(), "review".into())].into());
    assert_eq!(
        validate_project(&project).unwrap_err().field,
        "nodes.implement.next"
    );
    let graph = build_graph(&project.workflow);
    assert_eq!(graph.nodes.len(), 5);
    assert_eq!(graph.nodes, single_route_nodes);
    assert_eq!(
        graph
            .edges
            .iter()
            .filter(|e| e.from == "implement")
            .cloned()
            .collect::<Vec<_>>(),
        vec![
            edge("implement", "review", Some("a")),
            edge("implement", "review", Some("z")),
        ]
    );
}
