//! Connection configuration and the top-level [`Store`] that ties the two
//! dissociated sub-stores together.

use sqlx::mysql::{MySqlConnectOptions, MySqlPoolOptions};
use sqlx::ConnectOptions;

use crate::player_store::PlayerStore;
use crate::world_store::WorldStore;

/// Connection parameters for the authority store. The caller sources these
/// however it likes (env, `.env`, CLI, a secrets manager); persistence stays
/// agnostic to where configuration comes from — no env var names live here.
#[derive(Debug, Clone)]
pub struct DbConfig {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub password: String,
    pub database: String,
}

/// The authority's persistence, split into two dissociated stores. They share a
/// pool but never share rows: the world is the shared simulation, the player
/// domain is per-deity, and the only coupling between them is the nudge in and
/// the effect out.
#[derive(Clone)]
pub struct Store {
    pub world: WorldStore,
    pub players: PlayerStore,
}

impl Store {
    /// Connect to MySQL, creating the database if absent (turnkey local setup)
    /// and running migrations. Panics on any failure — a server that cannot
    /// reach its save has nothing to serve.
    pub async fn connect(cfg: &DbConfig) -> Self {
        assert!(
            cfg.database
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_'),
            "database name must be a bare identifier (letters, digits, underscore)"
        );

        // Password characters like `$ @ ^` make a hand-built `mysql://` URL
        // error-prone, so drive the connect options directly — no encoding.
        let base = MySqlConnectOptions::new()
            .host(&cfg.host)
            .port(cfg.port)
            .username(&cfg.user)
            .password(&cfg.password);

        {
            let mut conn = base
                .clone()
                .connect()
                .await
                .expect("connect to MySQL server");
            sqlx::query(&format!("CREATE DATABASE IF NOT EXISTS `{}`", cfg.database))
                .execute(&mut conn)
                .await
                .expect("create database");
        }

        let pool = MySqlPoolOptions::new()
            .max_connections(10)
            .connect_with(base.database(&cfg.database))
            .await
            .expect("connect to MySQL database pool");

        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .expect("run database migrations");

        Self {
            world: WorldStore::new(pool.clone()),
            players: PlayerStore::new(pool),
        }
    }
}
