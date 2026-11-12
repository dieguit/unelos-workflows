use crate::skill_document::{atomic_write, collect, reference_field_edit};
use camino::{Utf8Path, Utf8PathBuf};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fs};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct McpDocument {
    pub id: String,
    #[serde(flatten)]
    pub server: McpServer,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum McpServer {
    Local {
        command: Vec<String>,
        #[serde(default)]
        environment: BTreeMap<String, String>,
    },
    Remote {
        url: String,
        #[serde(default)]
        headers: BTreeMap<String, String>,
    },
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct McpSummary {
    pub name: String,
    pub kind: Option<String>,
    pub target: Option<String>,
    pub problem: Option<String>,
}

fn path(root: &Utf8Path, id: &str) -> Result<Utf8PathBuf, String> {
    crate::skill_document::valid_id(id).map_err(|_| format!("Invalid MCP ID {id:?}"))?;
    crate::project::bounded_path(
        root,
        &Utf8PathBuf::from(format!("mcps/{id}.yaml")),
        root,
        "MCP",
    )
    .map_err(|e| e.to_string())
}

fn validate(server: &McpServer) -> Result<(), String> {
    match server {
        McpServer::Local { command, .. } if command.is_empty() || command[0].trim().is_empty() => {
            Err("MCP local command must not be empty".into())
        }
        McpServer::Remote { url, .. } => {
            let parsed =
                url::Url::parse(url).map_err(|_| "MCP remote URL must be an http(s) URL")?;
            if !matches!(parsed.scheme(), "http" | "https") || parsed.host_str().is_none() {
                return Err("MCP remote URL must be an http(s) URL".into());
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

pub(crate) fn read_mcp(authoring: &Utf8Path, id: &str) -> Result<McpDocument, String> {
    let source = path(authoring, id)?;
    if !fs::metadata(&source).map_err(|e| e.to_string())?.is_file() {
        return Err("MCP definition must be an ordinary file".into());
    }
    let text = fs::read_to_string(source).map_err(|e| e.to_string())?;
    let server = serde_yaml_ng::from_str(&text).map_err(|e| e.to_string())?;
    validate(&server)?;
    Ok(McpDocument {
        id: id.into(),
        server,
    })
}

pub fn get_mcp(root: &Utf8Path, id: &str) -> Result<McpDocument, String> {
    let root = crate::project::absolute_root(root)
        .map_err(|e| e.to_string())?
        .join(".unelos");
    read_mcp(&root, id)
}

pub fn mcp_summaries(root: &Utf8Path) -> Result<Vec<McpSummary>, String> {
    let root = crate::project::absolute_root(root)
        .map_err(|e| e.to_string())?
        .join(".unelos");
    summaries_at(&root)
}

fn summaries_at(root: &Utf8Path) -> Result<Vec<McpSummary>, String> {
    let directory = crate::project::bounded_path(&root, Utf8Path::new("mcps"), &root, "MCP")
        .map_err(|e| e.to_string())?;
    if !directory.exists() {
        return Ok(Vec::new());
    }
    let mut summaries = Vec::new();
    for entry in fs::read_dir(directory).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let file = Utf8PathBuf::from_path_buf(entry.path()).map_err(|_| "Non-UTF-8 MCP path")?;
        if file.extension() != Some("yaml") {
            continue;
        }
        let name = file.file_stem().unwrap().to_owned();
        let (kind, target, problem) = match read_mcp(&root, &name) {
            Ok(McpDocument {
                server: McpServer::Local { command, .. },
                ..
            }) => (Some("local".into()), Some(command.join(" ")), None),
            Ok(McpDocument {
                server: McpServer::Remote { url, .. },
                ..
            }) => (Some("remote".into()), Some(url), None),
            Err(error) => (None, None, Some(error)),
        };
        summaries.push(McpSummary {
            name,
            kind,
            target,
            problem,
        });
    }
    summaries.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(summaries)
}

pub fn save_mcp(
    root: &Utf8Path,
    original_id: Option<&str>,
    mcp: McpDocument,
) -> Result<McpDocument, String> {
    let root = crate::project::absolute_root(root)
        .map_err(|e| e.to_string())?
        .join(".unelos");
    save_at(&root, original_id, mcp, true)
}

fn save_at(root: &Utf8Path, original_id: Option<&str>, mcp: McpDocument, rewrite: bool) -> Result<McpDocument, String> {
    validate(&mcp.server)?;
    let target = path(&root, &mcp.id)?;
    let source = original_id.map(|id| path(&root, id)).transpose()?;
    if source.as_ref() != Some(&target) && target.exists() {
        return Err(format!("MCP {:?} already exists", mcp.id));
    }
    if let Some(source) = &source {
        if !fs::metadata(source).map_err(|e| e.to_string())?.is_file() {
            return Err("MCP definition must be an ordinary file".into());
        }
    }
    let mut edits = Vec::new();
    if let Some(old) = original_id.filter(|id| rewrite && *id != mcp.id) {
        let mut files = Vec::new();
        collect(&root, "agents", &mut files)?;
        files.sort();
        for file in files {
            let original = fs::read_to_string(&file).map_err(|e| e.to_string())?;
            let updated = reference_field_edit(&original, old, &mcp.id, "mcps")?;
            if original != updated {
                edits.push((file, original, updated));
            }
        }
    }
    let yaml = serde_yaml_ng::to_string(&mcp.server).map_err(|e| e.to_string())?;
    fs::create_dir_all(target.parent().unwrap()).map_err(|e| e.to_string())?;
    let renamed = source.as_ref().is_some_and(|s| s != &target);
    if renamed {
        fs::rename(source.as_ref().unwrap(), &target).map_err(|e| e.to_string())?;
    }
    let mut written = 0;
    let result = (|| {
        for (file, _, updated) in &edits {
            atomic_write(file, updated)?;
            written += 1;
        }
        atomic_write(&target, &yaml)
    })();
    if let Err(error) = result {
        let mut rollback = Vec::new();
        for (file, original, _) in edits[..written].iter().rev() {
            if let Err(e) = atomic_write(file, original) {
                rollback.push(e);
            }
        }
        if renamed {
            if let Err(e) = fs::rename(&target, source.unwrap()) {
                rollback.push(e.to_string());
            }
        }
        return Err(format!("{error}; rollback errors: {rollback:?}"));
    }
    Ok(mcp)
}

pub fn get_global_mcp(id: &str) -> Result<McpDocument, String> {
    read_mcp(&crate::task_document::global_root()?, id)
}

pub fn global_mcp_summaries() -> Result<Vec<McpSummary>, String> {
    summaries_at(&crate::task_document::global_root()?)
}

pub fn save_global_mcp(original_id: Option<&str>, mcp: McpDocument) -> Result<McpDocument, String> {
    save_at(&crate::task_document::global_root()?, original_id, mcp, false)
}

fn remove_at(authoring: &Utf8Path, id: &str, project: Option<&Utf8Path>, reference: &str) -> Result<(), String> {
    let target = path(authoring, id)?;
    if let Some(root) = project {
        let mut files = Vec::new();
        collect(&root.join(".unelos"), "agents", &mut files)?;
        files.sort();
        for file in files {
            let text = fs::read_to_string(&file).map_err(|e| e.to_string())?;
            let agent: crate::AgentDefinition = serde_yaml_ng::from_str(&text).map_err(|e| format!("{file}: {e}"))?;
            if agent.mcps.iter().any(|id| id == reference) {
                return Err(format!("Cannot remove MCP {reference:?}: referenced by {file}"));
            }
        }
    }
    fs::remove_file(target).map_err(|e| e.to_string())
}

pub fn remove_mcp(root: &Utf8Path, id: &str) -> Result<(), String> {
    let root = crate::project::absolute_root(root).map_err(|e| e.to_string())?;
    remove_at(&root.join(".unelos"), id, Some(&root), id)
}

pub fn remove_global_mcp(root: Option<&Utf8Path>, id: &str) -> Result<(), String> {
    let root = root.map(crate::project::absolute_root).transpose().map_err(|e| e.to_string())?;
    remove_at(&crate::task_document::global_root()?, id, root.as_deref(), &format!("global/{id}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn removal_checks_agent_references() {
        let temp = assert_fs::TempDir::new().unwrap();
        let root = Utf8Path::from_path(temp.path()).unwrap();
        fs::create_dir_all(root.join(".unelos/agents/worker")).unwrap();
        let agent = root.join(".unelos/agents/worker/agent.yaml");
        fs::write(&agent, "instructions: Work\nmcps: [browser]\n").unwrap();
        save_mcp(root, None, McpDocument { id: "browser".into(), server: McpServer::Remote { url: "https://example.com".into(), headers: BTreeMap::new() } }).unwrap();
        let error = remove_mcp(root, "browser").unwrap_err();
        assert!(error.contains(agent.as_str()), "{error}");
        assert!(root.join(".unelos/mcps/browser.yaml").exists());
        fs::write(agent, "instructions: Work\n").unwrap();
        remove_mcp(root, "browser").unwrap();
        assert!(!root.join(".unelos/mcps/browser.yaml").exists());
    }
}
