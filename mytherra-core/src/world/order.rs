//! Runtime great Orders (GDD 5.4): the world's professional fellowships. Where a
//! House is a bloodline seated in one region, an Order is an institution bound to
//! a calling — the Arcane Circle of mages, the Warriors' Order, the Merchant
//! League — that spans every region its kind dwell in and outlasts any single
//! member. An Order arises when a role reaches a critical mass of the living
//! across the world, draws its standing from the fellowship's numbers, and lends
//! its cultural weight to each region that hosts a chapter, until its ranks thin
//! and it is dissolved. Orders arise dynamically, so there is no seed content;
//! they reference their members by role rather than by name.

use crate::data::HeroRole;
use serde::{Deserialize, Serialize};

/// A great Order — a trans-regional fellowship of one calling.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Order {
    pub id: String,
    /// The Order's name, e.g. "the Arcane Circle".
    pub name: String,
    /// The calling that binds the Order; its membership is every living hero of
    /// this role, wherever they dwell.
    pub role: HeroRole,
    /// The Order's standing, drifting toward the size of its living fellowship: it
    /// swells as the calling flourishes across the world and fades as its ranks
    /// thin, and it sets how much cultural weight the Order lends its chapters.
    pub prestige: f32,
    /// The year the Order was founded, for the chronicle.
    pub founded_year: u32,
}
