use crate::{
    global_config::{atomic_write, config_dir, error, io_error},
    InstallError,
};
use serde::{Deserialize, Deserializer, Serialize};
use std::{collections::BTreeMap, fs};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MappedModel {
    pub model: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effort: Option<String>,
}

impl From<&str> for MappedModel {
    fn from(value: &str) -> Self {
        let (model, effort) = match value.split_once('#') {
            Some((model, effort)) => (model, Some(effort.to_owned())),
            None => (value, None),
        };
        Self {
            model: model.to_owned(),
            effort,
        }
    }
}

impl<'de> Deserialize<'de> for MappedModel {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Fields {
            model: String,
            effort: Option<String>,
        }
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Wire {
            Legacy(String),
            Object(Fields),
        }
        Ok(match Wire::deserialize(deserializer)? {
            Wire::Legacy(value) => Self::from(value.as_str()),
            Wire::Object(fields) => Self {
                model: fields.model,
                effort: fields.effort,
            },
        })
    }
}

pub type ModelMappings = BTreeMap<String, BTreeMap<String, MappedModel>>;

pub(crate) fn resolve_harness_model(
    model: &crate::AgentModel,
    effort: Option<&str>,
    harness: &str,
) -> Result<Option<MappedModel>, InstallError> {
    let mut resolved = match model {
        crate::AgentModel::Alias(alias) => list_model_mappings()?
            .get(alias)
            .and_then(|models| models.get(harness))
            .cloned()
            .ok_or_else(|| {
                let name = match harness {
                    "opencode" => "OpenCode",
                    "claude" => "Claude",
                    other => other,
                };
                error(
                    "unmapped_model_alias",
                    format!("No {name} model mapped for alias '{alias}'"),
                )
            })?,
        crate::AgentModel::Harness {
            harness: target,
            model,
        } if target == harness => MappedModel {
            model: model.clone(),
            effort: None,
        },
        _ => return Ok(None),
    };
    if let Some(effort) = effort {
        resolved.effort = (effort != "default").then(|| effort.to_owned());
    }
    Ok(Some(resolved))
}

pub(crate) fn valid_effort(effort: &str) -> bool {
    !effort.is_empty() && !effort.chars().any(|c| c.is_whitespace() || c == '#')
}

pub fn list_model_mappings() -> Result<ModelMappings, InstallError> {
    match fs::read(config_dir()?.join("models.json")) {
        Ok(bytes) => serde_json::from_slice(&bytes).map_err(|e| error("invalid_model_mappings", e)),
        // First use seeds default aliases; once the file exists, deletions stick.
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => write(
            ["smart", "balanced", "fast"]
                .map(|a| (a.to_owned(), BTreeMap::new()))
                .into(),
        ),
        Err(e) => Err(io_error(e)),
    }
}

fn write(mappings: ModelMappings) -> Result<ModelMappings, InstallError> {
    atomic_write(
        &config_dir()?.join("models.json"),
        &serde_json::to_vec_pretty(&mappings).map_err(io_error)?,
    )?;
    Ok(mappings)
}

pub fn save_model_mapping(
    original: Option<&str>,
    alias: &str,
    mut models: BTreeMap<String, MappedModel>,
) -> Result<ModelMappings, InstallError> {
    if !alias.split(['-', '_', '.']).all(|part| {
        !part.is_empty()
            && part
                .bytes()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
    }) {
        return Err(error(
            "invalid_model_alias",
            "Alias must be lowercase letters and digits separated by -, _ or .",
        ));
    }
    models.retain(|_, model| !model.model.is_empty());
    if models
        .values()
        .any(|model| model.model.chars().any(|c| c.is_whitespace() || c == '#'))
    {
        return Err(error(
            "invalid_model",
            "Model must not contain whitespace or #",
        ));
    }
    for model in models.values_mut() {
        if model.effort.as_deref() == Some("") {
            model.effort = None;
        }
        if model
            .effort
            .as_deref()
            .is_some_and(|effort| !valid_effort(effort))
        {
            return Err(error(
                "invalid_effort",
                "Effort must not contain whitespace or #",
            ));
        }
    }
    let mut mappings = list_model_mappings()?;
    if let Some(original) = original {
        if !mappings.contains_key(original) {
            return Err(error("model_alias_not_found", "Original alias not found"));
        }
    }
    if original != Some(alias) && mappings.contains_key(alias) {
        return Err(error("model_alias_exists", "Alias already exists"));
    }
    if let Some(original) = original {
        mappings.remove(original);
    }
    mappings.insert(alias.to_owned(), models);
    write(mappings)
}

pub fn delete_model_mapping(alias: &str) -> Result<ModelMappings, InstallError> {
    let mut mappings = list_model_mappings()?;
    if mappings.remove(alias).is_none() {
        return Err(error("model_alias_not_found", "Alias not found"));
    }
    write(mappings)
}
