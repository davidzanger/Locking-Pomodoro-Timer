use locking_pomodoro_timer::{run_cli, run_service};

fn main() {
    if std::env::args().any(|argument| argument == "--service") {
        if let Err(error) = run_service() {
            eprintln!("Service failed: {:#}", error);
        }
        return;
    }

    run_cli();
}
