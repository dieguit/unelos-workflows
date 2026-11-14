use assert_fs::TempDir;
use camino::Utf8Path;
use std::fs;
use unelos_core::project_references;

#[test]
fn project_references_direct_sorted_deduplicated_and_tolerant() {
    let temp = TempDir::new().unwrap();
    let root = Utf8Path::from_path(temp.path()).unwrap();
    for folder in ["agents/helper", "tasks/check", "workflows"] {
        fs::create_dir_all(root.join(format!(".unelos/{folder}"))).unwrap();
    }
    for (path, text) in [
        ("project.yaml", "{}"),
        ("agents/helper/agent.yaml", "instructions: Help\nskills: [shared, shared]\nmcps: [global/search]\n"),
        ("tasks/check/task.yaml", "instructions: Check\nskills: [shared]\n"),
        ("workflows/broken.yaml", "nodes: ["),
        ("workflows/z.yaml", "name: Zulu\nstart: a\nnodes:\n  a:\n    agent: helper\n    task: check\n    next: b\n  b:\n    type: parallel\n    tasks:\n      - agent: helper\n        task: check\n      - agent: global/helper\n        taskDefinition:\n          instructions: Inline\n          skills: [shared, global/plan]\n    next: done\n  done:\n    type: end\n"),
        ("workflows/a.yaml", "name: Alpha\nstart: a\nnodes:\n  a:\n    agent: helper\n    task: global/check\n    next: done\n  done:\n    type: end\n"),
    ] {
        fs::write(root.join(format!(".unelos/{path}")), text).unwrap();
    }
    let references = project_references(root).unwrap();
    assert_eq!(
        references.agents["helper"]
            .iter()
            .map(|r| r.name.as_str())
            .collect::<Vec<_>>(),
        ["Alpha", "Zulu"]
    );
    assert_eq!(references.tasks["check"].len(), 1);
    assert_eq!(references.tasks["global/check"][0].id, "a");
    assert_eq!(references.agents["global/helper"][0].kind, "workflow");
    assert_eq!(references.skills["global/plan"][0].id, "z");
    assert_eq!(
        references.skills["shared"]
            .iter()
            .map(|r| r.kind.as_str())
            .collect::<Vec<_>>(),
        ["workflow", "task", "agent"]
    );
    assert_eq!(references.mcps["global/search"][0].id, "helper");
}
