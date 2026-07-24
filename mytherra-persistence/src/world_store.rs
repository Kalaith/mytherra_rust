//! Storage for the one shared world (GDD 6 "shared/global tables").
//!
//! The world is decomposed by entity: every collection on `WorldState`
//! (`regions`, `heroes`, `settlements`, …) is its own table, one row per entity
//! (`ord`, `data`), rather than the whole world living in a single JSON field.
//! The non-collection state — the year, the `*_seq` counters, the era, the
//! chronicle, and crucially the world's RNG — rides in a single small
//! `world_core` row, so a reload resumes the exact deterministic sequence
//! (GDD 5.8).
//!
//! The decomposition is registry-driven and JSON-per-entity (not fully
//! columnar): each entity keeps its own shape as a `data` document, so adding a
//! new collection is one [`WORLD_COLLECTIONS`] line plus one table — no bespoke
//! per-entity column mapping. A collection whose contents did not change this
//! tick is not rewritten (the payoff of splitting the world out of one blob).

use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::{Arc, Mutex};

use mytherra_core::world::WorldState;
use serde_json::{json, Value};
use sqlx::mysql::MySqlPool;
use sqlx::types::Json;
use sqlx::Row;

/// `(table, WorldState JSON field)` for every world collection that gets its own
/// row-per-entity table. Anything not listed here stays in the `world_core`
/// document (scalars, the era, the chronicle, the RNG). Table names are a fixed
/// allowlist, never user input, so interpolating them into SQL is safe.
const WORLD_COLLECTIONS: &[(&str, &str)] = &[
    ("world_regions", "regions"),
    ("world_settlements", "settlements"),
    ("world_resource_nodes", "resource_nodes"),
    ("world_landmarks", "landmarks"),
    ("world_trade_routes", "trade_routes"),
    ("world_buildings", "buildings"),
    ("world_heroes", "heroes"),
    ("world_houses", "houses"),
    ("world_orders", "orders"),
    ("world_saints", "saints"),
    ("world_festivals", "festivals"),
    ("world_artifacts", "artifacts"),
    ("world_pending_consequences", "pending_consequences"),
    ("world_era_history", "era_history"),
    ("world_weather", "weather"),
    ("world_plagues", "plagues"),
    ("world_monsters", "monsters"),
    ("world_wars", "wars"),
    ("world_vassalages", "vassalages"),
    ("world_prophecies", "prophecies"),
    ("world_pacts", "pacts"),
    ("world_magic_paths", "magic_paths"),
    ("world_myths", "myths"),
    ("world_myth_candidates", "myth_candidates"),
    ("world_civilization", "civilization"),
    ("world_pantheon", "pantheon"),
    ("world_speculations", "speculations"),
];

/// Storage for the shared world. Cloneable — the pool is `Arc`-backed and the
/// change-tracking cache is shared, so every clone skips the same unchanged
/// collections.
#[derive(Clone)]
pub struct WorldStore {
    pool: MySqlPool,
    /// Hash of the last-written `data` array per collection table, so a
    /// collection untouched since the previous save is not rewritten.
    written: Arc<Mutex<HashMap<&'static str, u64>>>,
}

impl WorldStore {
    pub(crate) fn new(pool: MySqlPool) -> Self {
        Self {
            pool,
            written: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Reassemble the world from its `world_core` row and per-entity tables, or
    /// `None` on a fresh database. A document that no longer deserializes into
    /// the current types is a hard error, not a silent reset.
    pub async fn load(&self) -> Option<WorldState> {
        let row = sqlx::query("SELECT data FROM world_core WHERE id = 1")
            .fetch_optional(&self.pool)
            .await
            .expect("query world_core")?;
        let core: Json<Value> = row.try_get("data").expect("read world_core");
        let mut value = core.0;

        for (table, field) in WORLD_COLLECTIONS {
            let array = self.load_collection(table).await;
            self.written
                .lock()
                .expect("written cache")
                .insert(table, hash_array(&array));
            value[field] = array;
        }

        let world: WorldState = serde_json::from_value(value)
            .expect("deserialize world — the world schema changed; reset the DB");
        Some(world)
    }

    async fn load_collection(&self, table: &str) -> Value {
        let rows = sqlx::query(&format!("SELECT data FROM {table} ORDER BY ord"))
            .fetch_all(&self.pool)
            .await
            .expect("query world collection");
        Value::Array(
            rows.into_iter()
                .map(|r| {
                    let data: Json<Value> = r.try_get("data").expect("read world entity");
                    data.0
                })
                .collect(),
        )
    }

    /// Persist the world: each changed collection's rows plus the `world_core`
    /// document, in one transaction. Collections whose contents are unchanged
    /// since the last save are skipped.
    pub async fn save(&self, world: &WorldState) {
        let mut value = serde_json::to_value(world).expect("serialize world");

        // Split the collections out of the document and decide which changed —
        // done up front (no DB, no await) so the lock is never held across I/O.
        let mut plan: Vec<(&'static str, Value, bool)> =
            Vec::with_capacity(WORLD_COLLECTIONS.len());
        {
            let mut written = self.written.lock().expect("written cache");
            for (table, field) in WORLD_COLLECTIONS {
                let array = value
                    .get_mut(field)
                    .map(Value::take)
                    .unwrap_or_else(|| json!([]));
                let hash = hash_array(&array);
                let changed = written.get(table) != Some(&hash);
                if changed {
                    written.insert(table, hash);
                }
                plan.push((table, array, changed));
            }
        }

        let mut tx = self.pool.begin().await.expect("begin world transaction");
        for (table, array, changed) in &plan {
            if !changed {
                continue;
            }
            sqlx::query(&format!("DELETE FROM {table}"))
                .execute(&mut *tx)
                .await
                .expect("clear world collection");
            if let Value::Array(items) = array {
                for (ord, item) in items.iter().enumerate() {
                    sqlx::query(&format!("INSERT INTO {table} (ord, data) VALUES (?, ?)"))
                        .bind(ord as u32)
                        .bind(Json(item))
                        .execute(&mut *tx)
                        .await
                        .expect("insert world entity");
                }
            }
        }

        // `value` now holds only the non-collection state (the taken fields are
        // null and are overwritten on load), i.e. the `world_core` document.
        sqlx::query(
            "INSERT INTO world_core (id, data) VALUES (1, ?)
             ON DUPLICATE KEY UPDATE data = VALUES(data)",
        )
        .bind(Json(&value))
        .execute(&mut *tx)
        .await
        .expect("save world_core");

        tx.commit().await.expect("commit world transaction");
    }
}

/// A stable hash of a collection's JSON array, used only to detect whether it
/// changed since the last save. `serde_json` sorts object keys by default, so
/// the serialization — and thus the hash — is deterministic for equal state.
fn hash_array(array: &Value) -> u64 {
    let text = serde_json::to_string(array).expect("serialize collection for hashing");
    let mut hasher = DefaultHasher::new();
    text.hash(&mut hasher);
    hasher.finish()
}
