use crate::backend::Backend;
use std::{ffi::CString, mem};
use x11_dl::xlib;

impl Backend {
    pub fn set_hints(&self) {
        // Set WM name.
        let wm_name = "critwm";
        self.set_prop_string(self.atoms.net_wm_name, wm_name);
        self.set_prop_u64(
            self.atoms.net_supporting_wm_check,
            xlib::XA_WINDOW,
            self.root,
        );
    }

    fn set_prop_string(&self, atom: xlib::Atom, value: &str) {
        if let Ok(cstring) = CString::new(value) {
            unsafe {
                (self.xlib.XChangeProperty)(
                    self.display,
                    self.root,
                    atom,
                    xlib::XA_CARDINAL,
                    8,
                    xlib::PropModeReplace,
                    cstring.as_ptr().cast::<u8>(),
                    value.len() as i32,
                )
            };
        }
    }

    fn set_prop_u64(&self, atom: xlib::Atom, type_: u64, value: u64) {
        let data = vec![value as u32];
        unsafe {
            (self.xlib.XChangeProperty)(
                self.display,
                self.root,
                atom,
                type_,
                32,
                xlib::PropModeReplace,
                data.as_ptr().cast::<u8>(),
                1_i32,
            );
            mem::forget(data);
        };
    }
}
