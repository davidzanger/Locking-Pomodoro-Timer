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

/// Struct representing the options for a Pomodoro timer.
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default = "PomodoroOptions::default")]
pub struct PomodoroOptions {
    /// The duration of a single Pomodoro session in minutes.
    pub duration_pomodoro: i32,
    /// The additional duration in minutes to be added to a Pomodoro session when it is over.
    pub additional_duration: i32,
    /// Repeat the Additional Pomodoro until the user requests the due break.
    pub repeat_additional_pomodoro: bool,
    /// Ask whether to continue working after each completed Additional Pomodoro.
    pub ask_before_break: bool,
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
    /// After a break ends, the interval in minutes after which a reminder should be triggered.
    /// This shall remind the user to either go back to work or to start a new Pomodoro session if already working.
    /// This option is only relevant if `auto_start_pomodoro` is `false`.
    pub interval_reminder_after_break: i32,
    /// The event to be executed after the reminder interval after a break ends.
    pub event_reminder_after_break: EndEvent,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TimerSettings {
    pub duration_pomodoro: i32,
    pub additional_duration: i32,
    #[serde(default = "enabled_by_default")]
    pub repeat_additional_pomodoro: bool,
    #[serde(default = "enabled_by_default")]
    pub ask_before_break: bool,
    pub duration_short_break: i32,
    pub duration_long_break: i32,
    pub interval_long_break: i32,
    pub auto_start_break: bool,
    pub auto_start_pomodoro: bool,
}

pub(crate) fn enabled_by_default() -> bool {
    true
}

impl From<&PomodoroOptions> for TimerSettings {
    fn from(options: &PomodoroOptions) -> Self {
        Self {
            duration_pomodoro: options.duration_pomodoro,
            additional_duration: options.additional_duration,
            repeat_additional_pomodoro: options.repeat_additional_pomodoro,
            ask_before_break: options.ask_before_break,
            duration_short_break: options.duration_short_break,
            duration_long_break: options.duration_long_break,
            interval_long_break: options.interval_long_break,
            auto_start_break: options.auto_start_break,
            auto_start_pomodoro: options.auto_start_pomodoro,
        }
    }
}

impl TimerSettings {
    fn apply_to(self, options: &mut PomodoroOptions) {
        options.duration_pomodoro = self.duration_pomodoro;
        options.additional_duration = self.additional_duration;
        options.repeat_additional_pomodoro = self.repeat_additional_pomodoro;
        options.ask_before_break = self.ask_before_break;
        options.duration_short_break = self.duration_short_break;
        options.duration_long_break = self.duration_long_break;
        options.interval_long_break = self.interval_long_break;
        options.auto_start_break = self.auto_start_break;
        options.auto_start_pomodoro = self.auto_start_pomodoro;
    }
}

/// Error type for verification errors of `PomodoroOptions`.
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
    #[error("Long break interval should be at least 0 Pomodoros.")]
    InvalidLongBreakInterval,
    #[error("Reminder interval should be at least 0 minutes.")]
    InvalidReminderInterval,
    #[error("Sound file does not exist.")]
    InvalidSoundFile,
}

impl Default for PomodoroOptions {
    /// Creates a new `PomodoroOptions` instance with default values.
    fn default() -> Self {
        PomodoroOptions {
            duration_pomodoro: 25,
            additional_duration: 5,
            repeat_additional_pomodoro: true,
            ask_before_break: true,
            duration_short_break: 5,
            duration_long_break: 15,
            auto_start_break: true,
            auto_start_pomodoro: true,
            interval_long_break: 4,
            end_event_pomodoro: EndEvent::Sound {
                filepath_sound: PathBuf::new(),
            },
            end_event_additional_pomodoro: EndEvent::LockScreen,
            interval_reminder_after_break: 5,
            event_reminder_after_break: EndEvent::Sound {
                filepath_sound: PathBuf::new(),
            },
        }
    }
}

impl PomodoroOptions {
    /// Verifies the validity of the `PomodoroOptions` instance.
    ///
    /// # Errors
    ///
    /// Returns a `VerificationError` if any of the options are invalid.
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
        if self.interval_long_break < 0 {
            return Err(VerificationError::InvalidLongBreakInterval);
        }
        if self.interval_reminder_after_break < 0 {
            return Err(VerificationError::InvalidReminderInterval);
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

/// Error type for `PomodoroOptions` related errors.
#[derive(Error, Debug)]
pub(crate) enum PomodoroOptionsError {
    #[error("Failed to read options from JSON file at path: {:?}", _0)]
    OptionFileNotFound(PathBuf),
}

/// Reads the `PomodoroOptions` from a JSON file.
///
/// If `filepath_json` is `Some`, it reads the options from the specified file.
/// If `filepath_json` is `None`, it tries to find the options file next to the executable.
///
/// # Errors
///
/// Returns a `PomodoroOptionsError` if the options file is not found or if there are any other errors during the process.
pub fn read_options_from_json(filepath_json: Option<PathBuf>) -> Result<PomodoroOptions> {
    read_options_from_json_inner(filepath_json, false)
}

pub(crate) fn read_options_from_json_silent(
    filepath_json: Option<PathBuf>,
) -> Result<PomodoroOptions> {
    read_options_from_json_inner(filepath_json, true)
}

pub(crate) fn read_timer_settings_json() -> Result<String> {
    read_timer_settings_json_from_path(None)
}

fn read_timer_settings_json_from_path(filepath_json: Option<PathBuf>) -> Result<String> {
    let options = match read_options_from_json_silent(filepath_json) {
        Ok(options) => options,
        Err(error)
            if matches!(
                error.downcast_ref::<PomodoroOptionsError>(),
                Some(PomodoroOptionsError::OptionFileNotFound(_))
            ) =>
        {
            PomodoroOptions::default()
        }
        Err(error) => return Err(error),
    };
    serde_json::to_string(&TimerSettings::from(&options))
        .context("Failed to serialize timer settings")
}

pub(crate) fn save_timer_settings_json(settings_json: &str) -> Result<()> {
    save_timer_settings_json_to_path(settings_json, None)
}

fn save_timer_settings_json_to_path(
    settings_json: &str,
    filepath_json: Option<PathBuf>,
) -> Result<()> {
    let settings: TimerSettings =
        serde_json::from_str(settings_json).context("Failed to parse timer settings")?;
    let file_path = match filepath_json {
        Some(path) => path,
        None => get_filepath_options_next_to_executable()?,
    };
    let mut options = match read_options_from_json_silent(Some(file_path.clone())) {
        Ok(options) => options,
        Err(error)
            if matches!(
                error.downcast_ref::<PomodoroOptionsError>(),
                Some(PomodoroOptionsError::OptionFileNotFound(_))
            ) =>
        {
            PomodoroOptions::default()
        }
        Err(error) => return Err(error),
    };
    settings.apply_to(&mut options);
    options.verify().context("Invalid timer settings")?;
    write_options_to_json(&file_path, &options)
}

fn read_options_from_json_inner(
    filepath_json: Option<PathBuf>,
    silent_warnings: bool,
) -> Result<PomodoroOptions> {
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
            if silent_warnings {
                log::warn!("Sound file does not exist. Using default sound.");
            } else {
                println!("Sound file does not exist. Using default sound.");
            }
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

/// Writes the default `PomodoroOptions` to a JSON file next to the executable.
///
/// # Errors
///
/// Returns an error if there are any errors during the process of writing the options to the file.
pub(crate) fn write_default_options_to_json_next_to_executable() -> Result<()> {
    let file_path = get_filepath_options_next_to_executable()?;
    let options = PomodoroOptions::default();
    write_options_to_json(&file_path, &options)
}

/// Writes the `PomodoroOptions` to a JSON file.
///
/// # Arguments
///
/// * `file_path` - The path to the JSON file.
/// * `options` - The `PomodoroOptions` to write to the file.
///
/// # Errors
///
/// Returns an error if there are any errors during the process of writing the options to the file.
pub(crate) fn write_options_to_json(file_path: &PathBuf, options: &PomodoroOptions) -> Result<()> {
    options.verify().context("Invalid timer options")?;
    let file = File::create(&file_path)
        .with_context(|| format!("Failed to create file: {:?}", file_path))?;
    serde_json::to_writer_pretty(file, options)
        .with_context(|| format!("Failed to write to file: {:?}", file_path))?;
    Ok(())
}

/// Gets the path to the options file next to the executable.
///
/// # Errors
///
/// Returns an error if there are any errors during the process of getting the file path.
fn get_filepath_options_next_to_executable() -> Result<PathBuf> {
    let filename = "pomodoro_options.json";
    let mut path = get_folderpath_executable()?;
    path.push(filename);
    Ok(path)
}

/// Gets the folder path of the executable.
///
/// # Errors
///
/// Returns an error if there are any errors during the process of getting the folder path.
fn get_folderpath_executable() -> Result<PathBuf> {
    let exe_path = env::current_exe().context("Failed to get executable path.")?;
    let mut file_path = exe_path.clone();
    // Remove the executable name, keep the folder path.
    file_path.pop();
    Ok(file_path)
}

#[test]
fn test_read_options_from_json() {
    // Test case for `read_options_from_json` function.
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

#[cfg(test)]
mod additional_tests {
    use super::{
        read_options_from_json_inner, read_timer_settings_json_from_path,
        save_timer_settings_json_to_path, write_options_to_json, PomodoroOptions,
        VerificationError,
    };
    use crate::end_events::EndEvent;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static NEXT_FILE_ID: AtomicUsize = AtomicUsize::new(0);

    fn temporary_file(extension: &str) -> PathBuf {
        let id = NEXT_FILE_ID.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!(
            "pomodoro-options-{}-{}.{}",
            std::process::id(),
            id,
            extension
        ))
    }

    #[test]
    fn default_options_are_valid_and_use_expected_values() {
        let options = PomodoroOptions::default();

        assert_eq!(options.duration_pomodoro, 25);
        assert_eq!(options.additional_duration, 5);
        assert_eq!(options.interval_long_break, 4);
        assert!(options.verify().is_ok());
    }

    #[test]
    fn verification_rejects_each_negative_or_zero_duration() {
        let mut options = PomodoroOptions::default();
        options.duration_pomodoro = 0;
        assert!(matches!(
            options.verify(),
            Err(VerificationError::InvalidDuration)
        ));

        options = PomodoroOptions::default();
        options.additional_duration = -1;
        assert!(matches!(
            options.verify(),
            Err(VerificationError::InvalidAdditionalDuration)
        ));

        options = PomodoroOptions::default();
        options.duration_short_break = -1;
        assert!(matches!(
            options.verify(),
            Err(VerificationError::InvalidShortBreakDuration)
        ));

        options = PomodoroOptions::default();
        options.duration_long_break = -1;
        assert!(matches!(
            options.verify(),
            Err(VerificationError::InvalidLongBreakDuration)
        ));

        options = PomodoroOptions::default();
        options.interval_long_break = -1;
        assert!(matches!(
            options.verify(),
            Err(VerificationError::InvalidLongBreakInterval)
        ));
        options.interval_long_break = 0;
        assert!(options.verify().is_ok());
        options.interval_reminder_after_break = -1;
        assert!(matches!(
            options.verify(),
            Err(VerificationError::InvalidReminderInterval)
        ));
    }

    #[test]
    fn verification_rejects_missing_sound_file() {
        let mut options = PomodoroOptions::default();
        options.end_event_pomodoro = EndEvent::Sound {
            filepath_sound: PathBuf::from("missing-pomodoro-sound.wav"),
        };

        assert!(matches!(
            options.verify(),
            Err(VerificationError::InvalidSoundFile)
        ));
    }

    #[test]
    fn verification_checks_additional_session_sound_file_too() {
        let mut options = PomodoroOptions::default();
        options.end_event_additional_pomodoro = EndEvent::Sound {
            filepath_sound: PathBuf::from("missing-additional-sound.wav"),
        };

        assert!(matches!(
            options.verify(),
            Err(VerificationError::InvalidSoundFile)
        ));
    }

    #[test]
    fn verification_accepts_existing_sound_files() {
        let path = temporary_file("wav");
        std::fs::write(&path, []).unwrap();
        let options = PomodoroOptions {
            end_event_pomodoro: EndEvent::Sound {
                filepath_sound: path.clone(),
            },
            end_event_additional_pomodoro: EndEvent::Sound {
                filepath_sound: path.clone(),
            },
            ..PomodoroOptions::default()
        };

        assert!(options.verify().is_ok());
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn reading_missing_file_returns_typed_error() {
        let path = temporary_file("json");
        let error = read_options_from_json_inner(Some(path.clone()), true).unwrap_err();

        assert!(matches!(
            error.downcast_ref::<super::PomodoroOptionsError>(),
            Some(super::PomodoroOptionsError::OptionFileNotFound(missing)) if missing == &path
        ));
    }

    #[test]
    fn reading_malformed_json_returns_contextual_error() {
        let path = temporary_file("json");
        std::fs::write(&path, "{").unwrap();

        let error = read_options_from_json_inner(Some(path.clone()), true).unwrap_err();

        assert!(format!("{:#}", error).contains("Failed to parse JSON file"));
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn reading_non_utf8_file_returns_read_context() {
        let path = temporary_file("json");
        std::fs::write(&path, [0xff, 0xfe]).unwrap();

        let error = read_options_from_json_inner(Some(path.clone()), true).unwrap_err();

        assert!(format!("{:#}", error).contains("Failed to read file"));
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn writing_to_missing_parent_returns_create_context() {
        let path = temporary_file("missing/options.json");
        let error = write_options_to_json(&path, &PomodoroOptions::default()).unwrap_err();

        assert!(format!("{:#}", error).contains("Failed to create file"));
    }

    #[test]
    fn options_path_is_next_to_current_executable() {
        let path = super::get_filepath_options_next_to_executable().unwrap();
        let executable_parent = std::env::current_exe()
            .unwrap()
            .parent()
            .unwrap()
            .to_owned();

        assert_eq!(path.file_name().unwrap(), "pomodoro_options.json");
        assert_eq!(path.parent().unwrap(), executable_parent);
    }

    #[test]
    fn writing_then_reading_options_preserves_values() {
        let path = temporary_file("json");
        let mut options = PomodoroOptions::default();
        options.duration_pomodoro = 42;
        write_options_to_json(&path, &options).unwrap();

        let loaded = read_options_from_json_inner(Some(path.clone()), true).unwrap();

        assert_eq!(loaded.duration_pomodoro, 42);
        assert_eq!(loaded.duration_short_break, options.duration_short_break);
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn timer_settings_api_preserves_other_rust_options_and_never_writes_theme() {
        let path = temporary_file("json");
        let mut options = PomodoroOptions::default();
        options.interval_reminder_after_break = 9;
        write_options_to_json(&path, &options).unwrap();

        save_timer_settings_json_to_path(
            r#"{"durationPomodoro":30,"additionalDuration":0,"durationShortBreak":4,"durationLongBreak":12,"intervalLongBreak":3,"autoStartBreak":false,"autoStartPomodoro":false}"#,
            Some(path.clone()),
        )
        .unwrap();

        let contents = std::fs::read_to_string(&path).unwrap();
        let persisted: serde_json::Value = serde_json::from_str(&contents).unwrap();
        let timer_settings: serde_json::Value =
            serde_json::from_str(&read_timer_settings_json_from_path(Some(path.clone())).unwrap())
                .unwrap();

        assert_eq!(persisted["durationPomodoro"], 30);
        assert_eq!(persisted["intervalReminderAfterBreak"], 9);
        assert_eq!(persisted.get("themeMode"), None);
        assert_eq!(timer_settings["durationPomodoro"], 30);
        assert_eq!(timer_settings["autoStartBreak"], false);
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn older_options_enable_repetition_when_the_additional_duration_is_positive() {
        let path = temporary_file("json");
        std::fs::write(
            &path,
            r#"{"durationPomodoro":25,"additionalDuration":5,"durationShortBreak":5,"durationLongBreak":15,"intervalLongBreak":4,"autoStartBreak":true,"autoStartPomodoro":true}"#,
        )
        .unwrap();
        let options = read_options_from_json_inner(Some(path.clone()), true).unwrap();
        assert!(options.repeat_additional_pomodoro);
        assert!(options.ask_before_break);
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn timer_settings_api_rejects_invalid_values_without_saving() {
        let path = temporary_file("json");
        let original = PomodoroOptions::default();
        write_options_to_json(&path, &original).unwrap();

        let error = save_timer_settings_json_to_path(
            r#"{"durationPomodoro":0,"additionalDuration":0,"durationShortBreak":4,"durationLongBreak":12,"intervalLongBreak":3,"autoStartBreak":false,"autoStartPomodoro":false}"#,
            Some(path.clone()),
        )
        .unwrap_err();
        let saved = read_options_from_json_inner(Some(path.clone()), true).unwrap();

        assert!(format!("{error:#}").contains("Invalid timer settings"));
        assert_eq!(saved.duration_pomodoro, original.duration_pomodoro);
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn negative_long_break_interval_is_rejected_when_reading_and_saving() {
        let path = temporary_file("json");
        let mut options = PomodoroOptions::default();
        options.interval_long_break = -1;
        std::fs::write(&path, serde_json::to_vec(&options).unwrap()).unwrap();
        assert!(matches!(
            read_options_from_json_inner(Some(path.clone()), true)
                .unwrap_err()
                .downcast_ref::<VerificationError>(),
            Some(VerificationError::InvalidLongBreakInterval)
        ));

        options.interval_long_break = 4;
        write_options_to_json(&path, &options).unwrap();
        let error = save_timer_settings_json_to_path(
            r#"{"durationPomodoro":25,"additionalDuration":5,"durationShortBreak":5,"durationLongBreak":15,"intervalLongBreak":-1,"autoStartBreak":true,"autoStartPomodoro":true}"#,
            Some(path.clone()),
        )
        .unwrap_err();
        assert!(format!("{error:#}").contains("Long break interval"));
        assert_eq!(
            read_options_from_json_inner(Some(path.clone()), true)
                .unwrap()
                .interval_long_break,
            4
        );
        options.interval_long_break = -1;
        let error = write_options_to_json(&path, &options).unwrap_err();
        assert!(format!("{error:#}").contains("Long break interval"));
        assert_eq!(
            read_options_from_json_inner(Some(path.clone()), true)
                .unwrap()
                .interval_long_break,
            4
        );
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn invalid_sound_paths_are_replaced_with_default_paths() {
        let path = temporary_file("json");
        let mut options = PomodoroOptions::default();
        options.end_event_pomodoro = EndEvent::Sound {
            filepath_sound: PathBuf::from("missing-pomodoro-sound.wav"),
        };
        options.end_event_additional_pomodoro = EndEvent::Sound {
            filepath_sound: PathBuf::from("missing-additional-sound.wav"),
        };
        std::fs::write(&path, serde_json::to_vec(&options).unwrap()).unwrap();

        let loaded = read_options_from_json_inner(Some(path.clone()), true).unwrap();

        assert!(matches!(
            loaded.end_event_pomodoro,
            EndEvent::Sound { filepath_sound } if filepath_sound.as_os_str().is_empty()
        ));
        assert!(matches!(
            loaded.end_event_additional_pomodoro,
            EndEvent::Sound { filepath_sound } if filepath_sound.as_os_str().is_empty()
        ));
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn non_silent_read_warns_and_recovers_from_invalid_sound_paths() {
        let path = temporary_file("json");
        let options = PomodoroOptions {
            end_event_pomodoro: EndEvent::Sound {
                filepath_sound: PathBuf::from("missing-warning-sound.wav"),
            },
            ..PomodoroOptions::default()
        };
        std::fs::write(&path, serde_json::to_vec(&options).unwrap()).unwrap();

        let loaded = read_options_from_json_inner(Some(path.clone()), false).unwrap();

        assert!(matches!(
            loaded.end_event_pomodoro,
            EndEvent::Sound { filepath_sound } if filepath_sound.as_os_str().is_empty()
        ));
        std::fs::remove_file(path).unwrap();
    }
}
