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
