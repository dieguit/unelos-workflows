use assert_fs::{fixture::PathCopy, TempDir};
use camino::Utf8Path;
use unelos_core::*;

#[test]
fn export_deduplicates_agents_and_combines_native_modes() {
    let source = TempDir::new().unwrap();
    source
        .copy_from(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../examples/opencode-execution"),
            &["**/*"],
        )
        .unwrap();
    let root = Utf8Path::from_path(source.path()).unwrap();
    let workflow = root.join(".unelos/workflows/page-change.yaml");
    let text = std::fs::read_to_string(&workflow)
        .unwrap()
        .replace("agent: t16-engineer", "agent: t16-controller");
    std::fs::write(workflow, text).unwrap();
    let agents = export_workflow_agents(root, "page-change").unwrap();
    assert_eq!(agents.len(), 2);
    let controller = agents.iter().find(|a| a.id == "t16-controller").unwrap();
    assert!(controller.main && controller.subagent);
    assert!(controller.model.is_none());
}

#[test]
fn unsafe_agent_and_workflow_ids_are_rejected_before_writing() {
    let source = TempDir::new().unwrap();
    let target = TempDir::new().unwrap();
    source
        .copy_from(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../examples/opencode-execution"),
            &["**/*"],
        )
        .unwrap();
    let root = Utf8Path::from_path(source.path()).unwrap();
    let target = Utf8Path::from_path(target.path()).unwrap();
    assert!(install_workflow(target, Some(root), "../escape").is_err());
    let definitions = root.join(".unelos/project.yaml");
    let workflow = root.join(".unelos/workflows/page-change.yaml");
    for file in [definitions, workflow] {
        let text = std::fs::read_to_string(&file)
            .unwrap()
            .replace("t16-engineer", "../escape");
        std::fs::write(file, text).unwrap();
    }
    assert!(install_workflow(target, Some(root), "page-change").is_err());
    assert!(!target.join(".unelos/project.yaml").exists());
}
