use camino::Utf8Path;
use std::{collections::BTreeMap, fs};
use unelos_core::{get_task, save_task, save_task_with_outcome_renames, TaskDocument};

#[test]
fn roundtrip_and_external_instructions() {
    let temp = assert_fs::TempDir::new().unwrap();
    let root = Utf8Path::from_path(temp.path()).unwrap();
    let task = TaskDocument {
        id: "test".into(),
        description: Some("description".into()),
        instructions: "first".into(),
        skills: vec!["skill".into()],
        outcomes: vec!["done".into()],
    };
    save_task(root, None, task).unwrap();
    let mut task = get_task(root, "test").unwrap();
    assert_eq!(task.instructions, "first");
    assert_eq!(task.description.as_deref(), Some("description"));
    assert_eq!(task.skills, ["skill"]);
    assert_eq!(task.outcomes, ["done"]);
    assert!(save_task(root, None, get_task(root, "test").unwrap()).is_err());
    assert!(get_task(root, "../escape").is_err());
    let source = root.join(".unelos/tasks/test/task.yaml");
    std::fs::write(&source, "instructionsFile: prompt.md\n").unwrap();
    std::fs::write(source.with_file_name("prompt.md"), "before").unwrap();
    task.instructions = "after".into();
    save_task(root, Some("test"), task).unwrap();
    assert!(std::fs::read_to_string(source)
        .unwrap()
        .contains("instructionsFile: prompt.md"));
    assert_eq!(get_task(root, "test").unwrap().instructions, "after");
}

fn fixture() -> (assert_fs::TempDir, camino::Utf8PathBuf) {
    let temp = assert_fs::TempDir::new().unwrap();
    let root = camino::Utf8PathBuf::from_path_buf(temp.path().to_owned()).unwrap();
    fs::create_dir_all(root.join(".unelos")).unwrap();
    fs::write(root.join(".unelos/project.yaml"), "{}\n").unwrap();
    save_task(
        &root,
        None,
        TaskDocument {
            id: "test".into(),
            description: None,
            instructions: "do it".into(),
            skills: vec![],
            outcomes: vec!["good".into(), "bad".into()],
        },
    )
    .unwrap();
    save_task(
        &root,
        None,
        TaskDocument {
            id: "other".into(),
            description: None,
            instructions: "do it".into(),
            skills: vec![],
            outcomes: vec!["good".into(), "bad".into()],
        },
    )
    .unwrap();
    fs::create_dir_all(root.join(".unelos/agents/engineer")).unwrap();
    fs::write(
        root.join(".unelos/agents/engineer/agent.yaml"),
        "instructions: work\n",
    )
    .unwrap();
    fs::create_dir_all(root.join(".unelos/workflows")).unwrap();
    let workflow = "start: step\nnodes:\n  step:\n    task: test\n    agent: engineer\n    next:\n      good: success # keep\n      bad: failure\n  other:\n    task: other\n    agent: engineer\n    next: {good: success, bad: failure}\n  parallel:\n    type: parallel\n    tasks:\n      - taskDefinition:\n          instructions: work\n        agent: engineer\n    next: success\n  success:\n    type: end\n  failure:\n    type: end\n";
    for name in ["one", "two"] {
        fs::write(
            root.join(format!(".unelos/workflows/{name}.yaml")),
            format!("name: {name}\n{workflow}"),
        )
        .unwrap();
    }
    (temp, root)
}

#[test]
fn outcome_rename_updates_only_matching_nodes_and_preserves_destinations() {
    let (_temp, root) = fixture();
    let mut task = get_task(&root, "test").unwrap();
    task.outcomes[0] = "goodss".into();
    save_task_with_outcome_renames(
        &root,
        Some("test"),
        task,
        BTreeMap::from([("good".into(), "goodss".into())]),
    )
    .unwrap();
    for name in ["one", "two"] {
        let text = fs::read_to_string(root.join(format!(".unelos/workflows/{name}.yaml"))).unwrap();
        assert!(text.contains("\"goodss\": success # keep"));
        assert_eq!(text.matches("{good: success, bad: failure}").count(), 1);
        assert!(text.contains("    next: success"));
        unelos_core::parse_workflow_definition(name, &text).unwrap();
        unelos_core::validate_project(&unelos_core::load_project(&root, name).unwrap()).unwrap();
    }
}

#[test]
fn swaps_and_reordering_are_safe() {
    let (_temp, root) = fixture();
    let path = root.join(".unelos/workflows/one.yaml");
    let before = fs::read_to_string(&path).unwrap();
    let mut task = get_task(&root, "test").unwrap();
    task.outcomes.reverse();
    save_task(&root, Some("test"), task).unwrap();
    assert_eq!(fs::read_to_string(&path).unwrap(), before);
    let task = get_task(&root, "test").unwrap();
    save_task_with_outcome_renames(
        &root,
        Some("test"),
        task,
        BTreeMap::from([("good".into(), "bad".into()), ("bad".into(), "good".into())]),
    )
    .unwrap();
    let text = fs::read_to_string(path).unwrap();
    assert!(text.contains("\"bad\": success # keep"));
    assert!(text.contains("\"good\": failure"));
}

#[test]
fn invalid_and_destructive_changes_do_not_write() {
    let (_temp, root) = fixture();
    let source = root.join(".unelos/tasks/test/task.yaml");
    let workflow = root.join(".unelos/workflows/one.yaml");
    let before = (
        fs::read_to_string(&source).unwrap(),
        fs::read_to_string(&workflow).unwrap(),
    );
    for (outcomes, renames) in [
        (vec!["good", "bad"], vec![("missing", "good")]),
        (vec!["good", "bad"], vec![("good", "bad")]),
        (vec!["bad"], vec![]),
        (vec!["good", "bad", "new"], vec![]),
    ] {
        let mut task = get_task(&root, "test").unwrap();
        task.outcomes = outcomes.into_iter().map(str::to_owned).collect();
        assert!(save_task_with_outcome_renames(
            &root,
            Some("test"),
            task,
            renames
                .into_iter()
                .map(|(a, b)| (a.into(), b.into()))
                .collect()
        )
        .is_err());
        assert_eq!(fs::read_to_string(&source).unwrap(), before.0);
        assert_eq!(fs::read_to_string(&workflow).unwrap(), before.1);
    }
}

#[test]
fn workflow_write_failure_rolls_back_external_instructions_and_task() {
    let (_temp, root) = fixture();
    let source = root.join(".unelos/tasks/test/task.yaml");
    let prompt = source.with_file_name("prompt.md");
    fs::write(
        &source,
        "instructionsFile: prompt.md\noutcomes: [good, bad]\n",
    )
    .unwrap();
    fs::write(&prompt, "before").unwrap();
    let before = fs::read_to_string(&source).unwrap();
    let workflow = root.join(".unelos/workflows/one.yaml");
    let workflow_before = fs::read_to_string(&workflow).unwrap();
    fs::write(workflow.with_extension("unelos-tmp"), "block").unwrap();
    let mut task = get_task(&root, "test").unwrap();
    task.instructions = "after".into();
    task.outcomes[0] = "goodss".into();
    assert!(save_task_with_outcome_renames(
        &root,
        Some("test"),
        task,
        BTreeMap::from([("good".into(), "goodss".into())])
    )
    .is_err());
    assert_eq!(fs::read_to_string(&prompt).unwrap(), "before");
    assert_eq!(fs::read_to_string(&source).unwrap(), before);
    assert_eq!(fs::read_to_string(&workflow).unwrap(), workflow_before);
}
