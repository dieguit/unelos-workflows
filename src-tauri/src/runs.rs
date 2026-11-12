use crate::projects::{approved_root, load_from, store_file};
use std::path::Path;
use tauri::AppHandle;

#[tauri::command]
pub async fn list_runs(
    app: AppHandle,
    root: String,
) -> Result<Vec<unelos_core::RunListItem>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let root = approved_root(&load_from(&store_file(&app)?), Path::new(&root))?;
        unelos_core::list_runs(
            camino::Utf8Path::from_path(&root).ok_or("Project path is not UTF-8")?,
        )
        .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn get_run(
    app: AppHandle,
    root: String,
    run_id: String,
) -> Result<unelos_core::RunDetails, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let root = approved_root(&load_from(&store_file(&app)?), Path::new(&root))?;
        unelos_core::get_run(
            camino::Utf8Path::from_path(&root).ok_or("Project path is not UTF-8")?,
            &run_id,
        )
        .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn delete_run(app: AppHandle, root: String, run_id: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        let root = approved_root(&load_from(&store_file(&app)?), Path::new(&root))?;
        unelos_core::delete_run(
            camino::Utf8Path::from_path(&root).ok_or("Project path is not UTF-8")?,
            &run_id,
        )
        .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn read_run_artifact(
    app: AppHandle,
    root: String,
    run_id: String,
    path: String,
) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let root = approved_root(&load_from(&store_file(&app)?), Path::new(&root))?;
        unelos_core::read_run_artifact(
            camino::Utf8Path::from_path(&root).ok_or("Project path is not UTF-8")?,
            &run_id,
            camino::Utf8Path::new(&path),
        )
        .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}
