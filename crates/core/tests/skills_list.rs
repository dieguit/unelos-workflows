use assert_fs::TempDir;
use camino::{Utf8Path, Utf8PathBuf};
use std::fs;
use unelos_core::skill_summaries;

fn write(root: &Utf8Path, relative: &str, text: &str) {
    let path = root.join(relative);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

fn project() -> (TempDir, Utf8PathBuf) {
    let temp = TempDir::new().unwrap();
    let root = Utf8PathBuf::from_path_buf(temp.path().to_owned()).unwrap();
    write(&root, ".unelos/project.yaml", "{}\n");
    (temp, root)
}

#[test]
fn summaries_read_frontmatter_description_and_isolate_broken_skills() {
    let (_temp, root) = project();
    write(
        &root,
        ".unelos/skills/rust/SKILL.md",
        "---\nname: Rust conventions\ndescription: Follow Rust conventions.\nother: kept\n---\n# Rust\n",
    );
    write(
        &root,
        ".unelos/skills/crlf/SKILL.md",
        "---\r\ndescription: Windows file\r\n---\r\nBody\r\n",
    );
    write(
        &root,
        ".unelos/skills/plain/SKILL.md",
        "# Plain\nNo frontmatter.\n",
    );
    write(
        &root,
        ".unelos/skills/empty-front/SKILL.md",
        "---\n---\nBody\n",
    );
    write(&root, ".unelos/skills/my.skill/SKILL.md", "Dotted id\n");
    write(
        &root,
        ".unelos/skills/bad-yaml/SKILL.md",
        "---\ndescription: [\n---\n",
    );
    write(
        &root,
        ".unelos/skills/bad-type/SKILL.md",
        "---\ndescription: [a]\n---\n",
    );
    fs::create_dir_all(root.join(".unelos/skills/missing")).unwrap();
    write(&root, ".unelos/skills/README.md", "not a skill");

    let result = skill_summaries(&root).unwrap();
    let rows: Vec<_> = result
        .iter()
        .map(|s| {
            (
                s.name.as_str(),
                s.description.as_deref(),
                s.problem.is_some(),
            )
        })
        .collect();
    assert_eq!(
        rows,
        [
            ("bad-type", None, true),
            ("bad-yaml", None, true),
            ("crlf", Some("Windows file"), false),
            ("empty-front", None, false),
            ("missing", None, true),
            ("my.skill", None, false),
            ("plain", None, false),
            ("rust", Some("Follow Rust conventions."), false),
        ]
    );
    assert!(result
        .iter()
        .filter_map(|s| s.problem.as_deref())
        .all(|message| !message.is_empty()));
}

#[test]
fn missing_skills_directory_is_empty() {
    let (_temp, root) = project();
    let result = skill_summaries(&root).unwrap();
    assert!(result.is_empty());
}

#[test]
fn missing_project_settings_fail() {
    let temp = TempDir::new().unwrap();
    let root = Utf8PathBuf::from_path_buf(temp.path().to_owned()).unwrap();
    assert!(skill_summaries(&root).is_err());
}
