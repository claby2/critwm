pub mod float;
pub mod tile;

use crate::backend::{
    client::{Client, WindowGeometry},
    monitor::MonitorGeometry,
};

fn is_arrangeable(client: &Client, monitor_index: usize, workspace: usize) -> bool {
    // Layouts should only modify the geometry of clients that are arrangeable.
    !client.fullscreen
        && !client.floating
        && client.monitor == monitor_index
        && client.workspace == workspace
}

pub type Layout = fn(usize, usize, &MonitorGeometry, &[Client]) -> Vec<WindowGeometry>;
