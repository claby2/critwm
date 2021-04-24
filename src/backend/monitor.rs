use crate::{
    backend::client::WindowGeometry,
    error::{CritError, CritResult},
    util::XineramaInfo,
};
use x11_dl::xinerama;

#[derive(Debug)]
pub struct Monitor<const WORKSPACES: usize> {
    current_workspace: usize,
    x: XineramaInfo,
    y: XineramaInfo,
    width: XineramaInfo,
    height: XineramaInfo,
}

impl<const WORKSPACES: usize> From<&xinerama::XineramaScreenInfo> for Monitor<WORKSPACES> {
    fn from(info: &xinerama::XineramaScreenInfo) -> Self {
        Self {
            current_workspace: 0,
            x: info.x_org,
            y: info.y_org,
            width: info.width,
            height: info.height,
        }
    }
}

impl<const WORKSPACES: usize> Monitor<WORKSPACES> {
    pub fn has_window(&self, geometry: &WindowGeometry) -> bool {
        self.has_point(
            geometry.x as u32 + (geometry.width / 2),
            geometry.y as u32 + (geometry.height / 2),
        )
    }

    pub fn has_point(&self, x: u32, y: u32) -> bool {
        // TODO: Clean up number conversions.
        x >= self.x as u32
            && x <= self.x as u32 + self.width as u32
            && y >= self.y as u32
            && y <= self.y as u32 + self.height as u32
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
}
