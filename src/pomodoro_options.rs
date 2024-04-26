use crate::end_events::EndEvent;
use anyhow::{Context, Result};
#[cfg(test)]
use project_root::get_project_root;
use serde::{Deserialize, Serialize};
use std::env;
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;
use thiserror::Error;

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
    /// Flag indicating whether to automatically start a break after a Pomodoro session ends.
    pub auto_start_break: bool,
    /// Flag indicating whether to automatically start a new Pomodoro session after a break ends.
    pub auto_start_pomodoro: bool,
    /// The interval in number of Pomodoro sessions after which a long break should be taken.
    pub interval_long_break: i32,
    /// The end event to be executed after a Pomodoro session ends.
    pub end_event_pomodoro: EndEvent,
    /// The end event to be executed after the additional Pomodoro after a Pomodoro session ends.
    pub end_event_additional_pomodoro: EndEvent,
}
#[derive(Error, Debug)]
pub(crate) enum VerificationError {
    #[error("Pomodoro duration should be at least 1 minute.")]
    InvalidDuration,
    #[error("Additional duration should be at least 0 minute.")]
    InvalidAdditionalDuration,
    #[error("Short break duration should be at least 0 minute.")]
    InvalidShortBreakDuration,
    #[error("Long break duration should be at least 0 minute.")]
    InvalidLongBreakDuration,
    #[error("Sound file does not exist.")]
    InvalidSoundFile,
}

impl Default for PomodoroOptions {
    fn default() -> Self {
        PomodoroOptions {
            duration_pomodoro: 25,
            additional_duration: 5,
            duration_short_break: 5,
            duration_long_break: 15,
            auto_start_break: true,
            auto_start_pomodoro: true,
            interval_long_break: 4,
            end_event_pomodoro: EndEvent::Sound {
                filepath_sound: PathBuf::new(),
            },
            end_event_additional_pomodoro: EndEvent::LockScreen,
        }
    }
}

impl PomodoroOptions {
    fn verify(&self) -> Result<(), VerificationError> {
        if self.duration_pomodoro < 1 {
            return Err(VerificationError::InvalidDuration);
        }
        if self.additional_duration < 0 {
            return Err(VerificationError::InvalidAdditionalDuration);
        }
        if self.duration_short_break < 0 {
            return Err(VerificationError::InvalidShortBreakDuration);
        }
        if self.duration_long_break < 0 {
            return Err(VerificationError::InvalidLongBreakDuration);
        }
        if let EndEvent::Sound { filepath_sound } = &self.end_event_pomodoro {
            if !PathBuf::from(&filepath_sound).is_file() && !filepath_sound.as_os_str().is_empty() {
                return Err(VerificationError::InvalidSoundFile);
            }
        }
        if let EndEvent::Sound { filepath_sound } = &self.end_event_additional_pomodoro {
            if !PathBuf::from(&filepath_sound).is_file() && !filepath_sound.as_os_str().is_empty() {
                return Err(VerificationError::InvalidSoundFile);
            }
        }

        Ok(())
    }
}

#[derive(Error, Debug)]
pub(crate) enum PomodoroOptionsError {
    #[error("Failed to read options from JSON file at path: {:?}", _0)]
    OptionFileNotFound(PathBuf),
}

pub fn read_options_from_json(filepath_json: Option<PathBuf>) -> Result<PomodoroOptions> {
    let file_path = match filepath_json {
        Some(path) => path,
        None => get_filepath_options_next_to_executable()?,
    };
    if !file_path.is_file() {
        return Err(PomodoroOptionsError::OptionFileNotFound(file_path).into());
    }
    let mut file =
        File::open(&file_path).with_context(|| format!("Failed to open file: {:?}", file_path))?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)
        .with_context(|| format!("Failed to read file: {:?}", file_path))?;

    let mut data: PomodoroOptions = serde_json::from_str(&contents)
        .with_context(|| format!("Failed to parse JSON file: {:?}", file_path))?;
    match data.verify() {
        Ok(_) => (),
        Err(VerificationError::InvalidSoundFile) => {
            println!("Sound file does not exist. Using default sound.");
            if let EndEvent::Sound { filepath_sound } = &mut data.end_event_pomodoro {
                *filepath_sound = PathBuf::new();
            }
            if let EndEvent::Sound { filepath_sound } = &mut data.end_event_additional_pomodoro {
                *filepath_sound = PathBuf::new();
            }
        }
        Err(e) => return Err(e.into()),
    }
    Ok(data)
}

pub(crate) fn write_default_options_to_json_next_to_executable() -> Result<()> {
    let file_path = get_filepath_options_next_to_executable()?;
    let options = PomodoroOptions::default();
    write_options_to_json(&file_path, &options)
}

pub(crate) fn write_options_to_json(
    file_path: &PathBuf,
    options: &PomodoroOptions) -> Result<()> {
    let file = File::create(&file_path)
        .with_context(|| format!("Failed to create file: {:?}", file_path))?;
    serde_json::to_writer_pretty(file, options)
        .with_context(|| format!("Failed to write to file: {:?}", file_path))?;
    Ok(())
}

fn get_filepath_options_next_to_executable() -> Result<PathBuf> {
    let filename = "pomodoro_options.json";
    let mut path = get_folderpath_executable()?;
    path.push(filename);
    Ok(path)
}

fn get_folderpath_executable() -> Result<PathBuf> {
    let exe_path = env::current_exe().context("Failed to get executable path.")?;
    let mut file_path = exe_path.clone();
    // Remove the executable name, keep the folder path.
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
}
