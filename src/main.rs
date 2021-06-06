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
use error::{CritError, CritResult};
use std::{process, ptr};
use x11_dl::{xinerama, xlib};

fn run() -> CritResult<()> {
    // Open xlib.
    let xlib = xlib::Xlib::open()?;
    unsafe { (xlib.XSetErrorHandler)(Some(Backend::xerror)) };
    // Open display.
    let display = unsafe { (xlib.XOpenDisplay)(ptr::null()) };
    if display.is_null() {
        return Err(CritError::Other("Display is null.".to_owned()));
    }
    let xinerama_xlib = xinerama::Xlib::open()?;
    if unsafe { (xinerama_xlib.XineramaIsActive)(display) } == 0 {
        return Err(CritError::Other("Xinerama is not active.".to_owned()));
    }
    let mut backend = Backend::new(&xlib, &xinerama_xlib, display)?;
    backend.initialize()?;
    backend.grab_keys();
    backend.grab_buttons();
    loop {
        if backend.handle_signal()? {
            // Quit signal has been handled.
            break;
        }
        backend.handle_cursor();
        backend.handle_event()?;
    }
    unsafe { (xlib.XCloseDisplay)(display) };
    Ok(())
}

fn main() {
    env_logger::init();
    info!("Started critwm");
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
