use crate::util::{XWindowDimension, XWindowPosition};
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
    pub window: xlib::Window,
}

impl Client {
    pub fn new(xlib: &xlib::Xlib, display: *mut xlib::Display, window: xlib::Window) -> Self {
        Self {
            window_geometry: WindowGeometry::new(xlib, display, &window),
            window,
        }
    }
}
