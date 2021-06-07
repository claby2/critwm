pub mod float;
pub mod tile;

use crate::backend::{
    client::{Client, WindowGeometry},
    monitor::MonitorGeometry,
};
use serde::Serialize;
use std::fmt;

fn is_arrangeable(client: &Client, monitor_index: usize, workspace: usize) -> bool {
    // Layouts should only modify the geometry of clients that are arrangeable.
    !client.fullscreen
        && !client.floating
        && client.monitor == monitor_index
        && client.workspace == workspace
}

pub type LayoutFunc = fn(usize, usize, &MonitorGeometry, &[Client]) -> Vec<WindowGeometry>;

#[derive(Serialize, Clone)]
pub struct Layout {
    pub symbol: String,
    #[serde(skip_serializing)]
    pub func: LayoutFunc,
}

impl fmt::Debug for Layout {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // Only write symbol.
        write!(f, "{}", self.symbol)
    }
}

impl Layout {
    pub fn new(symbol: &str, func: LayoutFunc) -> Self {
        Self {
            symbol: symbol.to_owned(),
            func,
        }
    }
}
