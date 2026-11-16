use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};
use tauri::{AppHandle, Manager};
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons};

const MAX_RECENTS: usize = 10;

#[tauri::command]
pub async fn project_references(
    app: AppHandle,
    root: String,
) -> Result<unelos_core::ProjectReferences, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let root = approved_root(&load_from(&store_file(&app)?), Path::new(&root))?;
        unelos_core::project_references(
            camino::Utf8Path::from_path(&root).ok_or("Project path is not UTF-8")?,
        )
        .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn duplicate_agent(
    app: AppHandle,
    root: String,
    id: String,
    new_id: String,
) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        let root = approved_root(&load_from(&store_file(&app)?), Path::new(&root))?;
        unelos_core::duplicate_agent(
            camino::Utf8Path::from_path(&root).ok_or("Project path is not UTF-8")?,
            &id,
            &new_id,
        )
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
pub async fn duplicate_skill(
    app: AppHandle,
    root: String,
    id: String,
    new_id: String,
) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        let root = approved_root(&load_from(&store_file(&app)?), Path::new(&root))?;
        unelos_core::duplicate_skill(
            camino::Utf8Path::from_path(&root).ok_or("Project path is not UTF-8")?,
            &id,
            &new_id,
        )
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
pub async fn duplicate_workflow(
    app: AppHandle,
    root: String,
    id: String,
    new_id: String,
) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        let root = approved_root(&load_from(&store_file(&app)?), Path::new(&root))?;
        unelos_core::duplicate_workflow(
            camino::Utf8Path::from_path(&root).ok_or("Project path is not UTF-8")?,
            &id,
            &new_id,
        )
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
pub async fn remove_agent(app: AppHandle, root: String, id: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        let root = approved_root(&load_from(&store_file(&app)?), Path::new(&root))?;
        unelos_core::remove_agent(
            camino::Utf8Path::from_path(&root).ok_or("Project path is not UTF-8")?,
            &id,
        )
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
pub async fn remove_skill(app: AppHandle, root: String, id: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        let root = approved_root(&load_from(&store_file(&app)?), Path::new(&root))?;
        unelos_core::remove_skill(
            camino::Utf8Path::from_path(&root).ok_or("Project path is not UTF-8")?,
            &id,
        )
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
pub async fn remove_workflow(app: AppHandle, root: String, id: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        let root = approved_root(&load_from(&store_file(&app)?), Path::new(&root))?;
        unelos_core::remove_workflow(
            camino::Utf8Path::from_path(&root).ok_or("Project path is not UTF-8")?,
            &id,
        )
    })
    .await
    .map_err(|e| e.to_string())?
}

#[derive(Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Store {
    #[serde(default)]
    pub recents: Vec<StoredRecent>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoredRecent {
    pub path: String,
    pub opened_at: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecentProject {
    path: String,
    name: String,
    opened_at: u64,
    available: bool,
}

#[derive(Debug, Serialize)]
pub struct ProjectState {
    home: String,
    recents: Vec<RecentProject>,
}

pub fn load_from(file: &Path) -> Store {
    match fs::read_to_string(file) {
        Ok(text) => serde_json::from_str(&text).unwrap_or_else(|error| {
            log::warn!("ignoring unreadable {}: {error}", file.display());
            Store::default()
        }),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Store::default(),
        Err(error) => {
            log::warn!("could not read {}: {error}", file.display());
            Store::default()
        }
    }
}

pub fn save_to(file: &Path, store: &Store) -> Result<(), String> {
    if let Some(dir) = file.parent() {
        fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let text = serde_json::to_string_pretty(store).map_err(|e| e.to_string())?;
    let temporary = file.with_extension("json.tmp");
    // ponytail: fixed temp serializes saves; a crash-left temp fails safely until
    // removed. Use unique temp names if concurrent writers/recovery are needed.
    let mut output = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)
        .map_err(|e| e.to_string())?;
    let result = (|| {
        output.write_all(text.as_bytes())?;
        output.sync_all()?;
        drop(output);
        fs::rename(&temporary, file)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result.map_err(|e: io::Error| e.to_string())
}

pub fn push_recent(store: &mut Store, path: String, now: u64) {
    store.recents.retain(|r| r.path != path);
    store.recents.insert(
        0,
        StoredRecent {
            path,
            opened_at: now,
        },
    );
    store.recents.truncate(MAX_RECENTS);
}

pub fn forget(store: &mut Store, path: &str) {
    store.recents.retain(|r| r.path != path);
}

/// Returns the canonical project root or a user-facing error.
pub fn check_project(path: &Path) -> Result<PathBuf, String> {
    let name = folder_name(path);
    if !path.is_dir() {
        return Err(format!(
            "{name} couldn’t be opened: the folder no longer exists"
        ));
    }
    if !is_project(path) {
        return Err(format!(
            "{name} is not a Unelos project (missing .unelos/project.yaml)"
        ));
    }
    fs::canonicalize(path).map_err(|e| format!("{name} couldn’t be opened: {e}"))
}

/// Only native-picked roots persisted by this backend can be reopened or read.
pub(crate) fn approved_root(store: &Store, path: &Path) -> Result<PathBuf, String> {
    if !store.recents.iter().any(|r| Path::new(&r.path) == path) {
        return Err("Project is not approved; select it using Open folder…".into());
    }
    let root = check_project(path)?;
    if root != path {
        return Err("Project location changed; select it using Open folder…".into());
    }
    Ok(root)
}

pub fn state(store: &Store, home: String) -> ProjectState {
    ProjectState {
        home,
        recents: store
            .recents
            .iter()
            .map(|r| RecentProject {
                name: folder_name(Path::new(&r.path)),
                available: is_project(Path::new(&r.path)),
                path: r.path.clone(),
                opened_at: r.opened_at,
            })
            .collect(),
    }
}

fn is_project(path: &Path) -> bool {
    path.join(".unelos/project.yaml").is_file()
}

fn folder_name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string())
}

fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or_default()
}

pub(crate) fn store_file(app: &AppHandle) -> Result<PathBuf, String> {
    app.path()
        .config_dir()
        .map(|dir| dir.join("unelos/projects.json"))
        .map_err(|e| e.to_string())
}

fn home(app: &AppHandle) -> String {
    app.path()
        .home_dir()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_default()
}

#[tauri::command]
pub fn project_state(app: AppHandle) -> Result<ProjectState, String> {
    Ok(state(&load_from(&store_file(&app)?), home(&app)))
}

#[tauri::command]
pub fn open_project(app: AppHandle, path: String) -> Result<ProjectState, String> {
    let file = store_file(&app)?;
    let mut store = load_from(&file);
    let root = approved_root(&store, Path::new(&path))?;
    persist_project(&app, &file, &mut store, root)
}

fn persist_project(
    app: &AppHandle,
    file: &Path,
    store: &mut Store,
    root: PathBuf,
) -> Result<ProjectState, String> {
    let path = root.to_str().ok_or("Project path is not UTF-8")?.to_owned();
    push_recent(store, path, now_millis());
    save_to(file, store)?;
    Ok(state(store, home(app)))
}

#[tauri::command]
pub async fn pick_project(app: AppHandle) -> Result<Option<ProjectState>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let Some(selected) = app.dialog().file().blocking_pick_folder() else {
            return Ok(None);
        };
        let path = selected.into_path().map_err(|e| e.to_string())?;
        if path.is_dir() && !is_project(&path) {
            let init = app
                .dialog()
                .message("Everything will go to a .unelos folder.")
                .title("Initialize Unelos in this project?")
                .buttons(MessageDialogButtons::OkCancelCustom(
                    "Initialize".into(),
                    "Cancel".into(),
                ))
                .blocking_show();
            if !init {
                return Ok(None);
            }
            let utf8 = camino::Utf8Path::from_path(&path).ok_or("Project path is not UTF-8")?;
            unelos_core::initialize_project(utf8).map_err(|e| e.to_string())?;
        }
        let root = check_project(&path)?;
        let file = store_file(&app)?;
        let mut store = load_from(&file);
        persist_project(&app, &file, &mut store, root).map(Some)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub fn remove_recent(app: AppHandle, path: String) -> Result<ProjectState, String> {
    let file = store_file(&app)?;
    let mut store = load_from(&file);
    forget(&mut store, &path);
    save_to(&file, &store)?;
    Ok(state(&store, home(&app)))
}

fn uninstall_from(file: &Path, root: &Path, home: String) -> Result<ProjectState, String> {
    let root = approved_root(&load_from(file), root)?;
    let root = camino::Utf8Path::from_path(&root).ok_or("Project path is not UTF-8")?;
    unelos_core::uninstall_project(root).map_err(|e| e.to_string())?;
    let mut store = load_from(file);
    forget(&mut store, root.as_str());
    save_to(file, &store)
        .map_err(|e| format!("Uninstall succeeded, but saving recent projects failed: {e}"))?;
    Ok(state(&store, home))
}

#[tauri::command]
pub async fn uninstall_project(app: AppHandle, root: String) -> Result<ProjectState, String> {
    tauri::async_runtime::spawn_blocking(move || {
        uninstall_from(&store_file(&app)?, Path::new(&root), home(&app))
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub fn list_workflows(
    app: AppHandle,
    root: String,
) -> Result<Vec<unelos_core::WorkflowSummary>, String> {
    let root = approved_root(&load_from(&store_file(&app)?), Path::new(&root))?;
    let root = camino::Utf8Path::from_path(&root).ok_or("Project path is not UTF-8")?;
    unelos_core::workflow_summaries(root).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_agents(
    app: AppHandle,
    root: String,
) -> Result<Vec<unelos_core::AgentSummary>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let root = approved_root(&load_from(&store_file(&app)?), Path::new(&root))?;
        let root = camino::Utf8Path::from_path(&root).ok_or("Project path is not UTF-8")?;
        unelos_core::agent_summaries(root).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn create_workflow(app: AppHandle, root: String) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let root = approved_root(&load_from(&store_file(&app)?), Path::new(&root))?;
        let root = camino::Utf8Path::from_path(&root).ok_or("Project path is not UTF-8")?;
        unelos_core::create_workflow(root)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn get_workflow(
    app: AppHandle,
    root: String,
    id: String,
) -> Result<unelos_core::WorkflowDocument, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let root = approved_root(&load_from(&store_file(&app)?), Path::new(&root))?;
        let root = camino::Utf8Path::from_path(&root).ok_or("Project path is not UTF-8")?;
        unelos_core::get_workflow(root, &id)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn validate_workflow(
    app: AppHandle,
    root: String,
    id: String,
    draft: unelos_core::WorkflowDraft,
) -> Result<Vec<unelos_core::WorkflowIssue>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let root = approved_root(&load_from(&store_file(&app)?), Path::new(&root))?;
        let root = camino::Utf8Path::from_path(&root).ok_or("Project path is not UTF-8")?;
        unelos_core::validate_workflow(root, &id, &draft)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn save_workflow(
    app: AppHandle,
    root: String,
    id: String,
    draft: unelos_core::WorkflowDraft,
) -> Result<unelos_core::WorkflowDocument, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let root = approved_root(&load_from(&store_file(&app)?), Path::new(&root))?;
        let root = camino::Utf8Path::from_path(&root).ok_or("Project path is not UTF-8")?;
        unelos_core::save_workflow(root, &id, &draft)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn list_skills(
    app: AppHandle,
    root: String,
) -> Result<Vec<unelos_core::SkillSummary>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let root = approved_root(&load_from(&store_file(&app)?), Path::new(&root))?;
        let root = camino::Utf8Path::from_path(&root).ok_or("Project path is not UTF-8")?;
        unelos_core::skill_summaries(root).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn get_skill(
    app: AppHandle,
    root: String,
    id: String,
) -> Result<unelos_core::SkillDocument, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let root = approved_root(&load_from(&store_file(&app)?), Path::new(&root))?;
        let root = camino::Utf8Path::from_path(&root).ok_or("Project path is not UTF-8")?;
        unelos_core::get_skill(root, &id)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn create_skill(
    app: AppHandle,
    root: String,
    name: String,
    description: String,
    body: String,
) -> Result<unelos_core::SkillDocument, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let root = approved_root(&load_from(&store_file(&app)?), Path::new(&root))?;
        let root = camino::Utf8Path::from_path(&root).ok_or("Project path is not UTF-8")?;
        unelos_core::create_skill(root, &name, &description, &body)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn save_skill(
    app: AppHandle,
    root: String,
    id: String,
    name: String,
    description: String,
    body: String,
) -> Result<unelos_core::SaveSkillResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let root = approved_root(&load_from(&store_file(&app)?), Path::new(&root))?;
        let root = camino::Utf8Path::from_path(&root).ok_or("Project path is not UTF-8")?;
        unelos_core::save_skill(root, &id, &name, &description, &body)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn get_agent(
    app: AppHandle,
    root: String,
    id: String,
) -> Result<unelos_core::AgentDocument, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let root = approved_root(&load_from(&store_file(&app)?), Path::new(&root))?;
        let root = camino::Utf8Path::from_path(&root).ok_or("Project path is not UTF-8")?;
        unelos_core::get_agent(root, &id)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn save_agent(
    app: AppHandle,
    root: String,
    original_id: Option<String>,
    agent: unelos_core::AgentDocument,
) -> Result<unelos_core::AgentDocument, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let root = approved_root(&load_from(&store_file(&app)?), Path::new(&root))?;
        let root = camino::Utf8Path::from_path(&root).ok_or("Project path is not UTF-8")?;
        unelos_core::save_agent(root, original_id.as_deref(), agent)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn list_mcps(
    app: AppHandle,
    root: String,
) -> Result<Vec<unelos_core::McpSummary>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let root = approved_root(&load_from(&store_file(&app)?), Path::new(&root))?;
        let root = camino::Utf8Path::from_path(&root).ok_or("Project path is not UTF-8")?;
        unelos_core::mcp_summaries(root)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn get_mcp(
    app: AppHandle,
    root: String,
    id: String,
) -> Result<unelos_core::McpDocument, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let root = approved_root(&load_from(&store_file(&app)?), Path::new(&root))?;
        let root = camino::Utf8Path::from_path(&root).ok_or("Project path is not UTF-8")?;
        unelos_core::get_mcp(root, &id)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn save_mcp(
    app: AppHandle,
    root: String,
    original_id: Option<String>,
    mcp: unelos_core::McpDocument,
) -> Result<unelos_core::McpDocument, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let root = approved_root(&load_from(&store_file(&app)?), Path::new(&root))?;
        let root = camino::Utf8Path::from_path(&root).ok_or("Project path is not UTF-8")?;
        unelos_core::save_mcp(root, original_id.as_deref(), mcp)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn remove_mcp(app: AppHandle, root: String, id: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        let root = approved_root(&load_from(&store_file(&app)?), Path::new(&root))?;
        let root = camino::Utf8Path::from_path(&root).ok_or("Project path is not UTF-8")?;
        unelos_core::remove_mcp(root, &id)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn list_global_mcps() -> Result<Vec<unelos_core::McpSummary>, String> {
    tauri::async_runtime::spawn_blocking(unelos_core::global_mcp_summaries)
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn get_global_mcp(id: String) -> Result<unelos_core::McpDocument, String> {
    tauri::async_runtime::spawn_blocking(move || unelos_core::get_global_mcp(&id))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn save_global_mcp(
    original_id: Option<String>,
    mcp: unelos_core::McpDocument,
) -> Result<unelos_core::McpDocument, String> {
    tauri::async_runtime::spawn_blocking(move || {
        unelos_core::save_global_mcp(original_id.as_deref(), mcp)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn remove_global_mcp(
    app: AppHandle,
    root: Option<String>,
    id: String,
) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        let root = root
            .map(|root| approved_root(&load_from(&store_file(&app)?), Path::new(&root)))
            .transpose()?;
        let root = root
            .as_deref()
            .map(|root| camino::Utf8Path::from_path(root).ok_or("Project path is not UTF-8"))
            .transpose()?;
        unelos_core::remove_global_mcp(root, &id)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn list_global_skills() -> Result<Vec<unelos_core::SkillSummary>, String> {
    tauri::async_runtime::spawn_blocking(unelos_core::global_skill_summaries)
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn get_global_skill(id: String) -> Result<unelos_core::SkillDocument, String> {
    tauri::async_runtime::spawn_blocking(move || unelos_core::get_global_skill(&id))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn save_global_skill(
    app: AppHandle,
    root: Option<String>,
    id: String,
    name: String,
    description: String,
    body: String,
) -> Result<unelos_core::SaveSkillResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let root = root
            .map(|root| approved_root(&load_from(&store_file(&app)?), Path::new(&root)))
            .transpose()?;
        let root = root
            .as_deref()
            .map(|root| camino::Utf8Path::from_path(root).ok_or("Project path is not UTF-8"))
            .transpose()?;
        unelos_core::save_global_skill(root, &id, &name, &description, &body)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn create_global_skill(
    name: String,
    description: String,
    body: String,
) -> Result<unelos_core::SkillDocument, String> {
    tauri::async_runtime::spawn_blocking(move || {
        unelos_core::create_global_skill(&name, &description, &body)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn duplicate_global_skill(id: String, new_id: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || unelos_core::duplicate_global_skill(&id, &new_id))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn remove_global_skill(
    app: AppHandle,
    root: Option<String>,
    id: String,
) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        let root = root
            .map(|root| approved_root(&load_from(&store_file(&app)?), Path::new(&root)))
            .transpose()?;
        let root = root
            .as_deref()
            .map(|root| camino::Utf8Path::from_path(root).ok_or("Project path is not UTF-8"))
            .transpose()?;
        unelos_core::remove_global_skill(root, &id)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert_fs::TempDir;

    fn project(temp: &TempDir, name: &str) -> PathBuf {
        let root = temp.path().join(name);
        fs::create_dir_all(root.join(".unelos")).unwrap();
        fs::write(root.join(".unelos/project.yaml"), "{}").unwrap();
        root
    }

    #[test]
    fn uninstall_requires_approval_and_preserves_other_recents() {
        let temp = TempDir::new().unwrap();
        let root = fs::canonicalize(project(&temp, "target")).unwrap();
        let other = fs::canonicalize(project(&temp, "other")).unwrap();
        let file = temp.path().join("projects.json");
        assert!(uninstall_from(&file, &root, String::new()).is_err());
        assert!(root.join(".unelos").exists());
        let mut store = Store::default();
        push_recent(&mut store, other.to_str().unwrap().into(), 1);
        push_recent(&mut store, root.to_str().unwrap().into(), 2);
        save_to(&file, &store).unwrap();
        let result = uninstall_from(&file, &root, String::new()).unwrap();
        assert!(!root.join(".unelos").exists());
        assert!(other.join(".unelos").exists());
        assert_eq!(result.recents.len(), 1);
        assert_eq!(result.recents[0].path, other.to_str().unwrap());
        assert_eq!(load_from(&file).recents[0].path, other.to_str().unwrap());
    }

    #[test]
    fn uninstall_failure_preserves_recents_and_save_failure_is_truthful() {
        let temp = TempDir::new().unwrap();
        let root = fs::canonicalize(project(&temp, "target")).unwrap();
        let file = temp.path().join("projects.json");
        let mut store = Store::default();
        push_recent(&mut store, root.to_str().unwrap().into(), 1);
        save_to(&file, &store).unwrap();
        fs::create_dir_all(root.join(".claude/agents/unelos-directory.md")).unwrap();
        assert!(uninstall_from(&file, &root, String::new()).is_err());
        assert_eq!(load_from(&file), store);
        assert!(root.join(".unelos").exists());
        fs::remove_dir(root.join(".claude/agents/unelos-directory.md")).unwrap();
        fs::create_dir(file.with_extension("json.tmp")).unwrap();
        let error = uninstall_from(&file, &root, String::new()).unwrap_err();
        assert!(error.contains("Uninstall succeeded"));
        assert!(error.contains("recent"));
        assert!(!root.join(".unelos").exists());
        assert_eq!(load_from(&file), store);
    }

    #[test]
    fn push_moves_to_top_dedupes_and_caps() {
        let mut store = Store::default();
        for i in 0..12 {
            push_recent(&mut store, format!("/p{i}"), i);
        }
        push_recent(&mut store, "/p5".into(), 99);
        assert_eq!(store.recents.len(), MAX_RECENTS);
        assert_eq!(
            store.recents[0],
            StoredRecent {
                path: "/p5".into(),
                opened_at: 99
            }
        );
        assert_eq!(store.recents.iter().filter(|r| r.path == "/p5").count(), 1);
    }

    #[test]
    fn forget_removes_entry() {
        let mut store = Store::default();
        push_recent(&mut store, "/a".into(), 1);
        push_recent(&mut store, "/b".into(), 2);
        forget(&mut store, "/a");
        assert_eq!(store.recents.len(), 1);
        assert_eq!(store.recents[0].path, "/b");
    }

    #[test]
    fn save_then_load_round_trips_and_missing_is_empty() {
        let temp = TempDir::new().unwrap();
        let file = temp.path().join("nested/unelos/projects.json");
        assert_eq!(load_from(&file), Store::default());
        let mut store = Store::default();
        push_recent(&mut store, "/a".into(), 7);
        save_to(&file, &store).unwrap();
        assert_eq!(load_from(&file), store);
    }

    #[test]
    fn corrupt_file_is_empty() {
        let temp = TempDir::new().unwrap();
        let file = temp.path().join("projects.json");
        fs::write(&file, "{not json").unwrap();
        assert_eq!(load_from(&file), Store::default());
    }

    #[test]
    fn only_persisted_canonical_roots_are_approved() {
        let temp = TempDir::new().unwrap();
        let picked = fs::canonicalize(project(&temp, "picked")).unwrap();
        let other = project(&temp, "other");
        let mut store = Store::default();
        assert!(approved_root(&store, &picked).is_err());
        push_recent(&mut store, picked.to_str().unwrap().into(), 1);
        assert_eq!(approved_root(&store, &picked).unwrap(), picked);
        assert!(approved_root(&store, &other).is_err());
        assert!(approved_root(&store, &picked.join(".unelos")).is_err());
        fs::remove_file(picked.join(".unelos/project.yaml")).unwrap();
        assert!(approved_root(&store, &picked)
            .unwrap_err()
            .contains("project.yaml"));
        forget(&mut store, picked.to_str().unwrap());
        assert!(approved_root(&store, &picked).is_err());
    }

    #[test]
    fn agent_reads_require_an_approved_project_root() {
        let temp = TempDir::new().unwrap();
        let root = fs::canonicalize(project(&temp, "agents-project")).unwrap();
        assert!(approved_root(&Store::default(), &root).is_err());
        let mut store = Store::default();
        push_recent(&mut store, root.to_str().unwrap().to_owned(), 1);
        let approved = approved_root(&store, &root).unwrap();
        let result =
            unelos_core::agent_summaries(camino::Utf8Path::from_path(&approved).unwrap()).unwrap();
        assert!(result.is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn approval_does_not_follow_a_retargeted_root() {
        use std::os::unix::fs::symlink;
        let temp = TempDir::new().unwrap();
        let root = project(&temp, "picked");
        let other = project(&temp, "other");
        let mut store = Store::default();
        push_recent(&mut store, root.to_str().unwrap().into(), 1);
        fs::rename(&root, temp.path().join("moved")).unwrap();
        symlink(other, &root).unwrap();
        assert!(approved_root(&store, &root)
            .unwrap_err()
            .contains("location changed"));
    }

    #[test]
    fn failed_save_preserves_previous_recents() {
        let temp = TempDir::new().unwrap();
        let file = temp.path().join("projects.json");
        let mut store = Store::default();
        push_recent(&mut store, "/original".into(), 1);
        save_to(&file, &store).unwrap();
        let before = fs::read(&file).unwrap();
        fs::create_dir(file.with_extension("json.tmp")).unwrap();
        push_recent(&mut store, "/new".into(), 2);
        assert!(save_to(&file, &store).is_err());
        assert_eq!(fs::read(&file).unwrap(), before);
    }

    #[test]
    fn check_project_rejects_non_projects_and_canonicalizes() {
        let temp = TempDir::new().unwrap();
        let plain = temp.path().join("plain");
        fs::create_dir_all(&plain).unwrap();
        assert!(check_project(&plain)
            .unwrap_err()
            .contains("not a Unelos project"));
        assert!(check_project(&temp.path().join("gone"))
            .unwrap_err()
            .contains("no longer exists"));
        let root = project(&temp, "app");
        let with_slash = PathBuf::from(format!("{}/", root.display()));
        assert_eq!(
            check_project(&with_slash).unwrap(),
            fs::canonicalize(&root).unwrap()
        );
    }

    #[test]
    fn state_marks_availability_and_names() {
        let temp = TempDir::new().unwrap();
        let root = project(&temp, "app");
        let mut store = Store::default();
        push_recent(
            &mut store,
            temp.path().join("gone").to_string_lossy().into(),
            1,
        );
        push_recent(&mut store, root.to_string_lossy().into(), 2);
        let s = state(&store, "/home/x".into());
        assert_eq!(
            (s.recents[0].name.as_str(), s.recents[0].available),
            ("app", true)
        );
        assert_eq!(
            (s.recents[1].name.as_str(), s.recents[1].available),
            ("gone", false)
        );
    }
}

#[tauri::command]
pub async fn duplicate_task(
    app: AppHandle,
    root: String,
    id: String,
    new_id: String,
) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        let root = approved_root(&load_from(&store_file(&app)?), Path::new(&root))?;
        unelos_core::duplicate_task(
            camino::Utf8Path::from_path(&root).ok_or("Project path is not UTF-8")?,
            &id,
            &new_id,
        )
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn remove_task(app: AppHandle, root: String, id: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        let root = approved_root(&load_from(&store_file(&app)?), Path::new(&root))?;
        unelos_core::remove_task(
            camino::Utf8Path::from_path(&root).ok_or("Project path is not UTF-8")?,
            &id,
        )
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn list_tasks(
    app: AppHandle,
    root: String,
) -> Result<Vec<unelos_core::TaskSummary>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let root = approved_root(&load_from(&store_file(&app)?), Path::new(&root))?;
        let root = camino::Utf8Path::from_path(&root).ok_or("Project path is not UTF-8")?;
        unelos_core::task_summaries(root).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn get_task(
    app: AppHandle,
    root: String,
    id: String,
) -> Result<unelos_core::TaskDocument, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let root = approved_root(&load_from(&store_file(&app)?), Path::new(&root))?;
        let root = camino::Utf8Path::from_path(&root).ok_or("Project path is not UTF-8")?;
        unelos_core::get_task(root, &id)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn save_task(
    app: AppHandle,
    root: String,
    original_id: Option<String>,
    task: unelos_core::TaskDocument,
    outcome_renames: Option<std::collections::BTreeMap<String, String>>,
) -> Result<unelos_core::TaskDocument, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let root = approved_root(&load_from(&store_file(&app)?), Path::new(&root))?;
        let root = camino::Utf8Path::from_path(&root).ok_or("Project path is not UTF-8")?;
        unelos_core::save_task_with_outcome_renames(
            root,
            original_id.as_deref(),
            task,
            outcome_renames.unwrap_or_default(),
        )
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn list_global_tasks() -> Result<Vec<unelos_core::TaskSummary>, String> {
    tauri::async_runtime::spawn_blocking(unelos_core::global_task_summaries)
        .await
        .map_err(|e| e.to_string())?
}
#[tauri::command]
pub async fn get_global_task(id: String) -> Result<unelos_core::TaskDocument, String> {
    tauri::async_runtime::spawn_blocking(move || unelos_core::get_global_task(&id))
        .await
        .map_err(|e| e.to_string())?
}
#[tauri::command]
pub async fn save_global_task(
    original_id: Option<String>,
    task: unelos_core::TaskDocument,
) -> Result<unelos_core::TaskDocument, String> {
    tauri::async_runtime::spawn_blocking(move || {
        unelos_core::save_global_task(original_id.as_deref(), task)
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
pub async fn duplicate_global_task(id: String, new_id: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || unelos_core::duplicate_global_task(&id, &new_id))
        .await
        .map_err(|e| e.to_string())?
}
#[tauri::command]
pub async fn remove_global_task(id: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || unelos_core::remove_global_task(&id))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn get_project_settings(
    app: AppHandle,
    root: String,
) -> Result<unelos_core::ProjectSettings, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let root = approved_root(&load_from(&store_file(&app)?), Path::new(&root))?;
        unelos_core::get_project_settings(
            camino::Utf8Path::from_path(&root).ok_or("Project path is not UTF-8")?,
        )
        .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
pub async fn save_project_settings(
    app: AppHandle,
    root: String,
    settings: unelos_core::ProjectSettings,
) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        let root = approved_root(&load_from(&store_file(&app)?), Path::new(&root))?;
        unelos_core::save_project_settings(
            camino::Utf8Path::from_path(&root).ok_or("Project path is not UTF-8")?,
            &settings,
        )
        .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
pub async fn build_harness_config(
    app: AppHandle,
    root: String,
) -> Result<unelos_core::HarnessBuild, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let root = approved_root(&load_from(&store_file(&app)?), Path::new(&root))?;
        unelos_core::build_harness_config(
            camino::Utf8Path::from_path(&root).ok_or("Project path is not UTF-8")?,
        )
        .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
pub async fn auto_build_harness(
    app: AppHandle,
    root: String,
) -> Result<Option<unelos_core::HarnessBuild>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let root = approved_root(&load_from(&store_file(&app)?), Path::new(&root))?;
        unelos_core::auto_build_harness(
            camino::Utf8Path::from_path(&root).ok_or("Project path is not UTF-8")?,
        )
        .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn make_skill_global(app: AppHandle, root: String, id: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        let root = approved_root(&load_from(&store_file(&app)?), Path::new(&root))?;
        unelos_core::make_skill_global(
            camino::Utf8Path::from_path(&root).ok_or("Project path is not UTF-8")?,
            &id,
        )
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn make_mcp_global(app: AppHandle, root: String, id: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        let root = approved_root(&load_from(&store_file(&app)?), Path::new(&root))?;
        unelos_core::make_mcp_global(
            camino::Utf8Path::from_path(&root).ok_or("Project path is not UTF-8")?,
            &id,
        )
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn make_task_global(app: AppHandle, root: String, id: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        let root = approved_root(&load_from(&store_file(&app)?), Path::new(&root))?;
        unelos_core::make_task_global(
            camino::Utf8Path::from_path(&root).ok_or("Project path is not UTF-8")?,
            &id,
        )
    })
    .await
    .map_err(|e| e.to_string())?
}
