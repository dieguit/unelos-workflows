use indexmap::IndexMap;
use serde::{
    de::{DeserializeOwned, MapAccess, SeqAccess, Visitor},
    Deserialize, Deserializer, Serialize,
};
use std::collections::BTreeMap;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectDefinitions {
    pub task_definitions: BTreeMap<String, TaskDefinition>,
    pub agents: BTreeMap<String, AgentDefinition>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(try_from = "TaskWire", into = "TaskWire")]
pub struct TaskDefinition {
    pub instructions: String,
    pub instructions_file: Option<String>,
    pub description: Option<String>,
    pub skills: Vec<String>,
    pub outcomes: Vec<String>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct TaskWire {
    #[serde(
        default,
        deserialize_with = "present_string",
        skip_serializing_if = "Option::is_none"
    )]
    instructions: Option<String>,
    #[serde(
        default,
        rename = "instructionsFile",
        deserialize_with = "present_string",
        skip_serializing_if = "Option::is_none"
    )]
    instructions_file: Option<String>,
    #[serde(default, deserialize_with = "strict_option_string")]
    description: Option<String>,
    #[serde(default, deserialize_with = "strict_strings")]
    skills: Vec<String>,
    #[serde(default, deserialize_with = "strict_strings")]
    outcomes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(try_from = "AgentWire", into = "AgentWire")]
pub struct AgentDefinition {
    pub instructions: String,
    pub instructions_file: Option<String>,
    pub skills: Vec<String>,
    pub mcps: Vec<String>,
    pub tools: Vec<String>,
    pub model: Option<AgentModel>,
    pub effort: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged, deny_unknown_fields)]
pub enum AgentModel {
    Alias(#[serde(deserialize_with = "nonempty_string")] String),
    Harness {
        #[serde(deserialize_with = "nonempty_string")]
        harness: String,
        #[serde(deserialize_with = "nonempty_string")]
        model: String,
    },
}

fn nonempty_string<'de, D: Deserializer<'de>>(deserializer: D) -> Result<String, D::Error> {
    let value = strict_string(deserializer)?;
    if value.trim().is_empty() {
        return Err(serde::de::Error::custom("expected a nonempty string"));
    }
    Ok(value)
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct AgentWire {
    #[serde(
        default,
        deserialize_with = "present_string",
        skip_serializing_if = "Option::is_none"
    )]
    instructions: Option<String>,
    #[serde(
        default,
        rename = "instructionsFile",
        deserialize_with = "present_string",
        skip_serializing_if = "Option::is_none"
    )]
    instructions_file: Option<String>,
    #[serde(
        default,
        deserialize_with = "strict_strings",
        skip_serializing_if = "Vec::is_empty"
    )]
    skills: Vec<String>,
    #[serde(
        default,
        deserialize_with = "strict_strings",
        skip_serializing_if = "Vec::is_empty"
    )]
    mcps: Vec<String>,
    #[serde(
        default,
        deserialize_with = "strict_strings",
        skip_serializing_if = "Vec::is_empty"
    )]
    tools: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    model: Option<AgentModel>,
    #[serde(
        default,
        deserialize_with = "present_string",
        skip_serializing_if = "Option::is_none"
    )]
    effort: Option<String>,
}

fn instruction_source(
    instructions: Option<String>,
    file: &Option<String>,
) -> Result<String, String> {
    match (instructions, file) {
        (Some(text), None) => Ok(text),
        (None, Some(path)) if !path.is_empty() => Ok(String::new()),
        _ => Err("expected exactly one of instructions and nonempty instructionsFile".into()),
    }
}

impl TryFrom<TaskWire> for TaskDefinition {
    type Error = String;
    fn try_from(raw: TaskWire) -> Result<Self, Self::Error> {
        Ok(Self {
            instructions: instruction_source(raw.instructions, &raw.instructions_file)?,
            instructions_file: raw.instructions_file,
            description: raw.description,
            skills: raw.skills,
            outcomes: raw.outcomes,
        })
    }
}

impl From<TaskDefinition> for TaskWire {
    fn from(task: TaskDefinition) -> Self {
        Self {
            instructions: task
                .instructions_file
                .is_none()
                .then_some(task.instructions),
            instructions_file: task.instructions_file,
            description: task.description,
            skills: task.skills,
            outcomes: task.outcomes,
        }
    }
}

impl TryFrom<AgentWire> for AgentDefinition {
    type Error = String;
    fn try_from(raw: AgentWire) -> Result<Self, Self::Error> {
        if let Some(effort) = &raw.effort {
            if raw.model.is_none() {
                return Err("effort requires model".into());
            }
            if !crate::model_mappings::valid_effort(effort) {
                return Err("invalid effort".into());
            }
        }
        Ok(Self {
            instructions: instruction_source(raw.instructions, &raw.instructions_file)?,
            instructions_file: raw.instructions_file,
            skills: raw.skills,
            mcps: raw.mcps,
            tools: raw.tools,
            model: raw.model,
            effort: raw.effort,
        })
    }
}

impl From<AgentDefinition> for AgentWire {
    fn from(agent: AgentDefinition) -> Self {
        Self {
            instructions: agent
                .instructions_file
                .is_none()
                .then_some(agent.instructions),
            instructions_file: agent.instructions_file,
            skills: agent.skills,
            mcps: agent.mcps,
            tools: agent.tools,
            model: agent.model,
            effort: agent.effort,
        }
    }
}

fn present_string<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Option<String>, D::Error> {
    strict_string(deserializer).map(Some)
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ProjectSettings {
    pub auto_build_harness: Option<bool>,
}

pub fn parse_project_settings(
    source: &str,
    yaml: &str,
) -> Result<ProjectSettings, DefinitionParseError> {
    parse(source, yaml).map_err(|mut error| {
        let message = error.diagnostic.to_string();
        if message.contains("unknown field `taskDefinitions`")
            || message.contains("unknown field `agents`")
        {
            error.diagnostic = <serde_yaml_ng::Error as serde::de::Error>::custom(format!(
                "{message}; definitions belong in tasks/<id>/task.yaml and agents/<id>/agent.yaml"
            ));
        }
        error
    })
}

pub fn parse_task_definition(
    source: &str,
    yaml: &str,
) -> Result<TaskDefinition, DefinitionParseError> {
    parse(source, yaml)
}

pub fn parse_agent_definition(
    source: &str,
    yaml: &str,
) -> Result<AgentDefinition, DefinitionParseError> {
    parse(source, yaml)
}

fn strict_string<'de, D: Deserializer<'de>>(deserializer: D) -> Result<String, D::Error> {
    struct StrictString;
    impl<'de> Visitor<'de> for StrictString {
        type Value = String;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a string")
        }

        fn visit_str<E: serde::de::Error>(self, value: &str) -> Result<String, E> {
            Ok(value.to_owned())
        }

        fn visit_string<E: serde::de::Error>(self, value: String) -> Result<String, E> {
            Ok(value)
        }
    }
    deserializer.deserialize_any(StrictString)
}

fn strict_option_string<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<String>, D::Error> {
    struct StrictOptionString;
    impl<'de> Visitor<'de> for StrictOptionString {
        type Value = Option<String>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a string or null")
        }

        fn visit_none<E: serde::de::Error>(self) -> Result<Self::Value, E> {
            Ok(None)
        }

        fn visit_unit<E: serde::de::Error>(self) -> Result<Self::Value, E> {
            Ok(None)
        }

        fn visit_some<D: Deserializer<'de>>(
            self,
            deserializer: D,
        ) -> Result<Self::Value, D::Error> {
            strict_string(deserializer).map(Some)
        }
    }
    deserializer.deserialize_option(StrictOptionString)
}

fn strict_strings<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<String>, D::Error> {
    struct StrictStrings;
    impl<'de> Visitor<'de> for StrictStrings {
        type Value = Vec<String>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a sequence of strings")
        }

        fn visit_seq<A: SeqAccess<'de>>(self, mut sequence: A) -> Result<Vec<String>, A::Error> {
            let mut values = Vec::new();
            while let Some(value) = sequence.next_element::<StrictStringValue>()? {
                values.push(value.0);
            }
            Ok(values)
        }
    }
    deserializer.deserialize_seq(StrictStrings)
}

struct StrictMapKey(String);

impl<'de> Deserialize<'de> for StrictMapKey {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        strict_string(deserializer).map(Self)
    }
}

struct StrictStringValue(String);

impl<'de> Deserialize<'de> for StrictStringValue {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        strict_string(deserializer).map(Self)
    }
}

fn strict_map<'de, D, V>(deserializer: D) -> Result<IndexMap<String, V>, D::Error>
where
    D: Deserializer<'de>,
    V: Deserialize<'de>,
{
    struct StrictMap<V>(std::marker::PhantomData<V>);
    impl<'de, V: Deserialize<'de>> Visitor<'de> for StrictMap<V> {
        type Value = IndexMap<String, V>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a map with unique string keys")
        }

        fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
            let mut values = IndexMap::new();
            while let Some((StrictMapKey(key), value)) = map.next_entry::<StrictMapKey, V>()? {
                if values.insert(key.clone(), value).is_some() {
                    return Err(serde::de::Error::custom(format!("duplicate key `{key}`")));
                }
            }
            Ok(values)
        }
    }
    deserializer.deserialize_map(StrictMap(std::marker::PhantomData))
}

fn strict_string_map<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<IndexMap<String, String>, D::Error> {
    strict_map::<D, StrictStringValue>(deserializer).map(|values| {
        values
            .into_iter()
            .map(|(key, value)| (key, value.0))
            .collect()
    })
}

fn strict_outcome_map<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<IndexMap<String, String>, D::Error> {
    strict_string_map(deserializer)
        .map_err(|error| serde::de::Error::custom(format!("next outcome map: {error}")))
}

#[derive(Debug, thiserror::Error)]
#[error("{context}: {diagnostic}")]
pub struct DefinitionParseError {
    pub context: String,
    #[source]
    pub diagnostic: serde_yaml_ng::Error,
}

fn parse<T: DeserializeOwned>(context: &str, yaml: &str) -> Result<T, DefinitionParseError> {
    serde_yaml_ng::from_str(yaml).map_err(|diagnostic| DefinitionParseError {
        context: context.to_owned(),
        diagnostic,
    })
}

/// A Workflow's author-authored name, entry node, and nodes keyed by identity.
/// Map keys are the node IDs; parsing does not load files, resolve agent
/// references, or validate graph edges.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WorkflowDefinition {
    #[serde(deserialize_with = "strict_string")]
    pub name: String,
    #[serde(
        default,
        deserialize_with = "strict_option_string",
        skip_serializing_if = "Option::is_none"
    )]
    pub description: Option<String>,
    #[serde(deserialize_with = "strict_string")]
    pub start: String,
    #[serde(default, deserialize_with = "strict_strings")]
    pub inputs: Vec<String>,
    #[serde(
        deserialize_with = "strict_map",
        serialize_with = "serialize_workflow_nodes"
    )]
    pub nodes: IndexMap<String, NodeDefinition>,
}

fn serialize_workflow_nodes<S: serde::Serializer>(
    nodes: &IndexMap<String, NodeDefinition>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    let nodes = nodes
        .iter()
        .map(|(id, node)| {
            let mut value = serde_yaml_ng::to_value(node).map_err(serde::ser::Error::custom)?;
            let fields = value.as_mapping_mut().expect("nodes serialize as maps");
            let kind = fields.remove("kind").expect("nodes serialize with kind");
            fields.insert("type".into(), kind);
            Ok((id, value))
        })
        .collect::<Result<IndexMap<_, _>, S::Error>>()?;
    nodes.serialize(serializer)
}

/// How a task node runs. Parallel workers are implicitly subagents.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Execution {
    #[default]
    Main,
    Subagent,
}

/// A single next-node identity or outcome labels mapped to destination IDs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub enum Transition {
    Single(String),
    Outcomes(IndexMap<String, String>),
}

impl<'de> Deserialize<'de> for Transition {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct TransitionVisitor;
        impl<'de> Visitor<'de> for TransitionVisitor {
            type Value = Transition;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("the next field must be a destination string or outcome map")
            }

            fn visit_str<E: serde::de::Error>(self, value: &str) -> Result<Self::Value, E> {
                Ok(Transition::Single(value.to_owned()))
            }

            fn visit_string<E: serde::de::Error>(self, value: String) -> Result<Self::Value, E> {
                Ok(Transition::Single(value))
            }

            fn visit_seq<A: SeqAccess<'de>>(self, _sequence: A) -> Result<Self::Value, A::Error> {
                Err(serde::de::Error::custom(
                    "next must be a destination string or outcome map",
                ))
            }

            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let mut outcomes = IndexMap::new();
                while let Some(StrictMapKey(outcome)) = map.next_key()? {
                    let destination = map
                        .next_value::<StrictStringValue>()
                        .map_err(|error| {
                            serde::de::Error::custom(format!(
                                "next outcome destination must be a string: {error}"
                            ))
                        })?
                        .0;
                    if outcomes.insert(outcome.clone(), destination).is_some() {
                        return Err(serde::de::Error::custom(format!(
                            "duplicate key `{outcome}`"
                        )));
                    }
                }
                Ok(Transition::Outcomes(outcomes))
            }
        }
        deserializer.deserialize_any(TransitionVisitor)
    }
}

/// A task node; artifact names and agent references remain opaque strings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum TaskSelection {
    #[serde(rename = "task")]
    Named(String),
    #[serde(rename = "taskDefinition")]
    Inline(TaskDefinition),
}

impl From<String> for TaskSelection {
    fn from(id: String) -> Self {
        Self::Named(id)
    }
}

impl From<&str> for TaskSelection {
    fn from(id: &str) -> Self {
        Self::Named(id.into())
    }
}

fn select_task(
    task: Option<String>,
    inline: Option<TaskDefinition>,
) -> Result<TaskSelection, String> {
    match (task, inline) {
        (Some(id), None) => Ok(TaskSelection::Named(id)),
        (None, Some(definition)) => Ok(TaskSelection::Inline(definition)),
        _ => Err("expected exactly one of task and taskDefinition".into()),
    }
}

fn present_task<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<TaskDefinition>, D::Error> {
    TaskDefinition::deserialize(deserializer).map(Some)
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(try_from = "TaskNodeWire")]
pub struct TaskNode {
    #[serde(flatten)]
    pub task: TaskSelection,
    pub agent: String,
    pub execution: Execution,
    pub inputs: Vec<String>,
    pub outputs: Vec<String>,
    pub next: Transition,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TaskNodeWire {
    #[serde(default, deserialize_with = "present_string")]
    task: Option<String>,
    #[serde(default, rename = "taskDefinition", deserialize_with = "present_task")]
    task_definition: Option<TaskDefinition>,
    #[serde(default, deserialize_with = "strict_string")]
    agent: String,
    #[serde(default)]
    execution: Execution,
    #[serde(default, deserialize_with = "strict_strings")]
    inputs: Vec<String>,
    #[serde(default, deserialize_with = "strict_strings")]
    outputs: Vec<String>,
    next: Transition,
}

impl TryFrom<TaskNodeWire> for TaskNode {
    type Error = String;
    fn try_from(raw: TaskNodeWire) -> Result<Self, Self::Error> {
        Ok(Self {
            task: select_task(raw.task, raw.task_definition)?,
            agent: raw.agent,
            execution: raw.execution,
            inputs: raw.inputs,
            outputs: raw.outputs,
            next: raw.next,
        })
    }
}

/// An opaque task/agent reference used by a parallel node.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(try_from = "WorkerWire")]
pub struct WorkerReference {
    #[serde(flatten)]
    pub task: TaskSelection,
    pub agent: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WorkerWire {
    #[serde(default, deserialize_with = "present_string")]
    task: Option<String>,
    #[serde(default, rename = "taskDefinition", deserialize_with = "present_task")]
    task_definition: Option<TaskDefinition>,
    #[serde(deserialize_with = "strict_string")]
    agent: String,
}

impl TryFrom<WorkerWire> for WorkerReference {
    type Error = String;
    fn try_from(raw: WorkerWire) -> Result<Self, Self::Error> {
        Ok(Self {
            task: select_task(raw.task, raw.task_definition)?,
            agent: raw.agent,
        })
    }
}

/// Concurrent workers, with authored input/output ordering preserved.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ParallelNode {
    pub tasks: Vec<WorkerReference>,
    #[serde(default, deserialize_with = "strict_strings")]
    pub inputs: Vec<String>,
    #[serde(default, deserialize_with = "strict_strings")]
    pub outputs: Vec<String>,
    pub next: Transition,
}

/// A human approval gate. Route labels (including `approved` and `rejected`)
/// are not validated here; that semantic check is deferred to T04.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct HumanGateNode {
    #[serde(
        default,
        deserialize_with = "present_string",
        skip_serializing_if = "Option::is_none"
    )]
    pub description: Option<String>,
    #[serde(default, deserialize_with = "strict_strings")]
    pub inputs: Vec<String>,
    #[serde(deserialize_with = "strict_outcome_map")]
    pub next: IndexMap<String, String>,
}

/// One of the supported structural Workflow node forms.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum NodeDefinition {
    Task(TaskNode),
    Parallel(ParallelNode),
    HumanGate(HumanGateNode),
    Workflow(WorkflowNode),
    End,
}

impl<'de> Deserialize<'de> for NodeDefinition {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        use serde::de::Error;
        use serde_yaml_ng::Value;
        let mut fields = match Value::deserialize(deserializer)? {
            Value::Mapping(fields) => fields,
            _ => return Err(D::Error::custom("node must be a mapping")),
        };
        let type_key = Value::String("type".into());
        if let Some((field, _)) = fields
            .iter()
            .find(|(_, value)| matches!(value, Value::Null))
        {
            return Err(D::Error::custom(format!(
                "node field {field:?} cannot be null"
            )));
        }
        let kind = match fields.remove(&type_key) {
            Some(Value::String(kind)) => kind,
            Some(_) => return Err(D::Error::custom("node type must be a string")),
            None if fields.contains_key(Value::String("task".into()))
                || fields.contains_key(Value::String("taskDefinition".into())) =>
            {
                "task".into()
            }
            None => return Err(D::Error::missing_field("type")),
        };
        let value = Value::Mapping(fields);
        match kind.as_str() {
            "task" => TaskNode::deserialize(value)
                .map(Self::Task)
                .map_err(D::Error::custom),
            "parallel" => ParallelNode::deserialize(value)
                .map(Self::Parallel)
                .map_err(D::Error::custom),
            "human-gate" => HumanGateNode::deserialize(value)
                .map(Self::HumanGate)
                .map_err(D::Error::custom),
            "workflow" => WorkflowNode::deserialize(value)
                .map(Self::Workflow)
                .map_err(D::Error::custom),
            "end" => {
                let Value::Mapping(fields) = value else {
                    unreachable!()
                };
                if let Some(field) = fields.keys().next() {
                    return Err(D::Error::custom(format!(
                        "end node cannot contain field {field:?}"
                    )));
                }
                Ok(Self::End)
            }
            other => Err(D::Error::unknown_variant(
                other,
                &["task", "parallel", "human-gate", "workflow", "end"],
            )),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(from = "WorkflowNodeWire")]
pub struct WorkflowNode {
    pub workflow: String,
    pub inputs: IndexMap<String, String>,
    pub outputs: IndexMap<String, String>,
    pub next: Transition,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WorkflowNodeWire {
    #[serde(deserialize_with = "strict_string")]
    workflow: String,
    #[serde(default, deserialize_with = "strict_string_map")]
    inputs: IndexMap<String, String>,
    #[serde(default, deserialize_with = "strict_string_map")]
    outputs: IndexMap<String, String>,
    next: Transition,
}

impl From<WorkflowNodeWire> for WorkflowNode {
    fn from(raw: WorkflowNodeWire) -> Self {
        Self {
            workflow: raw.workflow,
            inputs: raw.inputs,
            outputs: raw.outputs,
            next: raw.next,
        }
    }
}

/// Parse Workflow authoring text without reading files or resolving references.
pub fn parse_workflow_definition(
    source: &str,
    yaml: &str,
) -> Result<WorkflowDefinition, DefinitionParseError> {
    parse(source, yaml)
}
