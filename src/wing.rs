//! Wingman, a component that attacks a player
//!
//! Once spawned, a wingman can only be shutdown by setting
//! an atomic flag that's provided on startup.
//!
//! Right now, the wingman simply follows and shoots a player.
//! It's really dumb...

use airmash_client::{Client, ClientBase};
use airmash_protocol as protocol;

use pathfinding::prelude::astar;
use std::sync::{atomic, Arc};
use std::time;
use url::Url;

use crate::types::MapPosition;

const MIN_FIRE_DIST: f32 = 500.0;

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
        mut client: ClientBase,
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

            // Fire when close to the target.
            let mut fire = if (pos - client.world.get_me().pos).length().inner() < MIN_FIRE_DIST {
                true
            } else {
                false
            };

            if time::Instant::now() - prev > time::Duration::from_millis(500) {
                await!(client.press_key(protocol::KeyCode::Up))?;
                prev = time::Instant::now();
            }

            let src_map_pos: MapPosition = client.world.get_me().pos.into();
            let mut dst_map_pos: MapPosition = pos.into();
            let mut pathfinding_enabled = true;

            // astar will search the entire map if the destination is occupied so pick
            // a free adjacent position.
            if dst_map_pos.is_occupied() {
                if let Some(p) = dst_map_pos.adjacent_positions().next() {
                    dst_map_pos = p;
                } else {
                    // Couldn't find an unoccupied position on the map, so disable
                    // pathfinding so the cpu doesn't spike.
                    pathfinding_enabled = false;
                }
            }

            if pathfinding_enabled {
                // Only use pathfinding if there's an obstacle (mountain) between us and
                // the target.
                if let Some(ob_map_pos) = src_map_pos.obstacle_between(dst_map_pos) {
                    // Don't fire if don't have line-of-sight.
                    fire = false;

                    // Make sure the obstacle is near, otherwise we can just head in its
                    // direction.
                    // Distance is in map units (1 = 64 world units), so this is taking us within
                    // 960 of the obstacle.
                    if ob_map_pos.distance(src_map_pos) < 16 {
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
                }
            }

            await!(client.point_at(pos))?;

            if fire {
                await!(client.press_key(protocol::KeyCode::Fire))?;
            } else {
                await!(client.release_key(protocol::KeyCode::Fire))?;
            }

            let delay_time = u64::from((client.world.ping * 2).min(1000).max(10));
            await!(client.wait(time::Duration::from_millis(delay_time)))?;
        }

        await!(client.release_key(protocol::KeyCode::Up))
    }
}
