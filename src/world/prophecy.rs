//! Runtime prophecies (GDD 5.6): the world's foretold turnings. Between the
//! passing portents of an omen and the structural turn of an era lies a longer
//! arc — a doom or a golden age the world's own drift foretells, that builds
//! toward its coming while the world stays its course and is turned aside when the
//! world turns. A prophecy is the aggregate state of the world made narrative:
//! spoken when the realms as a whole tip toward darkness or plenty, fulfilled if
//! that tide holds, averted if it ebbs. Prophecies arise dynamically, so there is
//! no seed content.

use serde::{Deserialize, Serialize};

/// The two poles a prophecy foretells: a gathering darkness, or a coming plenty.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProphecyKind {
    Doom,
    GoldenAge,
}

/// A foretold turning of the whole world.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Prophecy {
    pub id: String,
    /// The prophecy's name, e.g. "the Coming of the Long Dark".
    pub name: String,
    pub kind: ProphecyKind,
    /// How near the foretold turning is, 0..1: it advances while the world holds
    /// to the course that was foretold, and recedes when the world turns from it,
    /// so a doom can be averted and a golden age can slip away ungrasped.
    pub progress: f32,
    /// The year the prophecy was first spoken, for the chronicle.
    pub foretold_year: u32,
}
