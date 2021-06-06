mod atom;
pub mod client;
mod hints;
pub mod monitor;
pub mod signal;

use crate::{
    config,
    error::CritResult,
    layouts::Layout,
    util::{Action, Cursor, Key, XCursor, XCursorShape},
};
use atom::Atom;
use client::Client;
use monitor::{Monitor, MonitorManager};
use std::{cmp, collections::HashMap, mem, slice};
use x11_dl::{xinerama, xlib};

pub struct Backend<'a> {
    xlib: &'a xlib::Xlib,
    xinerama_xlib: &'a xinerama::Xlib,
    display: *mut xlib::Display,
    root: xlib::Window,
    start: xlib::XButtonEvent,
    attrs: xlib::XWindowAttributes,
    atoms: Atom,
    cursor: Cursor,
    key_map: HashMap<Key, Action>,
    clients: Vec<Client>,
    current_client: Option<usize>,
    monitors: MonitorManager,
    layouts: Vec<(String, Box<Layout>)>,
    current_monitor: usize,
}

impl<'a> Backend<'a> {
    const POINTER_BUTTON_MASK: u32 =
        (xlib::PointerMotionMask | xlib::ButtonPressMask | xlib::ButtonReleaseMask) as u32;

    pub fn new(
        xlib: &'a xlib::Xlib,
        xinerama_xlib: &'a xinerama::Xlib,
        display: *mut xlib::Display,
    ) -> CritResult<Self> {
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
        Ok(Self {
            xlib,
            xinerama_xlib,
            display,
            root,
            start: unsafe { mem::zeroed() },
            attrs: unsafe { mem::zeroed() },
            atoms,
            cursor,
            key_map: config::get_keymap(),
            clients: Vec::new(),
            // current_client as None means that no client is focused.
            current_client: None,
            monitors: MonitorManager::new(),
            layouts: config::get_layouts(),
            current_monitor: 0,
        })
    }

    pub fn initialize(&mut self) -> CritResult<()> {
        self.set_hints();
        self.set_cursor(self.cursor.norm);
        self.fetch_monitors()?;
        self.scan();
        Ok(())
    }

    pub fn grab_keys(&self) {
        unsafe { (self.xlib.XUngrabKey)(self.display, xlib::AnyKey, xlib::AnyModifier, self.root) };
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
        unsafe {
            (self.xlib.XUngrabButton)(
                self.display,
                xlib::AnyButton as u32,
                xlib::AnyModifier,
                self.root,
            )
        };
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

    pub fn handle_cursor(&mut self) {
        // Handle monitor switching case.
        // If the current monitor does not contain the cursor position, find the monitor which has it.
        let (mut x, mut y) = (0, 0);
        let mut root_return: xlib::Window = 0;
        let mut child_return: xlib::Window = 0;
        let mut root_x = 0;
        let mut root_y = 0;
        let mut mask = 0;
        unsafe {
            (self.xlib.XQueryPointer)(
                self.display,
                self.root,
                &mut root_return,
                &mut child_return,
                &mut x,
                &mut y,
                &mut root_x,
                &mut root_y,
                &mut mask,
            );
        };
        if !self.monitors[self.current_monitor]
            .get_geometry()
            .has_point(x, y)
        {
            // While iterating, skip over checking the current monitor.
            if let Some(monitor_index) =
                self.monitors.iter().enumerate().position(|(i, monitor)| {
                    i != self.current_monitor && monitor.get_geometry().has_point(x, y)
                })
            {
                self.current_monitor = monitor_index;
                // Ensure that subwindow is 0.
                if child_return == 0 {
                    // Cursor has entered a new monitor but is not over any clients.
                    // Find a client to focus on.
                    let workspace = self.monitors[self.current_monitor].get_current_workspace();
                    if let Some(client_index) = self
                        .clients
                        .iter()
                        .position(|client| self.is_visible(workspace, client))
                    {
                        self.set_focus(Some(client_index));
                    } else {
                        self.set_focus(None);
                    }
                }
            }
        }
    }

    pub fn handle_event(&mut self) -> CritResult<()> {
        // Handle events from xlib.
        let mut event: xlib::XEvent = unsafe { mem::zeroed() };
        unsafe { (self.xlib.XNextEvent)(self.display, &mut event) };
        let event_type = event.get_type();
        trace!("New event: {:?}", event_type);
        match event_type {
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
                if let Some(client_index) = self
                    .clients
                    .iter()
                    .position(|client| client.window == self.start.subwindow)
                {
                    self.clients[client_index].floating = true;
                    self.arrange(
                        self.current_monitor,
                        self.monitors[self.current_monitor].get_current_workspace(),
                    );
                }
            }
            xlib::MotionNotify => {
                if self.start.subwindow != 0 {
                    if let Some(current_client) = self.current_client {
                        // Compress motion notify events.
                        while unsafe {
                            (self.xlib.XCheckTypedEvent)(
                                self.display,
                                xlib::MotionNotify,
                                &mut event,
                            )
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
                                    current_client,
                                    self.attrs.x + dx,
                                    self.attrs.y + dy,
                                );
                            }
                            xlib::Button3 => {
                                self.set_cursor(self.cursor.res);
                                let (dw, dh) = diff();
                                self.resize_client(
                                    current_client,
                                    self.attrs.width + dw,
                                    self.attrs.height + dh,
                                );
                            }
                            _ => {}
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
                let mut attrs: xlib::XWindowAttributes = unsafe { mem::zeroed() };
                unsafe {
                    (self.xlib.XGetWindowAttributes)(self.display, window, &mut attrs);
                    (self.xlib.XRaiseWindow)(self.display, event.button.subwindow);
                };
                if attrs.override_redirect == 0
                    && !self.clients.iter().any(|client| client.window == window)
                {
                    self.add_window(window);
                    let index = self.clients.len() - 1;
                    // Update the window type as window may request to be floating and would therefore be flagged to not be arranged.
                    self.update_window_type(index);
                    self.arrange(
                        self.current_monitor,
                        self.monitors[self.current_monitor].get_current_workspace(),
                    );
                    unsafe {
                        (self.xlib.XMapWindow)(self.display, window);
                    }
                    self.set_focus(Some(index));
                }
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
                if let Some((client_index, target_client)) = self
                    .clients
                    .iter()
                    .enumerate()
                    .find(|(_, client)| client.window == unsafe { event.destroy_window.window })
                {
                    let workspace = self.monitors[target_client.monitor].get_current_workspace();
                    // Remove destroyed client.
                    self.clients.remove(client_index);
                    if let Some(new_focus) = self
                        .clients
                        .iter()
                        .rev()
                        .position(|client| self.is_visible(workspace, client))
                    {
                        self.set_focus(Some(self.clients.len() - new_focus - 1));
                    }
                    self.arrange(self.current_monitor, workspace);
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
                let property_event = xlib::XPropertyEvent::from(event);
                if let Some(client_index) = self
                    .clients
                    .iter()
                    .position(|client| client.window == property_event.window)
                {
                    if property_event.atom == xlib::XA_WM_NORMAL_HINTS {
                        self.update_size_hints(client_index);
                        let (width, height) = self.apply_size_hints(client_index);
                        self.clients[client_index].floating = true;
                        self.resize_client(client_index, width, height);
                    } else if property_event.atom == self.atoms.net_wm_window_type {
                        self.update_window_type(client_index);
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn scan(&mut self) {
        let mut root_return = 0;
        let mut parent_return = 0;
        let mut array: *mut xlib::Window = unsafe { std::mem::zeroed() };
        let mut length = 0;
        if unsafe {
            (self.xlib.XQueryTree)(
                self.display,
                self.root,
                &mut root_return,
                &mut parent_return,
                &mut array,
                &mut length,
            )
        } != 0
        {
            let windows: Vec<&xlib::Window> =
                unsafe { slice::from_raw_parts(array, length as usize) }
                    .iter()
                    .filter(|window| {
                        let mut attrs: xlib::XWindowAttributes = unsafe { mem::zeroed() };
                        let status = unsafe {
                            (self.xlib.XGetWindowAttributes)(self.display, **window, &mut attrs)
                        };
                        status != 0
                            && attrs.override_redirect == 0
                            && attrs.map_state == xlib::IsViewable
                    })
                    .collect();
            info!("Queried {} viewable windows", windows.len());
            windows.iter().for_each(|window| self.add_window(**window));
            self.arrange(
                self.current_monitor,
                self.monitors[self.current_monitor].get_current_workspace(),
            );
            self.set_focus(if !self.clients.is_empty() {
                Some(0)
            } else {
                None
            });
            windows.iter().for_each(|window| unsafe {
                (self.xlib.XMapWindow)(self.display, **window);
            });
        }
    }

    fn add_window(&mut self, window: xlib::Window) {
        unsafe {
            (self.xlib.XSelectInput)(
                self.display,
                window,
                xlib::StructureNotifyMask | xlib::EnterWindowMask | xlib::PropertyChangeMask,
            );
        };
        let workspace = self.monitors[self.current_monitor].get_current_workspace();
        self.clients.push(Client::fetch(
            &self.xlib,
            self.display,
            window,
            self.current_monitor,
            workspace,
        ));
    }

    // Return if client is visible in the current monitor in given workspace.
    fn is_visible(&self, workspace: usize, client: &Client) -> bool {
        client.monitor == self.current_monitor && client.workspace == workspace
    }

    fn set_cursor(&self, cursor: XCursor) {
        unsafe { (self.xlib.XDefineCursor)(self.display, self.root, cursor) };
    }

    fn arrange(&mut self, monitor: usize, workspace: usize) {
        let layout = self.monitors[monitor].get_layout();
        for (index, geometry) in layout(
            monitor,
            workspace,
            self.monitors[monitor].get_geometry(),
            &self.clients,
        )
        .iter()
        .enumerate()
        {
            if self.clients[index].get_geometry() != geometry {
                unsafe {
                    (self.xlib.XMoveResizeWindow)(
                        self.display,
                        self.clients[index].window,
                        geometry.x,
                        geometry.y,
                        geometry.width as u32,
                        geometry.height as u32,
                    )
                };
                self.clients[index].update_geometry(&self.xlib, self.display);
            }
        }
    }

    fn move_resize_client(&mut self, index: usize, x: i32, y: i32, width: i32, height: i32) {
        unsafe {
            (self.xlib.XMoveResizeWindow)(
                self.display,
                self.clients[index].window,
                x,
                y,
                width as u32,
                height as u32,
            )
        };
        self.set_client_monitor(index);
    }

    fn move_client(&mut self, index: usize, x: i32, y: i32) {
        unsafe { (self.xlib.XMoveWindow)(self.display, self.clients[index].window, x, y) };
        self.set_client_monitor(index);
    }

    fn resize_client(&mut self, index: usize, width: i32, height: i32) {
        unsafe {
            (self.xlib.XResizeWindow)(
                self.display,
                self.clients[index].window,
                width as u32,
                height as u32,
            )
        };
        self.set_client_monitor(index);
    }

    fn update_size_hints(&mut self, index: usize) {
        let mut supplied = 0;
        let mut size: xlib::XSizeHints = unsafe { mem::zeroed() };
        let client = &mut self.clients[index];
        if unsafe {
            (self.xlib.XGetWMNormalHints)(self.display, client.window, &mut size, &mut supplied)
        } == 0
        {
            size.flags = xlib::PSize;
        }
        let geometry = client.get_geometry_mut();
        // Update base width and height.
        if size.flags & xlib::PBaseSize != 0 {
            geometry.base_width = size.base_width;
            geometry.base_height = size.base_height;
        } else if size.flags & xlib::PMinSize != 0 {
            geometry.base_width = size.min_width;
            geometry.base_height = size.min_height;
        } else {
            geometry.base_width = 0;
            geometry.base_height = 0;
        }
        // Update inc width and height.
        if size.flags & xlib::PResizeInc != 0 {
            geometry.inc_width = size.width_inc;
            geometry.inc_height = size.height_inc;
        } else {
            geometry.inc_width = 0;
            geometry.inc_height = 0;
        }
        // Update max width and height.
        if size.flags & xlib::PMaxSize != 0 {
            geometry.max_width = size.max_width;
            geometry.max_height = size.max_height;
        } else {
            geometry.max_width = 0;
            geometry.max_height = 0;
        }
        if size.flags & xlib::PMinSize != 0 {
            geometry.min_width = size.min_width;
            geometry.min_height = size.min_height;
        } else if size.flags & xlib::PBaseSize != 0 {
            geometry.min_width = size.base_width;
            geometry.min_height = size.base_height;
        } else {
            geometry.min_width = 0;
            geometry.min_height = 0;
        }
        if size.flags & xlib::PAspect != 0 {
            geometry.max_aspect = (size.max_aspect.x / size.max_aspect.y) as f32;
            geometry.min_aspect = (size.min_aspect.y / size.min_aspect.x) as f32;
        } else {
            geometry.max_aspect = 0.0;
            geometry.min_aspect = 0.0;
        }
    }

    fn apply_size_hints(&self, index: usize) -> (i32, i32) {
        let geometry = self.clients[index].get_geometry();
        let (mut width, mut height) = (
            cmp::max(geometry.width, geometry.base_width),
            cmp::max(geometry.height, geometry.base_height),
        );
        // height = cmp::max(height, geometry.base_height);
        // width = cmp::max(width, geometry.base_width);
        let base_is_min = geometry.base_width == geometry.min_width
            && geometry.base_height == geometry.min_height;
        if !base_is_min {
            // Remove base dimensions.
            width -= geometry.base_width;
            height -= geometry.base_height;
        }
        // Aspect limits.
        if geometry.min_aspect > 0.0 && geometry.max_aspect > 0.0 {
            if geometry.max_aspect < (width / height) as f32 {
                width = (height as f32 * geometry.max_aspect + 0.5) as i32;
            } else if geometry.min_aspect < (height / width) as f32 {
                height = (width as f32 * geometry.min_aspect + 0.5) as i32;
            }
        }
        if base_is_min {
            width -= geometry.base_width;
            height -= geometry.base_height;
        }
        if geometry.inc_width != 0 {
            width -= width % geometry.inc_width;
        }
        if geometry.inc_height != 0 {
            height -= height % geometry.inc_height;
        }
        width = cmp::max(width + geometry.base_width, geometry.min_width);
        height = cmp::max(height + height, geometry.min_height);
        if geometry.max_width != 0 {
            width = cmp::min(width, geometry.max_width);
        }
        if geometry.max_height != 0 {
            height = cmp::min(height, geometry.max_height);
        }
        (width, height)
    }

    fn update_window_type(&mut self, index: usize) {
        if let Some(state) = self.get_atom_prop(self.clients[index].window, self.atoms.net_wm_state)
        {
            if state == self.atoms.net_wm_state_fullscreen {
                self.toggle_fullscreen(index);
            }
        }
        if let Some(window_type) =
            self.get_atom_prop(self.clients[index].window, self.atoms.net_wm_window_type)
        {
            if window_type == self.atoms.net_wm_window_type_dialog {
                self.clients[index].floating = true;
            }
        }
    }

    fn set_client_monitor(&mut self, index: usize) {
        // Ensure that the client's monitor is correct.
        let client = &mut self.clients[index];
        client.update_geometry(&self.xlib, self.display);
        let geometry = client.get_geometry();
        if let Some(monitor_index) = self
            .monitors
            .iter()
            .position(|monitor| monitor.get_geometry().has_window(&geometry))
        {
            client.monitor = monitor_index;
            client.workspace = self.monitors[monitor_index].get_current_workspace();
        }
    }

    // Set new input focus. If index is None, set focus to root.
    fn set_focus(&mut self, index: Option<usize>) {
        self.current_client = index;
        let new_focus = match index {
            Some(index) => self.clients[index].window,
            None => self.root,
        };
        unsafe {
            (self.xlib.XSetInputFocus)(
                self.display,
                new_focus,
                xlib::RevertToPointerRoot,
                xlib::CurrentTime,
            );
        }
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
                self.monitors[self.current_monitor].get_x(),
                self.monitors[self.current_monitor].get_y(),
                self.monitors[self.current_monitor].get_width(),
                self.monitors[self.current_monitor].get_height(),
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
            self.arrange(
                self.current_monitor,
                self.monitors[self.current_monitor].get_current_workspace(),
            );
        }
    }

    fn fetch_monitors(&mut self) -> CritResult<()> {
        let mut screen_count = 0;
        let raw_infos =
            unsafe { (self.xinerama_xlib.XineramaQueryScreens)(self.display, &mut screen_count) };
        let xinerama_infos: &[xinerama::XineramaScreenInfo] =
            unsafe { slice::from_raw_parts(raw_infos, screen_count as usize) };
        self.monitors = xinerama_infos
            .iter()
            .map(|info| Monitor::new(&self.layouts[0].1, &info))
            .collect();
        Ok(())
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

    pub extern "C" fn xerror(_: *mut xlib::Display, e: *mut xlib::XErrorEvent) -> i32 {
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
