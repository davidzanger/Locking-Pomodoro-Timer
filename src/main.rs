use indicatif::{ProgressBar, ProgressStyle};
use rodio::{Decoder, OutputStream, Sink};
use serde::{Deserialize, Serialize};
use std::env;
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;
use std::time::Duration;
use std::{thread, time};
mod timer;
use anyhow::{Context, Result};
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct PomodoroOptions {
    duration_pomodoro: i32,
    additional_duration: i32,
    duration_short_break: i32,
    duration_long_break: i32,
    filepath_sound: String,
}

impl PomodoroOptions {
    fn verify(&self) -> Result<(), String> {
        if self.duration_pomodoro < 1 {
            return Err("Pomodoro duration should be at least 1 minute.".to_string());
        }
        if self.additional_duration < 0 {
            return Err("Additional duration should be at least 0 minute.".to_string());
        }
        if self.duration_short_break < 1 {
            return Err("Short break duration should be at least 1 minute.".to_string());
        }
        if self.duration_long_break < 1 {
            return Err("Long break duration should be at least 1 minute.".to_string());
        }
        if !PathBuf::from(&self.filepath_sound).is_file() || !self.filepath_sound.is_empty() {
            return Err("Sound file does not exist.".to_string());
        }
        Ok(())
    }
}
fn main() {
    // Read the JSON file
    let data = read_options_from_json().expect("Failed to read options from JSON file.");
    start_pomodoro(&data);
}

fn read_options_from_json() -> Result<PomodoroOptions> {
    let mut folderpath = get_folderpath_executable()?;
    let file_path = &mut folderpath;
    let filename = "pomodoro_options.json";
    file_path.push(filename);

    let mut file =
        File::open(&file_path).with_context(|| format!("Failed to open file: {:?}", file_path))?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)
        .with_context(|| format!("Failed to read file: {:?}", file_path))?;

    let mut data: PomodoroOptions = serde_json::from_str(&contents)
        .with_context(|| format!("Failed to parse JSON file: {:?}", file_path))?;
    if !PathBuf::from(data.filepath_sound.clone()).is_file() && !data.filepath_sound.is_empty() {
        println!("Sound file does not exist. Using default sound.");
        data.filepath_sound = "".to_string();
    }
    Ok(data)
}

fn get_folderpath_executable() -> Result<PathBuf> {
    let exe_path = env::current_exe().context("Failed to get executable path.")?;
    let mut file_path = exe_path.clone();
    file_path.pop();
    Ok(file_path)
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

fn display_screensaver_and_lock_screen() {
    // Turn on the screen saver for windows and lock the screen.
    std::process::Command::new("cmd")
        .args(&[
            "/C",
            "start",
            "",
            "scrnsave.scr",
            "/s",
            "&",
            "rundll32",
            "user32.dll,LockWorkStation",
        ])
        .output()
        .expect("Failed to start screen saver.");
}

fn play_sound(filepath_sound: PathBuf) {
    let (_stream, stream_handle) =
        OutputStream::try_default().expect("Failed to create output stream.");
    let sink = Sink::try_new(&stream_handle).expect("Failed to create sink.");

    if filepath_sound.is_file() {
        let sound_file = std::fs::File::open(filepath_sound).expect("Failed to open sound file.");
        let source = Decoder::new(sound_file).expect("Failed to decode sound file.");
        sink.append(source);
    } else {
        // include_bytes! macro is used to include the sound file in the binary.
        let sound_file = include_bytes!("C:/Windows/Media/Alarm01.wav");
        let sound_cursor = std::io::Cursor::new(&sound_file[..]);
        let source = Decoder::new(sound_cursor).unwrap();
        sink.append(source);
    }
    sink.sleep_until_end();
}
