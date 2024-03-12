use rodio::{source::Source, Decoder, OutputStream};
use serde::{Deserialize, Serialize};
use std::env;
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;
use std::{thread, time};
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct PomodoroOptions {
    duration: i32,
    additional_duration: i32,
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

    // Use the imported data
    println!("{:?}", data);
    let duration: time::Duration = time::Duration::from_secs((5) as u64);
    let additional_duration: time::Duration =
        time::Duration::from_secs((data.additional_duration * 60) as u64);
    execute_timer(duration, additional_duration, || {
        play_sound(PathBuf::from(data.filepath_sound.clone()))
    });
    loop {
        println!("Do you want to repeat the timer? (Enter 'r' for repeat)");
        let mut input = String::new();
        std::io::stdin()
            .read_line(&mut input)
            .expect("Failed to read input.");
        if input.trim() == "r" {
            execute_timer(duration, additional_duration, || {
                play_sound(PathBuf::from(data.filepath_sound.clone()))
            });
        } else {
            break;
        }
    }
}

fn execute_timer<F>(duration: time::Duration, additional_duration: time::Duration, end_event:  F) where F: Fn(){
    println!("Timer started for {} minutes. ", duration.as_secs() / 60);
    let start_time = time::Instant::now();
    let mut now = time::Instant::now();
    let mut time_elapsed = now.duration_since(start_time);
    while time_elapsed < duration {
        let time_remaining = duration - time_elapsed;
        println!(
            "Time remaining: {:.0} minutes.",
            time_remaining.as_secs() / 60
        );
        now = time::Instant::now();
        time_elapsed = now.duration_since(start_time);
        thread::sleep(time::Duration::from_secs(6));
    }

    end_event();
    println!("Times up!");
    // Wait for 5 minutes before turning on the screen saver
    thread::sleep(additional_duration);
    println!(
        "Turning on the screen saver in {} minutes.",
        additional_duration.as_secs() / 60
    );
    display_screensaver_and_lock_screen();
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
    if filepath_sound.is_file() {
        let sound_file = std::fs::File::open(filepath_sound).expect("Failed to open sound file.");
        let source = Decoder::new(sound_file).expect("Failed to decode sound file.");
        let _ = stream_handle.play_raw(source.convert_samples());
    } else {
        // include_bytes! macro is used to include the sound file in the binary.
        let sound_file = include_bytes!("C:/Windows/Media/Alarm01.wav");
        let sound_cursor = std::io::Cursor::new(&sound_file[..]);
        let source = Decoder::new(sound_cursor).unwrap();
        let _ = stream_handle.play_raw(source.convert_samples());
    }
}
