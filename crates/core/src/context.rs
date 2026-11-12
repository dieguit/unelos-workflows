use camino::Utf8PathBuf;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::{
    resolve_skills, Execution, LoadedProject, NodeDefinition, ResolvedSkill, SkillResolveError,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskContext {
    pub kind: &'static str,
    pub node_id: String,
    pub task_definition: String,
    pub agent: String,
    pub execution: Execution,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub task_instructions: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub agent_instructions: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub skills: Vec<ResolvedSkill>,
    pub model: Option<crate::AgentModel>,
    pub mcps: Vec<String>,
    pub tools: Vec<String>,
    pub outcomes: Vec<String>,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub completion_instructions: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TaskInspection {
    #[serde(flatten)]
    pub context: TaskContext,
    pub inputs: Vec<String>,
    pub outputs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactReference {
    pub name: String,
    pub path: Utf8PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransitionFeedback {
    pub source_node_id: String,
    pub source_execution_id: String,
    pub outcome: Option<String>,
    pub reason: Option<String>,
    pub artifacts: Vec<ArtifactReference>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskRuntimeInput {
    pub run_id: String,
    pub task_id: String,
    pub node_id: String,
    pub request: String,
    pub inputs: Vec<ArtifactReference>,
    pub outputs: Vec<ArtifactReference>,
    pub feedback: Option<TransitionFeedback>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskPacket {
    #[serde(flatten)]
    pub context: TaskContext,
    pub run_id: String,
    pub task_id: String,
    pub request: String,
    pub inputs: Vec<ArtifactReference>,
    pub outputs: Vec<ArtifactReference>,
    pub feedback: Option<TransitionFeedback>,
}

#[derive(Debug, thiserror::Error)]
pub enum ContextError {
    #[error(transparent)]
    Skill(#[from] SkillResolveError),
    #[error("{field} at {path}: {cause}")]
    Diagnostic {
        path: Utf8PathBuf,
        field: String,
        cause: String,
    },
}

/// Resolve an ordinary Task node without creating runtime identity or artifact bindings.
pub fn inspect_task(
    project: &LoadedProject,
    node_id: &str,
) -> Result<TaskInspection, ContextError> {
    let node = project
        .workflow
        .nodes
        .get(node_id)
        .ok_or_else(|| ContextError::Diagnostic {
            path: project.workflow_source.clone(),
            field: format!("nodes.{node_id}"),
            cause: "unknown Node".into(),
        })?;
    let NodeDefinition::Task(node) = node else {
        return Err(ContextError::Diagnostic {
            path: project.workflow_source.clone(),
            field: format!("nodes.{node_id}"),
            cause: "inspection is supported only for ordinary Task nodes".into(),
        });
    };
    Ok(TaskInspection {
        context: resolve_task_context(
            project,
            node_id,
            &format!("nodes.{node_id}"),
            &node.task,
            &node.agent,
            node.execution,
        )?,
        inputs: node.inputs.clone(),
        outputs: node.outputs.clone(),
    })
}

pub(crate) fn resolve_task_context(
    project: &LoadedProject,
    node_id: &str,
    field: &str,
    selection: &crate::TaskSelection,
    agent_id: &str,
    execution: crate::Execution,
) -> Result<TaskContext, ContextError> {
    let (task, identity) =
        project
            .selected_task(selection, field)
            .map_err(|error| ContextError::Diagnostic {
                path: error.path,
                field: error.field,
                cause: error.cause,
            })?;
    let agent =
        project
            .definitions
            .agents
            .get(agent_id)
            .ok_or_else(|| ContextError::Diagnostic {
                path: project.workflow_source.clone(),
                field: format!("{field}.agent"),
                cause: format!("referenced Agent {agent_id:?} is missing"),
            })?;
    let ids: Vec<&str> = task
        .skills
        .iter()
        .chain(agent.skills.iter())
        .map(String::as_str)
        .collect();
    let skills = resolve_skills(&project.project_root, &ids)?;

    Ok(TaskContext {
        kind: "task",
        node_id: node_id.to_owned(),
        task_definition: identity,
        agent: agent_id.to_owned(),
        execution,
        task_instructions: task.instructions.clone(),
        agent_instructions: agent.instructions.clone(),
        skills,
        model: agent.model.clone(),
        mcps: agent.mcps.clone(),
        tools: agent.tools.clone(),
        outcomes: task.outcomes.clone(),
        completion_instructions: include_str!("instructions/task-outputs.md")
            .trim_end()
            .into(),
    })
}

/// Build a runtime packet using caller-supplied identities and opaque artifact paths.
pub fn build_task_packet(
    project: &LoadedProject,
    input: TaskRuntimeInput,
) -> Result<TaskPacket, ContextError> {
    let inspection = inspect_task(project, &input.node_id)?;
    let inputs = ordered_bindings(
        project,
        &input.node_id,
        "inputs",
        inspection.inputs,
        input.inputs,
    )?;
    let outputs = ordered_bindings(
        project,
        &input.node_id,
        "outputs",
        inspection.outputs,
        input.outputs,
    )?;
    Ok(TaskPacket {
        context: inspection.context,
        run_id: input.run_id,
        task_id: input.task_id,
        request: input.request,
        inputs,
        outputs,
        feedback: input.feedback,
    })
}

pub(crate) fn ordered_bindings(
    project: &LoadedProject,
    node_id: &str,
    side: &str,
    declared: Vec<String>,
    supplied: Vec<ArtifactReference>,
) -> Result<Vec<ArtifactReference>, ContextError> {
    let mut by_name = BTreeMap::new();
    for binding in supplied {
        let name = binding.name.clone();
        if by_name.insert(name.clone(), binding).is_some() {
            return Err(binding_error(
                project,
                node_id,
                side,
                &name,
                "duplicate binding",
            ));
        }
    }
    for name in by_name.keys() {
        if !declared.iter().any(|declared| declared == name) {
            return Err(binding_error(
                project,
                node_id,
                side,
                name,
                "undeclared binding",
            ));
        }
    }
    declared
        .into_iter()
        .map(|name| {
            by_name
                .remove(&name)
                .ok_or_else(|| binding_error(project, node_id, side, &name, "missing binding"))
        })
        .collect()
}

fn binding_error(
    project: &LoadedProject,
    node_id: &str,
    side: &str,
    name: &str,
    cause: &str,
) -> ContextError {
    ContextError::Diagnostic {
        path: project.workflow_source.clone(),
        field: format!("nodes.{node_id}.{side}.{name}"),
        cause: cause.into(),
    }
}
