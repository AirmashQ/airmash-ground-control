#![feature(futures_api, await_macro, async_await)]

#[macro_use]
mod logging;

mod commands;
mod map;
mod server;
mod types;
mod wing;

use airmash_client::Client;
use airmash_protocol as protocol;

use std::process;
use url::Url;

/// Default ground control name
static DEFAULT_GROUND_CTRL_NAME: &'static str = "GROUND-CTRL";

/// Maximum number of wingmen per player
const DEFAULT_MAX_WINGMEN: u8 = 5;

/// Arguments provided from the command line
/// used for spawning servers
struct ServerArgs {
    /// URL of the client we're talking to
    url: Url,
    /// The maximum number of wingmen for the
    /// eventual server
    max_wingmen: u8,
    /// True if we should announce ourselves
    /// to newly joining players, else false
    /// to stay quiet
    announce: bool,
    /// The ground controller's name
    ctrl_name: String,
}

/// Command-line argument parsing. Returns the arguments
/// to start servers, or a message describing an error.
fn parse_args() -> Result<Vec<ServerArgs>, String> {
    use clap::{crate_version, App, Arg};
    let default_wingmen_str = DEFAULT_MAX_WINGMEN.to_string();
    let args = App::new("AIRMASH Ground Control")
        .about("Client for dispatching bots")
        .version(crate_version!())
        .arg(
            Arg::with_name("servers")
                .help("The AIRMASH websocket servers to interface")
                .takes_value(true)
                .multiple(true)
                .required(true),
        )
        .arg(
            Arg::with_name("max_wingmen")
                .long("max-wingmen")
                .help("The maximum number of wingmen per server")
                .default_value(&default_wingmen_str)
                .takes_value(true)
                .required(false),
        )
        .arg(
            Arg::with_name("no_announce")
                .long("no-announce")
                .help("When a new player joins, do not announce yourself")
                .default_value("true")
                .required(false)
                .takes_value(false),
        )
        .arg(
            Arg::with_name("ctrl_name")
                .long("name")
                .help("Ground controller's name")
                .default_value(DEFAULT_GROUND_CTRL_NAME)
                .required(false),
        )
        .get_matches();

    let servers: Result<Vec<Url>, _> = args
        .values_of("servers")
        .map(|servers| servers.map(Url::parse))
        .unwrap() // clap enforces required value
        .collect();

    let servers = match servers {
        Ok(servers) => servers,
        Err(err) => return Err(format!("{}", err)),
    };

    let max_wingmen = args
        .value_of("max_wingmen")
        .and_then(|max| max.parse().ok())
        .unwrap_or(DEFAULT_MAX_WINGMEN);

    let announce = !args.is_present("no_announce");
    let ctrl_name = args
        .value_of("ctrl_name")
        .unwrap_or(DEFAULT_GROUND_CTRL_NAME)
        .to_owned();

    Ok(servers
        .into_iter()
        .map(|url| ServerArgs {
            url,
            max_wingmen,
            announce,
            ctrl_name: ctrl_name.clone(),
        })
        .collect())
}

/// Spawns tasks that communicate with the servers
async fn start_servers(args: Vec<ServerArgs>) {
    for arg in args {
        let mut client = match await!(Client::new_insecure(arg.url.clone())) {
            Ok(client) => client,
            Err(err) => {
                log::error!("client connection error: {}", err);
                return;
            }
        };

        if let Err(err) = await!(client.send(protocol::client::Login {
            flag: "UN".to_owned(),
            name: arg.ctrl_name,
            session: "none".to_owned(),
            horizon_x: 3000,
            horizon_y: 3000,
            protocol: 5,
        })) {
            log::error!("client login error {}", err);
            return;
        } else if let Err(err) = await!(client.wait_for_login()) {
            log::error!("wait for login error {}", err);
            return;
        }

        // Force ground control to spectate
        if let Err(err) = await!(client.send(protocol::client::Command {
            com: "spectate".to_owned(),
            data: "-3".to_owned(),
        })) {
            log::error!("force spectate error {}", err);
            return;
        }

        log::info!("Starting ground control on server {}", arg.url);
        let server = server::Server::new(arg.url, client, arg.max_wingmen, arg.announce);
        tokio::spawn_async(server.run());
    }
}

fn main() {
    env_logger::init();

    let args = match parse_args() {
        Err(err) => {
            log::error!("{}", err);
            process::exit(1);
        }
        Ok(args) => args,
    };

    tokio::run_async(start_servers(args));
}
