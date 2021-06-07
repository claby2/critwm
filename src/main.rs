#[macro_use]
extern crate log;

use critwm::{
    backend::Backend,
    error::{CritError, CritResult},
    socket::{self, StateSocket},
};
use std::{path::PathBuf, process, ptr};
use x11_dl::{xinerama, xlib};

async fn start() -> CritResult<()> {
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
    let mut backend = unsafe { Backend::new(&xlib, &xinerama_xlib, display)? };
    backend.initialize()?;
    backend.grab_keys();
    backend.grab_buttons();
    run(&mut backend).await?;
    Ok(())
}

async fn run(backend: &mut Backend<'_>) -> CritResult<()> {
    let mut state_socket = StateSocket::new(PathBuf::from(socket::SOCKET_PATH));
    state_socket.listen().await?;
    loop {
        if backend.handle_signal()? {
            // Quit signal has been handled.
            break;
        }
        backend.handle_cursor();
        backend.handle_event()?;
        state_socket.write(&backend).await?;
    }
    state_socket.close().await?;
    Ok(())
}

#[tokio::main]
async fn main() {
    env_logger::init();
    info!("Started critwm");
    if cfg!(feature = "custom_config") {
        info!("Using custom configuration");
    } else {
        info!("Using default configuration");
    }
    match start().await {
        Ok(_) => {
            info!("Closed critwm successfully");
            process::exit(0);
        }
        Err(e) => {
            error!("{:?}", e);
            process::exit(1)
        }
    }
}
