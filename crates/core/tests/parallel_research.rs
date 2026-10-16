use assert_fs::TempDir;
use camino::{Utf8Path, Utf8PathBuf};
use unelos_core::*;

const WORKFLOW: &str = "parallel-research";

fn fixture() -> (TempDir, Utf8PathBuf) {
    let temp = TempDir::new().unwrap();
    let root = Utf8Path::from_path(temp.path()).unwrap().to_owned();
    install_workflow(
        &root,
        Some(&Utf8Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/parallel-research")),
        WORKFLOW,
    )
    .unwrap();
    (temp, root)
}

#[test]
fn example_installs_with_two_isolated_workers() {
    let (_temp, root) = fixture();
    let project = load_project(&root, WORKFLOW).unwrap();
    validate_project(&project).unwrap();
    let value = serde_json::to_value(inspect_node(&project, "parallel-research").unwrap()).unwrap();
    let workers = value["workers"].as_array().unwrap();
    assert_eq!(workers.len(), 2);
    assert_eq!(workers[0]["agent"], "backend-researcher");
    assert_eq!(workers[1]["agent"], "frontend-researcher");
    let skills = |i: usize| workers[i]["skills"].to_string();
    assert!(skills(0).contains("backend-research") && !skills(0).contains("frontend-research"));
    assert!(skills(1).contains("frontend-research") && !skills(1).contains("backend-research"));
}
