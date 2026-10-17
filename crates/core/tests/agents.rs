use assert_fs::TempDir;
use camino::{Utf8Path, Utf8PathBuf};
use std::fs;
use unelos_core::{agent_summaries as summaries, AgentSummary};

struct Problem {
    file: String,
    message: String,
}

/// Valid rows vs problem rows, with the problem's definition file for path assertions.
struct AgentList {
    agents: Vec<AgentSummary>,
    problems: Vec<Problem>,
}

fn agent_summaries(root: &Utf8Path) -> Result<AgentList, unelos_core::ProjectLoadError> {
    let (problems, agents): (Vec<_>, Vec<_>) =
        summaries(root)?.into_iter().partition(|a| a.problem.is_some());
    Ok(AgentList {
        agents,
        problems: problems
            .into_iter()
            .map(|a| Problem {
                file: format!(".unelos/agents/{}/agent.yaml", a.name),
                message: a.problem.unwrap(),
            })
            .collect(),
    })
}

fn write(root: &Utf8Path, relative: &str, text: &str) {
    let path = root.join(relative);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

fn project() -> (TempDir, Utf8PathBuf) {
    let temp = TempDir::new().unwrap();
    let root = Utf8PathBuf::from_path_buf(temp.path().to_owned()).unwrap();
    write(&root, ".unelos/project.yaml", "{}\n");
    (temp, root)
}

#[test]
fn summaries_isolate_bad_agents_and_preserve_metadata() {
    let temp = TempDir::new().unwrap();
    let root = Utf8PathBuf::from_path_buf(temp.path().to_owned()).unwrap();
    write(&root, ".unelos/project.yaml", "{}\n");
    write(
        &root,
        ".unelos/agents/zeta/agent.yaml",
        "instructions: Work\n",
    );
    write(
        &root,
        ".unelos/agents/engineer/agent.yaml",
        "instructions: Build\nmodel: provider/model\nskills: [rust]\nmcps: [github]\n",
    );
    write(
        &root,
        ".unelos/agents/broken/agent.yaml",
        "instructions: [\n",
    );
    let result = agent_summaries(&root).unwrap();
    assert_eq!(
        result
            .agents
            .iter()
            .map(|a| a.name.as_str())
            .collect::<Vec<_>>(),
        ["engineer", "zeta"]
    );
    assert_eq!(result.agents[0].model.as_deref(), Some("provider/model"));
    assert_eq!(result.agents[0].skills, ["rust"]);
    assert_eq!(result.agents[0].mcps, ["github"]);
    assert!(result.agents[1].model.is_none());
    assert_eq!(result.problems.len(), 1);
    assert!(result.problems[0]
        .file
        .ends_with("agents/broken/agent.yaml"));
    let broken = summaries(&root).unwrap().remove(0);
    assert_eq!(broken.name, "broken");
    assert!(broken.problem.is_some());
    assert!(broken.model.is_none() && broken.skills.is_empty() && broken.mcps.is_empty());
}

#[test]
fn missing_or_empty_agents_directory_is_empty() {
    let (_missing, missing) = project();
    let result = agent_summaries(&missing).unwrap();
    assert!(result.agents.is_empty());
    assert!(result.problems.is_empty());

    let (_empty, empty) = project();
    fs::create_dir_all(empty.join(".unelos/agents")).unwrap();
    let result = agent_summaries(&empty).unwrap();
    assert!(result.agents.is_empty());
    assert!(result.problems.is_empty());
}

#[test]
fn agents_path_that_is_not_a_directory_is_an_error() {
    let (_temp, root) = project();
    write(&root, ".unelos/agents", "not a directory\n");
    assert!(agent_summaries(&root).is_err());
}

#[test]
fn all_invalid_definitions_yield_no_agents_and_diagnostics() {
    let (_temp, root) = project();
    write(&root, ".unelos/agents/one/agent.yaml", "instructions: [\n");
    write(&root, ".unelos/agents/two/agent.yaml", "model: nope\n");
    let result = agent_summaries(&root).unwrap();
    assert!(result.agents.is_empty());
    assert_eq!(result.problems.len(), 2);
}

#[test]
fn missing_agent_file_is_isolated() {
    let (_temp, root) = project();
    write(
        &root,
        ".unelos/agents/valid/agent.yaml",
        "instructions: Work\n",
    );
    fs::create_dir_all(root.join(".unelos/agents/missing")).unwrap();
    let result = agent_summaries(&root).unwrap();
    assert_eq!(
        result
            .agents
            .iter()
            .map(|a| a.name.as_str())
            .collect::<Vec<_>>(),
        ["valid"]
    );
    assert_eq!(result.problems.len(), 1);
    assert!(result.problems[0]
        .file
        .ends_with("agents/missing/agent.yaml"));
}

#[test]
fn invalid_directory_id_is_isolated() {
    let (_temp, root) = project();
    write(
        &root,
        ".unelos/agents/valid/agent.yaml",
        "instructions: Work\n",
    );
    write(
        &root,
        ".unelos/agents/bad.id/agent.yaml",
        "instructions: Work\n",
    );
    let result = agent_summaries(&root).unwrap();
    assert_eq!(
        result
            .agents
            .iter()
            .map(|a| a.name.as_str())
            .collect::<Vec<_>>(),
        ["valid"]
    );
    assert_eq!(result.problems.len(), 1);
    assert!(result.problems[0]
        .file
        .ends_with("agents/bad.id/agent.yaml"));
}

#[test]
fn missing_instructions_file_is_isolated() {
    let (_temp, root) = project();
    write(
        &root,
        ".unelos/agents/valid/agent.yaml",
        "instructions: Work\n",
    );
    write(
        &root,
        ".unelos/agents/broken/agent.yaml",
        "instructionsFile: missing.md\n",
    );
    let result = agent_summaries(&root).unwrap();
    assert_eq!(
        result
            .agents
            .iter()
            .map(|a| a.name.as_str())
            .collect::<Vec<_>>(),
        ["valid"]
    );
    assert_eq!(result.problems.len(), 1);
    assert!(result.problems[0]
        .file
        .ends_with("agents/broken/agent.yaml"));
}

#[test]
fn valid_inline_and_file_instructions_are_accepted() {
    let (_temp, root) = project();
    write(
        &root,
        ".unelos/agents/inline/agent.yaml",
        "instructions: Inline\n",
    );
    write(
        &root,
        ".unelos/agents/file/agent.yaml",
        "instructionsFile: prompt.md\n",
    );
    write(&root, ".unelos/agents/file/prompt.md", "Prompt\n");
    let result = agent_summaries(&root).unwrap();
    assert_eq!(
        result
            .agents
            .iter()
            .map(|a| a.name.as_str())
            .collect::<Vec<_>>(),
        ["file", "inline"]
    );
    assert!(result.problems.is_empty());
}

#[test]
fn unrelated_broken_workflow_and_task_are_ignored() {
    let (_temp, root) = project();
    write(
        &root,
        ".unelos/agents/valid/agent.yaml",
        "instructions: Work\n",
    );
    write(&root, ".unelos/workflows/broken.yaml", "name: [\n");
    write(&root, ".unelos/tasks/broken/task.yaml", "instructions: [\n");
    let result = agent_summaries(&root).unwrap();
    assert_eq!(
        result
            .agents
            .iter()
            .map(|a| a.name.as_str())
            .collect::<Vec<_>>(),
        ["valid"]
    );
    assert!(result.problems.is_empty());
}

#[test]
fn ordinary_files_in_agents_directory_are_ignored() {
    let (_temp, root) = project();
    write(
        &root,
        ".unelos/agents/valid/agent.yaml",
        "instructions: Work\n",
    );
    write(&root, ".unelos/agents/README.md", "notes\n");
    let result = agent_summaries(&root).unwrap();
    assert_eq!(
        result
            .agents
            .iter()
            .map(|a| a.name.as_str())
            .collect::<Vec<_>>(),
        ["valid"]
    );
    assert!(result.problems.is_empty());
}

#[cfg(unix)]
#[test]
fn invalid_utf8_child_is_isolated() {
    use std::{ffi::OsStr, os::unix::ffi::OsStrExt};
    let (_temp, root) = project();
    write(
        &root,
        ".unelos/agents/valid/agent.yaml",
        "instructions: Work\n",
    );
    fs::create_dir_all(
        root.join(".unelos/agents")
            .as_std_path()
            .join(OsStr::from_bytes(b"bad\xff")),
    )
    .unwrap();
    let result = agent_summaries(&root).unwrap();
    assert_eq!(
        result
            .agents
            .iter()
            .map(|a| a.name.as_str())
            .collect::<Vec<_>>(),
        ["valid"]
    );
    assert_eq!(result.problems.len(), 1);
    assert!(result.problems[0].file.ends_with("/agent.yaml"));
}

#[cfg(unix)]
#[test]
fn symlinked_agent_directory_is_isolated_and_not_read() {
    let (_temp, root) = project();
    let outside = TempDir::new().unwrap();
    let outside_root = Utf8PathBuf::from_path_buf(outside.path().to_owned()).unwrap();
    write(
        &outside_root,
        "agent/agent.yaml",
        "instructions: Leak\nmodel: leaked/model\n",
    );
    write(
        &root,
        ".unelos/agents/valid/agent.yaml",
        "instructions: Work\n",
    );
    std::os::unix::fs::symlink(
        outside_root.join("agent"),
        root.join(".unelos/agents/linked"),
    )
    .unwrap();
    let result = agent_summaries(&root).unwrap();
    assert_eq!(
        result
            .agents
            .iter()
            .map(|a| a.name.as_str())
            .collect::<Vec<_>>(),
        ["valid"]
    );
    assert_eq!(result.problems.len(), 1);
    assert!(result.problems[0]
        .file
        .ends_with("agents/linked/agent.yaml"));
}

#[cfg(unix)]
#[test]
fn symlinked_agent_file_is_isolated_and_not_read() {
    let (_temp, root) = project();
    let outside = TempDir::new().unwrap();
    let outside_root = Utf8PathBuf::from_path_buf(outside.path().to_owned()).unwrap();
    write(
        &outside_root,
        "agent.yaml",
        "instructions: Leak\nmodel: leaked/model\n",
    );
    write(
        &root,
        ".unelos/agents/valid/agent.yaml",
        "instructions: Work\n",
    );
    fs::create_dir_all(root.join(".unelos/agents/linked")).unwrap();
    std::os::unix::fs::symlink(
        outside_root.join("agent.yaml"),
        root.join(".unelos/agents/linked/agent.yaml"),
    )
    .unwrap();
    let result = agent_summaries(&root).unwrap();
    assert_eq!(
        result
            .agents
            .iter()
            .map(|a| a.name.as_str())
            .collect::<Vec<_>>(),
        ["valid"]
    );
    assert_eq!(result.problems.len(), 1);
    assert!(result.problems[0]
        .file
        .ends_with("agents/linked/agent.yaml"));
}

#[cfg(unix)]
#[test]
fn symlinked_instructions_file_is_isolated_and_not_read() {
    let (_temp, root) = project();
    let outside = TempDir::new().unwrap();
    let outside_root = Utf8PathBuf::from_path_buf(outside.path().to_owned()).unwrap();
    write(&outside_root, "prompt.md", "Leak\n");
    write(
        &root,
        ".unelos/agents/valid/agent.yaml",
        "instructions: Work\n",
    );
    write(
        &root,
        ".unelos/agents/linked/agent.yaml",
        "instructionsFile: prompt.md\n",
    );
    std::os::unix::fs::symlink(
        outside_root.join("prompt.md"),
        root.join(".unelos/agents/linked/prompt.md"),
    )
    .unwrap();
    let result = agent_summaries(&root).unwrap();
    assert_eq!(
        result
            .agents
            .iter()
            .map(|a| a.name.as_str())
            .collect::<Vec<_>>(),
        ["valid"]
    );
    assert_eq!(result.problems.len(), 1);
    assert!(result.problems[0]
        .file
        .ends_with("agents/linked/agent.yaml"));
}

#[cfg(unix)]
#[test]
fn symlinked_authoring_root_is_an_error() {
    let temp = TempDir::new().unwrap();
    let root = Utf8PathBuf::from_path_buf(temp.path().to_owned()).unwrap();
    let outside = TempDir::new().unwrap();
    let outside_root = Utf8PathBuf::from_path_buf(outside.path().to_owned()).unwrap();
    write(&outside_root, "project.yaml", "{}\n");
    write(
        &outside_root,
        "agents/valid/agent.yaml",
        "instructions: Work\n",
    );
    std::os::unix::fs::symlink(&outside_root, root.join(".unelos")).unwrap();
    assert!(agent_summaries(&root).is_err());
}

#[cfg(unix)]
#[test]
fn symlinked_agents_directory_is_an_error() {
    let (_temp, root) = project();
    let outside = TempDir::new().unwrap();
    let outside_root = Utf8PathBuf::from_path_buf(outside.path().to_owned()).unwrap();
    write(
        &outside_root,
        "agents/valid/agent.yaml",
        "instructions: Work\n",
    );
    std::os::unix::fs::symlink(outside_root.join("agents"), root.join(".unelos/agents")).unwrap();
    assert!(agent_summaries(&root).is_err());
}

#[cfg(unix)]
fn mkfifo(path: &Utf8Path) {
    let status = std::process::Command::new("mkfifo")
        .arg(path)
        .status()
        .unwrap();
    assert!(status.success());
}

// Opening a FIFO without a writer blocks forever, so scan on a thread with a deadline.
#[cfg(unix)]
fn summaries_before_deadline(root: &Utf8Path) -> AgentList {
    let (sender, receiver) = std::sync::mpsc::channel();
    let root = root.to_owned();
    std::thread::spawn(move || sender.send(agent_summaries(&root)).unwrap());
    receiver
        .recv_timeout(std::time::Duration::from_secs(10))
        .expect("agent_summaries blocked on a non-regular file")
        .unwrap()
}

#[cfg(unix)]
#[test]
fn fifo_agent_definition_is_isolated_without_blocking() {
    let (_temp, root) = project();
    write(
        &root,
        ".unelos/agents/valid/agent.yaml",
        "instructions: Work\n",
    );
    fs::create_dir_all(root.join(".unelos/agents/pipe")).unwrap();
    mkfifo(&root.join(".unelos/agents/pipe/agent.yaml"));
    let result = summaries_before_deadline(&root);
    assert_eq!(
        result
            .agents
            .iter()
            .map(|a| a.name.as_str())
            .collect::<Vec<_>>(),
        ["valid"]
    );
    assert_eq!(result.problems.len(), 1);
    assert!(result.problems[0].file.ends_with("agents/pipe/agent.yaml"));
}

#[cfg(unix)]
#[test]
fn fifo_instructions_file_is_isolated_without_blocking() {
    let (_temp, root) = project();
    write(
        &root,
        ".unelos/agents/valid/agent.yaml",
        "instructions: Work\n",
    );
    write(
        &root,
        ".unelos/agents/pipe/agent.yaml",
        "instructionsFile: prompt.md\n",
    );
    mkfifo(&root.join(".unelos/agents/pipe/prompt.md"));
    let result = summaries_before_deadline(&root);
    assert_eq!(
        result
            .agents
            .iter()
            .map(|a| a.name.as_str())
            .collect::<Vec<_>>(),
        ["valid"]
    );
    assert_eq!(result.problems.len(), 1);
    assert!(result.problems[0].file.ends_with("agents/pipe/agent.yaml"));
}

#[test]
fn raw_instructions_escapes_are_isolated() {
    let (_temp, root) = project();
    let outside = TempDir::new().unwrap();
    let outside_root = Utf8PathBuf::from_path_buf(outside.path().to_owned()).unwrap();
    write(&outside_root, "prompt.md", "Leak\n");
    write(&root, "escape.md", "Leak\n");
    write(
        &root,
        ".unelos/agents/valid/agent.yaml",
        "instructions: Work\n",
    );
    let cases = [
        (
            "absolute",
            format!("instructionsFile: {}\n", outside_root.join("prompt.md")),
            "expected a relative instructionsFile path",
        ),
        (
            "backslash",
            "instructionsFile: '..\\escape.md'\n".to_owned(),
            "expected a relative instructionsFile path",
        ),
        (
            "parent",
            "instructionsFile: ../../../escape.md\n".to_owned(),
            "path escapes authoring root",
        ),
        (
            "reserved",
            "instructionsFile: ../../project.yaml\n".to_owned(),
            "must reference authoring",
        ),
    ];
    for (name, yaml, _) in &cases {
        write(&root, &format!(".unelos/agents/{name}/agent.yaml"), yaml);
    }
    let result = agent_summaries(&root).unwrap();
    assert_eq!(
        result
            .agents
            .iter()
            .map(|a| a.name.as_str())
            .collect::<Vec<_>>(),
        ["valid"]
    );
    assert_eq!(result.problems.len(), cases.len());
    for (name, _, fragment) in cases {
        let problem = result
            .problems
            .iter()
            .find(|problem| problem.file.ends_with(&format!("agents/{name}/agent.yaml")))
            .unwrap();
        assert!(
            problem.message.contains(fragment),
            "{name}: {}",
            problem.message
        );
    }
}
