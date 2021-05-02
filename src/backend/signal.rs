use lazy_static::lazy_static;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone)]
pub enum Dir {
    Up,
    Down,
}

#[derive(Debug, Clone)]
pub enum Signal {
    KillClient,
    ToggleFloating,
    SetLayout(usize),
    ChangeWorkspace(usize),
    MoveToWorkspace(usize),
    FocusStack(Dir),
}

lazy_static! {
    // SIGNAL_STACK stores global signals that are executed accordingly in the backend.
    // This system allows signals to be freely added and executed externally.
    pub static ref SIGNAL_STACK: Arc<Mutex<Vec<Signal>>> = Arc::new(Mutex::new(Vec::new()));
}
