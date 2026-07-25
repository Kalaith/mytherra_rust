//! Client-side account state (GDD 7.3): remembering the WebHatchery account
//! token so the client resumes *its own* deity across restarts and devices,
//! rather than a fresh guest each launch.
//!
//! The token is the only credential the client persists (§12) — never world
//! state, which lives solely on the authority. Storage rides the toolkit's
//! cross-platform key store: an app-data file on native, the `localStorage`
//! bridge on wasm, so the same code path serves the browser and native client.

use macroquad_toolkit::persistence::{load_string_key, save_string_key};

/// The key the account token is stored under, per game.
const TOKEN_KEY: &str = "account_token";

/// The persisted account token, if the player has linked before. An empty stored
/// value (a cleared token) reads as `None`.
pub fn load_token(game_name: &str) -> Option<String> {
    load_string_key(game_name, TOKEN_KEY)
        .ok()
        .map(|token| token.trim().to_owned())
        .filter(|token| !token.is_empty())
}

/// Remember the account token so the next launch resumes this account's deity.
/// A storage failure is non-fatal — the player simply isn't remembered — so it
/// never interrupts play.
pub fn save_token(game_name: &str, token: &str) {
    let _ = save_string_key(game_name, TOKEN_KEY, token);
}
