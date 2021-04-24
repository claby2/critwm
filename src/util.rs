use std::{
    os::raw::{c_uint, c_ulong},
    process::{self, Command},
};

pub type ModMask = c_uint;
pub type XKeysym = c_ulong;
pub type XCursorShape = c_uint;
pub type XCursor = c_ulong;

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

#[derive(Debug, Default, Clone)]
pub struct Argument {
    i: Option<isize>,
    ui: Option<usize>,
    f: Option<f32>,
    s: Option<String>,
}

macro_rules! argument_from {
    ($t:ty, $i:ident) => {
        impl From<$t> for Argument {
            fn from(e: $t) -> Self {
                Self {
                    $i: Some(e.to_owned()),
                    ..Default::default()
                }
            }
        }
    };
}

argument_from!(&isize, i);
argument_from!(&usize, ui);
argument_from!(&f32, f);
argument_from!(&str, s);

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
    if let Some(program) = arg.s {
        Command::new(program).spawn().unwrap();
    }
}

pub fn quit(_: Argument) {
    process::exit(0);
}
