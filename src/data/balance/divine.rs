//! Divine Observatory betting and divine-tool tuning (GDD 5.5, 5.6): artifacts,
//! weather, magic, myths, the pantheon, and civilization agendas.

use crate::data::artifact::ArtifactFocus;
use serde::{Deserialize, Serialize};

/// Divine Observatory betting tuning (GDD 5.5).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BettingBalance {
    /// How many active (unresolved) speculation events to keep available.
    pub active_events: usize,
    /// Hard cap on stored events before old resolved ones are pruned.
    pub event_cap: usize,
    /// Cap on retained *resolved* wagers; pending wagers are never pruned.
    pub bet_history_cap: usize,
    /// Selectable stake amounts.
    pub stake_presets: Vec<i64>,
    /// Range of the total simulated crowd stake seeded onto an event; it is then
    /// split between the outcomes by the crowd's read of the likelihood.
    pub crowd_seed_min: f32,
    pub crowd_seed_max: f32,
    /// How far the simulated crowd's lean can wander from the true likelihood —
    /// the crowd is wise but not perfectly rational (GDD 5.5).
    pub crowd_noise: f32,
    /// Total simulated crowd stake added per tick to each active event, split by
    /// the event's *current* likelihood — so the watching deities keep betting as
    /// the world shifts and their lean tracks it, rewarding an early read (5.5).
    pub crowd_drift: f32,
    /// Bounds on the world-state-derived target odds modifier.
    pub target_mod_min: f32,
    pub target_mod_max: f32,
    /// Bounds on the crowd-lean payout adjustment.
    pub crowd_lean_min: f32,
    pub crowd_lean_max: f32,
    /// Bounds on the final gross payout multiplier.
    pub payout_min_mult: f32,
    pub payout_max_mult: f32,
    /// Floor on final odds.
    pub min_odds: f32,
}

/// Omens forecasting tuning (GDD 5.6). Omens never mutate world state; these
/// values only shape how far the read-only projection extrapolates.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OmensBalance {
    /// How many ticks of the current pressure drift to extrapolate for the
    /// generational horizon.
    pub horizon_ticks: f32,
    /// Deadzone (in pressure points/tick) below which the drift reads as
    /// "holding" rather than deepening or easing.
    pub trend_deadzone: f32,
}

/// Artifact tool tuning (GDD 5.6).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactBalance {
    pub max_active: usize,
    pub create_cost: i64,
    pub empower_base_cost: i64,
    pub empower_power_mult: i64,
    pub empower_instability_div: f32,
    pub transfer_cost: i64,
    pub stabilize_cost: i64,
    pub stabilize_amount: f32,
    pub empower_instability_gain: f32,
    pub instability_per_tick: f32,
    pub instability_power_mult: f32,
    pub backlash_threshold: f32,
    pub backlash_chaos: f32,
    pub backlash_danger: f32,
    /// A backlash isn't over when the relic shatters: its aftermath unfolds in
    /// two delayed steps (GDD 5.6). First a settlement is blighted, then a later
    /// pulse of unrest strikes the region.
    pub aftermath_blight_delay: i32,
    pub aftermath_blight_prosperity: f32,
    pub aftermath_unrest_delay: i32,
    pub aftermath_unrest_chaos: f32,
    pub aftermath_unrest_danger: f32,
    pub focus_effect: ArtifactFocusEffect,
}

/// Per-power stat magnitude each artifact focus applies to its region.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactFocusEffect {
    pub protection: f32,
    pub prosperity: f32,
    pub war: f32,
    pub knowledge: f32,
}

impl ArtifactFocusEffect {
    pub fn per_power(&self, focus: ArtifactFocus) -> f32 {
        match focus {
            ArtifactFocus::Protection => self.protection,
            ArtifactFocus::Prosperity => self.prosperity,
            ArtifactFocus::War => self.war,
            ArtifactFocus::Knowledge => self.knowledge,
        }
    }
}

/// Weather tool tuning (GDD 5.6).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeatherBalance {
    pub base_cost: i64,
    pub decay_per_tick: f32,
    pub min_magnitude: f32,
    pub max_active: usize,
    /// Per-tick chance a natural weather front arises somewhere, biased by the
    /// region's climate — the skies live without the player shaping them.
    pub natural_chance: f32,
    /// Chance a natural front is Strong rather than Gentle.
    pub natural_strong_chance: f32,
    /// Intensity ids a natural front uses (never Cataclysmic — that's the
    /// player's alone).
    pub natural_gentle_id: String,
    pub natural_strong_id: String,
    /// A harmful weather-working (net loss of prosperity) leaves a delayed scar
    /// — flood or famine following the storm (GDD 5.6). Blight is per unit of the
    /// shaped intensity's magnitude.
    pub aftermath_delay: i32,
    pub aftermath_blight: f32,
    /// Beneficial weather (net gain of prosperity) instead leaves a delayed
    /// bounty — a bountiful harvest per unit of the shaped intensity's magnitude.
    pub aftermath_bloom: f32,
}

/// Pantheon tool tuning (GDD 5.6).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PantheonBalance {
    pub appease_cost: i64,
    pub challenge_cost: i64,
    pub appease_amount: f32,
    pub challenge_amount: f32,
    /// How much an action ripples to the target's ally / rival.
    pub ripple: f32,
    pub cooldown: i32,
    pub drift_target: f32,
    pub drift_rate: f32,
    /// Autonomous diamond coupling per tick (GDD 5.6): a rival's agitation above
    /// the resting baseline provokes a deity (`rival_coupling`), while an ally's
    /// pressure pulls it into solidarity (`ally_coupling`). Both read every
    /// deity's pressure as it stood at tick start, so the web stays deterministic.
    pub rival_coupling: f32,
    pub ally_coupling: f32,
    /// How strongly the world's average of a deity's domain stat shifts its
    /// pressure target away from the baseline (GDD 5.6): a deity whose domain is
    /// ascendant across the world stirs on its own, so a dangerous age rouses the
    /// war god and a prosperous one both its patron and its nemesis.
    pub domain_response: f32,
    /// Ascending pressure tier thresholds and their effect multipliers.
    pub tiers: Vec<f32>,
    pub tier_mults: Vec<f32>,
}

/// Civilization tool tuning (GDD 5.6).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CivilizationBalance {
    pub apply_threshold: f32,
    pub advance_cost: i64,
    pub advance_boost: f32,
    pub boost_decay: f32,
    pub advance_cooldown: i32,
}

/// Myth tool tuning (GDD 5.6).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MythBalance {
    pub promote_cost: i64,
    pub cap: usize,
    pub echo_cooldown: i32,
    pub echo_threshold: f32,
    pub candidate_count: usize,
    pub resonance_min: f32,
    pub resonance_max: f32,
    pub resonance_spread: f32,
    pub resonance_scale: f32,
    /// Resonance a living myth loses each tick as it fades from living memory; a
    /// deeply-rooted tale (high initial resonance) endures for generations, a
    /// marginal one is soon forgotten (GDD 5.6).
    pub resonance_decay: f32,
    /// Resonance below which a myth is forgotten entirely and removed, freeing a
    /// slot on the capped roster so new tales can rise.
    pub forgotten_floor: f32,
    /// Baseline weight every region carries when a themed myth looks for a home,
    /// so a legend can still arise where its subject is faint — just less often.
    pub region_floor: f32,
    /// Theme (by id) a myth takes when it commemorates a hero's passage into
    /// legend (GDD 5.4 <-> 5.6). Falls back to the first theme if unmatched.
    pub legend_theme_id: String,
}

/// Magic tool tuning (GDD 5.6).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MagicBalance {
    pub progress_per_tick: f32,
    pub evidence_per_tick: f32,
    pub magic_affinity_coeff: f32,
    pub emerging_progress: f32,
    pub emerging_evidence: f32,
    pub known_progress: f32,
    pub known_evidence: f32,
    pub research_cost: i64,
    pub research_progress_gain: f32,
    pub research_evidence_gain: f32,
    pub emerging_effect_scale: f32,
    pub stat_cap: f32,
    /// A mature path's effect on a region is scaled by that region's arcane
    /// attunement: `affinity_base + magic_affinity * affinity_coeff` (GDD 5.6).
    /// Magic flows along the world's currents — attuned lands are reshaped more.
    pub affinity_base: f32,
    pub affinity_coeff: f32,
    /// Renown each *Known* path grants, per tick, to a living hero — scaled by
    /// the hero's region attunement. Magic reaches living things too, not just
    /// the land (GDD 5.6): an age of mastered arcana breeds legends.
    pub known_renown_per_tick: f32,
}
