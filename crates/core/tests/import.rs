use camino::{Utf8Path, Utf8PathBuf};
use std::{collections::BTreeMap, fs, sync::atomic::AtomicBool};
use unelos_core::*;

fn put(root: &Utf8Path, relative: &str, text: &str) {
    let path = root.join(relative);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

fn source() -> (tempfile::TempDir, Utf8PathBuf) {
    let temp = tempfile::tempdir().unwrap();
    let root = Utf8PathBuf::from_path_buf(temp.path().to_owned()).unwrap();
    put(&root, "workflows/demo.yaml", "# workflow\nname: demo\nstart: work\nnodes:\n  work:\n    task: shared\n    agent: worker\n    next: end\n  end: {type: end}\n");
    put(
        &root,
        "tasks/shared/task.yaml",
        "instructions: work\nskills: [review]\n",
    );
    put(
        &root,
        "agents/worker/agent.yaml",
        "instructions: 'review worker'\nskills: [review] # keep\nmcps: [tools]\n",
    );
    put(
        &root,
        "skills/review/SKILL.md",
        "---\nname: review # keep\ndescription: review prose\n---\nreview prose\n",
    );
    put(&root, "skills/review/resource.txt", "resource");
    put(
        &root,
        "mcps/tools.yaml",
        "type: local\ncommand: [echo, hello]\n",
    );
    (temp, root)
}

fn destination() -> (tempfile::TempDir, Utf8PathBuf) {
    let temp = tempfile::tempdir().unwrap();
    let root = Utf8PathBuf::from_path_buf(temp.path().to_owned()).unwrap();
    (temp, root)
}

fn import(root: &Utf8Path, source: &Utf8Path) {
    assert!(matches!(
        commit_import(root, source, "source", &BTreeMap::new()).unwrap(),
        ImportOutcome::Imported { .. }
    ));
}

#[test]
fn identical_items_are_reused_and_resources_conflict_as_a_whole() {
    let (_source, source) = source();
    let (_dest, root) = destination();
    import(&root, &source);
    let plan = plan_import(&root, &source, "display", &BTreeMap::new()).unwrap();
    assert_eq!(plan.reused.len(), 5);
    assert!(plan.conflicts.is_empty());
    assert_eq!(plan.mcps[0].command.as_deref(), Some("echo hello"));
    assert_eq!(serde_json::to_value(&plan).unwrap()["source"], "display");
    put(&root, ".unelos/skills/review/resource.txt", "different");
    let plan = plan_import(&root, &source, "display", &BTreeMap::new()).unwrap();
    let conflict = plan
        .conflicts
        .iter()
        .find(|c| c.key == "skill:review")
        .unwrap();
    assert_eq!(conflict.reason, "exists");
    assert_eq!(conflict.suggested, "review-2");
    assert!(!plan.reused.contains(&"skill:review".into()));
    let agent = plan
        .conflicts
        .iter()
        .find(|c| c.key == "agent:worker")
        .unwrap();
    assert_eq!(agent.reason, "referencesRenamed");
    assert_eq!(agent.caused_by.as_deref(), Some("skill:review"));
    assert!(plan
        .conflicts
        .iter()
        .any(|c| c.key == "workflow:demo" && c.reason == "referencesRenamed"));
    assert!(plan.conflicts.iter().all(|c| c.error.is_none()));
    import(&root, &source);
    let loaded = load_project(&root, "demo-2").unwrap();
    assert!(loaded.definitions.agents.contains_key("worker-2"));
    assert_eq!(
        loaded.definitions.agents["worker-2"].skills,
        vec!["review-2"]
    );
    assert_eq!(
        fs::read_to_string(root.join(".unelos/skills/review/resource.txt")).unwrap(),
        "different"
    );
    assert!(!root.join(".unelos/.import-staging").exists());
}

#[test]
fn extra_project_file_also_causes_whole_item_conflict() {
    let (_source, source) = source();
    let (_dest, root) = destination();
    import(&root, &source);
    put(&root, ".unelos/agents/worker/extra.txt", "extra");
    let plan = plan_import(&root, &source, "", &BTreeMap::new()).unwrap();
    assert_eq!(
        plan.conflicts
            .iter()
            .find(|c| c.key == "agent:worker")
            .unwrap()
            .reason,
        "exists"
    );
}

#[test]
fn rename_validation_and_duplicate_rows() {
    let (_source, source) = source();
    let (_dest, root) = destination();
    import(&root, &source);
    for (name, error) in [
        ("", "Enter a name"),
        ("../bad", "Use lowercase letters, numbers and dashes"),
        ("Upper", "Use lowercase letters, numbers and dashes"),
        ("has_space", "Use lowercase letters, numbers and dashes"),
        ("-lead", "Use lowercase letters, numbers and dashes"),
        ("double--dash", "Use lowercase letters, numbers and dashes"),
        ("trail-", "Use lowercase letters, numbers and dashes"),
        ("occupied", "A skill named “occupied” already exists"),
    ] {
        put(&root, ".unelos/skills/occupied/SKILL.md", "occupied");
        let renames = BTreeMap::from([("skill:review".into(), name.into())]);
        let plan = plan_import(&root, &source, "", &renames).unwrap();
        assert_eq!(
            plan.conflicts
                .iter()
                .find(|c| c.key == "skill:review")
                .unwrap()
                .error
                .as_deref(),
            Some(error)
        );
        assert!(matches!(
            commit_import(&root, &source, "", &renames).unwrap(),
            ImportOutcome::Conflicts(_)
        ));
        assert!(!root.join(".unelos/.import-staging").exists());
        assert!(!root.join(".unelos/workflows/demo-2.yaml").exists());
    }
    put(
        &source,
        "agents/second/agent.yaml",
        "instructions: second\n",
    );
    let text = fs::read_to_string(source.join("workflows/demo.yaml"))
        .unwrap()
        .replace(
            "next: end",
            "next: second\n  second:\n    task: shared\n    agent: second\n    next: end",
        );
    put(&source, "workflows/demo.yaml", &text);
    let renames = BTreeMap::from([
        ("agent:worker".into(), "same".into()),
        ("agent:second".into(), "same".into()),
    ]);
    let plan = plan_import(&root, &source, "", &renames).unwrap();
    for id in ["agent:worker", "agent:second"] {
        assert_eq!(
            plan.conflicts
                .iter()
                .find(|c| c.key == id)
                .unwrap()
                .error
                .as_deref(),
            Some("Two items can’t both be named “same”")
        );
    }
}

#[test]
fn suggestions_slugify_historical_ids_without_restricting_original_names() {
    let (_source, source) = source();
    let (_dest, root) = destination();
    fs::rename(
        source.join("skills/review"),
        source.join("skills/Code_REVIEW"),
    )
    .unwrap();
    for path in ["tasks/shared/task.yaml", "agents/worker/agent.yaml"] {
        let text = fs::read_to_string(source.join(path))
            .unwrap()
            .replace("[review]", "[Code_REVIEW]");
        put(&source, path, &text);
    }
    put(
        &source,
        "skills/Code_REVIEW/SKILL.md",
        "---\nname: Code_REVIEW\n---\nbody\n",
    );
    import(&root, &source);
    let identical = plan_import(&root, &source, "", &BTreeMap::new()).unwrap();
    assert!(identical.reused.contains(&"skill:Code_REVIEW".into()));
    assert!(identical.conflicts.is_empty());
    put(&root, ".unelos/skills/Code_REVIEW/resource.txt", "changed");
    put(&root, ".unelos/skills/code-review-2/SKILL.md", "occupied");
    let plan = plan_import(&root, &source, "", &BTreeMap::new()).unwrap();
    let conflict = plan
        .conflicts
        .iter()
        .find(|c| c.key == "skill:Code_REVIEW")
        .unwrap();
    assert_eq!(conflict.suggested, "code-review-3");
    assert!(conflict.error.is_none());
    import(&root, &source);
    assert!(root.join(".unelos/skills/code-review-3/SKILL.md").exists());
}

#[test]
fn suggestions_skip_existing_incoming_and_requested_names() {
    let (_source, source) = source();
    let (_dest, root) = destination();
    import(&root, &source);
    put(&root, ".unelos/skills/review/resource.txt", "changed");
    put(&root, ".unelos/skills/review-2/SKILL.md", "occupied");
    put(&source, "skills/review-3/SKILL.md", "incoming");
    put(&source, "skills/other/SKILL.md", "incoming");
    put(
        &source,
        "tasks/shared/task.yaml",
        "instructions: work\nskills: [review, review-3, other]\n",
    );
    let renames = BTreeMap::from([("skill:other".into(), "review-4".into())]);
    let plan = plan_import(&root, &source, "", &renames).unwrap();
    assert_eq!(
        plan.conflicts
            .iter()
            .find(|c| c.key == "skill:review")
            .unwrap()
            .suggested,
        "review-5"
    );
}

#[test]
fn targeted_rewrites_cover_parallel_inline_frontmatter_and_preserve_prose() {
    let mut files = BTreeMap::new();
    files.insert("workflows/demo.yaml".into(), b"# comment\nname: demo\nnodes:\n  one: {agent: worker, task: shared}\n  parallel:\n    tasks:\n      - {agent: worker, task: shared}\n      - {agent: worker, taskDefinition: {instructions: review, skills: [review, global/review]}}\n".to_vec());
    files.insert("agents/worker/agent.yaml".into(), b"instructions: 'review worker'\nskills: [review, global/review] # keep\nmcps: [tools, global/tools]\n".to_vec());
    files.insert(
        "tasks/shared/task.yaml".into(),
        b"instructions: review\nskills: [review]\n".to_vec(),
    );
    files.insert(
        "skills/review/SKILL.md".into(),
        b"---\nname: review # keep\ndescription: review\n---\nreview body\n".to_vec(),
    );
    let renames = BTreeMap::from([
        ("workflow:demo".into(), "renamed".into()),
        ("agent:worker".into(), "new-worker".into()),
        ("task:shared".into(), "new-shared".into()),
        ("skill:review".into(), "new-review".into()),
        ("mcp:tools".into(), "new-tools".into()),
    ]);
    let output = apply_renames(&files, &renames).unwrap();
    let workflow: serde_yaml_ng::Value =
        serde_yaml_ng::from_slice(&output[Utf8Path::new("workflows/renamed.yaml")]).unwrap();
    assert_eq!(workflow["name"], "renamed");
    assert_eq!(workflow["nodes"]["one"]["agent"], "new-worker");
    assert_eq!(
        workflow["nodes"]["parallel"]["tasks"][0]["task"],
        "new-shared"
    );
    assert_eq!(
        workflow["nodes"]["parallel"]["tasks"][1]["taskDefinition"]["skills"][0],
        "new-review"
    );
    let agent =
        String::from_utf8(output[Utf8Path::new("agents/new-worker/agent.yaml")].clone()).unwrap();
    assert!(agent.contains("instructions: 'review worker'"));
    assert!(agent.contains("global/review"));
    assert!(agent.contains("global/tools"));
    assert!(agent.contains("# keep"));
    let skill =
        String::from_utf8(output[Utf8Path::new("skills/new-review/SKILL.md")].clone()).unwrap();
    assert!(skill.contains("name: \"new-review\" # keep"));
    assert!(skill.ends_with("---\nreview body\n"));
    assert!(skill.contains("description: review"));
}

#[test]
fn layout_names_and_local_fetch_cancellation() {
    let (_source, source) = source();
    let (_dest, root) = destination();
    put(&source, "workflows/other.yaml", "name: other");
    let error = plan_import(&root, &source, "", &BTreeMap::new()).unwrap_err();
    assert_eq!(error.message, "This folder must contain exactly one workflow (found 2)\nFound demo and other. Point to the subfolder of the one you want.");
    fs::remove_file(source.join("workflows/demo.yaml")).unwrap();
    fs::remove_file(source.join("workflows/other.yaml")).unwrap();
    assert!(plan_import(&root, &source, "", &BTreeMap::new())
        .unwrap_err()
        .message
        .contains("(found 0)"));
    assert_eq!(
        fetch_source(source.as_str(), &AtomicBool::new(false))
            .unwrap()
            .dir(),
        source
    );
    assert!(fetch_source(source.as_str(), &AtomicBool::new(true)).is_err());
}

#[cfg(unix)]
#[test]
fn failed_staging_write_leaves_no_imported_items() {
    let (_source, source) = source();
    let (_dest, root) = destination();
    initialize_project(&root).unwrap();
    // A near-limit valid source filename cannot accommodate the atomic writer's temporary suffix.
    let filename = "x".repeat(250);
    put(&source, &format!("skills/review/{filename}"), "resource");
    assert!(commit_import(&root, &source, "", &BTreeMap::new()).is_err());
    assert!(!root.join(".unelos/agents/worker").exists());
    assert!(!root.join(".unelos/skills/review").exists());
    assert!(!root.join(".unelos/workflows/demo.yaml").exists());
    assert!(!root.join(".unelos/.import-staging").exists());
}

#[test]
fn missing_globals_and_unresolved_models_warn_without_blocking() {
    let (_source, source) = source();
    let (_dest, root) = destination();
    put(&source, "agents/worker/agent.yaml", "instructions: work\nmodel: import-test-unmapped-alias\nskills: [global/import-test-missing-skill]\nmcps: [global/import-test-missing-mcp]\n");
    let text = fs::read_to_string(source.join("workflows/demo.yaml"))
        .unwrap()
        .replace("task: shared", "task: global/import-test-missing-task");
    put(&source, "workflows/demo.yaml", &text);
    let plan = plan_import(&root, &source, "", &BTreeMap::new()).unwrap();
    assert_eq!(
        plan.warnings
            .iter()
            .filter(|w| w.kind == "missingGlobal")
            .count(),
        3
    );
    assert_eq!(
        plan.warnings
            .iter()
            .filter(|w| w.kind == "unresolvedModel")
            .count(),
        1
    );
    assert!(matches!(
        commit_import(&root, &source, "", &BTreeMap::new()).unwrap(),
        ImportOutcome::Imported {
            warning_count: 4,
            ..
        }
    ));
}
