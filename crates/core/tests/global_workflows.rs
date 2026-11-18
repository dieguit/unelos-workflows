use camino::{Utf8Path, Utf8PathBuf};
use std::{fs, process::Command};
use unelos_core::*;

fn write(root: &Utf8Path, parts: &[&str], text: &str) -> Utf8PathBuf {
    let path = parts
        .iter()
        .fold(root.to_owned(), |path, part| path.join(part));
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, text).unwrap();
    path
}

#[test]
fn global_workflow_listing_runtime_composition_authoring_and_harness() {
    if std::env::var_os("UNELOS_GLOBAL_WORKFLOW_CHILD").is_none() {
        let config = assert_fs::TempDir::new().unwrap();
        let status = Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "global_workflow_listing_runtime_composition_authoring_and_harness",
            ])
            .env("UNELOS_GLOBAL_WORKFLOW_CHILD", "1")
            .env("UNELOS_CONFIG_DIR", config.path())
            .env("HOME", config.path())
            .env("XDG_CONFIG_HOME", config.path())
            .env("OPENCODE_CONFIG_DIR", config.path().join("opencode"))
            .status()
            .unwrap();
        assert!(status.success());
        return;
    }
    let config =
        Utf8PathBuf::from_path_buf(std::env::var_os("UNELOS_CONFIG_DIR").unwrap().into()).unwrap();
    let project = assert_fs::TempDir::new().unwrap();
    let root = Utf8Path::from_path(project.path()).unwrap();
    write(root, &[".unelos", "project.yaml"], "{}\n");
    write(
        &config,
        &["agents", "shared", "agent.yaml"],
        "instructions: shared agent\n",
    );
    let text = "name: shared\nstart: work\nnodes:\n  work:\n    agent: global/shared\n    taskDefinition:\n      instructions: shared task\n    next: end\n  end:\n    type: end\n";
    let source = write(&config, &["workflows", "shared.yaml"], text);
    write(root, &[".unelos", "workflows", "local.yaml"], "name: local\nstart: call\nnodes:\n  call:\n    type: workflow\n    workflow: global/shared\n    next: end\n  end:\n    type: end\n");
    assert_eq!(list_workflows(root).unwrap(), ["local", "global/shared"]);
    let rows = workflow_summaries(root).unwrap();
    assert!(!rows[0].global);
    assert!(rows[1].global);
    assert_eq!(rows[1].file, "shared.yaml");
    let broken = write(&config, &["workflows", "broken.yaml"], "not: [valid\n");
    assert!(workflow_summaries(root)
        .unwrap()
        .iter()
        .any(|row| row.name == "global/broken" && row.global && row.problem.is_some()));
    let global_rows = global_workflow_summaries().unwrap();
    assert_eq!(global_rows.len(), 2);
    assert!(global_rows.iter().all(|row| row.global));
    assert!(global_rows
        .iter()
        .any(|row| row.name == "broken" && row.problem.is_some()));
    assert!(global_rows
        .iter()
        .any(|row| row.name == "shared" && row.problem.is_none()));
    assert_eq!(list_workflows(root).unwrap(), ["local", "global/shared"]);
    fs::remove_file(broken).unwrap();
    let created = create_global_workflow().unwrap();
    assert_eq!(created, "new-workflow");
    let created_document = get_global_workflow(&created).unwrap();
    assert_eq!(created_document.id, created);
    assert_eq!(created_document.settings.start, "new-task");
    assert!(!created_document.issues.is_empty());
    assert_eq!(create_global_workflow().unwrap(), "new-workflow-2");
    remove_global_workflow("new-workflow-2").unwrap();
    remove_global_workflow(&created).unwrap();
    let run = start_workflow(root, "global/shared", Some("shared-run")).unwrap();
    assert_eq!(run.workflow_name, "global/shared");
    next(root, &run.run_id).unwrap();
    assert!(complete_node(root, &run.run_id, "work", None)
        .unwrap()
        .is_finished());
    let local = load_project(root, "local").unwrap();
    validate_project(&local).unwrap();
    assert_eq!(local.node_sources["call/work"].path, source);
    for (replacement, field) in [
        (text.replace("agent: global/shared", "agent: project-agent"), "nodes.work.agent"),
        (text.replace("taskDefinition:\n      instructions: shared task", "task: project-task"), "nodes.work.task"),
        (text.replace("instructions: shared task", "instructions: shared task\n      skills: [project-skill]"), "nodes.work.taskDefinition.skills[0]"),
        ("name: shared\nstart: call\nnodes:\n  call:\n    type: workflow\n    workflow: local\n    next: end\n  end:\n    type: end\n".into(), "nodes.call.workflow"),
    ] {
        fs::write(&source, replacement).unwrap();
        let error = load_project(root, "global/shared").unwrap_err().to_string();
        assert!(error.contains(field), "{error}");
    }
    fs::write(&source, text).unwrap();
    assert!(get_global_workflow("shared").unwrap().issues.is_empty());
    let document = get_global_workflow("shared").unwrap();
    assert_eq!(document.id, "shared");
    let mut draft = WorkflowDraft {
        settings: document.settings,
        steps: document.steps,
    };
    draft.settings.description = Some("updated global description".into());
    assert!(validate_global_workflow("shared", &draft)
        .unwrap()
        .is_empty());
    save_global_workflow("shared", &draft).unwrap();
    assert_eq!(
        get_global_workflow("shared")
            .unwrap()
            .settings
            .description
            .as_deref(),
        Some("updated global description")
    );
    let saved = fs::read(&source).unwrap();
    draft
        .steps
        .iter_mut()
        .find(|step| step.id == "work")
        .unwrap()
        .agent = Some("local-agent".into());
    assert!(!validate_global_workflow("shared", &draft)
        .unwrap()
        .is_empty());
    assert!(save_global_workflow("shared", &draft).is_err());
    assert_eq!(fs::read(&source).unwrap(), saved);
    fs::write(
        &source,
        text.replace("instructions: shared task", "instructionsFile: prompt.md"),
    )
    .unwrap();
    write(
        &config,
        &["workflows", "prompt.md"],
        "global child prompt\r\n",
    );
    let child = load_project(root, "local").unwrap();
    let NodeDefinition::Task(task) = &child.workflow.nodes["call/work"] else {
        panic!("expected task");
    };
    let TaskSelection::Inline(task) = &task.task else {
        panic!("expected inline task");
    };
    assert_eq!(
        task.instructions.replace("\r\n", "\n"),
        "global child prompt\n"
    );
    fs::write(&source, text).unwrap();
    duplicate_global_workflow("shared", "copy").unwrap();
    assert_eq!(get_global_workflow("copy").unwrap().settings.name, "copy");
    remove_global_workflow("copy").unwrap();
    assert!(get_global_workflow("../escape").is_err());
    let cli = write(root, &["cli"], "test executable");
    install_harness("opencode", &cli).unwrap();
    let user_agent = config
        .join("opencode")
        .join("agents")
        .join("unelos-global-shared.md");
    assert!(!user_agent.exists());
    let direct_text = text.replacen("name: shared", "name: direct", 1).replace(
        "agent: global/shared",
        "agent: global/shared\n    execution: subagent",
    );
    let direct = write(root, &[".unelos", "workflows", "direct.yaml"], &direct_text);
    build_harness_config(root).unwrap();
    let other = assert_fs::TempDir::new().unwrap();
    build_harness_config(Utf8Path::from_path(other.path()).unwrap()).unwrap();
    build_global_harness_agents().unwrap();
    assert!(user_agent.is_file());
    let project_agent = write(
        root,
        &[".opencode", "agents", "unelos-global-shared.md"],
        &fs::read_to_string(&user_agent)
            .unwrap()
            .replace("mode: subagent", "mode: primary"),
    );
    let build = build_harness_config(root).unwrap();
    assert!(build.skipped.is_empty(), "{:?}", build.skipped);
    assert!(!project_agent.exists());
    fs::write(
        &direct,
        text.replace("name: shared", "name: direct")
            .replace("agent: global/shared", "agent: global/missing"),
    )
    .unwrap();
    assert!(build_harness_config(root)
        .unwrap()
        .skipped
        .iter()
        .any(|skip| skip.starts_with("direct:") && skip.contains("global/missing")));
    fs::remove_file(&direct).unwrap();
    let built = build_harness_config(root).unwrap();
    assert!(built.removed.contains(&user_agent));
    assert!(!user_agent.exists());
    write(
        root,
        &[".unelos", "agents", "global-shared", "agent.yaml"],
        "instructions: local collision\n",
    );
    write(root, &[".unelos", "workflows", "collision.yaml"], "name: collision\nstart: work\nnodes:\n  work:\n    agent: global-shared\n    taskDefinition:\n      instructions: local\n    next: end\n  end:\n    type: end\n");
    assert!(build_harness_config(root)
        .unwrap_err()
        .message
        .contains("collision"));
    write(&config, &["workflows", "caller.yaml"], "name: caller\nstart: call\nnodes:\n  call:\n    type: workflow\n    workflow: global/shared\n    next: end\n  end:\n    type: end\n");
    assert!(remove_global_workflow("shared")
        .unwrap_err()
        .contains("referenced"));
    assert!(remove_global_agent("shared")
        .unwrap_err()
        .contains("referenced"));
    remove_global_workflow("caller").unwrap();
    remove_global_workflow("shared").unwrap();
    fs::remove_file(root.join(".unelos").join("workflows").join("local.yaml")).unwrap();
    fs::remove_file(
        root.join(".unelos")
            .join("workflows")
            .join("collision.yaml"),
    )
    .unwrap();
    fs::write(&direct, &direct_text).unwrap();
    assert!(build_harness_config(root).unwrap().skipped.is_empty());
    build_harness_config(Utf8Path::from_path(other.path()).unwrap()).unwrap();
    build_global_harness_agents().unwrap();
    assert!(fs::read_to_string(&user_agent)
        .unwrap()
        .contains("mode: subagent"));
    remove_global_agent("shared").unwrap();
    let broken_agent = write(
        &config,
        &["agents", "broken", "agent.yaml"],
        "instructions: [bad\n",
    );
    let result = build_global_harness_agents().unwrap();
    assert!(result
        .skipped
        .iter()
        .any(|skip| skip.starts_with("global/broken:")));
    assert!(user_agent.exists());
    fs::remove_dir_all(broken_agent.parent().unwrap()).unwrap();
    build_global_harness_agents().unwrap();
    assert!(!user_agent.exists());
}

#[test]
fn global_import_dependencies_stay_external_when_present_or_missing() {
    if std::env::var_os("UNELOS_GLOBAL_IMPORT_CHILD").is_none() {
        let config = assert_fs::TempDir::new().unwrap();
        let status = Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "global_import_dependencies_stay_external_when_present_or_missing",
            ])
            .env("UNELOS_GLOBAL_IMPORT_CHILD", "1")
            .env("UNELOS_CONFIG_DIR", config.path())
            .status()
            .unwrap();
        assert!(status.success());
        return;
    }
    let source = assert_fs::TempDir::new().unwrap();
    let source = Utf8Path::from_path(source.path()).unwrap();
    write(source, &["project.yaml"], "{}\n");
    write(source, &["workflows", "incoming.yaml"], "name: incoming\nstart: work\nnodes:\n  work:\n    agent: global/shared\n    taskDefinition:\n      instructions: incoming\n    next: call\n  call:\n    type: workflow\n    workflow: global/child\n    next: end\n  end:\n    type: end\n");
    let destination = assert_fs::TempDir::new().unwrap();
    let root = Utf8Path::from_path(destination.path()).unwrap();
    let plan = plan_import(root, source, source.as_str(), &Default::default()).unwrap();
    assert!(plan.warnings.iter().any(
        |warning| warning.kind == "missingGlobal" && warning.message.contains("global/shared")
    ));
    assert!(
        plan.warnings
            .iter()
            .any(|warning| warning.kind == "missingGlobal"
                && warning.message.contains("global/child"))
    );
    commit_import(root, source, source.as_str(), &Default::default()).unwrap();
    assert!(!root.join(".unelos").join("agents").join("global").exists());
    assert!(!root
        .join(".unelos")
        .join("workflows")
        .join("child.yaml")
        .exists());
    let config =
        Utf8PathBuf::from_path_buf(std::env::var_os("UNELOS_CONFIG_DIR").unwrap().into()).unwrap();
    write(
        &config,
        &["agents", "shared", "agent.yaml"],
        "instructions: shared\n",
    );
    write(&config, &["workflows", "child.yaml"], "name: child\nstart: work\nnodes:\n  work:\n    agent: global/shared\n    taskDefinition:\n      instructions: global child\n    next: end\n  end:\n    type: end\n");
    let plan = plan_import(root, source, source.as_str(), &Default::default()).unwrap();
    assert!(!plan
        .warnings
        .iter()
        .any(|warning| warning.kind == "missingGlobal"));
    commit_import(root, source, source.as_str(), &Default::default()).unwrap();
    validate_project(&load_project(root, "incoming").unwrap()).unwrap();
    assert!(!root.join(".unelos").join("agents").join("shared").exists());
    assert!(!root
        .join(".unelos")
        .join("workflows")
        .join("child.yaml")
        .exists());
}
