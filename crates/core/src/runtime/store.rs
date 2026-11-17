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

    pub(super) fn root(&self) -> &Utf8Path {
        &self.root
    }

    pub(super) fn runs(&self) -> Result<Vec<RunState>, RuntimeError> {
        let directory = self.root.join(".unelos/runs");
        reject_symlink(&directory)?;
        let entries = match fs::read_dir(&directory) {
            Ok(entries) => entries,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(vec![]),
            Err(source) => return Err(path_error(&directory, source)),
        };
        let mut runs = Vec::new();
        let mut ids = std::collections::BTreeSet::new();
        for entry in entries {
            let entry = entry.map_err(|source| path_error(&directory, source))?;
            let path = Utf8PathBuf::from_path_buf(entry.path())
                .map_err(|_| RuntimeError::InvalidState("run path is not UTF-8".into()))?;
            reject_symlink(&path)?;
            if !entry
                .file_type()
                .map_err(|source| path_error(&path, source))?
                .is_dir()
            {
                continue;
            }
            let name = path.file_name().unwrap();
            if !valid_run_name(name) {
                return Err(RuntimeError::InvalidName { name: name.into() });
            }
            reject_symlink(&path.join("ledger.md"))?;
            let state_path = path.join("state.json");
            reject_symlink(&state_path)?;
            if !fs::metadata(&state_path)
                .map_err(|source| path_error(&state_path, source))?
                .is_file()
            {
                return Err(RuntimeError::InvalidState(format!(
                    "state is not a regular file: {state_path}"
                )));
            }
            let bytes = fs::read(&state_path).map_err(|source| path_error(&state_path, source))?;
            let state: RunState =
                serde_json::from_slice(&bytes).map_err(|source| RuntimeError::Json {
                    path: state_path,
                    source,
                })?;
            validate_state(&state)?;
            validate_artifacts(&state, &path)?;
            if state.run_id != name {
                return Err(RuntimeError::InvalidState(format!(
                    "run directory {name:?} does not match state run ID {:?}",
                    state.run_id
                )));
            }
            if !ids.insert(state.run_id.clone()) {
                return Err(RuntimeError::DuplicateRunId {
                    run_id: state.run_id,
                });
            }
            runs.push(state);
        }
        runs.sort_by(|a, b| a.run_id.cmp(&b.run_id));
        Ok(runs)
    }

    pub(super) fn run(&self, run_id: &str) -> Result<RunState, RuntimeError> {
        if !valid_run_name(run_id) {
            return Err(RuntimeError::InvalidName {
                name: run_id.into(),
            });
        }
        // ponytail: linear validated scan; add an ID index only if run counts make lookup costly.
        self.runs()?
            .into_iter()
            .find(|state| state.run_id == run_id)
            .ok_or_else(|| RuntimeError::UnknownRun {
                run_id: run_id.into(),
            })
    }

    pub(super) fn ensure_start_available(&self, name: Option<&str>) -> Result<(), RuntimeError> {
        let runs = self.runs()?;
        if let Some(name) = name {
            if runs.iter().any(|state| state.name == name) {
                return Err(RuntimeError::Collision { name: name.into() });
            }
        }
        Ok(())
    }

    pub(super) fn allocate_run_id(&self) -> Result<String, RuntimeError> {
        let runtime = self.root.join(".unelos");
        let counter = runtime.join("run-sequence");
        reject_symlink(&counter)?;
        let bytes = match fs::read(&counter) {
            Ok(bytes) => Some(bytes),
            Err(error) if error.kind() == io::ErrorKind::NotFound => None,
            Err(source) => return Err(path_error(&counter, source)),
        };
        let mut sequence = if let Some(bytes) = bytes {
            let text = std::str::from_utf8(&bytes)
                .map_err(|_| RuntimeError::InvalidState("malformed run ID sequence".into()))?;
            let number = text
                .parse::<u64>()
                .map_err(|_| RuntimeError::InvalidState("malformed run ID sequence".into()))?;
            if number.to_string() != text {
                return Err(RuntimeError::InvalidState(
                    "malformed run ID sequence".into(),
                ));
            }
            number
        } else {
            0
        };

        let runs = runtime.join("runs");
        ensure_directory(&runs)?;
        let saved_id_max = self
            .runs()?
            .iter()
            .filter_map(|saved| numeric_run_id(&saved.run_id))
            .max()
            .unwrap_or(0);
        sequence = sequence.max(saved_id_max);

        loop {
            sequence = sequence
                .checked_add(1)
                .ok_or_else(|| RuntimeError::InvalidState("run ID sequence overflow".into()))?;
            let id = format!("r-{sequence:06}");
            let destination = runs.join(&id);
            let occupied = match fs::symlink_metadata(&destination) {
                Ok(_) => true,
                Err(error) if error.kind() == io::ErrorKind::NotFound => false,
                Err(source) => return Err(path_error(&destination, source)),
            };
            if occupied {
                continue;
            }
            atomic_write(&counter, sequence.to_string().as_bytes())?;
            return Ok(id);
        }
    }

    pub(super) fn publish(
        &self,
        state: &RunState,
        prepare: impl FnOnce(&Utf8Path) -> Result<(), RuntimeError>,
    ) -> Result<(), RuntimeError> {
        self.publish_with(state, prepare, atomic_write)
    }

    pub(super) fn update(&self, state: &RunState) -> Result<(), RuntimeError> {
        self.update_with(state, atomic_write)
    }

    fn update_with(
        &self,
        state: &RunState,
        write: impl Fn(&Utf8Path, &[u8]) -> Result<(), RuntimeError>,
    ) -> Result<(), RuntimeError> {
        let active = self.run(&state.run_id)?;
        if state.run_id != active.run_id {
            return Err(RuntimeError::InvalidState(
                "run update does not match selected run identity".into(),
            ));
        }
        validate_state(state)?;
        if state.name != active.name {
            self.ensure_start_available(Some(&state.name))?;
        }
        let run_path = self.root.join(".unelos/runs").join(&state.run_id);
        validate_artifacts(state, &run_path)?;
        let state_path = run_path.join("state.json");
        reject_symlink(&state_path)?;
        if !fs::metadata(&state_path)
            .map_err(|source| path_error(&state_path, source))?
            .is_file()
        {
            return Err(path_error(
                &state_path,
                io::Error::new(io::ErrorKind::InvalidInput, "state is not a regular file"),
            ));
        }
        let mut state = state.clone();
        state.started_at = active.started_at;
        state.updated_at = active.updated_at;
        // Idempotent replays must not bump updatedAt or rewrite state.
        if state == active {
            return Ok(());
        }
        state.updated_at = Some(now_millis());
        let bytes = serde_json::to_vec(&state).map_err(|source| RuntimeError::Json {
            path: state_path.clone(),
            source,
        })?;
        write(&state_path, &bytes)
    }

    fn publish_with(
        &self,
        state: &RunState,
        prepare: impl FnOnce(&Utf8Path) -> Result<(), RuntimeError>,
        write: impl Fn(&Utf8Path, &[u8]) -> Result<(), RuntimeError>,
    ) -> Result<(), RuntimeError> {
        validate_state(state)?;
        let runtime = self.root.join(".unelos");
        ensure_directory(&runtime)?;
        let runs = runtime.join("runs");
        ensure_directory(&runs)?;
        self.ensure_start_available(Some(&state.name))?;
        let run_dir = runs.join(&state.run_id);
        match fs::symlink_metadata(&run_dir) {
            Ok(_) => {
                return Err(RuntimeError::Collision {
                    name: state.name.clone(),
                })
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(source) => return Err(path_error(&run_dir, source)),
        }
        if self
            .runs()?
            .iter()
            .any(|saved| saved.run_id == state.run_id)
        {
            return Err(RuntimeError::DuplicateRunId {
                run_id: state.run_id.clone(),
            });
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
        let mut state = state.clone();
        let now = now_millis();
        state.started_at = Some(now);
        state.updated_at = Some(now);
        let bytes = serde_json::to_vec(&state).map_err(|source| RuntimeError::Json {
            path: state_path.clone(),
            source,
        })?;
        write(&state_path, &bytes)
    }
}

impl Drop for RunStore {
    fn drop(&mut self) {
        let _ = self.lock.unlock();
    }
}

fn now_millis() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

pub(super) fn validate_state(state: &RunState) -> Result<(), RuntimeError> {
    if !valid_run_name(&state.name) || !valid_run_name(&state.run_id) {
        return Err(RuntimeError::InvalidState("invalid run ID or name".into()));
    }
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
    fn validate_receipt(work: &CurrentWork) -> Result<(), RuntimeError> {
        let record = match work {
            CurrentWork::Task(record) | CurrentWork::Parallel(record) => record,
            CurrentWork::HumanGate(record) => {
                if record.confirmed_execution_id.is_some() {
                    return Err(RuntimeError::InvalidState(
                        "human-gate work cannot have output confirmation".into(),
                    ));
                }
                return Ok(());
            }
            CurrentWork::Finished { .. } => return Ok(()),
        };
        for (key, result) in &record.worker_receipts {
            if !matches!(work, CurrentWork::Parallel(_))
                || !record
                    .worker_reports
                    .iter()
                    .any(|report| &report.key == key)
                || result.worker_key.as_ref() != Some(key)
                || result.task_id != record.execution_id
                || result.status != crate::WorkerStatus::Finished
                || result.outcome.is_some()
            {
                return Err(RuntimeError::InvalidState(format!(
                    "invalid worker receipt for key {key:?}"
                )));
            }
        }
        for path in record.output_digests.keys() {
            if !record
                .outputs
                .iter()
                .chain(record.worker_reports.iter().map(|r| &r.report))
                .any(|output| &output.path == path)
            {
                return Err(RuntimeError::InvalidState(format!(
                    "unexpected output snapshot path {path}"
                )));
            }
        }
        if let Some(confirmed) = &record.confirmed_execution_id {
            if confirmed != &record.execution_id {
                return Err(RuntimeError::InvalidState(format!(
                    "confirmation {confirmed:?} does not match execution {:?}",
                    record.execution_id
                )));
            }
        }
        Ok(())
    }
    validate_receipt(&state.current)?;
    for completed in &state.history {
        validate_receipt(&completed.work)?;
    }
    Ok(())
}

fn numeric_run_id(run_id: &str) -> Option<u64> {
    let digits = run_id.strip_prefix("r-")?;
    if digits.is_empty() || !digits.bytes().all(|digit| digit.is_ascii_digit()) {
        return None;
    }
    let number = digits.parse::<u64>().ok()?;
    (number > 0).then_some(number)
}

pub(super) fn validate_artifacts(
    state: &RunState,
    run_path: &Utf8Path,
) -> Result<(), RuntimeError> {
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
        if let Some(node) = expected_node {
            let node_path = super::node_dir(run_path, node);
            if reference.path.strip_prefix(&node_path).is_err() || reference.path == node_path {
                return Err(RuntimeError::InvalidState(format!(
                    "artifact path is outside its expected Node directory: {}",
                    reference.path
                )));
            }
        }
        for component in components {
            let camino::Utf8Component::Normal(part) = component else {
                return Err(RuntimeError::InvalidState(format!(
                    "artifact path has a non-normal component: {}",
                    reference.path
                )));
            };
            super::check_component(part, "persisted artifact")?;
            path.push(part);
            reject_symlink(&path)?;
        }
        Ok(())
    }

    fn feedback(
        feedback: Option<&crate::TransitionFeedback>,
        state: &RunState,
        run_path: &Utf8Path,
    ) -> Result<(), RuntimeError> {
        if let Some(feedback) = feedback {
            super::check_node_id(&feedback.source_node_id, "feedback Node")?;
            let source_gate = state.history.iter().find_map(|entry| match &entry.work {
                CurrentWork::HumanGate(record)
                    if record.node_id == feedback.source_node_id
                        && record.execution_id == feedback.source_execution_id =>
                {
                    Some(record)
                }
                _ => None,
            });
            if let Some(gate) = source_gate {
                if feedback.artifacts != gate.inputs {
                    return Err(RuntimeError::InvalidState(
                        "gate feedback does not match presented artifacts".into(),
                    ));
                }
            }
            let expected_node =
                source_gate.map_or(Some(feedback.source_node_id.as_str()), |_| None);
            for artifact in &feedback.artifacts {
                binding(artifact, run_path, expected_node)?;
            }
        }
        Ok(())
    }

    fn work(
        work: &super::CurrentWork,
        state: &RunState,
        run_path: &Utf8Path,
    ) -> Result<(), RuntimeError> {
        let record = match work {
            super::CurrentWork::Task(record)
            | super::CurrentWork::Parallel(record)
            | super::CurrentWork::HumanGate(record) => record,
            super::CurrentWork::Finished { node_id } => {
                super::check_node_id(node_id, "Node")?;
                return Ok(());
            }
        };
        super::check_node_id(&record.node_id, "Node")?;
        for artifact in &record.inputs {
            binding(artifact, run_path, None)?;
        }
        for artifact in &record.outputs {
            binding(artifact, run_path, Some(&record.node_id))?;
        }
        for report in &record.worker_reports {
            binding(&report.report, run_path, Some(&record.node_id))?;
        }
        feedback(record.feedback.as_ref(), state, run_path)
    }

    work(&state.current, state, run_path)?;
    for completed in &state.history {
        work(&completed.work, state, run_path)?;
    }
    Ok(())
}

pub(super) fn reject_symlink(path: &Utf8Path) -> Result<(), RuntimeError> {
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

pub(super) fn atomic_write(target: &Utf8Path, bytes: &[u8]) -> Result<(), RuntimeError> {
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
            started_at: None,
            updated_at: None,
            run_id: name.into(),
            name: name.into(),
            workflow_name: "flow".into(),
            request: "request".into(),
            naming_pending: false,
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
        let mut state = state("roundtrip");
        store.publish(&state, |_| Ok(())).unwrap();
        let saved = store.run("roundtrip").unwrap();
        state.started_at = saved.started_at;
        state.updated_at = saved.updated_at;
        assert_eq!(saved, state);
    }

    #[test]
    fn missing_active_pointer_does_not_create_run_storage() {
        let (_temp, root) = root();
        let store = RunStore::open(&root).unwrap();
        assert!(matches!(
            store.run("missing"),
            Err(RuntimeError::UnknownRun { .. })
        ));
        assert!(!root.join(".unelos/runs").exists());
        assert!(!root.join(".unelos/active-run").exists());
    }

    #[test]
    fn lookup_ignores_pointer_and_rejects_missing_state_corrupt_json_and_mismatch() {
        let (_temp, root) = root();
        let store = RunStore::open(&root).unwrap();
        let runtime = root.join(".unelos");
        fs::write(runtime.join("active-run"), "../outside").unwrap();
        assert!(store.runs().unwrap().is_empty());
        fs::write(runtime.join("active-run"), "missing").unwrap();
        assert!(store.run("missing").is_err());
        let run = runtime.join("runs/malformed");
        fs::create_dir_all(&run).unwrap();
        assert!(store.runs().is_err());
        fs::write(run.join("state.json"), "{").unwrap();
        fs::write(runtime.join("active-run"), "malformed").unwrap();
        assert!(matches!(
            store.run("malformed"),
            Err(RuntimeError::Json { .. })
        ));
        fs::write(
            run.join("state.json"),
            serde_json::to_vec(&state("different")).unwrap(),
        )
        .unwrap();
        assert!(matches!(
            store.run("malformed"),
            Err(RuntimeError::InvalidState(_))
        ));
    }

    #[test]
    fn publish_rejects_collisions_without_changing_active_pointer() {
        let (_temp, root) = root();
        let store = RunStore::open(&root).unwrap();
        store.publish(&state("first"), |_| Ok(())).unwrap();
        let before = fs::read(root.join(".unelos/runs/first/state.json")).unwrap();
        assert!(matches!(
            store.publish(&state("first"), |_| Ok(())),
            Err(RuntimeError::Collision { .. })
        ));
        assert_eq!(
            fs::read(root.join(".unelos/runs/first/state.json")).unwrap(),
            before
        );
    }

    #[test]
    fn publication_state_write_failure_preserves_prior_run_bytes() {
        let (_temp, root) = root();
        let store = RunStore::open(&root).unwrap();
        store.publish(&state("prior"), |_| Ok(())).unwrap();
        let prior = fs::read(root.join(".unelos/runs/prior/state.json")).unwrap();
        let result = store.publish_with(
            &state("next"),
            |_| Ok(()),
            |path, bytes| {
                if path.file_name() == Some("state.json") {
                    return Err(path_error(path, io::Error::other("injected state failure")));
                }
                atomic_write(path, bytes)
            },
        );
        assert!(result.is_err());
        assert_eq!(
            fs::read(root.join(".unelos/runs/prior/state.json")).unwrap(),
            prior
        );
        let unpublished = root.join(".unelos/runs/next/state.json");
        assert!(!unpublished.exists());
        assert!(!root.join(".unelos/active-run").exists());
    }

    #[test]
    fn failed_update_writer_preserves_saved_state_and_active_pointer() {
        let (_temp, root) = root();
        let store = RunStore::open(&root).unwrap();
        let original = state("update-run");
        store.publish(&original, |_| Ok(())).unwrap();
        let original = store.run(&original.run_id).unwrap();
        let state_path = root.join(".unelos/runs/update-run/state.json");
        let before = fs::read(&state_path).unwrap();
        let mut changed = original.clone();
        changed.request = "changed".into();
        let error = store.update_with(&changed, |path, _| {
            Err(path_error(
                path,
                io::Error::other("injected update failure"),
            ))
        });
        assert!(error.is_err());
        assert_eq!(fs::read(&state_path).unwrap(), before);
        assert!(!root.join(".unelos/active-run").exists());
        assert_eq!(store.run(&original.run_id).unwrap(), original);
    }

    #[test]
    fn failed_update_of_prepared_completion_preserves_run_and_pointer() {
        use assert_fs::fixture::PathCopy;

        let (_temp, project_root) = root();
        let fixture = Utf8Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/project");
        _temp.copy_from(fixture, &["**/*"]).unwrap();
        let project = crate::load_project(&project_root, "implement-feature").unwrap();
        let started = crate::start(&project, "atomic-completion", "Request").unwrap();
        let record = match &started.current {
            CurrentWork::Task(record) => record,
            _ => unreachable!(),
        };
        fs::write(&record.outputs[0].path, "plan").unwrap();
        let store = RunStore::open(&project_root).unwrap();
        let active = store.run("atomic-completion").unwrap();
        let before =
            fs::read(project_root.join(".unelos/runs/atomic-completion/state.json")).unwrap();
        let project = crate::load_project(&project_root, "implement-feature").unwrap();
        let mut confirmed = active.clone();
        if let CurrentWork::Task(record) | CurrentWork::Parallel(record) = &mut confirmed.current {
            record.confirmed_execution_id = Some(record.execution_id.clone());
        } else {
            unreachable!();
        }
        let candidate = super::super::completion::prepare_completion(
            &project, &confirmed, "task-1", None, None,
        )
        .unwrap();
        let error = store.update_with(&candidate, |path, _| {
            Err(path_error(
                path,
                io::Error::other("injected completion persistence failure"),
            ))
        });
        assert!(error.is_err());
        assert_eq!(
            fs::read(project_root.join(".unelos/runs/atomic-completion/state.json")).unwrap(),
            before
        );
        assert!(!project_root.join(".unelos/active-run").exists());
        assert_eq!(store.run("atomic-completion").unwrap(), active);
        assert_eq!(
            fs::read(project_root.join(".unelos/runs/atomic-completion/state.json")).unwrap(),
            before
        );
    }

    #[test]
    fn failed_gate_decision_updates_preserve_run_and_pointer() {
        use assert_fs::fixture::PathCopy;

        for (outcome, reason) in [("approved", None), ("rejected", Some("revise"))] {
            let (_temp, project_root) = root();
            let fixture = Utf8Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/project");
            _temp.copy_from(fixture, &["**/*"]).unwrap();
            let project = crate::load_project(&project_root, "implement-feature").unwrap();
            let started = crate::start(&project, "atomic-gate", "Request").unwrap();
            let plan = match &started.current {
                CurrentWork::Task(record) => record,
                _ => unreachable!(),
            };
            fs::write(&plan.outputs[0].path, "plan").unwrap();
            crate::confirm_outputs(&project_root, "atomic-gate", "task-1").unwrap();
            crate::complete(&project_root, "atomic-gate", "task-1", None, None).unwrap();

            let store = RunStore::open(&project_root).unwrap();
            let active = store.run("atomic-gate").unwrap();
            let state_path = project_root.join(".unelos/runs/atomic-gate/state.json");
            let before = fs::read(&state_path).unwrap();
            let project = crate::load_project(&project_root, "implement-feature").unwrap();
            let candidate =
                super::super::gates::prepare_decision(&project, &active, "gate-2", outcome, reason)
                    .unwrap();
            let error = store.update_with(&candidate, |path, _| {
                Err(path_error(
                    path,
                    io::Error::other("injected gate decision persistence failure"),
                ))
            });
            assert!(error.is_err());
            assert_eq!(fs::read(state_path).unwrap(), before);
            assert!(!project_root.join(".unelos/active-run").exists());
            assert_eq!(store.run("atomic-gate").unwrap(), active);
        }
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
            output_digests: Default::default(),
            worker_receipts: Default::default(),
            invalid_worker_results: 0,
            feedback: None,
            confirmed_execution_id: None,
        });
        fs::write(run.join("state.json"), serde_json::to_vec(&saved).unwrap()).unwrap();
        fs::write(runtime.join("active-run"), "bad-state").unwrap();
        assert!(matches!(
            store.run("bad-state"),
            Err(RuntimeError::InvalidState(_))
        ));
        saved.current = CurrentWork::Task(crate::ExecutionRecord {
            execution_id: "task-1".into(),
            ..match saved.current {
                CurrentWork::Task(record) => record,
                _ => unreachable!(),
            }
        });
        saved.next_execution_sequence = 1;
        fs::write(run.join("state.json"), serde_json::to_vec(&saved).unwrap()).unwrap();
        assert!(matches!(
            store.run("bad-state"),
            Err(RuntimeError::InvalidState(_))
        ));
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
        assert!(store.run("linked").is_err());
    }

    #[test]
    fn existing_lock_file_is_opened_as_lock_not_treated_as_locked() {
        let (_temp, root) = root();
        fs::create_dir_all(root.join(".unelos")).unwrap();
        fs::write(root.join(".unelos/runtime.lock"), "").unwrap();
        assert!(RunStore::open(&root).is_ok());
    }
}
