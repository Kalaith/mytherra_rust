//! Per-player projections of shared world state (§7.7).
//!
//! [`project`] is the server-authoritative filter: it takes the full
//! [`WorldState`] and a player's [`Standing`] and returns a [`WorldView`] that
//! contains only the entity classes that player has revealed — an un-unlocked
//! class arrives *empty*, not merely hidden, so a low-tier Watcher's payload is
//! genuinely small. A player's own [`PlayerView`] is never masked; it's private
//! to them.

use mytherra_core::capability::{BettingMarket, Standing, VisibilityScope as V};
use mytherra_core::data::GameData;
use mytherra_core::world::{
    Artifact, EraRecord, EraState, Hero, Landmark, MagicPath, Myth, MythCandidate, PantheonDeity,
    PlayerState, Region, RegionAgendas, ResourceNode, Settlement, SpeculationEvent, WorldEvent,
    WorldState, WorldSummary,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// How many recent chronicle events a player without `FullChronicle` receives.
const RECENT_EVENTS: usize = 32;

/// The full per-player payload a client polls (`GET /view`): its Standing-
/// filtered world view and its own private player view (§7.7). Shared so the
/// server serializes exactly what the client deserializes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientView {
    pub world: WorldView,
    pub player: PlayerView,
}

/// The chronicle change-delta and the new since-cursor (`GET /events?since=`,
/// §7.4): the events pushed since the client last acknowledged, plus the cursor
/// to send next time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventsDelta {
    pub events: Vec<WorldEvent>,
    pub cursor: u64,
}

/// A player's private view of their own deity — never filtered.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerView {
    pub player: PlayerState,
    pub standing: Standing,
    /// The favor ceiling and per-tick recovery at the player's current standing,
    /// pre-computed so the client needn't carry balance tables to display them.
    pub max_favor: i64,
    pub favor_recovery: i64,
}

/// The slice of shared world state a player's Standing reveals (§7.7). Every
/// collection is empty unless the matching [`VisibilityScope`] is unlocked;
/// [`revealed`](WorldView::revealed) records which, so the client can tell
/// "locked" from "genuinely empty".
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldView {
    pub year: u32,
    pub tick_count: u64,
    pub revealed: BTreeSet<V>,
    /// The world's overall tenor — always sent, even to a Watcher who can't see
    /// individual regions (GDD 10 dashboard).
    pub summary: WorldSummary,
    /// The current age — always sent; the full history is `Eras`-gated below.
    pub era: EraState,

    pub heroes: Vec<Hero>,
    pub regions: Vec<Region>,
    pub settlements: Vec<Settlement>,
    pub resource_nodes: Vec<ResourceNode>,
    pub landmarks: Vec<Landmark>,
    pub artifacts: Vec<Artifact>,
    pub magic_paths: Vec<MagicPath>,
    pub myths: Vec<Myth>,
    pub myth_candidates: Vec<MythCandidate>,
    pub civilization: Vec<RegionAgendas>,
    pub pantheon: Vec<PantheonDeity>,
    /// Open speculation events the player may wager on — filtered to the markets
    /// their Standing has unlocked (§5.9).
    pub speculations: Vec<SpeculationEvent>,
    pub era_history: Vec<EraRecord>,
    /// Newest-first: the whole chronicle with `FullChronicle`, else the most
    /// recent [`RECENT_EVENTS`].
    pub chronicle: Vec<WorldEvent>,
}

/// Project the shared world and a player onto what that player's [`Standing`]
/// reveals (§7.7). The server calls this per player, per poll.
pub fn project(
    world: &WorldState,
    player: &PlayerState,
    standing: &Standing,
    data: &GameData,
) -> (WorldView, PlayerView) {
    // Each collection is revealed only if its scope is unlocked, else sent empty.
    let heroes = if standing.can_see(V::Heroes) {
        world.heroes.clone()
    } else {
        Vec::new()
    };
    let regions = if standing.can_see(V::Regions) {
        world.regions.clone()
    } else {
        Vec::new()
    };
    let settlements = if standing.can_see(V::Settlements) {
        world.settlements.clone()
    } else {
        Vec::new()
    };
    let resource_nodes = if standing.can_see(V::Resources) {
        world.resource_nodes.clone()
    } else {
        Vec::new()
    };
    // Landmarks are region furniture — revealed with Regions.
    let landmarks = if standing.can_see(V::Regions) {
        world.landmarks.clone()
    } else {
        Vec::new()
    };
    // Artifacts / magic / myths / agendas are the divine-tools screen.
    let (artifacts, magic_paths, myths, myth_candidates, civilization) =
        if standing.can_see(V::DivineTools) {
            (
                world.artifacts.clone(),
                world.magic_paths.clone(),
                world.myths.clone(),
                world.myth_candidates.clone(),
                world.civilization.clone(),
            )
        } else {
            Default::default()
        };
    let pantheon = if standing.can_see(V::Pantheon) {
        world.pantheon.clone()
    } else {
        Vec::new()
    };
    let speculations = if standing.can_see(V::Observatory) {
        world
            .speculations
            .iter()
            .filter(|event| standing.can_bet(BettingMarket::of(event.predicate)))
            .cloned()
            .collect()
    } else {
        Vec::new()
    };
    let era_history = if standing.can_see(V::Eras) {
        world.era_history.clone()
    } else {
        Vec::new()
    };
    let chronicle = if standing.can_see(V::FullChronicle) {
        world.chronicle.iter_newest().cloned().collect()
    } else {
        world.chronicle.recent(RECENT_EVENTS).cloned().collect()
    };

    let view = WorldView {
        year: world.year,
        tick_count: world.tick_count,
        revealed: standing.scopes.clone(),
        summary: world.summary(),
        era: world.era.clone(),
        heroes,
        regions,
        settlements,
        resource_nodes,
        landmarks,
        artifacts,
        magic_paths,
        myths,
        myth_candidates,
        civilization,
        pantheon,
        speculations,
        era_history,
        chronicle,
    };

    let player_view = PlayerView {
        max_favor: player.max_favor(&data.config, &data.balance.player),
        favor_recovery: player.favor_recovery(&data.config, &data.balance.player),
        player: player.clone(),
        standing: standing.clone(),
    };

    (view, player_view)
}

#[cfg(test)]
mod tests {
    use super::*;
    use mytherra_core::capability::Tier;
    use mytherra_core::world::WorldState;

    fn fixtures() -> (GameData, WorldState, PlayerState) {
        let data = GameData::load().unwrap();
        let world = WorldState::new(&data);
        let player = PlayerState::new(&data.config);
        (data, world, player)
    }

    #[test]
    fn a_watcher_receives_heroes_but_no_regions() {
        let (data, world, player) = fixtures();
        let watcher = data.tiers.standing(Tier::Watcher);
        let (view, _) = project(&world, &player, &watcher, &data);
        assert!(!view.heroes.is_empty(), "a Watcher should see heroes");
        assert!(
            view.regions.is_empty(),
            "a Watcher has not unlocked regions"
        );
        assert!(view.pantheon.is_empty());
        // The aggregate tenor is always present, even without per-region access.
        assert!(view.summary.region_count > 0);
        assert!(!view.revealed.contains(&V::Regions));
    }

    #[test]
    fn an_elder_receives_the_whole_world() {
        let (data, world, player) = fixtures();
        let elder = data.tiers.standing(Tier::Elder);
        let (view, pv) = project(&world, &player, &elder, &data);
        assert!(!view.regions.is_empty());
        assert!(!view.heroes.is_empty());
        assert!(!view.pantheon.is_empty());
        assert!(view.revealed.contains(&V::FullChronicle));
        // The player's own favor ceiling comes through pre-computed.
        assert_eq!(pv.player.favor, player.favor);
        assert!(pv.max_favor > 0);
    }

    #[test]
    fn projection_serializes_to_json() {
        let (data, world, player) = fixtures();
        let patron = data.tiers.standing(Tier::Patron);
        let (view, pv) = project(&world, &player, &patron, &data);
        assert!(serde_json::to_string(&view).is_ok());
        assert!(serde_json::to_string(&pv).is_ok());
    }
}
