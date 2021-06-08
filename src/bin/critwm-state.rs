#[macro_use]
extern crate log;

use clap::{App, Arg, ArgMatches};
use critwm::{error::CritResult, socket::SOCKET_PATH};
use serde_json::Value;
use std::{path::PathBuf, process};
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    net::UnixStream,
};

async fn start(matches: ArgMatches<'_>) -> CritResult<()> {
    let stream = UnixStream::connect(PathBuf::from(SOCKET_PATH)).await?;
    let mut reader = BufReader::new(stream).lines();

    let tail = matches.is_present("tail");
    let monitor = matches
        .value_of("monitor")
        .map(|monitor_index| monitor_index.parse::<usize>().unwrap());
    while let Some(line) = reader.next_line().await? {
        let value: Value = serde_json::from_str(&line)?;
        println!(
            "{}",
            match monitor {
                Some(monitor_index) => &value["monitors"][monitor_index],
                None => &value,
            }
        );
        if !tail {
            break;
        }
    }
    Ok(())
}

#[tokio::main]
async fn main() {
    let matches = App::new("critwm-state")
        .about("Query the state of critwm.")
        .arg(
            Arg::with_name("monitor")
                .value_name("MONITOR INDEX")
                .takes_value(true)
                .short("m")
                .long("monitor")
                .help("Specify which monitor to query"),
        )
        .arg(
            Arg::with_name("tail")
                .short("t")
                .long("tail")
                .help("Continously output"),
        )
        .get_matches();
    match start(matches).await {
        Ok(_) => {
            process::exit(0);
        }
        Err(e) => {
            error!("{:?}", e);
            process::exit(1)
        }
    }
}
