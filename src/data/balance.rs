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
}

/// Favor-economy tuning for the player's own level-ups (GDD 5.1).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerBalance {
    pub level_base_cost: i64,
    pub level_cost_step: i64,
}

/// Settings-screen tuning (GDD 10): the selectable auto-tick cadences, in real
/// seconds between world ticks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettingsBalance {
    pub tick_speed_presets: Vec<f32>,
}
