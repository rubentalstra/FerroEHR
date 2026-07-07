//! `GET /management/info` — build + spec provenance (binding doc §2).
//!
//! Extends the P11 `/management/info` (name + version) with the git commit,
//! build timestamp, `rustc` version, the pinned openEHR specification versions,
//! and the `PostgreSQL` target. The same [`BuildInfo`] feeds the
//! `ehrbase_build_info` gauge and the `OTel` resource attributes in the binary,
//! so version provenance is captured once.

use axum::Json;
use serde::Serialize;

/// The openEHR ITS-REST contract version this server implements.
pub const OPENEHR_REST_API_VERSION: &str = "1.0.3";
/// The AQL specification version.
pub const OPENEHR_AQL_VERSION: &str = "1.1.0";
/// The pinned openEHR Reference Model version (`docs/VERSIONS.md`).
pub const RM_VERSION: &str = "1.2.0";
/// The pinned openEHR BASE version.
pub const BASE_VERSION: &str = "1.3.0";
/// The pinned openEHR Archetype Model versions.
pub const AM_VERSION: &str = "1.4.0 + 2.4.0";
/// The pinned openEHR Terminology version.
pub const TERM_VERSION: &str = "3.1.0";
/// The `PostgreSQL` version this server targets.
pub const PG_TARGET: &str = "18.4+";

/// Build- and spec-provenance, captured once. Cheap to clone.
#[derive(Debug, Clone, Serialize)]
pub struct BuildInfo {
    /// The crate/package name.
    pub name: &'static str,
    /// The crate version.
    pub version: &'static str,
    /// The git commit (short SHA) the binary was built from, or `unknown`.
    pub git_sha: &'static str,
    /// The build timestamp (ISO-8601 UTC), or `unknown` if unavailable.
    pub build_date: String,
    /// The `rustc` version string the binary was compiled with.
    pub rustc: &'static str,
    /// The pinned openEHR specification versions.
    pub spec: SpecVersions,
    /// The `PostgreSQL` version target.
    pub postgres_target: &'static str,
}

/// The pinned openEHR specification versions surfaced by `/management/info`.
#[derive(Debug, Clone, Serialize)]
pub struct SpecVersions {
    /// ITS-REST contract version.
    pub its_rest: &'static str,
    /// AQL version.
    pub aql: &'static str,
    /// Reference Model version.
    pub rm: &'static str,
    /// BASE version.
    pub base: &'static str,
    /// Archetype Model versions.
    pub am: &'static str,
    /// Terminology version.
    pub term: &'static str,
}

impl BuildInfo {
    /// The build info for this binary, from the values captured by `build.rs`.
    #[must_use]
    pub fn current() -> Self {
        Self {
            name: env!("CARGO_PKG_NAME"),
            version: env!("CARGO_PKG_VERSION"),
            git_sha: env!("EHRBASE_GIT_SHA"),
            build_date: build_date(),
            rustc: env!("EHRBASE_RUSTC"),
            spec: SpecVersions {
                its_rest: OPENEHR_REST_API_VERSION,
                aql: OPENEHR_AQL_VERSION,
                rm: RM_VERSION,
                base: BASE_VERSION,
                am: AM_VERSION,
                term: TERM_VERSION,
            },
            postgres_target: PG_TARGET,
        }
    }
}

impl Default for BuildInfo {
    fn default() -> Self {
        Self::current()
    }
}

/// Render the build epoch captured by `build.rs` as an ISO-8601 UTC string.
fn build_date() -> String {
    env!("EHRBASE_BUILD_EPOCH")
        .parse::<i64>()
        .ok()
        .and_then(|secs| jiff::Timestamp::from_second(secs).ok())
        .map_or_else(|| "unknown".to_owned(), |ts| ts.to_string())
}

/// `GET /management/info`.
pub(super) fn info(build: BuildInfo) -> Json<BuildInfo> {
    Json(build)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_info_is_populated() {
        let info = BuildInfo::current();
        assert_eq!(info.name, "ehrbase-rest");
        assert_eq!(info.spec.rm, "1.2.0");
        assert_eq!(info.spec.its_rest, "1.0.3");
        assert_eq!(info.postgres_target, "18.4+");
        // build_date parses to a real timestamp (not the "unknown" fallback) in
        // a normal build where build.rs ran.
        assert!(!info.build_date.is_empty());
    }
}
