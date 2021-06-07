pub mod float;
pub mod tile;

use crate::{
    backend::{
        client::{Client, WindowGeometry},
        monitor::MonitorGeometry,
    },
    config,
};
use serde::{Serialize, Serializer};
use std::fmt;

#[derive(Debug)]
pub enum BarStatus {
    Show,
    Hide,
}

impl Serialize for BarStatus {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_bool(match self {
            Self::Show => true,
            Self::Hide => false,
        })
    }
}

impl Default for BarStatus {
    fn default() -> Self {
        Self::Show
    }
}

fn is_arrangeable(client: &Client, monitor_index: usize, workspace: usize) -> bool {
    // Layouts should only modify the geometry of clients that are arrangeable.
    !client.fullscreen
        && !client.floating
        && !client.dock
        && client.monitor == monitor_index
        && client.workspace == workspace
}

fn get_bar_margin(bar_status: &BarStatus) -> i32 {
    match bar_status {
        BarStatus::Show => config::BAR_MARGIN,
        BarStatus::Hide => 0,
    }
}

pub type LayoutFunc =
    fn(usize, usize, &MonitorGeometry, &[Client], &BarStatus) -> Vec<WindowGeometry>;

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
