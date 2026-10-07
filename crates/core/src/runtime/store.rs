use super::{path_error, valid_run_name, CurrentWork, RunState, RuntimeError};
use camino::{Utf8Path, Utf8PathBuf};
use std::{
    fs::{self, File, OpenOptions},
    io::{self, Write},
    sync::atomic::{AtomicU64, Ordering},
};

static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

pub(super) struct RunStore {
    root: Utf8PathBuf,
    lock: File,
}

impl RunStore {
    pub(super) fn open(root: &Utf8Path) -> Result<Self, RuntimeError> {
        let root =
            crate::project::absolute_root(root).map_err(|source| path_error(root, source))?;
        let unelos = root.join(".unelos");
        ensure_directory(&unelos)?;
        let lock_path = unelos.join("runtime.lock");
        reject_symlink(&lock_path)?;
        let lock = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .truncate(false)
            .open(&lock_path)
            .map_err(|source| path_error(&lock_path, source))?;
        lock.lock()
            .map_err(|source| path_error(&lock_path, source))?;
        Ok(Self { root, lock })
    }

    pub(super) fn active(&self) -> Result<RunState, RuntimeError> {
        let runtime = self.root.join(".unelos");
        ensure_directory(&runtime)?;
        let pointer_path = runtime.join("active-run");
        reject_symlink(&pointer_path)?;
        let pointer = match fs::read_to_string(&pointer_path) {
            Ok(pointer) => pointer,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                return Err(RuntimeError::NoActiveRun { path: pointer_path });
            }
            Err(source) => return Err(path_error(&pointer_path, source)),
        };
        let name = pointer.as_str();
        if !valid_run_name(name) {
            return Err(RuntimeError::InvalidName { name: name.into() });
        }
        let runs_path = runtime.join("runs");
        check_directory(&runs_path)?;
        let run_path = runs_path.join(name);
        reject_symlink(&run_path)?;
        reject_symlink(&run_path.join("ledger.md"))?;
        let state_path = run_path.join("state.json");
        reject_symlink(&state_path)?;
        let bytes = fs::read(&state_path).map_err(|source| path_error(&state_path, source))?;
        let state: RunState =
            serde_json::from_slice(&bytes).map_err(|source| RuntimeError::Json {
                path: state_path,
                source,
            })?;
        validate_state(&state)?;
        validate_artifacts(&state, &run_path)?;
        if state.name != name || state.run_id != name {
            return Err(RuntimeError::InvalidState(format!(
                "active pointer {name:?} does not match state name/run ID {:?}/{:?}",
                state.name, state.run_id
            )));
        }
        Ok(state)
    }

    pub(super) fn publish(
        &self,
        state: &RunState,
        prepare: impl FnOnce(&Utf8Path) -> Result<(), RuntimeError>,
    ) -> Result<(), RuntimeError> {
        self.publish_with(state, prepare, atomic_write)
    }

    fn publish_with(
        &self,
        state: &RunState,
        prepare: impl FnOnce(&Utf8Path) -> Result<(), RuntimeError>,
        write: impl Fn(&Utf8Path, &[u8]) -> Result<(), RuntimeError>,
    ) -> Result<(), RuntimeError> {
        if !valid_run_name(&state.name) {
            return Err(RuntimeError::InvalidName {
                name: state.name.clone(),
            });
        }
        if state.run_id != state.name {
            return Err(RuntimeError::InvalidState(
                "run ID must match run name".into(),
            ));
        }
        validate_state(state)?;
        let runtime = self.root.join(".unelos");
        ensure_directory(&runtime)?;
        let runs = runtime.join("runs");
        ensure_directory(&runs)?;
        let run_dir = runs.join(&state.name);
        match fs::symlink_metadata(&run_dir) {
            Ok(_) => {
                return Err(RuntimeError::Collision {
                    name: state.name.clone(),
                })
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(source) => return Err(path_error(&run_dir, source)),
        }
        fs::create_dir(&run_dir).map_err(|source| {
            if source.kind() == io::ErrorKind::AlreadyExists {
                RuntimeError::Collision {
                    name: state.name.clone(),
                }
            } else {
                path_error(&run_dir, source)
            }
        })?;
        prepare(&run_dir)?;
        let state_path = run_dir.join("state.json");
        let bytes = serde_json::to_vec(state).map_err(|source| RuntimeError::Json {
            path: state_path.clone(),
            source,
        })?;
        write(&state_path, &bytes)?;
        write(&runtime.join("active-run"), state.name.as_bytes())
    }
}

impl Drop for RunStore {
    fn drop(&mut self) {
        let _ = self.lock.unlock();
    }
}

fn validate_state(state: &RunState) -> Result<(), RuntimeError> {
    if state.next_execution_sequence == 0 {
        return Err(RuntimeError::InvalidState(
            "next execution sequence must be positive".into(),
        ));
    }
    let (identity, gate) = match &state.current {
        CurrentWork::Task(record) | CurrentWork::Parallel(record) => {
            (Some(record.execution_id.as_str()), false)
        }
        CurrentWork::HumanGate(record) => (Some(record.execution_id.as_str()), true),
        CurrentWork::Finished { .. } => (None, false),
    };
    if let Some(identity) = identity {
        let prefix = if gate { "gate-" } else { "task-" };
        let sequence = identity
            .strip_prefix(prefix)
            .and_then(|number| number.parse::<u64>().ok())
            .filter(|number| *number > 0 && number.to_string() == identity[prefix.len()..])
            .ok_or_else(|| {
                RuntimeError::InvalidState(format!(
                    "malformed current execution identity {identity:?}"
                ))
            })?;
        if state.next_execution_sequence <= sequence {
            return Err(RuntimeError::InvalidState(format!(
                "next execution sequence {} must exceed current identity {identity:?}",
                state.next_execution_sequence
            )));
        }
    }
    Ok(())
}

fn validate_artifacts(state: &RunState, run_path: &Utf8Path) -> Result<(), RuntimeError> {
    fn binding(
        reference: &crate::ArtifactReference,
        run_path: &Utf8Path,
        expected_node: Option<&str>,
    ) -> Result<(), RuntimeError> {
        let relative = reference.path.strip_prefix(run_path).map_err(|_| {
            RuntimeError::InvalidState(format!("artifact path escapes run: {}", reference.path))
        })?;
        let components: Vec<_> = relative.components().collect();
        if components.len() < 2 {
            return Err(RuntimeError::InvalidState(format!(
                "artifact path is outside a Node directory: {}",
                reference.path
            )));
        }
        if relative
            .as_str()
            .split(|separator| separator == '/' || (cfg!(windows) && separator == '\\'))
            .any(|part| matches!(part, "" | "." | ".."))
        {
            return Err(RuntimeError::InvalidState(format!(
                "artifact path has a noncanonical component: {}",
                reference.path
            )));
        }
        let mut path = run_path.to_owned();
        for (index, component) in components.into_iter().enumerate() {
            let camino::Utf8Component::Normal(part) = component else {
                return Err(RuntimeError::InvalidState(format!(
                    "artifact path has a non-normal component: {}",
                    reference.path
                )));
            };
            super::check_component(part, "persisted artifact")?;
            path.push(part);
            reject_symlink(&path)?;
            if index == 0 && expected_node.is_some_and(|node| part != node) {
                return Err(RuntimeError::InvalidState(format!(
                    "artifact path is outside its expected Node directory: {}",
                    reference.path
                )));
            }
        }
        Ok(())
    }

    fn feedback(
        feedback: Option<&crate::TransitionFeedback>,
        run_path: &Utf8Path,
    ) -> Result<(), RuntimeError> {
        if let Some(feedback) = feedback {
            super::check_component(&feedback.source_node_id, "feedback Node")?;
            for artifact in &feedback.artifacts {
                binding(artifact, run_path, Some(&feedback.source_node_id))?;
            }
        }
        Ok(())
    }

    fn work(work: &super::CurrentWork, run_path: &Utf8Path) -> Result<(), RuntimeError> {
        let record = match work {
            super::CurrentWork::Task(record)
            | super::CurrentWork::Parallel(record)
            | super::CurrentWork::HumanGate(record) => record,
            super::CurrentWork::Finished { node_id } => {
                super::check_component(node_id, "Node")?;
                return Ok(());
            }
        };
        super::check_component(&record.node_id, "Node")?;
        for artifact in &record.inputs {
            binding(artifact, run_path, None)?;
        }
        for artifact in &record.outputs {
            binding(artifact, run_path, Some(&record.node_id))?;
        }
        for report in &record.worker_reports {
            binding(&report.report, run_path, Some(&record.node_id))?;
        }
        feedback(record.feedback.as_ref(), run_path)
    }

    work(&state.current, run_path)?;
    for completed in &state.history {
        work(&completed.work, run_path)?;
    }
    Ok(())
}

fn reject_symlink(path: &Utf8Path) -> Result<(), RuntimeError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => Err(path_error(
            path,
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "symlinked runtime path is not allowed",
            ),
        )),
        Ok(_) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(source) => Err(path_error(path, source)),
    }
}

fn ensure_directory(path: &Utf8Path) -> Result<(), RuntimeError> {
    reject_symlink(path)?;
    match fs::create_dir(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
            let metadata = fs::metadata(path).map_err(|source| path_error(path, source))?;
            if metadata.is_dir() {
                Ok(())
            } else {
                Err(path_error(
                    path,
                    io::Error::new(io::ErrorKind::NotADirectory, "not a directory"),
                ))
            }
        }
        Err(source) => Err(path_error(path, source)),
    }
}

pub(super) fn ensure_runtime_directory(path: &Utf8Path) -> Result<(), RuntimeError> {
    ensure_directory(path)
}

fn check_directory(path: &Utf8Path) -> Result<(), RuntimeError> {
    reject_symlink(path)?;
    let metadata = fs::metadata(path).map_err(|source| path_error(path, source))?;
    if metadata.is_dir() {
        Ok(())
    } else {
        Err(path_error(
            path,
            io::Error::new(io::ErrorKind::NotADirectory, "not a directory"),
        ))
    }
}

fn atomic_write(target: &Utf8Path, bytes: &[u8]) -> Result<(), RuntimeError> {
    atomic_write_with(target, |file| file.write_all(bytes))
}

fn atomic_write_with(
    target: &Utf8Path,
    write: impl FnOnce(&mut File) -> io::Result<()>,
) -> Result<(), RuntimeError> {
    let parent = target.parent().expect("atomic target has parent");
    let file_name = target.file_name().expect("atomic target has filename");
    let mut write = Some(write);
    for _ in 0..100 {
        let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let temp = parent.join(format!(
            ".{file_name}.{}.{}.tmp",
            std::process::id(),
            sequence
        ));
        let mut file = match OpenOptions::new().write(true).create_new(true).open(&temp) {
            Ok(file) => file,
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(source) => return Err(path_error(&temp, source)),
        };
        let result =
            write.take().expect("writer used once")(&mut file).and_then(|()| file.sync_all());
        drop(file);
        let result = result.and_then(|()| fs::rename(&temp, target));
        if let Err(source) = result {
            let _ = fs::remove_file(&temp);
            return Err(path_error(target, source));
        }
        return Ok(());
    }
    Err(path_error(
        target,
        io::Error::new(
            io::ErrorKind::AlreadyExists,
            "could not reserve a unique temporary file",
        ),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn root() -> (assert_fs::TempDir, Utf8PathBuf) {
        let temp = assert_fs::TempDir::new().unwrap();
        let path = Utf8PathBuf::from_path_buf(temp.to_path_buf()).unwrap();
        (temp, path)
    }

    fn state(name: &str) -> RunState {
        RunState {
            run_id: name.into(),
            name: name.into(),
            workflow_name: "flow".into(),
            request: "request".into(),
            current: CurrentWork::Finished {
                node_id: "done".into(),
            },
            history: vec![],
            next_execution_sequence: 1,
        }
    }

    #[test]
    fn publish_and_active_roundtrip() {
        let (_temp, root) = root();
        let store = RunStore::open(&root).unwrap();
        let state = state("roundtrip");
        store.publish(&state, |_| Ok(())).unwrap();
        assert_eq!(store.active().unwrap(), state);
    }

    #[test]
    fn missing_active_pointer_does_not_create_run_storage() {
        let (_temp, root) = root();
        let store = RunStore::open(&root).unwrap();
        assert!(matches!(
            store.active(),
            Err(RuntimeError::NoActiveRun { .. })
        ));
        assert!(!root.join(".unelos/runs").exists());
        assert!(!root.join(".unelos/active-run").exists());
    }

    #[test]
    fn active_rejects_unsafe_pointer_missing_state_corrupt_json_and_mismatch() {
        let (_temp, root) = root();
        let store = RunStore::open(&root).unwrap();
        let runtime = root.join(".unelos");
        fs::write(runtime.join("active-run"), "../outside").unwrap();
        assert!(matches!(
            store.active(),
            Err(RuntimeError::InvalidName { .. })
        ));
        fs::write(runtime.join("active-run"), "missing").unwrap();
        assert!(store.active().is_err());
        let run = runtime.join("runs/malformed");
        fs::create_dir_all(&run).unwrap();
        fs::write(run.join("state.json"), "{").unwrap();
        fs::write(runtime.join("active-run"), "malformed").unwrap();
        assert!(matches!(store.active(), Err(RuntimeError::Json { .. })));
        fs::write(
            run.join("state.json"),
            serde_json::to_vec(&state("different")).unwrap(),
        )
        .unwrap();
        assert!(matches!(store.active(), Err(RuntimeError::InvalidState(_))));
    }

    #[test]
    fn publish_rejects_collisions_without_changing_active_pointer() {
        let (_temp, root) = root();
        let store = RunStore::open(&root).unwrap();
        store.publish(&state("first"), |_| Ok(())).unwrap();
        let pointer = fs::read(root.join(".unelos/active-run")).unwrap();
        assert!(matches!(
            store.publish(&state("first"), |_| Ok(())),
            Err(RuntimeError::Collision { .. })
        ));
        assert_eq!(fs::read(root.join(".unelos/active-run")).unwrap(), pointer);
    }

    #[test]
    fn pointer_write_failure_keeps_previous_active_run() {
        let (_temp, root) = root();
        let store = RunStore::open(&root).unwrap();
        store.publish(&state("prior"), |_| Ok(())).unwrap();
        let pointer = fs::read(root.join(".unelos/active-run")).unwrap();
        let result = store.publish_with(
            &state("next"),
            |_| Ok(()),
            |path, bytes| {
                if path.file_name() == Some("active-run") {
                    return Err(path_error(
                        path,
                        io::Error::other("injected pointer failure"),
                    ));
                }
                atomic_write(path, bytes)
            },
        );
        assert!(result.is_err());
        assert_eq!(fs::read(root.join(".unelos/active-run")).unwrap(), pointer);
        assert_eq!(store.active().unwrap(), state("prior"));
        let unpublished = root.join(".unelos/runs/next/state.json");
        assert_eq!(
            serde_json::from_slice::<RunState>(&fs::read(unpublished).unwrap()).unwrap(),
            state("next")
        );
    }

    #[test]
    fn atomic_write_does_not_publish_partial_data_on_writer_or_rename_failure() {
        let (_temp, root) = root();
        let target = root.join("target");
        fs::write(&target, "old").unwrap();
        let error = atomic_write_with(&target, |file| {
            file.write_all(b"prefix")?;
            Err(io::Error::other("injected failure"))
        });
        assert!(error.is_err());
        assert_eq!(fs::read(&target).unwrap(), b"old");
        assert_eq!(fs::read_dir(&root).unwrap().count(), 1);
        atomic_write(&target, b"replacement").unwrap();
        assert_eq!(fs::read(&target).unwrap(), b"replacement");
        let error = atomic_write_with(&target, |file| {
            file.write_all(b"prefix")?;
            Err(io::Error::other("injected failure"))
        });
        assert!(error.is_err());
        assert_eq!(fs::read(&target).unwrap(), b"replacement");
        let destination = root.join("directory");
        fs::create_dir(&destination).unwrap();
        assert!(atomic_write(&destination, b"new").is_err());
        assert!(destination.is_dir());
    }

    #[test]
    fn active_rejects_malformed_execution_identity_and_sequence() {
        let (_temp, root) = root();
        let store = RunStore::open(&root).unwrap();
        let runtime = root.join(".unelos");
        let run = runtime.join("runs/bad-state");
        fs::create_dir_all(&run).unwrap();
        let mut saved = state("bad-state");
        saved.current = CurrentWork::Task(crate::ExecutionRecord {
            node_id: "plan".into(),
            execution_id: "gate-1".into(),
            inputs: vec![],
            outputs: vec![],
            worker_reports: vec![],
            feedback: None,
        });
        fs::write(run.join("state.json"), serde_json::to_vec(&saved).unwrap()).unwrap();
        fs::write(runtime.join("active-run"), "bad-state").unwrap();
        assert!(matches!(store.active(), Err(RuntimeError::InvalidState(_))));
        saved.current = CurrentWork::Task(crate::ExecutionRecord {
            execution_id: "task-1".into(),
            ..match saved.current {
                CurrentWork::Task(record) => record,
                _ => unreachable!(),
            }
        });
        saved.next_execution_sequence = 1;
        fs::write(run.join("state.json"), serde_json::to_vec(&saved).unwrap()).unwrap();
        assert!(matches!(store.active(), Err(RuntimeError::InvalidState(_))));
    }

    #[cfg(unix)]
    #[test]
    fn active_rejects_symlinked_run_directory() {
        use std::os::unix::fs::symlink;
        let (_temp, root) = root();
        let store = RunStore::open(&root).unwrap();
        store.publish(&state("linked"), |_| Ok(())).unwrap();
        let run = root.join(".unelos/runs/linked");
        let backup = root.join("backup");
        fs::rename(&run, &backup).unwrap();
        symlink(&backup, &run).unwrap();
        assert!(store.active().is_err());
    }

    #[test]
    fn existing_lock_file_is_opened_as_lock_not_treated_as_locked() {
        let (_temp, root) = root();
        fs::create_dir_all(root.join(".unelos")).unwrap();
        fs::write(root.join(".unelos/runtime.lock"), "").unwrap();
        assert!(RunStore::open(&root).is_ok());
    }
}
