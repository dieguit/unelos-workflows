use assert_fs::TempDir;
use camino::{Utf8Path, Utf8PathBuf};
use std::{fs, process::Command};
use unelos_core::{resolve_skills, SkillResolveError};

fn root(temp: &TempDir) -> Utf8PathBuf {
    Utf8PathBuf::from_path_buf(temp.path().to_owned()).unwrap()
}

fn write_skill(root: &Utf8Path, id: &str, content: impl AsRef<[u8]>) -> Utf8PathBuf {
    let directory = root.join(".unelos").join("skills").join(id);
    fs::create_dir_all(&directory).unwrap();
    fs::write(directory.join("SKILL.md"), content).unwrap();
    directory
}

#[test]
fn selected_skills_preserve_order_full_content_and_resource_locations() {
    let temp = TempDir::new().unwrap();
    let root = root(&temp);
    let text = "---\r\nname: verification\r\n---\r\n\r\nRead resources/check.txt. 🦀\r\n";
    let directory = write_skill(&root, "verification", text);
    fs::create_dir(directory.join("resources")).unwrap();
    fs::write(
        directory.join("resources").join("check.txt"),
        "resource, not injected",
    )
    .unwrap();
    write_skill(&root, "rust", "Rust instructions\n");
    let selected = resolve_skills(&root, &["verification", "rust", "verification"]).unwrap();
    assert_eq!(
        selected
            .iter()
            .map(|skill| skill.id.as_str())
            .collect::<Vec<_>>(),
        ["verification", "rust"]
    );
    assert_eq!(selected[0].content, text);
    assert_eq!(selected[1].content, "Rust instructions\n");
    assert_eq!(selected[0].directory, directory);
    assert_eq!(
        selected[1].directory,
        root.join(".unelos").join("skills").join("rust")
    );
    assert!(selected[0].directory.is_absolute());
    assert_eq!(
        fs::read_to_string(selected[0].directory.join("resources").join("check.txt")).unwrap(),
        "resource, not injected"
    );
    write_skill(&root, "unused", [0xff]);
    assert_eq!(
        resolve_skills(&root, &["verification", "rust", "verification"]).unwrap(),
        selected
    );
    assert!(resolve_skills(&root, &[] as &[&str]).unwrap().is_empty());
}

#[test]
fn missing_selected_skill_reports_id_and_expected_path() {
    let temp = TempDir::new().unwrap();
    let root = root(&temp);
    write_skill(&root, "rust", "Rust instructions");
    // Match native join separators when checking the rendered diagnostic on Windows.
    let expected = root
        .join(".unelos")
        .join("skills")
        .join("missing")
        .join("SKILL.md");
    let error = resolve_skills(&root, &["rust", "missing"]).unwrap_err();
    assert!(error.to_string().contains("missing"));
    assert!(error.to_string().contains(expected.as_str()));
    assert!(
        matches!(error, SkillResolveError::Missing { id, path, .. } if id == "missing" && path == expected)
    );
}

#[test]
fn unreadable_selected_skill_reports_id_and_path() {
    let temp = TempDir::new().unwrap();
    let root = root(&temp);
    let directory = write_skill(&root, "broken", [0xff]);
    let expected = directory.join("SKILL.md");
    for is_directory in [false, true] {
        if is_directory {
            fs::remove_file(&expected).unwrap();
            fs::create_dir(&expected).unwrap();
        }
        let error = resolve_skills(&root, &["broken"]).unwrap_err();
        assert!(error.to_string().contains("broken"));
        assert!(error.to_string().contains(expected.as_str()));
        assert!(
            matches!(error, SkillResolveError::Read { id, path, .. } if id == "broken" && path == expected)
        );
    }
}

#[test]
fn rejects_path_shaped_skill_ids() {
    let temp = TempDir::new().unwrap();
    let root = root(&temp);
    for id in [
        "",
        ".",
        "..",
        "../outside",
        "/absolute",
        "nested/name",
        "nested\\name",
        "C:\\outside",
        "C:outside",
        "bad\0id",
    ] {
        assert!(
            matches!(resolve_skills(&root, &[id]), Err(SkillResolveError::InvalidId { id: rejected }) if rejected == id)
        );
    }
    write_skill(&root, "café skill", "Unicode and spaces are valid");
    assert_eq!(
        resolve_skills(&root, &[String::from("café skill")]).unwrap()[0].content,
        "Unicode and spaces are valid"
    );
}

#[test]
fn relative_root_is_resolved_once_from_the_calling_directory() {
    if let Ok(parent) = std::env::var("UNELOS_T03_TEST_PARENT") {
        let parent = Utf8Path::new(&parent);
        let cwd = Utf8PathBuf::from_path_buf(std::env::current_dir().unwrap()).unwrap();
        let relative = resolve_skills(Utf8Path::new("project"), &["rust"]).unwrap();
        assert_eq!(
            relative[0].directory,
            cwd.join("project")
                .join(".unelos")
                .join("skills")
                .join("rust")
        );
        assert_eq!(relative[0].content, "Rust instructions");
        let absolute = resolve_skills(&parent.join("project"), &["rust"]).unwrap();
        assert_eq!(absolute[0].content, relative[0].content);
        return;
    }
    let temp = TempDir::new().unwrap();
    write_skill(&root(&temp).join("project"), "rust", "Rust instructions");
    let output = Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "relative_root_is_resolved_once_from_the_calling_directory",
            "--nocapture",
        ])
        .current_dir(temp.path())
        .env("UNELOS_T03_TEST_PARENT", temp.path())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
