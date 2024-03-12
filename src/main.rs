use indicatif::{ProgressBar, ProgressStyle};
use rodio::{Decoder, OutputStream, Sink};
use serde::{Deserialize, Serialize};
use std::env;
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;
use std::time::Duration;
use std::{thread, time};

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct PomodoroOptions {
    duration_pomodoro: i32,
    additional_duration: i32,
    duration_short_break: i32,
    duration_long_break: i32,
    filepath_sound: String,
}
fn main() {
    // Read the JSON file
    let exe_path = env::current_exe().expect("Failed to get current executable path.");
    let mut file_path = exe_path.clone();
    file_path.pop(); // Remove the executable name from the path
    file_path.push("pomodoro_options.json");

    let mut file = File::open(file_path).expect("Failed to open file.");
    let mut contents = String::new();
    file.read_to_string(&mut contents)
        .expect("Failed to read file.");

    // Deserialize the JSON data into your data structure
    let data: PomodoroOptions =
        serde_json::from_str(&contents).expect("Failed to deserialize JSON.");

    start_pomodoro(data);
}

fn start_pomodoro(data: PomodoroOptions) {
    // Use the imported data
    println!("{:?}", data);
    let duration: Duration = Duration::from_secs((data.duration_pomodoro * 60) as u64);
    let additional_duration: Duration = Duration::from_secs((data.additional_duration * 60) as u64);

    let mut counter = 0;
    let mut input = String::new();
    loop {
        if counter != 0 {
            println!("Do you want to repeat the timer? (Enter 'r' for repeat)");
            std::io::stdin()
                .read_line(&mut input)
                .expect("Failed to read input.");
        } else {
            input = "r".to_string();
        }
        if input.trim() == "r" {
            execute_timer(duration, additional_duration, || {
                play_sound(PathBuf::from(data.filepath_sound.clone()))
            });
            let break_duration: Duration;
            if counter % 4 == 3 {
                break_duration = Duration::from_secs((data.duration_long_break * 60) as u64)
            } else {
                break_duration = Duration::from_secs((data.duration_short_break * 60) as u64)
            };
            println!(
                "Press any key to start the break of {} minutes",
                break_duration.as_secs() / 60
            );

            std::io::stdin()
                .read_line(&mut input)
                .expect("Failed to read input.");
            execute_timer(break_duration, Duration::from_secs(0), || {
                play_sound(PathBuf::from(data.filepath_sound.clone()))
            });
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
    let mut now = time::Instant::now();
    let mut time_elapsed = now.duration_since(start_time);
    let bar = ProgressBar::new(duration.as_secs());
    bar.set_style(
        ProgressStyle::with_template("[{elapsed}/{eta}] {wide_bar:.cyan/blue} ").unwrap(),
    );
    let delta = 1;
    while time_elapsed < duration {
        now = time::Instant::now();
        time_elapsed = now.duration_since(start_time);
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
    let sink = Sink::try_new(&stream_handle).unwrap();

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
