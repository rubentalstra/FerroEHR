//! The `[db]` section — `PostgreSQL` connection settings.
//!
//! No openEHR spec governs persistence — our own design
//! (`docs/design/configuration.md` §3.2). No loader of its own: this struct is
//! a field of the one config tree ([`crate::config::EhrbaseConfig`]), assembled
//! once at boot. The DSN is a [`SecretUrl`]: its embedded credentials are
//! redacted from every rendering (`Debug`, `/management/env`, `config check`).

use crate::config::secret::SecretUrl;
use serde::{Deserialize, Serialize};

/// The zero-config dev DSN (matches the compose dev stack). Production MUST
/// override it (`docs/design/configuration.md` §3.16 checklist).
pub const DEFAULT_URL: &str = "postgres://ehrbase:ehrbase@localhost:5432/ehrbase";

/// Connection settings for the application `PostgreSQL` database.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct DbConfig {
    /// `PostgreSQL` connection DSN (`postgres://user:pass@host:port/db`).
    /// Credentials are redacted from every rendering ([`SecretUrl`]).
    pub url: SecretUrl,
    /// Upper bound of the connection pool.
    pub max_connections: u32,
    /// Connections the pool keeps open when idle (avoids cold reopen +
    /// `SET search_path` churn under variable load).
    pub min_connections: u32,
    /// Seconds to wait for a free connection before failing.
    pub acquire_timeout_secs: u64,
}

impl Default for DbConfig {
    fn default() -> Self {
        Self {
            url: SecretUrl::new(DEFAULT_URL),
            // Deliberate P20 defaults: 20 max (10 hard-capped realistic write
            // concurrency ×2), 2 min (no cold reopen churn at idle).
            max_connections: 20,
            min_connections: 2,
            acquire_timeout_secs: 30,
        }
    }
}

impl DbConfig {
    /// Settings for `url` with defaults for everything else.
    #[must_use]
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: SecretUrl::new(url.into()),
            ..Self::default()
        }
    }

    /// Whether the DSN is the built-in dev default (no operator override). The
    /// boot path logs a prominent warning in this case
    /// (`docs/design/configuration.md` §3.16) so a production deployment never
    /// silently runs against the dev database.
    #[must_use]
    pub fn is_dev_default(&self) -> bool {
        self.url.expose() == DEFAULT_URL
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_applied() {
        let s = DbConfig::new("postgres://localhost/ehrbase");
        assert_eq!(s.url.expose(), "postgres://localhost/ehrbase");
        assert_eq!(s.max_connections, 20);
        assert_eq!(s.min_connections, 2);
        assert_eq!(s.acquire_timeout_secs, 30);
        assert!(!s.is_dev_default());
    }

    #[test]
    fn default_url_is_the_dev_dsn() {
        assert!(DbConfig::default().is_dev_default());
    }
}
