use crate::projects::{approved_root, load_from, store_file};
use std::{
    collections::{BTreeMap, HashMap},
    path::Path,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
};
use tauri::{AppHandle, Manager, State};
use unelos_core::{FetchedSource, ImportOutcome, ImportPlan};

pub struct Session {
    cancel: Arc<AtomicBool>,
    fetched: Option<FetchedSource>,
    source: String,
}

pub type ImportSessions = Mutex<HashMap<String, Session>>;
static COMMIT: Mutex<()> = Mutex::new(());

#[derive(serde::Serialize)]
#[serde(tag = "status", rename_all = "camelCase")]
pub enum CommitResult {
    Imported {
        workflow: String,
        #[serde(rename = "warningCount")]
        warning_count: usize,
    },
    Conflicts {
        plan: ImportPlan,
    },
}

#[tauri::command]
pub async fn import_prepare(
    app: AppHandle,
    root: String,
    id: String,
    source: String,
) -> Result<ImportPlan, String> {
    let cancel = Arc::new(AtomicBool::new(false));
    {
        let state = app.state::<ImportSessions>();
        let mut sessions = state.lock().map_err(|e| e.to_string())?;
        if sessions.contains_key(&id) {
            return Err("Import session already exists".into());
        }
        sessions.insert(
            id.clone(),
            Session {
                cancel: cancel.clone(),
                fetched: None,
                source: source.clone(),
            },
        );
    }
    tauri::async_runtime::spawn_blocking(move || {
        let result = (|| {
            let root = approved_root(&load_from(&store_file(&app)?), Path::new(&root))?;
            let root = camino::Utf8Path::from_path(&root).ok_or("Project path is not UTF-8")?;
            let fetched = unelos_core::fetch_source(&source, &cancel).map_err(|e| e.to_string())?;
            let plan = unelos_core::plan_import(root, fetched.dir(), &source, &BTreeMap::new())
                .map_err(|e| e.to_string())?;
            Ok::<_, String>((fetched, plan))
        })();
        let state = app.state::<ImportSessions>();
        let mut sessions = state.lock().map_err(|e| e.to_string())?;
        if cancel.load(Ordering::Relaxed) {
            return Err("Import cancelled".into());
        }
        match result {
            Ok((fetched, plan)) => {
                let session = sessions.get_mut(&id).ok_or("Import cancelled")?;
                session.fetched = Some(fetched);
                Ok(plan)
            }
            Err(error) => {
                sessions.remove(&id);
                Err(error)
            }
        }
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn import_plan(
    app: AppHandle,
    root: String,
    id: String,
    renames: BTreeMap<String, String>,
) -> Result<ImportPlan, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let root = approved_root(&load_from(&store_file(&app)?), Path::new(&root))?;
        let root = camino::Utf8Path::from_path(&root).ok_or("Project path is not UTF-8")?;
        let state = app.state::<ImportSessions>();
        let sessions = state.lock().map_err(|e| e.to_string())?;
        let session = sessions.get(&id).ok_or("Import session not found")?;
        let fetched = session
            .fetched
            .as_ref()
            .ok_or("Import is still preparing")?;
        unelos_core::plan_import(root, fetched.dir(), &session.source, &renames)
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn import_commit(
    app: AppHandle,
    root: String,
    id: String,
    renames: BTreeMap<String, String>,
) -> Result<CommitResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let _commit = COMMIT.lock().map_err(|e| e.to_string())?;
        let root = approved_root(&load_from(&store_file(&app)?), Path::new(&root))?;
        let root = camino::Utf8Path::from_path(&root).ok_or("Project path is not UTF-8")?;
        let state = app.state::<ImportSessions>();
        let mut sessions = state.lock().map_err(|e| e.to_string())?;
        let session = sessions.get(&id).ok_or("Import session not found")?;
        let fetched = session
            .fetched
            .as_ref()
            .ok_or("Import is still preparing")?;
        match unelos_core::commit_import(root, fetched.dir(), &session.source, &renames)
            .map_err(|e| e.to_string())?
        {
            ImportOutcome::Imported {
                workflow,
                warning_count,
                ..
            } => {
                sessions.remove(&id);
                Ok(CommitResult::Imported {
                    workflow,
                    warning_count,
                })
            }
            ImportOutcome::Conflicts(plan) => Ok(CommitResult::Conflicts { plan }),
        }
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub fn import_cancel(state: State<'_, ImportSessions>, id: String) -> Result<(), String> {
    if let Some(session) = state.lock().map_err(|e| e.to_string())?.remove(&id) {
        session.cancel.store(true, Ordering::Relaxed);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn imported_response_matches_contract() {
        let result = CommitResult::Imported {
            workflow: "review".into(),
            warning_count: 2,
        };
        assert_eq!(
            serde_json::to_value(result).unwrap(),
            serde_json::json!({"status": "imported", "workflow": "review", "warningCount": 2})
        );
    }
}
