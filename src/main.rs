//! Mytherra — a minor deity watching one living, shared world.
//!
//! This binary is the macroquad client. It currently runs a local, deterministic
//! simulation of the world (GDD 5.2-5.7 mechanics); the multiplayer server layer
//! (GDD 7) is a later phase.

use macroquad::prelude::*;
use macroquad_toolkit::capture;

mod game;
mod ui;

// The native HTTP client for the authority server (GDD 7.4). WASM uses a
// different transport (a later spike), so this is native-only; it is not yet
// wired into the loop (the client still runs offline), hence `allow(dead_code)`.
#[cfg(not(target_arch = "wasm32"))]
#[allow(dead_code)]
mod net;

// The simulation core lives in the `mytherra-core` crate (GDD 7.2). Re-export
// its modules at the crate root so the client's `crate::{data,world,sim,save}`
// paths resolve unchanged.
pub use mytherra_core::{data, save, sim, world};

use game::Game;

fn window_conf() -> Conf {
    capture::capture_window_conf(
        "MYTHERRA",
        "Mytherra",
        ui::LOGICAL_WIDTH as i32,
        ui::LOGICAL_HEIGHT as i32,
    )
}

#[macroquad::main(window_conf)]
async fn main() {
    let mut game = Game::new().await;

    // Screenshot harness: when MYTHERRA_CAPTURE_PATH is set, render deterministic
    // frames, write a PNG, and exit.
    if let Some(config) = capture::CaptureConfig::from_env("MYTHERRA") {
        game.begin_capture_scene(&config.scene);
        capture::run_capture(&config, |dt| {
            game.update(dt);
            game.draw();
        })
        .await;
        return;
    }

    loop {
        let dt = get_frame_time().min(0.1);
        game.update(dt);
        game.draw();
        if game.quit_requested() {
            break;
        }
        next_frame().await;
    }
}
