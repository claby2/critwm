use crate::{
    backend::{api::Api, Backend},
    error::CritResult,
};
use std::{
    io::Write,
    os::unix::net::{UnixListener, UnixStream},
    path::PathBuf,
    sync::Arc,
};
use tokio::{fs, sync::Mutex, task};

pub const SOCKET_PATH: &str = "/tmp/critwm_state.sock";

#[derive(Debug, Default)]
pub struct State {
    streams: Vec<Option<UnixStream>>,
    last_state: String,
}

#[derive(Debug)]
pub struct StateSocket {
    state: Arc<Mutex<State>>,
    listener: Option<task::JoinHandle<()>>,
    socket_path: PathBuf,
}

impl StateSocket {
    pub fn new(socket_path: PathBuf) -> Self {
        Self {
            state: Arc::new(Mutex::new(State::default())),
            listener: None,
            socket_path,
        }
    }

    pub async fn listen(&mut self) -> CritResult<()> {
        let state = self.state.clone();
        let listener = UnixListener::bind(&self.socket_path)?;
        self.listener = Some(tokio::spawn(async move {
            loop {
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        let mut state = state.lock().await;
                        if stream.write_all(state.last_state.as_bytes()).is_ok() {
                            info!("Pushed stream: {:?}", stream);
                            state.streams.push(Some(stream));
                        }
                    }
                    Err(e) => {
                        error!("Listener accept failed: {:?}", e);
                    }
                }
            }
        }));
        Ok(())
    }

    pub async fn write(&mut self, backend: &Backend<'_>) -> CritResult<()> {
        if self.listener.is_some() {
            let api = Api::from(backend);
            let mut json = serde_json::to_string(&api)?;
            json.push('\n');
            let mut state = self.state.lock().await;
            if json != state.last_state {
                state.streams.retain(Option::is_some);
                for stream in &mut state.streams {
                    if let Some(mut_stream) = stream.as_mut() {
                        if mut_stream.write_all(json.as_bytes()).is_err() {
                            stream.take();
                        }
                    }
                }
                state.last_state = json;
            }
        }
        Ok(())
    }

    pub async fn close(&mut self) -> CritResult<()> {
        if let Some(listener) = self.listener.take() {
            listener.abort();
            fs::remove_file(&self.socket_path).await.ok();
        }
        info!("Closed state socket");
        Ok(())
    }
}
