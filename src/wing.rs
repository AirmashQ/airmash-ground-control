//! Wingman, a component that attacks a player
//!
//! Once spawned, a wingman can only be shutdown by setting
//! an atomic flag that's provided on startup.
//!
//! Right now, the wingman simply follows and shoots a player.
//! It's really dumb...

use airmash_client::Client;
use airmash_protocol as protocol;

use pathfinding::prelude::astar;
use std::sync::{atomic, Arc};
use std::time;
use url::Url;

use crate::types::MapPosition;

/// Flag used to shutdown a wingman's event loop
#[derive(Clone)]
pub struct Flag {
    inner: Arc<atomic::AtomicBool>,
}

impl Default for Flag {
    fn default() -> Self {
        Flag {
            inner: Arc::new(atomic::ATOMIC_BOOL_INIT),
        }
    }
}

impl Flag {
    fn read(&self) -> bool {
        self.inner.load(atomic::Ordering::SeqCst)
    }
}

impl Drop for Flag {
    fn drop(&mut self) {
        self.inner.store(true, atomic::Ordering::SeqCst);
    }
}

pub struct Wingman;

impl Wingman {
    /// Spawn a wingman that connects to the associated URL and follows the target
    ///
    /// When the shutdown flag goes high, the wingman shuts down.
    ///
    /// We need to use the name of a target, not an ID, because the IDs for players
    /// seem to vary across clients.
    pub async fn spawn(url: Url, target: String, shutdown: Flag) {
        let mut client = match await!(Client::new_insecure(url)) {
            Err(err) => {
                log::error!("error connection wingman client {}", err);
                return;
            }
            Ok(client) => client,
        };

        if let Err(err) = await!(client.send(protocol::client::Login {
            flag: "UN".to_owned(),
            name: target.clone(),
            session: "none".to_owned(),
            horizon_x: 3000,
            horizon_y: 3000,
            protocol: 5,
        })) {
            log::error!("error logging in wingman {}", err);
            return;
        }

        if let Err(err) = await!(client.wait_for_login()) {
            log::error!("error waiting for wingman login {}", err);
            return;
        }

        let id = match client.world.names.get(&target) {
            Some(x) => *x,
            None => {
                log::error!("no player with name {} in game", target);
                return;
            }
        };

        warn_on_err!(await!(Self::follow(client, id, shutdown)));
        log::debug!("shutting down wingmen on {}", target);
    }

    async fn follow(
        mut client: Client,
        player: u16,
        shutdown: Flag,
    ) -> airmash_client::ClientResult<()> {
        let mut pos;
        let mut prev = time::Instant::now();
        await!(client.press_key(protocol::KeyCode::Up))?;
        while let Some(_) = await!(client.next())? {
            if shutdown.read() {
                break;
            }

            if let Some(p) = client.world.players.get(&player) {
                pos = p.pos;
            } else {
                break;
            }
            if time::Instant::now() - prev > time::Duration::from_millis(500) {
                await!(client.press_key(protocol::KeyCode::Up))?;
                prev = time::Instant::now();
            }

            let src_map_pos: MapPosition = client.world.get_me().pos.into();
            let mut dst_map_pos: MapPosition = pos.into();

            // astar will search the entire map if the destination is occupied so pick
            // a free adjacent position.
            if dst_map_pos.is_occupied() {
                if let Some(p) = dst_map_pos.adjacent_positions().get(0) {
                    dst_map_pos = *p;
                }
            }

            // Only use pathfinding if there's an obstacle (mountain) between us and
            // the target.
            if let Some(_) = src_map_pos.obstacle_between(dst_map_pos) {
                let path_positions = astar(
                    &src_map_pos,
                    |p| p.adjacent_positions().into_iter().map(|pp| (pp, 1)),
                    |p| p.distance(dst_map_pos),
                    |p| p.x == dst_map_pos.x && p.y == dst_map_pos.y,
                );
                if let Some((positions, _)) = path_positions {
                    if let Some(p) = positions.get(1) {
                        pos = (*p).into();
                    }
                }
            }

            await!(client.point_at(pos))?;
            await!(client.press_key(protocol::KeyCode::Fire))?;
            await!(client.wait(time::Duration::from_millis(u64::from(
                (client.world.ping * 2).min(1000).max(10)
            ))))?;
        }

        await!(client.release_key(protocol::KeyCode::Up))
    }
}
