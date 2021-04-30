use crate::backend::{
    client::{Client, WindowGeometry},
    monitor::MonitorGeometry,
};

pub fn float(
    monitor_index: usize,
    workspace: usize,
    monitor_geometry: &MonitorGeometry,
    clients: &[Client],
) -> Vec<WindowGeometry> {
    let mut last = None;
    for (index, client) in clients.iter().enumerate() {
        if client.monitor == monitor_index && client.workspace == workspace {
            last = Some(index);
        }
    }
    let mut window_geometry = clients
        .iter()
        .map(|client| client.get_geometry())
        .cloned()
        .collect::<Vec<WindowGeometry>>();
    if let Some(last) = last {
        if let Some(geometry) = window_geometry.get_mut(last) {
            geometry.x = monitor_geometry.x as i32;
            geometry.y = monitor_geometry.y as i32;
        }
    }
    window_geometry
}
