//! Mytherra authority server (GDD 7).
//!
//! Owns the one shared, persistent world and advances it on the server's own
//! tick schedule (§7.1) — never a player button. Each poll returns a player the
//! projection their Standing reveals (§7.7); the server is the sole simulation
//! authority (§7.1, §5.8).
//!
//! M1 serves a single in-memory guest: `GET /view` (Standing-filtered
//! projection), `GET /events?since=` (the change delta, §7.4), and `POST
//! /action` (authorize + apply). Multiple authenticated accounts, nudge caps
//! (§7.5), and DB persistence are the phases that follow.

use std::sync::Arc;
use std::time::Duration;

use axum::http::StatusCode;
use axum::{
    extract::{Query, State},
    routing::{get, post},
    Json, Router,
};
use mytherra_core::capability::Tier;
use mytherra_core::command::{apply, authorize, ActionReport, PlayerAction};
use mytherra_core::data::GameData;
use mytherra_core::sim::tick_world;
use mytherra_core::world::{PlayerState, WorldState};
use mytherra_protocol::{project, ClientView, EventsDelta, Standing};
use serde::Deserialize;
use tokio::sync::Mutex;
use tower_http::cors::CorsLayer;

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

/// `GET /events?since=<cursor>` — a returning player asks what changed since
/// they last acknowledged (GDD 7.4). Omitting `since` yields the retained tail.
/// (`ClientView` and `EventsDelta` responses are shared wire types in
/// `mytherra_protocol`.)
#[derive(Deserialize)]
struct EventsQuery {
    #[serde(default)]
    since: u64,
}

#[tokio::main]
async fn main() {
    let shared: Shared = Arc::new(Mutex::new(Authority::load()));

    // Listen address and tick cadence both come from config (GDD 7.6), not
    // source constants, so the deployment address lives in one place.
    let listen_addr = shared.lock().await.data.config.server_listen_addr.clone();

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
        .route("/events", get(events))
        .route("/action", post(action))
        // The browser client is served from a different origin than this port, so
        // it needs permissive CORS to call the API. M1 dev default; a later phase
        // narrows this to the deployed page's origin (§7.6).
        .layer(CorsLayer::permissive())
        .with_state(shared);

    let listener = tokio::net::TcpListener::bind(&listen_addr)
        .await
        .expect("bind listen address");
    println!("mytherra-server listening on http://{listen_addr}");
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

/// The chronicle events pushed since the client's cursor, plus the new cursor
/// (GDD 7.4) — so a returning player sees exactly what changed, including other
/// deities' visible acts, instead of a blind refetch.
async fn events(
    State(shared): State<Shared>,
    Query(query): Query<EventsQuery>,
) -> Json<EventsDelta> {
    let authority = shared.lock().await;
    let (events, cursor) = authority.world.chronicle.since(query.since);
    Json(EventsDelta {
        events: events.into_iter().cloned().collect(),
        cursor,
    })
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
