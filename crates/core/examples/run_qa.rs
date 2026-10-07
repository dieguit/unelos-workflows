use camino::Utf8Path;
use serde::Serialize;
use std::{fs, process};
use unelos_core::{
    approve, complete, confirm_outputs, load_project, next, prepare_work, reject, start, status,
    verify_outputs, verify_work_outputs, CompletedExecution, CurrentWork, RuntimeError,
    TransitionFeedback,
};

#[cfg(test)]
mod tests {
    use super::*;
    use assert_fs::{fixture::PathCopy, TempDir};
    use std::fs;
    use unelos_core::{approve, complete, confirm_outputs, reject, CurrentWork};

    #[test]
    fn rejection_reason_requires_exact_flag_and_one_value() {
        assert_eq!(
            rejection_reason(&["--reason".into(), "  ".into()]),
            Ok("  ")
        );
        for args in [
            vec![],
            vec!["--reason".into()],
            vec!["--unknown".into(), "reason".into()],
            vec!["--reason".into(), "reason".into(), "extra".into()],
        ] {
            assert!(rejection_reason(&args).is_err(), "accepted {args:?}");
        }
    }

    #[test]
    fn gate_decision_fixture_persists_rejection_and_approval_feedback() {
        let temp = TempDir::new().unwrap();
        let root = Utf8Path::from_path(temp.path()).unwrap().to_owned();
        temp.copy_from(
            Utf8Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/manual-project"),
            &["**/*"],
        )
        .unwrap();
        let project = load_project(&root, "implement-feature").unwrap();
        let state = start(&project, "gate-loop", "Change button color.").unwrap();
        let CurrentWork::Task(plan) = &state.current else {
            panic!("expected initial plan")
        };
        fs::write(&plan.outputs[0].path, "Plan v1").unwrap();
        confirm_outputs(&root, "task-1").unwrap();
        let gate = complete(&root, "gate-loop", "task-1", None, None).unwrap();
        assert!(matches!(gate.current, CurrentWork::HumanGate(_)));

        let revised = reject(
            &root,
            "gate-loop",
            "gate-2",
            "Explain migration and rollback.",
        )
        .unwrap();
        let CurrentWork::Task(rework) = &revised.current else {
            panic!("expected revised plan")
        };
        assert_eq!(rework.execution_id, "task-3");
        assert_eq!(
            rework.feedback.as_ref().and_then(|f| f.reason.as_deref()),
            Some("Explain migration and rollback.")
        );
        assert_eq!(revised.history[1].outcome.as_deref(), Some("rejected"));
        assert_eq!(
            revised.history[1].reason.as_deref(),
            Some("Explain migration and rollback.")
        );

        fs::write(
            &rework.outputs[0].path,
            "Plan v2: migration and rollback explained",
        )
        .unwrap();
        confirm_outputs(&root, "task-3").unwrap();
        complete(&root, "gate-loop", "task-3", None, None).unwrap();
        let before_stale = fs::read(root.join(".unelos/runs/gate-loop/state.json")).unwrap();
        assert!(approve(&root, "gate-loop", "gate-2").is_err());
        assert_eq!(
            fs::read(root.join(".unelos/runs/gate-loop/state.json")).unwrap(),
            before_stale
        );

        let approved = approve(&root, "gate-loop", "gate-4").unwrap();
        let CurrentWork::Task(implementation) = &approved.current else {
            panic!("expected implementation")
        };
        assert_eq!(implementation.execution_id, "task-5");
        assert_eq!(
            implementation
                .feedback
                .as_ref()
                .and_then(|f| f.outcome.as_deref()),
            Some("approved")
        );
        assert_eq!(approved.history[3].outcome.as_deref(), Some("approved"));
        assert_eq!(approved.history[3].reason, None);
        assert_eq!(approved.history[1].outcome.as_deref(), Some("rejected"));
        assert_eq!(
            approved.history[1].reason.as_deref(),
            Some("Explain migration and rollback.")
        );
        assert_eq!(status(&root).unwrap(), approved);
        assert_eq!(next(&root).unwrap(), next(&root).unwrap());
    }

    #[test]
    fn completion_options_accept_pairs_in_either_order_and_reject_bad_syntax() {
        assert_eq!(completion_options(&[]).unwrap(), (None, None));
        assert_eq!(
            completion_options(&["--outcome".into(), "approved".into()]).unwrap(),
            (Some("approved"), None)
        );
        assert_eq!(
            completion_options(&["--reason".into(), "Looks good".into()]).unwrap(),
            (None, Some("Looks good"))
        );
        assert_eq!(
            completion_options(&[
                "--reason".into(),
                "Looks good".into(),
                "--outcome".into(),
                "approved".into()
            ])
            .unwrap(),
            (Some("approved"), Some("Looks good"))
        );
        for args in [
            vec!["--reason".into()],
            vec![
                "--reason".into(),
                "reason".into(),
                "--reason".into(),
                "again".into(),
            ],
            vec![
                "--outcome".into(),
                "ok".into(),
                "--outcome".into(),
                "no".into(),
            ],
            vec!["--unknown".into(), "value".into()],
        ] {
            assert!(completion_options(&args).is_err(), "accepted {args:?}");
        }
    }

    #[test]
    fn completion_loop_fixture_finishes_with_review_feedback_in_history() {
        let temp = TempDir::new().unwrap();
        let root = Utf8Path::from_path(temp.path()).unwrap().to_owned();
        temp.copy_from(
            Utf8Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/manual-project"),
            &["**/*"],
        )
        .unwrap();
        let project = load_project(&root, "completion-loop").unwrap();
        let mut state = start(&project, "qa-loop", "Change button color.").unwrap();
        for (id, report) in [
            ("task-1", "plan"),
            ("task-2", "implementation"),
            ("task-3", "review"),
            ("task-4", "corrected implementation"),
            ("task-5", "final review"),
        ] {
            let CurrentWork::Task(work) = &state.current else {
                panic!("expected task {id}")
            };
            fs::write(&work.outputs[0].path, report).unwrap();
            confirm_outputs(&root, id).unwrap();
            state = complete(
                &root,
                "qa-loop",
                id,
                match id {
                    "task-3" => Some("changes_requested"),
                    "task-5" => Some("approved"),
                    _ => None,
                },
                (id == "task-3").then_some("Adjust color contrast"),
            )
            .unwrap();
            if id == "task-3" {
                let CurrentWork::Task(rework) = &state.current else {
                    panic!("expected rework task")
                };
                assert_eq!(
                    rework
                        .feedback
                        .as_ref()
                        .and_then(|feedback| feedback.outcome.as_deref()),
                    Some("changes_requested")
                );
                assert_eq!(
                    rework
                        .feedback
                        .as_ref()
                        .and_then(|feedback| feedback.reason.as_deref()),
                    Some("Adjust color contrast")
                );
            }
        }
        assert!(matches!(state.current, CurrentWork::Finished { .. }));
        assert_eq!(state.history.len(), 5);
        assert_eq!(
            state.history[2].outcome.as_deref(),
            Some("changes_requested")
        );
        assert_eq!(
            state.history[2].reason.as_deref(),
            Some("Adjust color contrast")
        );
        assert_eq!(state.history[4].outcome.as_deref(), Some("approved"));
    }

    #[test]
    fn bindings_demo_reuses_report_path_without_reusing_confirmation_or_state() {
        let temp = TempDir::new().unwrap();
        let root = Utf8Path::from_path(temp.path()).unwrap().to_owned();
        temp.copy_from(
            Utf8Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/manual-project"),
            &["**/*"],
        )
        .unwrap();
        let project = load_project(&root, "implement-feature").unwrap();
        let state = start(&project, "bindings-demo", "Change button color.").unwrap();
        let CurrentWork::Task(plan) = &state.current else {
            panic!("expected plan")
        };
        fs::write(&plan.outputs[0].path, "confirmed plan").unwrap();
        confirm_outputs(&root, "task-1").unwrap();
        let state_path = root.join(".unelos/runs/bindings-demo/state.json");
        let before = fs::read(&state_path).unwrap();

        let result = bindings_demo(&root).unwrap();

        assert_eq!(fs::read(state_path).unwrap(), before);
        assert_eq!(result["sameOutputPath"], true);
        assert_eq!(result["newExecutionId"], true);
        assert_eq!(result["staleOutputRejected"], true);
        assert_eq!(result["feedbackPreserved"], true);
        assert_eq!(result["persistedStateUnchanged"], true);
    }

    #[cfg(unix)]
    #[test]
    fn candidate_writer_rejects_symlinked_run_directory_without_writing_outside_root() {
        use std::os::unix::fs::symlink;

        let temp = TempDir::new().unwrap();
        let project = temp.path().join("project");
        fs::create_dir(&project).unwrap();
        let root = Utf8Path::from_path(&project).unwrap().to_owned();
        let outside = Utf8Path::from_path(&temp.path().join("outside"))
            .unwrap()
            .to_owned();
        fs::create_dir_all(&outside).unwrap();
        let runs = root.join(".unelos/runs");
        fs::create_dir_all(&runs).unwrap();
        symlink(&outside, runs.join("demo-run")).unwrap();
        let candidate = candidate_state(
            "demo-run",
            root.join(".unelos/runs/demo-run/implement/report.md"),
        );

        assert!(write_candidate_outputs(&root, &candidate).is_err());
        assert!(!outside.join("implement/report.md").exists());
    }

    #[test]
    fn candidate_writer_rejects_non_directory_run_root() {
        let temp = TempDir::new().unwrap();
        let root = Utf8Path::from_path(temp.path()).unwrap().to_owned();
        let run_dir = root.join(".unelos/runs/demo-run");
        fs::create_dir_all(run_dir.parent().unwrap()).unwrap();
        fs::write(&run_dir, "leave unchanged").unwrap();
        let candidate = candidate_state("demo-run", run_dir.join("implement/report.md"));

        assert!(write_candidate_outputs(&root, &candidate).is_err());
        assert_eq!(fs::read(&run_dir).unwrap(), b"leave unchanged");
    }

    fn candidate_state(name: &str, output_path: camino::Utf8PathBuf) -> unelos_core::RunState {
        use unelos_core::{ArtifactReference, ExecutionRecord, RunState};

        RunState {
            run_id: name.into(),
            name: name.into(),
            workflow_name: "implement-feature".into(),
            request: "demo".into(),
            current: CurrentWork::Task(ExecutionRecord {
                node_id: "implement".into(),
                execution_id: "task-2".into(),
                inputs: vec![],
                outputs: vec![ArtifactReference {
                    name: "implementation-report.md".into(),
                    path: output_path,
                }],
                worker_reports: vec![],
                feedback: None,
                confirmed_execution_id: None,
            }),
            history: vec![],
            next_execution_sequence: 3,
        }
    }
}

fn main() {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let result: Result<serde_json::Value, Box<dyn std::error::Error>> = match args.as_slice() {
        [command, project, workflow, name, request] if command == "start" => (|| {
            let loaded = load_project(Utf8Path::new(project), workflow)?;
            Ok(serde_json::to_value(start(&loaded, name, request)?)?)
        })(),
        [command, project] if command == "status" => json(status(Utf8Path::new(project))),
        [command, project] if command == "next" => json(next(Utf8Path::new(project))),
        [command, project, expected] if command == "confirm" => {
            json(confirm_outputs(Utf8Path::new(project), expected))
        }
        [command, project, expected] if command == "verify" => {
            json(verify_outputs(Utf8Path::new(project), expected))
        }
        [command, project, run_id, expected, options @ ..] if command == "complete" => {
            match completion_options(options) {
                Ok((outcome, reason)) => json(complete(
                    Utf8Path::new(project),
                    run_id,
                    expected,
                    outcome,
                    reason,
                )),
                Err(_) => usage(),
            }
        }
        [command, project, run_id, expected] if command == "approve" => {
            json(approve(Utf8Path::new(project), run_id, expected))
        }
        [command, project, run_id, expected, options @ ..] if command == "reject" => {
            match rejection_reason(options) {
                Ok(reason) => json(reject(Utf8Path::new(project), run_id, expected, reason)),
                Err(_) => usage(),
            }
        }
        [command, project] if command == "bindings-demo" => bindings_demo(Utf8Path::new(project)),
        _ => usage(),
    };

    match result {
        Ok(value) => match serde_json::to_string_pretty(&value) {
            Ok(json) => println!("{json}"),
            Err(error) => {
                eprintln!("{error}");
                process::exit(1);
            }
        },
        Err(error) => {
            eprintln!("{error}");
            process::exit(1);
        }
    }
}

fn usage() -> ! {
    eprintln!(
        "usage:\n  run_qa start PROJECT WORKFLOW NAME REQUEST\n  run_qa status PROJECT\n  run_qa next PROJECT\n  run_qa confirm PROJECT EXECUTION_ID\n  run_qa verify PROJECT EXECUTION_ID\n  run_qa complete PROJECT RUN_ID EXECUTION_ID [--outcome LABEL] [--reason TEXT]\n  run_qa approve PROJECT RUN_ID ACTIVATION_ID\n  run_qa reject PROJECT RUN_ID ACTIVATION_ID --reason TEXT\n  run_qa bindings-demo PROJECT"
    );
    process::exit(2)
}

fn completion_options(args: &[String]) -> Result<(Option<&str>, Option<&str>), String> {
    let (mut outcome, mut reason) = (None, None);
    let mut args = args.iter();
    while let Some(flag) = args.next() {
        let value = args
            .next()
            .filter(|value| !value.starts_with("--"))
            .ok_or_else(|| format!("missing value for {flag}"))?;
        let target = match flag.as_str() {
            "--outcome" => &mut outcome,
            "--reason" => &mut reason,
            _ => return Err(format!("unknown option {flag}")),
        };
        if target.replace(value.as_str()).is_some() {
            return Err(format!("repeated option {flag}"));
        }
    }
    Ok((outcome, reason))
}

fn rejection_reason(args: &[String]) -> Result<&str, &'static str> {
    match args {
        [flag, reason] if flag == "--reason" => Ok(reason),
        _ => Err("expected --reason TEXT"),
    }
}

fn json<T: Serialize>(
    result: Result<T, RuntimeError>,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    Ok(serde_json::to_value(result?)?)
}

fn bindings_demo(root: &Utf8Path) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let active = status(root)?;
    let CurrentWork::Task(plan) = &active.current else {
        return Err("bindings-demo requires current plan task work".into());
    };
    if plan.node_id != "plan" || active.workflow_name != "implement-feature" {
        return Err("bindings-demo requires the active implement-feature plan".into());
    }
    verify_outputs(root, &plan.execution_id)?;
    let state_path = root
        .join(".unelos/runs")
        .join(&active.name)
        .join("state.json");
    let persisted_before = fs::read(&state_path)?;
    let project = load_project(root, "implement-feature")?;

    let mut synthetic = active.clone();
    synthetic.history.push(CompletedExecution {
        work: synthetic.current.clone(),
        outcome: None,
        reason: None,
    });
    let implementation = prepare_work(&project, &synthetic, "implement", None)?;
    write_candidate_outputs(root, &implementation)?;
    let implementation_id = current_execution_id(&implementation)?;
    let implementation_path = current_output_path(&implementation)?;

    synthetic = implementation;
    synthetic.history.push(CompletedExecution {
        work: synthetic.current.clone(),
        outcome: None,
        reason: None,
    });
    let review = prepare_work(&project, &synthetic, "review", None)?;
    write_candidate_outputs(root, &review)?;
    let review_id = current_execution_id(&review)?;
    let feedback = TransitionFeedback {
        source_node_id: "review".into(),
        source_execution_id: review_id,
        outcome: Some("changes_requested".into()),
        reason: Some("Demonstration correction".into()),
        artifacts: current_outputs(&review)?,
    };
    synthetic = review;
    synthetic.history.push(CompletedExecution {
        work: synthetic.current.clone(),
        outcome: Some("changes_requested".into()),
        reason: feedback.reason.clone(),
    });
    let correction = prepare_work(&project, &synthetic, "implement", Some(feedback.clone()))?;
    let correction_id = current_execution_id(&correction)?;
    let correction_path = current_output_path(&correction)?;
    let CurrentWork::Task(correction_record) = &correction.current else {
        return Err("expected correction task".into());
    };
    if correction_record.confirmed_execution_id.is_some() {
        return Err("re-entry unexpectedly retained output confirmation".into());
    }
    write_candidate_outputs(root, &correction)?;
    let stale_output_rejected = verify_work_outputs(root, &correction, &implementation_id).is_err();
    let feedback_preserved = correction_record.feedback.as_ref() == Some(&feedback);
    let state_unchanged = fs::read(&state_path)? == persisted_before;
    if implementation_path != correction_path
        || implementation_id == correction_id
        || !stale_output_rejected
        || !feedback_preserved
        || !state_unchanged
    {
        return Err("bindings-demo assertion failed".into());
    }
    Ok(serde_json::json!({
        "sameOutputPath": implementation_path == correction_path,
        "newExecutionId": implementation_id != correction_id,
        "staleOutputRejected": stale_output_rejected,
        "feedbackPreserved": feedback_preserved,
        "persistedStateUnchanged": state_unchanged,
    }))
}

fn current_execution_id(state: &unelos_core::RunState) -> Result<String, &'static str> {
    match &state.current {
        CurrentWork::Task(record) | CurrentWork::Parallel(record) => {
            Ok(record.execution_id.clone())
        }
        _ => Err("expected task work"),
    }
}

fn current_outputs(
    state: &unelos_core::RunState,
) -> Result<Vec<unelos_core::ArtifactReference>, &'static str> {
    match &state.current {
        CurrentWork::Task(record) | CurrentWork::Parallel(record) => Ok(record.outputs.clone()),
        _ => Err("expected task work"),
    }
}

fn current_output_path(state: &unelos_core::RunState) -> Result<camino::Utf8PathBuf, &'static str> {
    current_outputs(state)?
        .into_iter()
        .next()
        .map(|output| output.path)
        .ok_or("expected declared output")
}

fn write_candidate_outputs(
    root: &Utf8Path,
    state: &unelos_core::RunState,
) -> Result<(), Box<dyn std::error::Error>> {
    let run_dir = root.join(".unelos/runs").join(&state.name);
    let metadata = fs::symlink_metadata(&run_dir)?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err(format!("unsafe demonstration run directory: {run_dir}").into());
    }
    let outputs = current_outputs(state)?;
    for output in &outputs {
        if !output.path.starts_with(&run_dir) || output.path == run_dir {
            return Err(format!("unsafe demonstration output path: {}", output.path).into());
        }
        let relative = output.path.strip_prefix(&run_dir)?;
        if relative
            .components()
            .any(|component| !matches!(component, camino::Utf8Component::Normal(_)))
        {
            return Err(format!("unsafe demonstration output path: {}", output.path).into());
        }
        let mut directory = run_dir.clone();
        for component in relative.parent().unwrap().components() {
            directory.push(component);
            match fs::symlink_metadata(&directory) {
                Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => break,
                Ok(_) => return Err(format!("unsafe demonstration directory: {directory}").into()),
                Err(error) => return Err(error.into()),
            }
        }
        if let Ok(metadata) = fs::symlink_metadata(&output.path) {
            if !metadata.is_file() || metadata.file_type().is_symlink() {
                return Err(format!("unsafe demonstration output file: {}", output.path).into());
            }
        }
    }
    for output in &outputs {
        let mut directory = run_dir.clone();
        for component in output
            .path
            .strip_prefix(&run_dir)?
            .parent()
            .unwrap()
            .components()
        {
            directory.push(component);
            if !directory.exists() {
                fs::create_dir(&directory)?;
            }
        }
    }
    for output in outputs {
        fs::write(output.path, format!("Synthetic {} report\n", state.name))?;
    }
    Ok(())
}
