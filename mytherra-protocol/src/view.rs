//! Per-player projections of shared world state (§7.7).
//!
//! [`project`] is the server-authoritative filter: it takes the full
//! [`WorldState`] and a player's [`Standing`] and returns a [`WorldView`] that
//! contains only the entity classes that player has revealed — an un-unlocked
//! class arrives *empty*, not merely hidden, so a low-tier Watcher's payload is
//! genuinely small. A player's own [`PlayerView`] is never masked; it's private
//! to them.

use mytherra_core::capability::{BettingMarket, Standing, Tier, VisibilityScope as V};
use mytherra_core::data::GameData;
use mytherra_core::world::{
    Artifact, Building, DelayedConsequence, EraRecord, EraState, EventKind, Hero, House, Landmark,
    MagicPath, Monster, Myth, MythCandidate, Order, Pact, PantheonDeity, Plague, PlayerState,
    Region, RegionAgendas, ResourceNode, Settlement, SpeculationEvent, TradeRoute, Vassalage, War,
    WeatherEvent, WorldEvent, WorldState, WorldSummary,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// How many recent chronicle events a player without `FullChronicle` receives.
const RECENT_EVENTS: usize = 32;

/// The server's reply to `POST /session` (and `POST /link`): the deity id the
/// client then presents (as `X-Player-Id`) on every later request (GDD 7.7).
/// Each connected deity has its own id, its own favor, and its own Standing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionResponse {
    pub player_id: String,
    /// Whether this deity is bound to a WebHatchery account (GDD 7.3) — `true`
    /// after a successful link or an account resume, `false` for a pure guest.
    /// Defaults so an older client (or a pre-linking server) still deserializes.
    #[serde(default)]
    pub linked: bool,
}

/// The client's entry-point config (`GET /login-info`): where to send a player to
/// sign in to WebHatchery so a guest deity can be linked for cross-device
/// continuity (GDD 7.3). The server reads the URL from its environment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginInfo {
    pub login_url: String,
}

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
    /// `favor_recovery` is the *total* per-tick income — passive recovery plus
    /// the faith tithe from the full world — so the client renders the real
    /// figure without reconstructing the tithe from a view that may hide regions.
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
    /// The noble houses and great Orders the world's legends have raised (GDD
    /// 5.4). Both are hero-derived — a house is a bloodline of heroes, an Order
    /// the living fellowship of a role — so both are revealed with `Heroes`
    /// rather than carrying a scope of their own.
    pub houses: Vec<House>,
    pub orders: Vec<Order>,
    pub regions: Vec<Region>,
    pub settlements: Vec<Settlement>,
    pub resource_nodes: Vec<ResourceNode>,
    pub landmarks: Vec<Landmark>,
    // Region furniture the detail views read — revealed with `Regions` (a
    // Watcher sees no regions, so none of this either).
    pub buildings: Vec<Building>,
    pub trade_routes: Vec<TradeRoute>,
    pub weather: Vec<WeatherEvent>,
    pub plagues: Vec<Plague>,
    pub monsters: Vec<Monster>,
    pub wars: Vec<War>,
    pub pacts: Vec<Pact>,
    pub vassalages: Vec<Vassalage>,
    pub pending_consequences: Vec<DelayedConsequence>,
    /// Decaying tallies of recent conquests/secessions feeding era pressure —
    /// aggregate scalars (like `summary`), always sent (GDD 5.2 ↔ 5.7).
    pub conquest_momentum: f32,
    pub secession_momentum: f32,
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

impl WorldView {
    /// The revealed region at `index`, if any (mirrors `WorldState::region`).
    pub fn region(&self, index: usize) -> Option<&Region> {
        self.regions.get(index)
    }

    /// A revealed region's display name by id (for hero/UI cross-references).
    pub fn region_name(&self, id: &str) -> Option<&str> {
        self.regions
            .iter()
            .find(|r| r.id == id)
            .map(|r| r.name.as_str())
    }

    /// Count of living heroes in the revealed roster.
    pub fn living_heroes(&self) -> usize {
        self.heroes.iter().filter(|h| h.is_alive).count()
    }
}

/// The Standing a read-only spectator holds: every visibility scope, and
/// deliberately no verbs and no betting markets. It is the Standing behind the
/// server's spectator endpoints — a client that reads the whole world without
/// owning a deity (GDD 7.7 ↔ the exchange's world source). Because the verb and
/// market sets are empty, a spectator projection can authorize no action and
/// surfaces no wagers: `speculations` filters to empty even though `Observatory`
/// is revealed, since `can_bet` is false for every market.
pub fn spectator_standing() -> Standing {
    Standing {
        // It sees what an Elder sees, so it reports an Elder's rank; the empty
        // verb set is what actually makes it powerless.
        tier: Tier::Elder.rank(),
        scopes: V::ALL.into_iter().collect(),
        verbs: BTreeSet::new(),
        markets: BTreeSet::new(),
    }
}

/// Project the shared world onto what a [`Standing`] reveals (§7.7), with no
/// player attached. Split out of [`project`] so a spectator — which has no
/// deity, no favor, and no `PlayerState` — can be served the same filtered world
/// through the same one code path.
pub fn project_world(world: &WorldState, standing: &Standing) -> WorldView {
    // Each collection is revealed only if its scope is unlocked, else sent empty.
    let heroes = if standing.can_see(V::Heroes) {
        world.heroes.clone()
    } else {
        Vec::new()
    };
    // Houses and Orders are hero-derived, so they ride with the hero roster.
    let (houses, orders) = if standing.can_see(V::Heroes) {
        (world.houses.clone(), world.orders.clone())
    } else {
        Default::default()
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
    // Landmarks and the rest of a region's furniture are revealed with Regions.
    let landmarks = if standing.can_see(V::Regions) {
        world.landmarks.clone()
    } else {
        Vec::new()
    };
    #[allow(clippy::type_complexity)]
    let (
        buildings,
        trade_routes,
        weather,
        plagues,
        monsters,
        wars,
        pacts,
        vassalages,
        pending_consequences,
    ): (
        Vec<Building>,
        Vec<TradeRoute>,
        Vec<WeatherEvent>,
        Vec<Plague>,
        Vec<Monster>,
        Vec<War>,
        Vec<Pact>,
        Vec<Vassalage>,
        Vec<DelayedConsequence>,
    ) = if standing.can_see(V::Regions) {
        (
            world.buildings.clone(),
            world.trade_routes.clone(),
            world.weather.clone(),
            world.plagues.clone(),
            world.monsters.clone(),
            world.wars.clone(),
            world.pacts.clone(),
            world.vassalages.clone(),
            world.pending_consequences.clone(),
        )
    } else {
        Default::default()
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

    WorldView {
        year: world.year,
        tick_count: world.tick_count,
        revealed: standing.scopes.clone(),
        summary: world.summary(),
        era: world.era.clone(),
        heroes,
        houses,
        orders,
        regions,
        settlements,
        resource_nodes,
        landmarks,
        buildings,
        trade_routes,
        weather,
        plagues,
        monsters,
        wars,
        pacts,
        vassalages,
        pending_consequences,
        conquest_momentum: world.conquest_momentum,
        secession_momentum: world.secession_momentum,
        artifacts,
        magic_paths,
        myths,
        myth_candidates,
        civilization,
        pantheon,
        speculations,
        era_history,
        chronicle,
    }
}

/// Project the shared world and a player onto what that player's [`Standing`]
/// reveals (§7.7). The server calls this per player, per poll.
pub fn project(
    world: &WorldState,
    player: &PlayerState,
    standing: &Standing,
    data: &GameData,
) -> (WorldView, PlayerView) {
    let view = project_world(world, standing);

    // Income is passive recovery plus the faith tithe from the *full* world
    // (§5.1 <-> 5.4) — the same tithe the sim applies each tick. Computed here,
    // where the unfiltered world is in hand, so a client whose view hides regions
    // still shows the true income.
    let favor_recovery = player.favor_recovery(&data.config, &data.balance.player)
        + mytherra_core::sim::faith_tithe(&world.regions, &data.balance.player);
    let player_view = PlayerView {
        max_favor: player.max_favor(&data.config, &data.balance.player),
        favor_recovery,
        player: player.clone(),
        standing: standing.clone(),
    };

    (view, player_view)
}

/// The visibility scope that reveals a chronicle event of the given kind, or
/// `None` for kinds that are always visible. Region/hero events are gated behind
/// the scope that reveals those entities; a player's visible divine acts
/// (Pillar 4) and system bookkeeping carry no secret and stay universal.
fn event_scope(kind: EventKind) -> Option<V> {
    match kind {
        EventKind::Region => Some(V::Regions),
        EventKind::Hero => Some(V::Heroes),
        EventKind::Divine | EventKind::System => None,
    }
}

/// Filter a chronicle delta (`GET /events`) to what a player's [`Standing`] may
/// see (§7.7), mirroring the volume + kind gating [`project`] applies to the
/// embedded chronicle. Without this, any session could poll `/events?since=0`
/// and reconstruct history its tier has not unlocked.
///
/// Two rules, both conservative:
/// - **Kind:** drop events whose revealing scope the player lacks; a kind with
///   no clean scope stays visible rather than inventing a new scope.
/// - **Volume:** without `FullChronicle`, cap the result at the newest
///   [`RECENT_EVENTS`] — the same depth `/view` already grants, so no new leak.
///
/// The caller advances the client's cursor by the *unfiltered* `since` cursor,
/// so skipped events are never re-served.
pub fn project_events<'a>(
    events: impl IntoIterator<Item = &'a WorldEvent>,
    standing: &Standing,
) -> Vec<WorldEvent> {
    if standing.can_see(V::FullChronicle) {
        return events.into_iter().cloned().collect();
    }
    let visible: Vec<WorldEvent> = events
        .into_iter()
        .filter(|event| event_scope(event.kind).is_none_or(|scope| standing.can_see(scope)))
        .cloned()
        .collect();
    // Volume cap: keep only the newest RECENT_EVENTS (they arrive oldest-first).
    let start = visible.len().saturating_sub(RECENT_EVENTS);
    visible[start..].to_vec()
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
        // Region furniture is gated with Regions — a Watcher gets none of it,
        // even the buildings a fresh world seeds.
        assert!(view.buildings.is_empty(), "buildings are region-gated");
        assert!(view.weather.is_empty());
        assert!(view.plagues.is_empty() && view.vassalages.is_empty());
        // The aggregate tenor and momentum scalars are always present, even
        // without per-region access.
        assert!(view.summary.region_count > 0);
        assert_eq!(view.conquest_momentum, world.conquest_momentum);
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
        // An Elder receives the region furniture in full (all buildings, etc.).
        assert_eq!(view.buildings.len(), world.buildings.len());
        assert!(view.revealed.contains(&V::FullChronicle));
        // The player's own favor ceiling comes through pre-computed.
        assert_eq!(pv.player.favor, player.favor);
        assert!(pv.max_favor > 0);
    }

    #[test]
    fn favor_recovery_includes_the_full_world_tithe_even_when_regions_are_hidden() {
        let (data, mut world, player) = fixtures();
        // Guarantee a non-zero tithe by consecrating a region well above the
        // tithing baseline — so the tithe is a real term the test can detect.
        world.regions[0].divine_resonance = data.balance.player.favor_tithe_baseline + 50.0;
        let expected = player.favor_recovery(&data.config, &data.balance.player)
            + mytherra_core::sim::faith_tithe(&world.regions, &data.balance.player);
        assert!(
            expected > player.favor_recovery(&data.config, &data.balance.player),
            "the consecrated region must add a real tithe"
        );

        // A Watcher's view hides every region, yet its income figure still folds
        // in the full-world tithe — it does not depend on what the view reveals.
        let watcher = data.tiers.standing(Tier::Watcher);
        let (view, pv) = project(&world, &player, &watcher, &data);
        assert!(view.regions.is_empty(), "a Watcher sees no regions");
        assert_eq!(pv.favor_recovery, expected);
    }

    #[test]
    fn events_delta_is_gated_by_standing() {
        let data = GameData::load().unwrap();
        let watcher = data.tiers.standing(Tier::Watcher);
        let elder = data.tiers.standing(Tier::Elder);

        // A chronicle mixing every kind, well past the volume cap.
        let mut chronicle = mytherra_core::world::Chronicle::default();
        for i in 0..40u32 {
            chronicle.push(i, EventKind::Region, format!("region {i}"));
            chronicle.push(i, EventKind::Hero, format!("hero {i}"));
            chronicle.push(i, EventKind::Divine, format!("divine {i}"));
            chronicle.push(i, EventKind::System, format!("system {i}"));
        }
        let (events, cursor) = chronicle.since(0);
        let full_count = events.len();

        // Elder (FullChronicle) receives everything, uncapped.
        let elder_events = project_events(events.iter().copied(), &elder);
        assert_eq!(elder_events.len(), full_count);

        // A Watcher sees heroes but not regions: no Region events survive, hero
        // events do, and the whole delta is capped at RECENT_EVENTS.
        let watcher_events = project_events(events.iter().copied(), &watcher);
        assert!(
            watcher_events.len() <= RECENT_EVENTS,
            "the volume cap bounds a low-tier delta"
        );
        assert!(
            watcher_events.iter().all(|e| e.kind != EventKind::Region),
            "region history stays hidden from a Watcher"
        );
        assert!(
            watcher_events.iter().any(|e| e.kind == EventKind::Hero),
            "a Watcher still sees the hero events its tier reveals"
        );
        // The cursor the caller returns is the unfiltered one — skipped events
        // are not re-served next poll.
        assert_eq!(cursor, chronicle.cursor());
    }

    #[test]
    fn houses_and_orders_ride_with_the_hero_roster() {
        let (data, mut world, player) = fixtures();
        // Houses and Orders arise dynamically, so a fresh world seeds none —
        // plant one of each to prove the projection carries them.
        world.houses.push(mytherra_core::world::House {
            id: "house-test".to_owned(),
            name: "The House of Test".to_owned(),
            seat_region_id: world.regions[0].id.clone(),
            founder_name: "Test Founder".to_owned(),
            member_ids: vec![world.heroes[0].id.clone()],
            prestige: 42.0,
        });
        world.orders.push(mytherra_core::world::Order {
            id: "order-test".to_owned(),
            name: "the Test Circle".to_owned(),
            role: world.heroes[0].role,
            prestige: 17.0,
            founded_year: world.year,
        });

        // A Watcher sees heroes, so it sees the bloodlines and fellowships too —
        // this is what makes house scrip and order charters tradeable.
        let watcher = data.tiers.standing(Tier::Watcher);
        let (view, _) = project(&world, &player, &watcher, &data);
        assert_eq!(view.houses.len(), 1);
        assert_eq!(view.orders.len(), 1);
        assert_eq!(view.houses[0].prestige, 42.0);

        // A Standing without Heroes gets neither, rather than a partial roster.
        let blind = Standing::default();
        let blind_view = project_world(&world, &blind);
        assert!(blind_view.houses.is_empty() && blind_view.orders.is_empty());
    }

    #[test]
    fn a_spectator_sees_the_whole_world_and_can_do_nothing() {
        let (_, world, _) = fixtures();
        let standing = spectator_standing();

        // Every scope is revealed, so no collection is withheld.
        for scope in V::ALL {
            assert!(standing.can_see(scope), "a spectator is denied {scope:?}");
        }
        let view = project_world(&world, &standing);
        assert!(!view.regions.is_empty(), "a spectator sees regions");
        assert!(!view.heroes.is_empty());
        assert!(!view.settlements.is_empty());
        assert!(!view.resource_nodes.is_empty());
        assert!(!view.trade_routes.is_empty());
        assert_eq!(view.revealed.len(), V::ALL.len());

        // But it holds no verb and no market, so it can authorize nothing and is
        // offered no wagers even though `Observatory` is revealed.
        assert!(standing.verbs.is_empty(), "a spectator holds no verb");
        assert!(standing.markets.is_empty());
        assert!(
            view.speculations.is_empty(),
            "no market unlocked means no wager is offered"
        );
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
