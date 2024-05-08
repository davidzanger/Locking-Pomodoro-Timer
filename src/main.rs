#![warn(missing_docs)]
#![doc = include_str!("../README.md")]
// #![doc(html_favicon_url = )]
// #![doc(html_logo_url = "https://raw.githubusercontent.com/davidzanger/Locking-Pomodoro-Timer/master/assets/logo.png")]
use crate::end_events::start_end_event;
use crate::message_creator::{
    generate_print_message_before_additional_break, generate_print_message_before_break,
    generate_print_message_before_pomodoro,
};
use crate::pomo_info::PomoInfo;
use crate::pomodoro_options::{read_options_from_json, write_default_options_to_json_next_to_executable};
use crate::timer::Timer;
use indicatif::{ProgressBar, ProgressStyle};
use log::debug;
use crate::pomodoro_options::{PomodoroOptions, PomodoroOptionsError};
use std::thread;
use std::time::Duration;
mod end_events;
mod message_creator;
mod pomo_info;
mod pomodoro_options;
mod timer;
mod input_handler;


/// The main entry point of the program.
fn main() {
    // Initialize the logger
    env_logger::init();

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
    start_pomodoro(&json_data);
}

/// Starts the Pomodoro timer.
/// 
/// The function reads the Pomodoro options from the JSON file and starts the Pomodoro timer.
/// The timer runs in a loop and can be repeated by pressing the enter key.
/// The timer can be paused and resumed by pressing the 'p' and 'r' keys respectively.
/// It can be stopped by pressing the 'q' key.
/// Also it can be exited by pressing the 'ctrl+c' key combination.
/// 
/// # Arguments
/// * `options` - The Pomodoro options.
fn start_pomodoro(options: &PomodoroOptions) {
    // Use the imported data
    println!("Options: {}", serde_json::to_string_pretty(options).unwrap());

    // Convert the duration and additional duration to `Duration` type
    let duration: Duration = Duration::from_secs((options.duration_pomodoro * 60) as u64);
    let additional_duration: Duration =
        Duration::from_secs((options.additional_duration * 60) as u64);

    let mut counter = 0;
    let mut input = String::new();
    let end_event = || {
        start_end_event(&options.end_event_pomodoro);
    };
    debug!("Starting input stream.");
    let receiver = input_handler::start_input_stream();
    loop {
        // Check if the timer should be repeated
        if counter != 0 && !options.auto_start_pomodoro {
            input.clear();
            println!("Do you want to repeat the timer? (Press enter to repeat and 'q' to quit.)");
            loop {
                let pressed_key = receiver.recv().expect("Failed to receive input.");
                if pressed_key == "q" {
                    input = "q".to_string();
                    break;
                } else if pressed_key == "\n" {
                    input = "".to_string();
                    break;
                }
            }
        } else {
            input = "".to_string();
        }

        if input.trim().is_empty() {
            let pomo_info = PomoInfo::from_options(options, counter);

            let print_message = generate_print_message_before_pomodoro(&pomo_info, &options);
            println!("{}", print_message);

            execute_timer(duration, &receiver, end_event);

            if options.additional_duration != 0 {
                let print_message =
                    generate_print_message_before_additional_break(&pomo_info, &options);
                println!("{}", print_message);
                time_with_progress_bar(
                    additional_duration,
                    &receiver,
                    || start_end_event(&options.end_event_additional_pomodoro),
                );
            }

            if !pomo_info.break_duration.is_zero() {
                if !options.auto_start_break {
                    if pomo_info.is_long_break_coming {
                        println!(
                            "Press enter to start the long break of {:.0} minutes.",
                            pomo_info.break_duration.as_secs() / 60
                        );
                    } else {
                        println!(
                            "Press enter to start the short break of {:.0} minutes.",
                            pomo_info.break_duration.as_secs() / 60
                        );
                    }
                    loop {
                        let pressed_key = receiver.recv().expect("Failed to receive input.");
                        if pressed_key == "\n" {
                            input = "".to_string();
                            break;
                        }
                    }
                }
                let print_message = generate_print_message_before_break(&pomo_info, &options);
                println!("{}", print_message);
                execute_timer(pomo_info.break_duration, &receiver, end_event);
            }
        } else {
            break;
        }
        counter += 1;
    }
}

/// Executes the timer with the specified duration.
/// 
/// # Arguments
/// * `duration` - The duration of the timer.
/// * `receiver` - The receiver for input events.
/// * `end_event` - The function to execute when the timer ends.
fn execute_timer<F: Fn()>(duration: Duration, receiver: &std::sync::mpsc::Receiver<String>, end_event: F) {
    time_with_progress_bar(duration, &receiver, end_event);
    println!("Times up!");
}

/// Executes the timer with the specified duration and displays a progress bar.
/// 
/// The timer runs in a separate thread and increments the progress bar every second.
/// It can be paused and resumed using the 'p' and 'r' keys respectively.
/// Also it can be stopped using the 'q' key.
/// 
/// # Arguments
/// * `duration` - The duration of the timer.
/// * `receiver` - The receiver for input events.
/// * `end_event` - The function to execute when the timer ends.
fn time_with_progress_bar<F: Fn()>(
    duration: Duration,
    receiver: &std::sync::mpsc::Receiver<String>,
    end_event: F,
) {
    let timer = Timer::new(duration);
    let mut bar = ProgressBar::new(duration.as_secs());
    bar.set_style(
        ProgressStyle::with_template("[{elapsed}/{eta}] {wide_bar:.cyan/blue} ").unwrap(),
    );
    let delta: u64 = 1;
    timer.start();
    println!("Press 'p' to pause, 'q' to quit current timer.");
    while timer.get_elapsed_time() < duration {
        if let Ok(input) = receiver.try_recv() {
            if input == "p" {
                timer.pause();
                println!("Timer paused.");
                println!("Press 'r' to resume, 'q' to quit current timer.");
            } else if input == "r" {
                timer.resume();
                println!("Timer resumed.");
                println!("Press 'p' to pause, 'q' to quit current timer.");
                bar = bar.with_elapsed(timer.get_elapsed_time());
                bar.reset_eta();
            } else if input == "q" {
                // Return early to not execute the end event.
                println!("Exiting the current timer.");
                return;
            } else if input == "ctrl+c" {
                println!("Exiting the program.");
                std::process::exit(0);
            } else {
                debug!("Invalid input: {}", input);
            }
            debug!("Elapsed time: {:?}", timer.get_elapsed_time());
        }
        thread::sleep(Duration::from_secs(delta));
        if !timer.is_paused() {
            bar.inc(delta)
        }
    }
    bar.finish();
    end_event();
}

