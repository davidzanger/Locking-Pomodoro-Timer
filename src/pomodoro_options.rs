use anyhow::{Context, Result};
use project_root::get_project_root;
use serde::{Deserialize, Serialize};
use std::env;
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default = "PomodoroOptions::default")]
/// Struct representing the options for a Pomodoro timer.
pub struct PomodoroOptions {
    /// The duration of a single Pomodoro session in minutes.
    pub duration_pomodoro: i32,
    /// The additional duration in minutes to be added to a Pomodoro session when it is over.
    pub additional_duration: i32,
    /// The duration of a short break in minutes.
    pub duration_short_break: i32,
    /// The duration of a long break in minutes.
    pub duration_long_break: i32,
    /// The filepath to the sound file to be played when a Pomodoro session ends.
    pub filepath_sound: String,
    /// Flag indicating whether to automatically start a break after a Pomodoro session ends.
    pub auto_start_break: bool,
    /// Flag indicating whether to automatically start a new Pomodoro session after a break ends.
    pub auto_start_pomodoro: bool,
    /// The interval in number of Pomodoro sessions after which a long break should be taken.
    pub interval_long_break: i32,
}

impl Default for PomodoroOptions {
    fn default() -> Self {
        PomodoroOptions {
            duration_pomodoro: 25,
            additional_duration: 5,
            duration_short_break: 5,
            duration_long_break: 15,
            filepath_sound: "".to_string(),
            auto_start_break: true,
            auto_start_pomodoro: true,
            interval_long_break: 4,
        }
    }
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
pub fn read_options_from_json(filepath_json: Option<PathBuf>) -> Result<PomodoroOptions> {
    let filename = "pomodoro_options.json";
    let file_path = match filepath_json {
        Some(path) => path,
        None => {
            let mut path = get_folderpath_executable()?;
            path.push(filename);
            path
        }
    };
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

#[test]
fn test_read_options_from_json() {
    let filepath_test_json = get_project_root()
        .unwrap()
        .join("tests")
        .join("data")
        .join("pomodoro_options.json");
    // Assuming you have a valid JSON file with the correct structure
    let options = read_options_from_json(Some(filepath_test_json)).unwrap();

    assert_eq!(options.duration_pomodoro, 25);
    assert_eq!(options.additional_duration, 5);
    assert_eq!(options.duration_short_break, 5);
    assert_eq!(options.duration_long_break, 15);
    assert_eq!(options.filepath_sound, "");
}
