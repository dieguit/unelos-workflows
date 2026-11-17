use std::{fs, path::Path};

use assert_fs::TempDir;
use camino::Utf8Path;
use unelos_core::*;

#[test]
fn catalog_sources_are_delivered_with_runtime_paths_kept_separate() {
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/instructions");
    let controller = fs::read_to_string(source.join("controller.md")).unwrap();
    let outputs = fs::read_to_string(source.join("task-outputs.md")).unwrap();
    let result = fs::read_to_string(source.join("worker-result.md")).unwrap();
    let temp = TempDir::new().unwrap();
    let root = Utf8Path::from_path(temp.path()).unwrap();
    install_workflow(
        root,
        Some(
            &Utf8Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures/simple-plan-execute-review"),
        ),
        "simple-plan-execute-review",
    )
    .unwrap();
    let mut project = load_project(root, "simple-plan-execute-review").unwrap();
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
                missing_inputs: vec![],
                run_id: state.run_id.clone(),
                task_id: record.execution_id.clone(),
                node_id: record.node_id.clone(),
                request: state.request.clone(),
                inputs: record.inputs.clone(),
                outputs: record.outputs.clone(),
                feedback: record.feedback.clone(),
            },
            ledger_path: root
                .join(".unelos/runs")
                .join(&state.run_id)
                .join("ledger.md"),
        },
    )
    .unwrap();
    let WorkPacket::Task(task) = work else {
        panic!("expected Task")
    };
    assert_eq!(
        task.packet.context.completion_instructions,
        outputs.trim_end()
    );
    assert_eq!(task.packet.outputs[0].name, "implementation-report.md");
    assert_eq!(
        task.packet.outputs[0].path,
        root.join(".unelos/runs")
            .join(&state.run_id)
            .join("implement/implementation-report.md")
    );
    for path in [
        task.packet.outputs[0].path.clone(),
        r"C:\project\.unelos\runs\catalog-check\implement\implementation-report.md".into(),
    ] {
        let mut task = task.clone();
        task.packet.outputs[0].path = path.clone();
        let package = render_execution_package(
            Some(WorkPacket::Task(task)),
            vec![],
            Some("Change greeting"),
        )
        .unwrap();
        assert_eq!(
            package.controller_instructions,
            format!(
                "{controller}\n{}",
                fs::read_to_string(source.join("controller-delegated.md")).unwrap()
            )
        );
        let prompt = package.worker_prompt.unwrap();
        assert!(prompt.ends_with(
            result
                .replace(
                    "{taskId}",
                    &serde_json::to_string(&record.execution_id).unwrap()
                )
                .replace("{outcome}", "null")
                .trim_end()
        ));
        assert!(prompt.contains(path.as_str()));
    }
}
