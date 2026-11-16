use std::collections::BTreeMap;
use unelos_core::ModelMappings;

#[tauri::command]
pub async fn agent_model_options() -> Result<unelos_core::AgentModelOptions, String> {
    tauri::async_runtime::spawn_blocking(unelos_core::agent_model_options)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_model_mappings() -> Result<ModelMappings, String> {
    tauri::async_runtime::spawn_blocking(|| {
        unelos_core::list_model_mappings().map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn save_model_mapping(
    original: Option<String>,
    alias: String,
    models: BTreeMap<String, unelos_core::MappedModel>,
) -> Result<ModelMappings, String> {
    tauri::async_runtime::spawn_blocking(move || {
        unelos_core::save_model_mapping(original.as_deref(), &alias, models)
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn delete_model_mapping(alias: String) -> Result<ModelMappings, String> {
    tauri::async_runtime::spawn_blocking(move || {
        unelos_core::delete_model_mapping(&alias).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn harness_models(id: String) -> Result<Vec<String>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        unelos_core::harness_models(&id).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}
