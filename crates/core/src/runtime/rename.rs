use super::{
    store::{self, RunStore},
    valid_run_name, CurrentWork, RunState, RuntimeError,
};
use camino::{Utf8Path, Utf8PathBuf};
use serde::{Deserialize, Serialize};
use std::{fs, io};

const INTENT_FILE: &str = ".unelos/rename.json";

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct RenameIntent {
    version: u8,
    old_name: String,
    new_name: String,
    original: RunState,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum RenameStage {
    IntentWritten,
    DirectoryMoved,
    StateWritten,
    PointerWritten,
    BeforeClear,
}

pub fn rename_run(root: &Utf8Path, run_id: &str, name: &str) -> Result<RunState, RuntimeError> {
    if !valid_run_name(name) {
        return Err(RuntimeError::InvalidName { name: name.into() });
    }
    let store = RunStore::open(root)?;
    let state = store.active()?;
    if state.run_id != run_id {
        return Err(RuntimeError::StaleRun {
            expected: run_id.into(),
            current: state.run_id,
        });
    }
    if state.name == name {
        if !state.naming_pending {
            return Ok(state);
        }
        let mut settled = state;
        settled.naming_pending = false;
        store.update(&settled)?;
        return Ok(settled);
    }
    rename_with_hook(&store, &state, name, |_| Ok(()))?;
    store.active()
}

fn rename_with_hook(
    store: &RunStore,
    original: &RunState,
    name: &str,
    mut hook: impl FnMut(RenameStage) -> Result<(), RuntimeError>,
) -> Result<(), RuntimeError> {
    if !valid_run_name(name) {
        return Err(RuntimeError::InvalidName { name: name.into() });
    }
    let active = store.active()?;
    if active != *original {
        return Err(RuntimeError::StaleRun {
            expected: original.run_id.clone(),
            current: active.run_id,
        });
    }
    if !valid_run_name(&original.name) || !valid_run_name(&original.run_id) {
        return Err(RuntimeError::InvalidState(
            "invalid run identity for rename".into(),
        ));
    }
    let paths = paths(store.root(), &original.name, name);
    ensure_directory(&store.root().join(".unelos/runs"))?;
    if paths.old == paths.new {
        return Ok(());
    }
    ensure_directory(&paths.old)?;
    ensure_absent(&paths.new)?;
    let intent_path = store.root().join(INTENT_FILE);
    store::reject_symlink(&intent_path)?;
    if exists(&intent_path)? {
        return Err(recovery_error(&intent_path, "rename intent already exists"));
    }
    validate_state_at(original, &paths.old)?;
    let intent = RenameIntent {
        version: 1,
        old_name: original.name.clone(),
        new_name: name.into(),
        original: original.clone(),
    };
    write_intent(&intent_path, &intent)?;
    hook(RenameStage::IntentWritten)?;
    fs::rename(&paths.old, &paths.new).map_err(|source| super::path_error(&paths.old, source))?;
    hook(RenameStage::DirectoryMoved)?;
    finish(store.root(), &intent, &paths, &mut hook)
}

pub(super) fn recover(store: &RunStore) -> Result<(), RuntimeError> {
    let intent_path = store.root().join(INTENT_FILE);
    store::reject_symlink(&intent_path)?;
    let bytes = match fs::read(&intent_path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(source) => return Err(super::path_error(&intent_path, source)),
    };
    let intent: RenameIntent =
        serde_json::from_slice(&bytes).map_err(|source| RuntimeError::Json {
            path: intent_path.clone(),
            source,
        })?;
    ensure_directory(&store.root().join(".unelos/runs"))?;
    validate_intent(store, &intent)?;
    let paths = paths(store.root(), &intent.old_name, &intent.new_name);
    let pointer_path = store.root().join(".unelos/active-run");
    store::reject_symlink(&pointer_path)?;
    let pointer = fs::read_to_string(&pointer_path)
        .map_err(|source| recovery_path_error(&pointer_path, source))?;
    if pointer != intent.old_name && pointer != intent.new_name {
        return Err(recovery_error(
            &pointer_path,
            "active pointer matches neither rename name",
        ));
    }
    let old_exists = exists(&paths.old)?;
    let new_exists = exists(&paths.new)?;
    match (old_exists, new_exists) {
        (true, false) => {
            ensure_directory(&paths.old)?;
            let saved = read_state(&paths.old)?;
            if saved != intent.original {
                return Err(recovery_error(
                    &paths.old,
                    "old run differs from recorded state",
                ));
            }
            validate_state_at(&saved, &paths.old)?;
            fs::rename(&paths.old, &paths.new)
                .map_err(|source| recovery_path_error(&paths.old, source))?;
        }
        (false, true) => {
            ensure_directory(&paths.new)?;
            let saved = read_state(&paths.new)?;
            let candidate = candidate(store.root(), &intent)?;
            if saved != intent.original && saved != candidate {
                return Err(recovery_error(
                    &paths.new,
                    "new run has unrelated state or identity",
                ));
            }
        }
        (true, true) => {
            return Err(recovery_error(
                &paths.new,
                "both old and new run directories exist",
            ))
        }
        (false, false) => {
            return Err(recovery_error(
                &paths.old,
                "neither old nor new run directory exists",
            ))
        }
    }
    let mut no_hook = |_| Ok(());
    finish(store.root(), &intent, &paths, &mut no_hook)
}

fn validate_intent(store: &RunStore, intent: &RenameIntent) -> Result<(), RuntimeError> {
    if intent.version != 1
        || !valid_run_name(&intent.old_name)
        || !valid_run_name(&intent.new_name)
        || intent.old_name == intent.new_name
        || intent.original.name != intent.old_name
        || !valid_run_name(&intent.original.run_id)
    {
        return Err(recovery_error(
            &store.root().join(INTENT_FILE),
            "invalid rename intent version, names, or identity",
        ));
    }
    store::validate_state(&intent.original)?;
    Ok(())
}

fn finish(
    root: &Utf8Path,
    intent: &RenameIntent,
    paths: &RenamePaths,
    hook: &mut impl FnMut(RenameStage) -> Result<(), RuntimeError>,
) -> Result<(), RuntimeError> {
    ensure_directory(&paths.new)?;
    let candidate = candidate(root, intent)?;
    validate_state_at(&candidate, &paths.new)?;
    let state_path = paths.new.join("state.json");
    store::reject_symlink(&state_path)?;
    let bytes = serde_json::to_vec(&candidate).map_err(|source| RuntimeError::Json {
        path: state_path.clone(),
        source,
    })?;
    store::atomic_write(&state_path, &bytes)?;
    hook(RenameStage::StateWritten)?;
    let pointer = root.join(".unelos/active-run");
    store::reject_symlink(&pointer)?;
    store::atomic_write(&pointer, intent.new_name.as_bytes())?;
    hook(RenameStage::PointerWritten)?;
    hook(RenameStage::BeforeClear)?;
    let journal = root.join(INTENT_FILE);
    fs::remove_file(&journal).map_err(|source| recovery_path_error(&journal, source))
}

fn candidate(root: &Utf8Path, intent: &RenameIntent) -> Result<RunState, RuntimeError> {
    let paths = paths(root, &intent.old_name, &intent.new_name);
    let mut state = intent.original.clone();
    state.name = intent.new_name.clone();
    state.naming_pending = false;
    rebase_work(&mut state.current, &paths.old, &paths.new)?;
    for completed in &mut state.history {
        rebase_work(&mut completed.work, &paths.old, &paths.new)?;
    }
    Ok(state)
}

fn rebase_work(work: &mut CurrentWork, old: &Utf8Path, new: &Utf8Path) -> Result<(), RuntimeError> {
    let record = match work {
        CurrentWork::Task(record)
        | CurrentWork::Parallel(record)
        | CurrentWork::HumanGate(record) => record,
        CurrentWork::Finished { .. } => return Ok(()),
    };
    for reference in record.inputs.iter_mut().chain(&mut record.outputs).chain(
        record
            .worker_reports
            .iter_mut()
            .map(|report| &mut report.report),
    ) {
        rebase_reference(reference, old, new)?;
    }
    if let Some(feedback) = &mut record.feedback {
        for reference in &mut feedback.artifacts {
            rebase_reference(reference, old, new)?;
        }
    }
    Ok(())
}

fn rebase_reference(
    reference: &mut crate::ArtifactReference,
    old: &Utf8Path,
    new: &Utf8Path,
) -> Result<(), RuntimeError> {
    let relative = reference.path.strip_prefix(old).map_err(|_| {
        RuntimeError::InvalidState(format!("artifact escapes old run: {}", reference.path))
    })?;
    reference.path = new.join(relative);
    Ok(())
}

fn validate_state_at(state: &RunState, directory: &Utf8Path) -> Result<(), RuntimeError> {
    store::validate_state(state)?;
    store::validate_artifacts(state, directory)
}

fn read_state(directory: &Utf8Path) -> Result<RunState, RuntimeError> {
    let path = directory.join("state.json");
    store::reject_symlink(&path)?;
    let bytes = fs::read(&path).map_err(|source| recovery_path_error(&path, source))?;
    serde_json::from_slice(&bytes).map_err(|source| RuntimeError::Json { path, source })
}

fn write_intent(path: &Utf8Path, intent: &RenameIntent) -> Result<(), RuntimeError> {
    let bytes = serde_json::to_vec(intent).map_err(|source| RuntimeError::Json {
        path: path.to_owned(),
        source,
    })?;
    store::atomic_write(path, &bytes)
}

struct RenamePaths {
    old: Utf8PathBuf,
    new: Utf8PathBuf,
}

fn paths(root: &Utf8Path, old: &str, new: &str) -> RenamePaths {
    let runs = root.join(".unelos/runs");
    RenamePaths {
        old: runs.join(old),
        new: runs.join(new),
    }
}

fn ensure_directory(path: &Utf8Path) -> Result<(), RuntimeError> {
    store::reject_symlink(path)?;
    let metadata = fs::metadata(path).map_err(|source| recovery_path_error(path, source))?;
    if !metadata.is_dir() {
        return Err(recovery_error(path, "run path is not a directory"));
    }
    Ok(())
}

fn ensure_absent(path: &Utf8Path) -> Result<(), RuntimeError> {
    match fs::symlink_metadata(path) {
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Ok(_) => Err(RuntimeError::Collision {
            name: path.file_name().unwrap_or_default().into(),
        }),
        Err(source) => Err(super::path_error(path, source)),
    }
}

fn exists(path: &Utf8Path) -> Result<bool, RuntimeError> {
    match fs::symlink_metadata(path) {
        Ok(_) => Ok(true),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(source) => Err(super::path_error(path, source)),
    }
}

fn recovery_error(path: &Utf8Path, message: &str) -> RuntimeError {
    RuntimeError::InvalidState(format!("rename recovery required at {path}: {message}"))
}

fn recovery_path_error(path: &Utf8Path, source: io::Error) -> RuntimeError {
    RuntimeError::Io {
        path: path.to_owned(),
        source,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert_fs::{fixture::PathCopy, TempDir};

    fn project() -> (TempDir, Utf8PathBuf) {
        let temp = TempDir::new().unwrap();
        temp.copy_from(
            Utf8Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/project"),
            &["**/*"],
        )
        .unwrap();
        let root = Utf8Path::from_path(temp.path()).unwrap().to_owned();
        (temp, root)
    }

    #[test]
    fn interrupted_rename_recovers_at_each_boundary() {
        for stop in [
            RenameStage::IntentWritten,
            RenameStage::DirectoryMoved,
            RenameStage::StateWritten,
            RenameStage::PointerWritten,
            RenameStage::BeforeClear,
        ] {
            let (_temp, root) = project();
            let started = crate::start_workflow(&root, "implement-feature", None).unwrap();
            let CurrentWork::Task(record) = &started.current else {
                panic!("task");
            };
            fs::write(&record.outputs[0].path, "preserve").unwrap();
            let store = RunStore::open(&root).unwrap();
            let error = rename_with_hook(&store, &started, "recovered", |stage| {
                if stage == stop {
                    return Err(RuntimeError::Io {
                        path: root.join(INTENT_FILE),
                        source: io::Error::other("injected interruption"),
                    });
                }
                Ok(())
            });
            assert!(error.is_err());
            drop(store);
            let recovered = crate::status(&root).unwrap();
            assert_eq!(recovered.run_id, started.run_id);
            assert_eq!(recovered.name, "recovered");
            assert!(!recovered.naming_pending);
            assert!(!root.join(INTENT_FILE).exists());
            assert!(!root.join(".unelos/runs").join(&started.name).exists());
            assert_eq!(
                fs::read_to_string(root.join(".unelos/runs/recovered/plan/plan.md")).unwrap(),
                "preserve"
            );
            assert_eq!(
                fs::read_to_string(root.join(".unelos/active-run")).unwrap(),
                "recovered"
            );
        }
    }

    #[test]
    fn malformed_or_ambiguous_intents_fail_closed() {
        for mutation in ["json", "version", "name", "both", "pointer", "destination"] {
            let (_temp, root) = project();
            let original = crate::start_workflow(&root, "implement-feature", None).unwrap();
            let intent_path = root.join(INTENT_FILE);
            let intent = RenameIntent {
                version: 1,
                old_name: original.name.clone(),
                new_name: "new-name".into(),
                original: original.clone(),
            };
            write_intent(&intent_path, &intent).unwrap();
            let old = root.join(".unelos/runs").join(&original.name);
            let new = root.join(".unelos/runs/new-name");
            match mutation {
                "json" => fs::write(&intent_path, b"{").unwrap(),
                "version" => {
                    let mut value: serde_json::Value =
                        serde_json::from_slice(&fs::read(&intent_path).unwrap()).unwrap();
                    value["version"] = 2.into();
                    fs::write(&intent_path, serde_json::to_vec(&value).unwrap()).unwrap();
                }
                "name" => {
                    let mut value: serde_json::Value =
                        serde_json::from_slice(&fs::read(&intent_path).unwrap()).unwrap();
                    value["newName"] = "../outside".into();
                    fs::write(&intent_path, serde_json::to_vec(&value).unwrap()).unwrap();
                }
                "both" => fs::create_dir(&new).unwrap(),
                "pointer" => fs::write(root.join(".unelos/active-run"), "some-other-run").unwrap(),
                "destination" => {
                    fs::rename(&old, &new).unwrap();
                    let mut unrelated = original.clone();
                    unrelated.request = "unrelated".into();
                    fs::write(
                        new.join("state.json"),
                        serde_json::to_vec(&unrelated).unwrap(),
                    )
                    .unwrap();
                }
                _ => unreachable!(),
            }
            assert!(crate::status(&root).is_err(), "accepted {mutation} intent");
            assert!(intent_path.exists(), "cleared {mutation} intent");
        }
    }

    #[cfg(unix)]
    #[test]
    fn recovery_rejects_symlinked_source_or_intent() {
        use std::os::unix::fs::symlink;

        for symlink_intent in [false, true] {
            let (_temp, root) = project();
            let original = crate::start_workflow(&root, "implement-feature", None).unwrap();
            let intent_path = root.join(INTENT_FILE);
            let intent = RenameIntent {
                version: 1,
                old_name: original.name.clone(),
                new_name: "new-name".into(),
                original,
            };
            write_intent(&intent_path, &intent).unwrap();
            if symlink_intent {
                let actual = root.join("actual-intent.json");
                fs::rename(&intent_path, &actual).unwrap();
                symlink(&actual, &intent_path).unwrap();
            } else {
                let old = root.join(".unelos/runs").join(&intent.old_name);
                let saved = root.join("saved-run");
                fs::rename(&old, &saved).unwrap();
                symlink(saved, old).unwrap();
            }
            assert!(crate::status(&root).is_err());
            assert!(intent_path.exists());
        }
    }
}
