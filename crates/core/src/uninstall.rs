use crate::{check_install_path, InstallError};
use camino::{Utf8Path, Utf8PathBuf};
use serde::Serialize;
use std::{fs, io};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectUninstall {
    pub removed: Vec<Utf8PathBuf>,
}

fn failure(path: &Utf8Path, message: impl std::fmt::Display) -> InstallError {
    InstallError {
        code: "uninstall_failed",
        message: format!("{path}: {message}"),
    }
}

fn target(root: &Utf8Path, relative: &str, directory: bool) -> Result<bool, InstallError> {
    let relative = Utf8Path::new(relative);
    check_install_path(root, relative)?;
    let path = root.join(relative);
    match fs::symlink_metadata(&path) {
        Ok(meta)
            if if directory {
                meta.is_dir()
            } else {
                meta.is_file()
            } =>
        {
            Ok(true)
        }
        Ok(_) => Err(failure(&path, "Wrong uninstall target type")),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(failure(&path, e)),
    }
}

pub fn uninstall_project(root: &Utf8Path) -> Result<ProjectUninstall, InstallError> {
    uninstall_with(root, |path| fs::remove_file(path))
}

fn uninstall_with(
    root: &Utf8Path,
    mut remove_agent: impl FnMut(&Utf8Path) -> io::Result<()>,
) -> Result<ProjectUninstall, InstallError> {
    let canonical = fs::canonicalize(root).map_err(|e| failure(root, e))?;
    let root = Utf8Path::from_path(&canonical)
        .ok_or_else(|| failure(root, "Project path is not UTF-8"))?;
    if !root.is_dir() {
        return Err(failure(root, "Project must be an existing directory"));
    }
    let data = target(root, ".unelos", true)?;
    let mut agents = Vec::new();
    for harness in [".opencode", ".claude"] {
        target(root, harness, true)?;
        let directory = format!("{harness}/agents");
        if !target(root, &directory, true)? {
            continue;
        }
        let path = root.join(&directory);
        for entry in fs::read_dir(&path).map_err(|e| failure(&path, e))? {
            let entry = entry.map_err(|e| failure(&path, e))?;
            let name = entry.file_name();
            let Some(name) = name.to_str() else { continue };
            if name.starts_with("unelos-") && name.ends_with(".md") {
                let relative = format!("{directory}/{name}");
                target(root, &relative, false)?;
                agents.push(root.join(relative));
            }
        }
    }
    let config = crate::mcp_install::plan_project_uninstall(root)?;
    agents.sort();
    let mut removed = Vec::new();
    if let Some((path, text)) = config {
        crate::write_install_file(root, path.strip_prefix(root).unwrap(), text.as_bytes())?;
    }
    for path in agents {
        remove_agent(&path).map_err(|e| failure(&path, e))?;
        removed.push(path);
    }
    if data {
        let path = root.join(".unelos");
        fs::remove_dir_all(&path).map_err(|e| failure(&path, e))?;
        removed.push(path);
    }
    Ok(ProjectUninstall { removed })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn partial_failure_retains_state_and_retry_finishes() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8Path::from_path(temp.path()).unwrap();
        fs::create_dir_all(root.join(".unelos")).unwrap();
        fs::create_dir_all(root.join(".claude/agents")).unwrap();
        for name in ["unelos-a.md", "unelos-b.md"] {
            fs::write(root.join(".claude/agents").join(name), "").unwrap();
        }
        let mut calls = 0;
        let result = uninstall_with(root, |path| {
            calls += 1;
            if calls == 2 {
                Err(io::Error::other("injected removal failure"))
            } else {
                fs::remove_file(path)
            }
        });
        assert!(result
            .unwrap_err()
            .message
            .contains("injected removal failure"));
        assert!(!root.join(".claude/agents/unelos-a.md").exists());
        assert!(root.join(".claude/agents/unelos-b.md").exists());
        assert!(root.join(".unelos").exists());
        assert_eq!(uninstall_project(root).unwrap().removed.len(), 2);
    }
}
