use serde::{
    de::{DeserializeOwned, MapAccess, SeqAccess, Visitor},
    Deserialize, Deserializer,
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
    struct StrictStringValue(String);
    impl<'de> Deserialize<'de> for StrictStringValue {
        fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
            strict_string(deserializer).map(Self)
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
