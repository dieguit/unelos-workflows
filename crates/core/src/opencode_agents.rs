use crate::{write_install_file, AgentExport, InstallError};
use camino::{Utf8Path, Utf8PathBuf};
use serde_json::{json, Value};

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Version {
    V1,
    V2,
}

pub fn check_agents(root: &Utf8Path, agents: &[AgentExport]) -> Result<(), InstallError> {
    for agent in agents {
        if let Some(model) = &agent.model {
            crate::harness::opencode::resolve_agent_model(model, agent.effort.as_deref())?;
        }
        crate::check_install_path(
            root,
            &Utf8PathBuf::from(format!(".opencode/agents/unelos-{}.md", agent.id)),
        )?;
    }
    Ok(())
}

pub fn render_agents(
    version: Version,
    agents: &[AgentExport],
) -> Result<Vec<(Utf8PathBuf, String)>, InstallError> {
    let mut rendered = Vec::new();
    for agent in agents {
        let mode = match (agent.main, agent.subagent) {
            (true, true) => "all",
            (true, false) => "primary",
            _ => "subagent",
        };
        let mut front = json!({"description":format!("Unelos {} role",agent.id),"mode":mode});
        for id in &agent.mcps {
            let id = id.strip_prefix("global/").unwrap_or(id);
            if front.get("tools").is_none() {
                front["tools"] = json!({});
            }
            front["tools"][format!("{id}_*")] = true.into();
        }
        if let Some(model) = agent
            .model
            .as_ref()
            .map(|model| {
                crate::harness::opencode::resolve_agent_model(model, agent.effort.as_deref())
            })
            .transpose()?
        {
            match version {
                Version::V2 => front["model"] = model.clone().into(),
                Version::V1 => {
                    let (model, variant) = model
                        .split_once('#')
                        .map_or((model.as_str(), None), |(m, v)| (m, Some(v)));
                    front["model"] = model.into();
                    if let Some(variant) = variant {
                        front["variant"] = variant.into();
                    }
                }
            }
        }
        let requirements = if agent.tools.is_empty() {
            String::new()
        } else {
            format!(
                "\n\nRequired tools (availability, not permission grants): {}",
                agent.tools.join(", ")
            )
        };
        let yaml = serde_yaml_ng::to_string::<Value>(&front).map_err(|error| InstallError {
            code: "agent_render_failed",
            message: error.to_string(),
        })?;
        let text = format!("---\n{yaml}---\n{}{}\n", agent.instructions, requirements);
        rendered.push((
            Utf8PathBuf::from(format!(".opencode/agents/unelos-{}.md", agent.id)),
            text,
        ));
    }
    Ok(rendered)
}

pub fn write_agents(
    root: &Utf8Path,
    version: Version,
    agents: &[AgentExport],
) -> Result<Vec<Utf8PathBuf>, InstallError> {
    let mut paths = Vec::new();
    for (relative, text) in render_agents(version, agents)? {
        write_install_file(root, &relative, text.as_bytes())?;
        paths.push(root.join(relative));
    }
    Ok(paths)
}
