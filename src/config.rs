use crate::util::{self, Action, Key, ModMask};
use std::collections::HashMap;
use x11_dl::{keysym::*, xlib::*};

pub const MODKEY: ModMask = Mod4Mask;
const TERMINAL: &str = "st";

pub fn get_keymap() -> HashMap<Key, Action> {
    keymap![
        (MODKEY, XK_space, util::spawn, Argument::from("dmenu_run"),),
        (MODKEY, XK_Return, util::spawn, Argument::from(TERMINAL)),
        (MODKEY | ShiftMask, XK_q, util::quit, Argument::default())
    ]
}
