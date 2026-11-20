use super::*;
use global_config::{atomic_write, read_state, write_state};

pub struct OpenCode;

pub(crate) fn parse_opencode_model_list(json: &str) -> Option<Vec<ModelInfo>> {
    #[derive(Deserialize)]
    struct Listing {
        data: Vec<Model>,
    }
    #[derive(Deserialize)]
    struct Model {
        #[serde(rename = "providerID")]
        provider_id: String,
        id: String,
        variants: Vec<Variant>,
    }
    #[derive(Deserialize)]
    struct Variant {
        id: String,
    }
    let listing: Listing = serde_json::from_str(json).ok()?;
    let mut models: Vec<_> = listing
        .data
        .into_iter()
        .map(|model| ModelInfo {
            id: format!("{}/{}", model.provider_id, model.id),
            efforts: model
                .variants
                .into_iter()
                .map(|variant| variant.id)
                .collect(),
        })
        .collect();
    models.sort_by(|a, b| a.id.cmp(&b.id));
    Some(models)
}
pub(crate) fn resolve_agent_model(
    model: &crate::AgentModel,
    effort: Option<&str>,
) -> Result<String> {
    let resolved = crate::model_mappings::resolve_harness_model(model, effort, "opencode")?
        .ok_or_else(|| {
            let crate::AgentModel::Harness { harness, .. } = model else {
                unreachable!()
            };
            error(
                "model_harness_mismatch",
                format!("Model targets {harness}, expected opencode"),
            )
        })?;
    if effort.is_some() && resolved.model.contains('#') {
        return Err(error(
            "invalid_model",
            "Expected provider/model without inline variant when effort is set",
        ));
    }
    let model = match resolved.effort {
        Some(effort) => format!("{}#{effort}", resolved.model),
        None => resolved.model,
    };
    let (provider, rest) = model
        .split_once('/')
        .ok_or_else(|| error("invalid_model", "Expected provider/model[#variant]"))?;
    let parts: Vec<_> = rest.split('#').collect();
    if provider.is_empty()
        || parts.len() > 2
        || parts.iter().any(|s| s.is_empty())
        || model.chars().any(|c| c.is_whitespace() || c == '\0')
        || provider.contains('#')
    {
        return Err(error("invalid_model", "Expected provider/model[#variant]"));
    }
    Ok(model)
}
const FILES: &[(&str, &[u8])] = &[
    (
        "package.json",
        include_bytes!("../../../../integrations/opencode/package.json"),
    ),
    (
        "index.ts",
        include_bytes!("../../../../integrations/opencode/index.ts"),
    ),
    (
        "commands/unelos.md",
        include_bytes!("../../../../integrations/opencode/commands/unelos.md"),
    ),
];

fn assets() -> Result<Utf8PathBuf> {
    Ok(global_config::config_dir()?.join("integrations/opencode/unelos"))
}
fn config() -> Result<Utf8PathBuf> {
    let dir = OpenCode
        .config_dir()
        .ok_or_else(|| error("config_dir_missing", "Set OPENCODE_CONFIG_DIR or HOME"))?;
    let json = dir.join("opencode.json");
    let jsonc = dir.join("opencode.jsonc");
    if json.exists() && jsonc.exists() {
        return Err(error(
            "ambiguous_opencode_config",
            "Both opencode.json and opencode.jsonc exist",
        ));
    }
    Ok(if jsonc.exists() { jsonc } else { json })
}
fn text(path: &Utf8Path) -> Result<String> {
    match fs::read_to_string(path) {
        Ok(s) => Ok(s),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok("{}\n".into()),
        Err(e) => Err(io_error(e)),
    }
}

pub(crate) fn config_unreadable() -> bool {
    config()
        .and_then(|path| text(&path))
        .and_then(|text| jsonc::config_value(&text))
        .is_err()
}
pub(super) fn validate() -> Result<()> {
    let config = config()?;
    let package = assets()?;
    jsonc::configured_text(
        &text(&config)?,
        package.as_str(),
        "unelos",
        package.parent().unwrap(),
        &config,
    )?;
    read_state()?;
    Ok(())
}
impl HarnessAdapter for OpenCode {
    fn validate(&self) -> Result<()> {
        validate()
    }
    fn build_project(
        &self,
        root: &Utf8Path,
        workflows: &std::collections::BTreeMap<String, Vec<crate::AgentExport>>,
        legacy_globals: &[crate::AgentExport],
        prune: bool,
    ) -> Result<crate::HarnessBuild> {
        let mut agents = crate::harness_build::merge_agents(workflows)?;
        agents.retain(|a| !a.id.starts_with("global/"));
        crate::opencode_agents::check_agents(root, &agents)?;
        let mut rendered =
            crate::opencode_agents::render_agents(crate::opencode_agents::Version::V2, &agents)?;
        let (path, text, state) = crate::mcp_install::plan_replace(root, workflows, prune)?;
        if state.values().any(|servers| !servers.is_empty())
            || root.join(".unelos/opencode-mcps.json").exists()
        {
            rendered.push((
                Utf8PathBuf::from(".unelos/opencode-mcps.json"),
                serde_json::to_string_pretty(&state).map_err(io_error)?,
            ));
            rendered.push((path.strip_prefix(root).unwrap().to_owned(), text));
        }
        let legacy = crate::opencode_agents::render_agents(
            crate::opencode_agents::Version::V2,
            legacy_globals,
        )
        .unwrap_or_default();
        let legacy = legacy
            .into_iter()
            .flat_map(|(path, text)| {
                ["subagent", "primary", "all"].map(|mode| {
                    (
                        path.clone(),
                        text.replacen("mode: subagent\n", &format!("mode: {mode}\n"), 1),
                    )
                })
            })
            .collect();
        let mut result =
            crate::harness_build::write_agents(root, ".opencode/agents", rendered, legacy, prune)?;
        result.harnesses.push(self.id().into());
        Ok(result)
    }
    fn build_global_agents(
        &self,
        agents: &[crate::AgentExport],
        prune: bool,
    ) -> Result<crate::HarnessBuild> {
        let root = self
            .config_dir()
            .ok_or_else(|| error("config_dir_missing", "Set OPENCODE_CONFIG_DIR or HOME"))?;
        let mut agents = agents.to_vec();
        let path = config()?;
        crate::mcp_install::check_uninstall_file(&root, path.file_name().unwrap())?;
        let mut mcp_result =
            crate::mcp_install::sync_global(&path, &text(&path)?, &mut agents, prune, false)?;
        let rendered =
            crate::opencode_agents::render_agents(crate::opencode_agents::Version::V2, &agents)?;
        mcp_result.append(crate::harness_build::write_global_agents(
            &root,
            self.id(),
            rendered,
            prune,
        )?);
        Ok(mcp_result)
    }
    fn id(&self) -> &str {
        "opencode"
    }
    fn name(&self) -> &str {
        "OpenCode"
    }
    fn binaries(&self) -> &[&str] {
        &["opencode"]
    }
    fn config_dir(&self) -> Option<Utf8PathBuf> {
        env_path("OPENCODE_CONFIG_DIR")
            .or_else(|| env_path("XDG_CONFIG_HOME").map(|p| p.join("opencode")))
            .or_else(|| home().map(|p| p.join(".config/opencode")))
    }
    fn installable(&self) -> bool {
        true
    }
    fn integration(&self, cx: &Ctx) -> Result<Option<Installed>> {
        let mut state = read_state()?;
        let Some(record) = state.remove(self.id()) else {
            return Ok(None);
        };
        let config = config()?;
        let original = text(&config)?;
        let root = assets()?;
        let edited = jsonc::remove_registration(&original, root.parent().unwrap(), &config)?;
        if original == edited
            || !cx.binary.is_file()
            || !FILES.iter().all(|(name, _)| root.join(name).is_file())
        {
            return Ok(None);
        }
        Ok(Some(record))
    }
    fn install(&self, cx: &Ctx) -> Result<Installed> {
        let path = config()?;
        let root = assets()?;
        let original = text(&path)?;
        let edited = jsonc::configured_text(
            &original,
            root.as_str(),
            cx.binary.as_str(),
            root.parent().unwrap(),
            &path,
        )?;
        let mut state = read_state()?;
        let mut files = FILES
            .iter()
            .map(|(name, _)| root.join(name))
            .collect::<Vec<_>>();
        let legacy = root.join("index.js");
        if legacy.exists() {
            files.push(legacy.clone());
        }
        let global = global_config::config_dir()?;
        for file in &files {
            crate::check_install_path(&global, file.strip_prefix(&global).unwrap())?;
        }
        // Record ownership before writes so a partially failed install can be uninstalled.
        let mut record = match state.get(self.id()) {
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
        state.insert(self.id().into(), record.clone());
        write_state(&state)?;
        for (name, bytes) in FILES {
            atomic_write(&root.join(name), bytes)?;
        }
        if record.files.contains(&legacy) {
            fs::remove_file(&legacy).map_err(io_error)?;
            record.files.retain(|file| file != &legacy);
            state.insert(self.id().into(), record.clone());
            write_state(&state)?;
        }
        atomic_write(&path, edited.as_bytes())?;
        Ok(record)
    }
    fn uninstall(&self, _cx: &Ctx) -> Result<()> {
        let path = config()?;
        let root = assets()?;
        crate::mcp_install::check_uninstall_file(
            path.parent().unwrap(),
            path.file_name().unwrap(),
        )?;
        let original = text(&path)?;
        jsonc::remove_registration(&original, root.parent().unwrap(), &path)?;
        let mut state = read_state()?;
        if let Some(record) = state.get(self.id()) {
            for file in &record.files {
                if !FILES.iter().any(|(name, _)| root.join(name) == *file)
                    && *file != root.join("index.js")
                {
                    return Err(error(
                        "invalid_harness_state",
                        format!("Unowned file: {file}"),
                    ));
                }
                let global = global_config::config_dir()?;
                crate::check_install_path(&global, file.strip_prefix(&global).unwrap())?;
            }
        }
        let cleanup = crate::mcp_install::sync_global(&path, &original, &mut [], true, true)?;
        for skipped in cleanup.skipped {
            eprintln!("{skipped}");
        }
        let original = text(&path)?;
        let edited = jsonc::remove_registration(&original, root.parent().unwrap(), &path)?;
        if let Some(record) = state.get(self.id()) {
            for file in &record.files {
                match fs::remove_file(file) {
                    Ok(()) => (),
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
                    Err(e) => return Err(io_error(e)),
                }
            }
        }
        if original != edited {
            atomic_write(&path, edited.as_bytes())?;
        }
        if state.remove(self.id()).is_some() {
            write_state(&state)?;
        }
        Ok(())
    }
    fn list_models(&self) -> Result<Vec<ModelInfo>> {
        let bin = find_binary(self.binaries())
            .ok_or_else(|| error("harness_not_detected", "opencode is not on PATH"))?;
        if let Some(models) = process_output(&bin, &["api", "model.list"], Duration::from_secs(30))
            .ok()
            .and_then(|json| parse_opencode_model_list(&json))
        {
            return Ok(models);
        }
        Ok(process_output(&bin, &["models"], Duration::from_secs(30))?
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty())
            .map(|id| ModelInfo {
                id: id.into(),
                efforts: vec![],
            })
            .collect())
    }
}

pub(super) fn migrate_project(root: &Utf8Path) -> Result<bool> {
    let mut changes = Vec::new();
    for name in [
        "opencode.json",
        "opencode.jsonc",
        ".opencode/opencode.json",
        ".opencode/opencode.jsonc",
    ] {
        crate::check_install_path(root, Utf8Path::new(name))?;
        let path = root.join(name);
        if !path.exists() {
            continue;
        }
        let original = text(&path)?;
        let edited = jsonc::remove_registration(&original, &root.join(".unelos/opencode"), &path)?;
        if original != edited {
            changes.push((path, edited));
        }
    }
    if changes.is_empty() {
        return Ok(false);
    }
    // Only known bundled files are removed; preserve user-created files in these directories.
    let mut files = Vec::new();
    for dir in ["unelos", "thin-plugin", "plugin"] {
        for name in [
            "package.json",
            "index.js",
            "index.ts",
            "v1.js",
            "commands/unelos.md",
        ] {
            files.push(Utf8PathBuf::from(".unelos/opencode").join(dir).join(name));
        }
    }
    files.push(".unelos/integration.json".into());
    for file in &files {
        crate::check_install_path(root, file)?;
    }
    for (path, edited) in changes {
        atomic_write(&path, edited.as_bytes())?;
    }
    for file in files {
        match fs::remove_file(root.join(file)) {
            Ok(()) => (),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
            Err(e) => return Err(io_error(e)),
        }
    }
    let command = root.join(".opencode/commands/unelos.md");
    crate::check_install_path(root, Utf8Path::new(".opencode/commands/unelos.md"))?;
    if fs::read(&command).ok().as_deref() == Some(FILES[2].1) {
        fs::remove_file(command).map_err(io_error)?;
    }
    Ok(true)
}
