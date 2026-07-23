//! Mytherra's headless simulation core (GDD 7.2).
//!
//! Everything the world needs to advance, independent of how it's presented:
//! the shared [`world`] state and per-player economy, the deterministic
//! [`sim::tick_world`] tick, the [`data`]-driven content model, and [`save`]
//! serialization. This crate has no rendering dependency — the macroquad
//! client and the future server both build on it.

pub mod capability;
pub mod data;
pub mod save;
pub mod sim;
pub mod world;
