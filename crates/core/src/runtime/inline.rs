use std::collections::{BTreeMap, BTreeSet};

use camino::Utf8Path;
use indexmap::IndexMap;
use serde::Deserialize;

use crate::{
    ExecutionError, ExecutionPackage, HumanGateNode, NodeDefinition, RuntimeError, Transition,
    WorkflowDefinition, WorkflowNode,
};

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(untagged, deny_unknown_fields, rename_all_fields = "camelCase")]
pub enum InlineStep {
    Workflow {
        workflow: String,
        #[serde(default)]
        inputs: IndexMap<String, String>,
        #[serde(default)]
        stop_on: Vec<String>,
    },
    Gate {
        gate: String,
    },
}

impl InlineStep {
    pub fn workflow(workflow: impl Into<String>) -> Self {
        Self::Workflow {
            workflow: workflow.into(),
            inputs: IndexMap::new(),
            stop_on: vec![],
        }
    }
}

pub fn execution_start_inline(
    root: &Utf8Path,
    steps: &[InlineStep],
    name: Option<&str>,
    request: &str,
) -> Result<ExecutionPackage, ExecutionError> {
    let (definition, label) = synthesize(root, steps)?;
    let text = serde_yaml_ng::to_string(&definition)
        .map_err(|e| invalid(format!("cannot serialize inline workflow: {e}")))?;
    super::execution::start_project(root, name, request, |run_id| {
        let project = crate::project::load_inline_project(root, run_id, Some(&text))
            .map_err(RuntimeError::from)?;
        Ok((project, Some((label, text))))
    })
}

fn invalid(message: impl Into<String>) -> ExecutionError {
    ExecutionError::protocol("invalid_steps", message)
}

fn synthesize(
    root: &Utf8Path,
    steps: &[InlineStep],
) -> Result<(WorkflowDefinition, String), ExecutionError> {
    let mut children = BTreeMap::new();
    for step in steps {
        if let InlineStep::Workflow { workflow, .. } = step {
            if !children.contains_key(workflow) {
                let project = crate::load_project(root, workflow)
                    .map_err(|e| invalid(format!("cannot load workflow {workflow:?}: {e}")))?;
                children.insert(workflow, project.workflow);
            }
        }
    }
    if children.is_empty() {
        return Err(invalid("inline run requires at least one workflow step"));
    }

    let stem = |workflow: &str| {
        workflow
            .strip_prefix("global/")
            .unwrap_or(workflow)
            .to_owned()
    };
    let mut gates = 0;
    let mut reserved: BTreeSet<_> = [
        "done",
        "stopped",
        "workflow.yaml",
        "state.json",
        "ledger.md",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect();
    reserved.extend(
        (1..=steps
            .iter()
            .filter(|s| matches!(s, InlineStep::Gate { .. }))
            .count())
            .map(|n| format!("gate-{n}")),
    );
    let stems: BTreeSet<_> = children.keys().map(|workflow| stem(workflow)).collect();
    let mut used = reserved.clone();
    let ids: Vec<_> = steps
        .iter()
        .map(|step| match step {
            InlineStep::Workflow { workflow, .. } => {
                let base = stem(workflow);
                if used.insert(base.clone()) {
                    return base;
                }
                (2..)
                    .map(|n| format!("{base}-{n}"))
                    .find(|id| !stems.contains(id) && used.insert(id.clone()))
                    .expect("finite workflow IDs")
            }
            InlineStep::Gate { .. } => {
                gates += 1;
                format!("gate-{gates}")
            }
        })
        .collect();

    let mut inputs = BTreeSet::new();
    let mut available = BTreeSet::new();
    let mut latest = BTreeMap::new();
    let mut nodes = IndexMap::new();
    let mut stopped = false;
    let mut labels = Vec::new();
    for (index, step) in steps.iter().enumerate() {
        let id = &ids[index];
        let next = ids.get(index + 1).map(String::as_str).unwrap_or("done");
        let node = match step {
            InlineStep::Workflow {
                workflow,
                inputs: overrides,
                stop_on,
            } => {
                labels.push(workflow.as_str());
                let child = &children[workflow];
                for (name, source) in overrides {
                    if !child.inputs.contains(name) {
                        return Err(invalid(format!("step {id:?}: unknown input {name:?}")));
                    }
                    if !available.contains(source) && !inputs.contains(source) {
                        return Err(invalid(format!("step {id:?}: input {name:?} has no earlier output or declared input {source:?}")));
                    }
                }
                let bindings = child
                    .inputs
                    .iter()
                    .map(|name| {
                        let source = overrides
                            .get(name)
                            .or_else(|| latest.get(name))
                            .cloned()
                            .unwrap_or_else(|| {
                                inputs.insert(name.clone());
                                name.clone()
                            });
                        (name.clone(), source)
                    })
                    .collect();
                let routes: IndexMap<_, _> = child
                    .nodes
                    .iter()
                    .filter(|(_, node)| matches!(node, NodeDefinition::End))
                    .map(|(outcome, _)| {
                        let target = if stop_on.contains(outcome) {
                            "stopped"
                        } else {
                            next
                        };
                        (outcome.clone(), target.to_owned())
                    })
                    .collect();
                for outcome in stop_on {
                    if !routes.contains_key(outcome) {
                        return Err(invalid(format!(
                            "step {id:?}: unknown stopOn outcome {outcome:?}"
                        )));
                    }
                }
                stopped |= !stop_on.is_empty();
                for output in crate::compose::produced_artifacts(child) {
                    let scoped = format!("{id}/{output}");
                    latest.insert(
                        output.rsplit('/').next().unwrap().to_owned(),
                        scoped.clone(),
                    );
                    available.insert(scoped);
                }
                NodeDefinition::Workflow(WorkflowNode {
                    workflow: workflow.clone(),
                    inputs: bindings,
                    outputs: IndexMap::new(),
                    next: Transition::Outcomes(routes),
                })
            }
            InlineStep::Gate { gate } => {
                labels.push("gate");
                stopped = true;
                NodeDefinition::HumanGate(HumanGateNode {
                    description: Some(gate.clone()),
                    inputs: vec![],
                    next: IndexMap::from([
                        ("approved".into(), next.into()),
                        ("rejected".into(), "stopped".into()),
                    ]),
                })
            }
        };
        nodes.insert(id.clone(), node);
    }
    nodes.insert("done".into(), NodeDefinition::End);
    if stopped {
        nodes.insert("stopped".into(), NodeDefinition::End);
    }
    Ok((
        WorkflowDefinition {
            name: "inline/run".into(),
            description: None,
            start: ids[0].clone(),
            inputs: inputs.into_iter().collect(),
            nodes,
        },
        labels.join(" → "),
    ))
}
