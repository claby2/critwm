use crate::backend::{
    client::{Client, WindowGeometry},
    monitor::MonitorGeometry,
};

pub fn tile(
    monitor_index: usize,
    workspace: usize,
    monitor_geometry: &MonitorGeometry,
    clients: &[Client],
) -> Vec<WindowGeometry> {
    let mut stack_indices = Vec::new();
    for (index, client) in clients.iter().enumerate() {
        if client.monitor == monitor_index && client.workspace == workspace {
            stack_indices.push(index);
        }
    }
    let mut window_geometry = clients
        .iter()
        .map(|client| client.get_geometry())
        .cloned()
        .collect::<Vec<WindowGeometry>>();
    if !stack_indices.is_empty() {
        let (x, y, width, height) = (
            monitor_geometry.x as i32,
            monitor_geometry.y as i32,
            monitor_geometry.width as u32,
            monitor_geometry.height as u32,
        );
        // The main window is the window that was added last.
        let main = stack_indices[stack_indices.len() - 1];
        window_geometry[main].x = x;
        window_geometry[main].y = y;
        window_geometry[main].height = height;
        if stack_indices.len() > 1 {
            let middle_x = width / 2;
            let stack_height = height / (stack_indices.len() - 1) as u32;
            window_geometry[main].width = middle_x;
            // Pop out main window.
            stack_indices.pop();
            // Set position of children.
            for (i, geometry_index) in stack_indices.iter().enumerate() {
                let geometry_index = *geometry_index;
                window_geometry[geometry_index].x = x + middle_x as i32;
                window_geometry[geometry_index].y = y + (i as i32 * stack_height as i32);
                window_geometry[geometry_index].width = middle_x;
                window_geometry[geometry_index].height = stack_height;
            }
        } else {
            window_geometry[main].width = width;
        }
    }
    window_geometry
}
