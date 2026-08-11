use super::*;
use mytherra_core::data::GameData;
use mytherra_core::world::WorldState;

#[test]
fn every_world_collection_names_a_real_world_field() {
    // The entity-per-row design (GDD 6) hinges on each WORLD_COLLECTIONS entry
    // naming an actual array field on WorldState. A typo'd or stale field would
    // silently never match on save — that collection would ride forever inside
    // the `world_core` blob and its own table stay empty — so pin the mapping.
    let world = WorldState::new(&GameData::load().expect("load content"));
    let value = serde_json::to_value(&world).expect("serialize world");
    let object = value.as_object().expect("world serializes to an object");
    for (table, field) in WORLD_COLLECTIONS {
        match object.get(*field) {
            Some(Value::Array(_)) => {}
            other => panic!(
                "{table}: WORLD_COLLECTIONS field '{field}' is not a WorldState array ({other:?})"
            ),
        }
    }
}

#[test]
fn world_collection_tables_and_fields_are_unique() {
    // A duplicated table or field (an easy slip when copying a line) would let
    // two collections clobber each other on save; forbid it.
    let mut tables = std::collections::HashSet::new();
    let mut fields = std::collections::HashSet::new();
    for (table, field) in WORLD_COLLECTIONS {
        assert!(tables.insert(*table), "duplicate table '{table}'");
        assert!(fields.insert(*field), "duplicate field '{field}'");
    }
}
