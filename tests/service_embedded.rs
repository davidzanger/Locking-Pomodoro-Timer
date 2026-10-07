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
