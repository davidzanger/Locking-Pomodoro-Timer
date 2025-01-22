#![warn(missing_docs)]
#![doc = include_str!("../README.md")]
use std::path::PathBuf;

use crate::pomodoro_options::{
    read_options_from_json, write_default_options_to_json_next_to_executable,
};
use crate::pomodoro_options::{PomodoroOptions, PomodoroOptionsError};
use crate::cli_utilities::start_pomodoro;
mod input_handler;
mod message_creator;
mod pomo_info;
mod pomodoro_options;
mod timer;
mod cli_utilities;
mod end_events;
/// The main entry point of the program.
///
/// This function initializes the logger, reads the Pomodoro options from a JSON file,
/// and starts the Pomodoro timer. If the options file is not found, it writes default
/// options to a new JSON file and informs the user.
///
/// # Panics
/// This function will panic if it fails to write default options to the JSON file.
fn main() {
    // Initialize the logger
    let logging_config_file = PathBuf::from("pomodoro_logging.yaml");
    if logging_config_file.is_file() {
        log4rs::init_file(logging_config_file, Default::default()).unwrap();
    }
    // Read the JSON file
    let data = read_options_from_json(None);
    let json_data = match data {
        Ok(json_data) => json_data,
        Err(e) => {
            // Handle the error when the option file is not found
            match e.downcast_ref::<PomodoroOptionsError>() {
                Some(PomodoroOptionsError::OptionFileNotFound(_)) => {
                    // Write default options to JSON file
                    write_default_options_to_json_next_to_executable()
                        .expect("Failed to write default options to JSON file.");

                    // Print a message for first-time users
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
                    // Print the error and use default options
                    eprintln!("Error: {:#}", e);
                    eprintln!("Using default options.");
                    PomodoroOptions::default()
                }
            }
        }
    };

    // Start the Pomodoro timer
    start_pomodoro(&json_data)
    // if let Err(e) = std::panic::catch_unwind(|| start_pomodoro(&json_data)) {
        // log::error!("An error occurred: {:#?}", e);
    // }
}
