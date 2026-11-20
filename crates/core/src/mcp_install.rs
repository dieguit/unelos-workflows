use crate::{AgentExport, InstallError};
use camino::{Utf8Path, Utf8PathBuf};
use serde_json::{json, Value};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
};

const STATE: &str = ".unelos/opencode-mcps.json";
const GLOBAL_STATE: &str = "opencode-mcps.json";
type Workflows = BTreeMap<String, BTreeMap<String, crate::McpServer>>;
type McpKeys = BTreeMap<String, BTreeMap<String, Value>>;

#[derive(serde::Serialize, serde::Deserialize)]
struct GlobalOwnership {
    config: Utf8PathBuf,
    keys: McpKeys,
    released: BTreeSet<(String, String)>,
}

pub(crate) fn sync_global(
    path: &Utf8Path,
    original: &str,
    agents: &mut [AgentExport],
    prune: bool,
    uninstall: bool,
) -> Result<crate::HarnessBuild, InstallError> {
    let mut result = crate::HarnessBuild::default();
    let root = crate::global_config::config_dir()?;
    let state = root.join(GLOBAL_STATE);
    let relative = path.file_name().unwrap();
    check_uninstall_file(path.parent().unwrap(), relative)?;
    check_uninstall_file(&root, GLOBAL_STATE)?;
    let receipt = match fs::read(&state) {
        Ok(bytes) => Some(bytes),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => return Err(error("install_io_failed", e)),
    };
    let desired = projection(&BTreeMap::from([(
        "global".into(),
        agents.iter().flat_map(|a| a.mcp_servers.clone()).collect(),
    )]))?;
    let current = crate::harness::jsonc::config_value(original)?;
    if receipt.is_none() && desired["mcp"].as_object().unwrap().is_empty() {
        return Ok(result);
    }
    let (mut previous, mut released) = match &receipt {
        Some(bytes) => {
            let owned: GlobalOwnership =
                serde_json::from_slice(bytes).map_err(|e| error("invalid_mcp_state", e))?;
            if owned.config != path {
                return Err(error(
                    "invalid_mcp_state",
                    format!("OpenCode MCP ownership belongs to {}", owned.config),
                ));
            }
            (owned.keys, owned.released)
        }
        None => (BTreeMap::new(), BTreeSet::new()),
    };
    for section in ["mcp", "tools"] {
        previous
            .entry(section.into())
            .or_default()
            .retain(|key, value| {
                if current[section].get(key) == Some(value) {
                    return true;
                }
                result.skipped.push(format!(
                    "Managed OpenCode {section}.{key} was modified or removed; released ownership"
                ));
                released.insert((section.to_owned(), key.clone()));
                false
            });
    }
    let mut next = previous.clone();
    if prune {
        released.retain(|(section, key)| desired[section].get(key).is_some());
        next.get_mut("mcp")
            .unwrap()
            .retain(|id, _| desired["mcp"].get(id).is_some());
        next.get_mut("tools")
            .unwrap()
            .retain(|key, _| desired["tools"].get(key).is_some());
    }
    let mut blocked = BTreeSet::new();
    for (id, value) in desired["mcp"].as_object().unwrap() {
        let tool = format!("{id}_*");
        let keys = [
            ("mcp", id.as_str(), value),
            ("tools", tool.as_str(), &Value::Bool(false)),
        ];
        let mut conflict = current["mcp"]["servers"].get(id).is_some()
            || matches!(id.as_str(), "servers" | "timeout");
        if conflict {
            result.skipped.push(format!(
                "OpenCode MCP {id:?} conflicts with native mcp configuration"
            ));
        }
        for (section, key, desired) in keys {
            if released.contains(&(section.to_owned(), key.to_owned())) {
                conflict = true;
            } else if !previous[section].contains_key(key)
                && current[section]
                    .get(key)
                    .is_some_and(|existing| existing != desired)
            {
                result.skipped.push(format!(
                    "OpenCode {section}.{key} already exists with a different value"
                ));
                conflict = true;
            }
        }
        if conflict {
            blocked.insert(id.clone());
            continue;
        }
        for (section, key, value) in keys {
            if previous[section].contains_key(key) || current[section].get(key).is_none() {
                next.get_mut(section)
                    .unwrap()
                    .insert(key.into(), value.clone());
            }
        }
    }
    let edited = crate::harness::jsonc::mcp_text(original, &json!(previous), &json!(next))?;
    let updated = (!uninstall && (!next.values().all(BTreeMap::is_empty) || !released.is_empty()))
        .then(|| {
            serde_json::to_vec_pretty(&GlobalOwnership {
                config: path.to_owned(),
                keys: next,
                released,
            })
        })
        .transpose()
        .map_err(|e| error("invalid_mcp_state", e))?;
    let store = |bytes: Option<&[u8]>| match bytes {
        Some(bytes) => crate::global_config::atomic_write(&state, bytes),
        None => fs::remove_file(&state).map_err(|e| error("install_io_failed", e)),
    };
    if updated != receipt {
        store(updated.as_deref())?;
    }
    if edited != original {
        if let Err(failure) = crate::global_config::atomic_write(path, edited.as_bytes()) {
            if updated != receipt {
                store(receipt.as_deref())
                    .map_err(|e| error("install_io_failed", format!("{failure}; rollback: {e}")))?;
            }
            return Err(failure);
        }
        result.written.push(path.to_owned());
    }
    for agent in agents {
        agent
            .mcps
            .retain(|id| !blocked.contains(id.strip_prefix("global/").unwrap_or(id)));
    }
    Ok(result)
}

fn error(code: &'static str, message: impl ToString) -> InstallError {
    InstallError {
        code,
        message: message.to_string(),
    }
}

fn projection(workflows: &Workflows) -> Result<Value, InstallError> {
    let mut result = json!({"mcp": {}, "tools": {}});
    for servers in workflows.values() {
        for (id, server) in servers {
            if id == "unelos" {
                return Err(error("reserved_mcp_id", "MCP ID 'unelos' is reserved"));
            }
            let mut value = serde_json::to_value(server).map_err(|e| error("invalid_mcp", e))?;
            value["enabled"] = true.into();
            if let Some(existing) = result["mcp"].get(id) {
                if existing != &value {
                    return Err(error(
                        "mcp_collision",
                        format!("Workflows disagree on MCP {id:?}"),
                    ));
                }
            }
            result["mcp"][id] = value;
            result["tools"][format!("{id}_*")] = false.into();
        }
    }
    Ok(result)
}

fn plan(
    root: &Utf8Path,
    workflow: &str,
    agents: &[AgentExport],
) -> Result<(Utf8PathBuf, String, Workflows), InstallError> {
    plan_state(root, None, Some((workflow, agents)), false)
}

pub(crate) fn plan_replace(
    root: &Utf8Path,
    workflows: &BTreeMap<String, Vec<AgentExport>>,
    prune: bool,
) -> Result<(Utf8PathBuf, String, Workflows), InstallError> {
    let mut scopes = BTreeMap::new();
    for reference in workflows.values().flatten().flat_map(|agent| &agent.mcps) {
        let id = reference.strip_prefix("global/").unwrap_or(reference);
        if let Some(previous) = scopes.insert(id, reference) {
            if previous != reference {
                return Err(error(
                    "mcp_collision",
                    format!("Project and global MCP share ID {id:?}"),
                ));
            }
        }
    }
    let state = workflows
        .iter()
        .map(|(id, agents)| {
            (
                id.clone(),
                agents.iter().flat_map(|a| a.mcp_servers.clone()).collect(),
            )
        })
        .collect();
    plan_state(root, Some(state), None, prune)
}

pub(crate) fn plan_project_uninstall(
    root: &Utf8Path,
) -> Result<Option<(Utf8PathBuf, String)>, InstallError> {
    check_uninstall_file(root, STATE)?;
    let state: Workflows = match fs::read_to_string(root.join(STATE)) {
        Ok(text) => serde_json::from_str(&text).map_err(|e| error("invalid_mcp_state", e))?,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(error("install_io_failed", format!("{STATE}: {e}"))),
    };
    let mut previous = projection(&state)?;
    if state.values().all(BTreeMap::is_empty) {
        return Ok(None);
    }
    for file in ["opencode.json", "opencode.jsonc"] {
        check_uninstall_file(root, file)?;
    }
    let json = root.join("opencode.json");
    let jsonc = root.join("opencode.jsonc");
    if json.exists() && jsonc.exists() {
        return Err(error(
            "ambiguous_opencode_config",
            "Both OpenCode configs exist",
        ));
    }
    let path = if jsonc.exists() { jsonc } else { json };
    let text = match fs::read_to_string(&path) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(error("install_io_failed", format!("{path}: {e}"))),
    };
    let empty = json!({"mcp": {}, "tools": {}});
    crate::harness::jsonc::mcp_text(&text, &empty, &empty)?;
    let document = jsonc_parser::cst::CstRootNode::parse(&text, &Default::default())
        .map_err(|e| error("invalid_opencode_config", e))?;
    let current = document.object_value().unwrap().to_serde_value().unwrap();
    for section in ["mcp", "tools"] {
        previous[section]
            .as_object_mut()
            .unwrap()
            .retain(|key, value| current[section].get(key) == Some(value));
    }
    let edited = crate::harness::jsonc::mcp_text(&text, &previous, &empty)?;
    Ok((edited != text).then_some((path, edited)))
}

pub(crate) fn check_uninstall_file(root: &Utf8Path, relative: &str) -> Result<(), InstallError> {
    crate::check_install_path(root, Utf8Path::new(relative))?;
    let path = root.join(relative);
    match fs::symlink_metadata(&path) {
        Ok(meta) if meta.is_file() => Ok(()),
        Ok(_) => Err(error(
            "invalid_path",
            format!("Expected ordinary file: {path}"),
        )),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(error("install_io_failed", format!("{path}: {e}"))),
    }
}

fn plan_state(
    root: &Utf8Path,
    replacement: Option<Workflows>,
    update: Option<(&str, &[AgentExport])>,
    prune: bool,
) -> Result<(Utf8PathBuf, String, Workflows), InstallError> {
    for file in [STATE, "opencode.json", "opencode.jsonc"] {
        crate::check_install_path(root, Utf8Path::new(file))?;
    }
    let mut state: Workflows = match fs::read_to_string(root.join(STATE)) {
        Ok(text) => serde_json::from_str(&text).map_err(|e| error("invalid_mcp_state", e))?,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => BTreeMap::new(),
        Err(e) => return Err(error("install_io_failed", e)),
    };
    let previous = projection(&state)?;
    if let Some(replacement) = replacement {
        if prune {
            state.clear();
        }
        state.extend(replacement);
    }
    if let Some((workflow, agents)) = update {
        state.insert(
            workflow.into(),
            agents.iter().flat_map(|a| a.mcp_servers.clone()).collect(),
        );
    }
    let desired = projection(&state)?;
    let json = root.join("opencode.json");
    let jsonc = root.join("opencode.jsonc");
    if json.exists() && jsonc.exists() {
        return Err(error(
            "ambiguous_opencode_config",
            "Both opencode.json and opencode.jsonc exist",
        ));
    }
    let path = if jsonc.exists() { jsonc } else { json };
    let text = match fs::read_to_string(&path) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => "{}\n".into(),
        Err(e) => return Err(error("install_io_failed", e)),
    };
    let edited = crate::harness::jsonc::mcp_text(&text, &previous, &desired)?;
    Ok((path, edited, state))
}

pub fn check_opencode_mcps(
    root: &Utf8Path,
    workflow: &str,
    agents: &[AgentExport],
) -> Result<(), InstallError> {
    plan(root, workflow, agents).map(|_| ())
}

pub fn write_opencode_mcps(
    root: &Utf8Path,
    workflow: &str,
    agents: &[AgentExport],
) -> Result<(), InstallError> {
    let (path, text, state) = plan(root, workflow, agents)?;
    if state.values().all(BTreeMap::is_empty) && !root.join(STATE).exists() {
        return Ok(());
    }
    let bytes = serde_json::to_vec_pretty(&state).map_err(|e| error("invalid_mcp_state", e))?;
    let previous_state = match fs::read(root.join(STATE)) {
        Ok(bytes) => Some(bytes),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => return Err(error("install_io_failed", e)),
    };
    crate::write_install_file(root, Utf8Path::new(STATE), &bytes)?;
    if let Err(failure) =
        crate::write_install_file(root, path.strip_prefix(root).unwrap(), text.as_bytes())
    {
        let rollback = match previous_state {
            Some(bytes) => crate::write_install_file(root, Utf8Path::new(STATE), &bytes),
            None => fs::remove_file(root.join(STATE)).map_err(|e| error("install_io_failed", e)),
        };
        rollback.map_err(|e| error("install_io_failed", format!("{failure}; rollback: {e}")))?;
        return Err(failure);
    }
    Ok(())
}

pub fn replace_opencode_mcps(
    root: &Utf8Path,
    workflows: &BTreeMap<String, Vec<AgentExport>>,
) -> Result<Vec<Utf8PathBuf>, InstallError> {
    let (path, text, state) = plan_replace(root, workflows, true)?;
    if state.values().all(BTreeMap::is_empty) && !root.join(STATE).exists() {
        return Ok(Vec::new());
    }
    let bytes = serde_json::to_vec_pretty(&state).map_err(|e| error("invalid_mcp_state", e))?;
    let mut written = Vec::new();
    for (relative, bytes) in [
        (Utf8Path::new(STATE), bytes.as_slice()),
        (path.strip_prefix(root).unwrap(), text.as_bytes()),
    ] {
        match fs::read(root.join(relative)) {
            Ok(old) if old == bytes => continue,
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(error("install_io_failed", e)),
        }
        crate::write_install_file(root, relative, bytes)?;
        written.push(root.join(relative));
    }
    Ok(written)
}

pub fn uninstall_opencode_mcps(root: &Utf8Path, workflow: &str) -> Result<(), InstallError> {
    write_opencode_mcps(root, workflow, &[])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retained_workflow_mcps_are_checked_for_collisions() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8Path::from_path(temp.path()).unwrap();
        fs::create_dir(root.join(".unelos")).unwrap();
        let server = |command: &str| crate::McpServer::Local {
            command: vec![command.into()],
            environment: Default::default(),
        };
        let previous: Workflows =
            [("broken".into(), [("shared".into(), server("old"))].into())].into();
        fs::write(root.join(STATE), serde_json::to_vec(&previous).unwrap()).unwrap();
        let replacement: Workflows =
            [("valid".into(), [("shared".into(), server("new"))].into())].into();
        let error = plan_state(root, Some(replacement.clone()), None, false).unwrap_err();
        assert_eq!(error.code, "mcp_collision");
        assert_eq!(
            plan_state(root, Some(replacement.clone()), None, true)
                .unwrap()
                .2,
            replacement
        );
    }
}
