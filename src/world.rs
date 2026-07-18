//! The shared world state: everything the simulation advances that is NOT
//! private to one player (GDD 6 "shared/global tables").

mod artifact;
mod bet;
mod champion;
mod chronicle;
mod hero;
mod player;
mod region;
mod speculation;

pub use artifact::Artifact;
pub use bet::{quote_event, Bet};
pub use champion::Champion;
pub use chronicle::{Chronicle, EventKind};
pub use hero::Hero;
pub use player::PlayerState;
pub use region::{Region, RegionStatus};
pub use speculation::SpeculationEvent;

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
    pub heroes: Vec<Hero>,
    pub artifacts: Vec<Artifact>,
    /// Monotonic counter for unique created-artifact ids.
    pub artifact_seq: u64,
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
        let artifacts = data.artifacts.iter().map(Artifact::from_seed).collect();
        let mut world = Self {
            year: data.config.start_year,
            tick_count: 0,
            regions,
            heroes,
            artifacts,
            artifact_seq: 0,
            speculations: Vec::new(),
            speculation_seq: 0,
            chronicle: Chronicle::default(),
            rng: SeededRng::new(data.config.world_seed),
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
