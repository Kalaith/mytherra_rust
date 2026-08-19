//! Talking to the authority server (GDD 7.4).
//!
//! The cross-platform transport lives in the toolkit's optional `net` feature.
//! It uses `quad-net` to share one poll-based API across native (a background
//! thread) and WASM (macroquad's own `sapp-jsutils` JS interop — *not*
//! wasm-bindgen, so it coexists with macroquad's WebGL build).
//!
//! This module owns only Mytherra's endpoint vocabulary and session headers.
//! Each call returns a toolkit [`Pending<T>`] the caller polls once per frame —
//! never blocking the game loop. It mints a guest session (`/session`), polls
//! `/view` for its Standing-filtered projection, listens to `/events` for the
//! world's stirrings, and submits every command through `/action`.
//!
//! WASM runtime caveat: quad-net's browser side calls JS functions
//! (`http_make_request`/`http_try_recv`) that its companion JS shim provides.
//! That shim (`quad-net.js`) is deployed with the WebGL build by the shared
//! RustGames publisher; a headless build cannot verify fetch against a live
//! server in a real browser.

use macroquad_toolkit::net::HttpClient;
use mytherra_core::command::ActionReport;
use mytherra_protocol::{ClientView, EventsDelta, PlayerAction, SessionResponse};

pub use macroquad_toolkit::net::Pending;

/// The header every request presents to identify the client's guest session
/// (GDD 7.7) — matched by the server's `PLAYER_ID_HEADER`.
const PLAYER_ID_HEADER: &str = "X-Player-Id";

/// A handle to one authority server, addressed by base URL (e.g.
/// `http://127.0.0.1:8791`). It carries the session and account headers needed
/// by Mytherra's endpoint vocabulary and delegates request transport to the
/// shared toolkit.
pub struct ServerClient {
    api: HttpClient,
}

impl ServerClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            api: HttpClient::new(base_url),
        }
    }

    /// Adopt the guest session id the server minted, so every later request
    /// carries it (GDD 7.7).
    pub fn set_player_id(&mut self, player_id: String) {
        self.api.set_header(PLAYER_ID_HEADER, player_id);
    }

    /// Carry a WebHatchery account token on the next `/session` and `/link`
    /// (GDD 7.3), or clear it with `None`. Set from a persisted token on startup
    /// so the client resumes its account deity rather than a fresh guest.
    pub fn set_token(&mut self, token: Option<String>) {
        self.api.set_bearer_token(token.as_deref());
    }

    /// `POST /session` — begin a session (§7.7). With no token this mints a
    /// fresh guest; with an account token set it resumes that account's deity
    /// across devices (GDD 7.3). Feed the returned id to
    /// [`set_player_id`](ServerClient::set_player_id).
    pub fn create_session(&self) -> Pending<SessionResponse> {
        self.api.post("/session")
    }

    /// `POST /link` — bind the deity the client is currently playing to the
    /// WebHatchery account its token authenticates (GDD 7.3). The reply names
    /// the deity to carry forward.
    pub fn link(&self) -> Pending<SessionResponse> {
        self.api.post("/link")
    }

    /// `GET /view` — the player's Standing-filtered view of the world (§7.7).
    pub fn fetch_view(&self) -> Pending<ClientView> {
        self.api.get("/view")
    }

    /// `GET /events?since=` — the chronicle delta since `cursor` (§7.4).
    pub fn fetch_events(&self, since: u64) -> Pending<EventsDelta> {
        self.api.get(&format!("/events?since={since}"))
    }

    /// `POST /action` — submit an authoritative command, returning its
    /// feedback. A command beyond the player's Standing comes back as an error
    /// (§7.7).
    pub fn submit_action(&self, action: &PlayerAction) -> Pending<ActionReport> {
        self.api.post_json("/action", action)
    }
}

#[cfg(test)]
mod tests;
