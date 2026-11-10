use crate::InstallError;
use camino::{Utf8Path, Utf8PathBuf};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fs, io::Write};

pub(crate) fn error(code: &'static str, message: impl ToString) -> InstallError {
    InstallError {
        code,
        message: message.to_string(),
    }
}
pub(crate) fn io_error(e: impl ToString) -> InstallError {
    error("harness_io_failed", e)
}

pub(crate) fn env_path(key: &str) -> Option<Utf8PathBuf> {
    std::env::var(key)
        .ok()
        .filter(|v| !v.is_empty())
        .map(Utf8PathBuf::from)
}
pub(crate) fn home() -> Option<Utf8PathBuf> {
    env_path("HOME").or_else(|| env_path("USERPROFILE"))
}
pub fn config_dir() -> Result<Utf8PathBuf, InstallError> {
    env_path("UNELOS_CONFIG_DIR")
        .or_else(|| env_path("XDG_CONFIG_HOME").map(|p| p.join("unelos")))
        .or_else(|| home().map(|p| p.join(".config/unelos")))
        .ok_or_else(|| error("config_dir_missing", "Set UNELOS_CONFIG_DIR or HOME"))
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Installed {
    pub version: String,
    pub installed_at: u64,
    pub files: Vec<Utf8PathBuf>,
}
pub type HarnessState = BTreeMap<String, Installed>;

pub fn read_state() -> Result<HarnessState, InstallError> {
    match fs::read(config_dir()?.join("harnesses.json")) {
        Ok(bytes) => serde_json::from_slice(&bytes).map_err(|e| error("invalid_harness_state", e)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(BTreeMap::new()),
        Err(e) => Err(io_error(e)),
    }
}
pub fn write_state(state: &HarnessState) -> Result<(), InstallError> {
    atomic_write(
        &config_dir()?.join("harnesses.json"),
        &serde_json::to_vec_pretty(state).map_err(io_error)?,
    )
}

pub fn atomic_write(path: &Utf8Path, bytes: &[u8]) -> Result<(), InstallError> {
    if fs::read(path).ok().as_deref() == Some(bytes) {
        return Ok(());
    }
    let parent = path.parent().ok_or_else(|| error("invalid_path", path))?;
    fs::create_dir_all(parent).map_err(io_error)?;
    let mut temp = tempfile::NamedTempFile::new_in(parent).map_err(io_error)?;
    if let Ok(metadata) = fs::metadata(path) {
        temp.as_file()
            .set_permissions(metadata.permissions())
            .map_err(io_error)?;
    }
    temp.write_all(bytes).map_err(io_error)?;
    temp.as_file().sync_all().map_err(io_error)?;
    temp.persist(path).map_err(io_error)?;
    Ok(())
}
