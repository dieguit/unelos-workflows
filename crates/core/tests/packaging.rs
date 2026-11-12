use assert_fs::TempDir;
use camino::{Utf8Path, Utf8PathBuf};
use std::fs;
use unelos_core::*;

fn put(root: &Utf8Path, relative: &str, text: &str) {
    let path = root.join(relative);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

fn project() -> (TempDir, Utf8PathBuf) {
    let temp = TempDir::new().unwrap();
    let root = Utf8PathBuf::from_path_buf(temp.path().to_owned()).unwrap();
    put(&root, ".unelos/project.yaml", "# Keep settings\n{}\n");
    put(
        &root,
        ".unelos/tasks/shared/task.yaml",
        "instructionsFile: instructions.md\n",
    );
    put(
        &root,
        ".unelos/tasks/shared/instructions.md",
        "TASK_MARKER\n",
    );
    put(
        &root,
        ".unelos/agents/worker/agent.yaml",
        "instructionsFile: instructions.md\n",
    );
    put(
        &root,
        ".unelos/agents/worker/instructions.md",
        "AGENT_MARKER\n",
    );
    put(&root, ".unelos/workflows/demo.yaml", "name: demo\nstart: work\nnodes:\n  work:\n    task: shared\n    agent: worker\n    next: end\n  end: {type: end}\n");
    (temp, root)
}

#[test]
fn discovers_folders_and_resolves_file_local_instructions() {
    let (_temp, root) = project();
    let loaded = load_project(&root, "demo").unwrap();
    assert_eq!(
        loaded.definitions.task_definitions["shared"].instructions,
        "TASK_MARKER\n"
    );
    assert_eq!(
        loaded.definitions.agents["worker"].instructions,
        "AGENT_MARKER\n"
    );
}

#[test]
fn discovery_errors_name_the_actual_file() {
    for (relative, contents, detail) in [
        (
            ".unelos/tasks/broken/task.yaml",
            "instructions: [",
            "tasks/broken/task.yaml",
        ),
        (
            ".unelos/agents/broken/agent.yaml",
            "instructions: [",
            "agents/broken/agent.yaml",
        ),
        (
            ".unelos/tasks/bad id/task.yaml",
            "instructions: work",
            "bad id",
        ),
    ] {
        let (_temp, root) = project();
        put(&root, relative, contents);
        let error = load_project(&root, "demo").unwrap_err().to_string();
        assert!(error.contains(detail), "{error}");
    }
    let (_temp, root) = project();
    fs::create_dir_all(root.join(".unelos/agents/missing")).unwrap();
    let error = load_project(&root, "demo").unwrap_err().to_string();
    assert!(error.contains("agents/missing/agent.yaml"), "{error}");
}

#[test]
fn empty_registries_and_unselected_workflows_are_allowed() {
    let temp = TempDir::new().unwrap();
    let root = Utf8Path::from_path(temp.path()).unwrap();
    put(root, ".unelos/project.yaml", "{}\n");
    put(
        root,
        ".unelos/workflows/demo.yaml",
        "name: demo\nstart: end\nnodes: {end: {type: end}}\n",
    );
    put(root, ".unelos/workflows/broken.yaml", "nodes: [");
    put(root, ".unelos/tasks/not-a-definition.txt", "ignored");
    let loaded = load_project(root, "demo").unwrap();
    assert!(loaded.definitions.task_definitions.is_empty());
    assert!(loaded.definitions.agents.is_empty());
}

#[test]
fn instruction_paths_and_reads_are_bounded_and_diagnostic() {
    for path in [
        "/outside.md",
        "../../../../outside.md",
        "C:\\outside.md",
        "..\\..\\outside.md",
        "missing.md",
        "directory",
    ] {
        let (_temp, root) = project();
        fs::create_dir_all(root.join(".unelos/tasks/shared/directory")).unwrap();
        put(
            &root,
            ".unelos/tasks/shared/task.yaml",
            &format!("instructionsFile: '{path}'\n"),
        );
        let error = load_project(&root, "demo").unwrap_err().to_string();
        assert!(error.contains("tasks/shared/task.yaml"), "{error}");
        assert!(error.contains("instructionsFile"), "{error}");
    }
    let (_temp, root) = project();
    fs::write(root.join(".unelos/tasks/shared/instructions.md"), [0xff]).unwrap();
    let error = load_project(&root, "demo").unwrap_err().to_string();
    assert!(error.contains("instructions.md"), "{error}");
    assert!(error.contains("instructionsFile"), "{error}");
}

#[test]
fn parent_instruction_references_can_stay_inside_authoring_root() {
    let (_temp, root) = project();
    put(&root, ".unelos/shared/instructions.md", "SHARED_MARKER\n");
    put(
        &root,
        ".unelos/tasks/shared/task.yaml",
        "instructionsFile: ../../shared/instructions.md\n",
    );
    assert_eq!(
        load_project(&root, "demo")
            .unwrap()
            .definitions
            .task_definitions["shared"]
            .instructions,
        "SHARED_MARKER\n"
    );
}

#[cfg(unix)]
#[test]
fn symlinked_instructions_and_definition_folders_cannot_escape() {
    for folder in [false, true] {
        let (_temp, root) = project();
        let outside = TempDir::new().unwrap();
        fs::write(outside.path().join("task.yaml"), "instructions: external").unwrap();
        if folder {
            std::os::unix::fs::symlink(outside.path(), root.join(".unelos/tasks/escape")).unwrap();
        } else {
            fs::remove_file(root.join(".unelos/tasks/shared/instructions.md")).unwrap();
            std::os::unix::fs::symlink(
                outside.path().join("task.yaml"),
                root.join(".unelos/tasks/shared/instructions.md"),
            )
            .unwrap();
        }
        assert!(load_project(&root, "demo").is_err());
    }
}

#[test]
fn project_settings_are_not_definition_registries() {
    assert!(parse_project_settings("project.yaml", "# settings\n{}\n").is_ok());
    for yaml in [
        "taskDefinitions: {}",
        "agents: {}",
        "settings: {}",
        "null",
        "[]",
    ] {
        let error = parse_project_settings("project.yaml", yaml)
            .unwrap_err()
            .to_string();
        assert!(error.contains("project.yaml"), "{error}");
    }
    let (_temp, root) = project();
    put(
        &root,
        ".unelos/project.yaml",
        "taskDefinitions: {}\nagents: {}\n",
    );
    assert!(load_project(&root, "demo").is_err());
}

#[test]
fn init_creates_folder_scaffold_and_preserves_settings() {
    let temp = TempDir::new().unwrap();
    let root = Utf8Path::from_path(temp.path()).unwrap();
    initialize_project(root).unwrap();
    for directory in ["tasks", "agents", "workflows", "skills"] {
        assert!(root.join(".unelos").join(directory).is_dir());
    }
    let settings = fs::read_to_string(root.join(".unelos/project.yaml")).unwrap();
    parse_project_settings("project.yaml", &settings).unwrap();
    put(root, ".unelos/project.yaml", "# user settings\n{}\n");
    initialize_project(root).unwrap();
    assert_eq!(
        fs::read_to_string(root.join(".unelos/project.yaml")).unwrap(),
        "# user settings\n{}\n"
    );
}

#[test]
fn inline_tasks_are_local_and_use_workflow_relative_instructions() {
    let (_temp, root) = project();
    put(&root, ".unelos/workflows/local.md", "LOCAL_MARKER\n");
    let yaml = fs::read_to_string(root.join(".unelos/workflows/demo.yaml"))
        .unwrap()
        .replace(
            "task: shared",
            "taskDefinition: {instructionsFile: local.md}",
        );
    put(&root, ".unelos/workflows/demo.yaml", &yaml);
    put(
        &root,
        ".unelos/workflows/other.yaml",
        &yaml
            .replace("name: demo", "name: other")
            .replace("local.md", "other.md"),
    );
    put(&root, ".unelos/workflows/other.md", "OTHER_MARKER\n");
    let loaded = load_project(&root, "demo").unwrap();
    validate_project(&loaded).unwrap();
    let inspection = inspect_task(&loaded, "work").unwrap();
    assert_eq!(inspection.context.task_instructions, "LOCAL_MARKER\n");
    assert_eq!(
        inspection.context.task_definition,
        "nodes.work.taskDefinition"
    );
    assert_eq!(loaded.definitions.task_definitions.len(), 1);
    let other = load_project(&root, "other").unwrap();
    assert_eq!(
        inspect_task(&other, "work")
            .unwrap()
            .context
            .task_instructions,
        "OTHER_MARKER\n"
    );
    start(&loaded, "inline-run", "request").unwrap();
    let packet = serde_json::to_value(next(&root, "inline-run").unwrap()).unwrap();
    assert_eq!(packet["taskInstructions"], "LOCAL_MARKER\n");
    assert_eq!(packet["taskDefinition"], "nodes.work.taskDefinition");
}

#[test]
fn parallel_inline_workers_keep_isolated_context_and_referencing_diagnostics() {
    let (_temp, root) = project();
    put(&root, ".unelos/workflows/local.md", "INLINE_WORKER\n");
    put(&root, ".unelos/workflows/demo.yaml", "name: demo\nstart: work\nnodes:\n  work:\n    type: parallel\n    tasks:\n      - task: shared\n        agent: worker\n      - taskDefinition: {instructionsFile: local.md}\n        agent: worker\n    next: end\n  end: {type: end}\n");
    let loaded = load_project(&root, "demo").unwrap();
    validate_project(&loaded).unwrap();
    let value = serde_json::to_value(inspect_node(&loaded, "work").unwrap()).unwrap();
    assert_eq!(value["workers"][0]["taskInstructions"], "TASK_MARKER\n");
    assert_eq!(value["workers"][1]["taskInstructions"], "INLINE_WORKER\n");
    assert_eq!(
        value["workers"][1]["taskDefinition"],
        "nodes.work.tasks[1].taskDefinition"
    );
    assert_eq!(value["workers"][1]["key"], "worker-2");
    let yaml = fs::read_to_string(root.join(".unelos/workflows/demo.yaml"))
        .unwrap()
        .replace("agent: worker", "agent: missing");
    put(&root, ".unelos/workflows/demo.yaml", &yaml);
    let error = inspect_node(&load_project(&root, "demo").unwrap(), "work")
        .unwrap_err()
        .to_string();
    assert!(error.contains("nodes.work.tasks[0].agent"), "{error}");
    assert!(error.contains("workflows/demo.yaml"), "{error}");
}

#[test]
fn inline_outcomes_and_skill_deduplication_match_named_tasks() {
    let (_temp, root) = project();
    put(&root, ".unelos/skills/check/SKILL.md", "CHECK_SKILL\n");
    put(
        &root,
        ".unelos/agents/worker/agent.yaml",
        "instructions: agent\nskills: [check]\n",
    );
    put(&root, ".unelos/workflows/demo.yaml", "name: demo\nstart: work\nnodes:\n  work:\n    taskDefinition: {instructions: Work, skills: [check, check], outcomes: [passed, failed]}\n    agent: worker\n    next: {passed: end, failed: work}\n  end: {type: end}\n");
    let loaded = load_project(&root, "demo").unwrap();
    validate_project(&loaded).unwrap();
    let context = inspect_task(&loaded, "work").unwrap().context;
    assert_eq!(context.skills.len(), 1);
    assert_eq!(context.outcomes, ["passed", "failed"]);
    let yaml = fs::read_to_string(root.join(".unelos/workflows/demo.yaml"))
        .unwrap()
        .replace("next: {passed: end, failed: work}", "next: end");
    put(&root, ".unelos/workflows/demo.yaml", &yaml);
    assert!(validate_project(&load_project(&root, "demo").unwrap())
        .unwrap_err()
        .to_string()
        .contains("outcomes"));
}

#[test]
fn import_preserves_settings_and_copies_resources() {
    let (_source, source) = project();
    put(
        &source,
        ".unelos/agents/worker/templates/report.md",
        "REPORT_TEMPLATE\n",
    );
    let target = TempDir::new().unwrap();
    let root = Utf8Path::from_path(target.path()).unwrap();
    initialize_project(root).unwrap();
    put(root, ".unelos/project.yaml", "# User comment\n{}\n");
    install_workflow(root, Some(&source), "demo").unwrap();
    install_workflow(root, Some(&source), "demo").unwrap();
    assert_eq!(
        fs::read_to_string(root.join(".unelos/project.yaml")).unwrap(),
        "# User comment\n{}\n"
    );
    assert_eq!(
        fs::read_to_string(root.join(".unelos/agents/worker/templates/report.md")).unwrap(),
        "REPORT_TEMPLATE\n"
    );
    assert_eq!(
        inspect_task(&load_project(root, "demo").unwrap(), "work")
            .unwrap()
            .context
            .task_instructions,
        "TASK_MARKER\n"
    );
    install_workflow(root, Some(root), "demo").unwrap();
}

#[test]
fn flat_import_copies_shared_instruction_dependencies_and_inline_skills_only() {
    let source = TempDir::new().unwrap();
    let package = Utf8Path::from_path(source.path()).unwrap();
    put(package, "shared/instructions.md", "SHARED\n");
    put(
        package,
        "agents/worker/agent.yaml",
        "instructionsFile: ../../shared/instructions.md\n",
    );
    put(
        package,
        "tasks/shared/task.yaml",
        "instructionsFile: ../../shared/instructions.md\n",
    );
    put(
        package,
        "tasks/unused/task.yaml",
        "instructionsFile: instructions.md\n",
    );
    put(package, "tasks/unused/instructions.md", "UNUSED\n");
    put(package, "skills/check/SKILL.md", "CHECK\n");
    put(package, "skills/check/reference.md", "REFERENCE\n");
    put(package, "skills/unused/SKILL.md", "UNUSED_SKILL\n");
    put(package, "workflows/demo.yaml", "name: demo\nstart: work\nnodes:\n  work:\n    type: parallel\n    tasks:\n      - {task: shared, agent: worker}\n      - {taskDefinition: {instructionsFile: ../shared/instructions.md, skills: [check]}, agent: worker}\n    next: end\n  end: {type: end}\n");
    let target = TempDir::new().unwrap();
    let root = Utf8Path::from_path(target.path()).unwrap();
    install_workflow(root, Some(package), "demo").unwrap();
    let loaded = load_project(root, "demo").unwrap();
    assert_eq!(
        loaded.definitions.task_definitions["shared"].instructions,
        "SHARED\n"
    );
    assert_eq!(
        fs::read_to_string(root.join(".unelos/shared/instructions.md")).unwrap(),
        "SHARED\n"
    );
    assert!(root.join(".unelos/skills/check/reference.md").is_file());
    assert!(!root.join(".unelos/tasks/unused").exists());
    assert!(!root.join(".unelos/skills/unused").exists());
    let workers = serde_json::to_value(inspect_node(&loaded, "work").unwrap()).unwrap();
    assert_eq!(workers["workers"][1]["skills"][0]["content"], "CHECK\n");
}

#[test]
fn collisions_are_preflighted_before_any_authoring_write() {
    let (_source, source) = project();
    put(
        &source,
        ".unelos/agents/worker/templates/report.md",
        "REPORT\n",
    );
    for conflict in [
        ".unelos/tasks/shared/task.yaml",
        ".unelos/agents/worker/templates/report.md",
        ".unelos/workflows/demo.yaml",
    ] {
        let target = TempDir::new().unwrap();
        let root = Utf8Path::from_path(target.path()).unwrap();
        put(root, conflict, "USER_CONTENT\n");
        let error = install_workflow(root, Some(&source), "demo").unwrap_err();
        assert_eq!(error.code, "import_collision");
        assert_eq!(
            fs::read_to_string(root.join(conflict)).unwrap(),
            "USER_CONTENT\n"
        );
        assert!(!root.join(".unelos/project.yaml").exists());
        assert!(!root.join(".unelos/agents/worker/agent.yaml").exists());
    }
}

#[test]
fn sole_flat_example_installs_with_its_bundled_skills() {
    let package = Utf8Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/superpowers-dev");
    assert!(!package.join(".unelos").exists());
    assert!(!package.join("project.yaml").exists());
    let target = TempDir::new().unwrap();
    let root = Utf8Path::from_path(target.path()).unwrap();
    install_workflow(root, Some(&package), "superpowers-dev").unwrap();
    let loaded = load_project(root, "superpowers-dev").unwrap();
    validate_project(&loaded).unwrap();
    assert!(loaded.definitions.task_definitions.is_empty());
    assert_eq!(
        inspect_task(&loaded, "advance").unwrap().context.outcomes,
        ["fix", "escalate", "next_task", "all_done"]
    );
    assert_eq!(
        inspect_task(&loaded, "advance-final")
            .unwrap()
            .context
            .outcomes,
        ["final_fix", "finish"]
    );
    let mut skills: Vec<String> = std::fs::read_dir(root.join(".unelos/skills"))
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .collect();
    skills.sort();
    assert_eq!(
        skills,
        [
            "brainstorming",
            "code-review",
            "finishing-a-development-branch",
            "receiving-code-review",
            "sdd-controller",
            "systematic-debugging",
            "test-driven-development",
            "using-git-worktrees",
            "verification-before-completion",
            "writing-plans",
        ]
    );
    for id in &skills {
        let text = std::fs::read_to_string(root.join(".unelos/skills").join(id).join("SKILL.md"))
            .unwrap()
            .replace("\r\n", "\n");
        assert!(
            text.starts_with(&format!("---\nname: {id}\n")),
            "{id} frontmatter"
        );
        assert!(
            text.contains("Adapted from superpowers"),
            "{id} attribution"
        );
        assert!(text.lines().count() <= 200, "{id} exceeds 200 lines");
        for banned in ["superpowers:", "docs/superpowers", "scripts/"] {
            assert!(!text.contains(banned), "{id} contains {banned}");
        }
    }
    assert!(!root.join(".unelos/README.md").exists());
}

#[test]
fn instructions_cannot_import_runtime_state_or_integration_files() {
    for relative in [
        "runs/a/state.json",
        "runtime.lock",
        "run-sequence",
        "rename.json",
        "opencode/index.js",
        "integration.json",
        "active-run",
    ] {
        let (_temp, root) = project();
        put(&root, &format!(".unelos/{relative}"), "RUNTIME_STATE\n");
        put(
            &root,
            ".unelos/tasks/shared/task.yaml",
            &format!("instructionsFile: ../../{relative}\n"),
        );
        let error = load_project(&root, "demo").unwrap_err().to_string();
        assert!(error.contains("instructionsFile"), "{error}");
    }
}

#[cfg(unix)]
#[test]
fn workflow_discovery_rejects_symlinked_settings_and_directories() {
    for relative in ["project.yaml", "workflows"] {
        let (_temp, root) = project();
        fs::remove_file(root.join(".unelos/workflows/demo.yaml")).unwrap();
        let outside = TempDir::new().unwrap();
        let target = outside.path().join("external");
        let path = root.join(".unelos").join(relative);
        fs::rename(&path, &target).unwrap();
        std::os::unix::fs::symlink(&target, &path).unwrap();
        assert!(list_workflows(&root).is_err(), "accepted {relative}");
    }
}

#[test]
fn import_rejects_dependencies_that_create_incomplete_definition_folders() {
    let (_temp, source) = project();
    put(
        &source,
        ".unelos/tasks/unused/task.yaml",
        "instructions: unused\n",
    );
    put(&source, ".unelos/tasks/unused/shared.md", "SHARED\n");
    put(
        &source,
        ".unelos/tasks/shared/task.yaml",
        "instructionsFile: ../unused/shared.md\n",
    );
    load_project(&source, "demo").unwrap();
    let target = TempDir::new().unwrap();
    let root = Utf8Path::from_path(target.path()).unwrap();
    let error = install_workflow(root, Some(&source), "demo")
        .unwrap_err()
        .to_string();
    assert!(error.contains("unused"), "{error}");
    assert!(!root.join(".unelos").exists());
}

#[cfg(unix)]
#[test]
fn import_preflight_rejects_symlinked_resources_and_destinations() {
    for destination in [false, true] {
        let (_temp, source) = project();
        let outside = TempDir::new().unwrap();
        let external = outside.path().join("external");
        fs::write(&external, "PRESERVE\n").unwrap();
        let target = TempDir::new().unwrap();
        let root = Utf8Path::from_path(target.path()).unwrap();
        let link = if destination {
            root.join(".unelos/agents/worker/agent.yaml")
        } else {
            source.join(".unelos/agents/worker/template.md")
        };
        fs::create_dir_all(link.parent().unwrap()).unwrap();
        std::os::unix::fs::symlink(&external, &link).unwrap();
        assert!(install_workflow(root, Some(&source), "demo").is_err());
        assert_eq!(fs::read_to_string(external).unwrap(), "PRESERVE\n");
        assert!(!root.join(".unelos/project.yaml").exists());
        assert!(!root.join(".unelos/workflows/demo.yaml").exists());
    }
}
