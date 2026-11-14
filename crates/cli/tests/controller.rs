use assert_cmd::cargo::cargo_bin_cmd;
use assert_fs::TempDir;
use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};

#[test]
fn claude_hook_replaces_context_and_marks_workers() {
    let project = TempDir::new().unwrap();
    for worker in [false, true] {
        let mut input = json!({"session_id":"trusted","cwd":"/ignored","tool_name":"mcp__plugin_unelos_unelos__next",
            "tool_input":{"name":"kept","unelos_context":{"harness":"evil","sessionId":"model"}}});
        if worker {
            input["agent_id"] = json!("worker-id");
        }
        let output = cargo_bin_cmd!("unelos")
            .args(["hook", "claude", "pre-tool-use"])
            .env("CLAUDE_PROJECT_DIR", project.path())
            .write_stdin(input.to_string())
            .output()
            .unwrap();
        assert!(output.status.success(), "{output:?}");
        let value: Value = serde_json::from_slice(&output.stdout).unwrap();
        let updated = &value["hookSpecificOutput"]["updatedInput"];
        assert_eq!(updated["name"], "kept");
        assert_eq!(
            updated["unelos_context"],
            json!({"harness":"claude","sessionId":"trusted","project":project.path(),
            "role":if worker {"worker"} else {"controller"}})
        );
        assert_eq!(value["hookSpecificOutput"]["permissionDecision"], "allow");
    }
}

#[test]
fn activation_raw_request_and_exclusive_flags() {
    let temp = TempDir::new().unwrap();
    let project = camino::Utf8Path::from_path(temp.path()).unwrap();
    unelos_core::install_workflow(
        project,
        Some(
            &camino::Utf8Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../core/tests/fixtures/simple-plan-execute-review"),
        ),
        "simple-plan-execute-review",
    )
    .unwrap();
    for (request, expected) in [(" resume ", "resume"), ("resume run", "new"), ("", "new")] {
        let output = cargo_bin_cmd!("unelos")
            .arg("--project")
            .arg(project)
            .args([
                "session",
                "activate",
                "--harness",
                "claude",
                "--session",
                "s",
                "--request",
                request,
                "--json",
            ])
            .env("UNELOS_CONFIG_DIR", temp.path().join("config"))
            .output()
            .unwrap();
        assert!(output.status.success(), "{output:?}");
        let value: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(value["requestedAction"], request);
        let context = unelos_core::session_binding::SessionContext {
            harness: "claude".into(),
            session_id: "s".into(),
            project: project.into(),
            role: "controller".into(),
        };
        let service = unelos_core::session_binding::SessionService::new(project.join("config"));
        assert_eq!(service.binding(&context).unwrap().unwrap().mode, expected);
    }
    cargo_bin_cmd!("unelos")
        .args([
            "session",
            "activate",
            "--harness",
            "claude",
            "--session",
            "s",
            "--mode",
            "new",
            "--request",
            "resume",
        ])
        .assert()
        .failure();
}

#[test]
fn mcp_stdio_initialize_list_and_injected_next() {
    let temp = TempDir::new().unwrap();
    let mut child = Command::new(assert_cmd::cargo::cargo_bin!("unelos"))
        .arg("mcp")
        .env("UNELOS_CONFIG_DIR", temp.path().join("config"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .unwrap();
    let mut input = child.stdin.take().unwrap();
    let mut output = BufReader::new(child.stdout.take().unwrap());
    fn exchange(input: &mut impl Write, output: &mut impl BufRead, request: Value) -> Value {
        writeln!(input, "{request}").unwrap();
        input.flush().unwrap();
        let mut line = String::new();
        assert!(output.read_line(&mut line).unwrap() > 0);
        serde_json::from_str(&line).unwrap()
    }
    let initialized = exchange(
        &mut input,
        &mut output,
        json!({"jsonrpc":"2.0","id":1,"method":"initialize",
        "params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1"}}}),
    );
    assert_eq!(initialized["result"]["serverInfo"]["name"], "unelos");
    writeln!(
        input,
        "{}",
        json!({"jsonrpc":"2.0","method":"notifications/initialized"})
    )
    .unwrap();
    let tools = exchange(
        &mut input,
        &mut output,
        json!({"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}),
    );
    let tools = tools["result"]["tools"].as_array().unwrap();
    assert_eq!(tools.len(), 9);
    assert!(tools
        .iter()
        .all(|t| t["inputSchema"]["properties"]["unelos_context"].is_object()));
    let called = exchange(
        &mut input,
        &mut output,
        json!({"jsonrpc":"2.0","id":3,"method":"tools/call",
        "params":{"name":"next","arguments":{"unelos_context":{"harness":"claude","sessionId":"test","project":temp.path(),"role":"controller"}}}}),
    );
    assert_eq!(called["result"]["isError"], true);
    let error: Value =
        serde_json::from_str(called["result"]["content"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(error["error"]["code"], "controller_inactive");
    let project = camino::Utf8Path::from_path(temp.path()).unwrap();
    unelos_core::install_workflow(
        project,
        Some(
            &camino::Utf8Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../core/tests/fixtures/simple-plan-execute-review"),
        ),
        "simple-plan-execute-review",
    )
    .unwrap();
    let service = unelos_core::session_binding::SessionService::new(project.join("config"));
    let context = unelos_core::session_binding::SessionContext {
        harness: "claude".into(),
        session_id: "test".into(),
        project: project.into(),
        role: "controller".into(),
    };
    service.activate(&context, "new").unwrap();
    let started = service
        .dispatch(
            "start",
            &json!({"unelos_context":context,"workflow":"simple-plan-execute-review"}),
            None,
        )
        .unwrap();
    let called = exchange(
        &mut input,
        &mut output,
        json!({"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"next","arguments":{"unelos_context":context}}}),
    );
    assert_ne!(called["result"]["isError"], true);
    assert_eq!(called["result"]["content"].as_array().unwrap().len(), 1);
    let package: Value =
        serde_json::from_str(called["result"]["content"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(package["runId"], started["runId"]);
    assert!(package["harnessInstructions"]
        .as_str()
        .unwrap()
        .contains("mcp__plugin_unelos_unelos__next"));
    drop(input);
    assert!(child.wait().unwrap().success());
}

#[test]
fn session_start_is_read_only_and_recovers_bound_run() {
    let temp = TempDir::new().unwrap();
    let project = camino::Utf8Path::from_path(temp.path()).unwrap();
    let config = project.join("config");
    let run = || {
        cargo_bin_cmd!("unelos")
            .args(["hook", "claude", "session-start"])
            .env("UNELOS_CONFIG_DIR", &config)
            .env("CLAUDE_PROJECT_DIR", project)
            .write_stdin(json!({"source":"compact","session_id":"s"}).to_string())
            .output()
            .unwrap()
    };
    let output = run();
    assert!(output.status.success());
    assert!(output.stdout.is_empty());
    assert!(!config.exists());
    unelos_core::install_workflow(
        project,
        Some(
            &camino::Utf8Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../core/tests/fixtures/simple-plan-execute-review"),
        ),
        "simple-plan-execute-review",
    )
    .unwrap();
    let service = unelos_core::session_binding::SessionService::new(config.clone());
    let context = unelos_core::session_binding::SessionContext {
        harness: "claude".into(),
        session_id: "s".into(),
        project: project.into(),
        role: "controller".into(),
    };
    service.activate(&context, "new").unwrap();
    let started = service
        .dispatch(
            "start",
            &json!({"unelos_context":context,"workflow":"simple-plan-execute-review"}),
            None,
        )
        .unwrap();
    let output = run();
    assert!(output.status.success(), "{output:?}");
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    let text = value["hookSpecificOutput"]["additionalContext"]
        .as_str()
        .unwrap();
    assert!(text.contains(started["runId"].as_str().unwrap()));
    assert!(text.contains("next"));

    let elsewhere = TempDir::new().unwrap();
    let output = cargo_bin_cmd!("unelos")
        .args(["hook", "claude", "session-start"])
        .env("UNELOS_CONFIG_DIR", &config)
        .env_remove("CLAUDE_PROJECT_DIR")
        .write_stdin(json!({"source":"resume","session_id":"s","cwd":elsewhere.path()}).to_string())
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(output.stdout.is_empty());

    let path = std::fs::read_dir(config.join("sessions/claude"))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .find(|path| {
            path.extension()
                .is_some_and(|extension| extension == "json")
        })
        .unwrap();
    let mut binding: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    binding["pendingStart"] = json!({"entryId":binding["entryId"],"at":1});
    std::fs::write(&path, serde_json::to_vec(&binding).unwrap()).unwrap();
    let output = run();
    assert!(output.status.success());
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    let text = value["hookSpecificOutput"]["additionalContext"]
        .as_str()
        .unwrap();
    assert!(text.contains("pendingStart"));
    assert!(text.contains("/unelos-resume"));
    assert!(text.contains("never retry start"));
    assert!(!text.contains("__next"));

    binding["enabled"] = json!(false);
    std::fs::write(&path, serde_json::to_vec(&binding).unwrap()).unwrap();
    assert!(run().stdout.is_empty());
}
