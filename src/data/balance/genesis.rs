//! Region-genesis tuning (GDD 5.2): the three ways the map reshapes at runtime —
//! fracture, conquest, and frontier founding. See `sim/genesis/`.

use serde::{Deserialize, Serialize};

/// Region-fracture tuning: when a region is torn by sustained chaos and danger,
/// secession pressure ("strife") builds until a hero leads part of it to break
/// away as a wholly new region.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenesisBalance {
    /// A region only accrues strife while its `pressure()` exceeds this.
    pub strife_pressure_threshold: f32,
    /// Base strife gained per tick while over the threshold.
    pub strife_gain: f32,
    /// Extra strife per point of pressure above the threshold.
    pub strife_over_scale: f32,
    /// Strife shed per tick while calm — larger than the gain, so only
    /// *sustained* turmoil fractures a region.
    pub strife_decay: f32,
    /// Upper bound on accumulated strife.
    pub strife_cap: f32,
    /// Strife at which the region fractures (given a founder and the population).
    pub fracture_threshold: f32,
    /// The parent must hold at least this population to split.
    pub min_population: f32,
    /// A living hero of at least this level in the region leads the breakaway;
    /// with no such catalyst, strife keeps building but no region is born.
    pub founder_min_level: u32,
    /// Fraction of the parent's population that leaves with the breakaway.
    pub population_split: f32,
    /// Per-settlement chance that a town in the parent defects to the breakaway.
    pub settlement_defect_chance: f32,
    /// Breakaway starting chaos — it is born in revolt.
    pub child_chaos: f32,
    /// Breakaway starting prosperity — a frontier starts poor.
    pub child_prosperity: f32,
    /// Fraction of the parent's danger the breakaway carries over.
    pub child_danger_carry: f32,
    /// Breakaway starting divine resonance and cultural influence.
    pub child_resonance: f32,
    pub child_cultural_influence: f32,
    /// Relief the parent feels once the pressure vents into a new region.
    pub parent_chaos_relief: f32,
    pub parent_danger_relief: f32,
    pub parent_prosperity_hit: f32,
    /// Secession momentum each fracture adds to the world (feeds Collapse-era
    /// pressure, GDD 5.7), and the ceiling that momentum can reach.
    pub momentum_gain: f32,
    pub momentum_cap: f32,
    /// Strife each point of Knowledge-artifact power bleeds from its region per
    /// tick (GDD 5.6 ↔ 5.2) — the player's lever to quell secession by reason,
    /// the counterpart to a champion holding a region by devotion.
    pub artifact_knowledge_relief: f32,
}

/// Region-conquest tuning: a strong region can annex a trade-linked neighbour
/// that has collapsed into crisis, merging the loser into the winner. The
/// inverse of a fracture — it removes a region rather than adding one.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConquestBalance {
    /// Military-might weights: a region projects force from its wealth, numbers,
    /// standing threat, and (for martial cultures) a warlike bonus.
    pub might_prosperity: f32,
    pub might_population: f32,
    pub might_danger: f32,
    pub might_martial_bonus: f32,
    /// A region must reach this might to move on a neighbour at all.
    pub aggressor_min_might: f32,
    /// The aggressor's might must exceed the target's by this margin.
    pub conquest_margin: f32,
    /// A living hero of at least this level shields its region from conquest —
    /// the same calibre of hero who would instead lead it to secede.
    pub defender_min_level: u32,
    /// A Protection artifact of at least this power wards its region against
    /// conquest entirely (GDD 5.6 ↔ 5.2) — the player's divine lever to save a
    /// threatened region from being absorbed.
    pub shield_min_power: u32,
    /// Conquest might each point of War-artifact power adds to a region — the
    /// offensive counterpart to the shield: empower a war relic to turn a region
    /// into a conqueror (or a militarised holdout).
    pub artifact_war_might: f32,
    /// If true, conquest only follows an existing trade route between the pair.
    pub require_trade_link: bool,
    /// Fraction of the loser's population the winner absorbs (the rest is lost).
    pub population_transfer: f32,
    /// Stat marks the war of conquest leaves on the victor.
    pub winner_prosperity: f32,
    pub winner_chaos: f32,
    pub winner_danger: f32,
    /// The world will never be conquered below this many regions.
    pub min_regions: usize,
    /// Conquest momentum each annexation adds to the world (feeds Conquest-era
    /// pressure, GDD 5.7), and the ceiling that momentum can reach.
    pub momentum_gain: f32,
    pub momentum_cap: f32,
}

/// Frontier-founding tuning: the third genesis path and the mirror of a fracture
/// — born of prosperity, not strife. A veteran hero in a *thriving*, populous
/// region can lead settlers out to found a new frontier region.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrontierBalance {
    /// A hero needs at least this level to lead a founding expedition.
    pub founder_min_level: u32,
    /// The home region must hold at least this population to spare settlers.
    pub parent_min_population: f32,
    /// Per-eligible-hero, per-tick chance of founding — kept low so expansion is
    /// occasional rather than explosive.
    pub found_chance: f32,
    /// Founding chance each point of Prosperity-artifact power adds to a region
    /// (GDD 5.6 ↔ 5.2) — the player's lever to encourage expansion, the peaceful
    /// counterpart to War artifacts driving conquest.
    pub artifact_prosperity_chance: f32,
    /// Fraction of the home region's population that leaves to settle.
    pub settler_fraction: f32,
    /// The world will never grow past this many regions by founding.
    pub max_regions: usize,
    /// A new frontier's starting stats — a hopeful but raw and wild colony.
    pub child_prosperity: f32,
    pub child_chaos: f32,
    pub child_danger: f32,
    /// Fraction of the home region's magic affinity the frontier inherits.
    pub child_magic_carry: f32,
    pub child_resonance: f32,
    pub child_cultural_influence: f32,
}
