use crate::{
    backend::{
        client::{Client, WindowGeometry},
        monitor::MonitorGeometry,
    },
    layouts::{self, BarStatus},
};

pub fn float(
    monitor_index: usize,
    workspace: usize,
    monitor_geometry: &MonitorGeometry,
    clients: &[Client],
    _bar_status: &BarStatus,
) -> Vec<WindowGeometry> {
    clients
        .iter()
        .map(|client| {
            let mut geometry = client.get_geometry().clone();
            // Set client position to current monitor if it is arrangeable and currently outside.
            if layouts::is_arrangeable(client, monitor_index, workspace)
                && !monitor_geometry.has_window(&geometry)
            {
                geometry.x = monitor_geometry.x;
                geometry.y = monitor_geometry.y;
            }
            geometry
        })
        .collect::<Vec<WindowGeometry>>()
}
