//! Bundled openEHR terminology assets (compile-time embedded).
//!
//! Provenance: `github.com/openEHR/specifications-TERM`, tag
//! `Release-3.0.0`, commit `d45ef3e21a05d3759101ae7bdb260e8193a3d0da`,
//! vendored verbatim under `crates/openehr-term/assets/` (see the
//! README there). The English bundle is always present; other languages ride
//! behind `lang-*` feature flags.
//!
//! PORT NOTE: TERM Release-3.0.0 ships `en`, `es`, `ja`, and `pt` bundles
//! only. The `lang-de` and `lang-fr` features declared in `Cargo.toml` (per
//! `PORT_MASTER_PLAN.md` §9.1) are stubs until upstream publishes those
//! bundles; enabling them adds nothing yet.

/// Upstream tag the assets were vendored from.
pub const TERM_RELEASE: &str = "Release-3.0.0";
/// Upstream commit the assets were vendored from.
pub const TERM_COMMIT: &str = "d45ef3e21a05d3759101ae7bdb260e8193a3d0da";

/// `computable/XML/en/openehr_terminology.xml` (always bundled).
pub const OPENEHR_TERMINOLOGY_EN: &str = include_str!("../assets/en/openehr_terminology.xml");

/// `computable/XML/es/openehr_terminology.xml`.
#[cfg(feature = "lang-es")]
pub const OPENEHR_TERMINOLOGY_ES: &str = include_str!("../assets/es/openehr_terminology.xml");

/// `computable/XML/ja/openehr_terminology.xml`.
#[cfg(feature = "lang-ja")]
pub const OPENEHR_TERMINOLOGY_JA: &str = include_str!("../assets/ja/openehr_terminology.xml");

/// `computable/XML/pt/openehr_terminology.xml`.
#[cfg(feature = "lang-pt")]
pub const OPENEHR_TERMINOLOGY_PT: &str = include_str!("../assets/pt/openehr_terminology.xml");

/// `computable/XML/openehr_external_terminologies.xml` — the ISO/IANA code
/// sets (countries, character sets, languages, media types) in the same
/// document shape as the terminology bundles.
pub const OPENEHR_EXTERNAL_TERMINOLOGIES: &str =
    include_str!("../assets/openehr_external_terminologies.xml");

/// `computable/XML/PropertyUnitData.xml` — property/unit data backing
/// `DV_QUANTITY` validation in later phases.
pub const PROPERTY_UNIT_DATA: &str = include_str!("../assets/PropertyUnitData.xml");

/// The language bundles compiled into this build, `(language tag, xml)`,
/// English first.
#[must_use]
pub fn bundled_language_xml() -> Vec<(&'static str, &'static str)> {
    #[allow(unused_mut)] // mutated only when a lang-* feature is enabled
    let mut bundles = vec![("en", OPENEHR_TERMINOLOGY_EN)];
    #[cfg(feature = "lang-es")]
    bundles.push(("es", OPENEHR_TERMINOLOGY_ES));
    #[cfg(feature = "lang-ja")]
    bundles.push(("ja", OPENEHR_TERMINOLOGY_JA));
    #[cfg(feature = "lang-pt")]
    bundles.push(("pt", OPENEHR_TERMINOLOGY_PT));
    bundles
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: TERM Release-3.0.0 computable/XML assets (vendored) — specifications-TERM @ d45ef3e
//   source_loc: crates/openehr-term/assets/
//   confidence: high
//   todos: 0
//   note: lang-de / lang-fr are declared-but-empty features until upstream ships those bundles
// ─────────────────────────────────────────────
