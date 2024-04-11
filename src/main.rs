use crate::pomodoro_options::read_options_from_json;
use crate::timer::Timer;
use indicatif::{ProgressBar, ProgressStyle};
use pomodoro_options::PomodoroOptions;
use std::path::PathBuf;
use std::time::Duration;
use std::{thread, time};

use crate::end_events::{display_screensaver_and_lock_screen, play_sound};
mod end_events;
mod pomodoro_options;
mod timer;

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
    println!("{:#?}", options);
    let duration: Duration = Duration::from_secs((options.duration_pomodoro * 60) as u64);
    let additional_duration: Duration =
        Duration::from_secs((options.additional_duration * 60) as u64);

    let mut counter = 0;
    let mut input = String::new();
    let end_event = || {
        play_sound(PathBuf::from(options.filepath_sound.clone()));
    };
    loop {
        if counter != 0 && !options.auto_start_pomodoro {
            input.clear();
            println!("Do you want to repeat the timer? (Press enter to repeat, type anything else and press enter to exit)");
            std::io::stdin()
                .read_line(&mut input)
                .expect("Failed to read input.");
        } else {
            input = "".to_string();
        }
        if input.trim().is_empty() {
            execute_timer(duration, additional_duration, end_event);
            let break_duration: Duration;
            if counter % options.interval_long_break == options.interval_long_break - 1 {
                break_duration = Duration::from_secs((options.duration_long_break * 60) as u64)
            } else {
                break_duration = Duration::from_secs((options.duration_short_break * 60) as u64)
            };
            if options.auto_start_break {
                println!(
                    "Starting the break of {:.0} minutes",
                    break_duration.as_secs() / 60
                );
            } else {
                println!(
                    "Press enter to start the break of {:.0} minutes",
                    break_duration.as_secs() / 60
                );
                std::io::stdin()
                    .read_line(&mut input)
                    .expect("Failed to read input.");
            }
            if !break_duration.is_zero() {
                execute_timer(break_duration, Duration::from_secs(0), end_event);
            }
        } else {
            break;
        }
        counter += 1;
    }
}

fn execute_timer<F>(duration: Duration, additional_duration: Duration, end_event: F)
where
    F: Fn(),
{
    println!("Timer started for {} minutes. ", duration.as_secs() / 60);
    time_with_progress_bar(duration);

    end_event();
    println!("Times up!");
    if !additional_duration.is_zero() {
        println!(
            "You got an extra {} minutes.",
            additional_duration.as_secs() / 60
        );
        time_with_progress_bar(additional_duration);
        display_screensaver_and_lock_screen();
    }
}

fn time_with_progress_bar(duration: Duration) {
    let timer = Timer::new(duration);
    let bar = ProgressBar::new(duration.as_secs());
    bar.set_style(
        ProgressStyle::with_template("[{elapsed}/{eta}] {wide_bar:.cyan/blue} ").unwrap(),
    );
    let delta: u64 = 1;
    timer.start();
    while timer.get_elapsed_time() < duration {
        thread::sleep(Duration::from_secs(delta));
        bar.inc(delta)
    }
    bar.finish();
}
