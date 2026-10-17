use assert_fs::{fixture::PathCopy, TempDir};
use camino::{Utf8Path, Utf8PathBuf};
use std::fs;
use unelos_core::{list_workflows, workflow_summaries};

fn project_copy() -> (TempDir, Utf8PathBuf) {
    let temp = TempDir::new().unwrap();
    let root = Utf8Path::from_path(temp.path()).unwrap().to_owned();
    temp.copy_from(
        Utf8Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/project"),
        &["**/*"],
    )
    .unwrap();
    (temp, root)
}

const MISSING_AGENT: &str = "name: broken\nstart: a\nnodes:\n  a:\n    task: create-plan\n    agent: ghost\n    next: done\n  done:\n    type: end\n";

#[test]
fn summaries_sort_filenames_not_stems() {
    let (_temp, root) = project_copy();
    let source = root.join(".unelos/workflows/implement-feature.yaml");
    for name in ["a", "a.b"] {
        let yaml = fs::read_to_string(&source)
            .unwrap()
            .replace("name: implement-feature", &format!("name: {name}"));
        fs::write(root.join(format!(".unelos/workflows/{name}.yaml")), yaml).unwrap();
    }
    let rows = workflow_summaries(&root).unwrap();
    assert_eq!((&*rows[0].file, &*rows[1].file), ("a.b.yaml", "a.yaml"));
    let names = list_workflows(&root).unwrap();
    assert_eq!((&*names[0], &*names[1]), ("a", "a.b"));
}

#[test]
fn valid_rows_count_steps_without_end_and_sort_by_file() {
    let (_temp, root) = project_copy();
    let rows = workflow_summaries(&root).unwrap();
    let files: Vec<_> = rows.iter().map(|r| r.file.as_str()).collect();
    assert_eq!(
        files,
        [
            "completion-loop.yaml",
            "implement-feature.yaml",
            "research-feature.yaml"
        ]
    );
    let implement = &rows[1];
    assert_eq!(implement.name, "implement-feature");
    assert_eq!(implement.steps, Some(4));
    assert_eq!(implement.problem, None);
}

#[test]
fn invalid_workflow_becomes_problem_row_and_keeps_others() {
    let (_temp, root) = project_copy();
    fs::write(root.join(".unelos/workflows/broken.yaml"), MISSING_AGENT).unwrap();
    let rows = workflow_summaries(&root).unwrap();
    assert_eq!(rows.len(), 4);
    let broken = rows.iter().find(|r| r.name == "broken").unwrap();
    assert_eq!(broken.steps, Some(1));
    let problem = broken.problem.as_deref().unwrap();
    assert!(problem.contains("ghost"), "{problem}");
    assert!(
        !problem.contains(root.as_str()),
        "problem should not repeat the file path: {problem}"
    );
    assert!(rows
        .iter()
        .filter(|r| r.name != "broken")
        .all(|r| r.problem.is_none()));
    assert!(
        list_workflows(&root).is_err(),
        "list_workflows keeps fail-fast behavior"
    );
}

#[test]
fn unparseable_workflow_becomes_problem_row() {
    let (_temp, root) = project_copy();
    fs::write(
        root.join(".unelos/workflows/garbage.yaml"),
        "name: [unclosed",
    )
    .unwrap();
    let rows = workflow_summaries(&root).unwrap();
    let garbage = rows.iter().find(|r| r.name == "garbage").unwrap();
    assert_eq!((garbage.steps, garbage.description.as_deref()), (None, None));
    assert!(garbage.problem.is_some());
}

#[test]
fn missing_workflows_dir_is_empty() {
    let (_temp, root) = project_copy();
    fs::remove_dir_all(root.join(".unelos/workflows")).unwrap();
    assert!(workflow_summaries(&root).unwrap().is_empty());
}

#[test]
fn missing_project_yaml_is_error() {
    let (_temp, root) = project_copy();
    fs::remove_file(root.join(".unelos/project.yaml")).unwrap();
    assert!(workflow_summaries(&root).is_err());
}
