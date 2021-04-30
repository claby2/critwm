use crate::{
    backend::client::WindowGeometry,
    config,
    error::{CritError, CritResult},
    layouts::Layout,
    util::XineramaInfo,
};
use std::{
    fmt,
    iter::FromIterator,
    ops::{Index, IndexMut},
    slice::Iter,
};
use x11_dl::xinerama;

#[derive(Debug)]
pub struct MonitorGeometry {
    pub x: XineramaInfo,
    pub y: XineramaInfo,
    pub width: XineramaInfo,
    pub height: XineramaInfo,
}

impl MonitorGeometry {
    pub fn new(
        x: XineramaInfo,
        y: XineramaInfo,
        width: XineramaInfo,
        height: XineramaInfo,
    ) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }
}

pub struct Monitor<const WORKSPACES: usize> {
    current_workspace: usize,
    geometry: MonitorGeometry,
    layout: Layout,
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
            geometry: MonitorGeometry::new(info.x_org, info.y_org, info.width, info.height),
            layout: *layout,
        }
    }
}

pub trait AnyMonitor {
    fn has_window(&self, geometry: &WindowGeometry) -> bool;
    fn has_point(&self, x: u32, y: u32) -> bool;
    fn get_current_workspace(&self) -> usize;
    fn set_current_workspace(&mut self, workspace: usize) -> CritResult<()>;
    fn get_layout(&self) -> Layout;
    fn set_layout(&mut self, layout: &Layout);
    fn get_geometry(&self) -> &MonitorGeometry;
    fn get_x(&self) -> XineramaInfo;
    fn get_y(&self) -> XineramaInfo;
    fn get_width(&self) -> XineramaInfo;
    fn get_height(&self) -> XineramaInfo;
}

impl fmt::Debug for dyn AnyMonitor {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl<const WORKSPACES: usize> AnyMonitor for Monitor<WORKSPACES> {
    fn has_window(&self, geometry: &WindowGeometry) -> bool {
        self.has_point(
            geometry.x as u32 + (geometry.width / 2),
            geometry.y as u32 + (geometry.height / 2),
        )
    }

    fn has_point(&self, x: u32, y: u32) -> bool {
        // TODO: Clean up number conversions.
        x >= self.geometry.x as u32
            && x <= self.geometry.x as u32 + self.geometry.width as u32
            && y >= self.geometry.y as u32
            && y <= self.geometry.y as u32 + self.geometry.height as u32
    }

    fn get_current_workspace(&self) -> usize {
        self.current_workspace
    }

    fn set_current_workspace(&mut self, workspace: usize) -> CritResult<()> {
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

    fn get_layout(&self) -> Layout {
        self.layout
    }

    fn set_layout(&mut self, layout: &Layout) {
        self.layout = *layout;
    }

    fn get_geometry(&self) -> &MonitorGeometry {
        &self.geometry
    }

    fn get_x(&self) -> XineramaInfo {
        self.geometry.x
    }

    fn get_y(&self) -> XineramaInfo {
        self.geometry.y
    }

    fn get_width(&self) -> XineramaInfo {
        self.geometry.width
    }

    fn get_height(&self) -> XineramaInfo {
        self.geometry.height
    }
}

#[derive(Debug)]
pub struct MonitorManager(Vec<Box<dyn AnyMonitor>>);

impl Index<usize> for MonitorManager {
    type Output = Box<dyn AnyMonitor>;
    fn index(&self, index: usize) -> &Self::Output {
        &self.0[index]
    }
}

impl IndexMut<usize> for MonitorManager {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.0[index]
    }
}

impl FromIterator<Monitor<{ config::WORKSPACE_COUNT }>> for MonitorManager {
    fn from_iter<T>(iter: T) -> Self
    where
        T: IntoIterator<Item = Monitor<{ config::WORKSPACE_COUNT }>>,
    {
        let mut monitor_manager = Self::new();
        for monitor in iter {
            monitor_manager.0.push(Box::new(monitor));
        }
        monitor_manager
    }
}

impl MonitorManager {
    pub fn new() -> Self {
        Self { 0: Vec::new() }
    }

    pub fn iter(&self) -> Iter<'_, Box<dyn AnyMonitor>> {
        self.0.iter()
    }
}
