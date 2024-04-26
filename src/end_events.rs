/// This module defines the `EndEvent` enum and related functions for handling end events in the Pomodoro application.
///
/// The `EndEvent` enum represents different types of end events that can occur after a Pomodoro session, such as playing a sound or locking the screen.
///
/// # Examples
///
/// ```
/// use pomodoro::end_events::{EndEvent, lock_screen, play_sound};
/// use std::path::PathBuf;
///
/// let sound_event = EndEvent::Sound {
///     filepath_sound: PathBuf::from("sound.wav"),
/// };
/// let screensaver_event = EndEvent::LockScreen;
///
/// // Play a sound
/// if let EndEvent::Sound { filepath_sound } = sound_event {
///     play_sound(&filepath_sound);
/// }
///
/// // Lock the screen
/// if let EndEvent::LockScreen = screensaver_event {
///     lock_screen();
/// }
/// ```
///
/// # Note
///
/// - The `Sound` variant of `EndEvent` requires a file path to the sound file.
/// - The `LockScreen` variant of `EndEvent` locks the screen.
/// - The `lock_screen` function locks the screen on Windows using the `LockWorkStation` function from `user32.dll`.
/// - The `play_sound` function plays a sound file using the `rodio` crate.
use rodio::{Decoder, OutputStream, Sink};
use serde;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Represents different types of end events that can occur after a Pomodoro session.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", rename_all_fields = "camelCase")]
pub(crate) enum EndEvent {
    /// Play a sound specified by the file path.
    Sound { filepath_sound: PathBuf },
    /// Lock the screen.
    LockScreen,
}

/// Starts the specified end event.
pub(crate) fn start_end_event(end_event: &EndEvent) {
    match end_event {
        EndEvent::Sound { filepath_sound } => play_sound(&filepath_sound),
        EndEvent::LockScreen => lock_screen(),
    }
}

/// Locks the screen.
pub fn lock_screen() {
    if cfg!(windows) {
        lock_screen_on_windows();
    } else {
        // TODO: Implement for Linux and macOS.
        todo!("This feature is not implemented for this platform.")
    }
}

/// Locks the screen on Windows.
pub fn lock_screen_on_windows() {
    // Turn on the screen saver for windows and lock the screen.
    std::process::Command::new("cmd")
        .args(&["/C", "rundll32", "user32.dll,LockWorkStation"])
        .output()
        .expect("Failed to start screen saver.");
}

/// Plays the sound specified by the file path.
pub fn play_sound(filepath_sound: &PathBuf) {
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

#[test]
fn test_serialize_end_event_to_json() {
    let sound_event = EndEvent::Sound {
        filepath_sound: PathBuf::from("sound.wav"),
    };
    let screensaver_event = EndEvent::LockScreen;

    let sound_event_json = serde_json::to_string(&sound_event).unwrap();
    let screensaver_event_json = serde_json::to_string(&screensaver_event).unwrap();

    assert_eq!(
        sound_event_json,
        r#"{"sound":{"filepathSound":"sound.wav"}}"#
    );
    assert_eq!(screensaver_event_json, r#""lockScreen""#);
}
