use rodio::{Decoder, OutputStream, Sink};
use std::path::PathBuf;

pub fn display_screensaver_and_lock_screen() {
    if cfg!(windows) {
        display_screensaver_and_lock_screen_on_windows();
    } else {
        // TODO: Implement for Linux and macOS.
        unimplemented!("This feature is not implemented for this platform.")
    }
}
pub fn display_screensaver_and_lock_screen_on_windows() {
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

pub fn play_sound(filepath_sound: PathBuf) {
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
