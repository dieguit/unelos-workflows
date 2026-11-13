use crate::{NodeDefinition, TaskSelection};
use camino::{Utf8Path, Utf8PathBuf};
use std::{collections::BTreeMap, fs};

fn path(root: &Utf8Path, kind: &str, id: &str) -> Result<Utf8PathBuf, String> {
    authoring_path(
        &crate::project::absolute_root(root)
            .map_err(|e| e.to_string())?
            .join(".unelos"),
        kind,
        id,
    )
}

fn authoring_path(authoring: &Utf8Path, kind: &str, id: &str) -> Result<Utf8PathBuf, String> {
    match kind {
        "skills" => crate::skill_document::valid_id(id)?,
        "agents" | "tasks" if crate::project::safe_id(id) => (),
        "workflows" if !id.is_empty() && !matches!(id, "." | "..") && !id.contains(['/', '\\']) => {
        }
        _ => return Err(format!("Invalid {kind} ID {id:?}")),
    }
    let relative = if kind == "workflows" {
        format!("{kind}/{id}.yaml")
    } else {
        format!("{kind}/{id}")
    };
    crate::project::bounded_path(authoring, Utf8Path::new(&relative), authoring, kind)
        .map_err(|e| e.to_string())
}

fn absent(path: &Utf8Path) -> Result<(), String> {
    match fs::symlink_metadata(path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e.to_string()),
        Ok(_) => Err(format!("Already exists: {path}")),
    }
}

fn entries(path: &Utf8Path) -> Result<Vec<Utf8PathBuf>, String> {
    let mut result = Vec::new();
    for entry in fs::read_dir(path).map_err(|e| format!("{path}: {e}"))? {
        let entry = entry.map_err(|e| e.to_string())?;
        result.push(
            Utf8PathBuf::from_path_buf(entry.path()).map_err(|_| "Non-UTF-8 authoring path")?,
        );
    }
    result.sort();
    Ok(result)
}

fn check_tree(path: &Utf8Path) -> Result<(), String> {
    let kind = fs::symlink_metadata(path)
        .map_err(|e| format!("{path}: {e}"))?
        .file_type();
    if kind.is_symlink() || (!kind.is_dir() && !kind.is_file()) {
        return Err(format!("Unsupported authoring entry: {path}"));
    }
    if kind.is_dir() {
        for child in entries(path)? {
            check_tree(&child)?;
        }
    }
    Ok(())
}

fn copy_tree(source: &Utf8Path, target: &Utf8Path) -> Result<(), String> {
    if source.is_dir() {
        fs::create_dir(target).map_err(|e| e.to_string())?;
        for child in entries(source)? {
            copy_tree(&child, &target.join(child.file_name().unwrap()))?;
        }
    } else {
        let mut input = fs::File::open(source).map_err(|e| e.to_string())?;
        let mut output = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(target)
            .map_err(|e| e.to_string())?;
        std::io::copy(&mut input, &mut output).map_err(|e| e.to_string())?;
    }
    fs::set_permissions(
        target,
        fs::metadata(source)
            .map_err(|e| e.to_string())?
            .permissions(),
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

fn duplicate(root: &Utf8Path, kind: &str, id: &str, new_id: &str) -> Result<(), String> {
    duplicate_at(
        &crate::project::absolute_root(root)
            .map_err(|e| e.to_string())?
            .join(".unelos"),
        kind,
        id,
        new_id,
    )
}

fn duplicate_at(root: &Utf8Path, kind: &str, id: &str, new_id: &str) -> Result<(), String> {
    if kind != "skills" && !crate::project::safe_id(new_id) {
        return Err("Name must contain 1–128 ASCII letters, digits, underscores or hyphens".into());
    }
    let source = authoring_path(root, kind, id)?;
    let target = authoring_path(root, kind, new_id)?;
    absent(&target)?;
    check_tree(&source)?;
    if kind == "workflows" {
        let text = fs::read_to_string(&source).map_err(|e| e.to_string())?;
        let mut value: serde_yaml_ng::Value =
            serde_yaml_ng::from_str(&text).map_err(|e| e.to_string())?;
        value
            .as_mapping_mut()
            .ok_or("Workflow must be a mapping")?
            .insert("name".into(), new_id.into());
        let yaml = serde_yaml_ng::to_string(&value).map_err(|e| e.to_string())?;
        let staging = target.with_extension("duplicate-stage");
        fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&staging)
            .map_err(|e| e.to_string())?;
        let result = crate::skill_document::atomic_write(&staging, &yaml)
            .and_then(|_| fs::hard_link(&staging, &target).map_err(|e| e.to_string()));
        let _ = fs::remove_file(&staging);
        return result;
    }
    let definition = source.join(match kind {
        "skills" => "SKILL.md",
        "tasks" => "task.yaml",
        _ => "agent.yaml",
    });
    if !definition.is_file() {
        return Err(format!("Missing definition: {definition}"));
    }
    #[cfg_attr(not(unix), allow(unused_mut))]
    let mut directory = fs::DirBuilder::new();
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        directory.mode(0o700);
    }
    directory.create(&target).map_err(|e| e.to_string())?;
    // shortcut: the reserved target is visible while copying, stage publication if readers need atomic visibility.
    let result = (|| {
        for child in entries(&source)? {
            copy_tree(&child, &target.join(child.file_name().unwrap()))?;
        }
        fs::set_permissions(
            &target,
            fs::metadata(&source)
                .map_err(|e| e.to_string())?
                .permissions(),
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    })();
    if result.is_err() {
        fs::remove_dir_all(&target).map_err(|cleanup| format!("{result:?}; cleanup: {cleanup}"))?;
    }
    result
}

fn instruction_reference(
    authoring: &Utf8Path,
    source: &Utf8Path,
    file: &Option<String>,
    removed: &Utf8Path,
) -> Result<(), String> {
    let mut dependencies = BTreeMap::new();
    crate::project::resolve_instructions(
        authoring,
        source,
        "instructionsFile",
        file,
        &mut String::new(),
        &mut dependencies,
    )
    .map_err(|e| e.to_string())?;
    if dependencies
        .values()
        .flatten()
        .any(|relative| authoring.join(relative).starts_with(removed))
    {
        return Err(format!(
            "Cannot remove: {source} references instructions inside {removed}"
        ));
    }
    Ok(())
}

fn task_reference(
    authoring: &Utf8Path,
    source: &Utf8Path,
    task: &crate::TaskDefinition,
    kind: &str,
    id: &str,
    removed: &Utf8Path,
) -> Result<(), String> {
    if kind == "skills" && task.skills.iter().any(|skill| skill == id) {
        return Err(format!(
            "Cannot remove Skill {id:?}: referenced by {source}"
        ));
    }
    instruction_reference(authoring, source, &task.instructions_file, removed)
}

fn references(root: &Utf8Path, kind: &str, id: &str, removed: &Utf8Path) -> Result<(), String> {
    // shortcut: run creation is not locked with authoring, add a shared lock before supporting concurrent edits/execution.
    let runs = crate::runtime::stored_runs(root)
        .map_err(|e| format!("Cannot inspect stored runs: {e}"))?;
    if kind == "workflows"
        && runs
            .iter()
            .any(|run| run.workflow_name == id && !run.is_finished())
    {
        return Err("Cannot remove workflow: finish or remove its in-progress runs first".into());
    }
    let authoring = crate::project::absolute_root(root)
        .map_err(|e| e.to_string())?
        .join(".unelos");
    for directory in ["agents", "tasks", "workflows"] {
        let folder = crate::project::bounded_path(
            &authoring,
            Utf8Path::new(directory),
            &authoring,
            directory,
        )
        .map_err(|e| e.to_string())?;
        if !folder.try_exists().map_err(|e| e.to_string())? {
            continue;
        }
        for entry in entries(&folder)? {
            if entry == removed {
                continue;
            }
            check_tree(&entry)?;
            let source = match directory {
                "agents" | "tasks" if entry.is_dir() => entry.join(if directory == "agents" {
                    "agent.yaml"
                } else {
                    "task.yaml"
                }),
                "workflows" if entry.extension() == Some("yaml") && entry.is_file() => entry,
                _ => continue,
            };
            let text = fs::read_to_string(&source).map_err(|e| format!("{source}: {e}"))?;
            match directory {
                "agents" => {
                    let agent = crate::parse_agent_definition(source.as_str(), &text)
                        .map_err(|e| e.to_string())?;
                    if kind == "skills" && agent.skills.iter().any(|skill| skill == id) {
                        return Err(format!(
                            "Cannot remove Skill {id:?}: referenced by {source}"
                        ));
                    }
                    instruction_reference(&authoring, &source, &agent.instructions_file, removed)?;
                }
                "tasks" => task_reference(
                    &authoring,
                    &source,
                    &crate::parse_task_definition(source.as_str(), &text)
                        .map_err(|e| e.to_string())?,
                    kind,
                    id,
                    removed,
                )?,
                _ => {
                    let workflow = crate::parse_workflow_definition(source.as_str(), &text)
                        .map_err(|e| e.to_string())?;
                    for node in workflow.nodes.values() {
                        let workers: Vec<_> = match node {
                            NodeDefinition::Task(task) => vec![(&task.agent, &task.task)],
                            NodeDefinition::Parallel(parallel) => parallel
                                .tasks
                                .iter()
                                .map(|worker| (&worker.agent, &worker.task))
                                .collect(),
                            _ => vec![],
                        };
                        for (agent, task) in workers {
                            if kind == "agents" && agent == id {
                                return Err(format!(
                                    "Cannot remove Agent {id:?}: referenced by {source}"
                                ));
                            }
                            if kind == "tasks"
                                && matches!(task, TaskSelection::Named(name) if name == id)
                            {
                                return Err(format!(
                                    "Cannot remove Task {id:?}: referenced by {source}"
                                ));
                            }
                            if let TaskSelection::Inline(task) = task {
                                task_reference(&authoring, &source, task, kind, id, removed)?;
                            }
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

fn remove(root: &Utf8Path, kind: &str, id: &str) -> Result<(), String> {
    let source = path(root, kind, id)?;
    check_tree(&source)?;
    references(root, kind, id, &source)?;
    if kind == "workflows" {
        return fs::remove_file(&source).map_err(|e| e.to_string());
    }
    remove_directory(&source)
}

fn remove_directory(source: &Utf8Path) -> Result<(), String> {
    let staging = source.with_file_name(format!(".{}.remove-stage", source.file_name().unwrap()));
    absent(&staging)?;
    fs::rename(source, &staging).map_err(|e| e.to_string())?;
    // shortcut: recursive delete can partially progress, use recoverable trash if transactional deletion is required.
    if let Err(error) = fs::remove_dir_all(&staging) {
        fs::rename(&staging, source)
            .map_err(|rollback| format!("{error}; rollback: {rollback}"))?;
        return Err(error.to_string());
    }
    Ok(())
}

pub fn duplicate_agent(root: &Utf8Path, id: &str, new_id: &str) -> Result<(), String> {
    duplicate(root, "agents", id, new_id)
}
pub fn duplicate_skill(root: &Utf8Path, id: &str, new_id: &str) -> Result<(), String> {
    duplicate(root, "skills", id, new_id)
}
pub fn duplicate_workflow(root: &Utf8Path, id: &str, new_id: &str) -> Result<(), String> {
    duplicate(root, "workflows", id, new_id)
}
pub fn remove_agent(root: &Utf8Path, id: &str) -> Result<(), String> {
    remove(root, "agents", id)
}
pub fn remove_skill(root: &Utf8Path, id: &str) -> Result<(), String> {
    remove(root, "skills", id)
}
pub fn remove_workflow(root: &Utf8Path, id: &str) -> Result<(), String> {
    remove(root, "workflows", id)
}

pub fn duplicate_task(root: &Utf8Path, id: &str, new_id: &str) -> Result<(), String> {
    duplicate(root, "tasks", id, new_id)
}
pub fn remove_task(root: &Utf8Path, id: &str) -> Result<(), String> {
    remove(root, "tasks", id)
}

pub fn duplicate_global_task(id: &str, new_id: &str) -> Result<(), String> {
    duplicate_at(&crate::task_document::global_root()?, "tasks", id, new_id)
}
pub fn remove_global_task(id: &str) -> Result<(), String> {
    let source = authoring_path(&crate::task_document::global_root()?, "tasks", id)?;
    check_tree(&source)?;
    remove_directory(&source)
}
