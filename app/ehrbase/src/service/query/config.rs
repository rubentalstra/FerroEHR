//! The `[query]` section — AQL execution knobs.
//!
//! No openEHR spec governs these tuning knobs — our own operational extension
//! (`docs/design/configuration.md` §3.12). A field of the one config tree
//! ([`crate::config::EhrbaseConfig`]); no loader of its own. These replace the
//! two former raw-env reads (`EHRBASE__QUERY__PLAN_CACHE_CAPACITY` /
//! `EHRBASE__QUERY__TIMEOUT_MS`) whose parse failures were silently swallowed —
//! they are now ordinary typed fields, so a bad value is a boot error like
//! everywhere else.

use std::time::Duration;

use serde::{Deserialize, Serialize};

/// AQL execution configuration (`[query]`).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct QueryConfig {
    /// Max distinct cached query plans; `0` disables the plan cache (every
    /// lookup misses and the full parse→lower path runs).
    pub plan_cache_capacity: u64,
    /// Per-query DB execution budget in milliseconds; `0` disables it (the
    /// global request timeout remains the only guard). Overrun reports `408`
    /// (ITS-REST `Requests_and_responses` §HTTP status codes).
    pub timeout_ms: u64,
}

impl Default for QueryConfig {
    fn default() -> Self {
        Self {
            plan_cache_capacity: 256,
            timeout_ms: 0,
        }
    }
}

impl QueryConfig {
    /// The per-query execution budget as a [`Duration`], or `None` when
    /// disabled (`timeout_ms == 0`).
    #[must_use]
    pub fn timeout(&self) -> Option<Duration> {
        (self.timeout_ms > 0).then(|| Duration::from_millis(self.timeout_ms))
    }
}

#[cfg(test)]
#[allow(
    clippy::panic,
    clippy::print_stdout,
    clippy::print_stderr,
    let_underscore_drop
)] // test assertions/diagnostics/fixtures
mod tests {
    use super::*;

    #[test]
    fn defaults_cache_on_timeout_off() {
        let c = QueryConfig::default();
        assert_eq!(c.plan_cache_capacity, 256);
        assert_eq!(c.timeout_ms, 0);
        assert!(c.timeout().is_none());
    }

    #[test]
    fn positive_timeout_maps_to_duration() {
        let c = QueryConfig {
            timeout_ms: 1500,
            ..QueryConfig::default()
        };
        assert_eq!(c.timeout(), Some(Duration::from_millis(1500)));
    }
}
