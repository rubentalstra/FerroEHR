//! Build/spec provenance constants (spec pins from `docs/VERSIONS.md`,
//! emitted at build time by `build.rs`).

/// The openEHR ITS-REST contract version this server implements: the released
/// `Release-1.1.0` (19-Jul-2026). The vendored `-codegen` OAS at
/// `crates/openehr-its/vendor/rest-oas/PROVENANCE.md` pins that release
/// (tag `24058992d`), byte-identical to the earlier pre-release snapshot.
/// Reported by management `/info`, the System Options manifest, and `/status`.
pub const ITS_REST: &str = "Release-1.1.0";
/// The AQL (QUERY) specification version (`docs/VERSIONS.md`).
pub const AQL: &str = "1.1.0";
/// The openEHR Reference Model version (`docs/VERSIONS.md`).
pub const RM: &str = "1.2.0";
/// The openEHR BASE version (`docs/VERSIONS.md`).
pub const BASE: &str = "1.3.0";
/// The openEHR Archetype Model versions (`docs/VERSIONS.md`).
pub const AM: &str = "1.4.0 + 2.4.0";
/// The openEHR Terminology version (`docs/VERSIONS.md`).
pub const TERM: &str = "3.1.0";
/// The `PostgreSQL` version this server targets (`docs/VERSIONS.md`). No
/// openEHR spec governs the datastore — our own design.
pub const PG_TARGET: &str = "18.4+";
/// The last machine-computed ECC conformance verdict — the highest profile
/// obtained — advertised by the System Options manifest (`OPTIONS /`
/// `conformance_profile`). No openEHR spec governs the value; the
/// conformance instrument computes it (CNF master03 profiles). Updated at
/// each conformance re-baseline from the runner's machine verdict recorded
/// in `docs/conformance/ehrbase-rs/CONFORMANCE_REPORT.md` §"Profile verdict"
/// (Core PASS · Standard PASS). The manifest MUST NOT out-claim it.
pub const CONFORMANCE_PROFILE: &str = "STANDARD";
