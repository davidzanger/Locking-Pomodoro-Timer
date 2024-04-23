use crate::end_events::{display_screensaver_and_lock_screen, play_sound};
use crate::message_creator::{
    generate_print_message_before_additional_break, generate_print_message_before_break,
    generate_print_message_before_pomodoro,
};
use crate::pomo_info::PomoInfo;
use crate::pomodoro_options::read_options_from_json;
use crate::timer::Timer;
use indicatif::{ProgressBar, ProgressStyle};
use log::debug;
use pomodoro_options::PomodoroOptions;
use std::path::PathBuf;
use std::thread;
use std::time::Duration;
mod end_events;
mod message_creator;
mod pomo_info;
mod pomodoro_options;
mod timer;
mod input_handler;


fn main() {
    env_logger::init();
    // Read the JSON file
    let data = read_options_from_json(None);
    let json_data = data.unwrap_or_else(|e| {
        eprintln!("Error: {:#}", e);
        eprintln!("Using default options.");
        PomodoroOptions::default()
    });
    start_pomodoro(&json_data);
}

fn start_pomodoro(options: &PomodoroOptions) {
    // Use the imported data
    println!("Options: {}", serde_json::to_string_pretty(options).unwrap());
    let duration: Duration = Duration::from_secs((options.duration_pomodoro * 60) as u64);
    let additional_duration: Duration =
        Duration::from_secs((options.additional_duration * 60) as u64);

    let mut counter = 0;
    let mut input = String::new();
    let end_event = || {
        play_sound(PathBuf::from(options.filepath_sound.clone()));
    };
    debug!("Starting input stream.");
    let receiver = input_handler::start_input_stream();
    loop {
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
                    display_screensaver_and_lock_screen,
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

fn execute_timer<F>(duration: Duration, receiver: &std::sync::mpsc::Receiver<String>, end_event: F)
where
    F: Fn(),
{
    time_with_progress_bar(duration, &receiver, end_event);
    println!("Times up!");
}

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

