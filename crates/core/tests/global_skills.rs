use camino::Utf8Path;
use std::{fs, process::Command};
use unelos_core::*;

#[test]
fn global_skill_lifecycle() {
    if std::env::var_os("UNELOS_GLOBAL_SKILL_TEST_CHILD").is_none() {
        let temp = assert_fs::TempDir::new().unwrap();
        let status = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "global_skill_lifecycle"])
            .env("UNELOS_GLOBAL_SKILL_TEST_CHILD", "1")
            .env("UNELOS_CONFIG_DIR", temp.path())
            .status()
            .unwrap();
        assert!(status.success());
        return;
    }
    assert!(global_skill_summaries().unwrap().is_empty());
    assert_eq!(create_global_skill("x", "", "Body").unwrap().body, "Body");
    save_global_skill(None, "x", "x", "shared", "Updated").unwrap();
    assert_eq!(get_global_skill("x").unwrap().body, "Updated");
    assert_eq!(
        global_skill_summaries().unwrap()[0].description.as_deref(),
        Some("shared")
    );
    assert!(get_global_skill("../escape").is_err());
    assert!(save_global_skill(None, "x", "../escape", "", "").is_err());
    assert!(duplicate_global_skill("x", "../escape").is_err());
    assert!(remove_global_skill(None, "../escape").is_err());
    duplicate_global_skill("x", "copy").unwrap();
    remove_global_skill(None, "copy").unwrap();

    let temp = assert_fs::TempDir::new().unwrap();
    let root = Utf8Path::from_path(temp.path()).unwrap();
    let resolved = resolve_skills(root, &["global/x"]).unwrap();
    assert_eq!(resolved[0].id, "global/x");
    assert!(resolved[0].content.contains("Updated"));
    assert!(resolve_skills(root, &["global/../escape"]).is_err());
    fs::create_dir_all(root.join(".unelos/tasks/local")).unwrap();
    fs::write(
        root.join(".unelos/tasks/local/task.yaml"),
        "instructions: local\nskills: [global/x]\n",
    )
    .unwrap();
    let mut task = TaskDocument {
        id: "t".into(),
        description: None,
        instructions: "prompt".into(),
        skills: vec!["local".into()],
        outcomes: vec![],
    };
    assert!(save_global_task(
        None,
        TaskDocument {
            id: "invalid".into(),
            description: None,
            instructions: "prompt".into(),
            skills: vec!["local".into()],
            outcomes: vec![]
        }
    )
    .unwrap_err()
    .contains("Global tasks can only reference global Skills"));
    task.skills = vec!["global/x".into()];
    save_global_task(None, task).unwrap();
    assert!(remove_global_skill(None, "x").is_err());
    let global_agent = std::path::PathBuf::from(std::env::var("UNELOS_CONFIG_DIR").unwrap())
        .join("agents/a/agent.yaml");
    fs::create_dir_all(global_agent.parent().unwrap()).unwrap();
    fs::write(&global_agent, "instructions: hi\nskills: [global/x]\n").unwrap();
    assert!(remove_global_skill(None, "x")
        .unwrap_err()
        .contains("agent.yaml"));
    let saved = save_global_skill(Some(root), "x", "renamed", "", "New").unwrap();
    assert!(fs::read_to_string(&global_agent)
        .unwrap()
        .contains("global/renamed"));
    fs::remove_dir_all(global_agent.parent().unwrap()).unwrap();
    assert_eq!(saved.updated_references.len(), 3);
    assert!(saved
        .updated_references
        .contains(&".unelos/tasks/local/task.yaml".to_owned()));
    assert_eq!(get_global_task("t").unwrap().skills, ["global/renamed"]);
    assert!(
        fs::read_to_string(root.join(".unelos/tasks/local/task.yaml"))
            .unwrap()
            .contains("global/renamed")
    );
    assert!(get_global_skill("x").is_err());
    remove_global_task("t").unwrap();
    assert!(remove_global_skill(Some(root), "renamed").is_err());
    fs::remove_file(root.join(".unelos/tasks/local/task.yaml")).unwrap();
    fs::remove_dir(root.join(".unelos/tasks/local")).unwrap();
    remove_global_skill(Some(root), "renamed").unwrap();
    assert!(global_skill_summaries().unwrap().is_empty());
}
