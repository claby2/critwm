use crate::{
    config,
    error::{CritError, CritResult},
    util::{Action, Cursor, Key, XCursor, XCursorShape, XWindowDimension, XWindowPosition},
};
use std::{cmp, collections::HashMap, mem, ptr};
use x11_dl::xlib;

#[derive(Debug)]
struct WindowGeometry {
    x: XWindowPosition,
    y: XWindowPosition,
    width: XWindowDimension,
    height: XWindowDimension,
    border_width: XWindowDimension,
    border_depth: XWindowDimension,
}

impl WindowGeometry {
    fn new(xlib: &xlib::Xlib, display: *mut xlib::Display, window: &xlib::Window) -> Self {
        let mut root: xlib::Window = 0;
        let mut x: XWindowPosition = 0;
        let mut y: XWindowPosition = 0;
        let mut width: XWindowDimension = 0;
        let mut height: XWindowDimension = 0;
        let mut border_width: XWindowDimension = 0;
        let mut border_depth: XWindowDimension = 0;
        unsafe {
            (xlib.XGetGeometry)(
                display,
                *window,
                &mut root,
                &mut x,
                &mut y,
                &mut width,
                &mut height,
                &mut border_width,
                &mut border_depth,
            )
        };
        Self {
            x,
            y,
            width,
            height,
            border_width,
            border_depth,
        }
    }
}

#[derive(Debug)]
pub struct Client {
    window_geometry: WindowGeometry,
    window: xlib::Window,
}

impl Client {
    fn new(xlib: &xlib::Xlib, display: *mut xlib::Display, window: xlib::Window) -> Self {
        Self {
            window_geometry: WindowGeometry::new(xlib, display, &window),
            window,
        }
    }
}

pub struct Backend {
    xlib: xlib::Xlib,
    display: *mut xlib::Display,
    root: xlib::Window,
    start: xlib::XButtonEvent,
    attrs: xlib::XWindowAttributes,
    cursor: Cursor,
    key_map: HashMap<Key, Action>,
    clients: Vec<Client>,
    current_client: usize,
}

impl Backend {
    pub fn new() -> CritResult<Self> {
        // Open xlib.
        let xlib = xlib::Xlib::open()?;
        unsafe { (xlib.XSetErrorHandler)(Some(Self::xerror)) };
        // Open display.
        let display = unsafe { (xlib.XOpenDisplay)(ptr::null()) };
        if display.is_null() {
            return Err(CritError::Other("Display is null".to_owned()));
        }
        // Get root window.
        let root = unsafe { (xlib.XDefaultRootWindow)(display) };
        unsafe { (xlib.XSelectInput)(display, root, xlib::SubstructureRedirectMask) };
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
        let backend = Self {
            xlib,
            display,
            root,
            start: unsafe { mem::zeroed() },
            attrs: unsafe { mem::zeroed() },
            cursor,
            key_map: config::get_keymap(),
            clients: Vec::new(),
            current_client: 0,
        };
        // Set initial cursor.
        backend.set_cursor(backend.cursor.norm);
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
                (xlib::ButtonPressMask | xlib::ButtonReleaseMask | xlib::PointerMotionMask) as u32,
                xlib::GrabModeAsync,
                xlib::GrabModeAsync,
                0,
                0,
            );
        };
        grab_button(xlib::Button1Mask);
        grab_button(xlib::Button3Mask);
    }

    pub fn handle_event(&mut self) {
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
            xlib::MotionNotify if self.start.subwindow != 0 => {
                let xdiff = unsafe { event.button.x_root - self.start.x_root };
                let ydiff = unsafe { event.button.y_root - self.start.y_root };
                match self.start.button {
                    xlib::Button1 => {
                        self.set_cursor(self.cursor.mov);
                        unsafe {
                            (self.xlib.XMoveWindow)(
                                self.display,
                                self.start.subwindow,
                                self.attrs.x + xdiff,
                                self.attrs.y + ydiff,
                            );
                        }
                    }
                    xlib::Button3 => {
                        self.set_cursor(self.cursor.res);
                        unsafe {
                            (self.xlib.XResizeWindow)(
                                self.display,
                                self.start.subwindow,
                                (self.attrs.width + xdiff) as u32,
                                (self.attrs.height + ydiff) as u32,
                            );
                        }
                    }
                    _ => {}
                };
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
                self.clients
                    .push(Client::new(&self.xlib, self.display, window));
                self.set_focus(Some(self.clients.len() - 1));
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

    extern "C" fn xerror(_: *mut xlib::Display, e: *mut xlib::XErrorEvent) -> i32 {
        let err = unsafe { *e };
        // Ignore BadWindow error code.
        if err.error_code == xlib::BadWindow {
            0
        } else {
            1
        }
    }
}
