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
        unsafe {
            (xlib.XSelectInput)(
                display,
                root,
                xlib::SubstructureRedirectMask
                    | xlib::StructureNotifyMask
                    | xlib::PointerMotionMask,
            )
        };
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
                Signal::ChangeWorkspace(new_workspace) => {
                    // Change workspace of selected monitor to given workspace.
                    let monitor = &self.monitors[self.current_monitor];
                    let is_visible = |workspace: usize, client: &Client| -> bool {
                        client.monitor == self.current_monitor && client.workspace == workspace
                    };
                    if monitor.get_current_workspace() != new_workspace {
                        // Unmap windows that are in the old workspace.
                        self.clients
                            .iter()
                            .filter(|client| is_visible(monitor.get_current_workspace(), client))
                            .for_each(|client| {
                                unsafe { (self.xlib.XUnmapWindow)(self.display, client.window) };
                            });
                        // Map windows that are in the new workspace.
                        self.clients
                            .iter()
                            .filter(|client| is_visible(new_workspace, client))
                            .for_each(|client| {
                                unsafe { (self.xlib.XMapWindow)(self.display, client.window) };
                            });
                        // Update workspace value to new value.
                        self.monitors[self.current_monitor].set_current_workspace(new_workspace)?;
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

    pub fn handle_event(&mut self) -> CritResult<()> {
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
                    (action)();
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
                // Handle monitor switching case.
                if unsafe { event.motion.window } == self.root {
                    // If the current monitor does not contain the cursor position, find the monitor which has it.
                    let (x, y) =
                        unsafe { (event.button.x_root as u32, event.button.y_root as u32) };
                    if !self.monitors[self.current_monitor].has_point(x, y) {
                        // While iterating, skip over checking the current monitor.
                        if let Some(monitor_index) =
                            self.monitors.iter().enumerate().position(|(i, monitor)| {
                                i != self.current_monitor && monitor.has_point(x, y)
                            })
                        {
                            self.current_monitor = monitor_index;
                            // Ensure that subwindow is 0.
                            if unsafe { event.motion.subwindow } == 0 {
                                // Cursor has entered a new monitor but is not over any clients.
                                // Find a client to focus on.
                                if let Some(client_index) = self
                                    .clients
                                    .iter()
                                    .position(|client| client.monitor == monitor_index)
                                {
                                    self.set_focus(Some(client_index));
                                }
                            }
                        }
                    }
                }
            }
            xlib::ButtonRelease => {
                self.set_cursor(self.cursor.norm);
                self.start.subwindow = 0
            }
            xlib::ConfigureRequest => {
                let request = unsafe { event.configure_request };
                if let Some(client) = self
                    .clients
                    .iter()
                    .find(|client| client.window == request.window)
                {
                    let geometry = client.get_geometry();
                    let mut configure_event: xlib::XEvent =
                        xlib::XConfigureEvent::into(xlib::XConfigureEvent {
                            type_: xlib::ConfigureNotify,
                            serial: 0,
                            send_event: 0,
                            display: self.display,
                            event: request.window,
                            window: request.window,
                            x: geometry.x,
                            y: geometry.y,
                            width: geometry.width as i32,
                            height: geometry.height as i32,
                            border_width: geometry.border_width as i32,
                            above: 0,
                            override_redirect: 0,
                        });
                    unsafe {
                        (self.xlib.XSendEvent)(
                            self.display,
                            request.window,
                            xlib::False,
                            xlib::StructureNotifyMask,
                            &mut configure_event,
                        )
                    };
                } else {
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
            }
            xlib::ConfigureNotify => {
                if unsafe { event.configure.window } == self.root {
                    // Root has notified configure.
                    self.fetch_monitors()?;
                    // Update client monitors.
                    for i in 0..self.clients.len() {
                        self.set_client_monitor(i);
                    }
                }
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
                        xlib::StructureNotifyMask
                            | xlib::EnterWindowMask
                            | xlib::PropertyChangeMask,
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
                // Ensure that client is created in the position of the current monitor.
                self.move_client(
                    index,
                    self.monitors[self.current_monitor].x as i32,
                    self.monitors[self.current_monitor].y as i32,
                );
            }
            xlib::UnmapNotify => {
                let unmap_event = unsafe { event.unmap };
                if unmap_event.window == self.start.subwindow {
                    self.set_cursor(self.cursor.norm);
                    self.start.subwindow = 0;
                }
            }
            xlib::EnterNotify => {
                // Pointer has entered a new window.
                // Iterate through all clients to find this window and focus it.
                if let Some(client_index) = self
                    .clients
                    .iter()
                    .position(|client| client.window == unsafe { event.crossing.window })
                {
                    self.set_focus(Some(client_index));
                }
            }
            xlib::DestroyNotify => {
                // Get the window that should be destroyed.
                if let Some(client_index) = self
                    .clients
                    .iter()
                    .position(|client| client.window == unsafe { event.destroy_window.window })
                {
                    // Remove destroyed client.
                    self.clients.remove(client_index);
                    // Adjust client index and ensure it is not out of bounds.
                    self.current_client =
                        cmp::max(cmp::min(self.clients.len() - 1, client_index), 0);
                    if self.clients.len() > 0 {
                        // Set focus to current client.
                        self.set_focus(None);
                    }
                }
            }
            xlib::ClientMessage => {
                let client_message = unsafe { event.client_message };
                if client_message.message_type == self.atoms.net_wm_state {
                    let data = client_message.data;
                    if data.get_long(1) == self.atoms.net_wm_state_fullscreen as i64
                        || data.get_long(2) == self.atoms.net_wm_state_fullscreen as i64
                    {
                        if let Some(index) = self
                            .clients
                            .iter()
                            .position(|client| client.window == client_message.window)
                        {
                            self.toggle_fullscreen(index);
                        }
                    }
                }
            }
            xlib::PropertyNotify => {
                if unsafe { event.property.atom } == self.atoms.net_wm_window_type {
                    if let Some(state) = self.get_atom_prop(
                        self.clients[self.current_client].window,
                        self.atoms.net_wm_state,
                    ) {
                        if state == self.atoms.net_wm_state_fullscreen {
                            self.toggle_fullscreen(self.current_client);
                        }
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn set_cursor(&self, cursor: XCursor) {
        unsafe { (self.xlib.XDefineCursor)(self.display, self.root, cursor) };
    }

    fn move_resize_client(&mut self, index: usize, x: i32, y: i32, width: u32, height: u32) {
        unsafe {
            (self.xlib.XMoveResizeWindow)(
                self.display,
                self.clients[index].window,
                x,
                y,
                width,
                height,
            )
        };
        self.set_client_monitor(index);
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
        // Ensure that the client's monitor is correct.
        let client = &mut self.clients[index];
        client.update_geometry(&self.xlib, self.display);
        let geometry = client.get_geometry();
        if let Some(monitor_index) = self
            .monitors
            .iter()
            .position(|monitor| monitor.has_window(&geometry))
        {
            client.monitor = monitor_index;
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

    fn toggle_fullscreen(&mut self, index: usize) {
        // Toggle client fullscreen state.
        self.clients[index].toggle_fullscreen();
        if self.clients[index].fullscreen {
            // Make client fullscreen.
            self.set_window_state(
                self.clients[index].window,
                self.atoms.net_wm_state_fullscreen,
            );
            self.move_resize_client(
                index,
                self.monitors[self.current_monitor].x as i32,
                self.monitors[self.current_monitor].y as i32,
                self.monitors[self.current_monitor].width as u32,
                self.monitors[self.current_monitor].height as u32,
            );
            unsafe { (self.xlib.XRaiseWindow)(self.display, self.clients[index].window) };
        } else {
            // Get client out of fullscreen.
            self.set_window_state(self.clients[index].window, 0);
            let client = self.clients[index].clone();
            let old_geometry = client.get_old_geometry();
            // Restore old geometry.
            self.move_resize_client(
                index,
                old_geometry.x,
                old_geometry.y,
                old_geometry.width,
                old_geometry.height,
            );
        }
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
