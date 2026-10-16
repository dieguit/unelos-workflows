use serde::{Deserialize, Serialize};
use std::{
    fs, io,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};
use tauri::{AppHandle, Manager};

const MAX_RECENTS: usize = 10;

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
    fs::write(file, text).map_err(|e| e.to_string())
}

pub fn push_recent(store: &mut Store, path: String, now: u64) {
    store.recents.retain(|r| r.path != path);
    store.recents.insert(0, StoredRecent { path, opened_at: now });
    store.recents.truncate(MAX_RECENTS);
}

pub fn forget(store: &mut Store, path: &str) {
    store.recents.retain(|r| r.path != path);
}

/// Returns the canonical project root or a user-facing error.
pub fn check_project(path: &Path) -> Result<PathBuf, String> {
    let name = folder_name(path);
    if !path.is_dir() {
        return Err(format!("{name} couldn’t be opened: the folder no longer exists"));
    }
    if !is_project(path) {
        return Err(format!("{name} is not a Unelos project (no .unelos folder)"));
    }
    fs::canonicalize(path).map_err(|e| format!("{name} couldn’t be opened: {e}"))
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

fn store_file(app: &AppHandle) -> Result<PathBuf, String> {
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
    let root = check_project(Path::new(&path))?;
    let file = store_file(&app)?;
    let mut store = load_from(&file);
    push_recent(&mut store, root.to_string_lossy().into_owned(), now_millis());
    save_to(&file, &store)?;
    Ok(state(&store, home(&app)))
}

#[tauri::command]
pub fn remove_recent(app: AppHandle, path: String) -> Result<ProjectState, String> {
    let file = store_file(&app)?;
    let mut store = load_from(&file);
    forget(&mut store, &path);
    save_to(&file, &store)?;
    Ok(state(&store, home(&app)))
}

#[tauri::command]
pub fn list_workflows(root: String) -> Result<Vec<unelos_core::WorkflowSummary>, String> {
    unelos_core::workflow_summaries(camino::Utf8Path::new(&root)).map_err(|e| e.to_string())
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
    fn push_moves_to_top_dedupes_and_caps() {
        let mut store = Store::default();
        for i in 0..12 {
            push_recent(&mut store, format!("/p{i}"), i);
        }
        push_recent(&mut store, "/p5".into(), 99);
        assert_eq!(store.recents.len(), MAX_RECENTS);
        assert_eq!(store.recents[0], StoredRecent { path: "/p5".into(), opened_at: 99 });
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
    fn check_project_rejects_non_projects_and_canonicalizes() {
        let temp = TempDir::new().unwrap();
        let plain = temp.path().join("plain");
        fs::create_dir_all(&plain).unwrap();
        assert!(check_project(&plain).unwrap_err().contains("not a Unelos project"));
        assert!(check_project(&temp.path().join("gone")).unwrap_err().contains("no longer exists"));
        let root = project(&temp, "app");
        let with_slash = PathBuf::from(format!("{}/", root.display()));
        assert_eq!(check_project(&with_slash).unwrap(), fs::canonicalize(&root).unwrap());
    }

    #[test]
    fn state_marks_availability_and_names() {
        let temp = TempDir::new().unwrap();
        let root = project(&temp, "app");
        let mut store = Store::default();
        push_recent(&mut store, temp.path().join("gone").to_string_lossy().into(), 1);
        push_recent(&mut store, root.to_string_lossy().into(), 2);
        let s = state(&store, "/home/x".into());
        assert_eq!((s.recents[0].name.as_str(), s.recents[0].available), ("app", true));
        assert_eq!((s.recents[1].name.as_str(), s.recents[1].available), ("gone", false));
    }
}
