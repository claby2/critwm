use crate::backend::signal::{Signal, SIGNAL_STACK};
use std::{
    os::raw::{c_int, c_uint, c_ulong},
    process::Command,
};

pub type ModMask = c_uint;
pub type XKeysym = c_ulong;
pub type XCursorShape = c_uint;
pub type XCursor = c_ulong;
pub type XWindowPosition = c_int;
pub type XWindowDimension = c_uint;

#[derive(Debug)]
pub struct Cursor {
    pub norm: XCursor,
    pub res: XCursor,
    pub mov: XCursor,
}

impl Cursor {
    pub const NORM: XCursorShape = 68;
    pub const RES: XCursorShape = 120;
    pub const MOV: XCursorShape = 52;

    pub fn new(norm: XCursor, res: XCursor, mov: XCursor) -> Self {
        Self { norm, res, mov }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Key {
    pub modifier: ModMask,
    pub sym: XKeysym,
}

impl Key {
    pub fn new(modifier: ModMask, sym: XKeysym) -> Self {
        Self { modifier, sym }
    }
}

pub type Action = Box<dyn Fn()>;

macro_rules! key {
    ($modifier:expr, $sym:expr, $action:expr) => {
        (
            Key::new($modifier, $sym as crate::util::XKeysym),
            Box::new(move || $action),
        )
    };
}

pub fn spawn(parts: &str) {
    let parts = String::from(parts);
    let mut parts = parts.trim().split_whitespace();
    Command::new(parts.next().unwrap())
        .args(parts)
        .spawn()
        .unwrap();
}

pub fn signal(signal: Signal) {
    SIGNAL_STACK.lock().unwrap().push(signal);
}
