//! Where the server sources its database configuration. Persistence stays
//! agnostic to this; the server reads a local `.env` (or the real environment)
//! and hands the storage layer a [`DbConfig`].

use mytherra_persistence::DbConfig;

/// Auth configuration for account linking (GDD 7.3): the shared secret used to
/// verify WebHatchery account tokens, and the URL a client sends a player to in
/// order to sign in. Both come from the environment, never a code default.
pub struct AuthConfig {
    pub jwt_secret: String,
    pub login_url: String,
}

/// Load the crate's own `.env` first (so config is found regardless of the
/// working directory the server is launched from), falling back to any ambient
/// `.env`. Real environment variables always win. Idempotent — safe to call from
/// each config builder.
fn load_env() {
    let crate_env = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(".env");
    if dotenvy::from_path(&crate_env).is_err() {
        dotenvy::dotenv().ok();
    }
}

/// Build the DB configuration from the environment. Fails fast on any missing
/// var — no code defaults for configuration.
pub fn db_config() -> DbConfig {
    load_env();

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

/// Build the auth configuration for account linking (GDD 7.3). Fails fast if the
/// shared secret or login URL is missing — a server that cannot verify account
/// tokens must not pretend linking works.
pub fn auth_config() -> AuthConfig {
    load_env();
    AuthConfig {
        jwt_secret: require("JWT_SECRET"),
        login_url: require("WEBHATCHERY_LOGIN_URL"),
    }
}

fn require(key: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| panic!("{key} must be set (see mytherra-server/.env)"))
}
