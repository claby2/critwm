use crate::{backend::Backend, config, error::CritResult};
use lazy_static::lazy_static;
use std::sync::{Arc, Mutex};
use x11_dl::xlib;

#[derive(Debug, Clone)]
pub enum Dir {
    Up,
    Down,
}

#[derive(Debug, Clone)]
pub enum Signal {
    Quit,
    KillClient,
    ToggleFloating,
    SetLayout(usize),
    ChangeWorkspace(usize),
    MoveToWorkspace(usize),
    FocusMon(Dir),
    FocusStack(Dir),
}

impl Backend<'_> {
    // Returns true if quit signal is handled.
    pub fn handle_signal(&mut self) -> CritResult<bool> {
        if let Some(signal) = SIGNAL_STACK.lock().unwrap().pop() {
            info!("Received signal: {:?}", signal);
            match signal {
                Signal::Quit => {
                    self.quit();
                    return Ok(true);
                }
                Signal::KillClient => self.kill_client(),
                Signal::ToggleFloating => {
                    if let Some(current_client) = self.current_client {
                        self.toggle_floating(current_client);
                    }
                }
                Signal::SetLayout(layout_index) => self.set_layout(layout_index),
                Signal::ChangeWorkspace(new_workspace) => self.change_workspace(new_workspace)?,
                Signal::MoveToWorkspace(new_workspace) => self.move_to_workspace(new_workspace),
                Signal::FocusMon(direction) => self.focus_monitor(direction),
                Signal::FocusStack(direction) => self.focus_stack(direction),
            }
        }
        Ok(false)
    }

    pub fn quit(&mut self) {
        self.clients.iter().for_each(|client| unsafe {
            (self.xlib.XMapWindow)(self.display, client.window);
        });
        unsafe {
            (self.xlib.XSetInputFocus)(
                self.display,
                xlib::PointerRoot as u64,
                xlib::RevertToPointerRoot,
                xlib::CurrentTime,
            );
            (self.xlib.XSync)(self.display, xlib::False);
            (self.xlib.XDestroyWindow)(self.display, self.root);
            (self.xlib.XCloseDisplay)(self.display);
        }
    }

    pub fn kill_client(&self) {
        if let Some(current_client) = self.current_client {
            if let Some(client) = self.clients.get(current_client) {
                // Try kill the client nicely.
                if !self.send_xevent_atom(client.window, self.atoms.wm_delete) {
                    // Force kill the client.
                    unsafe {
                        (self.xlib.XGrabServer)(self.display);
                        (self.xlib.XSetErrorHandler)(Some(Self::xerror_dummy));
                        (self.xlib.XSetCloseDownMode)(self.display, xlib::DestroyAll);
                        (self.xlib.XKillClient)(self.display, client.window);
                        (self.xlib.XSync)(self.display, xlib::False);
                        (self.xlib.XSetErrorHandler)(Some(Self::xerror));
                        (self.xlib.XUngrabServer)(self.display);
                    }
                }
            }
        }
    }

    pub fn toggle_floating(&mut self, index: usize) {
        self.clients[index].floating = !self.clients[index].floating;
        unsafe { (self.xlib.XRaiseWindow)(self.display, self.clients[index].window) };
        self.arrange(
            self.current_monitor,
            self.monitors[self.current_monitor].get_current_workspace(),
        );
    }

    pub fn set_layout(&mut self, layout_index: usize) {
        self.monitors[self.current_monitor].set_layout(&self.layouts[layout_index].1);
        // Ensure that all clients in current monitor are not floating.
        for client in self.clients.iter_mut() {
            if client.monitor == self.current_monitor {
                client.floating = false;
            }
        }
        // Go through each workspace in current monitor and arrange windows.
        for workspace in 0..config::WORKSPACE_COUNT {
            self.arrange(self.current_monitor, workspace);
        }
    }

    pub fn change_workspace(&mut self, new_workspace: usize) -> CritResult<()> {
        // Change workspace of selected monitor to given workspace.
        let monitor = &self.monitors[self.current_monitor];
        if monitor.get_current_workspace() != new_workspace {
            // Unmap windows that are in the old workspace.
            self.clients
                .iter()
                .filter(|client| self.is_visible(monitor.get_current_workspace(), client))
                .for_each(|client| {
                    unsafe { (self.xlib.XUnmapWindow)(self.display, client.window) };
                });
            // Map windows that are in the new workspace.
            self.clients
                .iter()
                .filter(|client| self.is_visible(new_workspace, client))
                .for_each(|client| {
                    unsafe { (self.xlib.XMapWindow)(self.display, client.window) };
                });
            // Update workspace value to new value.
            self.monitors[self.current_monitor].set_current_workspace(new_workspace)?;
        }
        Ok(())
    }

    pub fn move_to_workspace(&mut self, new_workspace: usize) {
        // Move currently focused client to given workspace.
        if let Some(current_client) = self.current_client {
            let mut client = &mut self.clients[current_client];
            if client.workspace != new_workspace {
                client.workspace = new_workspace;
                // Hide window as it has moved to another workspace.
                unsafe { (self.xlib.XUnmapWindow)(self.display, client.window) };
                // Arrange both the current workspace and the new workspace.
                self.arrange(
                    self.current_monitor,
                    self.monitors[self.current_monitor].get_current_workspace(),
                );
                self.arrange(self.current_monitor, new_workspace);
            }
        }
    }

    pub fn focus_monitor(&mut self, direction: Dir) {
        self.current_monitor = match direction {
            Dir::Up => {
                if self.current_monitor < self.monitors.len() - 1 {
                    self.current_monitor + 1
                } else {
                    0
                }
            }
            Dir::Down => {
                if self.current_monitor > 0 {
                    self.current_monitor - 1
                } else {
                    self.monitors.len() - 1
                }
            }
        };
        self.focus_current_monitor();
        if let Some(current_client) = self.current_client {
            let window_geometry = self.clients[current_client].get_geometry();
            self.cursor_warp(
                &self.clients[current_client].window,
                window_geometry.width / 2,
                window_geometry.height / 2,
            );
        } else {
            let monitor_geometry = self.monitors[self.current_monitor].get_geometry();
            self.cursor_warp(
                &self.root,
                monitor_geometry.x + (monitor_geometry.width / 2),
                monitor_geometry.y + (monitor_geometry.height / 2),
            );
        }
    }

    pub fn focus_stack(&mut self, direction: Dir) {
        if let Some(current_client) = self.current_client {
            let workspace = self.monitors[self.current_monitor].get_current_workspace();
            if let Some((index, _)) = match direction {
                Dir::Up => self
                    .clients
                    .iter()
                    .enumerate()
                    .cycle()
                    .skip(current_client + 1)
                    .find(|(_, client)| self.is_visible(workspace, client)),
                Dir::Down => self
                    .clients
                    .iter()
                    .enumerate()
                    .rev()
                    .cycle()
                    .skip(self.clients.len() - current_client)
                    .find(|(_, client)| self.is_visible(workspace, client)),
            } {
                self.set_focus(Some(index));
                let geometry = self.clients[index].get_geometry();
                self.cursor_warp(
                    &self.clients[index].window,
                    geometry.width / 2,
                    geometry.height / 2,
                );
            };
        }
    }

    fn cursor_warp(&self, window: &xlib::Window, x: i32, y: i32) {
        unsafe { (self.xlib.XWarpPointer)(self.display, 0, *window, 0, 0, 0, 0, x, y) };
    }
}

lazy_static! {
    // SIGNAL_STACK stores global signals that are executed accordingly in the backend.
    // This system allows signals to be freely added and executed externally.
    pub static ref SIGNAL_STACK: Arc<Mutex<Vec<Signal>>> = Arc::new(Mutex::new(Vec::new()));
}
