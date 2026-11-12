use super::{path_error, store::RunStore, CurrentWork, RunState, RuntimeError};
use camino::Utf8Path;
use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunListItem {
    pub run_id: String,
    pub name: String,
    pub workflow_name: String,
    pub status: &'static str,
    pub node_id: String,
    pub started_at: Option<u64>,
    pub updated_at: Option<u64>,
}

pub fn list_runs(root: &Utf8Path) -> Result<Vec<RunListItem>, RuntimeError> {
    let store = RunStore::open(root)?;
    let mut runs: Vec<_> = store
        .runs()?
        .into_iter()
        .map(|state| {
            let (status, node_id) = match state.current {
                CurrentWork::Task(r) | CurrentWork::Parallel(r) => ("running", r.node_id),
                CurrentWork::HumanGate(r) => ("waiting", r.node_id),
                CurrentWork::Finished { node_id } => ("finished", node_id),
            };
            RunListItem {
                run_id: state.run_id,
                name: state.name,
                workflow_name: state.workflow_name,
                status,
                node_id,
                started_at: state.started_at,
                updated_at: state.updated_at,
            }
        })
        .collect();
    runs.sort_by(|a, b| {
        b.updated_at
            .cmp(&a.updated_at)
            .then(a.run_id.cmp(&b.run_id))
    });
    Ok(runs)
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunDetails {
    #[serde(flatten)]
    pub state: RunState,
    pub existing_artifact_paths: Vec<String>,
}

pub fn get_run(root: &Utf8Path, run_id: &str) -> Result<RunDetails, RuntimeError> {
    let state = RunStore::open(root)?.run(run_id)?;
    let mut existing_artifact_paths = Vec::new();
    for path in artifact_paths(&state) {
        super::store::reject_symlink(path)?;
        match std::fs::metadata(path) {
            Ok(metadata) if metadata.is_file() => {
                if !existing_artifact_paths.iter().any(|existing| existing == path.as_str()) {
                    existing_artifact_paths.push(path.to_string());
                }
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(path_error(path, error)),
        }
    }
    Ok(RunDetails { state, existing_artifact_paths })
}

pub fn delete_run(root: &Utf8Path, run_id: &str) -> Result<(), RuntimeError> {
    let store = RunStore::open(root)?;
    store.run(run_id)?;
    let path = store.root().join(".unelos/runs").join(run_id);
    super::store::reject_symlink(&path)?;
    std::fs::remove_dir_all(&path).map_err(|e| path_error(&path, e))
}

pub fn read_run_artifact(
    root: &Utf8Path,
    run_id: &str,
    path: &Utf8Path,
) -> Result<String, RuntimeError> {
    let store = RunStore::open(root)?;
    let state = store.run(run_id)?;
    let referenced = artifact_paths(&state).any(|reference| reference == path);
    if !referenced {
        return Err(RuntimeError::InvalidState(format!(
            "artifact is not referenced by run {run_id}: {path}"
        )));
    }
    super::store::reject_symlink(path)?;
    let metadata = std::fs::metadata(path).map_err(|e| path_error(path, e))?;
    if !metadata.is_file() {
        return Err(RuntimeError::InvalidState(format!(
            "artifact is not a regular file: {path}"
        )));
    }
    std::fs::read_to_string(path).map_err(|e| path_error(path, e))
}

fn artifact_paths(state: &RunState) -> impl Iterator<Item = &Utf8Path> {
    std::iter::once(&state.current)
        .chain(state.history.iter().map(|h| &h.work))
        .filter_map(|work| {
            let record = match work {
                CurrentWork::Task(r) | CurrentWork::Parallel(r) | CurrentWork::HumanGate(r) => r,
                CurrentWork::Finished { .. } => return None,
            };
            Some(record)
        })
        .flat_map(|record| record
                .inputs
                .iter()
                .chain(&record.outputs)
                .chain(record.worker_reports.iter().map(|r| &r.report))
                .chain(record.feedback.iter().flat_map(|f| &f.artifacts))
                .map(|reference| reference.path.as_path()))
}
