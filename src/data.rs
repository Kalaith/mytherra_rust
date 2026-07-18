//! Embedded, data-driven game content loaded from `assets/data/*.json`.
//!
//! Content and balance values live in JSON per the RustGames data-driven rule;
//! Rust only describes their shape. Native builds could later read these from
//! disk, but the embedded copies keep WASM builds self-contained.

mod action;
mod balance;
mod config;
mod region;
pub mod strings;

pub use action::RegionActionDef;
pub use balance::{Balance, PlayerBalance, RegionBalance};
pub use config::GameConfig;
pub use region::{ClimateType, Culture, RegionSeed};
pub use strings::{fill, Strings};

use macroquad_toolkit::data_loader::{
    load_embedded_json, load_embedded_json_labeled, DataRegistry,
};

const GAME_CONFIG_JSON: &str = include_str!("../assets/data/game_config.json");
const REGIONS_JSON: &str = include_str!("../assets/data/regions.json");
const REGION_ACTIONS_JSON: &str = include_str!("../assets/data/region_actions.json");
const BALANCE_JSON: &str = include_str!("../assets/data/balance.json");
const STRINGS_JSON: &str = include_str!("../assets/data/strings.json");

/// All static content the game needs, resolved once at boot.
#[derive(Debug, Clone)]
pub struct GameData {
    pub config: GameConfig,
    pub regions: Vec<RegionSeed>,
    pub region_actions: DataRegistry<RegionActionDef>,
    pub balance: Balance,
    pub strings: Strings,
}

impl GameData {
    pub fn load() -> Result<Self, String> {
        let config = load_embedded_json_labeled("game_config", GAME_CONFIG_JSON)?;
        let regions: Vec<RegionSeed> = load_embedded_json(REGIONS_JSON)?;
        let region_actions = DataRegistry::from_embedded_json(REGION_ACTIONS_JSON, "id")?;
        let balance: Balance = load_embedded_json_labeled("balance", BALANCE_JSON)?;
        let strings: Strings = load_embedded_json_labeled("strings", STRINGS_JSON)?;

        if regions.is_empty() {
            return Err("regions.json contained no regions".to_owned());
        }

        Ok(Self {
            config,
            regions,
            region_actions,
            balance,
            strings,
        })
    }

    /// Region actions in a stable authored order (cheapest first) for the UI.
    pub fn ordered_region_actions(&self) -> Vec<&RegionActionDef> {
        let mut actions: Vec<&RegionActionDef> =
            self.region_actions.iter().map(|(_, a)| a).collect();
        actions.sort_by(|a, b| a.cost.cmp(&b.cost).then_with(|| a.id.cmp(&b.id)));
        actions
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_data_loads() {
        let data = GameData::load().unwrap();
        assert_eq!(data.config.game_name, "mytherra");
        assert!(data.regions.len() >= 3);
        assert!(data.region_actions.contains("bless"));
        assert!(data.region_actions.contains("corrupt"));
        assert!(data.region_actions.contains("guide"));
    }

    #[test]
    fn region_actions_have_positive_cost() {
        let data = GameData::load().unwrap();
        for (_, action) in data.region_actions.iter() {
            assert!(action.cost > 0, "{} has non-positive cost", action.id);
        }
    }
}
