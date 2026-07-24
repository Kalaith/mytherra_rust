//! Mytherra authority server (GDD 7).
//!
//! Owns the one shared, persistent world and advances it on the server's own
//! tick schedule (§7.1) — never a player button. Each poll returns a player the
//! projection their Standing reveals (§7.7); the server is the sole simulation
//! authority (§7.1, §5.8).
//!
//! Serves many concurrent guests (M2): `POST /session` mints a guest id the
//! client then presents as `X-Player-Id`; `GET /view` (that guest's Standing-
//! filtered projection), `GET /events?since=` (the shared change delta, §7.4),
//! and `POST /action` (authorize + apply for that guest). One shared world ticks
//! once per interval; every connected deity's favor, champions, wagers, and
//! Standing are its own.
//!
//! The world and every deity persist to MySQL (the DB *is* the save, §6/§8) via
//! `mytherra-persistence`, whose two dissociated stores mirror the split here:
//! the world is the shared simulation the deities *nudge*, the player domain is
//! per-deity, and they never share a row. The authority is bootstrapped from the
//! store on startup and write-throughs after every mint, action, and tick, so a
//! restart resumes the same world rather than resetting it.

mod config;

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;

use axum::http::{HeaderMap, StatusCode};
use axum::{
    extract::{Query, State},
    routing::{get, post},
    Json, Router,
};
use mytherra_core::capability::Tier;
use mytherra_core::command::{apply, authorize, ActionReport, PlayerAction};
use mytherra_core::data::GameData;
use mytherra_core::sim::tick_shared;
use mytherra_core::world::{PlayerState, WorldState};
use mytherra_persistence::Store;
use mytherra_protocol::{project, ClientView, EventsDelta, SessionResponse, Standing};
use serde::Deserialize;
use tokio::sync::Mutex;
use tower_http::cors::CorsLayer;

/// The header a client presents to identify its guest session (GDD 7.7).
const PLAYER_ID_HEADER: &str = "x-player-id";

/// The one shared world plus every connected deity's private state. Players live
/// in a `Vec` (so the tick gets a contiguous `&mut` slice) with an id → index
/// map beside it; a deity's Standing is derived from its level on demand, never
/// stored stale.
struct Authority {
    data: GameData,
    world: WorldState,
    ids: BTreeMap<String, usize>,
    players: Vec<PlayerState>,
    /// Monotonic counter minting distinct guest ids.
    next_guest: u64,
}

impl Authority {
    /// Resume from the store, or seed a fresh world and persist it (GDD 6/8). The
    /// world and the player roster load independently — they are dissociated
    /// domains — and the id → index map is rebuilt from the roster's order.
    async fn bootstrap(store: &Store) -> Self {
        let data = GameData::load().expect("Mytherra content failed to load");

        let world = match store.world.load().await {
            Some(world) => world,
            None => {
                let world = WorldState::new(&data);
                store.world.save(&world).await;
                world
            }
        };

        let mut ids = BTreeMap::new();
        let mut players = Vec::new();
        for (id, state) in store.players.load_all().await {
            ids.insert(id, players.len());
            players.push(state);
        }
        let next_guest = store.players.next_guest().await;

        println!(
            "resumed world at year {}, {} deities connected across restarts",
            world.year,
            players.len()
        );
        Self {
            data,
            world,
            ids,
            players,
            next_guest,
        }
    }

    /// Advance the shared world one tick for every connected deity (GDD 7.1).
    /// With no one connected the world still turns; it simply has no deities to
    /// nudge it.
    fn tick(&mut self) {
        tick_shared(&mut self.world, &mut self.players, &self.data);
    }

    /// Mint a fresh guest deity and return its session id (GDD 7.7).
    fn new_guest(&mut self) -> String {
        let id = format!("guest-{}", self.next_guest);
        self.next_guest += 1;
        self.ids.insert(id.clone(), self.players.len());
        self.players.push(PlayerState::new(&self.data.config));
        id
    }

    /// The player index behind a session id, or a 401 if it names no live
    /// session.
    fn index_of(&self, id: &str) -> Result<usize, StatusCode> {
        self.ids.get(id).copied().ok_or(StatusCode::UNAUTHORIZED)
    }

    /// Every deity paired with its id, for a full write-through after a tick.
    fn roster(&self) -> Vec<(String, &PlayerState)> {
        self.ids
            .iter()
            .map(|(id, &index)| (id.clone(), &self.players[index]))
            .collect()
    }
}

/// The session id a request presents in `X-Player-Id`, or a 401 if the header is
/// missing (GDD 7.7).
fn player_id_of(headers: &HeaderMap) -> Result<String, StatusCode> {
    headers
        .get(PLAYER_ID_HEADER)
        .and_then(|value| value.to_str().ok())
        .map(|id| id.to_owned())
        .ok_or(StatusCode::UNAUTHORIZED)
}

/// The Standing a player of the current level holds, per the data-driven
/// thresholds (GDD 5.9).
fn standing_for(data: &GameData, player: &PlayerState) -> Standing {
    let tier = Tier::for_level(player.level, &data.balance.player.tier_unlock_levels);
    data.tiers.standing(tier)
}

/// Shared authority state plus the persistence handle, cloned into every handler
/// and the tick task. The `Store` is pooled and `Arc`-backed; the authority is
/// behind a `tokio::sync::Mutex` so store write-throughs can be awaited while it
/// is held (correct and simple at this scale; a later phase can move per-player
/// writes off the critical section).
#[derive(Clone)]
struct App {
    authority: Arc<Mutex<Authority>>,
    store: Store,
}

/// `GET /events?since=<cursor>` — a returning player asks what changed since
/// they last acknowledged (GDD 7.4). Omitting `since` yields the retained tail.
#[derive(Deserialize)]
struct EventsQuery {
    #[serde(default)]
    since: u64,
}

#[tokio::main]
async fn main() {
    let store = Store::connect(&config::db_config()).await;
    let authority = Authority::bootstrap(&store).await;

    // Listen address and tick cadence both come from config (GDD 7.6), not
    // source constants, so the deployment address lives in one place.
    let listen_addr = authority.data.config.server_listen_addr.clone();
    let seconds = authority.data.config.seconds_per_tick.max(1.0);

    let app = App {
        authority: Arc::new(Mutex::new(authority)),
        store,
    };

    // The world advances on the server's own schedule (GDD 7.1), persisting the
    // world and every deity each tick so a crash loses at most one interval.
    let ticker = app.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs_f32(seconds));
        interval.tick().await; // the first tick fires immediately; skip it.
        loop {
            interval.tick().await;
            let mut authority = ticker.authority.lock().await;
            authority.tick();
            ticker.store.world.save(&authority.world).await;
            ticker.store.players.save_all(&authority.roster()).await;
        }
    });

    let router = Router::new()
        .route("/health", get(health))
        .route("/session", post(session))
        .route("/view", get(view))
        .route("/events", get(events))
        .route("/action", post(action))
        // The browser client is served from a different origin than this port, so
        // it needs permissive CORS to call the API. M2 dev default; a later phase
        // narrows this to the deployed page's origin (§7.6).
        .layer(CorsLayer::permissive())
        .with_state(app);

    let listener = tokio::net::TcpListener::bind(&listen_addr)
        .await
        .expect("bind listen address");
    println!("mytherra-server listening on http://{listen_addr}");
    axum::serve(listener, router).await.expect("server error");
}

async fn health() -> &'static str {
    "ok"
}

/// Mint a fresh guest deity and hand back its session id (GDD 7.7). The client
/// presents this id as `X-Player-Id` on every later request; each guest gets its
/// own favor, champions, wagers, and Standing. The new deity and the bumped
/// guest counter persist immediately so the session survives a restart.
async fn session(State(app): State<App>) -> Json<SessionResponse> {
    let mut authority = app.authority.lock().await;
    let player_id = authority.new_guest();
    let index = authority.ids[&player_id];
    app.store
        .players
        .save(&player_id, &authority.players[index])
        .await;
    app.store.players.set_next_guest(authority.next_guest).await;
    Json(SessionResponse { player_id })
}

/// The requesting deity's own Standing-filtered view of the world (§7.7),
/// keyed by its `X-Player-Id` session.
async fn view(State(app): State<App>, headers: HeaderMap) -> Result<Json<ClientView>, StatusCode> {
    let authority = app.authority.lock().await;
    let index = authority.index_of(&player_id_of(&headers)?)?;
    let player = &authority.players[index];
    let standing = standing_for(&authority.data, player);
    let (world, player) = project(&authority.world, player, &standing, &authority.data);
    Ok(Json(ClientView { world, player }))
}

/// The chronicle events pushed since the client's cursor, plus the new cursor
/// (GDD 7.4) — the shared world's stirrings, including other deities' visible
/// acts. Requires a live session so only connected deities poll it.
async fn events(
    State(app): State<App>,
    headers: HeaderMap,
    Query(query): Query<EventsQuery>,
) -> Result<Json<EventsDelta>, StatusCode> {
    let authority = app.authority.lock().await;
    authority.index_of(&player_id_of(&headers)?)?;
    let (events, cursor) = authority.world.chronicle.since(query.since);
    Ok(Json(EventsDelta {
        events: events.into_iter().cloned().collect(),
        cursor,
    }))
}

/// Submit an authoritative command for the requesting deity (§7.1, §7.7). The
/// server checks *that deity's* Standing, applies the shared core `apply` on
/// success against its own player state, and returns the feedback; an action
/// beyond its Standing is a 403, an unknown session a 401. The mutated world and
/// that deity's state write-through before the response returns.
async fn action(
    State(app): State<App>,
    headers: HeaderMap,
    Json(command): Json<PlayerAction>,
) -> Result<Json<ActionReport>, StatusCode> {
    let mut authority = app.authority.lock().await;
    let id = player_id_of(&headers)?;
    let index = authority.index_of(&id)?;
    let standing = standing_for(&authority.data, &authority.players[index]);
    if !authorize(&standing, &authority.world, &command) {
        return Err(StatusCode::FORBIDDEN);
    }
    let report = {
        let Authority {
            data,
            world,
            players,
            ..
        } = &mut *authority;
        apply(world, &mut players[index], data, &command)
    };
    app.store.world.save(&authority.world).await;
    app.store.players.save(&id, &authority.players[index]).await;
    Ok(Json(report))
}
