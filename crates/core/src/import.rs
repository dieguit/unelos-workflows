use crate::install::{
    checked_path, collect_files, collect_import_files, fail, io_error, references,
};
use crate::{AgentExport, InstallError, LoadedProject, McpServer};
use camino::{Utf8Path, Utf8PathBuf};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    process::{Command, Stdio},
    sync::atomic::{AtomicBool, Ordering},
    thread,
    time::Duration,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceSpec {
    Local(Utf8PathBuf),
    Git {
        url: String,
        reference: Option<String>,
        subpath: Utf8PathBuf,
    },
}

// Folder links copied from GitHub/GitLab: `<repo>/tree/<ref>/<path>` or `<repo>/-/tree/<ref>/<path>`.
// shortcut: refs containing `/` are misread as ref + path; resolve via `git ls-remote` if users hit it.
fn browser_tree_spec(spec: &str) -> Option<String> {
    if !spec.starts_with("https://") || spec.contains('#') {
        return None;
    }
    let spec = spec.split(['?', '#']).next()?;
    let (repo, rest) = spec
        .split_once("/-/tree/")
        .or_else(|| spec.split_once("/tree/"))?;
    let (reference, subpath) = rest.split_once('/').unwrap_or((rest, ""));
    Some(format!(
        "{repo}#{reference}:{}",
        subpath.trim_end_matches('/')
    ))
}

pub fn parse_source_spec(spec: &str) -> Result<SourceSpec, InstallError> {
    let tree = browser_tree_spec(spec);
    let spec = tree.as_deref().unwrap_or(spec);
    if spec.starts_with("https://")
        || spec.starts_with("http://")
        || spec.starts_with("ssh://")
        || spec.starts_with("git://")
        || spec.starts_with("git@")
        || spec.starts_with("file://")
    {
        let (url, suffix) = spec.split_once('#').unwrap_or((spec, ""));
        let (reference, subpath) = suffix.split_once(':').unwrap_or((suffix, ""));
        let subpath = Utf8PathBuf::from(subpath);
        if subpath
            .components()
            .any(|c| !matches!(c, camino::Utf8Component::Normal(_)))
        {
            return Err(fail(
                "invalid_path",
                "Repository subpath must stay inside the clone",
            ));
        }
        return Ok(SourceSpec::Git {
            url: url.into(),
            reference: (!reference.is_empty()).then(|| reference.into()),
            subpath,
        });
    }
    if spec.is_empty() {
        return Err(fail("invalid_source", "Enter a source"));
    }
    let path = if spec == "~" || spec.starts_with("~/") {
        let home = crate::global_config::home()
            .ok_or_else(|| fail("invalid_source", "Cannot expand ~ without HOME"))?;
        home.join(spec.strip_prefix("~/").unwrap_or(""))
    } else {
        Utf8PathBuf::from(spec)
    };
    Ok(SourceSpec::Local(path))
}

pub struct FetchedSource {
    dir: Utf8PathBuf,
    _temporary: Option<tempfile::TempDir>,
}

impl FetchedSource {
    pub fn dir(&self) -> &Utf8Path {
        &self.dir
    }
}

fn ssh_url(url: &str) -> Option<String> {
    let rest = url.strip_prefix("https://")?;
    let (host, path) = rest.split_once('/')?;
    if host.contains('@') || path.is_empty() {
        return None;
    }
    let path = path.trim_end_matches('/');
    Some(format!(
        "git@{host}:{}.git",
        path.strip_suffix(".git").unwrap_or(path)
    ))
}

/// Outer error: cancelled or I/O; inner error: git's stderr.
fn clone_repo(
    url: &str,
    reference: Option<&str>,
    clone: &Utf8Path,
    cancel: &AtomicBool,
) -> Result<Result<(), String>, InstallError> {
    let stderr = tempfile::tempfile().map_err(io_error)?;
    let mut command = Command::new("git");
    command
        .env("GIT_TERMINAL_PROMPT", "0")
        .args(["clone", "--depth", "1"]);
    if let Some(reference) = reference {
        command.args(["--branch", reference]);
    }
    let mut child = match command
        .arg("--")
        .arg(url)
        .arg(clone)
        .stdout(Stdio::null())
        .stderr(stderr.try_clone().map_err(io_error)?)
        .spawn()
    {
        Ok(child) => child,
        Err(error) => return Ok(Err(error.to_string())),
    };
    let status = loop {
        if cancel.load(Ordering::Relaxed) {
            let _ = child.kill();
            let _ = child.wait();
            return Err(fail("import_cancelled", "Import cancelled"));
        }
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) => thread::sleep(Duration::from_millis(25)),
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(io_error(error));
            }
        }
    };
    if status.success() {
        return Ok(Ok(()));
    }
    use std::io::{Read, Seek, SeekFrom};
    let mut stderr = stderr;
    stderr.seek(SeekFrom::Start(0)).map_err(io_error)?;
    let mut text = String::new();
    stderr.read_to_string(&mut text).map_err(io_error)?;
    Ok(Err(text.trim().to_owned()))
}

pub fn fetch_source(spec: &str, cancel: &AtomicBool) -> Result<FetchedSource, InstallError> {
    if cancel.load(Ordering::Relaxed) {
        return Err(fail("import_cancelled", "Import cancelled"));
    }
    match parse_source_spec(spec)? {
        SourceSpec::Local(dir) => {
            if !dir.is_dir() {
                return Err(fail(
                    "invalid_source",
                    format!("Source folder does not exist: {dir}"),
                ));
            }
            Ok(FetchedSource {
                dir,
                _temporary: None,
            })
        }
        SourceSpec::Git {
            url,
            reference,
            subpath,
        } => {
            let temp = tempfile::tempdir().map_err(io_error)?;
            let root = Utf8Path::from_path(temp.path())
                .ok_or_else(|| fail("invalid_path", "Temporary path is not UTF-8"))?;
            let clone = root.join("source");
            let clone_error = |detail: String| {
                fail("clone_failed", format!("Couldn’t clone repository: {detail}\nCheck that git can reach this repository with your credentials, or clone it yourself and choose the local folder."))
            };
            // Copied https links fail on private repos when git has no https credentials; ssh keys often work.
            if let Err(https) = clone_repo(&url, reference.as_deref(), &clone, cancel)? {
                let Some(ssh) = ssh_url(&url) else {
                    return Err(clone_error(https));
                };
                let _ = fs::remove_dir_all(&clone);
                if let Err(detail) = clone_repo(&ssh, reference.as_deref(), &clone, cancel)? {
                    return Err(clone_error(format!("{https}\nSSH ({ssh}): {detail}")));
                }
            }
            let dir = if subpath.as_str().is_empty() {
                clone
            } else {
                checked_path(&clone, &subpath)?
            };
            if !dir.is_dir() {
                return Err(fail(
                    "invalid_source",
                    format!("Source folder does not exist: {dir}"),
                ));
            }
            Ok(FetchedSource {
                dir,
                _temporary: Some(temp),
            })
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ImportKind {
    Workflow,
    Agent,
    Task,
    Skill,
    Mcp,
}

impl ImportKind {
    fn label(self) -> &'static str {
        match self {
            Self::Workflow => "workflow",
            Self::Agent => "agent",
            Self::Task => "task",
            Self::Skill => "skill",
            Self::Mcp => "mcp",
        }
    }
    fn registry(self) -> String {
        format!("{}s", self.label())
    }
    fn path(self, id: &str) -> Utf8PathBuf {
        let name = if matches!(self, Self::Workflow | Self::Mcp) {
            format!("{id}.yaml")
        } else {
            id.into()
        };
        Utf8PathBuf::from(self.registry()).join(name)
    }
    fn valid(self, id: &str) -> bool {
        id.split('-').all(|part| {
            !part.is_empty()
                && part
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
        }) && crate::project::safe_id(id)
            && crate::skill_document::valid_id(id).is_ok()
    }
}

fn suggestion_base(id: &str) -> String {
    let mut base = id
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|part| !part.is_empty())
        .map(str::to_ascii_lowercase)
        .collect::<Vec<_>>()
        .join("-");
    base.truncate(120);
    base = base.trim_end_matches('-').into();
    if base.is_empty() {
        "item".into()
    } else {
        base
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportItem {
    pub kind: ImportKind,
    pub id: String,
}
impl ImportItem {
    fn key(&self) -> String {
        format!("{}:{}", self.kind.label(), self.id)
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportWorkflow {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportMcp {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportWarning {
    pub kind: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportConflict {
    pub key: String,
    pub kind: ImportKind,
    pub id: String,
    pub suggested: String,
    pub reason: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub caused_by: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportPlan {
    pub source: String,
    pub workflow: ImportWorkflow,
    pub items: Vec<ImportItem>,
    pub reused: Vec<String>,
    pub mcps: Vec<ImportMcp>,
    pub warnings: Vec<ImportWarning>,
    pub conflicts: Vec<ImportConflict>,
}

#[derive(Debug)]
pub enum ImportOutcome {
    Imported {
        workflow: String,
        agents: Vec<AgentExport>,
        warning_count: usize,
    },
    Conflicts(ImportPlan),
}

type Files = BTreeMap<Utf8PathBuf, Vec<u8>>;

fn layout(source: &Utf8Path) -> Result<String, InstallError> {
    let root = if source.join(".unelos").exists() {
        source.join(".unelos")
    } else {
        source.to_owned()
    };
    let path = checked_path(&root, Utf8Path::new("workflows"))?;
    let mut names = Vec::new();
    match fs::read_dir(path) {
        Ok(entries) => {
            for entry in entries {
                let entry = entry.map_err(io_error)?;
                let path = Utf8PathBuf::from_path_buf(entry.path())
                    .map_err(|_| fail("invalid_path", "Workflow path is not UTF-8"))?;
                if path.extension() == Some("yaml") {
                    names.push(path.file_stem().unwrap().to_owned());
                }
            }
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(io_error(e)),
    }
    names.sort();
    if names.len() != 1 {
        let found = match names.as_slice() {
            [] => "none".into(),
            [name] => name.clone(),
            [head @ .., last] => format!("{} and {last}", head.join(", ")),
        };
        return Err(fail("invalid_source", format!("This folder must contain exactly one workflow (found {})\nFound {found}. Point to the subfolder of the one you want.", names.len())));
    }
    Ok(names.remove(0))
}

fn item_for(path: &Utf8Path) -> Option<ImportItem> {
    let parts: Vec<_> = path.iter().collect();
    let kind = match *parts.first()? {
        "workflows" if parts.len() == 2 && path.extension() == Some("yaml") => ImportKind::Workflow,
        "mcps" if parts.len() == 2 && path.extension() == Some("yaml") => ImportKind::Mcp,
        "agents" if parts.len() >= 3 => ImportKind::Agent,
        "tasks" if parts.len() >= 3 => ImportKind::Task,
        "skills" if parts.len() >= 3 => ImportKind::Skill,
        _ => return None,
    };
    let id = if matches!(kind, ImportKind::Workflow | ImportKind::Mcp) {
        path.file_stem()?
    } else {
        parts[1]
    };
    Some(ImportItem {
        kind,
        id: id.into(),
    })
}

fn item_files(files: &Files, item: &ImportItem, name: &str) -> Files {
    let path = item.kind.path(name);
    files
        .iter()
        .filter(|(p, _)| **p == path || p.starts_with(&path))
        .map(|(p, b)| (p.strip_prefix(&path).unwrap().to_owned(), b.clone()))
        .collect()
}

fn existing(root: &Utf8Path, kind: ImportKind, name: &str) -> Result<Option<Files>, InstallError> {
    let relative = kind.path(name);
    let path = checked_path(root, &relative)?;
    if !path.exists() {
        return Ok(None);
    }
    let metadata = fs::metadata(&path).map_err(io_error)?;
    if !metadata.is_file() && !metadata.is_dir() {
        return Err(fail(
            "invalid_path",
            "Import destinations must be ordinary files/directories",
        ));
    }
    let mut files = Files::new();
    if matches!(kind, ImportKind::Workflow | ImportKind::Mcp) {
        files.insert(Utf8PathBuf::new(), fs::read(path).map_err(io_error)?);
    } else if path.is_dir() {
        let mut collected = Files::new();
        collect_files(root, &relative, &mut collected)?;
        for (p, b) in collected {
            files.insert(p.strip_prefix(&relative).unwrap().to_owned(), b);
        }
    } else {
        files.insert(Utf8PathBuf::new(), fs::read(path).map_err(io_error)?);
    }
    Ok(Some(files))
}

fn same(left: &Files, right: &Files) -> bool {
    left.keys().eq(right.keys())
        && left.iter().all(|(p, b)| b.len() == right[p].len())
        && left == right
}

fn text_error(error: impl ToString) -> InstallError {
    fail("invalid_project", error.to_string())
}

pub fn apply_renames(
    files: &Files,
    renames: &BTreeMap<String, String>,
) -> Result<Files, InstallError> {
    let mut result = Files::new();
    for (path, bytes) in files {
        let item = item_for(path);
        let mut destination = path.clone();
        if let Some(item) = &item {
            if let Some(name) = renames.get(&item.key()) {
                if !item.kind.valid(name) {
                    return Err(fail(
                        "invalid_name",
                        "Use lowercase letters, numbers and dashes",
                    ));
                }
                destination = item
                    .kind
                    .path(name)
                    .join(path.strip_prefix(item.kind.path(&item.id)).unwrap());
                if matches!(item.kind, ImportKind::Workflow | ImportKind::Mcp) {
                    destination = item.kind.path(name);
                }
            }
        }
        let is_yaml = matches!(path.file_name(), Some("agent.yaml" | "task.yaml"))
            || item
                .as_ref()
                .is_some_and(|i| i.kind == ImportKind::Workflow);
        let is_skill = path.file_name() == Some("SKILL.md");
        let mut output = bytes.clone();
        if is_yaml || is_skill {
            let mut text = String::from_utf8(bytes.clone()).map_err(text_error)?;
            let (prefix, yaml, suffix) = if is_skill {
                let start = if text.starts_with("---\r\n") {
                    5
                } else if text.starts_with("---\n") {
                    4
                } else {
                    0
                };
                if start == 0 {
                    (String::new(), String::new(), text.clone())
                } else {
                    let mut offset = start;
                    let mut end = None;
                    for line in text[start..].split_inclusive('\n') {
                        if line.trim_end_matches(['\r', '\n']) == "---" {
                            end = Some((offset, offset + line.len()));
                            break;
                        }
                        offset += line.len();
                    }
                    let (end, body) =
                        end.ok_or_else(|| text_error("Unclosed Skill frontmatter"))?;
                    (
                        text[..start].into(),
                        text[start..end].into(),
                        text[end..body].to_owned() + &text[body..],
                    )
                }
            } else {
                (String::new(), text.clone(), String::new())
            };
            let mut edits = Vec::new();
            for (key, name) in renames {
                let Some((kind, old)) = key.split_once(':') else {
                    continue;
                };
                if old == name || old.starts_with("global/") {
                    continue;
                }
                let fields: &[&str] = match kind {
                    "skill" if !is_skill => &["skills"],
                    "mcp" if path.file_name() == Some("agent.yaml") => &["mcps"],
                    "agent"
                        if item
                            .as_ref()
                            .is_some_and(|i| i.kind == ImportKind::Workflow) =>
                    {
                        &["agent"]
                    }
                    "task"
                        if item
                            .as_ref()
                            .is_some_and(|i| i.kind == ImportKind::Workflow) =>
                    {
                        &["task"]
                    }
                    _ => &[],
                };
                for field in fields {
                    edits.push((old, name.as_str(), *field));
                }
                if item.as_ref().is_some_and(|i| i.key() == *key)
                    && (is_skill
                        || item
                            .as_ref()
                            .is_some_and(|i| i.kind == ImportKind::Workflow))
                {
                    edits.push((old, name.as_str(), "name"));
                }
            }
            // Simultaneous renames must not rewrite another item's replacement.
            let mut rewritten = yaml;
            for (index, (old, _, field)) in edits.iter().enumerate() {
                rewritten = crate::skill_document::reference_field_edit(
                    &rewritten,
                    old,
                    &format!("__unelos_import_{index}__"),
                    field,
                )
                .map_err(text_error)?;
            }
            for (index, (_, name, field)) in edits.iter().enumerate() {
                rewritten = crate::skill_document::reference_field_edit(
                    &rewritten,
                    &format!("__unelos_import_{index}__"),
                    name,
                    field,
                )
                .map_err(text_error)?;
            }
            text = prefix + &rewritten + &suffix;
            output = text.into_bytes();
        }
        if result.insert(destination.clone(), output).is_some() {
            return Err(fail(
                "import_collision",
                format!("Duplicate import destination: {destination}"),
            ));
        }
    }
    Ok(result)
}

fn load_source(
    source: &Utf8Path,
) -> Result<(LoadedProject, Files, Vec<ImportWarning>), InstallError> {
    let workflow = layout(source)?;
    let mut project = crate::install::load_import_source(source, &workflow)?;
    let mut warnings = Vec::new();
    let (_, agent_ids) = references(&project);
    for id in agent_ids.keys().filter(|id| id.starts_with("global/")) {
        if !project.definitions.agents.contains_key(id) {
            let agent = crate::parse_agent_definition(id, "instructions: External global agent\n")
                .map_err(text_error)?;
            project.definitions.agents.insert(id.clone(), agent);
        }
    }
    // Missing global tasks are external dependencies, not invalid local definitions.
    for node in project.workflow.nodes.values() {
        let selections: Vec<_> = match node {
            crate::NodeDefinition::Task(n) => vec![(
                &n.task,
                match &n.next {
                    crate::Transition::Outcomes(m) => m.keys().cloned().collect(),
                    _ => Vec::new(),
                },
            )],
            crate::NodeDefinition::Parallel(n) => {
                n.tasks.iter().map(|w| (&w.task, Vec::new())).collect()
            }
            _ => Vec::new(),
        };
        for (selection, outcomes) in selections {
            if let crate::TaskSelection::Named(id) = selection {
                if id.starts_with("global/")
                    && !project.definitions.task_definitions.contains_key(id)
                {
                    project.definitions.task_definitions.insert(
                        id.clone(),
                        crate::TaskDefinition {
                            instructions: "External global task".into(),
                            instructions_file: None,
                            description: None,
                            skills: Vec::new(),
                            outcomes,
                        },
                    );
                }
            }
        }
    }
    crate::validate_project(&project).map_err(text_error)?;
    let files = collect_import_files(&project, &workflow)?;
    let (tasks, agents) = references(&project);
    let global = crate::task_document::global_root().ok();
    let mut globals = BTreeSet::new();
    for (path, workflow, _) in &project.child_workflows {
        if global.as_ref().is_some_and(|root| path.starts_with(root)) {
            globals.insert(("workflow", format!("global/{}", workflow.name)));
        }
    }
    for id in tasks.iter().filter(|id| id.starts_with("global/")) {
        globals.insert(("task", id.clone()));
    }
    for id in agents.keys() {
        if id.starts_with("global/") {
            globals.insert(("agent", id.clone()));
        }
        let agent = &project.definitions.agents[id];
        if let Some(crate::AgentModel::Alias(alias)) = &agent.model {
            if crate::list_model_mappings()?
                .get(alias)
                .is_none_or(|models| models.is_empty())
            {
                warnings.push(ImportWarning {
                    kind: "unresolvedModel".into(),
                    message: format!("Agent {id}: No harness model mapped for alias '{alias}'"),
                });
            }
        }
        for skill in &agent.skills {
            if skill.starts_with("global/") {
                globals.insert(("skill", skill.clone()));
            }
        }
        for mcp in &agent.mcps {
            if mcp.starts_with("global/") {
                globals.insert(("mcp", mcp.clone()));
            }
        }
    }
    for node in project.workflow.nodes.values() {
        let selections: Vec<_> = match node {
            crate::NodeDefinition::Task(n) => vec![&n.task],
            crate::NodeDefinition::Parallel(n) => n.tasks.iter().map(|w| &w.task).collect(),
            _ => Vec::new(),
        };
        for selection in selections {
            let (task, _) = project
                .selected_task(selection, "task")
                .map_err(text_error)?;
            for skill in &task.skills {
                if skill.starts_with("global/") {
                    globals.insert(("skill", skill.clone()));
                }
            }
        }
    }
    for (kind, id) in globals {
        let bare = id.strip_prefix("global/").unwrap();
        let path = match kind {
            "workflow" => format!("workflows/{bare}.yaml"),
            "agent" => format!("agents/{bare}/agent.yaml"),
            "task" => format!("tasks/{bare}/task.yaml"),
            "skill" => format!("skills/{bare}/SKILL.md"),
            _ => format!("mcps/{bare}.yaml"),
        };
        if !global
            .as_ref()
            .is_some_and(|root| root.join(path).is_file())
        {
            warnings.push(ImportWarning {
                kind: "missingGlobal".into(),
                message: format!("Global {kind} {id} is not installed"),
            });
        }
    }
    Ok((project, files, warnings))
}

struct Prepared {
    plan: ImportPlan,
    files: Files,
    names: BTreeMap<String, String>,
    project: LoadedProject,
}

fn prepare(
    root: &Utf8Path,
    source: &Utf8Path,
    display: &str,
    renames: &BTreeMap<String, String>,
) -> Result<Prepared, InstallError> {
    crate::check_project_init(root)?;
    let (project, files, warnings) = load_source(source)?;
    let mut unique = BTreeMap::new();
    for path in files.keys() {
        if let Some(item) = item_for(path) {
            unique.insert(item.key(), item);
        }
    }
    let items: Vec<_> = unique.into_values().collect();
    let authoring = root.join(".unelos");
    for (path, bytes) in &files {
        if item_for(path).is_none() {
            let dest = checked_path(&authoring, path)?;
            if dest.exists() && !dest.is_file() {
                return Err(fail(
                    "invalid_path",
                    format!("Import destination is not an ordinary file: {path}"),
                ));
            }
            match fs::read(&dest) {
                Ok(existing) if existing != *bytes => {
                    return Err(fail(
                        "import_collision",
                        format!("Existing authoring differs: {path}"),
                    ))
                }
                Ok(_) => {}
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => return Err(io_error(e)),
            }
        }
    }
    let mut conflicts = BTreeMap::<String, ImportConflict>::new();
    let mut names = renames.clone();
    let mut rewritten = files.clone();
    loop {
        let mut added = false;
        for item in &items {
            let key = item.key();
            let original = existing(&authoring, item.kind, &item.id)?;
            let was_different = original
                .as_ref()
                .is_some_and(|old| !same(old, &item_files(&files, item, &item.id)));
            let final_name = names.get(&key).unwrap_or(&item.id);
            let now_different = final_name == &item.id
                && original
                    .as_ref()
                    .is_some_and(|old| !same(old, &item_files(&rewritten, item, final_name)));
            if !conflicts.contains_key(&key)
                && (was_different || now_different || renames.contains_key(&key))
            {
                let mut n = 2;
                let suggested = loop {
                    let candidate = format!("{}-{n}", suggestion_base(&item.id));
                    let occupied = checked_path(&authoring, &item.kind.path(&candidate))?.exists()
                        || items
                            .iter()
                            .any(|i| i.kind == item.kind && i.id == candidate)
                        || names.iter().any(|(k, v)| {
                            k.starts_with(&format!("{}:", item.kind.label())) && *v == candidate
                        })
                        || conflicts
                            .values()
                            .any(|c| c.kind == item.kind && c.suggested == candidate);
                    if !occupied {
                        break candidate;
                    }
                    n += 1;
                };
                let caused_by = if !was_different && now_different {
                    names.iter().find_map(|(cause, name)| {
                        if cause == &key {
                            return None;
                        }
                        let single = BTreeMap::from([(cause.clone(), name.clone())]);
                        apply_renames(&files, &single)
                            .ok()
                            .filter(|edited| {
                                item_files(edited, item, &item.id)
                                    != item_files(&files, item, &item.id)
                            })
                            .map(|_| cause.clone())
                    })
                } else {
                    None
                };
                conflicts.insert(
                    key.clone(),
                    ImportConflict {
                        key: key.clone(),
                        kind: item.kind,
                        id: item.id.clone(),
                        suggested: suggested.clone(),
                        reason: if was_different {
                            "exists"
                        } else {
                            "referencesRenamed"
                        }
                        .into(),
                        caused_by,
                        error: None,
                    },
                );
                names.entry(key).or_insert(suggested);
                added = true;
            }
        }
        let mut valid_names = names.clone();
        for item in &items {
            if let Some(name) = valid_names.get(&item.key()) {
                if !item.kind.valid(name) {
                    valid_names.remove(&item.key());
                }
            }
        }
        // Invalid duplicate destinations still need a plan with errors for both rows.
        let targets: Vec<_> = items
            .iter()
            .map(|item| {
                (
                    item.kind,
                    item.key(),
                    valid_names.get(&item.key()).unwrap_or(&item.id).clone(),
                )
            })
            .collect();
        for (index, (kind, key, name)) in targets.iter().enumerate() {
            if targets.iter().any(|(other_kind, other_key, other_name)| {
                kind == other_kind && key != other_key && name == other_name
            }) {
                valid_names.insert(key.clone(), format!("unelos-invalid-{index}"));
            }
        }
        rewritten = apply_renames(&files, &valid_names)?;
        if !added {
            break;
        }
    }
    for conflict in conflicts.values_mut() {
        let name = &names[&conflict.key];
        conflict.error = if name.is_empty() {
            Some("Enter a name".into())
        } else if !conflict.kind.valid(name) {
            Some("Use lowercase letters, numbers and dashes".into())
        } else if items.iter().any(|i| {
            i.kind == conflict.kind
                && i.key() != conflict.key
                && names.get(&i.key()).unwrap_or(&i.id) == name
        }) {
            Some(format!("Two items can’t both be named “{name}”"))
        } else if let Some(old) = existing(&authoring, conflict.kind, name)? {
            if same(
                &old,
                &item_files(
                    &rewritten,
                    &ImportItem {
                        kind: conflict.kind,
                        id: conflict.id.clone(),
                    },
                    name,
                ),
            ) {
                None
            } else {
                Some(format!(
                    "A {} named “{name}” already exists",
                    conflict.kind.label()
                ))
            }
        } else {
            None
        };
    }
    let mut reused = Vec::new();
    for item in &items {
        let name = names.get(&item.key()).unwrap_or(&item.id);
        if (!names.contains_key(&item.key()) || item.kind.valid(name))
            && existing(&authoring, item.kind, name)?
                .is_some_and(|old| same(&old, &item_files(&rewritten, item, name)))
        {
            reused.push(item.key());
        }
    }
    let mut mcps = Vec::new();
    for item in items.iter().filter(|i| i.kind == ImportKind::Mcp) {
        let document =
            crate::mcp_document::read_mcp(&project.authoring_root, &item.id).map_err(text_error)?;
        let (command, url) = match document.server {
            McpServer::Local { command, .. } => (Some(command.join(" ")), None),
            McpServer::Remote { url, .. } => (None, Some(url)),
        };
        mcps.push(ImportMcp {
            id: item.id.clone(),
            command,
            url,
        });
    }
    let plan = ImportPlan {
        source: display.into(),
        workflow: ImportWorkflow {
            id: project.workflow.name.clone(),
            description: project.workflow.description.clone(),
        },
        items,
        reused,
        mcps,
        warnings,
        conflicts: conflicts.into_values().collect(),
    };
    Ok(Prepared {
        plan,
        files: rewritten,
        names,
        project,
    })
}

pub fn plan_import(
    project_root: &Utf8Path,
    source_dir: &Utf8Path,
    source_display: &str,
    renames: &BTreeMap<String, String>,
) -> Result<ImportPlan, InstallError> {
    Ok(prepare(project_root, source_dir, source_display, renames)?.plan)
}

struct Staging {
    path: Utf8PathBuf,
    new_authoring: Option<Utf8PathBuf>,
}
impl Drop for Staging {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
        if let Some(path) = &self.new_authoring {
            let _ = fs::remove_dir_all(path);
        }
    }
}

pub fn commit_import(
    project_root: &Utf8Path,
    source_dir: &Utf8Path,
    source_display: &str,
    renames: &BTreeMap<String, String>,
) -> Result<ImportOutcome, InstallError> {
    let prepared = prepare(project_root, source_dir, source_display, renames)?;
    if prepared.plan.conflicts.iter().any(|c| c.error.is_some()) {
        return Ok(ImportOutcome::Conflicts(prepared.plan));
    }
    let authoring = project_root.join(".unelos");
    let had_authoring = authoring.exists();
    let staging_root = checked_path(project_root, Utf8Path::new(".unelos/.import-staging"))?;
    if staging_root.exists() {
        fs::remove_dir_all(&staging_root).map_err(io_error)?;
    }
    let mut cleanup = Staging {
        path: staging_root.clone(),
        new_authoring: (!had_authoring).then(|| authoring.clone()),
    };
    fs::create_dir_all(&staging_root).map_err(io_error)?;
    let temporary = tempfile::Builder::new()
        .prefix("import-")
        .tempdir_in(&staging_root)
        .map_err(io_error)?;
    let staging = Utf8Path::from_path(temporary.path())
        .ok_or_else(|| fail("invalid_path", "Temporary path is not UTF-8"))?;
    let mut destinations = BTreeSet::new();
    for item in &prepared.plan.items {
        if !prepared.plan.reused.contains(&item.key()) {
            let name = prepared.names.get(&item.key()).unwrap_or(&item.id);
            destinations.insert(item.kind.path(name));
        }
    }
    for path in prepared.files.keys().filter(|p| item_for(p).is_none()) {
        if !checked_path(&authoring, path)?.exists() {
            destinations.insert(path.clone());
        }
    }
    for (path, bytes) in &prepared.files {
        if destinations
            .iter()
            .any(|d| path == d || path.starts_with(d))
        {
            crate::write_install_file(staging, path, bytes)?;
        }
    }
    let mut moved = Vec::new();
    let mut created_dirs = BTreeSet::new();
    let had_settings = authoring.join("project.yaml").exists();
    let result = (|| {
        for relative in destinations {
            let destination = checked_path(&authoring, &relative)?;
            if destination.exists() {
                return Err(fail(
                    "import_collision",
                    format!("Existing authoring differs: {relative}"),
                ));
            }
            let mut parent = destination.parent().unwrap();
            while parent.starts_with(&authoring) && !parent.exists() {
                created_dirs.insert(parent.to_owned());
                let Some(next) = parent.parent() else { break };
                parent = next;
            }
            fs::create_dir_all(destination.parent().unwrap()).map_err(io_error)?;
            fs::rename(staging.join(&relative), &destination).map_err(io_error)?;
            moved.push(destination);
        }
        for name in ["workflows", "skills", "tasks", "agents"] {
            let path = authoring.join(name);
            if !path.exists() {
                created_dirs.insert(path);
            }
        }
        crate::initialize_project(project_root)
    })();
    if let Err(error) = result {
        for path in moved.iter().rev() {
            if path.is_dir() {
                let _ = fs::remove_dir_all(path);
            } else {
                let _ = fs::remove_file(path);
            }
        }
        if !had_settings {
            let _ = fs::remove_file(authoring.join("project.yaml"));
        }
        for path in created_dirs.iter().rev() {
            let _ = fs::remove_dir(path);
        }
        if !had_authoring {
            let _ = fs::remove_dir_all(&authoring);
        }
        return Err(error);
    }
    drop(temporary);
    cleanup.new_authoring = None;
    drop(cleanup);
    let (_, agents) = references(&prepared.project);
    let mut exports = Vec::new();
    for (id, subagent) in agents {
        let agent = &prepared.project.definitions.agents[&id];
        let mut mcp_servers = BTreeMap::new();
        let mcps: Vec<_> = agent
            .mcps
            .iter()
            .filter(|_| subagent)
            .map(|mcp| {
                prepared
                    .names
                    .get(&format!("mcp:{mcp}"))
                    .unwrap_or(mcp)
                    .clone()
            })
            .collect();
        for mcp in &mcps {
            let doc = if let Some(id) = mcp.strip_prefix("global/") {
                crate::get_global_mcp(id)
            } else {
                crate::mcp_document::read_mcp(&authoring, mcp)
            };
            if let Ok(doc) = doc {
                mcp_servers.insert(
                    mcp.strip_prefix("global/").unwrap_or(mcp).into(),
                    doc.server,
                );
            }
        }
        exports.push(AgentExport {
            id: prepared
                .names
                .get(&format!("agent:{id}"))
                .unwrap_or(&id)
                .clone(),
            instructions: agent.instructions.clone(),
            model: agent.model.clone(),
            effort: agent.effort.clone(),
            tools: agent.tools.clone(),
            mcps,
            mcp_servers,
            subagent,
        });
    }
    Ok(ImportOutcome::Imported {
        workflow: prepared
            .names
            .get(&format!("workflow:{}", prepared.plan.workflow.id))
            .unwrap_or(&prepared.plan.workflow.id)
            .clone(),
        agents: exports,
        warning_count: prepared.plan.warnings.len(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_specs() {
        assert_eq!(
            parse_source_spec("/some/folder").unwrap(),
            SourceSpec::Local("/some/folder".into())
        );
        for (suffix, reference, subpath) in [
            (
                "#main:workflows/research",
                Some("main"),
                "workflows/research",
            ),
            ("#main", Some("main"), ""),
            ("#:sub", None, "sub"),
            ("", None, ""),
        ] {
            assert_eq!(
                parse_source_spec(&format!("https://github.com/org/repo{suffix}")).unwrap(),
                SourceSpec::Git {
                    url: "https://github.com/org/repo".into(),
                    reference: reference.map(str::to_owned),
                    subpath: subpath.into(),
                }
            );
        }
        for (link, url) in [
            (
                "https://github.com/dieguit/unelos-workflows/tree/main/examples/superpowers-dev/",
                "https://github.com/dieguit/unelos-workflows",
            ),
            (
                "https://gitlab.com/org/repo/-/tree/main/examples/superpowers-dev?ref_type=heads",
                "https://gitlab.com/org/repo",
            ),
        ] {
            assert_eq!(
                parse_source_spec(link).unwrap(),
                SourceSpec::Git {
                    url: url.into(),
                    reference: Some("main".into()),
                    subpath: "examples/superpowers-dev".into()
                }
            );
        }
        assert_eq!(
            ssh_url("https://github.com/org/repo").as_deref(),
            Some("git@github.com:org/repo.git")
        );
        assert_eq!(
            ssh_url("https://gitlab.com/g/sub/repo.git/").as_deref(),
            Some("git@gitlab.com:g/sub/repo.git")
        );
        assert_eq!(ssh_url("https://user@host/repo"), None);
        assert_eq!(ssh_url("git@github.com:org/repo.git"), None);
        for suffix in ["#:../secret", "#:/absolute", "#:a/../secret"] {
            assert!(parse_source_spec(&format!("git@github.com:org/repo{suffix}")).is_err());
        }
        assert!(parse_source_spec("").is_err());
        assert!(
            matches!(parse_source_spec("~/source").unwrap(), SourceSpec::Local(path) if !path.starts_with("~"))
        );
    }
}
