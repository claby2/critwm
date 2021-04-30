pub mod float;
pub mod tile;

use crate::backend::{
    client::{Client, WindowGeometry},
    monitor::MonitorGeometry,
};

pub type Layout = fn(usize, usize, &MonitorGeometry, &[Client]) -> Vec<WindowGeometry>;
