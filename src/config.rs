use crate::{
    backend::signal::Signal,
    layouts::{self, Layout},
    util::{self, Action, Key, ModMask},
};
use std::{collections::HashMap, process};
use x11_dl::{keysym::*, xlib::*};

pub const WORKSPACE_COUNT: usize = 9;
// pub const WORKSPACES: [&str; WORKSPACE_COUNT] = ["1", "2", "3", "4", "5", "6", "7", "8", "9"];
const TAG_KEYS: [u32; WORKSPACE_COUNT] = [XK_1, XK_2, XK_3, XK_4, XK_5, XK_6, XK_7, XK_8, XK_9];

pub const MODKEY: ModMask = Mod4Mask;
const TERMINAL: &str = "st";

pub fn get_keymap() -> HashMap<Key, Action> {
    let mut keymap: Vec<(Key, Action)> = vec![
        key!(MODKEY, XK_space, util::spawn("dmenu_run")),
        key!(MODKEY, XK_Return, util::spawn(TERMINAL)),
        key!(MODKEY, XK_w, util::signal(Signal::KillClient)),
        key!(MODKEY, XK_s, util::signal(Signal::ToggleFloating)),
        key!(MODKEY, XK_t, util::signal(Signal::SetLayout(0))),
        key!(MODKEY, XK_f, util::signal(Signal::SetLayout(1))),
        key!(MODKEY | ShiftMask, XK_q, process::exit(0)),
    ];
    for (i, tag_key) in TAG_KEYS.iter().enumerate() {
        // Add workspace changing binds.
        keymap.push(key!(
            MODKEY,
            *tag_key,
            util::signal(Signal::ChangeWorkspace(i))
        ));
        // Add workspace moving binds.
        keymap.push(key!(
            MODKEY | ShiftMask,
            *tag_key,
            util::signal(Signal::MoveToWorkspace(i))
        ));
    }
    keymap.into_iter().collect::<HashMap<Key, Action>>()
}

pub fn get_layouts() -> Vec<(String, Box<Layout>)> {
    vec![
        // First entry is default.
        (String::from("[]="), Box::new(layouts::tile::tile)),
        (String::from("><>"), Box::new(layouts::float::float)),
    ]
}
