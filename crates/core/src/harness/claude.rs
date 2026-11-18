use super::*;
use global_config::{atomic_write, read_state, write_state};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

pub struct Claude;
pub(super) const CLAUDE_EFFORTS: [&str; 5] = ["low", "medium", "high", "xhigh", "max"];
const PLUGIN: &str = "unelos@unelos-local";
const FILES: &[(&str, &[u8])] = &[
    (
        ".claude-plugin/marketplace.json",
        include_bytes!("../../../../integrations/claude/.claude-plugin/marketplace.json"),
    ),
    (
        "unelos/.claude-plugin/plugin.json",
        include_bytes!("../../../../integrations/claude/unelos/.claude-plugin/plugin.json"),
    ),
    (
        "unelos/.mcp.json",
        include_bytes!("../../../../integrations/claude/unelos/.mcp.json"),
    ),
    (
        "unelos/hooks/hooks.json",
        include_bytes!("../../../../integrations/claude/unelos/hooks/hooks.json"),
    ),
    (
        "unelos/skills/unelos/SKILL.md",
        include_bytes!("../../../../integrations/claude/unelos/skills/unelos/SKILL.md"),
    ),
    (
        "unelos/skills/unelos-resume/SKILL.md",
        include_bytes!("../../../../integrations/claude/unelos/skills/unelos-resume/SKILL.md"),
    ),
    (
        "unelos/skills/unelos-off/SKILL.md",
        include_bytes!("../../../../integrations/claude/unelos/skills/unelos-off/SKILL.md"),
    ),
    (
        "README.md",
        include_bytes!("../../../../integrations/claude/README.md"),
    ),
];

fn shell_path(binary: &str) -> String {
    if !binary.is_empty()
        && binary
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"_./+-".contains(&b))
    {
        binary.into()
    } else {
        format!("'{}'", binary.replace('\'', "'\\''"))
    }
}

fn json_string_content(text: &str) -> Result<String> {
    let quoted = serde_json::to_string(text).map_err(|e| error("invalid_binary_path", e))?;
    Ok(quoted[1..quoted.len() - 1].into())
}

fn render_assets(binary: &str, files: &[(&str, &[u8])]) -> Result<Vec<(String, String)>> {
    let shell = shell_path(binary);
    let raw_json = json_string_content(binary)?;
    let shell_json = json_string_content(&shell)?;
    let mut rendered = Vec::new();
    let mut hash = Sha256::new();
    for (name, bytes) in files {
        let template =
            std::str::from_utf8(bytes).map_err(|e| error("invalid_integration_asset", e))?;
        let text = template
            .replace("{{UNELOS_BIN_JSON}}", &raw_json)
            .replace("{{UNELOS_BIN_SHELL}}", &shell)
            .replace("{{UNELOS_BIN_SHELL_JSON}}", &shell_json);
        hash.update(name.as_bytes());
        hash.update([0]);
        hash.update(text.as_bytes());
        hash.update([0]);
        rendered.push(((*name).to_owned(), text));
    }
    let version = format!(
        "{INTEGRATION_VERSION}-{}",
        &format!("{:x}", hash.finalize())[..8]
    );
    for (_, text) in &mut rendered {
        *text = text.replace("{{UNELOS_PLUGIN_VERSION}}", &version);
    }
    Ok(rendered)
}

fn assets() -> Result<Utf8PathBuf> {
    Ok(global_config::config_dir()?.join("integrations/claude"))
}

fn cli(args: &[&str]) -> Result<String> {
    let binary = env_path("UNELOS_CLAUDE_BIN")
        .or_else(|| find_binary(Claude.binaries()))
        .ok_or_else(|| error("harness_not_detected", "claude is not on PATH"))?;
    process_output(&binary, args, Duration::from_secs(30))
}

fn enabled() -> Result<bool> {
    let directory = Claude
        .config_dir()
        .ok_or_else(|| error("config_dir_missing", "Set CLAUDE_CONFIG_DIR or HOME"))?;
    match fs::read(directory.join("settings.json")) {
        Ok(bytes) => {
            let settings: Value =
                serde_json::from_slice(&bytes).map_err(|e| error("invalid_claude_config", e))?;
            Ok(settings["enabledPlugins"][PLUGIN] == true)
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(io_error(e)),
    }
}

fn contains_id(value: &Value, id: &str) -> bool {
    match value {
        Value::String(s) => s == id,
        Value::Array(values) => values.iter().any(|v| contains_id(v, id)),
        Value::Object(values) => {
            values.contains_key(id) || values.values().any(|v| contains_id(v, id))
        }
        _ => false,
    }
}

fn listed(args: &[&str], id: &str) -> Result<bool> {
    let value: Value =
        serde_json::from_str(&cli(args)?).map_err(|e| error("invalid_claude_response", e))?;
    Ok(contains_id(&value, id))
}

fn render_agents(agents: &[crate::AgentExport]) -> Result<Vec<(Utf8PathBuf, String)>> {
    agents
        .iter()
        .map(|agent| {
            let mut front = json!({
                "name": crate::install::native_agent_id(&agent.id),
                "description": format!("Unelos {} role", agent.id),
            });
            if let Some(model) = agent
                .model
                .as_ref()
                .map(|model| {
                    crate::model_mappings::resolve_harness_model(
                        model,
                        agent.effort.as_deref(),
                        "claude",
                    )
                })
                .transpose()?
                .flatten()
            {
                front["model"] = model.model.into();
                if let Some(effort) = model.effort {
                    if !CLAUDE_EFFORTS.contains(&effort.as_str()) {
                        return Err(error(
                            "invalid_effort",
                            format!("Invalid Claude effort '{effort}'"),
                        ));
                    }
                    front["effort"] = effort.into();
                }
            }
            let mut servers = Vec::new();
            for (id, server) in &agent.mcp_servers {
                if id == "unelos" {
                    return Err(error("reserved_mcp_id", "MCP ID 'unelos' is reserved"));
                }
                let config = match server {
                    crate::McpServer::Local {
                        command,
                        environment,
                    } => {
                        let (command, args) = command
                            .split_first()
                            .ok_or_else(|| error("invalid_mcp", "Empty MCP command"))?;
                        json!({"type":"stdio", "command":command, "args":args, "env":environment})
                    }
                    crate::McpServer::Remote { url, headers } => {
                        json!({"type":"http", "url":url, "headers":headers})
                    }
                };
                servers.push(json!({id:config}));
            }
            if !servers.is_empty() {
                front["mcpServers"] = servers.into();
            }
            let yaml =
                serde_yaml_ng::to_string(&front).map_err(|e| error("agent_render_failed", e))?;
            let requirements = if agent.tools.is_empty() {
                String::new()
            } else {
                format!(
                    "\n\nRequired tools (availability, not permission grants): {}",
                    agent.tools.join(", ")
                )
            };
            Ok((
                Utf8PathBuf::from(format!(
                    ".claude/agents/{}.md",
                    crate::install::native_agent_id(&agent.id)
                )),
                format!("---\n{yaml}---\n{}{}\n", agent.instructions, requirements),
            ))
        })
        .collect()
}

impl HarnessAdapter for Claude {
    fn id(&self) -> &str {
        "claude"
    }
    fn name(&self) -> &str {
        "Claude Code"
    }
    fn binaries(&self) -> &[&str] {
        &["claude", "claude-code"]
    }
    fn config_dir(&self) -> Option<Utf8PathBuf> {
        env_path("CLAUDE_CONFIG_DIR").or_else(|| home().map(|p| p.join(".claude")))
    }
    fn installable(&self) -> bool {
        true
    }
    fn list_models(&self) -> Result<Vec<ModelInfo>> {
        Ok(["fable", "opus", "sonnet", "haiku"]
            .map(|id| ModelInfo {
                id: id.into(),
                efforts: CLAUDE_EFFORTS.map(str::to_owned).into(),
            })
            .into())
    }
    fn validate(&self) -> Result<()> {
        read_state()?;
        enabled()?;
        Ok(())
    }
    fn integration(&self, cx: &Ctx) -> Result<Option<Installed>> {
        let Some(record) = read_state()?.remove(self.id()) else {
            return Ok(None);
        };
        let root = assets()?;
        if !cx.binary.is_file()
            || !FILES.iter().all(|(name, _)| root.join(name).is_file())
            || !enabled()?
        {
            return Ok(None);
        }
        Ok(Some(record))
    }
    fn install(&self, cx: &Ctx) -> Result<Installed> {
        let root = assets()?;
        let files: Vec<_> = FILES.iter().map(|(name, _)| root.join(name)).collect();
        let global = global_config::config_dir()?;
        for file in &files {
            crate::check_install_path(&global, file.strip_prefix(&global).unwrap())?;
        }
        let mut state = read_state()?;
        let record = match state.get(self.id()) {
            Some(old) if old.version == INTEGRATION_VERSION && old.files == files => old.clone(),
            _ => Installed {
                version: INTEGRATION_VERSION.into(),
                installed_at: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map_err(io_error)?
                    .as_secs(),
                files,
            },
        };
        // Record ownership before writes so failed CLI registration remains uninstallable.
        state.insert(self.id().into(), record.clone());
        write_state(&state)?;
        let provisioned = fs::canonicalize(&cx.binary).map_err(io_error)?;
        let provisioned = Utf8PathBuf::from_path_buf(provisioned)
            .map_err(|_| error("invalid_binary_path", "Non UTF-8 binary path"))?;
        for (name, text) in render_assets(provisioned.as_str(), FILES)? {
            atomic_write(&root.join(name), text.as_bytes())?;
        }
        if !listed(&["plugin", "marketplace", "list", "--json"], "unelos-local")? {
            cli(&["plugin", "marketplace", "add", root.as_str()])?;
        }
        if listed(&["plugin", "list", "--json"], PLUGIN)? {
            cli(&["plugin", "marketplace", "update", "unelos-local"])?;
            cli(&["plugin", "update", PLUGIN, "--scope", "user"])?;
        } else {
            cli(&["plugin", "install", PLUGIN, "--scope", "user"])?;
        }
        if !enabled()? {
            cli(&["plugin", "enable", PLUGIN, "--scope", "user"])?;
        }
        Ok(record)
    }
    fn uninstall(&self, _cx: &Ctx) -> Result<()> {
        let root = assets()?;
        let mut state = read_state()?;
        if let Some(record) = state.get(self.id()) {
            let global = global_config::config_dir()?;
            for file in &record.files {
                if !FILES.iter().any(|(name, _)| root.join(name) == *file) {
                    return Err(error(
                        "invalid_harness_state",
                        format!("Unowned file: {file}"),
                    ));
                }
                crate::check_install_path(&global, file.strip_prefix(&global).unwrap())?;
            }
            if listed(&["plugin", "list", "--json"], PLUGIN)? {
                cli(&["plugin", "uninstall", PLUGIN, "--scope", "user"])?;
            }
            if listed(&["plugin", "marketplace", "list", "--json"], "unelos-local")? {
                cli(&[
                    "plugin",
                    "marketplace",
                    "remove",
                    "unelos-local",
                    "--scope",
                    "user",
                ])?;
            }
            for file in &record.files {
                match fs::remove_file(file) {
                    Ok(()) => (),
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
                    Err(e) => return Err(io_error(e)),
                }
            }
            state.remove(self.id());
            write_state(&state)?;
        }
        Ok(())
    }
    fn build_project(
        &self,
        root: &Utf8Path,
        workflows: &BTreeMap<String, Vec<crate::AgentExport>>,
        prune: bool,
    ) -> Result<crate::HarnessBuild> {
        let agents = crate::harness_build::merge_agents(workflows)?;
        let rendered = render_agents(&agents)?;
        let mut result =
            crate::harness_build::write_agents(root, ".claude/agents", rendered, prune)?;
        result.harnesses.push(self.id().into());
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn asset_renderer_quotes_by_context_and_versions_content() {
        for binary in ["/opt/unelos/bin/unelos", "/opt/Unelos user's/bin/unelos"] {
            let rendered: BTreeMap<_, _> =
                render_assets(binary, FILES).unwrap().into_iter().collect();
            let mcp: Value = serde_json::from_str(&rendered["unelos/.mcp.json"]).unwrap();
            assert_eq!(mcp["mcpServers"]["unelos"]["command"], binary);
            let shell = shell_path(binary);
            let hooks: Value = serde_json::from_str(&rendered["unelos/hooks/hooks.json"]).unwrap();
            assert_eq!(
                hooks["hooks"]["PreToolUse"][0]["hooks"][0]["command"],
                format!("{shell} hook claude pre-tool-use")
            );
            for (skill, mode) in [
                ("unelos", "new"),
                ("unelos-resume", "resume"),
                ("unelos-off", "off"),
            ] {
                let text = &rendered[&format!("unelos/skills/{skill}/SKILL.md")];
                assert!(text.contains(&format!("allowed-tools: Bash({shell} *)")));
                assert!(text.contains(&format!("!`{shell} session activate")));
                assert!(text.contains(&format!("--mode {mode} --json")));
                let command = text.lines().find(|line| line.starts_with("!`")).unwrap();
                assert!(!command.contains("$ARGUMENTS"));
            }
            #[cfg(unix)]
            {
                let output = Command::new("/bin/sh")
                    .args(["-c", &format!("printf '%s' {shell}")])
                    .output()
                    .unwrap();
                assert!(output.status.success());
                assert_eq!(String::from_utf8(output.stdout).unwrap(), binary);
            }
        }
        assert_eq!(
            shell_path("/opt/Unelos user's/bin/unelos"),
            "'/opt/Unelos user'\\''s/bin/unelos'"
        );
        let first = render_assets(
            "/bin/unelos",
            &[
                ("plugin.json", br#"{"version":"{{UNELOS_PLUGIN_VERSION}}"}"#),
                ("skill", b"original"),
            ],
        )
        .unwrap();
        let second = render_assets(
            "/bin/unelos",
            &[
                ("plugin.json", br#"{"version":"{{UNELOS_PLUGIN_VERSION}}"}"#),
                ("skill", b"changed"),
            ],
        )
        .unwrap();
        assert_ne!(first[0].1, second[0].1);
        let manifest: Value = serde_json::from_str(&first[0].1).unwrap();
        assert!(manifest["version"]
            .as_str()
            .unwrap()
            .starts_with(&format!("{INTEGRATION_VERSION}-")));
    }
}
