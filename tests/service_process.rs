use std::io::Write;
use std::process::{Command, Stdio};

#[test]
fn service_mode_process_emits_ready_state_and_stopped_events() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_locking-pomodoro-timer"))
        .arg("--service")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();

    let stdin = child.stdin.as_mut().unwrap();
    writeln!(
        stdin,
        r#"{{"protocolVersion":1,"command":{{"type":"quit"}}}}"#
    )
    .unwrap();
    drop(child.stdin.take());

    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());

    let events: Vec<serde_json::Value> = String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    let event_types: Vec<&str> = events
        .iter()
        .map(|event| event["event"]["type"].as_str().unwrap())
        .collect();

    assert_eq!(event_types, ["ready", "state", "stopped"]);
}