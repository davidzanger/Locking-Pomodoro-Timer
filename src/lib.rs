#![warn(missing_docs)]
#![doc = include_str!("../README.md")]

use std::sync::mpsc::{Receiver, Sender};

use crate::cli_utilities::start_pomodoro;
use crate::pomodoro_options::{
    read_options_from_json, write_default_options_to_json_next_to_executable, PomodoroOptions,
    PomodoroOptionsError,
};
use std::path::PathBuf;

mod cli_utilities;
mod end_events;
mod input_handler;
mod message_creator;
mod pomo_info;
mod pomodoro_options;
mod service;
mod timer;

/// Loads the timer-specific configuration as JSON for the GUI bridge.
///
/// # Errors
///
/// Returns an error if the current timer options cannot be loaded or serialized.
pub fn read_timer_settings_json() -> Result<String, String> {
    pomodoro_options::read_timer_settings_json().map_err(|error| format!("{error:#}"))
}

/// Saves timer-specific configuration supplied by the GUI bridge.
///
/// # Errors
///
/// Returns an error if the JSON is invalid, the settings fail validation, or the
/// options file cannot be read or written.
pub fn save_timer_settings_json(settings_json: &str) -> Result<(), String> {
    pomodoro_options::save_timer_settings_json(settings_json).map_err(|error| format!("{error:#}"))
}

/// Starts the interactive command-line timer.
pub fn run_cli() {
    let logging_config_file = PathBuf::from("pomodoro_logging.yaml");
    if logging_config_file.is_file() {
        log4rs::init_file(logging_config_file, Default::default()).unwrap();
    }

    let data = read_options_from_json(None);
    let json_data = match data {
        Ok(json_data) => json_data,
        Err(error) => match error.downcast_ref::<PomodoroOptionsError>() {
            Some(PomodoroOptionsError::OptionFileNotFound(_)) => {
                write_default_options_to_json_next_to_executable()
                    .expect("Failed to write default options to JSON file.");
                println!(
                        "Apparently, you are using the Locking Pomodoro Timer for the first time (at least in this folder). \
                        A file named 'pomodoro_options.json' will be created next to the executable. \
                        You can change the settings in this file. \
                        To find more information about the Locking Pomodoro Timer, visit the GitHub page: \
                        https://github.com/davidzanger/Locking-Pomodoro-Timer.git"
                    );
                PomodoroOptions::default()
            }
            None => {
                eprintln!("Error: {:#}", error);
                eprintln!("Using default options.");
                PomodoroOptions::default()
            }
        },
    };

    start_pomodoro(&json_data)
}

/// Runs the existing stdin/stdout JSON-lines service.
pub fn run_service() -> anyhow::Result<()> {
    service::run()
}

/// Runs the JSON-lines service on channels for an in-process bridge.
///
/// Commands and events use the same versioned JSON-lines format as the CLI service.
pub fn run_embedded_service(
    commands: Receiver<String>,
    events: Sender<String>,
) -> Result<(), String> {
    service::run_embedded(commands, events).map_err(|error| format!("{error:#}"))
}
