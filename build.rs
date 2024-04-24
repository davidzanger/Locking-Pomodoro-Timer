use {
    std::{env, io},
    winres::WindowsResource,
};

fn main() -> io::Result<()> {
    // From: https://stackoverflow.com/questions/30291757/attaching-an-icon-resource-to-a-rust-application/65393488#65393488
    if env::var_os("CARGO_CFG_WINDOWS").is_some() {
        WindowsResource::new()
            // This path can be absolute, or relative to your crate root.
            .set_icon("assets/pomodoro.ico")
            .compile()?;
    }
    Ok(())
}
