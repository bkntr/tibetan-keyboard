#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

#[cfg(windows)]
mod windows_app;

#[cfg(windows)]
fn main() {
    if let Err(error) = windows_app::run() {
        windows_app::show_error("Tibetan EWTS Keyboard", &error.to_string());
    }
}

#[cfg(not(windows))]
fn main() {
    eprintln!("Tibetan EWTS Keyboard currently supports Windows only.");
}
