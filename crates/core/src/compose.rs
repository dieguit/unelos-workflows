use std::collections::{BTreeMap, BTreeSet};

use camino::{Utf8Path, Utf8PathBuf};
use indexmap::IndexMap;

use crate::{
    NodeDefinition, ProjectLoadError, ProjectValidationError, Transition, WorkflowDefinition,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeSource {
    pub path: Utf8PathBuf,
    pub node_id: String,
}

pub(crate) struct Expanded {
    pub workflow: WorkflowDefinition,
    pub node_sources: BTreeMap<String, NodeSource>,
    pub children: Vec<(
        Utf8PathBuf,
        WorkflowDefinition,
        BTreeMap<String, NodeSource>,
    )>,
}

pub fn artifact_file_name<'a>(node_id: &str, name: &'a str) -> &'a str {
    let mut remaining = name;
    let mut scope = node_id.split('/').peekable();
    while let Some(segment) = scope.next() {
        if scope.peek().is_none() {
            break;
        }
        match remaining
            .strip_prefix(segment)
            .and_then(|s| s.strip_prefix('/'))
        {
            Some(rest) => remaining = rest,
            None => break,
        }
    }
    remaining
}

fn error(path: &Utf8Path, id: &str, field: &str, cause: impl ToString) -> ProjectLoadError {
    ProjectValidationError {
        path: path.to_owned(),
        field: format!("nodes.{id}.{field}"),
        cause: cause.to_string(),
    }
    .into()
}

fn artifacts(node: &NodeDefinition) -> (Vec<String>, Vec<String>) {
    match node {
        NodeDefinition::Task(n) => (n.inputs.clone(), n.outputs.clone()),
        NodeDefinition::Parallel(n) => (n.inputs.clone(), n.outputs.clone()),
        NodeDefinition::HumanGate(n) => (n.inputs.clone(), vec![]),
        NodeDefinition::Workflow(n) => (
            n.inputs.values().cloned().collect(),
            n.outputs.values().cloned().collect(),
        ),
        NodeDefinition::End => (vec![], vec![]),
    }
}

fn rewrite(
    node: &mut NodeDefinition,
    target: &dyn Fn(&str) -> String,
    artifact: &dyn Fn(&str) -> String,
) {
    let transition = |next: &mut Transition| match next {
        Transition::Single(t) => *t = target(t),
        Transition::Outcomes(routes) => {
            for t in routes.values_mut() {
                *t = target(t);
            }
        }
    };
    let (inputs, outputs) = match node {
        NodeDefinition::Task(n) => {
            transition(&mut n.next);
            (&mut n.inputs, Some(&mut n.outputs))
        }
        NodeDefinition::Parallel(n) => {
            transition(&mut n.next);
            (&mut n.inputs, Some(&mut n.outputs))
        }
        NodeDefinition::HumanGate(n) => {
            for t in n.next.values_mut() {
                *t = target(t);
            }
            (&mut n.inputs, None)
        }
        _ => return,
    };
    for name in inputs.iter_mut().chain(outputs.into_iter().flatten()) {
        *name = artifact(name);
    }
}

pub(crate) fn expand(
    source: &Utf8Path,
    root: &WorkflowDefinition,
    load_child: &mut dyn FnMut(&str) -> Result<(Utf8PathBuf, WorkflowDefinition), ProjectLoadError>,
) -> Result<Expanded, ProjectLoadError> {
    expand_inner(source, root, load_child, &mut vec![root.name.clone()])
}

fn expand_inner(
    source: &Utf8Path,
    root: &WorkflowDefinition,
    load_child: &mut dyn FnMut(&str) -> Result<(Utf8PathBuf, WorkflowDefinition), ProjectLoadError>,
    stack: &mut Vec<String>,
) -> Result<Expanded, ProjectLoadError> {
    let mut workflow = root.clone();
    let mut nodes = IndexMap::new();
    let mut node_sources = BTreeMap::new();
    let mut entries = BTreeMap::new();
    let mut children = Vec::new();
    for (id, node) in &root.nodes {
        if id.contains('/') {
            return Err(error(
                source,
                id,
                "workflow",
                "authored node ID cannot contain /",
            ));
        }
        let NodeDefinition::Workflow(call) = node else {
            nodes.insert(id.clone(), node.clone());
            node_sources.insert(
                id.clone(),
                NodeSource {
                    path: source.to_owned(),
                    node_id: id.clone(),
                },
            );
            continue;
        };
        if stack.contains(&call.workflow) {
            let mut chain = stack.clone();
            chain.push(call.workflow.clone());
            return Err(error(
                source,
                id,
                "workflow",
                format!("workflow cycle: {}", chain.join(" -> ")),
            ));
        }
        let (path, child) = load_child(&call.workflow).map_err(|e| {
            error(
                source,
                id,
                "workflow",
                format!("cannot load {:?}: {e}", call.workflow),
            )
        })?;
        stack.push(call.workflow.clone());
        let expanded = expand_inner(&path, &child, load_child, stack)?;
        stack.pop();
        children.extend(expanded.children);
        children.push((
            path.clone(),
            expanded.workflow.clone(),
            expanded.node_sources.clone(),
        ));
        let child = expanded.workflow;
        if matches!(child.nodes.get(&child.start), Some(NodeDefinition::End)) {
            return Err(error(source, id, "workflow", "child start is an end node"));
        }
        let inputs: BTreeSet<_> = child.inputs.iter().collect();
        if call.inputs.keys().collect::<BTreeSet<_>>() != inputs {
            return Err(error(
                source,
                id,
                "inputs",
                format!("keys must match child inputs {:?}", child.inputs),
            ));
        }
        let produced: BTreeSet<_> = child.nodes.values().flat_map(|n| artifacts(n).1).collect();
        for name in call.outputs.keys() {
            if !produced.contains(name) {
                return Err(error(
                    source,
                    id,
                    "outputs",
                    format!("no child producer for {name:?}"),
                ));
            }
        }
        let ends: BTreeSet<_> = child
            .nodes
            .iter()
            .filter_map(|(n, v)| matches!(v, NodeDefinition::End).then_some(n.clone()))
            .collect();
        match &call.next {
            Transition::Single(_) if ends.len() != 1 => {
                return Err(error(
                    source,
                    id,
                    "next",
                    "single next requires exactly one child end",
                ))
            }
            Transition::Outcomes(routes)
                if routes.keys().cloned().collect::<BTreeSet<_>>() != ends =>
            {
                return Err(error(
                    source,
                    id,
                    "next",
                    format!("routes must match child outcomes {ends:?}"),
                ))
            }
            _ => {}
        }
        let targets: Vec<_> = match &call.next {
            Transition::Single(target) => vec![target],
            Transition::Outcomes(routes) => routes.values().collect(),
        };
        for target in targets {
            if !root.nodes.contains_key(target) {
                return Err(error(
                    source,
                    id,
                    "next",
                    format!("unknown parent target {target:?}"),
                ));
            }
        }
        let prefix = format!("{id}/");
        if root
            .inputs
            .iter()
            .cloned()
            .chain(root.nodes.values().flat_map(|n| {
                let (i, o) = artifacts(n);
                i.into_iter().chain(o)
            }))
            .any(|name| name.starts_with(&prefix))
        {
            return Err(error(
                source,
                id,
                "outputs",
                format!("parent artifact uses reserved scope {prefix}"),
            ));
        }
        let available: BTreeSet<_> = root
            .inputs
            .iter()
            .cloned()
            .chain(root.nodes.values().flat_map(|n| artifacts(n).1))
            .collect();
        for name in call.inputs.values() {
            if !available.contains(name) {
                return Err(error(
                    source,
                    id,
                    "inputs",
                    format!("no parent producer or declared input for {name:?}"),
                ));
            }
        }
        entries.insert(id.clone(), format!("{id}/{}", child.start));
        let target = |t: &str| {
            if ends.contains(t) {
                match &call.next {
                    Transition::Single(t) => t.clone(),
                    Transition::Outcomes(routes) => routes[t].clone(),
                }
            } else {
                format!("{id}/{t}")
            }
        };
        let artifact = |name: &str| {
            call.inputs
                .get(name)
                .or_else(|| call.outputs.get(name))
                .cloned()
                .unwrap_or_else(|| format!("{id}/{name}"))
        };
        for (n, mut node) in child.nodes {
            if matches!(node, NodeDefinition::End) {
                continue;
            }
            rewrite(&mut node, &target, &artifact);
            let scoped = format!("{id}/{n}");
            let outputs = artifacts(&node).1;
            let mut files = BTreeSet::new();
            for output in &outputs {
                if !files.insert(artifact_file_name(&scoped, output)) {
                    return Err(error(
                        source,
                        id,
                        "outputs",
                        "outputs resolve to the same file name",
                    ));
                }
            }
            node_sources.insert(scoped.clone(), expanded.node_sources[&n].clone());
            nodes.insert(scoped, node);
        }
    }
    let target = |t: &str| entries.get(t).cloned().unwrap_or_else(|| t.to_owned());
    for node in nodes.values_mut() {
        rewrite(node, &target, &str::to_owned);
    }
    workflow.start = target(&root.start);
    workflow.nodes = nodes;
    Ok(Expanded {
        workflow,
        node_sources,
        children,
    })
}
