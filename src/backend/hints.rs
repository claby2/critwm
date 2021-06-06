use crate::backend::Backend;
use std::{ffi::CString, mem};
use x11_dl::xlib;

impl Backend<'_> {
    pub fn set_hints(&self) {
        // Set WM name.
        let wm_name = "critwm";
        self.set_prop_string(self.atoms.net_wm_name, wm_name);
        self.set_prop_u64(
            self.atoms.net_supporting_wm_check,
            xlib::XA_WINDOW,
            self.root,
        );
        // Set supported net atoms.
        // Makes `$ xprop -root _NET_SUPPORTED` list supported net atoms.
        let net_supported = self.atoms.net_supported();
        let net_supported_ptr: *const xlib::Atom = net_supported.as_ptr();
        self.set_prop_u8_ptr(
            self.atoms.net_supported,
            xlib::XA_ATOM,
            net_supported.len() as i32,
            net_supported_ptr.cast::<u8>(),
        );
    }

    pub fn set_window_state(&self, window: xlib::Window, atom: xlib::Atom) {
        let data = vec![atom as u32];
        unsafe {
            (self.xlib.XChangeProperty)(
                self.display,
                window,
                self.atoms.net_wm_state,
                xlib::XA_ATOM,
                32,
                xlib::PropModeReplace,
                data.as_ptr().cast::<u8>(),
                data.len() as i32,
            );
        }
    }

    pub fn get_atom_prop(&self, window: xlib::Window, prop: xlib::Atom) -> Option<xlib::Atom> {
        let mut type_return = 0;
        let mut format_return = 0;
        let mut nitems_return = 0;
        let mut bytes_after_return = 0;
        let mut prop_return = unsafe { mem::zeroed() };
        let status = unsafe {
            (self.xlib.XGetWindowProperty)(
                self.display,
                window,
                prop,
                0,
                mem::size_of::<xlib::Atom>() as i64,
                0,
                xlib::XA_ATOM,
                &mut type_return,
                &mut format_return,
                &mut nitems_return,
                &mut bytes_after_return,
                &mut prop_return,
            )
        };
        Self::get_prop(status, prop_return)
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
        self.set_prop_u8_ptr(atom, type_, 1_i32, vec![value as u32].as_ptr().cast::<u8>());
    }

    fn set_prop_u8_ptr(&self, atom: xlib::Atom, type_: u64, len: i32, value: *const u8) {
        unsafe {
            (self.xlib.XChangeProperty)(
                self.display,
                self.root,
                atom,
                type_,
                32,
                xlib::PropModeReplace,
                value,
                len,
            );
        };
    }

    fn get_prop(status: i32, prop_return: *mut u8) -> Option<xlib::Atom> {
        if status == i32::from(xlib::Success) && !prop_return.is_null() {
            Some(unsafe { *(prop_return as *const xlib::Atom) })
        } else {
            None
        }
    }
}
