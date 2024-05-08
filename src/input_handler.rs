use crossterm::event::read;
use crossterm::event::Event;
use crossterm::event::KeyCode;
use crossterm::terminal::disable_raw_mode;
use crossterm::terminal::enable_raw_mode;
use log::debug;
use log::trace;

/// Creates a channel to communicate key events from the terminal to the main thread.
/// 
/// Not all key events are sent through the channel. Only printable characters and the enter key.
/// The ctrl+c key combination is used to exit the program and also sent through the channel.
/// Only the key press events are sent through the channel. The key release events are ignored.
///
/// # Returns
///
/// A Receiver that can be used to receive messages sent through the channel.
///
/// # Panics
///
/// This function will panic if the underlying operating system is unable to create the channel or
/// if the input thread is unable to start or if the terminal is unable to enter raw mode.
///
pub(crate) fn start_input_stream() -> std::sync::mpsc::Receiver<String> {
    let (sender, receiver) = std::sync::mpsc::channel::<String>();
    std::thread::Builder::new()
        .name("input_stream".to_string())
        .spawn(move || {
            trace!("Spawning input thread.");
            enable_raw_mode().expect("Failed to enable raw mode.");
            loop {
                let exit = process_key_event(&sender);
                if exit {
                    break;
                }
            }
        })
        .expect("Failed to spawn input thread.");
    receiver
}

/// Processes the key events received from the terminal.
/// 
/// This function reads key events from the terminal using the `crossterm` library.
/// It checks if the key event is a press event and filters out key release events.
/// If the key event is a printable character or the enter key, it sends the corresponding
/// string representation through the channel to the main thread.
/// If the key event is the ctrl+c combination, it sends "ctrl+c" through the channel to
/// indicate that the program should exit.
///
/// # Arguments
///
/// * `sender` - A reference to the sender channel used to send key events to the main thread.
///
/// # Returns
///
/// A boolean value indicating whether the program should exit or not.
///
fn process_key_event(sender: &std::sync::mpsc::Sender<String>) -> bool {
    let mut exit = false;
    if let Ok(Event::Key(key_event)) = read() {
        debug!("Received key event: {:?}", key_event);
        if key_event.kind != crossterm::event::KeyEventKind::Press {
            if key_event.code == KeyCode::Char('c')
                && key_event.modifiers == crossterm::event::KeyModifiers::CONTROL
            {
                debug!("Exiting the program.");
                sender
                    .send("ctrl+c".to_string())
                    .expect("Failed to send input.");
                disable_raw_mode().expect("Failed to disable raw mode.");
                exit = true;
            } else if let KeyCode::Char(c) = key_event.code {
                sender.send(c.to_string()).expect("Failed to send input.");
            } else if key_event.code == KeyCode::Enter {
                sender
                    .send("\n".to_string())
                    .expect("Failed to send input.");
            }
        }
    }
    exit
}
