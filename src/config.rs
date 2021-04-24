use crate::{
    backend::signal::Signal,
    util::{self, Action, Argument, Key, ModMask},
};
use std::collections::HashMap;
use x11_dl::{keysym::*, xlib::*};

pub const WORKSPACE_COUNT: usize = 9;
// pub const WORKSPACES: [&str; WORKSPACE_COUNT] = ["1", "2", "3", "4", "5", "6", "7", "8", "9"];
pub const MODKEY: ModMask = Mod4Mask;
const TAG_KEYS: [u32; WORKSPACE_COUNT] = [XK_1, XK_2, XK_3, XK_4, XK_5, XK_6, XK_7, XK_8, XK_9];
const TERMINAL: &str = "st";

pub fn get_keymap() -> HashMap<Key, Action> {
    let mut keymap = keymap![
        (MODKEY, XK_space, util::spawn, Argument::from("dmenu_run")),
        (MODKEY, XK_Return, util::spawn, Argument::from(TERMINAL)),
        (
            MODKEY,
            XK_w,
            util::signal,
            Argument::Signal(Signal::KillClient)
        ),
        (MODKEY | ShiftMask, XK_q, util::quit, Argument::Void)
    ];
    for (i, tag_key) in TAG_KEYS.iter().enumerate() {
        keymap.insert(
            key!(MODKEY, *tag_key),
            action!(util::signal, Argument::Signal(Signal::ChangeWorkspace(i))),
        );
        keymap.insert(
            key!(MODKEY | ShiftMask, *tag_key),
            action!(util::signal, Argument::Signal(Signal::MoveToWorkspace(i))),
        );
    }
    keymap
}
