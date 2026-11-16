use crate::{AgentExport, InstallError, ProjectSettings};
use camino::{Utf8Path, Utf8PathBuf};
use std::{collections::BTreeMap, fs};

fn error(e: impl ToString) -> InstallError {
    InstallError {
        code: "harness_build_failed",
        message: e.to_string(),
    }
}

#[derive(Debug, Default, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HarnessBuild {
    pub harnesses: Vec<String>,
    pub written: Vec<Utf8PathBuf>,
    pub removed: Vec<Utf8PathBuf>,
    pub skipped: Vec<String>,
}

pub fn get_project_settings(root: &Utf8Path) -> Result<ProjectSettings, InstallError> {
    let relative = Utf8Path::new(".unelos/project.yaml");
    crate::check_install_path(root, relative)?;
    match fs::read_to_string(root.join(relative)) {
        Ok(text) => crate::parse_project_settings(relative.as_str(), &text).map_err(error),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(ProjectSettings::default()),
        Err(e) => Err(error(e)),
    }
}

pub fn save_project_settings(
    root: &Utf8Path,
    settings: &ProjectSettings,
) -> Result<(), InstallError> {
    let text = serde_yaml_ng::to_string(settings).map_err(error)?;
    crate::write_install_file(root, Utf8Path::new(".unelos/project.yaml"), text.as_bytes())
}

pub fn auto_build_harness(root: &Utf8Path) -> Result<Option<HarnessBuild>, InstallError> {
    if get_project_settings(root)?.auto_build_harness == Some(false) {
        return Ok(None);
    }
    build_harness_config(root).map(Some)
}

fn changed(
    root: &Utf8Path,
    relative: &Utf8Path,
    bytes: &[u8],
    result: &mut HarnessBuild,
) -> Result<(), InstallError> {
    match fs::read(root.join(relative)) {
        Ok(old) if old == bytes => return Ok(()),
        Ok(_) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(error(e)),
    }
    crate::write_install_file(root, relative, bytes)?;
    result.written.push(root.join(relative));
    Ok(())
}

pub fn build_harness_config(root: &Utf8Path) -> Result<HarnessBuild, InstallError> {
    let mut result = HarnessBuild::default();
    let installed = crate::harness::list_harnesses();
    if !installed.iter().any(|h| h.installed) {
        return Ok(result);
    }
    let workflows_path = Utf8Path::new(".unelos/workflows");
    crate::check_install_path(root, workflows_path)?;
    let mut workflows = BTreeMap::new();
    // git doesn't keep empty folders, so a fresh clone may lack workflows/.
    let entries = match fs::read_dir(root.join(workflows_path)) {
        Ok(entries) => Some(entries),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => return Err(error(e)),
    };
    for entry in entries.into_iter().flatten() {
        let path = Utf8PathBuf::from_path_buf(entry.map_err(error)?.path())
            .map_err(|_| error("Non UTF-8 workflow path"))?;
        if path.extension() != Some("yaml") {
            continue;
        }
        let id = path
            .file_stem()
            .ok_or_else(|| error("Missing workflow name"))?;
        // Broken workflows must not block building the valid ones.
        match crate::export_workflow_agents(root, id) {
            Ok(agents) => {
                workflows.insert(id.to_owned(), agents);
            }
            Err(e) => result.skipped.push(format!("{id}: {}", e.message)),
        }
    }
    let prune = result.skipped.is_empty();
    for adapter in crate::harness::registry() {
        if !installed
            .iter()
            .any(|h| h.id == adapter.id() && h.installed)
        {
            continue;
        }
        match adapter.build_project(root, &workflows, prune) {
            Ok(build) => {
                result.harnesses.extend(build.harnesses);
                result.written.extend(build.written);
                result.removed.extend(build.removed);
                result.skipped.extend(build.skipped);
            }
            Err(e) => result
                .skipped
                .push(format!("{}: {}", adapter.id(), e.message)),
        }
    }
    Ok(result)
}

pub(crate) fn merge_agents(
    workflows: &BTreeMap<String, Vec<AgentExport>>,
) -> Result<Vec<AgentExport>, InstallError> {
    let mut scopes = BTreeMap::new();
    let mut servers = BTreeMap::new();
    let mut agents: BTreeMap<String, AgentExport> = BTreeMap::new();
    for agent in workflows.values().flatten() {
        for reference in &agent.mcps {
            let id = reference.strip_prefix("global/").unwrap_or(reference);
            if id == "unelos" {
                return Err(error("MCP ID 'unelos' is reserved"));
            }
            if scopes
                .insert(id, reference)
                .is_some_and(|old| old != reference)
            {
                return Err(error(format!("Project and global MCP share ID {id:?}")));
            }
        }
        for (id, server) in &agent.mcp_servers {
            if id == "unelos" {
                return Err(error("MCP ID 'unelos' is reserved"));
            }
            if servers.insert(id, server).is_some_and(|old| old != server) {
                return Err(error(format!("Workflows disagree on MCP {id:?}")));
            }
        }
        if let Some(existing) = agents.get_mut(&agent.id) {
            existing.mcp_servers.extend(agent.mcp_servers.clone());
            existing.main |= agent.main;
            existing.subagent |= agent.subagent;
            existing.mcps.extend(agent.mcps.clone());
            existing.mcps.sort();
            existing.mcps.dedup();
        } else {
            agents.insert(agent.id.clone(), agent.clone());
        }
    }
    Ok(agents.into_values().collect())
}

pub(crate) fn write_agents(
    root: &Utf8Path,
    directory: &str,
    rendered: Vec<(Utf8PathBuf, String)>,
    prune: bool,
) -> Result<HarnessBuild, InstallError> {
    let mut result = HarnessBuild::default();
    let directory = Utf8Path::new(directory);
    crate::check_install_path(root, directory)?;
    for (path, _) in &rendered {
        crate::check_install_path(root, path)?;
    }
    let mut stale = Vec::new();
    if prune && root.join(directory).exists() {
        for entry in fs::read_dir(root.join(directory)).map_err(error)? {
            let entry = entry.map_err(error)?;
            let name = entry
                .file_name()
                .into_string()
                .map_err(|_| error("Non UTF-8 agent path"))?;
            if name.starts_with("unelos-") && name.ends_with(".md") {
                let relative = directory.join(name);
                crate::check_install_path(root, &relative)?;
                if !rendered.iter().any(|(path, _)| path == &relative) {
                    stale.push(relative);
                }
            }
        }
    }
    stale.sort();
    let mut previous = BTreeMap::new();
    for path in rendered.iter().map(|(path, _)| path).chain(stale.iter()) {
        let bytes = match fs::read(root.join(path)) {
            Ok(bytes) => Some(bytes),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
            Err(e) => return Err(error(e)),
        };
        previous.insert(path.clone(), bytes);
    }
    let applied = (|| {
        for (path, text) in &rendered {
            changed(root, path, text.as_bytes(), &mut result)?;
        }
        for path in &stale {
            fs::remove_file(root.join(path)).map_err(error)?;
            result.removed.push(root.join(path));
        }
        Ok::<(), InstallError>(())
    })();
    if let Err(failure) = applied {
        for path in result.written.iter().chain(&result.removed).rev() {
            let relative = path.strip_prefix(root).unwrap();
            let rollback = match &previous[relative] {
                Some(bytes) => crate::write_install_file(root, relative, bytes),
                None => fs::remove_file(path).map_err(error),
            };
            rollback.map_err(|e| error(format!("{failure}; rollback failed: {e}")))?;
        }
        return Err(failure);
    }
    Ok(result)
}
