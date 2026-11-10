use camino::{Utf8Path, Utf8PathBuf};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fs};
use crate::{AgentDefinition, AgentModel};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentDocument {
    pub id: String,
    pub instructions: String,
    pub skills: Vec<String>,
    pub mcps: Vec<String>,
    pub tools: Vec<String>,
    pub model: Option<AgentModel>,
}

fn paths(root: &Utf8Path, id: &str) -> Result<(Utf8PathBuf, Utf8PathBuf), String> {
    if !crate::project::safe_id(id) { return Err(format!("Invalid Agent ID {id:?}")); }
    let root = crate::project::absolute_root(root).map_err(|e| e.to_string())?.join(".unelos");
    let source = crate::project::bounded_path(&root, &Utf8PathBuf::from(format!("agents/{id}/agent.yaml")), &root, "agent").map_err(|e| e.to_string())?;
    Ok((root, source))
}

fn load(root: &Utf8Path, source: &Utf8Path) -> Result<(AgentDefinition, BTreeMap<Utf8PathBuf, std::collections::BTreeSet<Utf8PathBuf>>), String> {
    let text = fs::read_to_string(source).map_err(|e| e.to_string())?;
    let mut agent = crate::parse_agent_definition(source.as_str(), &text).map_err(|e| e.to_string())?;
    let mut dependencies = BTreeMap::new();
    crate::project::resolve_instructions(root, source, "instructionsFile", &agent.instructions_file, &mut agent.instructions, &mut dependencies).map_err(|e| e.to_string())?;
    Ok((agent, dependencies))
}

pub fn get_agent(root: &Utf8Path, id: &str) -> Result<AgentDocument, String> {
    let (root, source) = paths(root, id)?;
    let (agent, _) = load(&root, &source)?;
    Ok(AgentDocument { id: id.into(), instructions: agent.instructions, skills: agent.skills, mcps: agent.mcps, tools: agent.tools, model: agent.model })
}

pub fn save_agent(root: &Utf8Path, original_id: Option<&str>, agent: AgentDocument) -> Result<AgentDocument, String> {
    let (authoring, source) = paths(root, &agent.id)?;
    let (instructions_file, dependencies) = if let Some(id) = original_id {
        if id != agent.id { return Err("Agent IDs are read-only".into()); }
        let (old, dependencies) = load(&authoring, &source)?;
        (old.instructions_file, dependencies)
    } else {
        if source.parent().unwrap().exists() { return Err(format!("Agent '{}' already exists", agent.id)); }
        (None, BTreeMap::new())
    };
    let definition = AgentDefinition { instructions: agent.instructions.clone(), instructions_file, skills: agent.skills, mcps: agent.mcps, tools: agent.tools, model: agent.model };
    let yaml = serde_yaml_ng::to_string(&definition).map_err(|e| e.to_string())?;
    crate::parse_agent_definition(source.as_str(), &yaml).map_err(|e| e.to_string())?;
    fs::create_dir_all(source.parent().unwrap()).map_err(|e| e.to_string())?;
    let instruction_path = dependencies.get(&source).and_then(|paths| paths.first()).map(|path| authoring.join(path));
    let original = instruction_path.as_ref().map(fs::read_to_string).transpose().map_err(|e| e.to_string())?;
    if let Some(path) = &instruction_path { crate::skill_document::atomic_write(path, &agent.instructions)?; }
    if let Err(error) = crate::skill_document::atomic_write(&source, &yaml) {
        if let (Some(path), Some(original)) = (instruction_path, original) {
            crate::skill_document::atomic_write(&path, &original).map_err(|rollback| format!("{error}; rollback: {rollback}"))?;
        }
        return Err(error);
    }
    get_agent(root, &agent.id)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentModelOptions { pub aliases: Vec<String>, pub harnesses: Vec<HarnessModelOptions> }
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HarnessModelOptions { pub id: String, pub name: String, pub models: Vec<String>, pub error: Option<String> }

pub fn agent_model_options() -> AgentModelOptions {
    AgentModelOptions {
        aliases: crate::list_model_mappings().unwrap_or_default().into_keys().collect(),
        harnesses: crate::list_harnesses().into_iter().filter(|h| h.detected || h.installed).map(|h| {
            let (models, error) = match crate::harness_models(&h.id) { Ok(models) => (models, None), Err(error) => (vec![], Some(error.to_string())) };
            HarnessModelOptions { id: h.id, name: h.name, models, error }
        }).collect(),
    }
}
