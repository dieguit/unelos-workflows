use super::{packet_for_snapshot, store::RunStore, CurrentWork};
use crate::{
    read_brief, render_execution_package, ExecutionEntry, ExecutionError, ExecutionIdentity,
    ExecutionPackage, ResumeContext, ResumeFile, RunCatalog, RunState, RunSummary, RuntimeError,
};
use camino::Utf8Path;
use std::{collections::BTreeSet, fs, io};

pub fn execution_new(root: &Utf8Path) -> Result<ExecutionPackage, ExecutionError> {
    let descriptions =
        crate::project::list_workflow_descriptions(root).map_err(RuntimeError::from)?;
    let mut package = render_execution_package(None, descriptions.keys().cloned().collect(), None)?;
    package.workflow_descriptions = descriptions
        .into_iter()
        .filter_map(|(name, description)| description.map(|text| (name, text)))
        .collect();
    package.entry_mode = Some(ExecutionEntry::New);
    Ok(package)
}

fn identity(
    root: &Utf8Path,
    state: &RunState,
) -> Result<Option<ExecutionIdentity>, ExecutionError> {
    let project = super::load_run_project(root, state)?;
    let packet = packet_for_snapshot(&project, state, false)?;
    Ok(ExecutionIdentity::from_packet(&packet))
}

pub fn execution_runs(root: &Utf8Path) -> Result<RunCatalog, ExecutionError> {
    let store = RunStore::open(root)?;
    let runs = store
        .runs()?
        .into_iter()
        .filter(|state| !state.is_finished())
        .map(|state| {
            let (status, node_id) = match &state.current {
                CurrentWork::Task(record) | CurrentWork::Parallel(record) => {
                    ("running", record.node_id.clone())
                }
                CurrentWork::HumanGate(record) => ("waiting", record.node_id.clone()),
                CurrentWork::Finished { .. } => unreachable!(),
            };
            let expected = identity(root, &state)?;
            Ok(RunSummary {
                run_id: state.run_id,
                name: state.name,
                workflow_name: state.workflow_name,
                status: status.into(),
                node_id,
                expected,
            })
        })
        .collect::<Result<Vec<_>, ExecutionError>>()?;
    Ok(RunCatalog {
        entry_mode: ExecutionEntry::Resume,
        controller_instructions: format!(
            "{}\n{}",
            include_str!("../instructions/controller.md"),
            include_str!("../instructions/controller-resume.md")
        ),
        runs,
    })
}

fn read_file(path: &Utf8Path) -> Result<ResumeFile, ExecutionError> {
    super::store::reject_symlink(path)?;
    let exists = match fs::metadata(path) {
        Ok(metadata) if metadata.is_file() => true,
        Ok(_) => {
            return Err(ExecutionError::protocol(
                "invalid_resume_context",
                format!("Recovery path is not a regular file: {path}"),
            ))
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => false,
        Err(error) => return Err(super::path_error(path, error).into()),
    };
    Ok(ResumeFile {
        path: path.into(),
        exists,
    })
}

pub fn execution_resume(root: &Utf8Path, run_id: &str) -> Result<ExecutionPackage, ExecutionError> {
    let store = RunStore::open(root)?;
    let state = store.run(run_id)?;
    if state.is_finished() {
        return Err(ExecutionError::protocol(
            "run_finished",
            "Finished runs cannot be resumed; begin a new run explicitly",
        ));
    }
    let project = super::load_run_project(store.root(), &state)?;
    crate::validate_project(&project).map_err(RuntimeError::from)?;
    // Missing required inputs are recovery issues, not a reason to hide the current identity.
    // All live definition/binding validation still runs.
    let packet = packet_for_snapshot(&project, &state, false)?;
    let ledger = read_file(
        &store
            .root()
            .join(".unelos/runs")
            .join(&state.run_id)
            .join("ledger.md"),
    )?;
    let record = match &state.current {
        CurrentWork::Task(record)
        | CurrentWork::Parallel(record)
        | CurrentWork::HumanGate(record) => record,
        CurrentWork::Finished { .. } => unreachable!(),
    };
    let mut issues = Vec::new();
    let mut reports = Vec::new();
    let mut seen = BTreeSet::new();
    let feedback = record
        .feedback
        .iter()
        .flat_map(|feedback| feedback.artifacts.iter());
    for report in record
        .inputs
        .iter()
        .chain(&record.outputs)
        .chain(record.worker_reports.iter().map(|r| &r.report))
        .chain(feedback)
    {
        if seen.insert(report.path.clone()) {
            reports.push(read_file(&report.path)?);
        }
    }
    for input in &record.inputs {
        if reports
            .iter()
            .any(|report| report.path == input.path && !report.exists)
        {
            issues.push(format!(
                "Restore missing required input before continuing: {}",
                input.path
            ));
        }
    }
    let brief = if ledger.exists {
        read_brief(
            &fs::read_to_string(&ledger.path)
                .map_err(|error| super::path_error(&ledger.path, error))?,
        )
    } else {
        None
    };
    let mut package = render_execution_package(Some(packet), vec![], brief.as_deref())?;
    package.controller_instructions.push('\n');
    package
        .controller_instructions
        .push_str(include_str!("../instructions/controller-resume.md"));
    if !issues.is_empty() {
        package.status = "needs-context".into();
        package.worker_prompt = None;
        package.worker_prompts.clear();
    }
    super::execution::filter_receipts(&mut package, &state);
    package.run_id = Some(state.run_id.clone());
    package.entry_mode = Some(ExecutionEntry::Resume);
    package.resume_context = Some(ResumeContext {
        state,
        ledger,
        reports,
        issues,
    });
    Ok(package)
}
