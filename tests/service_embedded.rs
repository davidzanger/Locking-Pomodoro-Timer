use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use locking_pomodoro_timer::run_embedded_service;

#[test]
fn embedded_service_emits_events_and_accepts_commands() {
    let (command_sender, command_receiver) = mpsc::channel();
    let (event_sender, event_receiver) = mpsc::channel();
    let worker = thread::spawn(move || {
        run_embedded_service(command_receiver, event_sender).unwrap();
    });

    let ready = event_receiver.recv_timeout(Duration::from_secs(5)).unwrap();
    let initial_state = event_receiver.recv_timeout(Duration::from_secs(5)).unwrap();
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&ready).unwrap()["event"]["type"],
        "ready"
    );
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&initial_state).unwrap()["event"]["type"],
        "state"
    );

    command_sender
        .send(r#"{"protocolVersion":1,"command":{"type":"quit"}}"#.to_owned())
        .unwrap();
    let stopped = event_receiver.recv_timeout(Duration::from_secs(5)).unwrap();
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&stopped).unwrap()["event"]["type"],
        "stopped"
    );
    worker.join().unwrap();
}

#[test]
fn embedded_service_rejects_a_stale_phase_advance_and_keeps_the_current_phase() {
    let (commands, command_receiver) = mpsc::channel();
    let (event_sender, events) = mpsc::channel();
    let worker =
        thread::spawn(move || run_embedded_service(command_receiver, event_sender).unwrap());
    let receive = || -> serde_json::Value {
        serde_json::from_str(&events.recv_timeout(Duration::from_secs(5)).unwrap()).unwrap()
    };

    assert_eq!(receive()["event"]["type"], "ready");
    assert_eq!(receive()["event"]["status"], "idle");
    commands
        .send(r#"{"protocolVersion":1,"command":{"type":"start"}}"#.to_owned())
        .unwrap();
    let started = receive();
    assert_eq!(started["event"]["status"], "running");
    let current_phase_id = started["event"]["phaseId"].as_u64().unwrap();

    commands
        .send(format!(
            r#"{{"protocolVersion":1,"command":{{"type":"nextPhase","expectedPhaseId":{}}}}}"#,
            current_phase_id + 1
        ))
        .unwrap();
    assert_eq!(receive()["event"]["type"], "error");
    commands
        .send(r#"{"protocolVersion":1,"command":{"type":"pause"}}"#.to_owned())
        .unwrap();
    let paused = receive();
    assert_eq!(paused["event"]["status"], "paused");
    assert_eq!(paused["event"]["phaseId"], current_phase_id);
    assert_eq!(paused["event"]["phase"], "pomodoro");

    commands
        .send(r#"{"protocolVersion":1,"command":{"type":"quit"}}"#.to_owned())
        .unwrap();
    assert_eq!(receive()["event"]["type"], "stopped");
    worker.join().unwrap();
}
