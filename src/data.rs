//! Embedded, data-driven game content loaded from `assets/data/*.json`.
//!
//! Content and balance values live in JSON per the RustGames data-driven rule;
//! Rust only describes their shape. Native builds could later read these from
//! disk, but the embedded copies keep WASM builds self-contained.

mod action;
mod artifact;
mod balance;
mod bet;
mod building;
mod champion;
mod civilization;
mod config;
mod era;
mod hero;
mod landmark;
mod magic;
mod myth;
mod pantheon;
mod region;
mod resource;
mod settlement;
pub mod strings;
mod trade;
mod weather;

pub use action::RegionActionDef;
pub use artifact::{ArtifactFocus, ArtifactSeed};
pub use balance::{
    ArtifactBalance, Balance, BettingBalance, ChampionBalance, CivilizationBalance,
    ConquestBalance, CultureBalance, EraBalance, FrontierBalance, GenesisBalance, HeroBalance,
    HeroMightWeights, MagicBalance, MigrationBalance, MythBalance, PantheonBalance, PlagueBalance,
    PlayerBalance, RegionBalance, ResourceBalance, ResourceOutputs, SettlementBalance,
    TradeBalance, WeatherBalance,
};
pub use bet::{BetPredicate, BetType, ConfidenceLevel, TargetKind, TimeframeModifier};
pub use building::{BuildingSeed, BuildingType};
pub use champion::ChampionFocus;
pub use civilization::{Agenda, CivStat, SpilloverTarget};
pub use config::GameConfig;
pub use era::{EraNameBank, EraTrigger};
pub use hero::{HeroNameBank, HeroRole, HeroSeed};
pub use landmark::{LandmarkNameBank, LandmarkSeed};
pub use magic::{MagicPathSeed, MagicStat};
pub use myth::{MythStat, MythTheme};
pub use pantheon::{DeitySeed, PantheonStat};
pub use region::{ClimateType, Culture, RegionSeed};
pub use resource::{ResourceNodeSeed, ResourceStatus, ResourceType};
pub use settlement::{SettlementNameBank, SettlementSeed};
pub use strings::{fill, Strings};
pub use trade::TradeRouteSeed;
pub use weather::{WeatherIntensity, WeatherPattern};

use macroquad_toolkit::data_loader::{
    load_embedded_json, load_embedded_json_labeled, DataRegistry,
};

const GAME_CONFIG_JSON: &str = include_str!("../assets/data/game_config.json");
const REGIONS_JSON: &str = include_str!("../assets/data/regions.json");
const REGION_ACTIONS_JSON: &str = include_str!("../assets/data/region_actions.json");
const HEROES_JSON: &str = include_str!("../assets/data/heroes.json");
const HERO_NAMES_JSON: &str = include_str!("../assets/data/hero_names.json");
const SETTLEMENTS_JSON: &str = include_str!("../assets/data/settlements.json");
const SETTLEMENT_NAMES_JSON: &str = include_str!("../assets/data/settlement_names.json");
const RESOURCE_NODES_JSON: &str = include_str!("../assets/data/resource_nodes.json");
const LANDMARKS_JSON: &str = include_str!("../assets/data/landmarks.json");
const LANDMARK_NAMES_JSON: &str = include_str!("../assets/data/landmark_names.json");
const PLAGUE_NAMES_JSON: &str = include_str!("../assets/data/plague_names.json");
const TRADE_ROUTES_JSON: &str = include_str!("../assets/data/trade_routes.json");
const BUILDING_TYPES_JSON: &str = include_str!("../assets/data/building_types.json");
const BUILDINGS_JSON: &str = include_str!("../assets/data/buildings.json");
const ARTIFACTS_JSON: &str = include_str!("../assets/data/artifacts.json");
const WEATHER_PATTERNS_JSON: &str = include_str!("../assets/data/weather_patterns.json");
const WEATHER_INTENSITIES_JSON: &str = include_str!("../assets/data/weather_intensities.json");
const MAGIC_PATHS_JSON: &str = include_str!("../assets/data/magic_paths.json");
const MYTH_THEMES_JSON: &str = include_str!("../assets/data/myth_themes.json");
const AGENDAS_JSON: &str = include_str!("../assets/data/agendas.json");
const PANTHEON_JSON: &str = include_str!("../assets/data/pantheon.json");
const ERA_NAMES_JSON: &str = include_str!("../assets/data/era_names.json");
const BET_TYPES_JSON: &str = include_str!("../assets/data/bet_types.json");
const CONFIDENCE_JSON: &str = include_str!("../assets/data/confidence_levels.json");
const TIMEFRAMES_JSON: &str = include_str!("../assets/data/timeframe_modifiers.json");
const BALANCE_JSON: &str = include_str!("../assets/data/balance.json");
const STRINGS_JSON: &str = include_str!("../assets/data/strings.json");
const ACHIEVEMENTS_JSON: &str = include_str!("../assets/data/achievements.json");

/// All static content the game needs, resolved once at boot.
#[derive(Debug, Clone)]
pub struct GameData {
    pub config: GameConfig,
    pub regions: Vec<RegionSeed>,
    pub region_actions: DataRegistry<RegionActionDef>,
    pub heroes: Vec<HeroSeed>,
    pub hero_names: HeroNameBank,
    pub settlements: Vec<SettlementSeed>,
    pub settlement_names: SettlementNameBank,
    pub resource_nodes: Vec<ResourceNodeSeed>,
    pub landmarks: Vec<LandmarkSeed>,
    pub landmark_names: LandmarkNameBank,
    /// Pestilence name bank — plagues arise dynamically, so they need no seed
    /// content, only names to draw from (GDD 5.3).
    pub plague_names: Vec<String>,
    pub trade_routes: Vec<TradeRouteSeed>,
    pub building_types: DataRegistry<BuildingType>,
    pub buildings: Vec<BuildingSeed>,
    pub artifacts: Vec<ArtifactSeed>,
    pub weather_patterns: Vec<WeatherPattern>,
    pub weather_intensities: Vec<WeatherIntensity>,
    pub magic_paths: Vec<MagicPathSeed>,
    pub myth_themes: Vec<MythTheme>,
    pub agendas: Vec<Agenda>,
    pub pantheon: Vec<DeitySeed>,
    pub era_names: EraNameBank,
    pub bet_types: Vec<BetType>,
    pub confidence_levels: Vec<ConfidenceLevel>,
    pub timeframes: Vec<TimeframeModifier>,
    pub balance: Balance,
    pub strings: Strings,
    /// Achievement definitions (unlock state lives in the player's save).
    pub achievements: Vec<macroquad_toolkit::achievements::Achievement>,
}

impl GameData {
    pub fn load() -> Result<Self, String> {
        let config = load_embedded_json_labeled("game_config", GAME_CONFIG_JSON)?;
        let regions: Vec<RegionSeed> = load_embedded_json(REGIONS_JSON)?;
        let region_actions = DataRegistry::from_embedded_json(REGION_ACTIONS_JSON, "id")?;
        let heroes: Vec<HeroSeed> = load_embedded_json(HEROES_JSON)?;
        let hero_names: HeroNameBank = load_embedded_json_labeled("hero_names", HERO_NAMES_JSON)?;
        let settlements: Vec<SettlementSeed> = load_embedded_json(SETTLEMENTS_JSON)?;
        let settlement_names: SettlementNameBank =
            load_embedded_json_labeled("settlement_names", SETTLEMENT_NAMES_JSON)?;
        let resource_nodes: Vec<ResourceNodeSeed> = load_embedded_json(RESOURCE_NODES_JSON)?;
        let landmarks: Vec<LandmarkSeed> = load_embedded_json(LANDMARKS_JSON)?;
        let landmark_names: LandmarkNameBank =
            load_embedded_json_labeled("landmark_names", LANDMARK_NAMES_JSON)?;
        let plague_names: Vec<String> = load_embedded_json(PLAGUE_NAMES_JSON)?;
        let trade_routes: Vec<TradeRouteSeed> = load_embedded_json(TRADE_ROUTES_JSON)?;
        let building_types = DataRegistry::from_embedded_json(BUILDING_TYPES_JSON, "id")?;
        let buildings: Vec<BuildingSeed> = load_embedded_json(BUILDINGS_JSON)?;
        let artifacts: Vec<ArtifactSeed> = load_embedded_json(ARTIFACTS_JSON)?;
        let weather_patterns: Vec<WeatherPattern> = load_embedded_json(WEATHER_PATTERNS_JSON)?;
        let weather_intensities: Vec<WeatherIntensity> =
            load_embedded_json(WEATHER_INTENSITIES_JSON)?;
        let magic_paths: Vec<MagicPathSeed> = load_embedded_json(MAGIC_PATHS_JSON)?;
        let myth_themes: Vec<MythTheme> = load_embedded_json(MYTH_THEMES_JSON)?;
        let agendas: Vec<Agenda> = load_embedded_json(AGENDAS_JSON)?;
        let pantheon: Vec<DeitySeed> = load_embedded_json(PANTHEON_JSON)?;
        let era_names: EraNameBank = load_embedded_json_labeled("era_names", ERA_NAMES_JSON)?;
        let bet_types: Vec<BetType> = load_embedded_json(BET_TYPES_JSON)?;
        let confidence_levels: Vec<ConfidenceLevel> = load_embedded_json(CONFIDENCE_JSON)?;
        let timeframes: Vec<TimeframeModifier> = load_embedded_json(TIMEFRAMES_JSON)?;
        let balance: Balance = load_embedded_json_labeled("balance", BALANCE_JSON)?;
        let strings: Strings = load_embedded_json_labeled("strings", STRINGS_JSON)?;
        let achievements: Vec<macroquad_toolkit::achievements::Achievement> =
            load_embedded_json(ACHIEVEMENTS_JSON)?;

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
            hero_names,
            settlements,
            settlement_names,
            resource_nodes,
            landmarks,
            landmark_names,
            plague_names,
            trade_routes,
            building_types,
            buildings,
            artifacts,
            weather_patterns,
            weather_intensities,
            magic_paths,
            myth_themes,
            agendas,
            pantheon,
            era_names,
            bet_types,
            confidence_levels,
            timeframes,
            balance,
            strings,
            achievements,
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
