//! Talking to the authority server (GDD 7.4).
//!
//! A cross-platform, non-blocking HTTP client for the client's future *online*
//! mode. It's built on `quad-net`, which shares one poll-based API across
//! native (a background thread) and WASM (macroquad's own `sapp-jsutils` JS
//! interop — *not* wasm-bindgen, so it coexists with macroquad's WebGL build).
//!
//! Each call returns a [`Pending<T>`] the caller polls once per frame — never
//! blocking the game loop. Not yet wired in: the client still runs its world
//! offline (embedding `mytherra-core` and ticking locally). The online mode that
//! polls `/view` + `/events` and submits through `/action`, letting the server's
//! authority replace the local tick, is the next phase.
//!
//! WASM runtime caveat: quad-net's browser side calls JS functions
//! (`http_make_request`/`http_try_recv`) that its companion JS shim must define
//! on the page. Wiring that shim into the publish pipeline's page template, and
//! verifying fetch against a deployed server in a real browser, is the one step
//! that can't be checked from a headless build — it remains to be done.

use mytherra_core::command::ActionReport;
use mytherra_protocol::{ClientView, EventsDelta, PlayerAction};
use quad_net::http_request::{Method, Request, RequestBuilder};
use serde::de::DeserializeOwned;
use std::marker::PhantomData;

/// A handle to one authority server, addressed by base URL (e.g.
/// `http://127.0.0.1:8791`).
pub struct ServerClient {
    base_url: String,
}

/// A request in flight. Poll it each frame with [`poll`](Pending::poll): `None`
/// while pending, `Some` once the response (or an error) has arrived.
pub struct Pending<T> {
    request: Request,
    _marker: PhantomData<fn() -> T>,
}

impl<T: DeserializeOwned> Pending<T> {
    fn new(request: Request) -> Self {
        Self {
            request,
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
}

impl ServerClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
        }
    }

    /// `GET /view` — the player's Standing-filtered view of the world (§7.7).
    pub fn fetch_view(&self) -> Pending<ClientView> {
        Pending::new(RequestBuilder::new(&format!("{}/view", self.base_url)).send())
    }

    /// `GET /events?since=` — the chronicle delta since `cursor` (§7.4).
    pub fn fetch_events(&self, since: u64) -> Pending<EventsDelta> {
        Pending::new(RequestBuilder::new(&format!("{}/events?since={since}", self.base_url)).send())
    }

    /// `POST /action` — submit an authoritative command, returning its feedback.
    /// A command beyond the player's Standing comes back as an error (§7.7).
    pub fn submit_action(&self, action: &PlayerAction) -> Pending<ActionReport> {
        let body = serde_json::to_string(action).unwrap_or_default();
        Pending::new(
            RequestBuilder::new(&format!("{}/action", self.base_url))
                .method(Method::Post)
                .header("Content-Type", "application/json")
                .body(&body)
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
        let client = ServerClient::new("http://127.0.0.1:8791");

        let view = block_on(client.fetch_view()).expect("fetch view");
        assert!(!view.world.heroes.is_empty(), "a fresh guest sees heroes");
        assert!(
            view.world.regions.is_empty(),
            "a Watcher has not unlocked regions"
        );

        // A Watcher may designate a champion (hero-adjacent, §5.9).
        let hero = view.world.heroes[0].id.clone();
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
}
