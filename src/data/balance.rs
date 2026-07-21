//! Simulation and economy tuning, loaded from `balance.json`.
//!
//! Every magic number the world sim and favor economy use lives here rather
//! than in Rust source, per the data-driven design rule. Rust only names the
//! shape; designers tune the values in JSON.
//!
//! The tuning structs are grouped by domain into sibling modules and
//! re-exported here, so the rest of the crate sees a single flat `data::…`
//! surface. The top-level [`Balance`] aggregate and the two odds-and-ends
//! structs that don't belong to a larger domain stay in this file.

mod champion;
mod divine;
mod economy;
mod era;
mod genesis;
mod hero;
mod region;

pub use champion::*;
pub use divine::*;
pub use economy::*;
pub use era::*;
pub use genesis::*;
pub use hero::*;
pub use region::*;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Balance {
    pub region: RegionBalance,
    pub genesis: GenesisBalance,
    pub conquest: ConquestBalance,
    pub frontier: FrontierBalance,
    pub hero: HeroBalance,
    pub champion: ChampionBalance,
    pub betting: BettingBalance,
    pub omens: OmensBalance,
    pub artifact: ArtifactBalance,
    pub weather: WeatherBalance,
    pub magic: MagicBalance,
    pub myth: MythBalance,
    pub civilization: CivilizationBalance,
    pub pantheon: PantheonBalance,
    pub era: EraBalance,
    pub settlement: SettlementBalance,
    pub resource: ResourceBalance,
    pub culture: CultureBalance,
    pub trade: TradeBalance,
    pub player: PlayerBalance,
    pub settings: SettingsBalance,
    pub tenor: TenorBalance,
}

/// Tuning for the dashboard's qualitative "state of the world" read (GDD 10): a
/// health score is `avg prosperity - avg danger - avg chaos - crises *
/// crisis_penalty`, and the descending `thresholds` bucket it into an age from
/// golden to dark. `strings.ui.tenor_labels` names each bucket (one more label
/// than thresholds).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenorBalance {
    pub crisis_penalty: f32,
    pub thresholds: Vec<f32>,
}

/// Favor-economy tuning for the player's own level-ups (GDD 5.1).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerBalance {
    pub level_base_cost: i64,
    pub level_cost_step: i64,
    /// A rising deity holds more divine power and recovers it faster: each level
    /// past the first adds this much to the favor cap and per-tick recovery,
    /// giving level-ups a mechanical payoff (GDD 5.1).
    pub max_favor_per_level: i64,
    pub favor_per_tick_per_level: i64,
    /// Experience granted for unlocking an achievement (GDD 5.1): a milestone is a
    /// great deed that elevates the deity, so achievements feed progression rather
    /// than being pure vanity. Awarded as experience, not favor, so it is never
    /// clipped by the favor ceiling and compounds into a higher standing.
    pub achievement_experience: i64,
    /// Favor a faithful land tithes its god each tick, per point of divine
    /// resonance above the neutral baseline (GDD 5.1 <-> 5.4): the devout pour
    /// power back to the deity they serve, so a world of hallowed lands sustains
    /// more divine action than a faithless one. This closes the favor loop —
    /// consecrating a region (or letting its Clerics tend it) now pays a standing
    /// income, not only cheaper and stronger actions on that one land.
    pub favor_per_resonance: f32,
    /// The resonance a region must exceed before it tithes — the neutral point
    /// (50) at which a land is neither faithful nor faithless, matching the
    /// cost/effect-multiplier pivot in `Region`.
    pub favor_tithe_baseline: f32,
}

/// Settings-screen tuning (GDD 10): the selectable auto-tick cadences, in real
/// seconds between world ticks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettingsBalance {
    pub tick_speed_presets: Vec<f32>,
}
