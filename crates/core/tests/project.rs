use assert_fs::{fixture::PathCopy, TempDir};
use camino::{Utf8Path, Utf8PathBuf};
use std::{fs, process::Command};
use unelos_core::{
    load_project, parse_workflow_definition, LoadedProject, NodeDefinition, ProjectLoadError,
    Transition,
};

const PROJECT: &str = include_str!("fixtures/project/.unelos/project.yaml");
const WORKFLOW: &str = include_str!("fixtures/project/.unelos/workflows/implement-feature.yaml");

fn fixture_root() -> Utf8PathBuf {
    Utf8Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/project")
}

fn assert_fixture(loaded: &LoadedProject, root: &Utf8Path) {
    assert_eq!(loaded.project_root, root);
    assert_eq!(loaded.project_source, root.join(".unelos/project.yaml"));
    assert_eq!(
        loaded.workflow_source,
        root.join(".unelos/workflows/implement-feature.yaml")
    );
    assert!(loaded.project_root.is_absolute());
    assert_eq!(
        loaded.definitions.task_definitions["create-plan"].instructions,
        "Write a plan for the supplied request to its output path."
    );
    assert_eq!(
        loaded.definitions.agents["engineer"].model,
        Some(unelos_core::AgentModel::Harness { harness: "opencode".into(), model: "example-provider/example-model".into() })
    );
    assert_eq!(
        loaded.workflow,
        parse_workflow_definition("fixture", WORKFLOW).unwrap()
    );
}

fn temporary_project() -> (TempDir, Utf8PathBuf) {
    let temp = TempDir::new().unwrap();
    let root = Utf8PathBuf::from_path_buf(temp.path().join("prøject with spaces")).unwrap();
    fs::create_dir_all(&root).unwrap();
    assert_fs::fixture::ChildPath::new(root.as_std_path())
        .copy_from(fixture_root(), &["**/*"])
        .unwrap();
    fs::create_dir_all(root.join(".unelos/workflows")).unwrap();
    fs::write(root.join(".unelos/project.yaml"), PROJECT).unwrap();
    fs::write(
        root.join(".unelos/workflows/implement-feature.yaml"),
        WORKFLOW,
    )
    .unwrap();
    (temp, root)
}

#[test]
fn complete_fixture_preserves_authoring_data_and_locations() {
    let root = fixture_root();
    let loaded = load_project(&root, "implement-feature").unwrap();
    assert_fixture(&loaded, &root);
    assert_eq!(loaded.workflow.start, "plan");
    assert_eq!(loaded.workflow.nodes.len(), 5);
    let NodeDefinition::HumanGate(gate) = &loaded.workflow.nodes["approve-plan"] else {
        panic!("expected gate")
    };
    assert_eq!(gate.next["rejected"], "plan");
    let NodeDefinition::Task(review) = &loaded.workflow.nodes["review"] else {
        panic!("expected review task")
    };
    let Transition::Outcomes(routes) = &review.next else {
        panic!("expected routes")
    };
    assert_eq!(routes["changes_requested"], "implement");
    for task in loaded.definitions.task_definitions.values() {
        for skill in &task.skills {
            assert!(root
                .join(format!(".unelos/skills/{skill}/SKILL.md"))
                .is_file());
        }
    }
    for agent in loaded.definitions.agents.values() {
        for skill in &agent.skills {
            assert!(root
                .join(format!(".unelos/skills/{skill}/SKILL.md"))
                .is_file());
        }
    }
}

#[test]
fn selection_ignores_unrelated_files_and_supports_unicode() {
    let (_temp, root) = temporary_project();
    fs::write(root.join(".unelos/workflows/broken.yaml"), "nodes: [").unwrap();
    fs::create_dir_all(root.join(".unelos/skills/unused")).unwrap();
    fs::write(root.join(".unelos/skills/unused/SKILL.md"), [0xff]).unwrap();
    assert_fixture(&load_project(&root, "implement-feature").unwrap(), &root);
    fs::write(
        root.join(".unelos/workflows/café plan.yaml"),
        WORKFLOW.replace("name: implement-feature", "name: café plan"),
    )
    .unwrap();
    assert_eq!(
        load_project(&root, "café plan").unwrap().workflow.name,
        "café plan"
    );
}

#[test]
fn missing_files_report_the_expected_absolute_path() {
    for relative in [
        ".unelos/project.yaml",
        ".unelos/workflows/implement-feature.yaml",
    ] {
        let (_temp, root) = temporary_project();
        let expected = root.join(relative);
        fs::remove_file(&expected).unwrap();
        let error = load_project(&root, "implement-feature").unwrap_err();
        assert!(error.to_string().contains(expected.as_str()));
        assert!(matches!(error, ProjectLoadError::Missing { path, .. } if path == expected));
    }
}

#[test]
fn malformed_yaml_retains_path_and_parser_diagnostic() {
    for relative in [
        ".unelos/project.yaml",
        ".unelos/workflows/implement-feature.yaml",
    ] {
        let (_temp, root) = temporary_project();
        let expected = root.join(relative);
        fs::write(&expected, "invalid: [").unwrap();
        let error = load_project(&root, "implement-feature").unwrap_err();
        assert!(error.to_string().contains(expected.as_str()));
        let ProjectLoadError::Parse(parse) = error else {
            panic!("expected parser error")
        };
        assert_eq!(parse.context, expected.as_str());
        assert!(!parse.diagnostic.to_string().is_empty());
    }
}

#[test]
fn unreadable_inputs_are_not_missing_or_yaml_errors() {
    for relative in [
        ".unelos/project.yaml",
        ".unelos/workflows/implement-feature.yaml",
    ] {
        for directory in [false, true] {
            let (_temp, root) = temporary_project();
            let expected = root.join(relative);
            if directory {
                fs::remove_file(&expected).unwrap();
                fs::create_dir(&expected).unwrap();
            } else {
                fs::write(&expected, [0xff]).unwrap();
            }
            let error = load_project(&root, "implement-feature").unwrap_err();
            assert!(error.to_string().contains(expected.as_str()));
            assert!(matches!(error, ProjectLoadError::Read { path, .. } if path == expected));
        }
    }
}

#[test]
fn rejects_declared_name_mismatch_and_path_shaped_names() {
    let (_temp, root) = temporary_project();
    fs::write(
        root.join(".unelos/workflows/implement-feature.yaml"),
        WORKFLOW.replace("name: implement-feature", "name: different"),
    )
    .unwrap();
    let error = load_project(&root, "implement-feature").unwrap_err();
    assert!(error.to_string().contains(
        root.join(".unelos/workflows/implement-feature.yaml")
            .as_str()
    ));
    assert!(
        matches!(error, ProjectLoadError::NameMismatch { requested, declared, .. }
        if requested == "implement-feature" && declared == "different")
    );
    for name in [
        "",
        ".",
        "..",
        "../outside",
        "/absolute",
        "nested/name",
        "nested\\name",
    ] {
        assert!(matches!(
            load_project(&root, name),
            Err(ProjectLoadError::InvalidWorkflowName { .. })
        ));
    }
}

#[test]
fn explicit_root_is_independent_of_working_directory() {
    if let Some(root) = std::env::var_os("UNELOS_T02_TEST_ROOT") {
        let root = Utf8PathBuf::from_path_buf(root.into()).unwrap();
        assert_fixture(&load_project(&root, "implement-feature").unwrap(), &root);
        let relative = root.file_name().unwrap();
        let cwd = Utf8PathBuf::from_path_buf(std::env::current_dir().unwrap()).unwrap();
        let expected_root = cwd.join(relative);
        assert_fixture(
            &load_project(Utf8Path::new(relative), "implement-feature").unwrap(),
            &expected_root,
        );
        return;
    }
    let (temp, root) = temporary_project();
    #[cfg(unix)]
    let root = {
        // Reproduce platforms where getcwd reports the physical path behind a symlink.
        let alias = Utf8PathBuf::from_path_buf(temp.path().join("cwd-alias")).unwrap();
        std::os::unix::fs::symlink(temp.path(), &alias).unwrap();
        alias.join(root.file_name().unwrap())
    };
    assert_fixture(&load_project(&root, "implement-feature").unwrap(), &root);
    let output = Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "explicit_root_is_independent_of_working_directory",
            "--nocapture",
        ])
        .current_dir(temp.path())
        .env("UNELOS_T02_TEST_ROOT", root.as_str())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[cfg(unix)]
#[test]
fn symlinked_root_is_supported() {
    let (temp, root) = temporary_project();
    let alias = Utf8PathBuf::from_path_buf(temp.path().join("alias")).unwrap();
    std::os::unix::fs::symlink(&root, &alias).unwrap();
    assert_fixture(&load_project(&alias, "implement-feature").unwrap(), &alias);
}
