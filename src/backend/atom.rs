use std::ffi::CString;
use x11_dl::xlib;

#[derive(Debug)]
pub struct Atom {
    pub wm_protocols: xlib::Atom,
    pub wm_delete: xlib::Atom,
    pub net_wm_name: xlib::Atom,
    pub net_supporting_wm_check: xlib::Atom,
}

impl Atom {
    pub fn new(xlib: &xlib::Xlib, display: *mut xlib::Display) -> Self {
        Self {
            wm_protocols: Self::get_atom(xlib, display, "WM_PROTOCOLS"),
            wm_delete: Self::get_atom(xlib, display, "WM_DELETE_WINDOW"),
            net_wm_name: Self::get_atom(xlib, display, "_NET_WM_NAME"),
            net_supporting_wm_check: Self::get_atom(xlib, display, "_NET_SUPPORTING_WM_CHECK"),
        }
    }

    fn get_atom(xlib: &xlib::Xlib, display: *mut xlib::Display, name: &str) -> xlib::Atom {
        unsafe {
            (xlib.XInternAtom)(
                display,
                CString::new(name).unwrap_or_default().into_raw(),
                xlib::False,
            )
        }
    }
}
