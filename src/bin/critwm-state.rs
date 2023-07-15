#[macro_use]
extern crate log;

use clap::Parser;
use critwm::{error::CritResult, socket::SOCKET_PATH};
use serde_json::Value;
use std::{path::PathBuf, process};
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    net::UnixStream,
};

async fn start(args: Args) -> CritResult<()> {
    let stream = UnixStream::connect(PathBuf::from(SOCKET_PATH)).await?;
    let mut reader = BufReader::new(stream).lines();

    let tail = args.tail;
    let monitor = args.monitor;
    while let Some(line) = reader.next_line().await? {
        let value: Value = serde_json::from_str(&line)?;
        let state = match monitor {
            Some(monitor_index) => &value["monitors"][monitor_index],
            None => &value,
        };
        println!("{state}");
        if !tail {
            break;
        }
    }
    Ok(())
}

/// Query the state of critwm
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Specify which monitor to query
    #[arg(short, long)]
    monitor: Option<usize>,

    ///Continuously output
    #[arg(short, long)]
    tail: bool,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    match start(args).await {
        Ok(_) => {
            process::exit(0);
        }
        Err(e) => {
            error!("{:?}", e);
            process::exit(1)
        }
    }
}
