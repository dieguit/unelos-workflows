use super::{packet_for_snapshot, store::RunStore, CurrentWork};
use crate::{
    read_brief, render_execution_package, ExecutionEntry, ExecutionError, ExecutionIdentity,
    ExecutionPackage, ResumeContext, ResumeFile, RunCatalog, RunState, RunSummary, RuntimeError,
    WorkPacket,
};
use camino::Utf8Path;
use std::{
    collections::BTreeSet,
    fs,
    io::{self, Read},
};

const MAX_RESUME_CONTEXT_BYTES: usize = 8 * 1024 * 1024;

pub fn execution_new(root: &Utf8Path) -> Result<ExecutionPackage, ExecutionError> {
    let descriptions =
        crate::project::list_workflow_descriptions(root).map_err(RuntimeError::from)?;
    let mut package =
        render_execution_package(None, descriptions.keys().cloned().collect(), None, None)?;
    package.workflow_descriptions = descriptions
        .into_iter()
        .filter_map(|(name, description)| description.map(|text| (name, text)))
        .collect();
    package.entry_mode = Some(ExecutionEntry::New);
    Ok(package)
}

fn identity(state: &RunState) -> Option<ExecutionIdentity> {
    match &state.current {
        CurrentWork::Task(record) => Some(ExecutionIdentity::Task {
            run_id: state.run_id.clone(),
            node_id: record.node_id.clone(),
            task_id: record.execution_id.clone(),
        }),
        CurrentWork::HumanGate(record) => Some(ExecutionIdentity::HumanGate {
            run_id: state.run_id.clone(),
            node_id: record.node_id.clone(),
            activation_id: record.execution_id.clone(),
        }),
        _ => None,
    }
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
            let expected = identity(&state);
            RunSummary {
                run_id: state.run_id,
                name: state.name,
                workflow_name: state.workflow_name,
                status: status.into(),
                node_id,
                expected,
            }
        })
        .collect();
    Ok(RunCatalog {
        entry_mode: ExecutionEntry::Resume,
        controller_instructions: include_str!("../instructions/controller.md").into(),
        runs,
    })
}

fn read_file(path: &Utf8Path, remaining: &mut usize) -> Result<ResumeFile, ExecutionError> {
    super::store::reject_symlink(path)?;
    let metadata = match fs::metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            return Ok(ResumeFile {
                path: path.into(),
                content: None,
            })
        }
        Err(source) => return Err(super::path_error(path, source).into()),
    };
    if !metadata.is_file() {
        return Err(ExecutionError::protocol(
            "invalid_resume_context",
            format!("Recovery path is not a regular file: {path}"),
        ));
    }
    let file = fs::File::open(path).map_err(|source| super::path_error(path, source))?;
    let mut bytes = Vec::new();
    file.take((*remaining + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|source| super::path_error(path, source))?;
    if bytes.len() > *remaining {
        return Err(ExecutionError::protocol(
            "resume_context_too_large",
            "Recovery context exceeds 8 MiB; reduce report size explicitly",
        ));
    }
    *remaining -= bytes.len();
    let content = String::from_utf8(bytes).map_err(|_| {
        ExecutionError::protocol(
            "invalid_resume_context",
            format!("Recovery report is not UTF-8: {path}"),
        )
    })?;
    Ok(ResumeFile {
        path: path.into(),
        content: Some(content),
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
    let project =
        crate::load_project(store.root(), &state.workflow_name).map_err(RuntimeError::from)?;
    crate::validate_project(&project).map_err(RuntimeError::from)?;
    // Missing required inputs are recovery issues, not a reason to hide the current identity.
    // All live definition/binding validation still runs.
    let packet = packet_for_snapshot(&project, &state, false)?;
    let mut remaining = MAX_RESUME_CONTEXT_BYTES;
    let ledger = read_file(
        &store
            .root()
            .join(".unelos/runs")
            .join(&state.name)
            .join("ledger.md"),
        &mut remaining,
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
            reports.push(read_file(&report.path, &mut remaining)?);
        }
    }
    for input in &record.inputs {
        if reports
            .iter()
            .any(|report| report.path == input.path && report.content.is_none())
        {
            issues.push(format!(
                "Restore missing required input before continuing: {}",
                input.path
            ));
        }
    }
    let delegated = matches!(&packet, WorkPacket::Task(task) if task.packet.context.execution == crate::Execution::Subagent);
    let brief = if delegated {
        match read_brief(ledger.content.as_deref().unwrap_or("")) {
            Ok(brief) => Some(brief),
            Err(error) => {
                issues.push(error.to_string());
                None
            }
        }
    } else {
        None
    };
    let mut package = if delegated && brief.is_none() {
        let mut package = render_execution_package(None, vec![], None, None)?;
        package.expected = ExecutionIdentity::from_packet(&packet);
        package.packet = Some(packet);
        package
    } else {
        render_execution_package(Some(packet), vec![], brief.as_deref(), None)?
    };
    if !issues.is_empty() {
        package.status = "needs-context".into();
        package.worker_prompt = None;
    }
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
