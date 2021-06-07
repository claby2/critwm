use crate::{
    backend::{client::Client, monitor::Monitor, Backend},
    config,
    layouts::Layout,
};
use serde::Serialize;

// Serialized backend.
#[derive(Serialize)]
pub struct Api<'a> {
    clients: &'a Vec<Client>,
    layouts: &'a Vec<Layout>,
    monitors: &'a Vec<Monitor<{ config::WORKSPACE_COUNT }>>,
    current_client: &'a Option<usize>,
    current_monitor: usize,
}

impl<'a> From<&'a Backend<'a>> for Api<'a> {
    fn from(backend: &'a Backend<'a>) -> Self {
        Self {
            clients: &backend.clients,
            layouts: &backend.layouts,
            monitors: &backend.monitors,
            current_client: &backend.current_client,
            current_monitor: backend.current_monitor,
        }
    }
}
