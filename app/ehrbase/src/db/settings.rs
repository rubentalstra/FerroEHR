use figment::{Figment, providers::Env};
use serde::Deserialize;

use crate::db::DbError;

/// Connection settings for the application `PostgreSQL` database (no openEHR
/// spec governs persistence — our own design).
///
/// Loaded from the environment: `EHRBASE_DB_URL`, `EHRBASE_DB_MAX_CONNECTIONS`,
/// `EHRBASE_DB_MIN_CONNECTIONS`, `EHRBASE_DB_ACQUIRE_TIMEOUT_SECS`; a bare
/// `DATABASE_URL` is accepted as a fallback for the URL. This covers only the
/// database connection; full server configuration is assembled elsewhere.
#[derive(Debug, Clone, Deserialize)]
pub struct DbSettings {
    /// `PostgreSQL` connection URL (`postgres://user:pass@host:port/db`).
    pub url: String,
    /// Upper bound of the connection pool.
    #[serde(default = "defaults::max_connections")]
    pub max_connections: u32,
    /// Connections the pool keeps open when idle.
    #[serde(default)]
    pub min_connections: u32,
    /// Seconds to wait for a free connection before failing.
    #[serde(default = "defaults::acquire_timeout_secs")]
    pub acquire_timeout_secs: u64,
}

mod defaults {
    pub(super) fn max_connections() -> u32 {
        10
    }

    pub(super) fn acquire_timeout_secs() -> u64 {
        30
    }
}

impl DbSettings {
    /// Settings for `url` with defaults for everything else.
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            max_connections: defaults::max_connections(),
            min_connections: 0,
            acquire_timeout_secs: defaults::acquire_timeout_secs(),
        }
    }

    /// Load settings from the process environment.
    ///
    /// # Errors
    ///
    /// Returns [`DbError::Config`] when no database URL is set or a value
    /// cannot be parsed (e.g. a non-numeric `EHRBASE_DB_MAX_CONNECTIONS`).
    pub fn from_env() -> Result<Self, DbError> {
        let figment = Figment::new()
            .merge(Env::prefixed("EHRBASE_DB_"))
            .join(Env::raw().only(&["DATABASE_URL"]).map(|_| "url".into()));
        figment.extract().map_err(|e| DbError::Config(Box::new(e)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_applied() {
        let s = DbSettings::new("postgres://localhost/ehrbase");
        assert_eq!(s.max_connections, 10);
        assert_eq!(s.min_connections, 0);
        assert_eq!(s.acquire_timeout_secs, 30);
    }

    #[test]
    #[allow(clippy::result_large_err)] // figment::Jail's closure signature
    fn from_env_reads_prefixed_vars_and_database_url_fallback() {
        figment::Jail::expect_with(|jail| {
            jail.set_env("DATABASE_URL", "postgres://fallback/db");
            let s = DbSettings::from_env().expect("settings");
            assert_eq!(s.url, "postgres://fallback/db");

            jail.set_env("EHRBASE_DB_URL", "postgres://primary/db");
            jail.set_env("EHRBASE_DB_MAX_CONNECTIONS", "3");
            let s = DbSettings::from_env().expect("settings");
            assert_eq!(s.url, "postgres://primary/db");
            assert_eq!(s.max_connections, 3);
            Ok(())
        });
    }
}
