#[macro_use]
mod util;
mod backend;
mod config;
mod error;

use backend::Backend;
use error::CritResult;
use std::process;

fn run() -> CritResult<()> {
    let mut backend = Backend::new()?;
    backend.grab_keys();
    backend.grab_buttons();
    loop {
        backend.handle_cursor();
        backend.handle_signal()?;
        backend.handle_event();
    }
}

fn main() {
    match run() {
        Ok(()) => process::exit(0),
        Err(e) => {
            eprintln!("ERROR: {}", e);
            process::exit(1)
        }
    }
}
