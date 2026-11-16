use camino::Utf8Path;
use unelos_core::{get_agent, save_agent, AgentDocument, AgentModel};

#[test]
fn create_update_models_and_validation() {
    let temp = assert_fs::TempDir::new().unwrap();
    let root = Utf8Path::from_path(temp.path()).unwrap();
    let agent = AgentDocument {
        id: "test".into(),
        instructions: "first".into(),
        skills: vec![],
        mcps: vec!["github".into()],
        tools: vec!["read".into()],
        model: Some(AgentModel::Alias("smart".into())),
        effort: None,
    };
    let mut agent = save_agent(root, None, agent).unwrap();
    assert_eq!(agent.instructions, "first");
    assert!(save_agent(root, None, get_agent(root, "test").unwrap())
        .unwrap_err()
        .contains("Agent 'test' already exists"));
    agent.instructions = "second".into();
    agent.model = Some(AgentModel::Harness {
        harness: "opencode".into(),
        model: "provider/model".into(),
    });
    let agent = save_agent(root, Some("test"), agent).unwrap();
    assert_eq!(agent.instructions, "second");
    assert_eq!(agent.mcps, ["github"]);
    assert!(matches!(agent.model, Some(AgentModel::Harness { .. })));
    assert!(save_agent(root, Some("other"), agent).is_err());
    assert!(get_agent(root, "../escape").is_err());
    let mut invalid = get_agent(root, "test").unwrap();
    invalid.id = "../escape".into();
    assert!(save_agent(root, None, invalid).is_err());
}

#[test]
fn direct_default_effort_normalized() {
    let temp = assert_fs::TempDir::new().unwrap();
    let root = Utf8Path::from_path(temp.path()).unwrap();
    let agent = AgentDocument {
        id: "test".into(),
        instructions: "x".into(),
        skills: vec![],
        mcps: vec![],
        tools: vec![],
        model: Some(AgentModel::Harness {
            harness: "claude".into(),
            model: "sonnet".into(),
        }),
        effort: Some("default".into()),
    };
    let mut agent = save_agent(root, None, agent).unwrap();
    assert_eq!(agent.effort, None);
    agent.model = Some(AgentModel::Alias("smart".into()));
    agent.effort = Some("default".into());
    let agent = save_agent(root, Some("test"), agent).unwrap();
    assert_eq!(agent.effort.as_deref(), Some("default"));
}

#[test]
fn legacy_inline_opencode_variant_split() {
    let temp = assert_fs::TempDir::new().unwrap();
    let root = Utf8Path::from_path(temp.path()).unwrap();
    let source = root.join(".unelos/agents/test/agent.yaml");
    std::fs::create_dir_all(source.parent().unwrap()).unwrap();
    let yaml = "instructions: x\nmodel: {harness: opencode, model: \"p/m#high\"}\n";
    std::fs::write(&source, yaml).unwrap();
    let agent = get_agent(root, "test").unwrap();
    assert_eq!(
        agent.model,
        Some(AgentModel::Harness {
            harness: "opencode".into(),
            model: "p/m".into()
        })
    );
    assert_eq!(agent.effort.as_deref(), Some("high"));
    assert_eq!(std::fs::read_to_string(&source).unwrap(), yaml);
    save_agent(root, Some("test"), agent).unwrap();
    let migrated = std::fs::read_to_string(&source).unwrap();
    assert!(!migrated.contains('#'));
    assert!(migrated.contains("effort: high"));
}

#[test]
fn preserves_external_instructions_and_rejects_invalid_models() {
    let temp = assert_fs::TempDir::new().unwrap();
    let root = Utf8Path::from_path(temp.path()).unwrap();
    std::fs::create_dir_all(root.join(".unelos/agents/test")).unwrap();
    let source = root.join(".unelos/agents/test/agent.yaml");
    std::fs::write(&source, "instructionsFile: prompt.md\nmodel: smart\n").unwrap();
    std::fs::write(source.with_file_name("prompt.md"), "before").unwrap();
    let mut agent = get_agent(root, "test").unwrap();
    assert_eq!(agent.instructions, "before");
    agent.instructions = "after".into();
    save_agent(root, Some("test"), agent).unwrap();
    assert!(std::fs::read_to_string(source)
        .unwrap()
        .contains("instructionsFile: prompt.md"));
    assert_eq!(get_agent(root, "test").unwrap().instructions, "after");
    for model in [
        "''",
        "false",
        "{harness: opencode, model: ''}",
        "{harness: opencode, model: x, extra: y}",
        "{harness: 123, model: x}",
    ] {
        assert!(
            unelos_core::parse_agent_definition(
                "test",
                &format!("instructions: hi\nmodel: {model}\n")
            )
            .is_err(),
            "{model}"
        );
    }
}
