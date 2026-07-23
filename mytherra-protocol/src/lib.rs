//! Mytherra's shared wire vocabulary (GDD 12).
//!
//! The client and the future server both depend on this crate so the shapes
//! they exchange can never disagree:
//!
//! - the §5.9 Standing model — [`VisibilityScope`], [`ActionVerb`],
//!   [`BettingMarket`], [`Standing`], [`Tier`] — re-exported from
//!   [`mytherra_core::capability`] (they're domain concepts; the tier →
//!   capability mapping is data-driven in [`mytherra_core::data::TierTable`]).
//! - [`PlayerAction`], the authoritative mutating commands a client submits —
//!   re-exported from [`mytherra_core::command`], which also owns the `apply`
//!   that both the server and the client's offline mode run.
//! - [`view`] — the per-player [`WorldView`]/[`PlayerView`] projections and the
//!   [`project`] function that filters shared world state by a player's Standing
//!   (§7.7).
//!
//! Nothing here performs I/O; the server owns transport and persistence.

pub mod view;

pub use mytherra_core::capability::{ActionVerb, BettingMarket, Standing, Tier, VisibilityScope};
pub use mytherra_core::command::PlayerAction;
pub use view::{project, PlayerView, WorldView};
