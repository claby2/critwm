use crate::util::{XWindowDimension, XWindowPosition};
use serde::Serialize;
use x11_dl::xlib;

#[derive(Debug, Clone, PartialEq, Default, Serialize)]
pub struct WindowGeometry {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub border_width: i32,

    // Hints.
    pub base_width: i32,
    pub base_height: i32,
    pub max_width: i32,
    pub max_height: i32,
    pub min_width: i32,
    pub min_height: i32,
    // Program specified resize increments.
    pub inc_width: i32,
    pub inc_height: i32,
    // Program specified min and max aspect ratios.
    pub max_aspect: f32,
    pub min_aspect: f32,
}

impl WindowGeometry {
    fn fetch(xlib: &xlib::Xlib, display: *mut xlib::Display, window: &xlib::Window) -> Self {
        let mut root: xlib::Window = 0;
        let mut x: XWindowPosition = 0;
        let mut y: XWindowPosition = 0;
        let mut width: XWindowDimension = 0;
        let mut height: XWindowDimension = 0;
        let mut border_width: XWindowDimension = 0;
        let mut depth: XWindowDimension = 0;
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
                &mut depth,
            )
        };
        Self {
            x,
            y,
            width: width as i32,
            height: height as i32,
            border_width: border_width as i32,
            ..Default::default()
        }
    }

    #[cfg(test)]
    pub fn new(x: i32, y: i32, width: i32, height: i32, border_width: i32) -> Self {
        Self {
            x,
            y,
            width,
            height,
            border_width,
            ..Default::default()
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Client {
    geometry: WindowGeometry,
    // old_geometry stores the geometry of the window before fullscreen was toggled.
    old_geometry: WindowGeometry,
    pub window: xlib::Window,
    pub monitor: usize,
    pub workspace: usize,
    pub fullscreen: bool,
    pub floating: bool,
}

impl Client {
    pub fn fetch(
        xlib: &xlib::Xlib,
        display: *mut xlib::Display,
        window: xlib::Window,
        monitor: usize,
        workspace: usize,
    ) -> Self {
        let geometry = WindowGeometry::fetch(xlib, display, &window);
        Self {
            geometry: geometry.clone(),
            old_geometry: geometry,
            window,
            monitor,
            workspace,
            fullscreen: false,
            floating: false,
        }
    }

    #[cfg(test)]
    pub fn new(geometry: WindowGeometry, monitor: usize, workspace: usize) -> Self {
        let old_geometry = geometry.clone();
        Self {
            geometry,
            old_geometry,
            window: 0,
            monitor,
            workspace,
            fullscreen: false,
            floating: false,
        }
    }

    #[cfg(test)]
    pub fn fullscreen(mut self) -> Self {
        self.fullscreen = true;
        self
    }

    #[cfg(test)]
    pub fn floating(mut self) -> Self {
        self.floating = true;
        self
    }

    pub fn get_geometry(&self) -> &WindowGeometry {
        &self.geometry
    }

    pub fn get_geometry_mut(&mut self) -> &mut WindowGeometry {
        &mut self.geometry
    }

    pub fn get_old_geometry(&self) -> &WindowGeometry {
        &self.old_geometry
    }

    pub fn update_geometry(&mut self, xlib: &xlib::Xlib, display: *mut xlib::Display) {
        self.geometry = WindowGeometry::fetch(xlib, display, &self.window);
    }

    pub fn toggle_fullscreen(&mut self) {
        if !self.fullscreen {
            // Cache geometry if client is being toggled to fullscreen.
            self.old_geometry = self.geometry.clone();
        }
        self.fullscreen = !self.fullscreen;
    }
}
