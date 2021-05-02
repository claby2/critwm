use crate::{
    backend::{
        client::{Client, WindowGeometry},
        monitor::MonitorGeometry,
    },
    config, layouts,
};

pub fn tile(
    monitor_index: usize,
    workspace: usize,
    monitor_geometry: &MonitorGeometry,
    clients: &[Client],
) -> Vec<WindowGeometry> {
    let mut stack_indices = Vec::new();
    for (index, _) in clients
        .iter()
        .enumerate()
        .filter(|(_, client)| layouts::is_arrangeable(client, monitor_index, workspace))
    {
        stack_indices.push(index);
    }
    let mut window_geometry = clients
        .iter()
        .map(|client| client.get_geometry())
        .cloned()
        .collect::<Vec<WindowGeometry>>();
    if !stack_indices.is_empty() {
        const DOUBLE_GAP: i32 = config::GAP * 2;
        let (x, y, width, height) = (
            monitor_geometry.x,
            monitor_geometry.y,
            monitor_geometry.width,
            monitor_geometry.height,
        );
        // The main window is the window that was added last.
        let main = stack_indices[stack_indices.len() - 1];
        window_geometry[main].x = x + config::GAP;
        window_geometry[main].y = y + config::GAP;
        window_geometry[main].height = height - DOUBLE_GAP;
        if stack_indices.len() > 1 {
            let middle_x = width / 2;
            let stack_height = (height - config::GAP) / (stack_indices.len() - 1) as i32;
            window_geometry[main].width = middle_x - config::GAP;
            // Pop out main window.
            stack_indices.pop();
            // Set position of children.
            for (i, geometry_index) in stack_indices.iter().rev().enumerate() {
                let geometry_index = *geometry_index;
                window_geometry[geometry_index].x = x + middle_x + config::GAP;
                window_geometry[geometry_index].y = y + (i as i32 * stack_height) + config::GAP;
                window_geometry[geometry_index].width = middle_x - DOUBLE_GAP;
                window_geometry[geometry_index].height = stack_height - config::GAP;
            }
        } else {
            // Only one main window with no children.
            window_geometry[main].width = width - DOUBLE_GAP;
        }
    }
    window_geometry
}
