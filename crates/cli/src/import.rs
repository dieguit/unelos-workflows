use super::*;
use std::{collections::BTreeMap, sync::atomic::AtomicBool};
use unelos_core::{commit_import, fetch_source, plan_import, ImportOutcome, ImportPlan};

fn error(code: &'static str, message: impl Into<String>) -> Error {
    unelos_core::InstallError {
        code,
        message: message.into(),
    }
    .into()
}

pub(super) fn parse_renames(values: &[String]) -> Result<BTreeMap<String, String>, Error> {
    let mut renames = BTreeMap::new();
    for value in values {
        let parsed = value.split_once('=').and_then(|(key, name)| {
            let (kind, id) = key.split_once(':')?;
            (matches!(kind, "workflow" | "agent" | "task" | "skill" | "mcp")
                && !id.is_empty()
                && !id.contains(':')
                && !name.contains('='))
            .then_some((key.to_owned(), name.to_owned()))
        });
        let (key, name) = parsed.ok_or_else(|| error("invalid_rename", format!("Invalid --rename {value:?}; expected kind:id=new (workflow, agent, task, skill, mcp)")))?;
        renames.insert(key, name);
    }
    Ok(renames)
}

fn conflicts(cli: &Cli, plan: &ImportPlan) -> Result<(), Error> {
    if cli.json {
        emit(
            &serde_json::json!({"status":"conflicts","plan":plan}),
            true,
            String::new(),
        )?;
    } else {
        eprintln!("KEY\tSUGGESTED\tERROR");
        for row in &plan.conflicts {
            eprintln!(
                "{}\t{}\t{}",
                row.key,
                row.suggested,
                row.error.as_deref().unwrap_or("-")
            );
        }
    }
    Err(error(
        "import_conflicts",
        "Resolve conflicts and re-run with --rename kind:id=new or --accept-suggested",
    ))
}

pub(super) fn install(
    cli: &Cli,
    source: &str,
    mut renames: BTreeMap<String, String>,
    trust_mcps: bool,
    accept_suggested: bool,
    selected: Option<opencode::Version>,
) -> Result<(), Error> {
    let fetched = fetch_source(source, &AtomicBool::new(false))?;
    let plan = plan_import(&cli.project, fetched.dir(), source, &renames)?;
    if !cli.json {
        for warning in &plan.warnings {
            eprintln!("Warning: {}", warning.message);
        }
    }
    if !plan.mcps.is_empty() && !trust_mcps {
        let list = plan
            .mcps
            .iter()
            .map(|mcp| {
                format!(
                    "{}: {}",
                    mcp.id,
                    mcp.command.as_deref().or(mcp.url.as_deref()).unwrap_or("-")
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        let mut trusted = false;
        if !cli.json && io::stdin().is_terminal() {
            eprintln!("{list}\nTrust these MCPs? [y/N]");
            let mut answer = String::new();
            io::stdin().read_line(&mut answer)?;
            trusted = matches!(answer.trim().to_ascii_lowercase().as_str(), "y" | "yes");
        }
        if !trusted {
            return Err(error(
                "mcp_trust_required",
                format!(
                    "MCP trust required; re-run with --trust-mcps\n{list}\nWarnings: {}",
                    serde_json::to_string(&plan.warnings)?
                ),
            ));
        }
    }
    if plan
        .conflicts
        .iter()
        .any(|row| row.error.is_some() || (!accept_suggested && !renames.contains_key(&row.key)))
    {
        return conflicts(cli, &plan);
    }
    for row in &plan.conflicts {
        renames
            .entry(row.key.clone())
            .or_insert_with(|| row.suggested.clone());
    }
    let workflow = renames
        .get(&format!("workflow:{}", plan.workflow.id))
        .unwrap_or(&plan.workflow.id);
    if selected.is_some() {
        let mut agents = unelos_core::export_workflow_agents(fetched.dir(), &plan.workflow.id)?;
        for agent in &mut agents {
            agent.id = renames
                .get(&format!("agent:{}", agent.id))
                .unwrap_or(&agent.id)
                .clone();
            for mcp in &mut agent.mcps {
                *mcp = renames.get(&format!("mcp:{mcp}")).unwrap_or(mcp).clone();
            }
            agent.mcp_servers = std::mem::take(&mut agent.mcp_servers)
                .into_iter()
                .map(|(id, server)| {
                    (
                        renames.get(&format!("mcp:{id}")).unwrap_or(&id).clone(),
                        server,
                    )
                })
                .collect();
        }
        opencode::check_agents(&cli.project, &agents)?;
        unelos_core::check_opencode_mcps(&cli.project, workflow, &agents)?;
    }
    match commit_import(&cli.project, fetched.dir(), source, &renames)? {
        ImportOutcome::Conflicts(plan) => conflicts(cli, &plan),
        ImportOutcome::Imported {
            workflow, agents, ..
        } => {
            let native_agents = if let Some(version) = selected {
                unelos_core::write_opencode_mcps(&cli.project, &workflow, &agents)?;
                opencode::write_agents(&cli.project, version, &agents)?
            } else {
                Vec::new()
            };
            if !cli.json {
                for (key, name) in &renames {
                    eprintln!("Renamed {key} → {name}");
                }
            }
            emit(
                &serde_json::json!({"workflow":workflow,"agents":agents,"nativeAgents":native_agents,"warnings":plan.warnings}),
                cli.json,
                format!("Installed workflow {workflow}"),
            )
        }
    }
}
