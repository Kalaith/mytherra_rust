//! Talking to the authority server (GDD 7.4).
//!
//! A cross-platform, non-blocking HTTP client — the client's live link to the
//! world. It's built on `quad-net`, which shares one poll-based API across native
//! (a background thread) and WASM (macroquad's own `sapp-jsutils` JS interop —
//! *not* wasm-bindgen, so it coexists with macroquad's WebGL build).
//!
//! Each call returns a [`Pending<T>`] the caller polls once per frame — never
//! blocking the game loop. It mints a guest session (`/session`), polls `/view`
//! for its Standing-filtered projection, listens to `/events` for the world's
//! stirrings, and submits every command through `/action` (see `game/online.rs`),
//! so the server's authority is the only simulation there is.
//!
//! WASM runtime caveat: quad-net's browser side calls JS functions
//! (`http_make_request`/`http_try_recv`) that its companion JS shim provides. That
//! shim (`quad-net.js`) is deployed with the WebGL build (see the RustGames
//! publish template); the one step that still can't be checked from a headless
//! build is verifying fetch against a deployed server in a real browser.

use mytherra_core::command::ActionReport;
use mytherra_protocol::{ClientView, EventsDelta, PlayerAction, SessionResponse};
use quad_net::http_request::{Method, Request, RequestBuilder};
use serde::de::DeserializeOwned;
use std::marker::PhantomData;

/// The header every request presents to identify the client's guest session
/// (GDD 7.7) — matched by the server's `PLAYER_ID_HEADER`.
const PLAYER_ID_HEADER: &str = "X-Player-Id";

/// A handle to one authority server, addressed by base URL (e.g.
/// `http://127.0.0.1:8791`). Carries the guest session id once
/// [`create_session`](ServerClient::create_session) has returned one, and
/// presents it on every subsequent request. If a WebHatchery account token has
/// been set (GDD 7.3), it also rides on the session/link requests so the server
/// resumes — or binds — this player's account deity.
pub struct ServerClient {
    base_url: String,
    player_id: Option<String>,
    /// A verified WebHatchery account token, once the player has signed in
    /// (GDD 7.3). Presented on `/session` (to resume the account's deity) and
    /// `/link` (to claim the current deity for the account).
    token: Option<String>,
}

/// A request in flight. Poll it each frame with [`poll`](Pending::poll): `None`
/// while pending, `Some` once the response (or an error) has arrived.
pub struct Pending<T> {
    request: Request,
    /// Seconds this request has been in flight, accumulated by [`poll_timed`].
    elapsed: f32,
    _marker: PhantomData<fn() -> T>,
}

impl<T: DeserializeOwned> Pending<T> {
    fn new(request: Request) -> Self {
        Self {
            request,
            elapsed: 0.0,
            _marker: PhantomData,
        }
    }

    /// Poll for the response. `None` while still in flight; `Some(Ok)` on a
    /// decoded body, `Some(Err)` on transport or parse failure. A Standing
    /// rejection is a 403, which quad-net surfaces as a transport error (with
    /// the status on native, collapsed to a generic failure on wasm).
    pub fn poll(&mut self) -> Option<Result<T, String>> {
        self.request.try_recv().map(|result| {
            result
                .map_err(|err| err.to_string())
                .and_then(|body| serde_json::from_str(&body).map_err(|err| err.to_string()))
        })
    }

    /// Poll, treating no response within `timeout` seconds as a failure. `dt` is
    /// the frame delta. This is the wasm safety net: quad-net's browser shim only
    /// resolves an HTTP 200, so a refused connection (server down) otherwise never
    /// resolves and the request would hang forever. Native transport errors still
    /// surface promptly through the inner [`poll`].
    pub fn poll_timed(&mut self, dt: f32, timeout: f32) -> Option<Result<T, String>> {
        if let Some(result) = self.poll() {
            return Some(result);
        }
        self.elapsed += dt;
        if self.elapsed >= timeout {
            Some(Err("the server did not respond in time".to_owned()))
        } else {
            None
        }
    }
}

impl ServerClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            player_id: None,
            token: None,
        }
    }

    /// Adopt the guest session id the server minted, so every later request
    /// carries it (GDD 7.7).
    pub fn set_player_id(&mut self, player_id: String) {
        self.player_id = Some(player_id);
    }

    /// Carry a WebHatchery account token on the next `/session` and `/link`
    /// (GDD 7.3), or clear it with `None`. Set from a persisted token on startup
    /// so the client resumes its account deity rather than a fresh guest.
    pub fn set_token(&mut self, token: Option<String>) {
        self.token = token;
    }

    /// A `GET` request builder for `path`, carrying the session header if we have
    /// one yet (`/session` itself does not).
    fn get(&self, path: &str) -> RequestBuilder {
        self.with_session(RequestBuilder::new(&format!("{}{path}", self.base_url)))
    }

    /// Attach the `X-Player-Id` header once a session has been established.
    fn with_session(&self, builder: RequestBuilder) -> RequestBuilder {
        match &self.player_id {
            Some(id) => builder.header(PLAYER_ID_HEADER, id),
            None => builder,
        }
    }

    /// Attach the `Authorization: Bearer` account token if one has been set
    /// (GDD 7.3); otherwise leave the request as a plain guest request.
    fn with_auth(&self, builder: RequestBuilder) -> RequestBuilder {
        match &self.token {
            Some(token) => builder.header("Authorization", &format!("Bearer {token}")),
            None => builder,
        }
    }

    /// `POST /session` — begin a session (§7.7). With no token this mints a fresh
    /// guest; with an account token set it resumes that account's deity across
    /// devices (GDD 7.3). Feed the returned id to
    /// [`set_player_id`](ServerClient::set_player_id).
    pub fn create_session(&self) -> Pending<SessionResponse> {
        Pending::new(
            self.with_auth(
                RequestBuilder::new(&format!("{}/session", self.base_url)).method(Method::Post),
            )
            .send(),
        )
    }

    /// `POST /link` — bind the deity the client is currently playing to the
    /// WebHatchery account its token authenticates (GDD 7.3). The reply names the
    /// deity to carry forward (usually the same one, now the account's; or, if the
    /// account already owned one, that deity to resume).
    pub fn link(&self) -> Pending<SessionResponse> {
        Pending::new(
            self.with_auth(self.with_session(
                RequestBuilder::new(&format!("{}/link", self.base_url)).method(Method::Post),
            ))
            .send(),
        )
    }

    /// `GET /view` — the player's Standing-filtered view of the world (§7.7).
    pub fn fetch_view(&self) -> Pending<ClientView> {
        Pending::new(self.get("/view").send())
    }

    /// `GET /events?since=` — the chronicle delta since `cursor` (§7.4).
    pub fn fetch_events(&self, since: u64) -> Pending<EventsDelta> {
        Pending::new(self.get(&format!("/events?since={since}")).send())
    }

    /// `POST /action` — submit an authoritative command, returning its feedback.
    /// A command beyond the player's Standing comes back as an error (§7.7).
    pub fn submit_action(&self, action: &PlayerAction) -> Pending<ActionReport> {
        let body = serde_json::to_string(action).unwrap_or_default();
        Pending::new(
            self.with_session(
                RequestBuilder::new(&format!("{}/action", self.base_url))
                    .method(Method::Post)
                    .header("Content-Type", "application/json")
                    .body(&body),
            )
            .send(),
        )
    }
}

#[cfg(test)]
mod tests;
