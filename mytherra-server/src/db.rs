//! MySQL persistence for the authority server (GDD 6/8): the DB *is* the save.
//!
//! `mytherra-core` stays pure — it does no I/O — so all persistence lives here.
//! The whole shared `WorldState` and each deity's `PlayerState` are already
//! serde-serializable, so v1 write-throughs each as a JSON document (GDD 7.2: a
//! server row resumes the exact same deterministic sequence, RNG included). The
//! fully-relational schema of GDD 6 is the M3 target, not this phase.
//!
//! Connection details come from a local `.env` (`DB_HOST`/`DB_PORT`/`DB_USER`/
//! `DB_PASSWORD`/`DB_DATABASE`, `DB_CONNECTION=mysql`) — never a committed config
//! file, and never a code default (fail fast on any missing var).

use std::collections::BTreeMap;

use mytherra_core::world::{PlayerState, WorldState};
use sqlx::mysql::{MySqlConnectOptions, MySqlPool, MySqlPoolOptions};
use sqlx::types::Json;
use sqlx::{ConnectOptions, Row};

/// The world and every deity as loaded back from the DB on startup.
pub struct LoadedWorld {
    pub world: WorldState,
    pub next_guest: u64,
    /// Deities in their original creation order (`seq`), so the tick advances
    /// them identically to before the restart (determinism, GDD 5.8).
    pub players: Vec<(String, PlayerState)>,
}

/// A handle to the authority's MySQL store. Cloneable — `MySqlPool` is an
/// `Arc`-backed connection pool shared across every request handler and the tick.
#[derive(Clone)]
pub struct Db {
    pool: MySqlPool,
}

/// Read a required env var, failing fast (no code default) per the project's
/// config discipline.
fn require_env(key: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| panic!("{key} must be set (see mytherra-server/.env)"))
}

impl Db {
    /// Connect to the configured MySQL, creating the database if it is absent,
    /// and run migrations. Panics on any failure — a server that cannot reach its
    /// save has nothing to serve.
    pub async fn connect() -> Self {
        // Load the crate's own `.env` regardless of the working directory the
        // server is launched from (workspace root vs. crate dir), then fall back
        // to a cwd-relative `.env`. Real environment variables always win.
        let crate_env = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(".env");
        if dotenvy::from_path(&crate_env).is_err() {
            dotenvy::dotenv().ok();
        }

        let connection = require_env("DB_CONNECTION");
        assert_eq!(
            connection, "mysql",
            "only DB_CONNECTION=mysql is supported by mytherra-server"
        );
        let host = require_env("DB_HOST");
        let port: u16 = require_env("DB_PORT")
            .parse()
            .expect("DB_PORT must be a valid port number");
        let user = require_env("DB_USER");
        let password = require_env("DB_PASSWORD");
        let database = require_env("DB_DATABASE");
        assert!(
            database
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_'),
            "DB_DATABASE must be a bare identifier (letters, digits, underscore)"
        );

        // Password characters like `$ @ ^` make hand-building a `mysql://` URL
        // error-prone, so drive the connect options directly — no percent-encoding.
        let base = MySqlConnectOptions::new()
            .host(&host)
            .port(port)
            .username(&user)
            .password(&password);

        // Create the database on first run so local setup is turnkey.
        {
            let mut conn = base
                .clone()
                .connect()
                .await
                .expect("connect to MySQL server");
            sqlx::query(&format!("CREATE DATABASE IF NOT EXISTS `{database}`"))
                .execute(&mut conn)
                .await
                .expect("create database");
        }

        let pool = MySqlPoolOptions::new()
            .max_connections(10)
            .connect_with(base.database(&database))
            .await
            .expect("connect to MySQL database pool");

        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .expect("run database migrations");

        Self { pool }
    }

    /// Load the persisted world and its deities, or `None` if the world has never
    /// been saved (a fresh database). A JSON that no longer deserializes into the
    /// current types is a hard error, not a silent reset — matching the fail-fast
    /// discipline of the local save path.
    pub async fn load(&self) -> Option<LoadedWorld> {
        let row = sqlx::query("SELECT next_guest, state FROM world_state WHERE id = 1")
            .fetch_optional(&self.pool)
            .await
            .expect("query world_state")?;

        let next_guest: u64 = row.try_get("next_guest").expect("read next_guest");
        let world: Json<WorldState> = row
            .try_get("state")
            .expect("deserialize world_state — the world schema changed; reset the DB");

        let player_rows = sqlx::query("SELECT player_id, state FROM players ORDER BY seq")
            .fetch_all(&self.pool)
            .await
            .expect("query players");
        let players = player_rows
            .into_iter()
            .map(|r| {
                let id: String = r.try_get("player_id").expect("read player_id");
                let state: Json<PlayerState> = r
                    .try_get("state")
                    .expect("deserialize player — the player schema changed; reset the DB");
                (id, state.0)
            })
            .collect();

        Some(LoadedWorld {
            world: world.0,
            next_guest,
            players,
        })
    }

    /// Upsert the singleton world row (shared state + the guest-id counter).
    pub async fn save_world(&self, world: &WorldState, next_guest: u64, version: &str) {
        sqlx::query(
            "INSERT INTO world_state (id, version, next_guest, state) VALUES (1, ?, ?, ?)
             ON DUPLICATE KEY UPDATE
                 version = VALUES(version),
                 next_guest = VALUES(next_guest),
                 state = VALUES(state)",
        )
        .bind(version)
        .bind(next_guest)
        .bind(Json(world))
        .execute(&self.pool)
        .await
        .expect("save world_state");
    }

    /// Bump just the persisted guest counter — cheaper than re-serializing the
    /// whole world when only a new session was minted.
    pub async fn save_next_guest(&self, next_guest: u64) {
        sqlx::query("UPDATE world_state SET next_guest = ? WHERE id = 1")
            .bind(next_guest)
            .execute(&self.pool)
            .await
            .expect("save next_guest");
    }

    /// Insert a freshly minted deity's row.
    pub async fn insert_player(&self, id: &str, state: &PlayerState) {
        sqlx::query("INSERT INTO players (player_id, state) VALUES (?, ?)")
            .bind(id)
            .bind(Json(state))
            .execute(&self.pool)
            .await
            .expect("insert player");
    }

    /// Write-through one deity's private state after it acts.
    pub async fn save_player(&self, id: &str, state: &PlayerState) {
        sqlx::query("UPDATE players SET state = ? WHERE player_id = ?")
            .bind(Json(state))
            .bind(id)
            .execute(&self.pool)
            .await
            .expect("save player");
    }

    /// Persist every deity in one transaction — used after a tick, which credits
    /// favor to (and can otherwise touch) every connected deity at once.
    pub async fn save_all_players(&self, ids: &BTreeMap<String, usize>, players: &[PlayerState]) {
        let mut tx = self.pool.begin().await.expect("begin tick transaction");
        for (id, &index) in ids {
            sqlx::query("UPDATE players SET state = ? WHERE player_id = ?")
                .bind(Json(&players[index]))
                .bind(id)
                .execute(&mut *tx)
                .await
                .expect("save player during tick");
        }
        tx.commit().await.expect("commit tick transaction");
    }
}
