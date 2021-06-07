use crate::{
    backend::client::WindowGeometry,
    error::{CritError, CritResult},
    layouts::{BarStatus, Layout},
};
use serde::Serialize;
use std::fmt;
use x11_dl::xinerama;

#[derive(Debug, Serialize)]
pub struct MonitorGeometry {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl MonitorGeometry {
    pub fn new(x: i32, y: i32, width: i32, height: i32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub fn has_window(&self, geometry: &WindowGeometry) -> bool {
        self.has_point(
            geometry.x + (geometry.width / 2),
            geometry.y + (geometry.height / 2),
        )
    }

    pub fn has_point(&self, x: i32, y: i32) -> bool {
        x >= self.x && x <= self.x + self.width && y >= self.y && y <= self.y + self.height
    }
}

#[derive(Serialize)]
pub struct Monitor<const WORKSPACES: usize> {
    current_workspace: usize,
    geometry: MonitorGeometry,
    layout: Layout,
    bar_status: BarStatus,
}

impl<const WORKSPACES: usize> fmt::Debug for Monitor<WORKSPACES> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // Write all fields in Monitor except layout.
        write!(f, "{:?}", (self.current_workspace, &self.geometry))
    }
}

impl<const WORKSPACES: usize> Monitor<WORKSPACES> {
    pub fn new(layout: &Layout, info: &xinerama::XineramaScreenInfo) -> Self {
        Self {
            current_workspace: 0,
            geometry: MonitorGeometry::new(
                info.x_org as i32,
                info.y_org as i32,
                info.width as i32,
                info.height as i32,
            ),
            layout: layout.clone(),
            bar_status: BarStatus::default(),
        }
    }

    pub fn get_current_workspace(&self) -> usize {
        self.current_workspace
    }

    pub fn set_current_workspace(&mut self, workspace: usize) -> CritResult<()> {
        if workspace <= WORKSPACES {
            self.current_workspace = workspace;
            Ok(())
        } else {
            Err(CritError::Other(format!(
                "Workspace of value {} cannot be set. Maximum value is {}",
                workspace, WORKSPACES
            )))
        }
    }

    pub fn get_layout(&self) -> &Layout {
        &self.layout
    }

    pub fn set_layout(&mut self, layout: &Layout) {
        self.layout = layout.clone();
    }

    pub fn get_geometry(&self) -> &MonitorGeometry {
        &self.geometry
    }

    pub fn get_x(&self) -> i32 {
        self.geometry.x
    }

    pub fn get_y(&self) -> i32 {
        self.geometry.y
    }

    pub fn get_width(&self) -> i32 {
        self.geometry.width
    }

    pub fn get_height(&self) -> i32 {
        self.geometry.height
    }

    pub fn get_bar_status(&self) -> &BarStatus {
        &self.bar_status
    }

    pub fn toggle_bar_status(&mut self) {
        self.bar_status = match self.bar_status {
            BarStatus::Show => BarStatus::Hide,
            BarStatus::Hide => BarStatus::Show,
        }
    }
}
