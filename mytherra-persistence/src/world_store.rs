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
//! per-entity column mapping.
//!
//! Writes are tracked **per entity, not per collection**: the store caches a
//! hash of each row (`table` → per-`ord` hashes) and, on save, upserts only the
//! rows whose content changed and deletes the tail a shrunk collection left
//! behind. An unchanged entity — a hero who did nothing this tick, a static
//! magic path — is never rewritten, even when its neighbours in the same
//! collection were. (Rows are keyed by position, so an insert/removal in the
//! *middle* of a collection shifts the entities after it and rewrites them; the
//! large collections that dominate the data — regions, heroes, settlements,
//! buildings — are append-or-mark-dead, so their order is stable.)

use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::{Arc, Mutex};

use mytherra_core::world::WorldState;
use serde_json::Value;
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

/// The per-`ord` write plan for one collection: which rows to upsert (their new
/// document) and which trailing rows to delete because the collection shrank.
struct CollectionPlan {
    table: &'static str,
    upserts: Vec<(u32, Value)>,
    deletes: Vec<u32>,
}

/// Storage for the shared world. Cloneable — the pool is `Arc`-backed and the
/// per-entity hash cache is shared, so every clone tracks the same rows.
#[derive(Clone)]
pub struct WorldStore {
    pool: MySqlPool,
    /// The hash of each last-written row, per table, indexed by `ord`. A row
    /// whose hash is unchanged since the previous save is not rewritten.
    written: Arc<Mutex<HashMap<&'static str, Vec<u64>>>>,
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
            let items = self.load_collection(table).await;
            // Prime the per-entity cache with what is on disk, so the first save
            // after a load rewrites only genuinely-changed rows.
            self.written
                .lock()
                .expect("written cache")
                .insert(table, items.iter().map(hash_value).collect());
            value[field] = Value::Array(items);
        }

        let world: WorldState = serde_json::from_value(value)
            .expect("deserialize world — the world schema changed; reset the DB");
        Some(world)
    }

    async fn load_collection(&self, table: &str) -> Vec<Value> {
        let rows = sqlx::query(&format!("SELECT data FROM {table} ORDER BY ord"))
            .fetch_all(&self.pool)
            .await
            .expect("query world collection");
        rows.into_iter()
            .map(|r| {
                let data: Json<Value> = r.try_get("data").expect("read world entity");
                data.0
            })
            .collect()
    }

    /// Persist the world: only the entity rows that changed since the last save,
    /// plus the `world_core` document, in one transaction.
    pub async fn save(&self, world: &WorldState) {
        let mut value = serde_json::to_value(world).expect("serialize world");

        // Diff each collection against the cache and build the row-level write
        // plan up front — no DB, no await — so the lock is never held across I/O.
        let mut plans: Vec<CollectionPlan> = Vec::new();
        {
            let mut written = self.written.lock().expect("written cache");
            for (table, field) in WORLD_COLLECTIONS {
                let items = match value.get_mut(field).map(Value::take) {
                    Some(Value::Array(items)) => items,
                    _ => Vec::new(),
                };
                let new_hashes: Vec<u64> = items.iter().map(hash_value).collect();
                let old_hashes = written.get(table).cloned().unwrap_or_default();
                if new_hashes == old_hashes {
                    continue; // whole collection untouched
                }

                let upserts: Vec<(u32, Value)> = items
                    .into_iter()
                    .enumerate()
                    .filter(|(ord, _)| old_hashes.get(*ord) != new_hashes.get(*ord))
                    .map(|(ord, item)| (ord as u32, item))
                    .collect();
                let deletes: Vec<u32> = (new_hashes.len()..old_hashes.len())
                    .map(|ord| ord as u32)
                    .collect();

                written.insert(table, new_hashes);
                plans.push(CollectionPlan {
                    table,
                    upserts,
                    deletes,
                });
            }
        }

        let mut tx = self.pool.begin().await.expect("begin world transaction");
        for plan in &plans {
            for ord in &plan.deletes {
                sqlx::query(&format!("DELETE FROM {} WHERE ord = ?", plan.table))
                    .bind(*ord)
                    .execute(&mut *tx)
                    .await
                    .expect("delete world entity");
            }
            for (ord, data) in &plan.upserts {
                sqlx::query(&format!(
                    "INSERT INTO {} (ord, data) VALUES (?, ?)
                     ON DUPLICATE KEY UPDATE data = VALUES(data)",
                    plan.table
                ))
                .bind(*ord)
                .bind(Json(data))
                .execute(&mut *tx)
                .await
                .expect("upsert world entity");
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

/// A stable hash of one entity's JSON document, used only to detect whether that
/// row changed since the last save. `serde_json` sorts object keys by default,
/// so the serialization — and thus the hash — is deterministic for equal state.
fn hash_value(value: &Value) -> u64 {
    let text = serde_json::to_string(value).expect("serialize entity for hashing");
    let mut hasher = DefaultHasher::new();
    text.hash(&mut hasher);
    hasher.finish()
}
