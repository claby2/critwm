mod atom;
mod client;
mod hints;
mod monitor;
pub mod signal;

use crate::{
    config,
    error::{CritError, CritResult},
    util::{Action, Cursor, Key, XCursor, XCursorShape},
};
use atom::Atom;
use client::Client;
use monitor::Monitor;
use signal::{Signal, SIGNAL_STACK};
use std::{cmp, collections::HashMap, mem, ptr, slice};
use x11_dl::{xinerama, xlib};

pub struct Backend {
    xlib: xlib::Xlib,
    display: *mut xlib::Display,
    root: xlib::Window,
    start: xlib::XButtonEvent,
    attrs: xlib::XWindowAttributes,
    atoms: Atom,
    cursor: Cursor,
    key_map: HashMap<Key, Action>,
    clients: Vec<Client>,
    current_client: usize,
    monitors: Vec<Monitor<{ config::WORKSPACE_COUNT }>>,
    current_monitor: usize,
}

impl Backend {
    const POINTER_BUTTON_MASK: u32 =
        (xlib::PointerMotionMask | xlib::ButtonPressMask | xlib::ButtonReleaseMask) as u32;

    pub fn new() -> CritResult<Self> {
        // Open xlib.
        let xlib = xlib::Xlib::open()?;
        unsafe { (xlib.XSetErrorHandler)(Some(Self::xerror)) };
        // Open display.
        let display = unsafe { (xlib.XOpenDisplay)(ptr::null()) };
        if display.is_null() {
            return Err(CritError::Other("Display is null.".to_owned()));
        }
        // Get root window.
        let root = unsafe { (xlib.XDefaultRootWindow)(display) };
        unsafe { (xlib.XSelectInput)(display, root, xlib::SubstructureRedirectMask) };
        // Create atoms.
        let atoms = Atom::new(&xlib, display);
        // Create cursors.
        let create_cursor = |shape: XCursorShape| -> XCursor {
            unsafe { (xlib.XCreateFontCursor)(display, shape) }
        };
        let cursor = Cursor::new(
            create_cursor(Cursor::NORM),
            create_cursor(Cursor::RES),
            create_cursor(Cursor::MOV),
        );
        // Construct backend.
        let mut backend = Self {
            xlib,
            display,
            root,
            start: unsafe { mem::zeroed() },
            attrs: unsafe { mem::zeroed() },
            atoms,
            cursor,
            key_map: config::get_keymap(),
            clients: Vec::new(),
            current_client: 0,
            monitors: Vec::new(),
            current_monitor: 0,
        };
        backend.set_hints();
        backend.set_cursor(backend.cursor.norm);
        backend.fetch_monitors()?;
        Ok(backend)
    }

    pub fn grab_keys(&self) {
        for key in self.key_map.keys() {
            let code = unsafe { (self.xlib.XKeysymToKeycode)(self.display, key.sym) };
            unsafe {
                (self.xlib.XGrabKey)(
                    self.display,
                    i32::from(code),
                    key.modifier,
                    self.root,
                    1,
                    xlib::GrabModeAsync,
                    xlib::GrabModeAsync,
                );
            }
        }
    }

    pub fn grab_buttons(&self) {
        let grab_button = |button: u32| unsafe {
            (self.xlib.XGrabButton)(
                self.display,
                button,
                config::MODKEY,
                self.root,
                0,
                Self::POINTER_BUTTON_MASK,
                xlib::GrabModeAsync,
                xlib::GrabModeAsync,
                0,
                0,
            );
        };
        grab_button(xlib::Button1Mask);
        grab_button(xlib::Button3Mask);
    }

    pub fn kill_client(&self) {
        if let Some(client) = self.clients.get(self.current_client) {
            // Try kill the client nicely.
            if !self.send_xevent_atom(client.window, self.atoms.wm_delete) {
                // Force kill the client.
                unsafe {
                    (self.xlib.XGrabServer)(self.display);
                    (self.xlib.XSetErrorHandler)(Some(Self::xerror_dummy));
                    (self.xlib.XSetCloseDownMode)(self.display, xlib::DestroyAll);
                    (self.xlib.XKillClient)(self.display, client.window);
                    (self.xlib.XSync)(self.display, xlib::False);
                    (self.xlib.XSetErrorHandler)(Some(Self::xerror));
                    (self.xlib.XUngrabServer)(self.display);
                }
            }
        }
    }

    pub fn handle_signal(&mut self) -> CritResult<()> {
        // Handle signals.
        if let Some(signal) = SIGNAL_STACK.lock().unwrap().pop() {
            match signal {
                Signal::KillClient => self.kill_client(),
                Signal::ChangeWorkspace(workspace) => {
                    // Change workspace of selected monitor to given workspace.
                    let monitor = &mut self.monitors[self.current_monitor];
                    // TODO: Clean this up.
                    if monitor.get_current_workspace() != workspace {
                        // Unmap windows that are in the old workspace.
                        for client in &self.clients {
                            if client.monitor == self.current_monitor
                                && client.workspace == monitor.get_current_workspace()
                            {
                                unsafe { (self.xlib.XUnmapWindow)(self.display, client.window) };
                            }
                        }
                        // Update workspace value to new value.
                        monitor.set_current_workspace(workspace)?;
                        // Map windows that are in the new workspace.
                        for client in &self.clients {
                            if client.monitor == self.current_monitor
                                && client.workspace == monitor.get_current_workspace()
                            {
                                unsafe { (self.xlib.XMapWindow)(self.display, client.window) };
                            }
                        }
                    }
                }
                Signal::MoveToWorkspace(workspace) => {
                    // Move currently focused client to given workspace.
                    let mut client = &mut self.clients[self.current_client];
                    if client.workspace != workspace {
                        client.workspace = workspace;
                        // Hide window as it has moved to another workspace.
                        unsafe { (self.xlib.XUnmapWindow)(self.display, client.window) };
                    }
                }
            }
        }
        Ok(())
    }

    pub fn handle_cursor(&mut self) {
        // Handle cursor position.
        let (mut x, mut y) = (0, 0);
        let mut window: xlib::Window = 0;
        let mut root_x = 0;
        let mut root_y = 0;
        let mut mask = 0;
        unsafe {
            (self.xlib.XQueryPointer)(
                self.display,
                self.root,
                &mut window,
                &mut window,
                &mut x,
                &mut y,
                &mut root_x,
                &mut root_y,
                &mut mask,
            );
        };
        let (x, y) = (x as u32, y as u32);
        // If the current monitor does not contain the cursor position, find the monitor which has it.
        if !self.monitors[self.current_monitor].has_point(x, y) {
            // While interating, skip over checking the current monitor.
            for (i, monitor) in self
                .monitors
                .iter()
                .enumerate()
                .filter(|&(i, _)| i != self.current_monitor)
            {
                if monitor.has_point(x, y) {
                    // Change monitor.
                    self.current_monitor = i;
                    break;
                }
            }
        }
    }

    pub fn handle_event(&mut self) {
        // Handle events from xlib.
        let mut event: xlib::XEvent = unsafe { mem::zeroed() };
        unsafe { (self.xlib.XNextEvent)(self.display, &mut event) };
        match event.get_type() {
            xlib::KeyPress => {
                let key_event = xlib::XKeyEvent::from(event);
                let keysym = unsafe {
                    (self.xlib.XKeycodeToKeysym)(self.display, key_event.keycode as u8, 0)
                };
                if let Some(action) = self.key_map.get(&Key::new(key_event.state, keysym)) {
                    action.run();
                }
            }
            xlib::ButtonPress if unsafe { event.button.subwindow != 0 } => {
                unsafe {
                    (self.xlib.XGetWindowAttributes)(
                        self.display,
                        event.button.subwindow,
                        &mut self.attrs,
                    );
                    (self.xlib.XRaiseWindow)(self.display, event.button.subwindow);
                };
                self.start = unsafe { event.button };
            }
            xlib::MotionNotify => {
                if self.start.subwindow != 0 {
                    // Compress motion notify events.
                    while unsafe {
                        (self.xlib.XCheckTypedEvent)(self.display, xlib::MotionNotify, &mut event)
                    } > 0
                    {}
                    let diff = || unsafe {
                        (
                            event.button.x_root - self.start.x_root,
                            event.button.y_root - self.start.y_root,
                        )
                    };
                    match self.start.button {
                        xlib::Button1 => {
                            self.set_cursor(self.cursor.mov);
                            let (dx, dy) = diff();
                            self.move_client(
                                self.current_client,
                                self.attrs.x + dx,
                                self.attrs.y + dy,
                            );
                        }
                        xlib::Button3 => {
                            self.set_cursor(self.cursor.res);
                            let (dw, dh) = diff();
                            self.resize_client(
                                self.current_client,
                                (self.attrs.width + dw) as u32,
                                (self.attrs.height + dh) as u32,
                            );
                        }
                        _ => {}
                    }
                }
            }
            xlib::ButtonRelease => {
                self.set_cursor(self.cursor.norm);
                self.start.subwindow = 0
            }
            xlib::ConfigureRequest => {
                let request = unsafe { event.configure_request };
                let mut changes = xlib::XWindowChanges {
                    x: request.x,
                    y: request.y,
                    width: request.width,
                    height: request.height,
                    border_width: request.border_width,
                    sibling: request.above,
                    stack_mode: request.detail,
                };
                unsafe {
                    (self.xlib.XConfigureWindow)(
                        self.display,
                        request.window,
                        request.value_mask as u32,
                        &mut changes,
                    )
                };
            }
            xlib::MappingNotify => {
                let mut mapping = unsafe { event.mapping };
                if mapping.request == xlib::MappingKeyboard
                    || mapping.request == xlib::MappingModifier
                {
                    unsafe { (self.xlib.XRefreshKeyboardMapping)(&mut mapping) };
                }
                self.grab_keys();
                self.grab_buttons();
            }
            xlib::MapRequest => {
                let window = unsafe { event.map_request.window };
                unsafe {
                    (self.xlib.XSelectInput)(
                        self.display,
                        window,
                        xlib::StructureNotifyMask | xlib::EnterWindowMask,
                    );
                    (self.xlib.XMapWindow)(self.display, window);
                };
                self.clients.push(Client::new(
                    &self.xlib,
                    self.display,
                    window,
                    self.current_monitor,
                    self.monitors[self.current_monitor].get_current_workspace(),
                ));
                let index = self.clients.len() - 1;
                self.set_focus(Some(index));
                self.set_client_monitor(index);
            }
            xlib::EnterNotify => {
                // Pointer has entered a new window.
                // Iterate through all clients to find this window and focus it.
                for (i, client) in self.clients.iter().enumerate() {
                    if client.window == unsafe { event.crossing.window } {
                        self.set_focus(Some(i));
                        break;
                    }
                }
            }
            xlib::DestroyNotify => {
                // Get the window that should be destroyed.
                for (i, client) in self.clients.iter().enumerate() {
                    if client.window == unsafe { event.destroy_window.window } {
                        // Remove destroyed client.
                        self.clients.remove(i);
                        if i > 0 {
                            // Adjust client index and ensure it is not out of bounds.
                            self.current_client = cmp::min(self.clients.len() - 1, i);
                            // Set focus to current client.
                            self.set_focus(None);
                        }
                        break;
                    }
                }
            }
            _ => {}
        }
    }

    fn set_cursor(&self, cursor: XCursor) {
        unsafe { (self.xlib.XDefineCursor)(self.display, self.root, cursor) };
    }

    fn move_client(&mut self, index: usize, x: i32, y: i32) {
        unsafe { (self.xlib.XMoveWindow)(self.display, self.clients[index].window, x, y) };
        self.set_client_monitor(index);
    }

    fn resize_client(&mut self, index: usize, width: u32, height: u32) {
        unsafe {
            (self.xlib.XResizeWindow)(self.display, self.clients[index].window, width, height)
        };
        self.set_client_monitor(index);
    }

    fn set_client_monitor(&mut self, index: usize) {
        let client = &mut self.clients[index];
        client.update_geometry(&self.xlib, self.display);
        let geometry = client.get_geometry();
        for (i, monitor) in self.monitors.iter().enumerate() {
            if monitor.has_window(&geometry) {
                client.monitor = i;
                break;
            }
        }
    }

    fn set_focus(&mut self, index: Option<usize>) {
        if let Some(index) = index {
            // If given Some(index), change the current client index.
            self.current_client = index;
        }
        unsafe {
            (self.xlib.XSetInputFocus)(
                self.display,
                self.clients[self.current_client].window,
                xlib::RevertToParent,
                xlib::CurrentTime,
            )
        };
    }

    fn fetch_monitors(&mut self) -> CritResult<()> {
        let xlib = xinerama::Xlib::open()?;
        if unsafe { (xlib.XineramaIsActive)(self.display) } > 0 {
            let mut screen_count = 0;
            let raw_infos = unsafe { (xlib.XineramaQueryScreens)(self.display, &mut screen_count) };
            let xinerama_infos: &[xinerama::XineramaScreenInfo] =
                unsafe { slice::from_raw_parts(raw_infos, screen_count as usize) };
            self.monitors = xinerama_infos.iter().map(Monitor::from).collect();
            Ok(())
        } else {
            Err(CritError::Other("Xinerama is not active.".to_owned()))
        }
    }

    fn send_xevent_atom(&self, window: xlib::Window, atom: xlib::Atom) -> bool {
        let mut array: *mut xlib::Atom = unsafe { std::mem::zeroed() };
        let mut length = unsafe { std::mem::zeroed() };
        let mut exists = false;
        if unsafe { (self.xlib.XGetWMProtocols)(self.display, window, &mut array, &mut length) } > 0
        {
            let protocols: &[xlib::Atom] = unsafe { slice::from_raw_parts(array, length as usize) };
            exists = protocols.contains(&atom);
        }
        if exists {
            let mut message: xlib::XClientMessageEvent = unsafe { std::mem::zeroed() };
            message.type_ = xlib::ClientMessage;
            message.window = window;
            message.message_type = self.atoms.wm_protocols;
            message.format = 32;
            message.data.set_long(0, atom as i64);
            message.data.set_long(1, xlib::CurrentTime as i64);
            let mut event: xlib::XEvent = message.into();
            unsafe {
                (self.xlib.XSendEvent)(self.display, window, 0, xlib::NoEventMask, &mut event)
            };
        }
        exists
    }

    extern "C" fn xerror(_: *mut xlib::Display, e: *mut xlib::XErrorEvent) -> i32 {
        let err = unsafe { *e };
        // Ignore BadWindow error code.
        if err.error_code == xlib::BadWindow {
            0
        } else {
            1
        }
    }

    extern "C" fn xerror_dummy(_: *mut xlib::Display, _: *mut xlib::XErrorEvent) -> i32 {
        0
    }
}
