//! Environment configuration. No secrets are baked into the binary.

use std::env;

pub const DEFAULT_DATABASE_URL: &str = "postgres://postgres:postgres@localhost:5432/coc_phoenix";
pub const DEFAULT_BIND: &str = "127.0.0.1:8080";

#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub bind: String,
    pub max_connections: u32,
    pub content_pack_id: String,
    /// Set `RUN_MIGRATIONS=false` to skip migrations on boot.
    pub run_migrations: bool,
    /// Set `SYNC_CONTENT=false` to skip republishing the bundled content pack on boot.
    pub sync_content: bool,
}

impl Config {
    /// Read configuration from the environment, falling back to local-development defaults.
    pub fn from_env() -> Config {
        Config {
            database_url: env::var("DATABASE_URL")
                .unwrap_or_else(|_| DEFAULT_DATABASE_URL.to_string()),
            bind: env::var("BIND_ADDR").unwrap_or_else(|_| DEFAULT_BIND.to_string()),
            max_connections: env::var("DB_MAX_CONNECTIONS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(5),
            content_pack_id: env::var("CONTENT_PACK_ID")
                .unwrap_or_else(|_| crate::db::DEFAULT_PACK_ID.to_string()),
            run_migrations: env::var("RUN_MIGRATIONS")
                .map(|v| v != "false" && v != "0")
                .unwrap_or(true),
            sync_content: env::var("SYNC_CONTENT")
                .map(|v| v != "false" && v != "0")
                .unwrap_or(true),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_documented_default_url_names_the_compose_database() {
        assert!(DEFAULT_DATABASE_URL.ends_with("/coc_phoenix"));
    }
}
