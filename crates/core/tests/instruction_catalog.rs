use std::{fs, path::Path};

use assert_fs::{fixture::PathCopy, TempDir};
use camino::Utf8Path;
use unelos_core::*;

#[test]
fn catalog_sources_are_delivered_with_runtime_paths_kept_separate() {
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/instructions");
    let controller = fs::read_to_string(source.join("controller.md")).unwrap();
    let outputs = fs::read_to_string(source.join("task-outputs.md")).unwrap();
    let result = fs::read_to_string(source.join("worker-result.md")).unwrap();
    let temp = TempDir::new().unwrap();
    temp.copy_from(
        Utf8Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/opencode-execution"),
        &["**/*"],
    )
    .unwrap();
    let root = Utf8Path::from_path(temp.path()).unwrap();
    let mut project = load_project(root, "page-change").unwrap();
    project.workflow.start = "implement".into();
    let NodeDefinition::Task(node) = project.workflow.nodes.get_mut("implement").unwrap() else {
        panic!("expected Task")
    };
    node.inputs.clear();
    let state = start(&project, "catalog-check", "Change greeting").unwrap();
    let CurrentWork::Task(record) = &state.current else {
        panic!("expected Task")
    };
    let work = build_work_packet(
        &project,
        WorkRuntimeInput::Task {
            input: TaskRuntimeInput {
                run_id: state.run_id.clone(),
                task_id: record.execution_id.clone(),
                node_id: record.node_id.clone(),
                request: state.request.clone(),
                inputs: record.inputs.clone(),
                outputs: record.outputs.clone(),
                feedback: record.feedback.clone(),
            },
            ledger_path: root.join(".unelos/runs/catalog-check/ledger.md"),
        },
    )
    .unwrap();
    let package =
        render_execution_package(Some(work), vec![], Some("Change greeting"), None).unwrap();
    assert_eq!(package.controller_instructions, controller);
    let WorkPacket::Task(task) = package.packet.unwrap() else {
        panic!("expected Task")
    };
    assert_eq!(
        task.packet.context.completion_instructions,
        outputs.trim_end()
    );
    assert_eq!(task.packet.outputs[0].name, "implementation-report.md");
    assert_eq!(
        task.packet.outputs[0].path,
        root.join(".unelos/runs/catalog-check/implement/implementation-report.md")
    );
    let prompt = package.worker_prompt.unwrap();
    assert!(prompt.ends_with(result.trim_end()));
    assert!(prompt.contains(task.packet.outputs[0].path.as_str()));
}
