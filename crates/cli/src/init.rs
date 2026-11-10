use crate::opencode::Version;
use camino::Utf8Path;
use serde_json::{json, Value};
use unelos_core::InstallError;

pub fn initialize(root: &Utf8Path) -> Result<Value, InstallError> {
    let root = camino::Utf8PathBuf::from_path_buf(std::fs::canonicalize(root).map_err(|e| {
        InstallError {
            code: "invalid_project",
            message: e.to_string(),
        }
    })?)
    .map_err(|_| InstallError {
        code: "invalid_path",
        message: "Project path must be UTF-8".into(),
    })?;
    let root = root.as_path();
    unelos_core::check_project_init(root)?;
    let migrated = unelos_core::harness::migrate_project(root)?;
    unelos_core::initialize_project(root)?;
    Ok(json!({"project": root, "migrated": migrated,
        "message": "Initialized Unelos. Run unelos harness install opencode for the global integration."}))
}

pub fn install_version(
    root: &Utf8Path,
    requested: Option<Version>,
) -> Result<Option<Version>, InstallError> {
    let path = root.join(".unelos/integration.json");
    unelos_core::check_install_path(root, Utf8Path::new(".unelos/integration.json"))?;
    match std::fs::read(path) {
        Ok(bytes) => {
            #[derive(serde::Deserialize)]
            struct Legacy {
                opencode: Version,
            }
            let legacy: Legacy = serde_json::from_slice(&bytes).map_err(|e| InstallError {
                code: "invalid_integration",
                message: e.to_string(),
            })?;
            Ok(Some(requested.unwrap_or(legacy.opencode)))
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(requested),
        Err(e) => Err(InstallError {
            code: "install_io_failed",
            message: e.to_string(),
        }),
    }
}
