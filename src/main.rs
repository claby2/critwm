#[macro_use]
extern crate log;

#[macro_use]
mod util;
mod backend;
mod error;
mod layouts;

mod config {
    // Parse configuration from user's filesystem if custom_config.
    #[cfg(feature = "custom_config")]
    include!(concat!(env!("HOME"), "/.config/critwm/config.rs"));

    // Fallback to default config in src if not custom_config.
    #[cfg(not(feature = "custom_config"))]
    include!("config.def.rs");
}

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
    if cfg!(feature = "custom_config") {
        info!("Using custom configuration");
    } else {
        info!("Using default configuration");
    }
    match run() {
        Ok(()) => process::exit(0),
        Err(e) => {
            error!("{}", e);
            process::exit(1)
        }
    }
}
