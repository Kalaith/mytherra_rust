//! Embedded, data-driven game content loaded from `assets/data/*.json`.
//!
//! Content and balance values live in JSON per the RustGames data-driven rule;
//! Rust only describes their shape. Native builds could later read these from
//! disk, but the embedded copies keep WASM builds self-contained.

mod action;
mod artifact;
mod balance;
mod bet;
mod champion;
mod config;
mod hero;
mod magic;
mod region;
pub mod strings;
mod weather;

pub use action::RegionActionDef;
pub use artifact::{ArtifactFocus, ArtifactSeed};
pub use balance::{
    ArtifactBalance, Balance, BettingBalance, ChampionBalance, HeroBalance, MagicBalance,
    PlayerBalance, RegionBalance, WeatherBalance,
};
pub use bet::{BetPredicate, BetType, ConfidenceLevel, TargetKind, TimeframeModifier};
pub use champion::ChampionFocus;
pub use config::GameConfig;
pub use hero::{HeroRole, HeroSeed};
pub use magic::{MagicPathSeed, MagicStat};
pub use region::{ClimateType, Culture, RegionSeed};
pub use strings::{fill, Strings};
pub use weather::{WeatherIntensity, WeatherPattern};

use macroquad_toolkit::data_loader::{
    load_embedded_json, load_embedded_json_labeled, DataRegistry,
};

const GAME_CONFIG_JSON: &str = include_str!("../assets/data/game_config.json");
const REGIONS_JSON: &str = include_str!("../assets/data/regions.json");
const REGION_ACTIONS_JSON: &str = include_str!("../assets/data/region_actions.json");
const HEROES_JSON: &str = include_str!("../assets/data/heroes.json");
const ARTIFACTS_JSON: &str = include_str!("../assets/data/artifacts.json");
const WEATHER_PATTERNS_JSON: &str = include_str!("../assets/data/weather_patterns.json");
const WEATHER_INTENSITIES_JSON: &str = include_str!("../assets/data/weather_intensities.json");
const MAGIC_PATHS_JSON: &str = include_str!("../assets/data/magic_paths.json");
const BET_TYPES_JSON: &str = include_str!("../assets/data/bet_types.json");
const CONFIDENCE_JSON: &str = include_str!("../assets/data/confidence_levels.json");
const TIMEFRAMES_JSON: &str = include_str!("../assets/data/timeframe_modifiers.json");
const BALANCE_JSON: &str = include_str!("../assets/data/balance.json");
const STRINGS_JSON: &str = include_str!("../assets/data/strings.json");

/// All static content the game needs, resolved once at boot.
#[derive(Debug, Clone)]
pub struct GameData {
    pub config: GameConfig,
    pub regions: Vec<RegionSeed>,
    pub region_actions: DataRegistry<RegionActionDef>,
    pub heroes: Vec<HeroSeed>,
    pub artifacts: Vec<ArtifactSeed>,
    pub weather_patterns: Vec<WeatherPattern>,
    pub weather_intensities: Vec<WeatherIntensity>,
    pub magic_paths: Vec<MagicPathSeed>,
    pub bet_types: Vec<BetType>,
    pub confidence_levels: Vec<ConfidenceLevel>,
    pub timeframes: Vec<TimeframeModifier>,
    pub balance: Balance,
    pub strings: Strings,
}

impl GameData {
    pub fn load() -> Result<Self, String> {
        let config = load_embedded_json_labeled("game_config", GAME_CONFIG_JSON)?;
        let regions: Vec<RegionSeed> = load_embedded_json(REGIONS_JSON)?;
        let region_actions = DataRegistry::from_embedded_json(REGION_ACTIONS_JSON, "id")?;
        let heroes: Vec<HeroSeed> = load_embedded_json(HEROES_JSON)?;
        let artifacts: Vec<ArtifactSeed> = load_embedded_json(ARTIFACTS_JSON)?;
        let weather_patterns: Vec<WeatherPattern> = load_embedded_json(WEATHER_PATTERNS_JSON)?;
        let weather_intensities: Vec<WeatherIntensity> =
            load_embedded_json(WEATHER_INTENSITIES_JSON)?;
        let magic_paths: Vec<MagicPathSeed> = load_embedded_json(MAGIC_PATHS_JSON)?;
        let bet_types: Vec<BetType> = load_embedded_json(BET_TYPES_JSON)?;
        let confidence_levels: Vec<ConfidenceLevel> = load_embedded_json(CONFIDENCE_JSON)?;
        let timeframes: Vec<TimeframeModifier> = load_embedded_json(TIMEFRAMES_JSON)?;
        let balance: Balance = load_embedded_json_labeled("balance", BALANCE_JSON)?;
        let strings: Strings = load_embedded_json_labeled("strings", STRINGS_JSON)?;

        if regions.is_empty() {
            return Err("regions.json contained no regions".to_owned());
        }
        if bet_types.is_empty() || confidence_levels.is_empty() || timeframes.is_empty() {
            return Err("betting config tables must not be empty".to_owned());
        }
        if weather_patterns.is_empty() || weather_intensities.is_empty() {
            return Err("weather config tables must not be empty".to_owned());
        }

        Ok(Self {
            config,
            regions,
            region_actions,
            heroes,
            artifacts,
            weather_patterns,
            weather_intensities,
            magic_paths,
            bet_types,
            confidence_levels,
            timeframes,
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
