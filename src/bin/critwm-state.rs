#[macro_use]
extern crate log;

use clap::{App, AppSettings, Arg, ArgGroup, ArgMatches, SubCommand};
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
    if let Some(line) = reader.next_line().await? {
        let value: Value = serde_json::from_str(&line)?;
        if matches.subcommand_matches("raw").is_some() {
            println!("{}", value);
        } else if let Some(matches) = matches.subcommand_matches("query") {
            let monitor = matches.value_of("monitor");
            if let Some(monitor) = monitor {
                let monitor: usize = monitor.parse().unwrap();
                if matches.is_present("workspace") {
                    println!(
                        "{}",
                        value["monitors"].as_array().unwrap()[monitor]["current_workspace"]
                    );
                } else if matches.is_present("geometry") {
                    println!(
                        "{}",
                        value["monitors"].as_array().unwrap()[monitor]["geometry"]
                    );
                } else if matches.is_present("layout") {
                    println!(
                        "{}",
                        value["monitors"].as_array().unwrap()[monitor]["layout"]["symbol"]
                            .as_str()
                            .unwrap()
                    );
                } else if matches.is_present("bar") {
                    println!(
                        "{}",
                        value["monitors"].as_array().unwrap()[monitor]["bar_status"]
                    );
                }
            } else {
                unreachable!();
            }
        } else if let Some(matches) = matches.subcommand_matches("current") {
            if matches.is_present("client") {
                println!("{}", value["current_client"]);
            } else if matches.is_present("monitor") {
                println!("{}", value["current_monitor"]);
            }
        } else if let Some(matches) = matches.subcommand_matches("list") {
            if matches.is_present("clients") {
                println!("{}", value["clients"]);
            } else if matches.is_present("monitors") {
                println!("{}", value["monitors"]);
            } else if matches.is_present("layouts") {
                println!("{}", value["layouts"]);
            }
        }
    }
    Ok(())
}

#[tokio::main]
async fn main() {
    let matches = App::new("critwm-state")
        .about("Query the state of critwm.")
        .setting(AppSettings::ArgRequiredElseHelp)
        .setting(AppSettings::VersionlessSubcommands)
        .subcommand(SubCommand::with_name("raw").about("Output raw JSON received from socket."))
        .subcommand(
            SubCommand::with_name("query")
                .about("Query information about specified monitor.")
                .arg(
                    Arg::with_name("monitor")
                        .required(true)
                        .value_name("MONITOR INDEX")
                        .takes_value(true)
                        .short("m")
                        .long("monitor")
                        .help("Specify which monitor to query"),
                )
                .arg(
                    Arg::with_name("workspace")
                        .short("w")
                        .long("workspace")
                        .help("Query selected workspace of monitor"),
                )
                .arg(
                    Arg::with_name("geometry")
                        .short("g")
                        .long("geometry")
                        .help("Query geometry of monitor"),
                )
                .arg(
                    Arg::with_name("layout")
                        .short("l")
                        .long("layout")
                        .help("Query symbol of current layout of monitor"),
                )
                .arg(
                    Arg::with_name("bar")
                        .short("b")
                        .long("bar")
                        .help("Query visibility of bar of monitor"),
                )
                .group(ArgGroup::with_name("query").required(true).args(&[
                    "workspace",
                    "geometry",
                    "layout",
                    "bar",
                ])),
        )
        .subcommand(
            SubCommand::with_name("current")
                .about("Query index of current client or monitor.")
                .arg(
                    Arg::with_name("client")
                        .short("c")
                        .long("client")
                        .help("Query index of current client"),
                )
                .arg(
                    Arg::with_name("monitor")
                        .short("m")
                        .long("monitor")
                        .help("Query index of current monitor"),
                )
                .group(
                    ArgGroup::with_name("current")
                        .required(true)
                        .args(&["client", "monitor"]),
                ),
        )
        .subcommand(
            SubCommand::with_name("list")
                .about("List all clients, layouts or monitors as JSON.")
                .arg(
                    Arg::with_name("clients")
                        .short("c")
                        .long("clients")
                        .help("List clients"),
                )
                .arg(
                    Arg::with_name("monitors")
                        .short("m")
                        .long("monitors")
                        .help("List monitors"),
                )
                .arg(
                    Arg::with_name("layouts")
                        .short("l")
                        .long("layouts")
                        .help("List layouts"),
                )
                .group(
                    ArgGroup::with_name("list")
                        .required(true)
                        .args(&["clients", "monitors", "layouts"]),
                ),
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
