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
    if !crate::harness::list_harnesses()
        .iter()
        .any(|h| h.id == "opencode" && h.installed)
    {
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
    let mut agents: BTreeMap<String, AgentExport> = BTreeMap::new();
    for agent in workflows.values().flatten() {
        if let Some(existing) = agents.get_mut(&agent.id) {
            existing.main |= agent.main;
            existing.subagent |= agent.subagent;
            existing.mcps.extend(agent.mcps.clone());
            existing.mcps.sort();
            existing.mcps.dedup();
        } else {
            agents.insert(agent.id.clone(), agent.clone());
        }
    }
    let agents: Vec<_> = agents.into_values().collect();
    crate::opencode_agents::check_agents(root, &agents)?;
    let rendered =
        crate::opencode_agents::render_agents(crate::opencode_agents::Version::V2, &agents)?;
    crate::mcp_install::plan_replace(root, &workflows)?;
    let directory = Utf8Path::new(".opencode/agents");
    crate::check_install_path(root, directory)?;
    let mut stale = Vec::new();
    if root.join(directory).exists() {
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
    for (path, text) in rendered {
        changed(root, &path, text.as_bytes(), &mut result)?;
    }
    result
        .written
        .extend(crate::replace_opencode_mcps(root, &workflows)?);
    stale.sort();
    for path in stale {
        fs::remove_file(root.join(&path)).map_err(error)?;
        result.removed.push(root.join(path));
    }
    result.harnesses.push("opencode".into());
    Ok(result)
}
