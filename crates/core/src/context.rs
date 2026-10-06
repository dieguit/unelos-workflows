use camino::Utf8PathBuf;
use serde::Serialize;

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
    pub task_instructions: String,
    pub agent_instructions: String,
    pub skills: Vec<ResolvedSkill>,
    pub model: Option<String>,
    pub mcps: Vec<String>,
    pub tools: Vec<String>,
    pub outcomes: Vec<String>,
    pub completion_instructions: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TaskInspection {
    #[serde(flatten)]
    pub context: TaskContext,
    pub inputs: Vec<String>,
    pub outputs: Vec<String>,
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
    let task = project
        .definitions
        .task_definitions
        .get(&node.task)
        .ok_or_else(|| ContextError::Diagnostic {
            path: project.project_source.clone(),
            field: format!("taskDefinitions.{}", node.task),
            cause: "referenced Task Definition is missing".into(),
        })?;
    let agent =
        project
            .definitions
            .agents
            .get(&node.agent)
            .ok_or_else(|| ContextError::Diagnostic {
                path: project.project_source.clone(),
                field: format!("agents.{}", node.agent),
                cause: "referenced Agent is missing".into(),
            })?;
    let ids: Vec<&str> = task
        .skills
        .iter()
        .chain(agent.skills.iter())
        .map(String::as_str)
        .collect();
    let skills = resolve_skills(&project.project_root, &ids)?;

    Ok(TaskInspection {
        context: TaskContext {
            kind: "task",
            node_id: node_id.to_owned(),
            task_definition: node.task.clone(),
            agent: node.agent.clone(),
            execution: node.execution,
            task_instructions: task.instructions.clone(),
            agent_instructions: agent.instructions.clone(),
            skills,
            model: agent.model.clone(),
            mcps: agent.mcps.clone(),
            tools: agent.tools.clone(),
            outcomes: task.outcomes.clone(),
            completion_instructions:
                "Produce the declared outputs before completing this current Task.".into(),
        },
        inputs: node.inputs.clone(),
        outputs: node.outputs.clone(),
    })
}
