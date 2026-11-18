use crate::{AgentExport, InstallError, ProjectSettings};
use camino::{Utf8Path, Utf8PathBuf};
use sha2::{Digest, Sha256};
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

impl HarnessBuild {
    fn append(&mut self, build: Self) {
        for id in build.harnesses {
            if !self.harnesses.contains(&id) {
                self.harnesses.push(id);
            }
        }
        self.written.extend(build.written);
        self.removed.extend(build.removed);
        self.skipped.extend(build.skipped);
    }
}

type Workflows = BTreeMap<String, Vec<AgentExport>>;

fn global_agents(result: &mut HarnessBuild) -> Result<Vec<AgentExport>, InstallError> {
    let mut agents = Vec::new();
    for summary in crate::global_agent_summaries().map_err(error)? {
        let id = format!("global/{}", summary.name);
        let load = || {
            let agent = crate::get_global_agent(&summary.name)?;
            let mut mcp_servers = BTreeMap::new();
            for reference in &agent.mcps {
                let name = reference.strip_prefix("global/").ok_or_else(|| {
                    format!("Global agent requires a global MCP reference: {reference:?}")
                })?;
                if name == "unelos" {
                    return Err("MCP ID 'unelos' is reserved".into());
                }
                let document = crate::get_global_mcp(name)
                    .map_err(|e| format!("Cannot resolve MCP {reference:?}: {e}"))?;
                mcp_servers.insert(name.to_owned(), document.server);
            }
            Ok(AgentExport {
                id: id.clone(),
                instructions: agent.instructions,
                model: agent.model,
                effort: agent.effort,
                tools: agent.tools,
                mcps: agent.mcps,
                mcp_servers,
                main: true,
                subagent: true,
            })
        };
        match summary.problem.map_or_else(load, Err) {
            Ok(agent) => agents.push(agent),
            Err(problem) => result.skipped.push(format!("{id}: {problem}")),
        }
    }
    Ok(agents)
}

fn global_workflows(result: &mut HarnessBuild) -> Result<Workflows, InstallError> {
    let mut workflows = BTreeMap::new();
    let root = crate::task_document::global_root().map_err(error)?;
    for summary in crate::global_workflow_summaries().map_err(error)? {
        let name = format!("global/{}", summary.name);
        match summary.problem.map_or_else(
            || crate::export_workflow_agents(&root, &name).map_err(|e| e.message),
            Err,
        ) {
            Ok(agents) => {
                workflows.insert(name, agents);
            }
            Err(problem) => result.skipped.push(format!("{}: {problem}", summary.name)),
        }
    }
    Ok(workflows)
}

pub fn build_global_harness_agents() -> Result<HarnessBuild, InstallError> {
    let installed = crate::harness::list_harnesses();
    let mut result = HarnessBuild::default();
    if installed.iter().any(|h| h.installed) {
        let agents = global_agents(&mut result)?;
        let prune = result.skipped.is_empty();
        for adapter in crate::harness::registry() {
            if installed
                .iter()
                .any(|h| h.installed && h.id == adapter.id())
            {
                refresh_globals(adapter.as_ref(), &agents, prune, &mut result);
            }
        }
    }
    Ok(result)
}

pub(crate) fn build_globals_for(
    adapter: &dyn crate::harness::HarnessAdapter,
) -> Result<HarnessBuild, InstallError> {
    let mut result = HarnessBuild::default();
    let agents = global_agents(&mut result)?;
    let prune = result.skipped.is_empty();
    refresh_globals(adapter, &agents, prune, &mut result);
    Ok(result)
}

fn refresh_globals(
    adapter: &dyn crate::harness::HarnessAdapter,
    agents: &[AgentExport],
    prune: bool,
    result: &mut HarnessBuild,
) {
    match adapter.build_global_agents(agents, prune) {
        Ok(build) => result.append(build),
        Err(e) => result
            .skipped
            .push(format!("{} globals: {}", adapter.id(), e.message)),
    }
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
    let mut global_result = HarnessBuild::default();
    let global_agents = global_agents(&mut global_result)?;
    let global_prune = global_result.skipped.is_empty();
    let mut legacy = global_workflows(&mut result)?;
    legacy.extend(workflows.clone());
    let legacy_agents = merge_agents(&legacy)?;
    let legacy_globals: Vec<_> = legacy_agents
        .into_iter()
        .filter(|a| a.id.starts_with("global/"))
        .collect();
    let global_names: std::collections::BTreeSet<_> = global_agents
        .iter()
        .map(|agent| crate::install::native_agent_id(&agent.id))
        .collect();
    for agent in workflows
        .values()
        .flatten()
        .filter(|a| !a.id.starts_with("global/"))
    {
        let name = crate::install::native_agent_id(&agent.id);
        if global_names.contains(&name) {
            return Err(error(format!("Agent native name collision: {name:?}")));
        }
    }
    result.append(global_result);
    for adapter in crate::harness::registry() {
        if !installed
            .iter()
            .any(|h| h.id == adapter.id() && h.installed)
        {
            continue;
        }
        refresh_globals(adapter.as_ref(), &global_agents, global_prune, &mut result);
        match adapter.build_project(root, &workflows, &legacy_globals, prune) {
            Ok(build) => result.append(build),
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
    let mut native_names = BTreeMap::new();
    for agent in workflows.values().flatten() {
        let native = crate::install::native_agent_id(&agent.id);
        if let Some(previous) = native_names.insert(native.clone(), agent.id.clone()) {
            if previous != agent.id {
                return Err(error(format!(
                    "Agent native name collision: {previous:?} and {:?} both render as {native:?}",
                    agent.id
                )));
            }
        }
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

pub(crate) fn write_global_agents(
    root: &Utf8Path,
    id: &str,
    mut rendered: Vec<(Utf8PathBuf, String)>,
    prune: bool,
) -> Result<HarnessBuild, InstallError> {
    for (path, _) in &mut rendered {
        *path = Utf8Path::new("agents").join(path.file_name().unwrap());
    }
    let mut result = write_agents(root, "agents", rendered, Vec::new(), prune)?;
    result.harnesses.push(id.into());
    Ok(result)
}

fn read_agent_file(path: &Utf8Path) -> std::io::Result<Vec<u8>> {
    if !fs::symlink_metadata(path)?.is_file() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("Not a regular file: {path}"),
        ));
    }
    fs::read(path)
}

pub(crate) fn write_agents(
    root: &Utf8Path,
    directory: &str,
    mut rendered: Vec<(Utf8PathBuf, String)>,
    legacy: Vec<(Utf8PathBuf, String)>,
    prune: bool,
) -> Result<HarnessBuild, InstallError> {
    let mut result = HarnessBuild::default();
    let directory = Utf8Path::new(directory);
    crate::check_install_path(root, directory)?;
    let manifest = directory.join(".unelos-owned-agents.json");
    crate::check_install_path(root, &manifest)?;
    let manifest_bytes = match read_agent_file(&root.join(&manifest)) {
        Ok(bytes) => Some(bytes),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => return Err(error(e)),
    };
    let mut owned: BTreeMap<String, String> = manifest_bytes
        .as_deref()
        .map(serde_json::from_slice)
        .transpose()
        .map_err(error)?
        .unwrap_or_default();
    let checksum = |bytes: &[u8]| format!("{:x}", Sha256::digest(bytes));
    for (path, _) in &rendered {
        crate::check_install_path(root, path)?;
    }
    let desired: std::collections::BTreeSet<_> =
        rendered.iter().map(|(path, _)| path.clone()).collect();
    let mut stale = Vec::new();
    let is_agent = |path: &Utf8Path| {
        path.parent() == Some(directory)
            && path.file_name().is_some_and(|name| {
                name.starts_with(if directory == "agents" {
                    "unelos-global-"
                } else {
                    "unelos-"
                }) && name.ends_with(".md")
            })
    };
    let mut writes = Vec::new();
    for (path, text) in rendered.drain(..) {
        if is_agent(&path) {
            let name = path.file_name().unwrap();
            match read_agent_file(&root.join(&path)) {
                Ok(bytes) if bytes == text.as_bytes() => {
                    // Equal content without an ownership record is still user-owned.
                    continue;
                }
                Ok(bytes) if owned.get(name) != Some(&checksum(&bytes)) => {
                    result
                        .skipped
                        .push(format!("Modified or unowned agent: {}", root.join(&path)));
                    continue;
                }
                Ok(_) => {}
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => return Err(error(e)),
            }
            owned.insert(name.to_owned(), checksum(text.as_bytes()));
        }
        writes.push((path, text));
    }
    if prune {
        for (name, hash) in owned.clone() {
            let path = directory.join(&name);
            if !is_agent(&path) || desired.contains(&path) {
                continue;
            }
            crate::check_install_path(root, &path)?;
            match read_agent_file(&root.join(&path)) {
                Ok(bytes) if checksum(&bytes) == hash => {
                    stale.push(path);
                    owned.remove(&name);
                }
                Ok(_) => {}
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                    owned.remove(&name);
                }
                Err(e) => return Err(error(e)),
            }
        }
    }
    // shortcut: legacy builds left no receipt, so migrate only reproducible renderings.
    for (path, text) in legacy {
        if !stale.contains(&path)
            && !desired.contains(&path)
            && !path
                .file_name()
                .is_some_and(|name| owned.contains_key(name))
        {
            match read_agent_file(&root.join(&path)) {
                Ok(old)
                    if std::str::from_utf8(&old).is_ok_and(|old| {
                        old.replace("\r\n", "\n") == text.replace("\r\n", "\n")
                    }) =>
                {
                    stale.push(path)
                }
                Ok(_) => {}
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) if e.kind() == std::io::ErrorKind::InvalidData => {}
                Err(e) => return Err(error(e)),
            }
        }
    }
    if manifest_bytes.is_some() || !owned.is_empty() {
        writes.push((
            manifest.clone(),
            serde_json::to_string_pretty(&owned).map_err(error)?,
        ));
    }
    stale.sort();
    let mut previous = BTreeMap::new();
    for path in writes.iter().map(|(path, _)| path).chain(stale.iter()) {
        let bytes = match read_agent_file(&root.join(path)) {
            Ok(bytes) => Some(bytes),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
            Err(e) => return Err(error(e)),
        };
        previous.insert(path.clone(), bytes);
    }
    let applied = (|| {
        for (path, text) in &writes {
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
    result.written.retain(|path| path != &root.join(&manifest));
    Ok(result)
}
