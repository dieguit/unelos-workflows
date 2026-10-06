use unelos_core::parse_project_definitions;

#[test]
fn project_preserves_authored_values_and_defaults() {
    let project = parse_project_definitions(
        ".unelos/project.yaml",
        include_str!("fixtures/definitions/project.yaml"),
    )
    .unwrap();
    let task = &project.task_definitions["implement-plan"];
    assert_eq!(
        task.description.as_deref(),
        Some("Implement the supplied plan.")
    );
    assert_eq!(task.instructions, "Read the plan.\nVerify the changes.\n");
    assert_eq!(task.skills, ["planning", "verification"]);
    assert_eq!(task.outcomes, ["approved", "changes_requested"]);
    let agent = &project.agents["engineer"];
    assert_eq!(agent.instructions, "Follow project conventions.");
    assert_eq!(agent.skills, ["rust", "verification"]);
    assert_eq!(agent.model.as_deref(), Some("vendor/model#variant"));
    assert_eq!(agent.mcps, ["github", "local"]);
    assert_eq!(agent.tools, ["read", "shell"]);
    let task = &project.task_definitions["minimal"];
    assert!(task.description.is_none());
    assert!(task.skills.is_empty() && task.outcomes.is_empty());
    let agent = &project.agents["minimal"];
    assert!(agent.model.is_none());
    assert!(agent.skills.is_empty() && agent.mcps.is_empty() && agent.tools.is_empty());
    let empty = parse_project_definitions("empty", "{}").unwrap();
    assert!(empty.task_definitions.is_empty() && empty.agents.is_empty());
}

#[test]
fn project_rejects_malformed_or_discarded_configuration() {
    for (yaml, detail) in [
        ("taskDefinitons: {}", "taskDefinitons"),
        (
            "taskDefinitions: {x: {instructions: work, skilss: []}}",
            "skilss",
        ),
        ("agents: {x: {instructions: work, tool: []}}", "tool"),
        ("taskDefinitions: {x: {skills: []}}", "instructions"),
        ("agents: {x: {skills: []}}", "instructions"),
        ("agents: {x: {instructions: work, skills: null}}", ""),
        ("agents: {x: {instructions: work, mcps: github}}", ""),
        ("agents: {x: {instructions: work, tools: [12]}}", ""),
        ("agents: {x: {instructions: work, model: false}}", ""),
        ("taskDefinitions: {x: {instructions: null}}", ""),
        ("taskDefinitions: {x: {instructions: 42}}", ""),
        ("agents: null", ""),
        ("agents: {}\nagents: {}", ""),
        (
            "agents: {x: {instructions: first}, x: {instructions: second}}",
            "",
        ),
        ("{}\n---\nagents: {}", ""),
        ("agents: [", ""),
    ] {
        let error = match parse_project_definitions("broken-project.yaml", yaml) {
            Ok(_) => panic!("accepted invalid YAML: {yaml}"),
            Err(error) => error,
        };
        let rendered = error.to_string();
        assert!(rendered.contains("broken-project.yaml"), "{rendered}");
        assert!(rendered.contains(detail), "{rendered}");
    }
}

#[test]
fn project_rejects_non_string_definition_ids() {
    for yaml in [
        "taskDefinitions: {12: {instructions: Work}}",
        "taskDefinitions: {true: {instructions: Work}}",
        "agents: {12: {instructions: Work}}",
        "agents: {true: {instructions: Work}}",
    ] {
        assert!(
            parse_project_definitions("broken-project.yaml", yaml).is_err(),
            "accepted non-string definition ID: {yaml}"
        );
    }
}
