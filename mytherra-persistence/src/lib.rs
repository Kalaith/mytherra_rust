//! Persistence for the Mytherra authority server (GDD 6/8): the DB *is* the save.
//!
//! This crate deliberately sits *outside* `mytherra-core`. The core is the pure,
//! deterministic simulation and must keep compiling to wasm (the client embeds
//! it for its capture fixture), so it can never link a database. Persistence
//! depends on core for the state types and does all the I/O here.
//!
//! The store is split in two, mirroring the player↔world dissociation: the world
//! is a self-contained simulation the deities *nudge*, not a thing any player
//! owns. So [`WorldStore`] and [`PlayerStore`] share a connection pool but never
//! share a row — a player's only tie to the world is the action it submits and
//! the favor/bet effects the world hands back.
//!
//! Configuration is the caller's concern: the server builds a [`DbConfig`] from
//! its own `.env`; this crate knows nothing about environment variable names.

mod config;
mod player_store;
mod world_store;

pub use config::{DbConfig, Store};
pub use player_store::PlayerStore;
pub use world_store::WorldStore;
