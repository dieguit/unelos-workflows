use camino::Utf8PathBuf;
use std::fs;
use unelos_core::{load_project, ProjectLoadError};

#[test]
fn rejects_self_call_as_validation_error() {
    let temp = assert_fs::TempDir::new().unwrap();
    let root = Utf8PathBuf::from_path_buf(temp.path().to_owned()).unwrap();
    fs::create_dir_all(root.join(".unelos/workflows")).unwrap();
    fs::write(root.join(".unelos/project.yaml"), "{}").unwrap();
    fs::write(root.join(".unelos/workflows/a.yaml"), "name: a\nstart: call\nnodes:\n  call: {type: workflow, workflow: a, next: done}\n  done: {type: end}").unwrap();
    assert!(matches!(
        load_project(&root, "a"),
        Err(ProjectLoadError::Validation(_))
    ));
}
