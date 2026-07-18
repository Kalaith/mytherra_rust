//! The shared world state: everything the simulation advances that is NOT
//! private to one player (GDD 6 "shared/global tables").

mod artifact;
mod bet;
mod building;
mod champion;
mod chronicle;
mod civilization;
mod era;
mod hero;
mod landmark;
mod magic;
mod myth;
mod pantheon;
mod player;
mod region;
mod resource;
mod settlement;
mod speculation;
mod trade;
mod weather;

pub use artifact::Artifact;
pub use bet::{quote_event, Bet};
pub use building::Building;
pub use champion::Champion;
pub use chronicle::{Chronicle, EventKind};
pub use civilization::{agenda_score, RegionAgendas};
pub use era::{compute_scores, generate_era_name, EraRecord, EraState};
pub use hero::Hero;
pub use landmark::Landmark;
pub use magic::{MagicPath, MagicState};
pub use myth::{Myth, MythCandidate};
pub use pantheon::{adjust_pressure, PantheonDeity};
pub use player::PlayerState;
pub use region::{Region, RegionStatus};
pub use resource::ResourceNode;
pub use settlement::Settlement;
pub use speculation::SpeculationEvent;
pub use trade::TradeRoute;
pub use weather::{weather_cost, WeatherEvent};

use crate::data::GameData;
use macroquad_toolkit::rng::SeededRng;
use serde::{Deserialize, Serialize};

/// Aggregate, read-only snapshot of the world's regions for dashboards.
#[derive(Debug, Clone, Copy, Default)]
pub struct WorldSummary {
    pub region_count: usize,
    pub avg_prosperity: f32,
    pub avg_chaos: f32,
    pub avg_danger: f32,
    pub avg_magic: f32,
    pub total_population: f32,
    pub regions_in_crisis: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldState {
    pub year: u32,
    pub tick_count: u64,
    pub regions: Vec<Region>,
    pub settlements: Vec<Settlement>,
    pub resource_nodes: Vec<ResourceNode>,
    pub landmarks: Vec<Landmark>,
    pub trade_routes: Vec<TradeRoute>,
    pub buildings: Vec<Building>,
    pub heroes: Vec<Hero>,
    /// Monotonic counter for unique descendant-hero ids.
    pub hero_seq: u64,
    pub artifacts: Vec<Artifact>,
    /// Monotonic counter for unique created-artifact ids.
    pub artifact_seq: u64,
    pub era: EraState,
    pub era_history: Vec<EraRecord>,
    pub weather: Vec<WeatherEvent>,
    pub magic_paths: Vec<MagicPath>,
    pub myths: Vec<Myth>,
    pub myth_candidates: Vec<MythCandidate>,
    /// Monotonic counter for unique myth ids.
    pub myth_seq: u64,
    pub civilization: Vec<RegionAgendas>,
    pub pantheon: Vec<PantheonDeity>,
    pub speculations: Vec<SpeculationEvent>,
    /// Monotonic counter for unique speculation event ids.
    pub speculation_seq: u64,
    pub chronicle: Chronicle,
    /// The world's own deterministic RNG (GDD 5.8); serialized so saves resume
    /// the exact same sequence.
    pub rng: SeededRng,
}

impl WorldState {
    /// Build a fresh world from seed content.
    pub fn new(data: &GameData) -> Self {
        let regions = data
            .regions
            .iter()
            .map(|seed| Region::from_seed(seed, &data.balance.region))
            .collect();
        let heroes = data.heroes.iter().map(Hero::from_seed).collect();
        let settlements = data.settlements.iter().map(Settlement::from_seed).collect();
        let resource_nodes = data
            .resource_nodes
            .iter()
            .map(ResourceNode::from_seed)
            .collect();
        let landmarks = data.landmarks.iter().map(Landmark::from_seed).collect();
        let trade_routes = data
            .trade_routes
            .iter()
            .map(TradeRoute::from_seed)
            .collect();
        let buildings = data
            .buildings
            .iter()
            .map(|seed| Building::from_seed(seed, &data.building_types))
            .collect();
        let artifacts = data.artifacts.iter().map(Artifact::from_seed).collect();
        let magic_paths = data.magic_paths.iter().map(MagicPath::from_seed).collect();
        let civilization = data
            .regions
            .iter()
            .map(|seed| RegionAgendas::new(seed.id.clone(), data.agendas.len()))
            .collect();
        let pantheon = data.pantheon.iter().map(PantheonDeity::from_seed).collect();
        let mut rng = SeededRng::new(data.config.world_seed);
        let era = EraState {
            number: 1,
            name: era::generate_era_name(&data.era_names, &mut rng),
            start_year: data.config.start_year,
            dominant_trigger: crate::data::EraTrigger::Cataclysm,
            pressure: 0.0,
        };
        let mut world = Self {
            year: data.config.start_year,
            tick_count: 0,
            regions,
            settlements,
            resource_nodes,
            landmarks,
            trade_routes,
            buildings,
            heroes,
            hero_seq: 0,
            artifacts,
            artifact_seq: 0,
            era,
            era_history: Vec::new(),
            weather: Vec::new(),
            magic_paths,
            myths: Vec::new(),
            myth_candidates: Vec::new(),
            myth_seq: 0,
            civilization,
            pantheon,
            speculations: Vec::new(),
            speculation_seq: 0,
            chronicle: Chronicle::default(),
            rng,
        };
        world.chronicle.push(
            world.year,
            EventKind::System,
            data.strings.chronicle.world_awakens.clone(),
        );
        world
    }

    pub fn region(&self, index: usize) -> Option<&Region> {
        self.regions.get(index)
    }

    pub fn region_mut(&mut self, index: usize) -> Option<&mut Region> {
        self.regions.get_mut(index)
    }

    /// Count of living heroes.
    pub fn living_heroes(&self) -> usize {
        self.heroes.iter().filter(|h| h.is_alive).count()
    }

    /// Look up a region's display name by id (for hero/UI cross-references).
    pub fn region_name(&self, id: &str) -> Option<&str> {
        self.regions
            .iter()
            .find(|r| r.id == id)
            .map(|r| r.name.as_str())
    }

    /// Aggregate region stats for the dashboard.
    pub fn summary(&self) -> WorldSummary {
        let count = self.regions.len();
        if count == 0 {
            return WorldSummary::default();
        }
        let mut summary = WorldSummary {
            region_count: count,
            ..Default::default()
        };
        for region in &self.regions {
            summary.avg_prosperity += region.prosperity;
            summary.avg_chaos += region.chaos;
            summary.avg_danger += region.danger;
            summary.avg_magic += region.magic_affinity;
            summary.total_population += region.population;
            if region.status.is_crisis() {
                summary.regions_in_crisis += 1;
            }
        }
        let n = count as f32;
        summary.avg_prosperity /= n;
        summary.avg_chaos /= n;
        summary.avg_danger /= n;
        summary.avg_magic /= n;
        summary
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_world_seeds_regions_and_year() {
        let data = GameData::load().unwrap();
        let world = WorldState::new(&data);
        assert_eq!(world.year, data.config.start_year);
        assert_eq!(world.regions.len(), data.regions.len());
        assert!(!world.chronicle.is_empty());
    }

    #[test]
    fn summary_averages_region_stats() {
        let data = GameData::load().unwrap();
        let world = WorldState::new(&data);
        let summary = world.summary();
        assert_eq!(summary.region_count, world.regions.len());
        assert!(summary.avg_prosperity > 0.0);
        assert!(summary.total_population > 0.0);
    }
}
