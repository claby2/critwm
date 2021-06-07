use crate::{
    backend::{
        client::{Client, WindowGeometry},
        monitor::MonitorGeometry,
    },
    config,
    layouts::{self, BarStatus},
};

pub fn tile(
    monitor_index: usize,
    workspace: usize,
    monitor_geometry: &MonitorGeometry,
    clients: &[Client],
    bar_status: &BarStatus,
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
        let bar_margin = layouts::get_bar_margin(bar_status);
        const DOUBLE_GAP: i32 = config::GAP * 2;
        let (x, y, width, height) = (
            monitor_geometry.x,
            monitor_geometry.y + bar_margin,
            monitor_geometry.width,
            monitor_geometry.height - bar_margin,
        );
        // The main window is the window that was added last.
        let main = stack_indices[stack_indices.len() - 1];
        window_geometry[main].x = x + config::GAP;
        window_geometry[main].y = y + config::GAP;
        window_geometry[main].height = height - DOUBLE_GAP;
        if stack_indices.len() > 1 {
            let middle_x = width / 2;
            let stack_width = (width - (3 * config::GAP)) / 2;
            let stack_height = (height - (stack_indices.len() as i32 * config::GAP))
                / (stack_indices.len() - 1) as i32;
            window_geometry[main].width = stack_width;
            // Pop out main window.
            stack_indices.pop();
            // Set position of children.
            for (i, geometry_index) in stack_indices.iter().rev().enumerate() {
                let geometry_index = *geometry_index;
                window_geometry[geometry_index].x = x + middle_x + (config::GAP / 2);
                window_geometry[geometry_index].y =
                    y + (i as i32 * (config::GAP + stack_height)) + config::GAP;
                window_geometry[geometry_index].width = stack_width;
                window_geometry[geometry_index].height = stack_height;
            }
        } else {
            // Only one main window with no children.
            window_geometry[main].width = width - DOUBLE_GAP;
        }
    }
    window_geometry
}

#[cfg(test)]
mod tests {
    use super::tile;
    use crate::{
        backend::{
            client::{Client, WindowGeometry},
            monitor::MonitorGeometry,
        },
        config,
        layouts::BarStatus,
    };

    #[test]
    fn single_window() {
        let monitor_index = 0;
        let workspace = 0;
        let monitor_geometry = MonitorGeometry::new(0, 0, 1920, 1080);
        let clients = [Client::new(
            WindowGeometry::default(),
            monitor_index,
            workspace,
        )];
        assert_eq!(
            tile(
                monitor_index,
                workspace,
                &monitor_geometry,
                &clients,
                &BarStatus::Hide
            ),
            vec![WindowGeometry::new(
                config::GAP,
                config::GAP,
                monitor_geometry.width - (2 * config::GAP),
                monitor_geometry.height - (2 * config::GAP),
                0
            )]
        );
    }

    #[test]
    fn ignore_floating_client() {
        let monitor_index = 0;
        let workspace = 0;
        let monitor_geometry = MonitorGeometry::new(0, 0, 1920, 1080);
        let clients = [Client::new(WindowGeometry::default(), monitor_index, workspace).floating()];
        assert_eq!(
            tile(
                monitor_index,
                workspace,
                &monitor_geometry,
                &clients,
                &BarStatus::Hide
            ),
            vec![WindowGeometry::default()]
        );
    }

    #[test]
    fn ignore_fullscreen_client() {
        let monitor_index = 0;
        let workspace = 0;
        let monitor_geometry = MonitorGeometry::new(0, 0, 1920, 1080);
        let clients =
            [Client::new(WindowGeometry::default(), monitor_index, workspace).fullscreen()];
        assert_eq!(
            tile(
                monitor_index,
                workspace,
                &monitor_geometry,
                &clients,
                &BarStatus::Hide
            ),
            vec![WindowGeometry::default()]
        );
    }

    #[test]
    fn three_clients() {
        let monitor_index = 0;
        let workspace = 0;
        let monitor_geometry = MonitorGeometry::new(0, 0, 1920, 1080);
        let clients = [
            Client::new(WindowGeometry::default(), monitor_index, workspace),
            Client::new(WindowGeometry::default(), monitor_index, workspace),
            Client::new(WindowGeometry::default(), monitor_index, workspace),
        ];
        let window_width = (monitor_geometry.width - (3 * config::GAP)) / 2;
        let stack_height = (monitor_geometry.height - (3 * config::GAP)) / 2;
        assert_eq!(
            tile(
                monitor_index,
                workspace,
                &monitor_geometry,
                &clients,
                &BarStatus::Hide
            ),
            vec![
                WindowGeometry::new(
                    (monitor_geometry.width / 2) + (config::GAP / 2),
                    (monitor_geometry.height / 2) + (config::GAP / 2),
                    window_width,
                    stack_height,
                    0,
                ),
                WindowGeometry::new(
                    (monitor_geometry.width / 2) + (config::GAP / 2),
                    config::GAP,
                    window_width,
                    stack_height,
                    0,
                ),
                // Main client.
                WindowGeometry::new(
                    config::GAP,
                    config::GAP,
                    window_width,
                    monitor_geometry.height - (2 * config::GAP),
                    0,
                )
            ]
        );
    }
}
