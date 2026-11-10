use crate::{
    global_config::{atomic_write, config_dir, error, io_error},
    InstallError,
};
use std::{collections::BTreeMap, fs};

pub type ModelMappings = BTreeMap<String, BTreeMap<String, String>>;

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
    mut models: BTreeMap<String, String>,
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
    models.retain(|_, model| !model.is_empty());
    if models
        .values()
        .any(|model| model.chars().any(char::is_whitespace))
    {
        return Err(error("invalid_model", "Model must not contain whitespace"));
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
