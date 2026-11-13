use serde::Serialize;

use crate::{NodeDefinition, Transition, WorkflowDefinition};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct WorkflowGraph {
    pub start: String,
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GraphNode {
    pub id: String,
    pub kind: NodeKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum NodeKind {
    Task,
    Parallel,
    HumanGate,
    End,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GraphEdge {
    pub from: String,
    pub to: String,
    pub outcome: Option<String>,
}

/// Derive topology after the caller successfully validates the loaded project.
/// Nodes are ordered by ID; edges by source ID and outcome label. Definitions
/// retain all configuration. Inputs and parallel workers create no edges/nodes.
pub fn build_graph(workflow: &WorkflowDefinition) -> WorkflowGraph {
    let mut graph = WorkflowGraph {
        start: workflow.start.clone(),
        nodes: Vec::new(),
        edges: Vec::new(),
    };
    for (id, node) in &workflow.nodes {
        let (kind, next) = match node {
            NodeDefinition::Task(task) => (NodeKind::Task, Some(&task.next)),
            NodeDefinition::Parallel(parallel) => (NodeKind::Parallel, Some(&parallel.next)),
            NodeDefinition::HumanGate(_) => (NodeKind::HumanGate, None),
            NodeDefinition::End => (NodeKind::End, None),
        };
        graph.nodes.push(GraphNode {
            id: id.clone(),
            kind,
        });
        let mut push = |to: &String, outcome: Option<&String>| {
            graph.edges.push(GraphEdge {
                from: id.clone(),
                to: to.clone(),
                outcome: outcome.cloned(),
            });
        };
        match next {
            Some(Transition::Single(to)) => push(to, None),
            Some(Transition::Outcomes(routes)) => {
                for (outcome, to) in routes {
                    push(to, Some(outcome));
                }
            }
            None => {
                if let NodeDefinition::HumanGate(gate) = node {
                    for (outcome, to) in &gate.next {
                        push(to, Some(outcome));
                    }
                }
            }
        }
    }
    graph.nodes.sort_by(|a, b| a.id.cmp(&b.id));
    graph
        .edges
        .sort_by(|a, b| (&a.from, &a.outcome).cmp(&(&b.from, &b.outcome)));
    graph
}
