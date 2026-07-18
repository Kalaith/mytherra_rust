//! Mytherra — a minor deity watching one living, shared world.
//!
//! This binary is the macroquad client. It currently runs a local, deterministic
//! simulation of the world (GDD 5.2-5.7 mechanics); the multiplayer server layer
//! (GDD 7) is a later phase.

use macroquad::prelude::*;
use macroquad_toolkit::capture;

mod data;
mod game;
mod save;
mod sim;
mod ui;
mod world;

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
        next_frame().await;
    }
}
