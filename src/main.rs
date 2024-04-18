use crate::end_events::{display_screensaver_and_lock_screen, play_sound};
use crate::pomodoro_options::read_options_from_json;
use crate::timer::Timer;
use crossterm::event::{read, Event, KeyCode};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use indicatif::{ProgressBar, ProgressStyle};
use log::{debug, trace};
use pomodoro_options::PomodoroOptions;
use std::path::PathBuf;
use std::thread;
use std::time::Duration;
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
    debug!("Starting input stream.");
    let receiver = start_input_stream();
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
            let pomodoros_till_long_break =
                options.interval_long_break - counter % options.interval_long_break;
            let minutes_till_long_break = pomodoros_till_long_break
                * (options.duration_pomodoro + options.additional_duration)
                + (pomodoros_till_long_break - 1) * options.duration_short_break;
            let is_long_break_coming =
                counter % options.interval_long_break == options.interval_long_break - 1;
            let break_duration: Duration;
            if is_long_break_coming {
                break_duration = Duration::from_secs((options.duration_long_break * 60) as u64)
            } else {
                break_duration = Duration::from_secs((options.duration_short_break * 60) as u64)
            };
            let mut print_message =
                format!("Current: Pomodoro ({:.0} min)", options.duration_pomodoro);
            if !additional_duration.is_zero() {
                print_message += &format!(
                    " | Upcoming: Additional {} min",
                    additional_duration.as_secs() / 60
                );
            } else if !break_duration.is_zero() && is_long_break_coming {
                print_message += &format!(
                    " | Upcoming: Long break ({:.0} min)",
                    options.duration_long_break
                );
            } else if !break_duration.is_zero() && !is_long_break_coming {
                print_message += &format!(
                    " | Upcoming: Short break ({:.0} min)",
                    options.duration_short_break
                );
            } else if break_duration.is_zero() {
                print_message += &format!(
                    " | Upcoming: Pomodoro ({:.0} min)",
                    options.duration_pomodoro
                );
            } else {
                panic!("Invalid state. This line should not be reached.");
            }
            print_message += &format!(
                " | Pomodoros till long break: {} ({} min)",
                pomodoros_till_long_break, minutes_till_long_break
            );
            println!("{}", print_message);

            execute_timer(duration, &receiver, end_event);
            if !additional_duration.is_zero() {
                let mut print_message = format!(
                    "Current: Additional ({:.0} min).",
                    additional_duration.as_secs() / 60
                );
                if !break_duration.is_zero() && is_long_break_coming {
                    print_message += &format!(
                        " | Upcoming: Long break ({:.0} min)",
                        options.duration_long_break
                    );
                } else if !break_duration.is_zero() && !is_long_break_coming {
                    print_message += &format!(
                        " | Upcoming: Short break ({:.0} min)",
                        options.duration_short_break
                    );
                } else if break_duration.is_zero() {
                    print_message += &format!(
                        " | Upcoming: Pomodoro ({:.0} min)",
                        options.duration_pomodoro
                    );
                } else {
                    panic!("Invalid state. This line should not be reached.");
                }
                let minutes_till_long_break = pomodoros_till_long_break
                    * options.additional_duration
                    + (pomodoros_till_long_break - 1)
                        * (options.duration_short_break + options.duration_pomodoro);
                print_message += &format!(
                    " | Pomodoros till long break: {} ({} min)",
                    pomodoros_till_long_break - 1,
                    minutes_till_long_break
                );
                println!("{}", print_message);
                time_with_progress_bar(
                    additional_duration,
                    &receiver,
                    display_screensaver_and_lock_screen,
                );
            }
            if !break_duration.is_zero() {
                if !options.auto_start_break {
                    if is_long_break_coming {
                        println!(
                            "Press enter to start the long break of {:.0} minutes.",
                            break_duration.as_secs() / 60
                        );
                    } else {
                        println!(
                            "Press enter to start the short break of {:.0} minutes.",
                            break_duration.as_secs() / 60
                        );
                    }
                    std::io::stdin()
                        .read_line(&mut input)
                        .expect("Failed to read input.");
                }
                let mut print_message = format!(
                    "Current: {} break ({:.0} min).",
                    if is_long_break_coming {
                        "Long"
                    } else {
                        "Short"
                    },
                    break_duration.as_secs() / 60
                );
                print_message += &format!(
                    " | Upcoming: Pomodoro ({:.0} min)",
                    options.duration_pomodoro
                );
                let minutes_till_long_break = (pomodoros_till_long_break - 1)
                    * (options.duration_pomodoro
                        + options.duration_short_break
                        + options.additional_duration);
                print_message += &format!(
                    " | Pomodoros till long break: {} ({} min)",
                    pomodoros_till_long_break - 1,
                    minutes_till_long_break
                );
                println!("{}", print_message);
                execute_timer(break_duration, &receiver, end_event);
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

fn start_input_stream() -> std::sync::mpsc::Receiver<String> {
    let (sender, receiver) = std::sync::mpsc::channel::<String>();
    std::thread::Builder::new()
        .name("input_stream".to_string())
        .spawn(move || {
            trace!("Spawning input thread.");
            enable_raw_mode().expect("Failed to enable raw mode.");
            loop {
                if let Ok(Event::Key(key_event)) = read() {
                    debug!("Received key event: {:?}", key_event);
                    if key_event.kind != crossterm::event::KeyEventKind::Press {
                        continue;
                    }
                    if key_event.code == KeyCode::Char('c')
                        && key_event.modifiers == crossterm::event::KeyModifiers::CONTROL
                    {
                        debug!("Exiting the program.");
                        sender
                            .send("ctrl+c".to_string())
                            .expect("Failed to send input.");
                        disable_raw_mode().expect("Failed to disable raw mode.");
                        break;
                    } else if let KeyCode::Char(c) = key_event.code {
                        sender.send(c.to_string()).expect("Failed to send input.");
                    }
                }
            }
        })
        .expect("Failed to spawn input thread.");
    receiver
}
