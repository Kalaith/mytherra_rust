//! The shared world state: everything the simulation advances that is NOT
//! private to one player (GDD 6 "shared/global tables").

mod chronicle;
mod player;
mod region;

pub use chronicle::{Chronicle, EventKind};
pub use player::PlayerState;
pub use region::{Region, RegionStatus};

use crate::data::GameData;
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
    pub chronicle: Chronicle,
}

impl WorldState {
    /// Build a fresh world from seed content.
    pub fn new(data: &GameData) -> Self {
        let regions = data.regions.iter().map(Region::from_seed).collect();
        let mut world = Self {
            year: data.config.start_year,
            tick_count: 0,
            regions,
            chronicle: Chronicle::default(),
        };
        world
            .chronicle
            .push(world.year, EventKind::System, "The world awakens.");
        world
    }

    pub fn region(&self, index: usize) -> Option<&Region> {
        self.regions.get(index)
    }

    pub fn region_mut(&mut self, index: usize) -> Option<&mut Region> {
        self.regions.get_mut(index)
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
