//! The shared world state: everything the simulation advances that is NOT
//! private to one player (GDD 6 "shared/global tables").

mod artifact;
mod bet;
mod building;
mod champion;
mod chronicle;
mod civilization;
mod consequence;
mod era;
mod hero;
mod house;
mod landmark;
mod magic;
mod monster;
mod myth;
mod pantheon;
mod plague;
mod player;
mod region;
mod resource;
mod settlement;
mod speculation;
mod trade;
mod war;
mod weather;

pub use artifact::Artifact;
pub use bet::{bet_record, quote_event, Bet};
pub use building::Building;
pub use champion::Champion;
pub use chronicle::{Chronicle, EventKind, WorldEvent};
pub use civilization::{agenda_score, dominant_agenda, spillover_target, RegionAgendas};
pub use consequence::{ConsequenceEffect, DelayedConsequence};
pub use era::{compute_scores, generate_era_name, pantheon_wrath, EraRecord, EraState};
pub use hero::Hero;
pub use house::House;
pub use landmark::Landmark;
pub use magic::{MagicPath, MagicState};
pub use monster::Monster;
pub use myth::{Myth, MythCandidate};
pub use pantheon::{adjust_pressure, PantheonDeity};
pub use plague::Plague;
pub use player::PlayerState;
pub use region::{resident_might, Region, RegionStatus};
pub use resource::ResourceNode;
pub use settlement::Settlement;
pub use speculation::SpeculationEvent;
pub use trade::TradeRoute;
pub use war::War;
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
    /// Average per-region change over the last tick, for the dashboard arrows.
    pub trend_prosperity: f32,
    pub trend_chaos: f32,
    pub trend_danger: f32,
    pub trend_magic: f32,
    pub total_population: f32,
    pub regions_in_crisis: usize,
}

impl WorldSummary {
    /// The world's qualitative tenor as a bucket index, from 0 (a golden age) up
    /// to `thresholds.len()` (a dark age). Health is prosperity minus the forces
    /// that trouble a world; the descending `thresholds` bucket it. Pure — the UI
    /// maps the index to a label (GDD 10).
    pub fn tenor(&self, thresholds: &[f32], crisis_penalty: f32) -> usize {
        let health = self.avg_prosperity
            - self.avg_danger
            - self.avg_chaos
            - self.regions_in_crisis as f32 * crisis_penalty;
        thresholds.iter().filter(|&&t| health < t).count()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldState {
    pub year: u32,
    pub tick_count: u64,
    pub regions: Vec<Region>,
    /// Monotonic counter for unique ids of regions born mid-run (breakaways,
    /// GDD 5.2), so no two fractured regions ever collide.
    #[serde(default)]
    pub region_seq: u64,
    /// Decaying tally of recent region conquests (GDD 5.2 ↔ 5.7): each genesis
    /// conquest bumps it and it bleeds off over time, feeding the era system's
    /// Conquest pressure so an age of realms devouring each other is defined by
    /// the conquests themselves, not just ambient danger.
    #[serde(default)]
    pub conquest_momentum: f32,
    /// Decaying tally of recent region fractures (GDD 5.2 ↔ 5.7): each secession
    /// bumps it and it bleeds off over time, feeding the era system's Collapse
    /// pressure so a world coming apart from within builds toward a Collapse age,
    /// distinct from one devoured by conquest.
    #[serde(default)]
    pub secession_momentum: f32,
    pub settlements: Vec<Settlement>,
    /// Monotonic counter for unique ids of towns founded mid-run (GDD 5.3), so no
    /// two founded settlements ever collide.
    #[serde(default)]
    pub settlement_seq: u64,
    /// Monotonic counter for unique ids of wonders raised mid-run (GDD 5.2).
    #[serde(default)]
    pub landmark_seq: u64,
    pub resource_nodes: Vec<ResourceNode>,
    /// Monotonic counter for unique ids of resource nodes discovered mid-run
    /// (GDD 5.3), so no two prospected nodes ever collide.
    #[serde(default)]
    pub resource_seq: u64,
    pub landmarks: Vec<Landmark>,
    pub trade_routes: Vec<TradeRoute>,
    /// Monotonic counter for unique ids of trade routes forged mid-run (GDD 5.2),
    /// so no two founded routes ever collide.
    #[serde(default)]
    pub trade_seq: u64,
    pub buildings: Vec<Building>,
    pub heroes: Vec<Hero>,
    /// Monotonic counter for unique descendant-hero ids.
    pub hero_seq: u64,
    /// The noble houses the world's legends have founded (GDD 5.4); arise
    /// dynamically, so this starts empty on a fresh world.
    #[serde(default)]
    pub houses: Vec<House>,
    /// Monotonic counter for unique ids of houses founded mid-run.
    #[serde(default)]
    pub house_seq: u64,
    pub artifacts: Vec<Artifact>,
    /// Monotonic counter for unique created-artifact ids.
    pub artifact_seq: u64,
    /// Scheduled aftermath steps of artifact backlashes (GDD 5.6).
    #[serde(default)]
    pub pending_consequences: Vec<DelayedConsequence>,
    pub era: EraState,
    pub era_history: Vec<EraRecord>,
    pub weather: Vec<WeatherEvent>,
    /// Active plagues gripping regions (GDD 5.3); arise dynamically, so this
    /// starts empty on a fresh world.
    #[serde(default)]
    pub plagues: Vec<Plague>,
    /// Monotonic counter for unique ids of plagues that break out mid-run.
    #[serde(default)]
    pub plague_seq: u64,
    /// Beasts stalking regions (GDD 5.2); emerge dynamically, so this starts
    /// empty on a fresh world.
    #[serde(default)]
    pub monsters: Vec<Monster>,
    /// Monotonic counter for unique ids of beasts that emerge mid-run.
    #[serde(default)]
    pub monster_seq: u64,
    /// Wars raging between regions (GDD 5.2); ignite dynamically, so this starts
    /// empty on a fresh world.
    #[serde(default)]
    pub wars: Vec<War>,
    /// Monotonic counter for unique ids of wars that break out mid-run.
    #[serde(default)]
    pub war_seq: u64,
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
            name: era::generate_era_name(&data.era_names, None, &mut rng),
            start_year: data.config.start_year,
            dominant_trigger: crate::data::EraTrigger::Cataclysm,
            pressure: 0.0,
        };
        let mut world = Self {
            year: data.config.start_year,
            tick_count: 0,
            regions,
            region_seq: 0,
            conquest_momentum: 0.0,
            secession_momentum: 0.0,
            settlements,
            resource_nodes,
            resource_seq: 0,
            landmarks,
            trade_routes,
            trade_seq: 0,
            buildings,
            heroes,
            settlement_seq: 0,
            landmark_seq: 0,
            hero_seq: 0,
            houses: Vec::new(),
            house_seq: 0,
            artifacts,
            artifact_seq: 0,
            pending_consequences: Vec::new(),
            era,
            era_history: Vec::new(),
            weather: Vec::new(),
            plagues: Vec::new(),
            plague_seq: 0,
            monsters: Vec::new(),
            monster_seq: 0,
            wars: Vec::new(),
            war_seq: 0,
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
            summary.trend_prosperity += region.prosperity - region.prev.prosperity;
            summary.trend_chaos += region.chaos - region.prev.chaos;
            summary.trend_danger += region.danger - region.prev.danger;
            summary.trend_magic += region.magic_affinity - region.prev.magic_affinity;
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
        summary.trend_prosperity /= n;
        summary.trend_chaos /= n;
        summary.trend_danger /= n;
        summary.trend_magic /= n;
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

    #[test]
    fn tenor_worsens_as_the_world_darkens() {
        let thresholds = [60.0, 35.0, 15.0, -10.0];
        let penalty = 12.0;
        let with = |prosperity: f32, danger: f32, chaos: f32, crises: usize| WorldSummary {
            avg_prosperity: prosperity,
            avg_danger: danger,
            avg_chaos: chaos,
            regions_in_crisis: crises,
            ..Default::default()
        };

        // A calm, rich world reads as a golden age (health 90 clears every bar).
        assert_eq!(with(95.0, 3.0, 2.0, 0).tenor(&thresholds, penalty), 0);
        // A troubled, crisis-stricken world sinks toward a dark age.
        let dark = with(20.0, 80.0, 70.0, 3).tenor(&thresholds, penalty);
        assert_eq!(dark, thresholds.len(), "a broken world is a dark age");
        // And the tenor is monotonic: more turmoil never improves the age.
        assert!(
            with(60.0, 40.0, 40.0, 1).tenor(&thresholds, penalty)
                >= with(80.0, 10.0, 10.0, 0).tenor(&thresholds, penalty)
        );
    }
}
