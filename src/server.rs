//! The server that manages the bots
//!
//! There is one `Server` instance connected to an
//! actual game server. The `Server` can be interacted with
//! through the ground control bot that spectates the game.
//!
//! `Server` is the main component that handles client requests
//! and manages bots.

use crate::commands;
use crate::commands::ControlTower;
use crate::wing;

use airmash_client::{ClientBase, ClientEvent};
use airmash_protocol as protocol;

use std::collections::HashMap;
use std::time;

use url::Url;

/// A connected server that can drop into an event
/// loop, handling client messages
pub struct Server {
    /// Client connection
    client: ClientBase,
    /// The control tower that handles wingmen commands and
    /// responses
    tower: ControlTower,
    /// The server that we're talking to
    url: Url,
    /// Players to associated wingmen control flags
    wingmen: HashMap<protocol::Player, Vec<wing::Flag>>,
    /// True to announce ourselves to new players, else false
    announce: bool,
}

impl Server {
    /// Create a new server connected to the specified URL using the fully-initialized
    /// client. The maximum number of wingment per player are capped at `max_wingmen`.
    ///
    /// If the server should announce itself to new players, set `announce` to `true`.
    /// Announcing mostly means that we will tell them about the help command.
    pub fn new(url: Url, client: ClientBase, max_wingmen: u8, announce: bool) -> Self {
        Server {
            client,
            tower: ControlTower::new(max_wingmen),
            url,
            wingmen: HashMap::new(),
            announce,
        }
    }

    fn player_name(&self, id: protocol::Player) -> Option<String> {
        self.client
            .world
            .players
            .get(&id.0)
            .map(|player| player.name.clone())
    }

    /// Spawn the number of wingmen specified by wings that track the named player
    async fn spawn_wingmen(&mut self, id: protocol::Player, wings: u8) {
        let name = match self.player_name(id) {
            None => {
                log::warn!("spawn_wingmen called with unknown player ID {}", id.0);
                return;
            }
            Some(name) => name,
        };

        let mut flags = Vec::new();
        for _ in 0..wings {
            let flag = wing::Flag::default();
            tokio::spawn_async(wing::Wingman::spawn(
                self.url.clone(),
                name.clone(),
                flag.clone(),
            ));
            flags.push(flag);
        }
        self.wingmen.insert(id, flags);
    }

    /// Remove the wingmen following the named player
    async fn clear_wingmen(&mut self, id: protocol::Player) {
        if let Some(flags) = self.wingmen.remove(&id) {
            log::debug!("clear_wingmen dropping {} wings", flags.len());
        }
    }

    /// Handle a user's message, possibly spawning or clearing bots
    async fn handle_message(&mut self, id: protocol::Player, message: String) {
        let name = match self.player_name(id) {
            None => {
                log::warn!("handle_message called with unknown player ID {}", id.0);
                return;
            }
            Some(name) => name,
        };

        let wingmen_count = self
            .wingmen
            .get(&id)
            .as_ref()
            .map(|wings| wings.len() as u8)
            .unwrap_or(0u8);
        let cmd = commands::Command::new(&message, &name, wingmen_count);
        match self.tower.parse_command(cmd) {
            // Not for us; do nothing
            None => (),
            // Bad command sent from the user
            Some(Err(err)) => warn_on_err!(await!(self.client.chat(format!("{}", err)))),
            // Good command; take some action
            Some(Ok(resp)) => {
                match resp.kind() {
                    Some(commands::ResponseKind::SetWings { wings, .. }) => {
                        await!(self.spawn_wingmen(id, wings))
                    }
                    Some(commands::ResponseKind::ClearWings) => await!(self.clear_wingmen(id)),
                    None => (),
                };
                // Send reply
                let msgs = resp.msg();
                for msg in msgs {
                    warn_on_err!(await!(self.client.chat(msg)));
                    warn_on_err!(await!(self.client.wait(time::Duration::from_millis(1000))));
                }
            }
        }
    }

    /// Handle a packet from the connected server
    async fn handle_packet(&mut self, packet: protocol::ServerPacket) {
        match packet {
            protocol::ServerPacket::ChatPublic(chat_public) => {
                await!(self.handle_message(chat_public.id, chat_public.text))
            }
            protocol::ServerPacket::PlayerLeave(player_leave) => {
                await!(self.clear_wingmen(player_leave.id))
            }
            protocol::ServerPacket::PlayerNew(ref player_new) if self.announce => {
                let msg = format!(
                    "Ground Control, standing by for {}! Use {} for help.",
                    player_new.name,
                    commands::command::HELP
                );
                warn_on_err!(await!(self.client.chat(msg)));
            }
            _ => (),
        };
    }

    /// Run the server event loop
    pub async fn run(mut self) {
        loop {
            match await!(self.client.next()) {
                Err(err) => {
                    log::error!("error awaiting client's next message {}", err);
                    return;
                }
                Ok(Some(ClientEvent::Packet(packet))) => await!(self.handle_packet(packet)),
                _ => continue,
            }
        }
    }
}
