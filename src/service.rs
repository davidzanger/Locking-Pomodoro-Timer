use crate::end_events::start_end_event;
use crate::end_events::EndEvent;
use crate::pomo_info::PomoInfo;
use crate::pomodoro_options::{
    read_options_from_json_silent, write_default_options_to_json_next_to_executable,
    PomodoroOptions, PomodoroOptionsError,
};
use crate::timer::Timer;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::io::{self, BufRead, BufWriter, Write};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::thread;
use std::time::{Duration, Instant};

pub(crate) const PROTOCOL_VERSION: u32 = 1;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Request {
    pub(crate) protocol_version: u32,
    pub(crate) command: Command,
}

#[derive(Debug, Deserialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub(crate) enum Command {
    Start,
    Pause,
    Resume,
    Skip { seconds: u64 },
    NextPhase { expected_phase_id: Option<u64> },
    ContinueAdditional,
    StartBreak,
    UpdateSettings { settings: TimerOptions },
    Cancel,
    Quit,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TimerOptions {
    duration_pomodoro: i32,
    additional_duration: i32,
    #[serde(default = "crate::pomodoro_options::enabled_by_default")]
    repeat_additional_pomodoro: bool,
    #[serde(default = "crate::pomodoro_options::enabled_by_default")]
    ask_before_break: bool,
    duration_short_break: i32,
    duration_long_break: i32,
    interval_long_break: i32,
    auto_start_break: bool,
    auto_start_pomodoro: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Response {
    pub(crate) protocol_version: u32,
    pub(crate) event: Event,
}

#[derive(Debug, Serialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub(crate) enum Event {
    Ready,
    State {
        status: TimerStatus,
        phase: Option<TimerPhase>,
        next_step: NextStep,
        phase_id: u64,
        break_decision_id: u64,
        break_decision_pending: bool,
        elapsed_seconds: u64,
        duration_seconds: u64,
        completed_pomodoros: u32,
    },
    AwaitingStart {
        phase: TimerPhase,
        completed_pomodoros: u32,
    },
    PhaseCompleted {
        phase: TimerPhase,
        completed_pomodoros: u32,
        phase_id: u64,
        skipped: bool,
    },
    Error {
        message: String,
    },
    Stopped,
}

#[derive(Debug, Serialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) enum TimerStatus {
    Idle,
    Running,
    Paused,
    Waiting,
}

#[derive(Debug, Serialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) enum TimerPhase {
    Pomodoro,
    AdditionalPomodoro,
    ShortBreak,
    LongBreak,
}

#[derive(Debug, Serialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) enum NextStep {
    Pomodoro,
    AdditionalPomodoro,
    ShortBreak,
    LongBreak,
}

#[cfg(test)]
mod tests {
    use super::{
        run_session, Command, Event, NextStep, Request, Response, Session, TimerPhase, TimerStatus,
        PROTOCOL_VERSION,
    };
    use crate::end_events::EndEvent;
    use crate::pomodoro_options::PomodoroOptions;
    use crate::timer::Timer;
    use std::io::{self, Write};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::mpsc;
    use std::thread;
    use std::time::{Duration, Instant};

    static REMINDERS: AtomicUsize = AtomicUsize::new(0);
    static END_EVENTS: AtomicUsize = AtomicUsize::new(0);

    fn count_reminder(_: &EndEvent) {
        REMINDERS.fetch_add(1, Ordering::Relaxed);
    }

    fn count_end_event(_: &EndEvent) {
        END_EVENTS.fetch_add(1, Ordering::Relaxed);
    }

    fn test_session(options: PomodoroOptions) -> (Session, mpsc::Sender<String>) {
        let (sender, receiver) = mpsc::channel();
        let mut session = Session::new(options, receiver);
        session.end_event_runner = |_| {};
        (session, sender)
    }

    fn command_line(command: &str) -> String {
        format!(
            r#"{{"protocolVersion":{},"command":{}}}"#,
            PROTOCOL_VERSION, command
        )
    }

    fn output_events(output: &[u8]) -> Vec<serde_json::Value> {
        std::str::from_utf8(output)
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect()
    }

    struct FailingWriter;

    impl Write for FailingWriter {
        fn write(&mut self, _: &[u8]) -> io::Result<usize> {
            Err(io::Error::new(io::ErrorKind::BrokenPipe, "closed"))
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn request_uses_versioned_camel_case_protocol() {
        let request: Request =
            serde_json::from_str(r#"{"protocolVersion":1,"command":{"type":"skip","seconds":60}}"#)
                .unwrap();

        assert_eq!(request.protocol_version, PROTOCOL_VERSION);
        assert!(matches!(request.command, Command::Skip { seconds: 60 }));
        let legacy: Request =
            serde_json::from_str(r#"{"protocolVersion":1,"command":{"type":"nextPhase"}}"#)
                .unwrap();
        assert!(matches!(
            legacy.command,
            Command::NextPhase {
                expected_phase_id: None
            }
        ));
    }

    #[test]
    fn settings_updates_take_effect_after_the_active_phase() {
        let (mut session, _) = test_session(PomodoroOptions::default());
        let mut output = Vec::new();
        session
            .handle_line(&command_line(r#"{"type":"start"}"#), &mut output)
            .unwrap();

        session
            .handle_line(
                &command_line(
                    r#"{"type":"updateSettings","settings":{"durationPomodoro":30,"additionalDuration":0,"durationShortBreak":2,"durationLongBreak":10,"intervalLongBreak":3,"autoStartBreak":false,"autoStartPomodoro":false}}"#,
                ),
                &mut output,
            )
            .unwrap();

        assert_eq!(session.duration, Duration::from_secs(25 * 60));
        session.complete_phase(&mut output).unwrap();
        assert_eq!(session.status, TimerStatus::Waiting);
        assert_eq!(session.phase, Some(TimerPhase::ShortBreak));
        assert_eq!(session.duration, Duration::from_secs(2 * 60));
    }

    #[test]
    fn invalid_settings_update_is_rejected_without_mutation() {
        let (mut session, _) = test_session(PomodoroOptions::default());
        let mut output = Vec::new();

        session
            .handle_line(
                &command_line(
                    r#"{"type":"updateSettings","settings":{"durationPomodoro":0,"additionalDuration":0,"durationShortBreak":5,"durationLongBreak":15,"intervalLongBreak":4,"autoStartBreak":true,"autoStartPomodoro":true}}"#,
                ),
                &mut output,
            )
            .unwrap();

        assert_eq!(session.options.duration_pomodoro, 25);
        assert_eq!(
            output_events(&output)[0]["event"]["message"],
            "Timer settings are out of range"
        );
    }

    #[test]
    fn negative_long_break_interval_update_is_rejected_without_mutation() {
        let (mut session, _) = test_session(PomodoroOptions::default());
        let mut output = Vec::new();
        session
            .handle_line(
                &command_line(
                    r#"{"type":"updateSettings","settings":{"durationPomodoro":25,"additionalDuration":5,"durationShortBreak":5,"durationLongBreak":15,"intervalLongBreak":-1,"autoStartBreak":true,"autoStartPomodoro":true}}"#,
                ),
                &mut output,
            )
            .unwrap();

        assert_eq!(session.options.interval_long_break, 4);
        assert_eq!(
            output_events(&output)[0]["event"]["message"],
            "Timer settings are out of range"
        );
    }

    #[test]
    fn state_event_serializes_for_flutter() {
        let response = Response {
            protocol_version: PROTOCOL_VERSION,
            event: Event::State {
                status: TimerStatus::Running,
                phase: Some(TimerPhase::Pomodoro),
                next_step: NextStep::AdditionalPomodoro,
                phase_id: 1,
                break_decision_id: 0,
                break_decision_pending: false,
                elapsed_seconds: 12,
                duration_seconds: 1500,
                completed_pomodoros: 0,
            },
        };

        assert_eq!(
            serde_json::to_string(&response).unwrap(),
            r#"{"protocolVersion":1,"event":{"type":"state","status":"running","phase":"pomodoro","nextStep":"additionalPomodoro","phaseId":1,"breakDecisionId":0,"breakDecisionPending":false,"elapsedSeconds":12,"durationSeconds":1500,"completedPomodoros":0}}"#
        );
    }

    #[test]
    fn next_step_reflects_disabled_and_repeating_additional_pomodoros() {
        let (_, receiver) = mpsc::channel();
        let mut options = PomodoroOptions::default();
        options.additional_duration = 0;
        let mut session = Session::new(options, receiver);
        session.phase = Some(TimerPhase::Pomodoro);
        session.status = TimerStatus::Running;
        assert_eq!(session.next_step(), NextStep::ShortBreak);

        session.options.additional_duration = 5;
        assert_eq!(session.next_step(), NextStep::AdditionalPomodoro);
        session.phase = Some(TimerPhase::AdditionalPomodoro);
        session.completed_pomodoros = 1;
        assert_eq!(session.next_step(), NextStep::AdditionalPomodoro);
        session.options.ask_before_break = false;
        assert_eq!(session.next_step(), NextStep::AdditionalPomodoro);
        session.options.additional_duration = 0;
        assert_eq!(session.next_step(), NextStep::ShortBreak);
        session.options.additional_duration = 5;
        session.options.repeat_additional_pomodoro = false;
        assert_eq!(session.next_step(), NextStep::ShortBreak);
    }

    #[test]
    fn service_loop_emits_jsonl_states_for_commands() {
        let (sender, receiver) = mpsc::channel();
        for command in ["start", "pause", "resume", "cancel", "quit"] {
            sender
                .send(format!(
                    r#"{{"protocolVersion":1,"command":{{"type":"{}"}}}}"#,
                    command
                ))
                .unwrap();
        }
        drop(sender);

        let mut output = Vec::new();
        run_session(PomodoroOptions::default(), receiver, &mut output).unwrap();
        let responses: Vec<serde_json::Value> = std::str::from_utf8(&output)
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        let event_types: Vec<&str> = responses
            .iter()
            .map(|response| response["event"]["type"].as_str().unwrap())
            .collect();
        let statuses: Vec<&str> = responses
            .iter()
            .filter_map(|response| response["event"]["status"].as_str())
            .collect();

        assert_eq!(event_types.first(), Some(&"ready"));
        assert_eq!(event_types.last(), Some(&"stopped"));
        assert!(statuses.contains(&"running"));
        assert!(statuses.contains(&"paused"));
        assert!(statuses.contains(&"idle"));
        assert!(responses.iter().all(|response| {
            response["protocolVersion"].as_u64() == Some(PROTOCOL_VERSION as u64)
        }));
    }

    #[test]
    fn invalid_requests_and_protocol_versions_emit_errors() {
        let (mut session, _) = test_session(PomodoroOptions::default());
        let mut output = Vec::new();

        assert!(!session.handle_line("not json", &mut output).unwrap());
        assert!(!session
            .handle_line(
                r#"{"protocolVersion":2,"command":{"type":"quit"}}"#,
                &mut output
            )
            .unwrap());

        let events = output_events(&output);
        assert!(events[0]["event"]["message"]
            .as_str()
            .unwrap()
            .starts_with("Invalid request:"));
        assert_eq!(
            events[1]["event"]["message"],
            "Unsupported protocol version 2; expected 1"
        );
    }

    #[test]
    fn commands_in_incompatible_states_emit_errors() {
        let (mut session, _) = test_session(PomodoroOptions::default());
        let mut output = Vec::new();

        for command in [
            r#"{"type":"pause"}"#,
            r#"{"type":"resume"}"#,
            r#"{"type":"skip","seconds":1}"#,
        ] {
            session
                .handle_line(&command_line(command), &mut output)
                .unwrap();
        }

        let events = output_events(&output);
        assert_eq!(events[0]["event"]["message"], "Timer is not running");
        assert_eq!(events[1]["event"]["message"], "Timer is not paused");
        assert_eq!(events[2]["event"]["message"], "No active timer to skip");
    }

    #[test]
    fn start_pause_resume_skip_and_cancel_follow_state_transitions() {
        let (mut session, _) = test_session(PomodoroOptions::default());
        let mut output = Vec::new();

        session
            .handle_line(&command_line(r#"{"type":"start"}"#), &mut output)
            .unwrap();
        session
            .handle_line(&command_line(r#"{"type":"pause"}"#), &mut output)
            .unwrap();
        session
            .handle_line(
                &command_line(r#"{"type":"skip","seconds":40}"#),
                &mut output,
            )
            .unwrap();
        session
            .handle_line(&command_line(r#"{"type":"resume"}"#), &mut output)
            .unwrap();
        session
            .handle_line(&command_line(r#"{"type":"cancel"}"#), &mut output)
            .unwrap();

        assert_eq!(session.status, TimerStatus::Idle);
        assert_eq!(session.phase, None);
        assert_eq!(session.duration, Duration::ZERO);
        assert_eq!(session.completed_pomodoros, 0);
        assert_eq!(session.last_reported_elapsed, 0);
        assert!(session.timer.is_none());
        let events = output_events(&output);
        assert!(events
            .iter()
            .any(|event| event["event"]["elapsedSeconds"] == 40));
        assert_eq!(events.last().unwrap()["event"]["status"], "idle");
    }

    #[test]
    fn next_phase_advances_without_running_end_events() {
        END_EVENTS.store(0, Ordering::Relaxed);
        let (mut session, _) = test_session(PomodoroOptions::default());
        session.end_event_runner = count_end_event;
        let mut output = Vec::new();
        session
            .handle_line(&command_line(r#"{"type":"start"}"#), &mut output)
            .unwrap();
        session
            .handle_line(
                &command_line(r#"{"type":"nextPhase","expectedPhaseId":1}"#),
                &mut output,
            )
            .unwrap();
        assert_eq!(session.phase, Some(TimerPhase::AdditionalPomodoro));
        session
            .handle_line(
                &command_line(r#"{"type":"nextPhase","expectedPhaseId":2}"#),
                &mut output,
            )
            .unwrap();
        assert_eq!(session.phase, Some(TimerPhase::ShortBreak));
        assert_eq!(END_EVENTS.load(Ordering::Relaxed), 0);
        assert_eq!(session.completed_pomodoros, 1);
    }

    #[test]
    fn starting_active_timer_is_rejected_and_zero_length_phase_is_not_started() {
        let (mut session, _) = test_session(PomodoroOptions::default());
        let mut output = Vec::new();
        session
            .handle_line(&command_line(r#"{"type":"start"}"#), &mut output)
            .unwrap();
        session
            .handle_line(&command_line(r#"{"type":"start"}"#), &mut output)
            .unwrap();

        let events = output_events(&output);
        assert_eq!(
            events.last().unwrap()["event"]["message"],
            "Timer is already active"
        );

        let mut options = PomodoroOptions::default();
        options.duration_pomodoro = 0;
        let (mut session, _) = test_session(options);
        let mut output = Vec::new();
        session
            .handle_line(&command_line(r#"{"type":"start"}"#), &mut output)
            .unwrap();
        assert_eq!(
            output_events(&output)[0]["event"]["message"],
            "Cannot start a zero-length phase"
        );
        assert_eq!(session.status, TimerStatus::Idle);
    }

    #[test]
    fn completion_of_fourth_pomodoro_starts_long_break() {
        let mut options = PomodoroOptions::default();
        options.repeat_additional_pomodoro = false;
        let (mut session, _) = test_session(options);
        let mut output = Vec::new();
        session.completed_pomodoros = 3;
        session.phase = Some(TimerPhase::Pomodoro);
        session.complete_phase(&mut output).unwrap();
        assert_eq!(session.phase, Some(TimerPhase::AdditionalPomodoro));
        session.complete_phase(&mut output).unwrap();

        assert_eq!(session.completed_pomodoros, 4);
        assert_eq!(session.phase, Some(TimerPhase::LongBreak));
        assert_eq!(session.status, TimerStatus::Running);
        assert_eq!(session.duration, Duration::from_secs(15 * 60));
        let events = output_events(&output);
        assert_eq!(events[0]["event"]["type"], "phaseCompleted");
        assert_eq!(events[0]["event"]["phase"], "pomodoro");
        assert_eq!(events[1]["event"]["phase"], "additionalPomodoro");
    }

    #[test]
    fn completing_additional_session_can_wait_for_break_start() {
        let mut options = PomodoroOptions::default();
        options.auto_start_break = false;
        options.repeat_additional_pomodoro = false;
        let (mut session, _) = test_session(options);
        let mut output = Vec::new();
        session.completed_pomodoros = 1;
        session.phase = Some(TimerPhase::AdditionalPomodoro);
        session.complete_phase(&mut output).unwrap();

        assert_eq!(session.status, TimerStatus::Waiting);
        assert_eq!(session.pending_start, Some(TimerPhase::ShortBreak));
        assert!(output_events(&output)
            .iter()
            .any(|event| event["event"]["type"] == "awaitingStart"));

        session
            .handle_line(&command_line(r#"{"type":"start"}"#), &mut output)
            .unwrap();
        assert_eq!(session.status, TimerStatus::Running);
        assert_eq!(session.phase, Some(TimerPhase::ShortBreak));
        assert!(session.pending_start.is_none());
    }

    #[test]
    fn additional_rounds_continue_while_a_decision_is_open() {
        let (mut session, _) = test_session(PomodoroOptions::default());
        let mut output = Vec::new();
        session.completed_pomodoros = 1;
        session.phase = Some(TimerPhase::AdditionalPomodoro);
        session.complete_phase(&mut output).unwrap();
        assert_eq!(session.status, TimerStatus::Running);
        assert!(session.timer.is_some());
        assert!(session.break_decision_pending);
        assert_eq!(session.break_decision_id, 1);

        session
            .handle_line(
                &command_line(r#"{"type":"continueAdditional"}"#),
                &mut output,
            )
            .unwrap();
        assert_eq!(session.phase, Some(TimerPhase::AdditionalPomodoro));
        assert_eq!(session.status, TimerStatus::Running);
        assert!(!session.break_decision_pending);
        assert_eq!(session.completed_pomodoros, 1);

        session.complete_phase(&mut output).unwrap();
        assert_eq!(session.break_decision_id, 2);
        assert!(session.break_decision_pending);
        session
            .handle_line(&command_line(r#"{"type":"startBreak"}"#), &mut output)
            .unwrap();
        assert_eq!(session.phase, Some(TimerPhase::ShortBreak));
        assert_eq!(session.status, TimerStatus::Running);
        assert_eq!(session.completed_pomodoros, 1);
    }

    #[test]
    fn break_requested_during_end_action_starts_without_another_additional_round() {
        let (mut session, sender) = test_session(PomodoroOptions::default());
        session.completed_pomodoros = 1;
        session
            .start_phase(TimerPhase::AdditionalPomodoro, &mut Vec::new())
            .unwrap();
        session.complete_phase(&mut Vec::new()).unwrap();
        assert!(session.break_decision_pending);
        sender
            .send(command_line(r#"{"type":"startBreak"}"#))
            .unwrap();

        session.complete_phase(&mut Vec::new()).unwrap();
        assert_eq!(session.phase, Some(TimerPhase::ShortBreak));
        assert_eq!(session.phase_id, 3);
    }

    #[test]
    fn disabling_repetition_during_an_additional_round_ends_at_the_due_break() {
        let (mut session, _) = test_session(PomodoroOptions::default());
        session.completed_pomodoros = 1;
        session
            .start_phase(TimerPhase::AdditionalPomodoro, &mut Vec::new())
            .unwrap();
        session.complete_phase(&mut Vec::new()).unwrap();
        assert!(session.break_decision_pending);
        session.options.repeat_additional_pomodoro = false;

        session.complete_phase(&mut Vec::new()).unwrap();
        assert_eq!(session.phase, Some(TimerPhase::ShortBreak));
        assert!(!session.break_decision_pending);
    }

    #[test]
    fn break_decision_can_end_a_paused_additional_round_without_its_end_event() {
        let (mut session, _) = test_session(PomodoroOptions::default());
        session.completed_pomodoros = 1;
        let mut output = Vec::new();
        session
            .start_phase(TimerPhase::AdditionalPomodoro, &mut output)
            .unwrap();
        session.complete_phase(&mut output).unwrap();
        session
            .handle_line(&command_line(r#"{"type":"pause"}"#), &mut output)
            .unwrap();
        session
            .handle_line(&command_line(r#"{"type":"startBreak"}"#), &mut output)
            .unwrap();
        assert_eq!(session.phase, Some(TimerPhase::ShortBreak));
        assert_eq!(
            output_events(&output)
                .iter()
                .filter(|event| event["event"]["type"] == "phaseCompleted")
                .count(),
            1
        );
    }

    #[test]
    fn repeated_additional_rounds_without_a_question_start_automatically() {
        let mut options = PomodoroOptions::default();
        options.ask_before_break = false;
        let (mut session, _) = test_session(options);
        session.completed_pomodoros = 1;
        session.phase = Some(TimerPhase::AdditionalPomodoro);
        session.complete_phase(&mut Vec::new()).unwrap();
        assert_eq!(session.phase, Some(TimerPhase::AdditionalPomodoro));
        assert_eq!(session.status, TimerStatus::Running);
        assert_eq!(session.completed_pomodoros, 1);
    }

    #[test]
    fn skipping_an_additional_round_starts_the_break_even_when_auto_start_is_off() {
        let mut options = PomodoroOptions::default();
        options.auto_start_break = false;
        let (mut session, _) = test_session(options);
        session.completed_pomodoros = 1;
        session
            .start_phase(TimerPhase::AdditionalPomodoro, &mut Vec::new())
            .unwrap();
        session
            .handle_line(
                &command_line(r#"{"type":"nextPhase","expectedPhaseId":1}"#),
                &mut Vec::new(),
            )
            .unwrap();
        assert_eq!(session.phase, Some(TimerPhase::ShortBreak));
        assert_eq!(session.status, TimerStatus::Running);
    }

    #[test]
    fn stale_skip_does_not_advance_the_new_additional_round() {
        let (mut session, _) = test_session(PomodoroOptions::default());
        session.completed_pomodoros = 1;
        session
            .start_phase(TimerPhase::AdditionalPomodoro, &mut Vec::new())
            .unwrap();
        let expired_phase_id = session.phase_id;
        session.complete_phase(&mut Vec::new()).unwrap();
        assert_eq!(session.phase, Some(TimerPhase::AdditionalPomodoro));
        assert_ne!(session.phase_id, expired_phase_id);

        let mut output = Vec::new();
        session
            .handle_line(
                &command_line(&format!(
                    r#"{{"type":"nextPhase","expectedPhaseId":{expired_phase_id}}}"#
                )),
                &mut output,
            )
            .unwrap();
        assert_eq!(session.phase, Some(TimerPhase::AdditionalPomodoro));
        assert_eq!(output_events(&output)[0]["event"]["type"], "error");
    }

    #[test]
    fn zero_length_due_break_bypasses_the_repeat_loop() {
        let mut options = PomodoroOptions::default();
        options.duration_short_break = 0;
        let (mut session, _) = test_session(options);
        session.phase = Some(TimerPhase::Pomodoro);
        session.complete_phase(&mut Vec::new()).unwrap();
        assert_eq!(session.phase, Some(TimerPhase::Pomodoro));
        assert_eq!(session.completed_pomodoros, 1);
    }

    #[test]
    fn completing_break_can_wait_for_next_pomodoro() {
        let mut options = PomodoroOptions::default();
        options.auto_start_pomodoro = false;
        let (mut session, _) = test_session(options);
        let mut output = Vec::new();
        session.phase = Some(TimerPhase::ShortBreak);
        session.complete_phase(&mut output).unwrap();

        assert_eq!(session.status, TimerStatus::Waiting);
        assert_eq!(session.pending_start, Some(TimerPhase::Pomodoro));
        assert_eq!(session.phase, Some(TimerPhase::Pomodoro));
    }

    #[test]
    fn waiting_for_pomodoro_repeats_reminders_until_started() {
        REMINDERS.store(0, Ordering::Relaxed);
        let mut options = PomodoroOptions::default();
        options.auto_start_pomodoro = false;
        options.interval_reminder_after_break = 1;
        let (mut session, _) = test_session(options);
        session.end_event_runner = count_reminder;
        session.phase = Some(TimerPhase::ShortBreak);
        session.complete_phase(&mut Vec::new()).unwrap();
        REMINDERS.store(0, Ordering::Relaxed);

        session.reminder_started = Some(Instant::now() - Duration::from_secs(61));
        session.trigger_reminder_if_due();
        assert_eq!(REMINDERS.load(Ordering::Relaxed), 1);
        session.trigger_reminder_if_due();
        assert_eq!(REMINDERS.load(Ordering::Relaxed), 1);

        session
            .handle_line(&command_line(r#"{"type":"start"}"#), &mut Vec::new())
            .unwrap();
        session.reminder_started = Some(Instant::now() - Duration::from_secs(61));
        session.trigger_reminder_if_due();
        assert_eq!(REMINDERS.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn phase_helpers_handle_disabled_breaks_and_nonpositive_intervals() {
        let mut options = PomodoroOptions::default();
        options.interval_long_break = 0;
        options.duration_short_break = 0;
        options.duration_long_break = -1;
        options.additional_duration = -1;
        let (session, _) = test_session(options);

        assert_eq!(session.next_break_phase(), None);
        assert_eq!(
            session.phase_duration(TimerPhase::LongBreak),
            Duration::ZERO
        );
        assert_eq!(
            session.phase_duration(TimerPhase::AdditionalPomodoro),
            Duration::ZERO
        );

        let mut options = PomodoroOptions::default();
        options.duration_short_break = 5;
        let (session, _) = test_session(options);
        assert_eq!(session.next_break_phase(), Some(TimerPhase::ShortBreak));
    }

    #[test]
    fn elapsed_reporting_only_emits_new_seconds_for_active_phases() {
        let (mut session, _) = test_session(PomodoroOptions::default());

        assert!(!session.has_new_elapsed_second());
        session.status = TimerStatus::Running;
        session.timer = Some(Timer::new(Duration::from_secs(60)));
        assert!(!session.has_new_elapsed_second());
        session.timer.as_ref().unwrap().skip(Duration::from_secs(2));
        assert!(session.has_new_elapsed_second());
        assert!(!session.has_new_elapsed_second());
        session.status = TimerStatus::Idle;
        session.timer.as_ref().unwrap().skip(Duration::from_secs(1));
        assert!(!session.has_new_elapsed_second());
    }

    #[test]
    fn finished_timer_completion_starts_next_phase() {
        let (mut session, _) = test_session(PomodoroOptions::default());
        session.status = TimerStatus::Running;
        session.phase = Some(TimerPhase::Pomodoro);
        session.duration = Duration::from_secs(1);
        let timer = Timer::new(Duration::from_secs(1));
        timer.skip(Duration::from_secs(1));
        session.timer = Some(timer);

        assert!(session.is_finished());
        let mut output = Vec::new();
        session.complete_phase(&mut output).unwrap();

        assert_eq!(session.phase, Some(TimerPhase::AdditionalPomodoro));
        assert_eq!(session.duration, Duration::from_secs(5 * 60));
        assert_eq!(session.completed_pomodoros, 1);
    }

    #[test]
    fn quit_command_stops_service_loop() {
        let (mut session, _) = test_session(PomodoroOptions::default());
        let mut output = Vec::new();

        assert!(session
            .handle_line(&command_line(r#"{"type":"quit"}"#), &mut output)
            .unwrap());
        assert_eq!(output_events(&output)[0]["event"]["type"], "stopped");
    }

    #[test]
    fn disconnected_input_channel_emits_stopped() {
        let (sender, receiver) = mpsc::channel();
        drop(sender);
        let mut output = Vec::new();

        run_session(PomodoroOptions::default(), receiver, &mut output).unwrap();

        let events = output_events(&output);
        assert_eq!(events.first().unwrap()["event"]["type"], "ready");
        assert_eq!(events.last().unwrap()["event"]["type"], "stopped");
    }

    #[test]
    fn service_loop_continues_after_receive_timeout() {
        let (sender, receiver) = mpsc::channel();
        let input_thread = thread::spawn(move || {
            thread::sleep(Duration::from_millis(120));
            sender.send(command_line(r#"{"type":"quit"}"#)).unwrap();
        });
        let mut output = Vec::new();

        run_session(PomodoroOptions::default(), receiver, &mut output).unwrap();
        input_thread.join().unwrap();

        assert_eq!(
            output_events(&output).last().unwrap()["event"]["type"],
            "stopped"
        );
    }

    #[test]
    fn waiting_session_without_pending_phase_ignores_start() {
        let (mut session, _) = test_session(PomodoroOptions::default());
        session.status = TimerStatus::Waiting;
        let mut output = Vec::new();

        assert!(!session
            .handle_line(&command_line(r#"{"type":"start"}"#), &mut output)
            .unwrap());
        assert_eq!(session.status, TimerStatus::Waiting);
        assert!(output.is_empty());
    }

    #[test]
    fn completion_without_a_phase_is_a_noop() {
        let (mut session, _) = test_session(PomodoroOptions::default());
        let mut output = Vec::new();

        session.complete_phase(&mut output).unwrap();

        assert!(output.is_empty());
        assert_eq!(session.completed_pomodoros, 0);
    }

    #[test]
    fn completion_without_additional_or_break_starts_next_pomodoro() {
        let mut options = PomodoroOptions::default();
        options.additional_duration = 0;
        options.duration_short_break = 0;
        let (mut session, _) = test_session(options);
        session.phase = Some(TimerPhase::Pomodoro);
        let mut output = Vec::new();

        session.complete_phase(&mut output).unwrap();

        assert_eq!(session.completed_pomodoros, 1);
        assert_eq!(session.phase, Some(TimerPhase::Pomodoro));
        assert_eq!(session.status, TimerStatus::Running);
    }

    #[test]
    fn completing_break_auto_starts_next_pomodoro_when_enabled() {
        let (mut session, _) = test_session(PomodoroOptions::default());
        session.phase = Some(TimerPhase::ShortBreak);
        let mut output = Vec::new();

        session.complete_phase(&mut output).unwrap();

        assert_eq!(session.phase, Some(TimerPhase::Pomodoro));
        assert_eq!(session.status, TimerStatus::Running);
    }

    #[test]
    fn writer_failures_are_returned_to_the_caller() {
        let (mut session, _) = test_session(PomodoroOptions::default());
        let error = session
            .handle_line(&command_line(r#"{"type":"pause"}"#), &mut FailingWriter)
            .unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::InvalidData);

        let (_, receiver) = mpsc::channel();
        let error =
            run_session(PomodoroOptions::default(), receiver, &mut FailingWriter).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
    }

    #[test]
    fn commands_without_a_timer_still_update_session_status() {
        let (mut session, _) = test_session(PomodoroOptions::default());
        let mut output = Vec::new();
        session.status = TimerStatus::Running;
        session.phase = Some(TimerPhase::Pomodoro);

        assert!(!session.is_finished());
        session
            .handle_line(&command_line(r#"{"type":"pause"}"#), &mut output)
            .unwrap();
        assert_eq!(session.status, TimerStatus::Paused);
        session
            .handle_line(&command_line(r#"{"type":"resume"}"#), &mut output)
            .unwrap();
        assert_eq!(session.status, TimerStatus::Running);
        session
            .handle_line(&command_line(r#"{"type":"skip","seconds":5}"#), &mut output)
            .unwrap();
        assert_eq!(session.status, TimerStatus::Running);
        session
            .handle_line(&command_line(r#"{"type":"cancel"}"#), &mut output)
            .unwrap();
        assert_eq!(session.status, TimerStatus::Idle);
        assert_eq!(output_events(&output).len(), 4);
    }
}

pub(crate) fn run() -> Result<()> {
    let stdout = io::stdout();
    let mut writer = BufWriter::new(stdout.lock());
    let options = match load_options() {
        Ok(options) => options,
        Err(error) => {
            send_event(
                &mut writer,
                Event::Error {
                    message: format!("{:#}", error),
                },
            )?;
            return Ok(());
        }
    };

    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let stdin = io::stdin();
        for line in stdin.lock().lines() {
            match line {
                Ok(line) => {
                    if sender.send(line).is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    });

    run_session(options, receiver, &mut writer).context("Service loop failed")
}

pub(crate) fn run_embedded(receiver: Receiver<String>, sender: Sender<String>) -> Result<()> {
    let mut writer = EventChannelWriter {
        sender,
        pending: Vec::new(),
    };
    let options = match load_options() {
        Ok(options) => options,
        Err(error) => {
            send_event(
                &mut writer,
                Event::Error {
                    message: format!("{:#}", error),
                },
            )?;
            return Ok(());
        }
    };

    run_session(options, receiver, &mut writer).context("Service loop failed")
}

struct EventChannelWriter {
    sender: Sender<String>,
    pending: Vec<u8>,
}

impl Write for EventChannelWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.pending.extend_from_slice(bytes);
        while let Some(line_end) = self.pending.iter().position(|byte| *byte == b'\n') {
            let line: Vec<_> = self.pending.drain(..=line_end).collect();
            let line = String::from_utf8(line[..line.len() - 1].to_vec())
                .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
            self.sender
                .send(line)
                .map_err(|error| io::Error::new(io::ErrorKind::BrokenPipe, error))?;
        }
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn run_session(
    options: PomodoroOptions,
    receiver: Receiver<String>,
    writer: &mut impl Write,
) -> io::Result<()> {
    send_event(writer, Event::Ready)?;
    let mut session = Session::new(options, receiver);
    session.emit_state(writer)?;
    loop {
        let next_command = session
            .deferred_commands
            .pop_front()
            .map(Ok)
            .unwrap_or_else(|| session.receiver.recv_timeout(Duration::from_millis(100)));
        match next_command {
            Ok(line) => {
                if session.handle_line(&line, writer)? {
                    break;
                }
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => {
                send_event(writer, Event::Stopped)?;
                break;
            }
        }

        if session.is_finished() {
            session.complete_phase(writer)?;
        } else if session.has_new_elapsed_second() {
            session.emit_state(writer)?;
        }
        session.trigger_reminder_if_due();
    }
    Ok(())
}

fn load_options() -> Result<PomodoroOptions> {
    match read_options_from_json_silent(None) {
        Ok(options) => Ok(options),
        Err(error)
            if matches!(
                error.downcast_ref::<PomodoroOptionsError>(),
                Some(PomodoroOptionsError::OptionFileNotFound(_))
            ) =>
        {
            write_default_options_to_json_next_to_executable()
                .context("Failed to create default options file")?;
            Ok(PomodoroOptions::default())
        }
        Err(error) => Err(error),
    }
}

fn send_event(writer: &mut impl Write, event: Event) -> io::Result<()> {
    let response = Response {
        protocol_version: PROTOCOL_VERSION,
        event,
    };
    serde_json::to_writer(&mut *writer, &response)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    writer.write_all(b"\n")?;
    writer.flush()
}

struct Session {
    options: PomodoroOptions,
    receiver: Receiver<String>,
    status: TimerStatus,
    phase: Option<TimerPhase>,
    timer: Option<Timer>,
    duration: Duration,
    pending_start: Option<TimerPhase>,
    completed_pomodoros: u32,
    phase_id: u64,
    break_decision_id: u64,
    break_decision_pending: bool,
    last_reported_elapsed: u64,
    reminder_started: Option<Instant>,
    end_event_runner: fn(&EndEvent),
    deferred_commands: VecDeque<String>,
}

impl Session {
    fn new(options: PomodoroOptions, receiver: Receiver<String>) -> Self {
        Self {
            options,
            receiver,
            status: TimerStatus::Idle,
            phase: None,
            timer: None,
            duration: Duration::ZERO,
            pending_start: None,
            completed_pomodoros: 0,
            phase_id: 0,
            break_decision_id: 0,
            break_decision_pending: false,
            last_reported_elapsed: 0,
            reminder_started: None,
            end_event_runner: start_end_event,
            deferred_commands: VecDeque::new(),
        }
    }

    fn handle_line(&mut self, line: &str, writer: &mut impl Write) -> io::Result<bool> {
        let request: Request = match serde_json::from_str(line) {
            Ok(request) => request,
            Err(error) => {
                return self.send_error(writer, format!("Invalid request: {}", error));
            }
        };
        if request.protocol_version != PROTOCOL_VERSION {
            return self.send_error(
                writer,
                format!(
                    "Unsupported protocol version {}; expected {}",
                    request.protocol_version, PROTOCOL_VERSION
                ),
            );
        }

        match request.command {
            Command::Start => match self.status {
                TimerStatus::Idle => self.start_phase(TimerPhase::Pomodoro, writer)?,
                TimerStatus::Waiting => {
                    if let Some(phase) = self.pending_start.take() {
                        self.start_phase(phase, writer)?;
                    }
                }
                _ => return self.send_error(writer, "Timer is already active".to_string()),
            },
            Command::Pause => {
                if self.status != TimerStatus::Running {
                    return self.send_error(writer, "Timer is not running".to_string());
                }
                if let Some(timer) = &self.timer {
                    timer.pause();
                }
                self.status = TimerStatus::Paused;
                self.emit_state(writer)?;
            }
            Command::Resume => {
                if self.status != TimerStatus::Paused {
                    return self.send_error(writer, "Timer is not paused".to_string());
                }
                if let Some(timer) = &self.timer {
                    timer.resume();
                }
                self.status = TimerStatus::Running;
                self.emit_state(writer)?;
            }
            Command::Skip { seconds } => {
                if !matches!(self.status, TimerStatus::Running | TimerStatus::Paused) {
                    return self.send_error(writer, "No active timer to skip".to_string());
                }
                if let Some(timer) = &self.timer {
                    timer.skip(Duration::from_secs(seconds));
                }
                self.emit_state(writer)?;
            }
            Command::NextPhase { expected_phase_id } => {
                if !matches!(self.status, TimerStatus::Running | TimerStatus::Paused) {
                    return self.send_error(writer, "No active timer to advance".to_string());
                }
                if expected_phase_id.is_some_and(|phase_id| self.phase_id != phase_id) {
                    return self.send_error(writer, "Timer phase changed before skip".to_string());
                }
                self.advance_phase(writer)?;
            }
            Command::ContinueAdditional => {
                if !self.break_decision_pending {
                    return self.send_error(writer, "No break decision is pending".to_string());
                }
                self.break_decision_pending = false;
                self.emit_state(writer)?;
            }
            Command::StartBreak => {
                if !self.break_decision_pending
                    || self.phase != Some(TimerPhase::AdditionalPomodoro)
                    || !matches!(self.status, TimerStatus::Running | TimerStatus::Paused)
                {
                    return self.send_error(writer, "No break decision is pending".to_string());
                }
                self.timer.take();
                self.break_decision_pending = false;
                self.start_due_break(writer)?;
            }
            Command::UpdateSettings { settings } => {
                if settings.duration_pomodoro < 1
                    || settings.additional_duration < 0
                    || settings.duration_short_break < 0
                    || settings.duration_long_break < 0
                    || settings.interval_long_break < 0
                {
                    return self.send_error(writer, "Timer settings are out of range".to_string());
                }
                self.options.duration_pomodoro = settings.duration_pomodoro;
                self.options.additional_duration = settings.additional_duration;
                self.options.repeat_additional_pomodoro = settings.repeat_additional_pomodoro;
                self.options.ask_before_break = settings.ask_before_break;
                self.options.duration_short_break = settings.duration_short_break;
                self.options.duration_long_break = settings.duration_long_break;
                self.options.interval_long_break = settings.interval_long_break;
                self.options.auto_start_break = settings.auto_start_break;
                self.options.auto_start_pomodoro = settings.auto_start_pomodoro;
                // The bridge saves all options before sending this command.
                // Refresh actions and reminders without restarting the service.
                if let Ok(saved) = read_options_from_json_silent(None) {
                    self.options.end_event_pomodoro = saved.end_event_pomodoro;
                    self.options.end_event_additional_pomodoro =
                        saved.end_event_additional_pomodoro;
                    self.options.interval_reminder_after_break =
                        saved.interval_reminder_after_break;
                    self.options.event_reminder_after_break = saved.event_reminder_after_break;
                }
                if self.status == TimerStatus::Waiting && self.phase == Some(TimerPhase::Pomodoro) {
                    self.reminder_started = Some(Instant::now());
                }

                if self.status == TimerStatus::Waiting {
                    if let Some(phase) = self.phase {
                        self.duration = self.phase_duration(phase);
                        self.emit_state(writer)?;
                    }
                }
            }
            Command::Cancel => {
                self.timer.take();
                self.status = TimerStatus::Idle;
                self.phase = None;
                self.duration = Duration::ZERO;
                self.pending_start = None;
                self.completed_pomodoros = 0;
                self.last_reported_elapsed = 0;
                self.reminder_started = None;
                self.break_decision_pending = false;
                self.emit_state(writer)?;
            }
            Command::Quit => {
                send_event(writer, Event::Stopped)?;
                return Ok(true);
            }
        }
        Ok(false)
    }

    fn start_phase(&mut self, phase: TimerPhase, writer: &mut impl Write) -> io::Result<()> {
        let duration = self.phase_duration(phase);
        if duration.is_zero() {
            send_event(
                writer,
                Event::Error {
                    message: "Cannot start a zero-length phase".to_string(),
                },
            )?;
            return Ok(());
        }
        let timer = Timer::new(duration);
        timer.start();
        self.phase = Some(phase);
        self.phase_id = self.phase_id.wrapping_add(1);
        if phase != TimerPhase::AdditionalPomodoro {
            self.break_decision_pending = false;
        }
        self.timer = Some(timer);
        self.duration = duration;
        self.status = TimerStatus::Running;
        self.pending_start = None;
        self.last_reported_elapsed = 0;
        self.reminder_started = None;
        self.emit_state(writer)
    }

    fn wait_for_start(&mut self, phase: TimerPhase, writer: &mut impl Write) -> io::Result<()> {
        self.phase = Some(phase);
        self.phase_id = self.phase_id.wrapping_add(1);
        self.break_decision_pending = false;
        self.timer = None;
        self.duration = self.phase_duration(phase);
        self.status = TimerStatus::Waiting;
        self.pending_start = Some(phase);
        self.last_reported_elapsed = 0;
        self.reminder_started = (phase == TimerPhase::Pomodoro).then(Instant::now);
        send_event(
            writer,
            Event::AwaitingStart {
                phase,
                completed_pomodoros: self.completed_pomodoros,
            },
        )?;
        self.emit_state(writer)
    }

    fn is_finished(&self) -> bool {
        matches!(self.status, TimerStatus::Running | TimerStatus::Paused)
            && self
                .timer
                .as_ref()
                .map(|timer| timer.get_elapsed_time() >= self.duration)
                .unwrap_or(false)
    }

    fn trigger_reminder_if_due(&mut self) {
        if self.status != TimerStatus::Waiting
            || self.phase != Some(TimerPhase::Pomodoro)
            || self.options.auto_start_pomodoro
            || self.options.interval_reminder_after_break <= 0
        {
            return;
        }
        let interval = Duration::from_secs(self.options.interval_reminder_after_break as u64 * 60);
        if let Some(started) = self.reminder_started {
            if started.elapsed() >= interval {
                (self.end_event_runner)(&self.options.event_reminder_after_break);
                self.reminder_started = Some(Instant::now());
            }
        }
    }

    fn has_new_elapsed_second(&mut self) -> bool {
        if !matches!(self.status, TimerStatus::Running | TimerStatus::Paused) {
            return false;
        }
        let elapsed = self.elapsed_seconds();
        if elapsed > self.last_reported_elapsed {
            self.last_reported_elapsed = elapsed;
            return true;
        }
        false
    }

    fn elapsed_seconds(&self) -> u64 {
        self.timer
            .as_ref()
            .map(|timer| timer.get_elapsed_time().as_secs())
            .unwrap_or(0)
    }

    fn emit_state(&self, writer: &mut impl Write) -> io::Result<()> {
        send_event(
            writer,
            Event::State {
                status: self.status,
                phase: self.phase,
                next_step: self.next_step(),
                phase_id: self.phase_id,
                break_decision_id: self.break_decision_id,
                break_decision_pending: self.break_decision_pending,
                elapsed_seconds: self.elapsed_seconds(),
                duration_seconds: self.duration.as_secs(),
                completed_pomodoros: self.completed_pomodoros,
            },
        )
    }

    fn send_error(&self, writer: &mut impl Write, message: String) -> io::Result<bool> {
        send_event(writer, Event::Error { message })?;
        Ok(false)
    }

    fn complete_phase(&mut self, writer: &mut impl Write) -> io::Result<()> {
        self.finish_phase(writer, true)
    }

    fn advance_phase(&mut self, writer: &mut impl Write) -> io::Result<()> {
        self.finish_phase(writer, false)
    }

    fn finish_phase(&mut self, writer: &mut impl Write, run_end_event: bool) -> io::Result<()> {
        let Some(phase) = self.phase.take() else {
            return Ok(());
        };
        self.timer.take();
        if phase == TimerPhase::Pomodoro {
            self.completed_pomodoros = self.completed_pomodoros.saturating_add(1);
        }
        send_event(
            writer,
            Event::PhaseCompleted {
                phase,
                completed_pomodoros: self.completed_pomodoros,
                phase_id: self.phase_id,
                skipped: !run_end_event,
            },
        )?;

        match phase {
            TimerPhase::Pomodoro => {
                if run_end_event {
                    (self.end_event_runner)(&self.options.end_event_pomodoro);
                }
                if self.options.repeat_additional_pomodoro && self.next_break_phase().is_none() {
                    self.after_break(writer)
                } else if self.options.additional_duration > 0 {
                    self.start_phase(TimerPhase::AdditionalPomodoro, writer)
                } else {
                    self.after_additional_phase(writer)
                }
            }
            TimerPhase::AdditionalPomodoro => {
                if run_end_event {
                    (self.end_event_runner)(&self.options.end_event_additional_pomodoro);
                }
                if self.drain_break_request_after_end_event(writer)? {
                    self.break_decision_pending = false;
                    return self.start_due_break(writer);
                }
                if run_end_event
                    && self.options.repeat_additional_pomodoro
                    && self.options.additional_duration > 0
                    && self.next_break_phase().is_some()
                {
                    if self.options.ask_before_break {
                        self.break_decision_id = self.break_decision_id.wrapping_add(1);
                        self.break_decision_pending = true;
                    } else {
                        self.break_decision_pending = false;
                    }
                    self.start_phase(TimerPhase::AdditionalPomodoro, writer)
                } else if !run_end_event {
                    self.start_due_break(writer)
                } else {
                    self.after_additional_phase(writer)
                }
            }
            TimerPhase::ShortBreak | TimerPhase::LongBreak => {
                if run_end_event {
                    (self.end_event_runner)(&self.options.end_event_pomodoro);
                }
                self.after_break(writer)
            }
        }
    }

    fn after_additional_phase(&mut self, writer: &mut impl Write) -> io::Result<()> {
        let break_phase = self.next_break_phase();
        if let Some(phase) = break_phase {
            if self.options.auto_start_break {
                self.start_phase(phase, writer)
            } else {
                self.wait_for_start(phase, writer)
            }
        } else {
            self.after_break(writer)
        }
    }

    fn drain_break_request_after_end_event(&mut self, writer: &mut impl Write) -> io::Result<bool> {
        let mut take_break = false;
        let mut decision_open = self.break_decision_pending;
        for line in self.receiver.try_iter() {
            match serde_json::from_str::<Request>(&line) {
                Ok(Request {
                    protocol_version: PROTOCOL_VERSION,
                    command: Command::StartBreak,
                }) => {
                    if decision_open {
                        take_break = true;
                        decision_open = false;
                    } else {
                        send_event(
                            writer,
                            Event::Error {
                                message: "No break decision is pending".to_string(),
                            },
                        )?;
                    }
                }
                Ok(Request {
                    protocol_version: PROTOCOL_VERSION,
                    command: Command::ContinueAdditional,
                }) => {
                    if decision_open {
                        decision_open = false;
                    } else {
                        send_event(
                            writer,
                            Event::Error {
                                message: "No break decision is pending".to_string(),
                            },
                        )?;
                    }
                }
                _ => self.deferred_commands.push_back(line),
            }
        }
        self.break_decision_pending = decision_open;
        Ok(take_break)
    }

    fn start_due_break(&mut self, writer: &mut impl Write) -> io::Result<()> {
        if let Some(phase) = self.next_break_phase() {
            self.start_phase(phase, writer)
        } else {
            self.after_break(writer)
        }
    }

    fn after_break(&mut self, writer: &mut impl Write) -> io::Result<()> {
        if self.options.auto_start_pomodoro {
            self.start_phase(TimerPhase::Pomodoro, writer)
        } else {
            self.wait_for_start(TimerPhase::Pomodoro, writer)
        }
    }

    fn next_break_phase(&self) -> Option<TimerPhase> {
        self.break_phase_after(self.completed_pomodoros)
    }

    fn next_step(&self) -> NextStep {
        let next_break = self.break_phase_after(
            self.completed_pomodoros
                .saturating_add(u32::from(self.phase == Some(TimerPhase::Pomodoro))),
        );
        match self.phase {
            None | Some(TimerPhase::ShortBreak | TimerPhase::LongBreak) => NextStep::Pomodoro,
            Some(TimerPhase::Pomodoro)
                if self.options.additional_duration > 0
                    && (!self.options.repeat_additional_pomodoro || next_break.is_some()) =>
            {
                NextStep::AdditionalPomodoro
            }
            Some(TimerPhase::AdditionalPomodoro)
                if self.options.repeat_additional_pomodoro
                    && self.options.additional_duration > 0
                    && next_break.is_some() =>
            {
                NextStep::AdditionalPomodoro
            }
            _ => match next_break {
                Some(TimerPhase::ShortBreak) => NextStep::ShortBreak,
                Some(TimerPhase::LongBreak) => NextStep::LongBreak,
                _ => NextStep::Pomodoro,
            },
        }
    }

    fn break_phase_after(&self, completed_pomodoros: u32) -> Option<TimerPhase> {
        if self.options.interval_long_break <= 0 {
            return (self.options.duration_short_break > 0).then_some(TimerPhase::ShortBreak);
        }
        let counter = completed_pomodoros.saturating_sub(1).min(i32::MAX as u32) as i32;
        let info = PomoInfo::from_options(&self.options, counter);
        if info.break_duration.is_zero() {
            None
        } else if info.is_long_break_coming {
            Some(TimerPhase::LongBreak)
        } else {
            Some(TimerPhase::ShortBreak)
        }
    }

    fn phase_duration(&self, phase: TimerPhase) -> Duration {
        let minutes = match phase {
            TimerPhase::Pomodoro => self.options.duration_pomodoro,
            TimerPhase::AdditionalPomodoro => self.options.additional_duration,
            TimerPhase::ShortBreak => self.options.duration_short_break,
            TimerPhase::LongBreak => self.options.duration_long_break,
        };
        Duration::from_secs(minutes.max(0) as u64 * 60)
    }
}
