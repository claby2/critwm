use crate::util::{XWindowDimension, XWindowPosition};
use x11_dl::xlib;

#[derive(Debug, Clone)]
pub struct WindowGeometry {
    pub x: XWindowPosition,
    pub y: XWindowPosition,
    pub width: XWindowDimension,
    pub height: XWindowDimension,
    pub border_width: XWindowDimension,
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

#[derive(Debug, Clone)]
pub struct Client {
    geometry: WindowGeometry,
    // old_geometry stores the geometry of the window before fullscreen was toggled.
    old_geometry: WindowGeometry,
    pub window: xlib::Window,
    pub monitor: usize,
    pub workspace: usize,
    pub fullscreen: bool,
}

impl Client {
    pub fn new(
        xlib: &xlib::Xlib,
        display: *mut xlib::Display,
        window: xlib::Window,
        monitor: usize,
        workspace: usize,
    ) -> Self {
        let geometry = WindowGeometry::new(xlib, display, &window);
        Self {
            geometry: geometry.clone(),
            old_geometry: geometry,
            window,
            monitor,
            workspace,
            fullscreen: false,
        }
    }

    pub fn get_geometry(&self) -> &WindowGeometry {
        &self.geometry
    }

    pub fn get_old_geometry(&self) -> &WindowGeometry {
        &self.old_geometry
    }

    pub fn update_geometry(&mut self, xlib: &xlib::Xlib, display: *mut xlib::Display) {
        self.geometry = WindowGeometry::new(xlib, display, &self.window);
    }

    pub fn toggle_fullscreen(&mut self) {
        if !self.fullscreen {
            // Cache geometry if client is being toggled to fullscreen.
            self.old_geometry = self.geometry.clone();
        }
        self.fullscreen = !self.fullscreen;
    }
}
