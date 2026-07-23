//! Talking to the authority server (GDD 7.4).
//!
//! A thin blocking HTTP client for the client's future *online* mode. Native
//! only for now — the WASM/WebGL build can't use a socket HTTP client and will
//! talk to the server through a browser-`fetch`-backed transport (a later
//! spike, GDD §7.4/§11), so this whole module is compiled out on `wasm32`.
//!
//! Not yet wired into the game loop: today the client runs its world offline
//! (embedding `mytherra-core` and ticking locally). The online mode that polls
//! `/view` + `/events` and submits through `/action` — replacing the local tick
//! with the server's authority — is the next phase.

use mytherra_core::command::ActionReport;
use mytherra_protocol::{ClientView, EventsDelta, PlayerAction};

/// A handle to one authority server, addressed by base URL (e.g.
/// `http://127.0.0.1:8791`).
pub struct ServerClient {
    base_url: String,
}

impl ServerClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
        }
    }

    /// `GET /view` — the player's Standing-filtered view of the world (§7.7).
    pub fn fetch_view(&self) -> Result<ClientView, String> {
        ureq::get(&format!("{}/view", self.base_url))
            .call()
            .map_err(|err| err.to_string())?
            .into_json()
            .map_err(|err| err.to_string())
    }

    /// `GET /events?since=` — the chronicle delta since `cursor` (§7.4).
    pub fn fetch_events(&self, since: u64) -> Result<EventsDelta, String> {
        ureq::get(&format!("{}/events", self.base_url))
            .query("since", &since.to_string())
            .call()
            .map_err(|err| err.to_string())?
            .into_json()
            .map_err(|err| err.to_string())
    }

    /// `POST /action` — submit an authoritative command, returning its feedback.
    /// A command beyond the player's Standing is a 403 (§7.7).
    pub fn submit_action(&self, action: &PlayerAction) -> Result<ActionReport, String> {
        match ureq::post(&format!("{}/action", self.base_url)).send_json(action) {
            Ok(response) => response.into_json().map_err(|err| err.to_string()),
            Err(ureq::Error::Status(403, _)) => {
                Err("that divine art lies beyond your present standing".to_owned())
            }
            Err(err) => Err(err.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// End-to-end against a running `mytherra-server` on the default port. Start
    /// the server, then run: `cargo test -p mytherra -- --ignored net`.
    #[test]
    #[ignore = "needs a live mytherra-server on 127.0.0.1:8791"]
    fn round_trip_against_a_live_server() {
        let client = ServerClient::new("http://127.0.0.1:8791");

        let view = client.fetch_view().expect("fetch view");
        assert!(!view.world.heroes.is_empty(), "a fresh guest sees heroes");
        assert!(
            view.world.regions.is_empty(),
            "a Watcher has not unlocked regions"
        );

        // A Watcher may designate a champion (hero-adjacent, §5.9).
        let hero = view.world.heroes[0].id.clone();
        let report = client
            .submit_action(&PlayerAction::DesignateChampion { hero_id: hero })
            .expect("designate champion");
        assert!(!report.feedback.is_empty(), "the act reports feedback");

        // ...but a region action is forbidden at Watcher standing (§7.7).
        let forbidden = client.submit_action(&PlayerAction::RegionAction {
            region_id: "aldermoor".to_owned(),
            action_id: "bless".to_owned(),
        });
        assert!(forbidden.is_err(), "regions are locked at Watcher standing");

        let delta = client.fetch_events(0).expect("fetch events");
        assert!(delta.cursor >= 1, "the awakening event advances the cursor");
    }
}
