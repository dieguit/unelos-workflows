use assert_fs::TempDir;
use camino::{Utf8Path, Utf8PathBuf};
use unelos_core::*;

const WORKFLOW: &str = "superpowers-dev";

fn fixture() -> (TempDir, Utf8PathBuf) {
    let temp = TempDir::new().unwrap();
    let root = Utf8Path::from_path(temp.path()).unwrap().to_owned();
    install_workflow(
        &root,
        Some(&Utf8Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/superpowers-dev")),
        WORKFLOW,
    )
    .unwrap();
    (temp, root)
}

fn node(state: &RunState) -> String {
    match &state.current {
        CurrentWork::Task(record) | CurrentWork::HumanGate(record) => record.node_id.clone(),
        CurrentWork::Finished { node_id } => node_id.clone(),
        _ => panic!("unexpected work"),
    }
}

fn names(artifacts: &[ArtifactReference]) -> Vec<String> {
    artifacts.iter().map(|a| a.name.clone()).collect()
}

fn task_inputs(state: &RunState) -> Vec<String> {
    let CurrentWork::Task(record) = &state.current else {
        panic!("expected task")
    };
    names(&record.inputs)
}

fn task_outputs(state: &RunState) -> Vec<Utf8PathBuf> {
    let CurrentWork::Task(record) = &state.current else {
        panic!("expected task")
    };
    record.outputs.iter().map(|a| a.path.clone()).collect()
}

fn feedback(state: &RunState) -> (String, Vec<String>) {
    let CurrentWork::Task(record) = &state.current else {
        panic!("expected task")
    };
    let feedback = record.feedback.as_ref().expect("feedback");
    (feedback.source_node_id.clone(), names(&feedback.artifacts))
}

fn complete(root: &Utf8Path, state: &RunState, outcome: Option<&str>) -> RunState {
    let CurrentWork::Task(record) = &state.current else {
        panic!("expected task at {}", node(state))
    };
    for output in &record.outputs {
        std::fs::write(
            &output.path,
            format!("{} {} report\n", record.node_id, record.execution_id),
        )
        .unwrap();
    }
    complete_node(root, &state.run_id, &record.node_id, outcome).unwrap()
}

#[test]
fn every_route_binds_inputs_and_delivers_loop_state_through_feedback() {
    let (_temp, root) = fixture();
    let s = start_workflow(&root, WORKFLOW, Some("walk")).unwrap();
    std::fs::write(
        root.join(".unelos/runs/r-000001/ledger.md"),
        "## Brief\nWalk every route\n",
    )
    .unwrap();
    assert_eq!(node(&s), "brainstorm");
    let s = complete(&root, &s, Some("check"));
    assert_eq!(node(&s), "spec-check");
    assert_eq!(task_inputs(&s), ["spec.md"]);
    let s = complete(&root, &s, None);
    assert_eq!(node(&s), "brainstorm");
    assert_eq!(
        feedback(&s),
        ("spec-check".into(), vec!["builder-questions.md".into()])
    );
    let s = complete(&root, &s, Some("ready"));
    assert_eq!(node(&s), "approve-spec");
    let s = reject_node(&root, &s.run_id, "approve-spec", "Clarify scope").unwrap();
    assert_eq!(node(&s), "brainstorm");
    let s = complete(&root, &s, Some("ready"));
    let s = approve_node(&root, &s.run_id, "approve-spec").unwrap();
    assert_eq!(node(&s), "write-plan");
    let s = complete(&root, &s, None);
    assert_eq!(node(&s), "approve-plan");
    let s = reject_node(&root, &s.run_id, "approve-plan", "Split task 2").unwrap();
    assert_eq!(node(&s), "write-plan");
    let s = complete(&root, &s, None);
    let s = approve_node(&root, &s.run_id, "approve-plan").unwrap();
    assert_eq!(node(&s), "setup-worktree");
    let s = complete(&root, &s, None);

    assert_eq!(node(&s), "implement-task");
    assert_eq!(task_inputs(&s), ["spec.md", "plan.md", "worktree.md"]);
    assert_eq!(
        feedback(&s),
        ("setup-worktree".into(), vec!["worktree.md".into()])
    );
    let report = task_outputs(&s);
    let s = complete(&root, &s, None);
    assert_eq!(node(&s), "review-task");
    let s = complete(&root, &s, None);
    assert_eq!(node(&s), "advance");
    let s = complete(&root, &s, Some("fix"));
    assert_eq!(node(&s), "implement-task");
    assert_eq!(feedback(&s), ("advance".into(), vec!["progress.md".into()]));
    assert_eq!(task_outputs(&s), report);
    let s = complete(&root, &s, None);
    let s = complete(&root, &s, None);
    let s = complete(&root, &s, Some("escalate"));
    assert_eq!(node(&s), "escalate-task");
    assert_eq!(feedback(&s), ("advance".into(), vec!["progress.md".into()]));
    let s = complete(&root, &s, None);
    assert_eq!(node(&s), "review-task");
    assert_eq!(
        feedback(&s),
        ("escalate-task".into(), vec!["escalation-report.md".into()])
    );
    let s = complete(&root, &s, None);
    let s = complete(&root, &s, Some("next_task"));
    assert_eq!(node(&s), "implement-task");
    let s = complete(&root, &s, None);
    let s = complete(&root, &s, None);
    let s = complete(&root, &s, Some("all_done"));

    assert_eq!(node(&s), "final-review");
    let final_review = task_outputs(&s);
    let s = complete(&root, &s, None);
    assert_eq!(node(&s), "advance-final");
    let s = complete(&root, &s, Some("final_fix"));
    assert_eq!(node(&s), "fix-final");
    let s = complete(&root, &s, None);
    assert_eq!(node(&s), "final-review");
    assert_eq!(
        feedback(&s),
        ("fix-final".into(), vec!["final-fix-report.md".into()])
    );
    assert_eq!(task_outputs(&s), final_review);
    let s = complete(&root, &s, None);
    let s = complete(&root, &s, Some("finish"));
    assert_eq!(node(&s), "finish");
    let s = complete(&root, &s, None);
    assert!(s.is_finished());
}
