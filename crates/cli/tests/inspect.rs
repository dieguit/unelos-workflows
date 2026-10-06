use assert_cmd::cargo::cargo_bin_cmd;
use serde_json::{json, Value};
use std::{fs, path::Path};

fn command() -> assert_cmd::Command {
    let mut command = cargo_bin_cmd!("unelos");
    command.env_clear();
    command
}

fn fixture() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../core/tests/fixtures/project")
}

#[test]
fn inspect_normalizes_the_existing_fixture() {
    let output = command()
        .arg("--project")
        .arg(fixture())
        .args(["inspect", "implement-feature", "--json"])
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    assert!(output.stderr.is_empty());
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["name"], "implement-feature");
    assert_eq!(value["start"], "plan");
    assert_eq!(value["nodes"].as_array().unwrap().len(), 5);
    let plan = value["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|node| node["id"] == "plan")
        .unwrap();
    assert_eq!(plan["kind"], "task");
    assert_eq!(plan["execution"], "main");
    assert_eq!(plan["inputs"], json!([]));
    assert_eq!(value["edges"].as_array().unwrap().len(), 6);
}

#[test]
fn inspect_shipped_example_matches_the_original_fixture() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let example = root.join("examples/manual-project");
    let output = command()
        .arg("--project")
        .arg(example)
        .args(["inspect", "implement-feature", "--json"])
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    let actual: Value = serde_json::from_slice(&output.stdout).unwrap();
    let expected: Value = serde_json::from_slice(&inspect(&fixture(), true).stdout).unwrap();
    assert_eq!(actual, expected);
}

#[test]
fn inspect_emits_full_normalized_workflow_and_is_read_only() {
    let temp = assert_fs::TempDir::new().unwrap();
    let root = temp.path().join("project with spaces");
    fs::create_dir_all(&root).unwrap();
    copy_tree(&fixture(), &root);
    let before = snapshot(&root);
    let compact = inspect(&root, true);
    let pretty = inspect(&root, false);
    assert!(
        String::from_utf8(pretty.stdout.clone())
            .unwrap()
            .lines()
            .count()
            > 1
    );
    assert_eq!(
        serde_json::from_slice::<Value>(&compact.stdout).unwrap(),
        serde_json::from_slice::<Value>(&pretty.stdout).unwrap()
    );
    let value: Value = serde_json::from_slice(&compact.stdout).unwrap();
    assert_eq!(
        value["nodes"],
        json!([
            {"id":"approve-plan", "kind":"human-gate", "inputs":["plan.md"], "next":{"approved":"implement", "rejected":"plan"}},
            {"id":"done", "kind":"end"},
            {"id":"implement", "kind":"task", "task":"implement-plan", "agent":"engineer", "execution":"subagent", "inputs":["plan.md"], "outputs":["implementation-report.md"], "next":"review"},
            {"id":"plan", "kind":"task", "task":"create-plan", "agent":"planner", "execution":"main", "inputs":[], "outputs":["plan.md"], "next":"approve-plan"},
            {"id":"review", "kind":"task", "task":"review-implementation", "agent":"reviewer", "execution":"subagent", "inputs":["plan.md", "implementation-report.md"], "outputs":["review.md"], "next":{"approved":"done", "changes_requested":"implement"}}
        ])
    );
    assert_eq!(
        value["edges"],
        json!([
            {"from":"approve-plan", "to":"implement", "outcome":"approved"},
            {"from":"approve-plan", "to":"plan", "outcome":"rejected"},
            {"from":"implement", "to":"review", "outcome":null},
            {"from":"plan", "to":"approve-plan", "outcome":null},
            {"from":"review", "to":"done", "outcome":"approved"},
            {"from":"review", "to":"implement", "outcome":"changes_requested"}
        ])
    );
    assert_eq!(snapshot(&root), before);
    assert_eq!(compact.stdout, inspect(&root, true).stdout);
    assert!(value.to_string().find("instructions").is_none());
    assert!(value.to_string().find("skills").is_none());
    assert!(value.to_string().find("verification").is_none());
}

#[test]
fn inspect_supports_default_project_and_reports_invalid_project() {
    let output = command()
        .current_dir(fixture())
        .args(["inspect", "implement-feature", "--json"])
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    for json_mode in [true, false] {
        let temp = assert_fs::TempDir::new().unwrap();
        let root = temp.path().join("invalid");
        fs::create_dir_all(root.join(".unelos/workflows")).unwrap();
        fs::write(root.join(".unelos/project.yaml"), project_yaml()).unwrap();
        fs::write(
            root.join(".unelos/workflows/implement-feature.yaml"),
            workflow_yaml().replace("next: review", "next: missing-node"),
        )
        .unwrap();
        let before = snapshot(&root);
        let output = inspect(&root, json_mode);
        assert_eq!(output.status.code(), Some(2));
        assert!(output.stdout.is_empty());
        let stderr = String::from_utf8(output.stderr).unwrap();
        assert!(stderr.contains("implement-feature.yaml"), "{stderr}");
        assert!(stderr.contains("nodes.implement.next"), "{stderr}");
        if json_mode {
            assert!(stderr.contains("invalid_project"), "{stderr}");
        } else {
            assert!(stderr.contains("--project"), "{stderr}");
        }
        assert_eq!(snapshot(&root), before);
    }
}

#[test]
fn inspect_reports_missing_sources_malformed_yaml_and_rejects_traversal() {
    let temp = assert_fs::TempDir::new().unwrap();
    for (root_files, expected) in [
        (vec![], ".unelos/project.yaml"),
        (
            vec![(".unelos/project.yaml", project_yaml())],
            "implement-feature.yaml",
        ),
        (
            vec![(".unelos/project.yaml", "bad: [".into())],
            ".unelos/project.yaml",
        ),
    ] {
        let root = temp.path().join(format!("case-{}", expected.len()));
        fs::create_dir_all(root.join(".unelos/workflows")).unwrap();
        for (path, contents) in root_files {
            fs::write(root.join(path), contents).unwrap();
        }
        let output = inspect(&root, true);
        assert_eq!(output.status.code(), Some(2));
        assert!(output.stdout.is_empty());
        assert!(String::from_utf8(output.stderr).unwrap().contains(expected));
    }
    let output = command()
        .arg("--project")
        .arg(fixture())
        .args(["inspect", "../implement-feature", "--json"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    assert!(!String::from_utf8_lossy(&output.stderr).contains("panicked"));
}

#[test]
fn inspect_serializes_parallel_workers_and_gate_and_end() {
    let temp = assert_fs::TempDir::new().unwrap();
    let root = temp.path();
    copy_tree(&fixture(), root);
    let workflow = workflow_yaml()
        .replace("outputs: [implementation-report.md]\n    next: review", "outputs: [implementation-report.md]\n    next: parallel-check")
        .replace("  review:\n", "  parallel-check:\n    type: parallel\n    tasks:\n      - task: implement-plan\n        agent: engineer\n      - task: create-plan\n        agent: planner\n    inputs: [implementation-report.md]\n    outputs: [checks.md]\n    next: review\n  review:\n");
    assert!(!workflow.contains("\r\n"));
    fs::write(
        root.join(".unelos/workflows/implement-feature.yaml"),
        workflow,
    )
    .unwrap();
    let value: Value = serde_json::from_slice(&inspect(root, true).stdout).unwrap();
    let parallel = value["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|node| node["id"] == "parallel-check")
        .unwrap();
    assert_eq!(
        parallel["tasks"],
        json!([{"task":"implement-plan","agent":"engineer"}, {"task":"create-plan","agent":"planner"}])
    );
    assert_eq!(parallel["inputs"], json!(["implementation-report.md"]));
    assert_eq!(parallel["outputs"], json!(["checks.md"]));
    assert_eq!(parallel["next"], "review");
    assert_eq!(
        value["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .find(|node| node["id"] == "done")
            .unwrap(),
        &json!({"id":"done", "kind":"end"})
    );
}

#[test]
fn inspect_resolves_one_task_context_deterministically_without_runtime_state() {
    let temp = assert_fs::TempDir::new().unwrap();
    let root = temp.path().join("project with spaces");
    fs::create_dir_all(&root).unwrap();
    copy_tree(&fixture(), &root);
    let before = snapshot(&root);

    let compact = inspect_node(&root, "implement", true);
    assert!(compact.status.success(), "{compact:?}");
    assert!(compact.stderr.is_empty());
    let pretty = inspect_node(&root, "implement", false);
    assert!(pretty.status.success(), "{pretty:?}");
    assert!(String::from_utf8_lossy(&pretty.stdout).lines().count() > 1);
    assert_eq!(
        serde_json::from_slice::<Value>(&compact.stdout).unwrap(),
        serde_json::from_slice::<Value>(&pretty.stdout).unwrap()
    );
    assert_eq!(
        inspect_node(&root, "implement", true).stdout,
        compact.stdout
    );

    let value: Value = serde_json::from_slice(&compact.stdout).unwrap();
    assert_eq!(value["kind"], "task");
    assert_eq!(value["nodeId"], "implement");
    assert_eq!(value["taskDefinition"], "implement-plan");
    assert_eq!(value["agent"], "engineer");
    assert_eq!(value["execution"], "subagent");
    assert_eq!(
        value["taskInstructions"],
        "Implement the supplied plan and report verification."
    );
    assert_eq!(
        value["agentInstructions"],
        "Follow existing project conventions."
    );
    assert_eq!(value["inputs"], json!(["plan.md"]));
    assert_eq!(value["outputs"], json!(["implementation-report.md"]));
    assert_eq!(value["model"], "example-provider/example-model");
    assert_eq!(value["mcps"], json!(["github"]));
    assert_eq!(value["tools"], json!(["read-file", "run-command"]));
    assert_eq!(value["skills"][0]["id"], "verification");
    assert_eq!(value["skills"][1]["id"], "rust");
    for (index, skill) in ["verification", "rust"].iter().enumerate() {
        assert_eq!(
            value["skills"][index]["content"],
            fs::read_to_string(fixture().join(format!(".unelos/skills/{skill}/SKILL.md"))).unwrap()
        );
    }
    assert_eq!(
        value["skills"][0]["directory"],
        root.join(".unelos/skills/verification")
            .to_string_lossy()
            .as_ref()
    );
    assert_eq!(
        value["skills"][1]["directory"],
        root.join(".unelos/skills/rust").to_string_lossy().as_ref()
    );
    for field in ["runId", "taskId", "request", "feedback"] {
        assert!(value.get(field).is_none(), "unexpected {field}");
    }
    assert!(!root.join(".unelos/runs").exists());
    assert!(!root.join(".unelos/active-run").exists());
    assert!(!root.join(".unelos/active-run.json").exists());
    assert_eq!(snapshot(&root), before);
}

#[test]
fn inspect_node_reads_only_selected_skills_in_order_and_reports_missing_skill() {
    let temp = assert_fs::TempDir::new().unwrap();
    let root = temp.path();
    copy_tree(&fixture(), root);
    let project_path = root.join(".unelos/project.yaml");
    let project = fs::read_to_string(&project_path)
        .unwrap()
        .replace("\r\n", "\n");
    fs::write(
        &project_path,
        project.replace("skills: [rust]", "skills: [rust, verification]"),
    )
    .unwrap();
    let selected_path = root.join(".unelos/skills/verification/SKILL.md");
    let selected = fs::read_to_string(&selected_path).unwrap();
    let selected = format!("{selected}\nSELECTED_SKILL_MARKER\n");
    fs::write(&selected_path, &selected).unwrap();
    let unrelated = root.join(".unelos/skills/unrelated");
    fs::create_dir_all(&unrelated).unwrap();
    fs::write(unrelated.join("SKILL.md"), "UNRELATED_SKILL_MARKER\n").unwrap();

    let output = inspect_node(root, "implement", true);
    assert!(output.status.success(), "{output:?}");
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    let skills = value["skills"].as_array().unwrap();
    assert_eq!(
        skills
            .iter()
            .map(|skill| skill["id"].as_str().unwrap())
            .collect::<Vec<_>>(),
        ["verification", "rust"]
    );
    assert_eq!(skills[0]["content"], selected);
    assert_eq!(
        skills[0]["directory"],
        root.join(".unelos/skills/verification")
            .to_string_lossy()
            .as_ref()
    );
    assert_eq!(
        skills[1]["directory"],
        root.join(".unelos/skills/rust").to_string_lossy().as_ref()
    );
    let serialized = value.to_string();
    assert_eq!(serialized.matches("SELECTED_SKILL_MARKER").count(), 1);
    assert!(!serialized.contains("UNRELATED_SKILL_MARKER"));

    fs::rename(
        &selected_path,
        selected_path.with_file_name("SKILL.md.disabled"),
    )
    .unwrap();
    for json_mode in [true, false] {
        let missing = inspect_node(root, "implement", json_mode);
        assert_eq!(missing.status.code(), Some(2));
        assert!(missing.stdout.is_empty());
        let stderr = String::from_utf8_lossy(&missing.stderr);
        assert!(stderr.contains("verification"), "{stderr}");
        assert!(stderr.contains("SKILL.md"), "{stderr}");
        if json_mode {
            let diagnostic: Value = serde_json::from_slice(&missing.stderr).unwrap();
            assert_eq!(diagnostic["error"]["code"], "context_resolution_failed");
        } else {
            assert!(stderr.contains("selected Skill"), "{stderr}");
        }
    }
    assert!(
        inspect(root, true).status.success(),
        "whole-Workflow inspection reads no Skills"
    );
}

#[test]
fn inspect_node_reports_unknown_and_unsupported_nodes_without_panicking() {
    let temp = assert_fs::TempDir::new().unwrap();
    let root = temp.path();
    copy_tree(&fixture(), root);
    for node in ["missing", "approve-plan", "done"] {
        let output = inspect_node(root, node, true);
        assert_eq!(output.status.code(), Some(2), "{output:?}");
        assert!(output.stdout.is_empty());
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(!stderr.contains("panicked"), "{stderr}");
        assert!(stderr.contains("context_resolution_failed"), "{stderr}");
        assert!(stderr.contains(node), "{stderr}");
    }

    let workflow_path = root.join(".unelos/workflows/implement-feature.yaml");
    let parallel = workflow_yaml().replace(
        "    next: review\n  review:",
        "    next: parallel-check\n  parallel-check:\n    type: parallel\n    tasks:\n      - task: implement-plan\n        agent: engineer\n    inputs: [plan.md]\n    outputs: [parallel.md]\n    next: review\n  review:",
    );
    fs::write(workflow_path, parallel).unwrap();
    let output = inspect_node(root, "parallel-check", true);
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("parallel-check"), "{stderr}");
    assert!(stderr.contains("ordinary Task"), "{stderr}");
}

fn inspect_node(root: &Path, node: &str, json_mode: bool) -> std::process::Output {
    let mut command = command();
    command
        .arg("--project")
        .arg(root)
        .args(["inspect", "implement-feature", "--node", node]);
    if json_mode {
        command.arg("--json");
    }
    command.output().unwrap()
}

fn inspect(root: &Path, json_mode: bool) -> std::process::Output {
    let mut command = command();
    command
        .arg("--project")
        .arg(root)
        .args(["inspect", "implement-feature"]);
    if json_mode {
        command.arg("--json");
    }
    command.output().unwrap()
}

fn snapshot(root: &Path) -> Vec<(String, Vec<u8>)> {
    fn walk(root: &Path, path: &Path, entries: &mut Vec<(String, Vec<u8>)>) {
        for entry in fs::read_dir(path).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.is_dir() {
                walk(root, &path, entries);
            } else {
                entries.push((
                    path.strip_prefix(root)
                        .unwrap()
                        .to_string_lossy()
                        .into_owned(),
                    fs::read(path).unwrap(),
                ));
            }
        }
    }
    let mut entries = Vec::new();
    walk(root, root, &mut entries);
    entries.sort_by(|a, b| a.0.cmp(&b.0));
    entries
}

fn copy_tree(source: &Path, destination: &Path) {
    for entry in fs::read_dir(source).unwrap() {
        let entry = entry.unwrap();
        let target = destination.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            fs::create_dir_all(&target).unwrap();
            copy_tree(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), target).unwrap();
        }
    }
}

fn project_yaml() -> String {
    fs::read_to_string(fixture().join(".unelos/project.yaml")).unwrap()
}

fn workflow_yaml() -> String {
    fs::read_to_string(fixture().join(".unelos/workflows/implement-feature.yaml"))
        .unwrap()
        .replace("\r\n", "\n")
}
