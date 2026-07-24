//! Storage for the per-deity player domain (GDD 6 "per-player tables").
//!
//! The player is dissociated from the world and stored as a first-class
//! relational entity: its economy (favor/level/experience/…) is real columns on
//! `players`, its cultivated roster and wagers are the `player_champions` and
//! `player_bets` child tables, and player *identity* — the guest-id counter —
//! lives in `player_registry`, not in any world row. A deity never appears in
//! the world's tables; it only nudges the world and receives favor/bet effects.
//!
//! `PlayerState` is reassembled through `serde_json` rather than by naming its
//! sub-types (`Champion`, `Bet`, the toolkit `Achievements`), so this crate need
//! not depend on `macroquad-toolkit` or reach into core's internals.

use mytherra_core::world::PlayerState;
use serde::Serialize;
use serde_json::{json, Value};
use sqlx::mysql::MySqlPool;
use sqlx::types::Json;
use sqlx::{MySql, Row, Transaction};

/// Storage for every deity's private state. Cloneable (pool is `Arc`-backed).
#[derive(Clone)]
pub struct PlayerStore {
    pool: MySqlPool,
}

impl PlayerStore {
    pub(crate) fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    /// The persisted guest-id counter (0 on a fresh database). This is player
    /// identity, deliberately kept out of the world's state.
    pub async fn next_guest(&self) -> u64 {
        sqlx::query("SELECT next_guest FROM player_registry WHERE id = 1")
            .fetch_optional(&self.pool)
            .await
            .expect("query player_registry")
            .map(|row| {
                row.try_get::<u64, _>("next_guest")
                    .expect("read next_guest")
            })
            .unwrap_or(0)
    }

    /// Persist the guest-id counter.
    pub async fn set_next_guest(&self, next_guest: u64) {
        sqlx::query(
            "INSERT INTO player_registry (id, next_guest) VALUES (1, ?)
             ON DUPLICATE KEY UPDATE next_guest = VALUES(next_guest)",
        )
        .bind(next_guest)
        .execute(&self.pool)
        .await
        .expect("save player_registry");
    }

    /// Every deity in creation order (`seq`), so the tick advances them
    /// identically to before a restart (determinism, GDD 5.8).
    pub async fn load_all(&self) -> Vec<(String, PlayerState)> {
        let rows = sqlx::query(
            "SELECT player_id, favor, level, experience, favor_spent, nudges, achievements
             FROM players ORDER BY seq",
        )
        .fetch_all(&self.pool)
        .await
        .expect("query players");

        let mut out = Vec::with_capacity(rows.len());
        for row in rows {
            let id: String = row.try_get("player_id").expect("read player_id");
            let achievements: Json<Value> = row.try_get("achievements").expect("read achievements");
            let champions = self.load_children("player_champions", &id).await;
            let bets = self.load_children("player_bets", &id).await;

            let value = json!({
                "favor": row.try_get::<i64, _>("favor").expect("read favor"),
                "level": row.try_get::<u32, _>("level").expect("read level"),
                "experience": row.try_get::<i64, _>("experience").expect("read experience"),
                "favor_spent": row.try_get::<i64, _>("favor_spent").expect("read favor_spent"),
                "nudges": row.try_get::<u32, _>("nudges").expect("read nudges"),
                "champions": champions,
                "bets": bets,
                "achievements": achievements.0,
            });
            let state: PlayerState = serde_json::from_value(value)
                .expect("deserialize player — the player schema changed; reset the DB");
            out.push((id, state));
        }
        out
    }

    async fn load_children(&self, table: &str, player_id: &str) -> Value {
        let rows = sqlx::query(&format!(
            "SELECT data FROM {table} WHERE player_id = ? ORDER BY ord"
        ))
        .bind(player_id)
        .fetch_all(&self.pool)
        .await
        .expect("query player children");
        Value::Array(
            rows.into_iter()
                .map(|r| {
                    let data: Json<Value> = r.try_get("data").expect("read player child");
                    data.0
                })
                .collect(),
        )
    }

    /// Upsert one deity (its economy row plus its champion/bet child rows). Also
    /// serves as the insert for a freshly minted guest — the first write assigns
    /// its `seq`.
    pub async fn save(&self, id: &str, player: &PlayerState) {
        let mut tx = self.pool.begin().await.expect("begin player transaction");
        save_player(&mut tx, id, player).await;
        tx.commit().await.expect("commit player transaction");
    }

    /// Upsert every deity in one transaction — used after a tick, which credits
    /// favor to (and settles wagers for) every connected deity at once.
    pub async fn save_all(&self, players: &[(String, &PlayerState)]) {
        let mut tx = self.pool.begin().await.expect("begin players transaction");
        for (id, player) in players {
            save_player(&mut tx, id, player).await;
        }
        tx.commit().await.expect("commit players transaction");
    }
}

async fn save_player(tx: &mut Transaction<'_, MySql>, id: &str, player: &PlayerState) {
    let achievements =
        Json(serde_json::to_value(&player.achievements).expect("serialize achievements"));
    sqlx::query(
        "INSERT INTO players (player_id, favor, level, experience, favor_spent, nudges, achievements)
         VALUES (?, ?, ?, ?, ?, ?, ?)
         ON DUPLICATE KEY UPDATE
             favor = VALUES(favor), level = VALUES(level), experience = VALUES(experience),
             favor_spent = VALUES(favor_spent), nudges = VALUES(nudges),
             achievements = VALUES(achievements)",
    )
    .bind(id)
    .bind(player.favor)
    .bind(player.level)
    .bind(player.experience)
    .bind(player.favor_spent)
    .bind(player.nudges)
    .bind(achievements)
    .execute(&mut **tx)
    .await
    .expect("save player row");

    replace_children(tx, "player_champions", id, &player.champions).await;
    replace_children(tx, "player_bets", id, &player.bets).await;
}

async fn replace_children<T: Serialize>(
    tx: &mut Transaction<'_, MySql>,
    table: &str,
    player_id: &str,
    items: &[T],
) {
    sqlx::query(&format!("DELETE FROM {table} WHERE player_id = ?"))
        .bind(player_id)
        .execute(&mut **tx)
        .await
        .expect("clear player children");
    for (ord, item) in items.iter().enumerate() {
        let data = Json(serde_json::to_value(item).expect("serialize player child"));
        sqlx::query(&format!(
            "INSERT INTO {table} (player_id, ord, data) VALUES (?, ?, ?)"
        ))
        .bind(player_id)
        .bind(ord as u32)
        .bind(data)
        .execute(&mut **tx)
        .await
        .expect("insert player child");
    }
}
