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

#[test]
fn content_meets_section_9_full_targets() {
    // The §9 Content Inventory full-target floors, as a guard: a future edit
    // that drops the world back below content-complete scale fails here
    // rather than silently shipping a demo-sized world (GDD 9, M3).
    let data = GameData::load().unwrap();
    let floors = [
        ("regions", data.regions.len(), 8),
        ("settlements", data.settlements.len(), 20),
        ("heroes", data.heroes.len(), 30),
        ("landmarks", data.landmarks.len(), 20),
        ("resource_nodes", data.resource_nodes.len(), 40),
        ("artifacts", data.artifacts.len(), 6),
        ("bet_types", data.bet_types.len(), 18),
    ];
    for (name, have, want) in floors {
        assert!(have >= want, "{name}: {have} < §9 full target {want}");
    }
}

#[test]
fn every_seeded_region_reference_resolves() {
    // Genesis seeds entities with `from_seed` and never checks that a
    // `region_id` names a real region — a typo would become an inert orphan
    // (sim lookups return None, no panic). Assert every reference resolves so
    // that failure mode can't reach a running world.
    let data = GameData::load().unwrap();
    let regions: std::collections::HashSet<&str> =
        data.regions.iter().map(|r| r.id.as_str()).collect();

    let mut refs: Vec<(&str, &str, &str)> = Vec::new();
    for s in &data.settlements {
        refs.push(("settlement", &s.id, &s.region_id));
    }
    for h in &data.heroes {
        refs.push(("hero", &h.id, &h.region_id));
    }
    for l in &data.landmarks {
        refs.push(("landmark", &l.id, &l.region_id));
    }
    for n in &data.resource_nodes {
        refs.push(("resource node", &n.id, &n.region_id));
    }
    for a in &data.artifacts {
        refs.push(("artifact", &a.id, &a.region_id));
    }
    for (kind, id, region_id) in refs {
        assert!(
            regions.contains(region_id),
            "{kind} '{id}' references unknown region '{region_id}'"
        );
    }
    for t in &data.trade_routes {
        for (end, region_id) in [("region_a", &t.region_a), ("region_b", &t.region_b)] {
            assert!(
                regions.contains(region_id.as_str()),
                "trade route '{}' {end} references unknown region '{region_id}'",
                t.id
            );
        }
    }
}
