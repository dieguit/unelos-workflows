use camino::Utf8PathBuf;
use std::collections::{BTreeMap, BTreeSet};

use crate::{LoadedProject, NodeDefinition, Transition};

#[derive(Debug, thiserror::Error)]
#[error("{path}: {field}: {cause}")]
pub struct ProjectValidationError {
    pub path: Utf8PathBuf,
    pub field: String,
    pub cause: String,
}

/// Validate only the selected Workflow and return logical output names mapped
/// to their producing Node IDs. Does not read Skills, connect MCPs, inspect
/// artifacts, or require an acyclic graph or all predecessors to complete.
/// The loader/parser already enforces structural Node field combinations.
pub fn validate_project(
    project: &LoadedProject,
) -> Result<BTreeMap<String, String>, ProjectValidationError> {
    let error = |field: &str, cause: String| ProjectValidationError {
        path: project.workflow_source.clone(),
        field: field.into(),
        cause,
    };
    if !project.workflow.nodes.contains_key(&project.workflow.start) {
        return Err(error(
            "start",
            format!("unknown target {:?}", project.workflow.start),
        ));
    }
    let mut producers = BTreeMap::new();
    let mut consumers = Vec::new();
    for (id, node) in &project.workflow.nodes {
        let field = format!("nodes.{id}");
        let (inputs, outputs) = match node {
            NodeDefinition::Task(task) => {
                validate_references(project, &field, &task.task, &task.agent)?;
                validate_transition(project, &field, &task.next)?;
                let (definition, identity) = project.selected_task(&task.task, &field)?;
                let outcomes = &definition.outcomes;
                let matches = match &task.next {
                    Transition::Single(_) => outcomes.is_empty(),
                    Transition::Outcomes(routes) => {
                        routes.len() == outcomes.len()
                            && routes.keys().collect::<BTreeSet<_>>() == outcomes.iter().collect()
                    }
                };
                if !matches {
                    return Err(error(&format!("{field}.next"), format!(
                        "routes must match Task Definition {:?} outcomes {:?}; declared outcomes require an outcome map",
                        identity, outcomes
                    )));
                }
                (&task.inputs, &task.outputs[..])
            }
            NodeDefinition::Parallel(parallel) => {
                if parallel.tasks.is_empty() {
                    return Err(error(
                        &format!("{field}.tasks"),
                        "expected a nonempty worker list".into(),
                    ));
                }
                for (index, worker) in parallel.tasks.iter().enumerate() {
                    validate_references(
                        project,
                        &format!("{field}.tasks[{index}]"),
                        &worker.task,
                        &worker.agent,
                    )?;
                    let worker_field = format!("{field}.tasks[{index}]");
                    let (definition, _) = project.selected_task(&worker.task, &worker_field)?;
                    if !definition.outcomes.is_empty() {
                        return Err(error(
                            &format!("{worker_field}.task"),
                            "parallel worker Tasks must declare no outcomes".into(),
                        ));
                    }
                }
                if !matches!(parallel.next, Transition::Single(_)) {
                    return Err(error(
                        &format!("{field}.next"),
                        "parallel next must be a single successor".into(),
                    ));
                }
                validate_transition(project, &field, &parallel.next)?;
                (&parallel.inputs, &parallel.outputs[..])
            }
            NodeDefinition::HumanGate(gate) => {
                if gate.next.len() != 2
                    || !gate.next.contains_key("approved")
                    || !gate.next.contains_key("rejected")
                {
                    return Err(error(
                        &format!("{field}.next"),
                        "expected exactly approved and rejected routes".into(),
                    ));
                }
                for (outcome, target) in &gate.next {
                    validate_target(project, &format!("{field}.next.{outcome}"), target)?;
                }
                (&gate.inputs, &[][..])
            }
            NodeDefinition::End => continue,
        };
        for (kind, names) in [("inputs", &inputs[..]), ("outputs", outputs)] {
            for name in names {
                validate_artifact(project, &format!("{field}.{kind}"), name)?;
            }
        }
        for name in outputs {
            if let Some(previous) = producers.insert(name.clone(), id.clone()) {
                if previous != *id {
                    return Err(error(
                        &format!("{field}.outputs"),
                        format!(
                            "artifact {name:?} has conflicting producers {previous:?} and {id:?}"
                        ),
                    ));
                }
            }
        }
        consumers.push((field, inputs));
    }
    // Producers can appear later in map order or on a rework path. Runtime
    // checks whether an input's producer has actually completed on the path taken.
    for (field, inputs) in consumers {
        for name in inputs {
            if !producers.contains_key(name) {
                return Err(error(
                    &format!("{field}.inputs"),
                    format!("no producer for {name:?}"),
                ));
            }
        }
    }
    Ok(producers)
}

fn validate_references(
    project: &LoadedProject,
    field: &str,
    task: &crate::TaskSelection,
    agent: &str,
) -> Result<(), ProjectValidationError> {
    project.selected_task(task, field)?;
    if !project.definitions.agents.contains_key(agent) {
        return Err(ProjectValidationError {
            path: project.workflow_source.clone(),
            field: format!("{field}.agent"),
            cause: if agent.trim().is_empty() { "Pick an agent".into() } else { format!("unknown Agent {agent:?}") },
        });
    }
    Ok(())
}

fn validate_transition(
    project: &LoadedProject,
    field: &str,
    next: &Transition,
) -> Result<(), ProjectValidationError> {
    match next {
        Transition::Single(target) => validate_target(project, &format!("{field}.next"), target),
        Transition::Outcomes(routes) => {
            if routes.is_empty() {
                return Err(ProjectValidationError {
                    path: project.workflow_source.clone(),
                    field: format!("{field}.next"),
                    cause: "expected a nonempty outcome map".into(),
                });
            }
            for (outcome, target) in routes {
                validate_target(project, &format!("{field}.next.{outcome}"), target)?;
            }
            Ok(())
        }
    }
}

fn validate_target(
    project: &LoadedProject,
    field: &str,
    target: &str,
) -> Result<(), ProjectValidationError> {
    if !project.workflow.nodes.contains_key(target) {
        return Err(ProjectValidationError {
            path: project.workflow_source.clone(),
            field: field.into(),
            cause: format!("unknown target {target:?}"),
        });
    }
    Ok(())
}

fn validate_artifact(
    project: &LoadedProject,
    field: &str,
    name: &str,
) -> Result<(), ProjectValidationError> {
    // Use one portable spelling, avoiding traversal and aliases such as ./a
    // and a//b that would defeat the logical-name producer index.
    if name.contains(['\\', ':', '\0'])
        || name.split('/').any(|part| matches!(part, "" | "." | ".."))
    {
        return Err(ProjectValidationError { path: project.workflow_source.clone(), field: field.into(), cause: format!("invalid artifact {name:?}: expected a relative path with nonempty components, no . or .., backslash, colon, or NUL") });
    }
    Ok(())
}
