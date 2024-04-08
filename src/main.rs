use crate::pomodoro_options::read_options_from_json;
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
    // Read the JSON file
    let data = read_options_from_json(None);
    match data {
        Ok(json_data) => {
            start_pomodoro(&json_data);
        }
        Err(e) => {
            eprintln!("Error: {:#}", e);
            let mut input = String::new();
            println!("Press enter to exit the program.");
            std::io::stdin()
                .read_line(&mut input)
                .expect("Failed to read input.");
            return;
        }
    }
}

fn start_pomodoro(data: &PomodoroOptions) {
    // Use the imported data
    println!("{:?}", data);
    let duration: Duration = Duration::from_secs((data.duration_pomodoro * 60) as u64);
    let additional_duration: Duration = Duration::from_secs((data.additional_duration * 60) as u64);

    let mut counter = 0;
    let mut input = String::new();
    let end_event = || {
        play_sound(PathBuf::from(data.filepath_sound.clone()));
    };
    loop {
        if counter != 0 {
            input.clear();
            println!("Do you want to repeat the timer? (Enter 'r' and/or press enter for repeat)");
            std::io::stdin()
                .read_line(&mut input)
                .expect("Failed to read input.");
        } else {
            input = "r".to_string();
        }
        if input.trim() == "r" || input.trim().is_empty() {
            execute_timer(duration, additional_duration, end_event);
            let break_duration: Duration;
            if counter % 4 == 3 {
                break_duration = Duration::from_secs((data.duration_long_break * 60) as u64)
            } else {
                break_duration = Duration::from_secs((data.duration_short_break * 60) as u64)
            };
            println!(
                "Press enter to start the break of {:.0} minutes",
                break_duration.as_secs() / 60
            );
            std::io::stdin()
                .read_line(&mut input)
                .expect("Failed to read input.");
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
    let start_time = time::Instant::now();
    let bar = ProgressBar::new(duration.as_secs());
    bar.set_style(
        ProgressStyle::with_template("[{elapsed}/{eta}] {wide_bar:.cyan/blue} ").unwrap(),
    );
    let delta: u64 = 1;
    while start_time.elapsed() < duration {
        thread::sleep(Duration::from_secs(delta));
        bar.inc(delta);
    }
    bar.finish();

    end_event();
    println!("Times up!");
    if !additional_duration.is_zero() {
        thread::sleep(additional_duration);
        println!(
            "You got an extra {} minutes.",
            additional_duration.as_secs() / 60
        );
        display_screensaver_and_lock_screen();
    }
}
