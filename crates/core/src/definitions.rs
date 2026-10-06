use serde::{
    de::{DeserializeOwned, MapAccess, SeqAccess, Visitor},
    Deserialize, Deserializer, Serialize,
};
use std::collections::BTreeMap;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectDefinitions {
    #[serde(default, rename = "taskDefinitions", deserialize_with = "strict_map")]
    pub task_definitions: BTreeMap<String, TaskDefinition>,
    #[serde(default, deserialize_with = "strict_map")]
    pub agents: BTreeMap<String, AgentDefinition>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TaskDefinition {
    #[serde(deserialize_with = "strict_string")]
    pub instructions: String,
    #[serde(default, deserialize_with = "strict_option_string")]
    pub description: Option<String>,
    #[serde(default, deserialize_with = "strict_strings")]
    pub skills: Vec<String>,
    #[serde(default, deserialize_with = "strict_strings")]
    pub outcomes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentDefinition {
    #[serde(deserialize_with = "strict_string")]
    pub instructions: String,
    #[serde(default, deserialize_with = "strict_strings")]
    pub skills: Vec<String>,
    #[serde(default, deserialize_with = "strict_strings")]
    pub mcps: Vec<String>,
    #[serde(default, deserialize_with = "strict_strings")]
    pub tools: Vec<String>,
    #[serde(default, deserialize_with = "strict_option_string")]
    pub model: Option<String>,
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

fn strict_map<'de, D, V>(deserializer: D) -> Result<BTreeMap<String, V>, D::Error>
where
    D: Deserializer<'de>,
    V: Deserialize<'de>,
{
    struct StrictMap<V>(std::marker::PhantomData<V>);
    impl<'de, V: Deserialize<'de>> Visitor<'de> for StrictMap<V> {
        type Value = BTreeMap<String, V>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a map with unique string keys")
        }

        fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
            let mut values = BTreeMap::new();
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
) -> Result<BTreeMap<String, String>, D::Error> {
    strict_map::<D, StrictStringValue>(deserializer).map(|values| {
        values
            .into_iter()
            .map(|(key, value)| (key, value.0))
            .collect()
    })
}

fn strict_outcome_map<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<BTreeMap<String, String>, D::Error> {
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

/// Parse authoring text without reading files or resolving references.
pub fn parse_project_definitions(
    source: &str,
    yaml: &str,
) -> Result<ProjectDefinitions, DefinitionParseError> {
    parse(source, yaml)
}

/// A Workflow's author-authored name, entry node, and nodes keyed by identity.
/// Map keys are the node IDs; parsing does not load files, resolve agent
/// references, or validate graph edges.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkflowDefinition {
    #[serde(deserialize_with = "strict_string")]
    pub name: String,
    #[serde(deserialize_with = "strict_string")]
    pub start: String,
    #[serde(deserialize_with = "strict_map")]
    pub nodes: BTreeMap<String, NodeDefinition>,
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
    Outcomes(BTreeMap<String, String>),
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
                let mut outcomes = BTreeMap::new();
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
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TaskNode {
    #[serde(deserialize_with = "strict_string")]
    pub task: String,
    #[serde(deserialize_with = "strict_string")]
    pub agent: String,
    #[serde(default)]
    pub execution: Execution,
    #[serde(default, deserialize_with = "strict_strings")]
    pub inputs: Vec<String>,
    #[serde(default, deserialize_with = "strict_strings")]
    pub outputs: Vec<String>,
    pub next: Transition,
}

/// An opaque task/agent reference used by a parallel node.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WorkerReference {
    #[serde(deserialize_with = "strict_string")]
    pub task: String,
    #[serde(deserialize_with = "strict_string")]
    pub agent: String,
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
    #[serde(default, deserialize_with = "strict_strings")]
    pub inputs: Vec<String>,
    #[serde(deserialize_with = "strict_outcome_map")]
    pub next: BTreeMap<String, String>,
}

/// One of the supported structural Workflow node forms.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum NodeDefinition {
    Task(TaskNode),
    Parallel(ParallelNode),
    HumanGate(HumanGateNode),
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
            None if fields.contains_key(Value::String("task".into())) => "task".into(),
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
                &["task", "parallel", "human-gate", "end"],
            )),
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
