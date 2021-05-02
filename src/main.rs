#[macro_use]
extern crate log;

#[macro_use]
mod util;
mod backend;
mod config;
mod error;
mod layouts;

use backend::Backend;
use error::CritResult;
use std::process;

fn run() -> CritResult<()> {
    info!("Started critwm");
    let mut backend = Backend::new()?;
    backend.grab_keys();
    backend.grab_buttons();
    loop {
        backend.handle_signal()?;
        backend.handle_cursor();
        backend.handle_event()?;
    }
}

fn main() {
    env_logger::init();
    match run() {
        Ok(()) => process::exit(0),
        Err(e) => {
            error!("{}", e);
            process::exit(1)
        }
    }
}
