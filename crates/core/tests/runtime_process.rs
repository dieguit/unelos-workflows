use assert_fs::{fixture::PathCopy, TempDir};
use camino::Utf8Path;
use serde_json::Value;
use std::{
    io::{BufRead, BufReader, Read, Write},
    process::{Child, Command, Stdio},
    sync::mpsc,
    time::{Duration, Instant},
};

const RESULT_PREFIX: &str = "UNELOS_RUNTIME_RESULT:";

fn project_copy() -> (TempDir, camino::Utf8PathBuf) {
    let temp = TempDir::new().unwrap();
    let root = Utf8Path::from_path(temp.path()).unwrap().to_owned();
    temp.copy_from(
        Utf8Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/manual-project"),
        &["**/*"],
    )
    .unwrap();
    (temp, root)
}

fn probe(root: &Utf8Path, operation: &str, name: &str, request: &str) -> Child {
    Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "process_probe", "--nocapture"])
        .env("UNELOS_RUNTIME_PROBE_ROOT", root.as_str())
        .env("UNELOS_RUNTIME_PROBE_OPERATION", operation)
        .env("UNELOS_RUNTIME_PROBE_NAME", name)
        .env("UNELOS_RUNTIME_PROBE_REQUEST", request)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap()
}

fn finish_probe(child: Child) -> (bool, String, String) {
    let output = wait_with_timeout(child);
    (
        output.status.success(),
        String::from_utf8(output.stdout).unwrap(),
        String::from_utf8(output.stderr).unwrap(),
    )
}

fn wait_with_timeout(mut child: Child) -> std::process::Output {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if child.try_wait().unwrap().is_some() {
            return child.wait_with_output().unwrap();
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let output = child.wait_with_output().unwrap();
            panic!(
                "probe timed out: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn result(stdout: &str) -> Value {
    let line = stdout
        .lines()
        .find_map(|line| line.strip_prefix(RESULT_PREFIX))
        .expect("probe result line");
    serde_json::from_str(line).unwrap()
}

#[test]
fn process_probe() {
    let Ok(root) = std::env::var("UNELOS_RUNTIME_PROBE_ROOT") else {
        return;
    };
    let root = Utf8Path::new(&root);
    let operation = std::env::var("UNELOS_RUNTIME_PROBE_OPERATION").unwrap();
    if operation == "hold-lock" {
        let lock = std::fs::OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .truncate(false)
            .open(root.join(".unelos/runtime.lock"))
            .unwrap();
        lock.lock().unwrap();
        println!("{RESULT_PREFIX}ready");
        std::io::stdout().flush().unwrap();
        std::io::stdin().read_to_end(&mut Vec::new()).unwrap();
        return;
    }

    let value = match operation.as_str() {
        "start" => {
            let project = unelos_core::load_project(root, "implement-feature").unwrap();
            let state = unelos_core::start(
                &project,
                &std::env::var("UNELOS_RUNTIME_PROBE_NAME").unwrap(),
                &std::env::var("UNELOS_RUNTIME_PROBE_REQUEST").unwrap(),
            );
            match state {
                Ok(state) => serde_json::to_value(state).unwrap(),
                Err(error) => {
                    eprintln!("{error}");
                    std::process::exit(1);
                }
            }
        }
        "status" => serde_json::to_value(unelos_core::status(root).unwrap()).unwrap(),
        "next" => serde_json::to_value(unelos_core::next(root).unwrap()).unwrap(),
        _ => panic!("unknown probe operation {operation}"),
    };
    println!("{RESULT_PREFIX}{value}");
}

#[test]
fn separate_processes_recover_the_same_run_and_work_packet() {
    let (_temp, root) = project_copy();
    let start = finish_probe(probe(&root, "start", "process-run", "A process request."));
    assert!(start.0, "{}{}", start.1, start.2);
    let started = result(&start.1);

    let status = finish_probe(probe(&root, "status", "", ""));
    assert!(status.0, "{}{}", status.1, status.2);
    let recovered = result(&status.1);
    assert_eq!(recovered["request"], "A process request.");
    assert_eq!(recovered["runId"], started["runId"]);
    assert_eq!(recovered["current"], started["current"]);

    let next = finish_probe(probe(&root, "next", "", ""));
    assert!(next.0, "{}{}", next.1, next.2);
    let packet = result(&next.1);
    assert_eq!(packet["request"], "A process request.");
    assert_eq!(packet["taskId"], "task-1");
    assert_eq!(packet["runId"], started["runId"]);
    let output = packet["outputs"][0]["path"].as_str().unwrap();
    assert!(Utf8Path::new(output).ends_with("process-run/plan/plan.md"));
    assert_eq!(recovered["current"]["outputs"][0]["path"], output);
}

#[test]
fn concurrent_start_processes_publish_exactly_one_active_run() {
    let (_temp, root) = project_copy();
    let first = probe(&root, "start", "race-one", "Request one");
    let second = probe(&root, "start", "race-two", "Request two");
    let first_out = wait_with_timeout(first);
    let second_out = wait_with_timeout(second);
    assert_ne!(first_out.status.success(), second_out.status.success());

    let winner = if first_out.status.success() {
        result(std::str::from_utf8(&first_out.stdout).unwrap())
    } else {
        result(std::str::from_utf8(&second_out.stdout).unwrap())
    };
    let loser_stderr = if first_out.status.success() {
        String::from_utf8(second_out.stderr).unwrap()
    } else {
        String::from_utf8(first_out.stderr).unwrap()
    };
    assert!(loser_stderr.contains("already active"), "{loser_stderr}");
    let active_name = std::fs::read_to_string(root.join(".unelos/active-run")).unwrap();
    assert_eq!(winner["name"], active_name);
    let saved: Value = serde_json::from_slice(
        &std::fs::read(root.join(format!(".unelos/runs/{active_name}/state.json"))).unwrap(),
    )
    .unwrap();
    assert_eq!(saved, winner);
}

#[test]
fn operating_system_releases_lock_when_process_is_killed() {
    let (_temp, root) = project_copy();
    let mut child = probe(&root, "hold-lock", "", "");
    let stdout = child.stdout.take().unwrap();
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let mut reader = BufReader::new(stdout);
        loop {
            let mut line = String::new();
            match reader.read_line(&mut line) {
                Ok(0) => {
                    let _ = tx.send(Ok(line));
                    return;
                }
                Ok(_) if line.contains(RESULT_PREFIX) => {
                    let _ = tx.send(Ok(line));
                    return;
                }
                Ok(_) => {}
                Err(error) => {
                    let _ = tx.send(Err(error));
                    return;
                }
            }
        }
    });

    let ready = rx.recv_timeout(Duration::from_secs(10));
    if ready.is_err() {
        let _ = child.kill();
        let _ = child.wait();
        panic!("lock holder did not signal readiness within 10 seconds");
    }
    let line = ready.unwrap().unwrap();
    child.kill().unwrap();
    child.wait().unwrap();
    assert_eq!(line.trim(), format!("{RESULT_PREFIX}ready"), "{line:?}");

    let start = finish_probe(probe(&root, "start", "after-kill", "fresh process"));
    assert!(start.0, "{}{}", start.1, start.2);
    assert_eq!(result(&start.1)["name"], "after-kill");
}
