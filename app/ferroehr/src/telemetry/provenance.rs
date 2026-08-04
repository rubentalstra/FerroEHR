//! Build/spec provenance constants — the spec pins **derived from the
//! `openehr-*` crate versions** (each spec crate is versioned by the openEHR
//! specification it implements, so its `SPEC_VERSION` constant is the pin),
//! never hand-typed literals: a pin bump in a crate manifest propagates here
//! at compile time, so the identity surfaces cannot drift.

/// The openEHR ITS-REST contract version this server implements (the released
/// `Release-1.1.0`, 19-Jul-2026; the vendored `-codegen` OAS at
/// `crates/openehr-its/vendor/rest-oas/PROVENANCE.md` pins that release).
/// Reported by management `/info`, the System Options manifest
/// (`restapi_specs_version` — a plain version string, per the System API OAS
/// example `restapi_specs_version: 1.1.0`), and `/status`.
pub const ITS_REST: &str = openehr_its::SPEC_VERSION;
/// The AQL (QUERY) specification version.
pub const AQL: &str = openehr_query::SPEC_VERSION;
/// The openEHR Reference Model version.
pub const RM: &str = openehr_rm::SPEC_VERSION;
/// The openEHR BASE version.
pub const BASE: &str = openehr_base::SPEC_VERSION;
/// The ADL 1.4 generation of the Archetype Model (the `openehr-am` crate
/// ships both extant generations side by side; this one is the `am14`
/// module's own pin).
pub const AM14: &str = openehr_am::am14::SPEC_VERSION;
/// The ADL 2 generation of the Archetype Model (= the `openehr-am` crate
/// version, its primary generation).
pub const AM24: &str = openehr_am::SPEC_VERSION;
/// The openEHR Terminology version.
pub const TERM: &str = openehr_term::SPEC_VERSION;
/// The `PostgreSQL` version this server targets. No openEHR spec governs the
/// datastore — our own design; no crate carries this pin, so it stays
/// hand-maintained here.
pub const PG_TARGET: &str = "18.4+";
/// The last machine-computed ECC conformance verdict — the highest profile
/// obtained — advertised by the System Options manifest (`OPTIONS /`
/// `conformance_profile`).
///
/// No openEHR spec governs the value; the conformance instrument computes it
/// (CNF master03 profiles). Updated at each conformance re-baseline from the
/// runner's machine verdict recorded in
/// `docs/conformance/ferroehr/CONFORMANCE_REPORT.md` §"Profile verdict" (Core
/// PASS · Standard PASS). The manifest MUST NOT out-claim it.
pub const CONFORMANCE_PROFILE: &str = "STANDARD";

#[cfg(test)]
mod tests {
    use super::*;

    /// The derivation chain holds end to end: every pin is the owning
    /// crate's own `SPEC_VERSION`, and the AM crate version is its ADL 2
    /// generation.
    #[test]
    fn pins_are_the_crate_versions() {
        assert_eq!(ITS_REST, openehr_its::SPEC_VERSION);
        assert_eq!(AQL, openehr_query::SPEC_VERSION);
        assert_eq!(RM, openehr_rm::SPEC_VERSION);
        assert_eq!(BASE, openehr_base::SPEC_VERSION);
        assert_eq!(AM14, openehr_am::am14::SPEC_VERSION);
        assert_eq!(AM24, openehr_am::am24::SPEC_VERSION);
        assert_eq!(TERM, openehr_term::SPEC_VERSION);
        assert_eq!(openehr_am::SPEC_VERSION, openehr_am::am24::SPEC_VERSION);
    }
}
