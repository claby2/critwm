use crate::backend::signal::{Signal, SIGNAL_STACK};
use std::{
    os::raw::{c_int, c_uint, c_ulong},
    process::{self, Command},
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

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum Argument {
    Void,
    Int(isize),
    UInt(usize),
    Float(f32),
    Str(String),
    Signal(Signal),
}

impl From<&str> for Argument {
    fn from(s: &str) -> Self {
        Self::Str(s.to_owned())
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

#[derive(Debug, Clone)]
pub struct Action {
    function: fn(Argument),
    argument: Argument,
}

impl Action {
    pub fn new(function: fn(Argument), argument: Argument) -> Self {
        Self { function, argument }
    }

    pub fn run(&self) {
        (self.function)(self.argument.clone());
    }
}

macro_rules! keymap {
    [$($key:expr),+] => {
        {
            use crate::util::*;
            [
                $((Key::new($key.0, $key.1 as XKeysym), Action::new($key.2 as fn(Argument), $key.3))),+
            ].iter().cloned().collect()
        }
    }
}

pub fn spawn(arg: Argument) {
    if let Argument::Str(program) = arg {
        Command::new(program).spawn().unwrap();
    }
}

pub fn signal(arg: Argument) {
    if let Argument::Signal(signal) = arg {
        SIGNAL_STACK.lock().unwrap().push(signal);
    }
}

pub fn quit(_: Argument) {
    process::exit(0);
}
