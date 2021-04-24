use crate::{
    config,
    error::{CritError, CritResult},
    util::{Action, Cursor, Key, XCursor, XCursorShape},
};
use std::{collections::HashMap, mem, ptr};
use x11_dl::xlib;

pub struct Backend {
    xlib: xlib::Xlib,
    display: *mut xlib::Display,
    root: xlib::Window,
    start: xlib::XButtonEvent,
    attrs: xlib::XWindowAttributes,
    cursor: Cursor,
    key_map: HashMap<Key, Action>,
}

impl Backend {
    pub fn new() -> CritResult<Self> {
        // Open xlib.
        let xlib = xlib::Xlib::open()?;
        // Open display.
        let display = unsafe { (xlib.XOpenDisplay)(ptr::null()) };
        if display.is_null() {
            return Err(CritError::Other("Display is null".to_owned()));
        }
        // Get root window.
        let root = unsafe { (xlib.XDefaultRootWindow)(display) };
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
            _ => {}
        }
    }

    fn set_cursor(&self, cursor: XCursor) {
        unsafe { (self.xlib.XDefineCursor)(self.display, self.root, cursor) };
    }
}
