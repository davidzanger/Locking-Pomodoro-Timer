use crossterm::event::read;
use crossterm::event::Event;
use crossterm::event::KeyCode;
use crossterm::terminal::disable_raw_mode;
use crossterm::terminal::enable_raw_mode;
use log::debug;
use log::trace;

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
