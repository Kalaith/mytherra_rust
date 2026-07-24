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
/// presents it on every subsequent request.
pub struct ServerClient {
    base_url: String,
    player_id: Option<String>,
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
        }
    }

    /// Adopt the guest session id the server minted, so every later request
    /// carries it (GDD 7.7).
    pub fn set_player_id(&mut self, player_id: String) {
        self.player_id = Some(player_id);
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

    /// `POST /session` — ask the server for a fresh guest session (§7.7). Feed the
    /// returned id to [`set_player_id`](ServerClient::set_player_id).
    pub fn create_session(&self) -> Pending<SessionResponse> {
        Pending::new(
            RequestBuilder::new(&format!("{}/session", self.base_url))
                .method(Method::Post)
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
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    /// Poll a request to completion (native tests only — quad-net resolves it on
    /// a background thread). Never used in the game loop, which polls per frame.
    fn block_on<T: DeserializeOwned>(mut pending: Pending<T>) -> Result<T, String> {
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            if let Some(result) = pending.poll() {
                return result;
            }
            if Instant::now() > deadline {
                return Err("timed out waiting for the server".to_owned());
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    /// End-to-end against a running `mytherra-server` on the default port. Start
    /// the server, then run: `cargo test -p mytherra -- --ignored net`.
    #[test]
    #[ignore = "needs a live mytherra-server on 127.0.0.1:8791"]
    fn round_trip_against_a_live_server() {
        let mut client = ServerClient::new("http://127.0.0.1:8791");

        // A request without a session is rejected (§7.7).
        assert!(
            block_on(client.fetch_view()).is_err(),
            "a view without a session is unauthorized"
        );

        // Establish a guest session; every later request carries its id.
        let session = block_on(client.create_session()).expect("create session");
        client.set_player_id(session.player_id);

        let view = block_on(client.fetch_view()).expect("fetch view");
        assert!(!view.world.heroes.is_empty(), "a fresh guest sees heroes");
        assert!(
            view.world.regions.is_empty(),
            "a Watcher has not unlocked regions"
        );

        // A Watcher may designate a champion (hero-adjacent, §5.9). Pick a *living*
        // hero — in a long-running world most of the roster has passed on, and a
        // champion can only be raised from the quick.
        let hero = view
            .world
            .heroes
            .iter()
            .find(|h| h.is_alive)
            .expect("a living hero to champion")
            .id
            .clone();
        let report =
            block_on(client.submit_action(&PlayerAction::DesignateChampion { hero_id: hero }))
                .expect("designate champion");
        assert!(!report.feedback.is_empty(), "the act reports feedback");

        // ...but a region action is forbidden at Watcher standing (§7.7).
        let forbidden = block_on(client.submit_action(&PlayerAction::RegionAction {
            region_id: "aldermoor".to_owned(),
            action_id: "bless".to_owned(),
        }));
        assert!(forbidden.is_err(), "regions are locked at Watcher standing");

        let delta = block_on(client.fetch_events(0)).expect("fetch events");
        assert!(delta.cursor >= 1, "the awakening event advances the cursor");
    }

    /// Two guests get independent state: each has its own favor, so one deity's
    /// spending never touches another's. Start the server, then run:
    /// `cargo test -p mytherra -- --ignored two_guests`.
    #[test]
    #[ignore = "needs a live mytherra-server on 127.0.0.1:8791"]
    fn two_guests_hold_independent_favor() {
        let base = "http://127.0.0.1:8791";
        let mut alice = ServerClient::new(base);
        let mut bob = ServerClient::new(base);
        alice.set_player_id(
            block_on(alice.create_session())
                .expect("alice session")
                .player_id,
        );
        bob.set_player_id(
            block_on(bob.create_session())
                .expect("bob session")
                .player_id,
        );

        let alice_before = block_on(alice.fetch_view())
            .expect("alice view")
            .player
            .player
            .favor;
        let bob_before = block_on(bob.fetch_view())
            .expect("bob view")
            .player
            .player
            .favor;

        // Alice designates a champion — favor leaves *her* purse. Champions are
        // raised only from living heroes, rare in a long-lived world.
        let hero = block_on(alice.fetch_view())
            .expect("alice view")
            .world
            .heroes
            .iter()
            .find(|h| h.is_alive)
            .expect("a living hero")
            .id
            .clone();
        block_on(alice.submit_action(&PlayerAction::DesignateChampion { hero_id: hero }))
            .expect("alice designates");

        let alice_after = block_on(alice.fetch_view())
            .expect("alice view")
            .player
            .player
            .favor;
        let bob_after = block_on(bob.fetch_view())
            .expect("bob view")
            .player
            .player
            .favor;

        assert!(alice_after < alice_before, "alice paid for her champion");
        assert_eq!(bob_after, bob_before, "bob's favor is untouched by alice");
    }
}
