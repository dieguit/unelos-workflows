use camino::{Utf8Path, Utf8PathBuf};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::fs::{self, File, OpenOptions};

pub type SessionError = crate::ExecutionError;

pub(crate) fn fail(code: &'static str, message: impl ToString) -> SessionError {
    SessionError::Protocol {
        code,
        message: message.to_string(),
    }
}
pub(crate) fn io_error(error: impl ToString) -> SessionError {
    fail("binding_io_failed", error)
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionContext {
    pub harness: String,
    pub session_id: String,
    pub project: Utf8PathBuf,
    pub role: String,
}

impl SessionContext {
    pub fn parse(value: Option<&Value>, meta: Option<&Value>) -> Result<Self, SessionError> {
        let context: Self = serde_json::from_value(value.cloned().unwrap_or(Value::Null))
            .map_err(|_| fail("context_missing", "Provide harness-injected unelos_context"))?;
        if context.session_id.is_empty()
            || context.session_id.chars().count() > 256
            || !context.project.is_absolute()
            || !["controller", "worker"].contains(&context.role.as_str())
        {
            return Err(fail("context_missing", "Malformed unelos_context"));
        }
        if !["opencode", "claude"].contains(&context.harness.as_str()) {
            return Err(fail("context_invalid", "Unsupported harness"));
        }
        if context.harness == "opencode" {
            if let Some(id) = meta.and_then(|m| m.get("ai.opencode/sessionID")) {
                if id.as_str() != Some(&context.session_id) {
                    return Err(fail(
                        "context_invalid",
                        "OpenCode session identity mismatch",
                    ));
                }
            }
        }
        if context.role == "worker" {
            return Err(fail(
                "child_controller",
                "Workers cannot operate the Unelos controller",
            ));
        }
        Ok(context)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PendingStart {
    pub entry_id: String,
    pub at: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionBinding {
    pub version: u32,
    pub harness: String,
    pub session_id: String,
    pub project: Utf8PathBuf,
    pub enabled: bool,
    pub mode: String,
    pub entry_id: String,
    pub run_id: Option<String>,
    pub pending_start: Option<PendingStart>,
}

#[derive(Clone)]
pub struct SessionService {
    config: Utf8PathBuf,
}

pub(crate) struct LockedBinding {
    path: Utf8PathBuf,
    _lock: File,
}

impl LockedBinding {
    fn current(&self, entry_id: &str) -> Result<SessionBinding, SessionError> {
        let binding = self.read()?.ok_or_else(|| {
            fail(
                "entry_superseded",
                "Entry superseded; explicitly select a run again",
            )
        })?;
        if binding.entry_id != entry_id {
            return Err(fail(
                "entry_superseded",
                "Entry superseded; explicitly select a run again",
            ));
        }
        Ok(binding)
    }
    pub(crate) fn read(&self) -> Result<Option<SessionBinding>, SessionError> {
        let bytes = match fs::read(&self.path) {
            Ok(bytes) => bytes,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(io_error(e)),
        };
        let value: Value = serde_json::from_slice(&bytes).map_err(io_error)?;
        if value["version"] != 1 {
            return Err(fail(
                "binding_version_unsupported",
                "Unsupported session binding version",
            ));
        }
        serde_json::from_value(value).map(Some).map_err(io_error)
    }
    pub(crate) fn write(&self, binding: &SessionBinding) -> Result<(), SessionError> {
        crate::global_config::atomic_write(
            &self.path,
            &serde_json::to_vec(binding).map_err(io_error)?,
        )
        .map_err(io_error)
    }
}

impl SessionService {
    pub fn activate_request(
        &self,
        context: &SessionContext,
        request: &str,
    ) -> Result<Value, SessionError> {
        let words = request.split_whitespace().collect::<Vec<_>>();
        let mode = match words.as_slice() {
            ["off"] => "off",
            ["resume"] => "resume",
            _ => "new",
        };
        let mut value = self.activate(context, mode)?;
        if mode != "off" {
            value["requestedAction"] = json!(request);
        }
        Ok(value)
    }
    pub fn new(config: Utf8PathBuf) -> Self {
        Self { config }
    }
    pub fn discover() -> Result<Self, SessionError> {
        Ok(Self::new(
            crate::global_config::config_dir().map_err(io_error)?,
        ))
    }
    pub(crate) fn lock(&self, context: &SessionContext) -> Result<LockedBinding, SessionError> {
        // Validate before using the harness as a path component.
        SessionContext::parse(
            Some(&serde_json::to_value(context).map_err(io_error)?),
            None,
        )?;
        let digest = format!("{:x}", Sha256::digest(context.session_id.as_bytes()));
        let directory = self.config.join("sessions").join(&context.harness);
        fs::create_dir_all(&directory).map_err(io_error)?;
        let lock = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(directory.join(format!("{digest}.lock")))
            .map_err(io_error)?;
        lock.lock().map_err(io_error)?;
        Ok(LockedBinding {
            path: directory.join(format!("{digest}.json")),
            _lock: lock,
        })
    }
    pub fn binding(
        &self,
        context: &SessionContext,
    ) -> Result<Option<SessionBinding>, SessionError> {
        SessionContext::parse(
            Some(&serde_json::to_value(context).map_err(io_error)?),
            None,
        )?;
        let digest = format!("{:x}", Sha256::digest(context.session_id.as_bytes()));
        let path = self
            .config
            .join("sessions")
            .join(&context.harness)
            .join(format!("{digest}.json"));
        let bytes = match fs::read(path) {
            Ok(bytes) => bytes,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(io_error(e)),
        };
        let value: Value = serde_json::from_slice(&bytes).map_err(io_error)?;
        if value["version"] != 1 {
            return Err(fail(
                "binding_version_unsupported",
                "Unsupported session binding version",
            ));
        }
        serde_json::from_value(value).map(Some).map_err(io_error)
    }
    pub fn activate(&self, context: &SessionContext, mode: &str) -> Result<Value, SessionError> {
        let lock = self.lock(context)?;
        if mode == "off" {
            match fs::remove_file(&lock.path) {
                Ok(()) => (),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
                Err(e) => return Err(io_error(e)),
            }
            return Ok(json!({"disabled":true,"message":"Unelos disabled for this session."}));
        }
        if !["new", "resume"].contains(&mode) {
            return Err(fail(
                "invalid_entry",
                "Activation mode must be new, resume, or off",
            ));
        }
        let binding = SessionBinding {
            version: 1,
            harness: context.harness.clone(),
            session_id: context.session_id.clone(),
            project: canonical(&context.project)?,
            enabled: true,
            mode: mode.into(),
            entry_id: uuid::Uuid::new_v4().to_string(),
            run_id: None,
            pending_start: None,
        };
        lock.write(&binding)?;
        drop(lock);
        let mut value = if mode == "new" {
            serde_json::to_value(crate::execution_new(&binding.project)?).map_err(io_error)?
        } else {
            serde_json::to_value(crate::execution_runs(&binding.project)?).map_err(io_error)?
        };
        let lock = self.lock(context)?;
        lock.current(&binding.entry_id)?;
        value["requestedAction"] = json!(if mode == "new" { "" } else { mode });
        Ok(crate::controller_mcp::native_package(
            value,
            &context.harness,
        ))
    }
}

pub(crate) fn canonical(path: &Utf8Path) -> Result<Utf8PathBuf, SessionError> {
    let path = fs::canonicalize(path)
        .map_err(|_| fail("project_changed", "Project unavailable; reactivate /unelos"))?;
    Utf8PathBuf::from_path_buf(path)
        .map_err(|_| fail("project_changed", "Project path is not UTF-8"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::controller_mcp::error_envelope;
    use std::sync::{Arc, Barrier};

    fn fixture() -> (tempfile::TempDir, SessionService, SessionContext) {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8Path::from_path(temp.path()).unwrap().to_owned();
        crate::install_workflow(
            &root,
            Some(
                &Utf8Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("tests/fixtures/simple-plan-execute-review"),
            ),
            "simple-plan-execute-review",
        )
        .unwrap();
        let service = SessionService::new(root.join("config"));
        let context = SessionContext {
            harness: "opencode".into(),
            session_id: "session/unsafe".into(),
            project: root,
            role: "controller".into(),
        };
        (temp, service, context)
    }
    fn input(context: &SessionContext) -> Value {
        json!({"unelos_context":context,"workflow":"simple-plan-execute-review"})
    }
    fn code(result: Result<Value, SessionError>, expected: &str) {
        let error = result.unwrap_err();
        assert_eq!(error.code(), expected, "{error}");
        assert_eq!(error_envelope(&error)["error"]["code"], expected);
        assert_eq!(
            error_envelope(&error)["error"]["message"],
            error.to_string()
        );
    }

    #[test]
    fn activation_new_resume_off_and_request() {
        let (_temp, service, context) = fixture();
        let value = service.activate(&context, "new").unwrap();
        assert!(value["harnessInstructions"]
            .as_str()
            .unwrap()
            .contains("unelos_next"));
        assert_eq!(service.binding(&context).unwrap().unwrap().mode, "new");
        let started = service.dispatch("start", &input(&context), None).unwrap();
        service.activate(&context, "resume").unwrap();
        let mut args = input(&context);
        args["runId"] = started["runId"].clone();
        let resumed = service.dispatch("resume", &args, None).unwrap();
        assert_eq!(resumed["runId"], started["runId"]);
        assert_eq!(
            service.dispatch("next", &args, None).unwrap()["runId"],
            started["runId"]
        );
        for (request, mode) in [
            ("", "new"),
            (" resume ", "resume"),
            ("resume run", "new"),
            ("do work", "new"),
        ] {
            let value = service.activate_request(&context, request).unwrap();
            assert_eq!(value["requestedAction"], request);
            assert_eq!(service.binding(&context).unwrap().unwrap().mode, mode);
        }
        assert_eq!(
            service.activate_request(&context, " off ").unwrap()["disabled"],
            true
        );
        assert!(service.binding(&context).unwrap().is_none());
        assert!(!crate::list_runs(&context.project).unwrap().is_empty());
    }

    #[test]
    fn context_and_binding_guards_preserve_error_codes() {
        let (_temp, service, context) = fixture();
        code(
            service.dispatch("next", &json!({}), None),
            "context_missing",
        );
        let mut args = input(&context);
        args["unelos_context"]["sessionId"] = json!("");
        code(service.dispatch("next", &args, None), "context_missing");
        args = input(&context);
        args["unelos_context"]["harness"] = json!("other");
        code(service.dispatch("next", &args, None), "context_invalid");
        args = input(&context);
        args["unelos_context"]["role"] = json!("worker");
        code(service.dispatch("next", &args, None), "child_controller");
        args = input(&context);
        code(
            service.dispatch(
                "next",
                &args,
                Some(&json!({"ai.opencode/sessionID":"other"})),
            ),
            "context_invalid",
        );
        code(service.dispatch("next", &args, None), "controller_inactive");
        service.activate(&context, "new").unwrap();
        code(service.dispatch("next", &args, None), "run_unbound");
        let elsewhere = tempfile::tempdir().unwrap();
        args["unelos_context"]["project"] = json!(elsewhere.path());
        code(service.dispatch("next", &args, None), "project_changed");
        args = input(&context);
        code(service.dispatch("resume", &args, None), "invalid_entry");
        service.dispatch("start", &args, None).unwrap();
        code(service.dispatch("start", &args, None), "invalid_entry");
        args["expected"] = json!({"runId":"wrong"});
        code(service.dispatch("complete", &args, None), "stale_execution");
        args.as_object_mut().unwrap().remove("expected");
        args["runId"] = json!("wrong");
        args["name"] = json!("new name");
        code(service.dispatch("rename", &args, None), "stale_execution");
    }

    #[test]
    fn superseded_version_and_pending_start_recovery() {
        let (_temp, service, context) = fixture();
        service.activate(&context, "new").unwrap();
        let lock = service.lock(&context).unwrap();
        assert_eq!(
            lock.current("old-entry").unwrap_err().code(),
            "entry_superseded"
        );
        let mut binding = lock.read().unwrap().unwrap();
        binding.pending_start = Some(PendingStart {
            entry_id: binding.entry_id.clone(),
            at: 1,
        });
        lock.write(&binding).unwrap();
        drop(lock);
        for tool in [
            "next", "start", "resume", "complete", "approve", "reject", "status", "inspect",
            "rename",
        ] {
            code(
                service.dispatch(tool, &input(&context), None),
                "mutation_uncertain",
            );
        }
        service.activate(&context, "resume").unwrap();
        assert!(service
            .binding(&context)
            .unwrap()
            .unwrap()
            .pending_start
            .is_none());
        service.activate(&context, "new").unwrap();
        service.dispatch("start", &input(&context), None).unwrap();
        let lock = service.lock(&context).unwrap();
        let mut binding = lock.read().unwrap().unwrap();
        binding.version = 2;
        lock.write(&binding).unwrap();
        drop(lock);
        code(
            service.dispatch("next", &input(&context), None),
            "binding_version_unsupported",
        );
        service.activate(&context, "off").unwrap();
    }

    #[test]
    fn concurrent_starts_serialize_on_session_lock() {
        let (_temp, service, context) = fixture();
        service.activate(&context, "new").unwrap();
        let barrier = Arc::new(Barrier::new(3));
        let handles = (0..2)
            .map(|_| {
                let (service, context, barrier) =
                    (service.clone(), context.clone(), barrier.clone());
                std::thread::spawn(move || {
                    barrier.wait();
                    service.dispatch("start", &input(&context), None)
                })
            })
            .collect::<Vec<_>>();
        barrier.wait();
        let results = handles
            .into_iter()
            .map(|h| h.join().unwrap())
            .collect::<Vec<_>>();
        assert_eq!(results.iter().filter(|r| r.is_ok()).count(), 1);
        assert_eq!(
            results.into_iter().find_map(Result::err).unwrap().code(),
            "invalid_entry"
        );
        assert_eq!(crate::list_runs(&context.project).unwrap().len(), 1);
    }

    #[test]
    fn read_only_binding_does_not_create_store() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8Path::from_path(temp.path()).unwrap();
        let service = SessionService::new(root.join("not-created"));
        let context = SessionContext {
            harness: "claude".into(),
            session_id: "id".into(),
            project: root.into(),
            role: "controller".into(),
        };
        assert!(service.binding(&context).unwrap().is_none());
        assert!(!root.join("not-created").exists());
    }
}
