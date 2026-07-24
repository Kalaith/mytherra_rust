//! Where the server sources its database configuration. Persistence stays
//! agnostic to this; the server reads a local `.env` (or the real environment)
//! and hands the storage layer a [`DbConfig`].

use mytherra_persistence::DbConfig;

/// Build the DB configuration from the environment, loading the crate's own
/// `.env` first so credentials are found regardless of the working directory the
/// server is launched from. Real environment variables always win. Fails fast on
/// any missing var — no code defaults for configuration.
pub fn db_config() -> DbConfig {
    let crate_env = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(".env");
    if dotenvy::from_path(&crate_env).is_err() {
        dotenvy::dotenv().ok();
    }

    let connection = require("DB_CONNECTION");
    assert_eq!(
        connection, "mysql",
        "only DB_CONNECTION=mysql is supported by mytherra-server"
    );

    DbConfig {
        host: require("DB_HOST"),
        port: require("DB_PORT")
            .parse()
            .expect("DB_PORT must be a valid port number"),
        user: require("DB_USER"),
        password: require("DB_PASSWORD"),
        database: require("DB_DATABASE"),
    }
}

fn require(key: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| panic!("{key} must be set (see mytherra-server/.env)"))
}
