use assert_fs::{fixture::PathCopy, TempDir};
use camino::Utf8Path;
use unelos_core::*;

#[test]
fn export_deduplicates_agents_and_combines_native_modes() {
    let source = TempDir::new().unwrap();
    source
        .copy_from(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../examples/simple-plan-execute-review"),
            &["**/*"],
        )
        .unwrap();
    let root = Utf8Path::from_path(source.path()).unwrap();
    let workflow = root.join("workflows/simple-plan-execute-review.yaml");
    let text = std::fs::read_to_string(&workflow)
        .unwrap()
        .replace("agent: simple-implementer", "agent: simple-controller");
    std::fs::write(workflow, text).unwrap();
    let agents = export_workflow_agents(root, "simple-plan-execute-review").unwrap();
    assert_eq!(agents.len(), 2);
    let controller = agents.iter().find(|a| a.id == "simple-controller").unwrap();
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
                .join("../../examples/simple-plan-execute-review"),
            &["**/*"],
        )
        .unwrap();
    let root = Utf8Path::from_path(source.path()).unwrap();
    let target = Utf8Path::from_path(target.path()).unwrap();
    assert!(install_workflow(target, Some(root), "../escape").is_err());
    let workflow = root.join("workflows/simple-plan-execute-review.yaml");
    let text = std::fs::read_to_string(&workflow)
        .unwrap()
        .replace("simple-implementer", "../escape");
    std::fs::write(workflow, text).unwrap();
    assert!(install_workflow(target, Some(root), "simple-plan-execute-review").is_err());
    assert!(!target.join(".unelos/project.yaml").exists());
}
