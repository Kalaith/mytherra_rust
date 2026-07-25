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

mod auth;
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
use config::AuthConfig;
use mytherra_core::capability::Tier;
use mytherra_core::command::{apply, authorize, ActionReport, PlayerAction};
use mytherra_core::data::GameData;
use mytherra_core::sim::tick_shared;
use mytherra_core::world::{PlayerState, WorldState};
use mytherra_persistence::Store;
use mytherra_protocol::{
    project, project_events, ClientView, EventsDelta, LoginInfo, SessionResponse, Standing,
};
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
    /// WebHatchery account id → the index of the deity it owns (GDD 7.3). Holds
    /// one deity per linked account; a pure guest never appears here. Rebuilt
    /// from the store on boot, updated on every link.
    accounts: BTreeMap<String, usize>,
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
        let mut accounts = BTreeMap::new();
        let mut players = Vec::new();
        for (id, account_id, mut state) in store.players.load_all().await {
            // Reconcile each saved deity's unlock state with the current
            // definitions — they may have changed between server versions, and a
            // deity minted before this fix carries an empty list (GDD 7.1).
            state
                .achievements
                .sync_definitions(data.achievements.clone());
            let index = players.len();
            if let Some(account_id) = account_id {
                accounts.insert(account_id, index);
            }
            ids.insert(id, index);
            players.push(state);
        }
        let next_guest = store.players.next_guest().await;

        println!(
            "resumed world at year {}, {} deities connected across restarts ({} linked to accounts)",
            world.year,
            players.len(),
            accounts.len()
        );
        Self {
            data,
            world,
            ids,
            players,
            accounts,
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
        let mut player = PlayerState::new(&self.data.config);
        // Populate the achievement definitions so the deity can unlock them
        // server-side (GDD 7.1) and its PlayerView renders a real list.
        player
            .achievements
            .sync_definitions(self.data.achievements.clone());
        self.players.push(player);
        id
    }

    /// The player index behind a session id, or a 401 if it names no live
    /// session.
    fn index_of(&self, id: &str) -> Result<usize, StatusCode> {
        self.ids.get(id).copied().ok_or(StatusCode::UNAUTHORIZED)
    }

    /// The session id behind a player index — the reverse of `ids`. Used only on
    /// the rare link/resume paths, so a small scan is fine.
    fn id_of(&self, index: usize) -> String {
        self.ids
            .iter()
            .find(|(_, &i)| i == index)
            .map(|(id, _)| id.clone())
            .expect("every live index has a session id")
    }

    /// Bind the deity a client is currently playing (`current`) to a WebHatchery
    /// account, per the "claim, else resume" rule (GDD 7.3): if the account has
    /// no deity yet, this guest becomes its deity; if it already owns one, the
    /// client is handed that deity to resume and the just-played guest is left
    /// behind; re-linking the same deity to the same account is idempotent. A
    /// deity already bound to a *different* account is refused, so a link never
    /// orphans another account's god.
    fn link(&mut self, current: usize, account_id: &str) -> Result<LinkOutcome, LinkError> {
        if let Some(&owner) = self.accounts.get(account_id) {
            return Ok(if owner == current {
                LinkOutcome::AlreadyLinked(self.id_of(current))
            } else {
                LinkOutcome::Resumed(self.id_of(owner))
            });
        }
        if self.accounts.values().any(|&i| i == current) {
            return Err(LinkError::AlreadyLinkedElsewhere);
        }
        self.accounts.insert(account_id.to_owned(), current);
        Ok(LinkOutcome::Claimed(self.id_of(current)))
    }

    /// Resume the deity an account already owns, or mint a fresh one bound to it
    /// (GDD 7.3) — the entry path for a client that presents an account token
    /// with no guest of its own. Returns the deity id and whether it was freshly
    /// created, so the caller can persist a new row.
    fn resume_or_create_account(&mut self, account_id: &str) -> (String, bool) {
        if let Some(&index) = self.accounts.get(account_id) {
            return (self.id_of(index), false);
        }
        let id = self.new_guest();
        let index = self.ids[&id];
        self.accounts.insert(account_id.to_owned(), index);
        (id, true)
    }

    /// Every deity paired with its id, for a full write-through after a tick.
    fn roster(&self) -> Vec<(String, &PlayerState)> {
        self.ids
            .iter()
            .map(|(id, &index)| (id.clone(), &self.players[index]))
            .collect()
    }
}

/// The result of a successful link (GDD 7.3) — which deity the client should
/// carry forward, and whether the authority now needs to persist the binding.
enum LinkOutcome {
    /// The current guest deity now belongs to the account; persist the binding.
    Claimed(String),
    /// The account already owned a deity; the client switches to this one.
    Resumed(String),
    /// The current deity was already bound to this account — nothing to persist.
    AlreadyLinked(String),
}

/// Why a link was refused.
enum LinkError {
    /// The current deity is already bound to a different account.
    AlreadyLinkedElsewhere,
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

/// The bearer token from an `Authorization: Bearer <token>` header, if present
/// and well-formed — the WebHatchery account token a client offers to link or
/// resume a deity (GDD 7.3). Absent header ⇒ `None` (a plain guest request).
fn bearer_token(headers: &HeaderMap) -> Option<String> {
    let value = headers.get("authorization")?.to_str().ok()?;
    value
        .strip_prefix("Bearer ")
        .or_else(|| value.strip_prefix("bearer "))
        .map(|token| token.trim().to_owned())
        .filter(|token| !token.is_empty())
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
    /// Shared WebHatchery auth config (secret + login URL) for account linking
    /// (GDD 7.3). `Arc` so cloning `App` into every handler stays cheap.
    auth: Arc<AuthConfig>,
}

impl App {
    /// Verify a request's bearer token to the account it authenticates, mapping
    /// each refusal to its HTTP status: no token or a bad one ⇒ 401, a valid but
    /// *guest* token ⇒ 400 (linking needs a real account, GDD 7.3).
    fn account_of(&self, headers: &HeaderMap) -> Result<auth::Account, StatusCode> {
        let token = bearer_token(headers).ok_or(StatusCode::UNAUTHORIZED)?;
        auth::verify_account(&token, &self.auth.jwt_secret).map_err(|err| match err {
            auth::AuthError::Guest => StatusCode::BAD_REQUEST,
            auth::AuthError::Invalid => StatusCode::UNAUTHORIZED,
        })
    }
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
        auth: Arc::new(config::auth_config()),
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
        .route("/login-info", get(login_info))
        .route("/session", post(session))
        .route("/link", post(link))
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

/// Where to send a player to sign in to WebHatchery, so the client can offer to
/// link a guest deity for cross-device continuity (GDD 7.3).
async fn login_info(State(app): State<App>) -> Json<LoginInfo> {
    Json(LoginInfo {
        login_url: app.auth.login_url.clone(),
    })
}

/// Begin a session. A plain request mints a fresh guest deity (GDD 7.7); a
/// request bearing a WebHatchery account token instead resumes the deity that
/// account owns — or mints one bound to it on first sign-in — so the same god
/// follows the player across devices (GDD 7.3). Either way the client presents
/// the returned id as `X-Player-Id` on every later request. New rows and the
/// bumped guest counter persist immediately so the session survives a restart.
async fn session(
    State(app): State<App>,
    headers: HeaderMap,
) -> Result<Json<SessionResponse>, StatusCode> {
    if bearer_token(&headers).is_some() {
        let account = app.account_of(&headers)?;
        let mut authority = app.authority.lock().await;
        let (player_id, created) = authority.resume_or_create_account(&account.id);
        if created {
            let index = authority.ids[&player_id];
            app.store
                .players
                .save(&player_id, &authority.players[index])
                .await;
            app.store.players.set_next_guest(authority.next_guest).await;
            app.store.players.set_account(&player_id, &account.id).await;
        }
        return Ok(Json(SessionResponse {
            player_id,
            linked: true,
        }));
    }

    let mut authority = app.authority.lock().await;
    let player_id = authority.new_guest();
    let index = authority.ids[&player_id];
    app.store
        .players
        .save(&player_id, &authority.players[index])
        .await;
    app.store.players.set_next_guest(authority.next_guest).await;
    Ok(Json(SessionResponse {
        player_id,
        linked: false,
    }))
}

/// Link the deity a client is currently playing (its `X-Player-Id`) to the
/// WebHatchery account its bearer token authenticates (GDD 7.3). Returns the
/// deity the client should carry forward, `linked: true`: normally the same
/// deity, now the account's; or, if the account already owned one, that deity to
/// resume (the just-played guest is left behind, per "claim, else resume"). A
/// guest token is a 400, an unknown session a 401, and a deity already bound to
/// a different account a 409.
async fn link(
    State(app): State<App>,
    headers: HeaderMap,
) -> Result<Json<SessionResponse>, StatusCode> {
    let account = app.account_of(&headers)?;
    let current = player_id_of(&headers)?;
    let mut authority = app.authority.lock().await;
    let index = authority.index_of(&current)?;
    match authority.link(index, &account.id) {
        Ok(LinkOutcome::Claimed(player_id)) => {
            app.store.players.set_account(&player_id, &account.id).await;
            Ok(Json(SessionResponse {
                player_id,
                linked: true,
            }))
        }
        Ok(LinkOutcome::Resumed(player_id) | LinkOutcome::AlreadyLinked(player_id)) => {
            Ok(Json(SessionResponse {
                player_id,
                linked: true,
            }))
        }
        Err(LinkError::AlreadyLinkedElsewhere) => Err(StatusCode::CONFLICT),
    }
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
    let index = authority.index_of(&player_id_of(&headers)?)?;
    let standing = standing_for(&authority.data, &authority.players[index]);
    // Filter the delta by the requesting deity's Standing (§7.7), but return the
    // unfiltered cursor so skipped events are never re-served on the next poll.
    let (events, cursor) = authority.world.chronicle.since(query.since);
    Ok(Json(EventsDelta {
        events: project_events(events, &standing),
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
