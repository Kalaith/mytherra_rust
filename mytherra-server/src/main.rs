//! Mytherra authority server (GDD 7).
//!
//! Owns the one shared, persistent world and advances it on the server's own
//! tick schedule (§7.1) — never a player button. Each poll returns a player the
//! projection their Standing reveals (§7.7); the server is the sole simulation
//! authority (§7.1, §5.8).
//!
//! This is the M1 scaffold: a single in-memory guest player and read-only
//! endpoints. `POST /action` (which needs the pure command-apply extracted into
//! `mytherra-core`), the since-cursor event delta, auth, and DB persistence are
//! the phases that follow.

use std::sync::Arc;
use std::time::Duration;

use axum::http::StatusCode;
use axum::{
    extract::State,
    routing::{get, post},
    Json, Router,
};
use mytherra_core::capability::Tier;
use mytherra_core::command::{apply, authorize, ActionReport, PlayerAction};
use mytherra_core::data::GameData;
use mytherra_core::sim::tick_world;
use mytherra_core::world::{PlayerState, WorldState};
use mytherra_protocol::{project, PlayerView, Standing, WorldView};
use serde::Serialize;
use tokio::sync::Mutex;

/// Where the server listens. M1 dev default; a later phase moves this to config.
const LISTEN_ADDR: &str = "127.0.0.1:8791";

/// The authoritative shared world plus the (single, for M1) player.
struct Authority {
    data: GameData,
    world: WorldState,
    player: PlayerState,
    standing: Standing,
}

impl Authority {
    fn load() -> Self {
        let data = GameData::load().expect("Mytherra content failed to load");
        let world = WorldState::new(&data);
        let player = PlayerState::new(&data.config);
        let standing = standing_for(&data, &player);
        Self {
            data,
            world,
            player,
            standing,
        }
    }

    /// Advance the world one tick and refresh the player's Standing (GDD 5.9).
    fn tick(&mut self) {
        tick_world(&mut self.world, &mut self.player, &self.data);
        self.standing = standing_for(&self.data, &self.player);
    }
}

/// The Standing a player of the current level holds, per the data-driven
/// thresholds (GDD 5.9).
fn standing_for(data: &GameData, player: &PlayerState) -> Standing {
    let tier = Tier::for_level(player.level, &data.balance.player.tier_unlock_levels);
    data.tiers.standing(tier)
}

type Shared = Arc<Mutex<Authority>>;

/// The per-player payload a client polls: its Standing-filtered world view and
/// its own private player view (§7.7).
#[derive(Serialize)]
struct ClientView {
    world: WorldView,
    player: PlayerView,
}

#[tokio::main]
async fn main() {
    let shared: Shared = Arc::new(Mutex::new(Authority::load()));

    // The world advances on the server's own schedule (GDD 7.1).
    let seconds = shared.lock().await.data.config.seconds_per_tick.max(1.0);
    let ticker = shared.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs_f32(seconds));
        interval.tick().await; // the first tick fires immediately; skip it.
        loop {
            interval.tick().await;
            ticker.lock().await.tick();
        }
    });

    let app = Router::new()
        .route("/health", get(health))
        .route("/view", get(view))
        .route("/action", post(action))
        .with_state(shared);

    let listener = tokio::net::TcpListener::bind(LISTEN_ADDR)
        .await
        .expect("bind listen address");
    println!("mytherra-server listening on http://{LISTEN_ADDR}");
    axum::serve(listener, app).await.expect("server error");
}

async fn health() -> &'static str {
    "ok"
}

/// The guest player's current view of the world (§7.7). One player for M1; a
/// later phase keys this by an authenticated account.
async fn view(State(shared): State<Shared>) -> Json<ClientView> {
    let authority = shared.lock().await;
    let (world, player) = project(
        &authority.world,
        &authority.player,
        &authority.standing,
        &authority.data,
    );
    Json(ClientView { world, player })
}

/// Submit an authoritative command (§7.1, §7.7). The server checks the guest's
/// Standing, applies the shared core `apply` on success, and returns the
/// feedback; an action beyond the player's Standing is a 403. One player for M1.
async fn action(
    State(shared): State<Shared>,
    Json(command): Json<PlayerAction>,
) -> Result<Json<ActionReport>, StatusCode> {
    let mut authority = shared.lock().await;
    if !authorize(&authority.standing, &authority.world, &command) {
        return Err(StatusCode::FORBIDDEN);
    }
    let Authority {
        data,
        world,
        player,
        standing,
    } = &mut *authority;
    let report = apply(world, player, data, &command);
    *standing = standing_for(data, player);
    Ok(Json(report))
}
