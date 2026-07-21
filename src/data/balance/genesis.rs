//! Region-genesis tuning (GDD 5.2): the three ways the map reshapes at runtime —
//! fracture, conquest, and frontier founding. See `sim/genesis/`.

use crate::data::HeroRole;
use serde::{Deserialize, Serialize};

/// How much each hero role contributes to its region's military might, as a
/// fraction of a warrior's full share (GDD 5.2): a land of warriors and rangers
/// stands far stronger against invasion than one of scholars and merchants of the
/// same renown, so a region's martial weight depends on *who* defends it, not just
/// how many. Mirrors the per-enum tables elsewhere (`ChampionFocuses`, etc.).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeroMightWeights {
    pub warrior: f32,
    pub mage: f32,
    pub scholar: f32,
    pub ranger: f32,
    pub merchant: f32,
    pub cleric: f32,
}

impl HeroMightWeights {
    pub fn get(&self, role: HeroRole) -> f32 {
        match role {
            HeroRole::Warrior => self.warrior,
            HeroRole::Mage => self.mage,
            HeroRole::Scholar => self.scholar,
            HeroRole::Ranger => self.ranger,
            HeroRole::Merchant => self.merchant,
            HeroRole::Cleric => self.cleric,
        }
    }
}

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
    /// Per-node chance that a resource node in the parent is carried off by the
    /// breakaway (GDD 5.2): the seceding land takes its mines, farms, and
    /// manasprings with it, so a fracture divides the parent's wealth the way a
    /// conquest seizes it — kept below the town defect rate, since territory
    /// splits less readily than allegiance. A breakaway is thus a full economic
    /// citizen, its carried nodes feeding it and able to corrupt or flourish.
    pub node_defect_chance: f32,
    /// Breakaway starting chaos — it is born in revolt.
    pub child_chaos: f32,
    /// Breakaway starting prosperity — a frontier starts poor.
    pub child_prosperity: f32,
    /// Fraction of the parent's danger the breakaway carries over.
    pub child_danger_carry: f32,
    /// Breakaway starting divine resonance and cultural influence.
    pub child_resonance: f32,
    pub child_cultural_influence: f32,
    /// Volume of the trade route linking a breakaway to the land it revolted from
    /// (GDD 5.2): kept low, since a bitter secession leaves strained ties — but
    /// nonzero, so the new region isn't born marooned from the trade network (and
    /// so it can, in time, be reconquered along that very road).
    pub child_trade_volume: f32,
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
    /// Extra might-gap an aggressor must clear to overrun a region whose prevailing
    /// civilization course is Defense (GDD 5.2 <-> 5.6): a people set on defense
    /// are harder to conquer — a graded resistance the player can raise with the
    /// Advance verb, short of the outright shield a hero or a ward provides.
    pub defense_margin_bonus: f32,
    /// Margin an aggressor whose prevailing course is Rivalry will forgo to strike
    /// (GDD 5.2 <-> 5.6): a bellicose people accept a closer fight, so a region set
    /// on Rivalry conquers where a cautious neighbour would hold off — the offensive
    /// mirror of Defense, completing the agenda's grip on the three genesis paths.
    pub rivalry_aggression: f32,
    /// A living hero of at least this level shields its region from conquest —
    /// the same calibre of hero who would instead lead it to secede.
    pub defender_min_level: u32,
    /// A hero of at least this renown likewise shields its region, even below the
    /// level bar: a famous name deters invaders (GDD 5.4 <-> 5.2). Since a
    /// cultivated champion's quests earn its hero renown, this makes the player's
    /// investment in a champion pay off in the defence of its home.
    pub defender_renown_min: f32,
    /// A Protection artifact of at least this power wards its region against
    /// conquest entirely (GDD 5.6 ↔ 5.2) — the player's divine lever to save a
    /// threatened region from being absorbed.
    pub shield_min_power: u32,
    /// Conquest might each point of War-artifact power adds to a region — the
    /// offensive counterpart to the shield: empower a war relic to turn a region
    /// into a conqueror (or a militarised holdout).
    pub artifact_war_might: f32,
    /// Conquest might each level of a living resident hero adds to a region (GDD
    /// 5.2), before its role weight: a land defended by many capable heroes is a
    /// stronger aggressor and a harder target, and one whose heroes have all
    /// fallen is ripe for the taking — distinct from the lone-legend shield, which
    /// blocks conquest outright.
    pub might_per_hero_level: f32,
    /// Per-role share of that per-level might: warriors count full, loremasters
    /// barely, so a region's martial weight reflects who defends it.
    pub hero_might_weights: HeroMightWeights,
    /// If true, conquest only follows an existing trade route between the pair.
    pub require_trade_link: bool,
    /// Fraction of the loser's population the winner absorbs (the rest is lost).
    pub population_transfer: f32,
    /// Stat marks the war of conquest leaves on the victor.
    pub winner_prosperity: f32,
    pub winner_chaos: f32,
    pub winner_danger: f32,
    /// The war that breaks a region falls hardest on its greatest city (GDD 5.2):
    /// the seat of resistance is sacked as the region falls, losing this fraction
    /// of its people and this many points of prosperity. A metropolis so sacked
    /// can drop a size tier — the fall of a great city, written into the map.
    pub sack_population_loss: f32,
    pub sack_prosperity_loss: f32,
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
    /// Founding-chance bonus a region gains when its prevailing civilization
    /// course is Expansion (GDD 5.6 <-> 5.2): a people set on expansion strike out
    /// to found frontiers more readily, so the player's Advance-agenda nudge on a
    /// thriving region becomes a lever on where the map grows.
    pub expansion_found_chance: f32,
    /// Founding-chance bonus a region gains per living Ranger dwelling in it (GDD
    /// 5.2 <-> 5.4): rangers are the pathfinders who scout the wilds and find the
    /// way to virgin land, so a land with wardens strikes out to found frontiers
    /// more readily. This is the Ranger role's own domain — the counterpart to a
    /// Cleric tending faith and a Merchant swelling trade — turning heroes into a
    /// driver of where the map grows, not only who defends it.
    pub ranger_found_chance: f32,
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
    /// Volume of the trade route linking a frontier to its motherland (GDD 5.2):
    /// kept high — an amicable expansion keeps warm, busy ties home, far more
    /// than a bitter breakaway does — so the colony shares in trade wealth and
    /// culture from birth.
    pub child_trade_volume: f32,
}
