use crate::end_events::start_end_event;
use crate::pomo_info::PomoInfo;
use crate::pomodoro_options::{
    read_options_from_json_silent, write_default_options_to_json_next_to_executable,
    PomodoroOptions, PomodoroOptionsError,
};
use crate::timer::Timer;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::io::{self, BufRead, BufWriter, Write};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::thread;
use std::time::Duration;

pub(crate) const PROTOCOL_VERSION: u32 = 1;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Request {
    pub(crate) protocol_version: u32,
    pub(crate) command: Command,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub(crate) enum Command {
    Start,
    Pause,
    Resume,
    Skip { seconds: u64 },
    Cancel,
    Quit,
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

#[cfg(test)]
mod tests {
    use super::{
        run_session, Command, Event, Request, Response, TimerPhase, TimerStatus, PROTOCOL_VERSION,
    };
    use crate::pomodoro_options::PomodoroOptions;
    use std::sync::mpsc;

    #[test]
    fn request_uses_versioned_camel_case_protocol() {
        let request: Request =
            serde_json::from_str(r#"{"protocolVersion":1,"command":{"type":"skip","seconds":60}}"#)
                .unwrap();

        assert_eq!(request.protocol_version, PROTOCOL_VERSION);
        assert!(matches!(request.command, Command::Skip { seconds: 60 }));
    }

    #[test]
    fn state_event_serializes_for_flutter() {
        let response = Response {
            protocol_version: PROTOCOL_VERSION,
            event: Event::State {
                status: TimerStatus::Running,
                phase: Some(TimerPhase::Pomodoro),
                elapsed_seconds: 12,
                duration_seconds: 1500,
                completed_pomodoros: 0,
            },
        };

        assert_eq!(
            serde_json::to_string(&response).unwrap(),
            r#"{"protocolVersion":1,"event":{"type":"state","status":"running","phase":"pomodoro","elapsedSeconds":12,"durationSeconds":1500,"completedPomodoros":0}}"#
        );
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

fn run_session(
    options: PomodoroOptions,
    receiver: Receiver<String>,
    writer: &mut impl Write,
) -> io::Result<()> {
    send_event(writer, Event::Ready)?;
    let mut session = Session::new(options, receiver);
    session.emit_state(writer)?;
    loop {
        match session.receiver.recv_timeout(Duration::from_millis(100)) {
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
    last_reported_elapsed: u64,
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
            last_reported_elapsed: 0,
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
            Command::Cancel => {
                self.timer.take();
                self.status = TimerStatus::Idle;
                self.phase = None;
                self.duration = Duration::ZERO;
                self.pending_start = None;
                self.completed_pomodoros = 0;
                self.last_reported_elapsed = 0;
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
        self.timer = Some(timer);
        self.duration = duration;
        self.status = TimerStatus::Running;
        self.pending_start = None;
        self.last_reported_elapsed = 0;
        self.emit_state(writer)
    }

    fn wait_for_start(&mut self, phase: TimerPhase, writer: &mut impl Write) -> io::Result<()> {
        self.phase = Some(phase);
        self.timer = None;
        self.duration = self.phase_duration(phase);
        self.status = TimerStatus::Waiting;
        self.pending_start = Some(phase);
        self.last_reported_elapsed = 0;
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
            },
        )?;

        match phase {
            TimerPhase::Pomodoro => {
                start_end_event(&self.options.end_event_pomodoro);
                if self.options.additional_duration > 0 {
                    self.start_phase(TimerPhase::AdditionalPomodoro, writer)
                } else {
                    self.after_additional_phase(writer)
                }
            }
            TimerPhase::AdditionalPomodoro => {
                start_end_event(&self.options.end_event_additional_pomodoro);
                self.after_additional_phase(writer)
            }
            TimerPhase::ShortBreak | TimerPhase::LongBreak => {
                start_end_event(&self.options.end_event_pomodoro);
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

    fn after_break(&mut self, writer: &mut impl Write) -> io::Result<()> {
        if self.options.auto_start_pomodoro {
            self.start_phase(TimerPhase::Pomodoro, writer)
        } else {
            self.wait_for_start(TimerPhase::Pomodoro, writer)
        }
    }

    fn next_break_phase(&self) -> Option<TimerPhase> {
        if self.options.interval_long_break <= 0 {
            return (self.options.duration_short_break > 0).then_some(TimerPhase::ShortBreak);
        }
        let counter = self
            .completed_pomodoros
            .saturating_sub(1)
            .min(i32::MAX as u32) as i32;
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
