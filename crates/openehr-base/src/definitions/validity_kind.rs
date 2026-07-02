//! `VALIDITY_KIND` — presence/absence constraint enumeration.
//!
//! openEHR class: `VALIDITY_KIND` (enumeration), package
//! `base.base_types.definitions`.
//!
//! An enumeration of three values that may commonly occur in constraint
//! models. Used as the type of any attribute within a reference model that
//! expresses a constraint on some attribute in a class in that reference
//! model — for example, to indicate the validity of Date/Time fields.

/// Closed three-value enumeration, transcribed directly as a Rust `enum`
/// with the spec's exact lower-case symbol names preserved via both
/// [`ValidityKind::symbol`] and the `#[serde(rename = "...")]` on each
/// variant below.
///
/// P4 update: `openehr-base` now depends on `serde`
/// (`PORT_MASTER_PLAN.md` §10). Unlike an RM/AM class name (which serializes
/// as an uppercase `_type` discriminator, e.g. `DV_TEXT`), `VALIDITY_KIND`
/// is an *enumeration value* embedded directly as the value of whatever
/// attribute is typed `VALIDITY_KIND` elsewhere in the RM — so each variant
/// here is tagged with the spec's own lower-case wire form (`mandatory`,
/// `optional`, `prohibited`), not an uppercase class-style tag.
/// [`ValidityKind::symbol`] remains available as a plain accessor so
/// non-serde call sites (e.g. `Display` impls, log messages) do not need to
/// round-trip through a serializer just to read the spec string.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum ValidityKind {
    /// `mandatory` — constant to indicate mandatory presence of something.
    #[serde(rename = "mandatory")]
    Mandatory,

    /// `optional` — constant to indicate optional presence of something.
    #[serde(rename = "optional")]
    Optional,

    /// `prohibited` — constant to indicate disallowed presence of something.
    #[serde(rename = "prohibited")]
    Prohibited,
}

impl ValidityKind {
    /// The spec's own lower-case symbol name for this enumeration value.
    pub const fn symbol(self) -> &'static str {
        match self {
            ValidityKind::Mandatory => "mandatory",
            ValidityKind::Optional => "optional",
            ValidityKind::Prohibited => "prohibited",
        }
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: BASE 1.2.0 base_types.definitions — docs/research/spec-cache/BASE-1.2.0/uml_classes/validity_kind.adoc (Release-1.2.0 @ 9064413)
//   source_loc: master03-definitions_package.adoc §Class Definitions / validity_kind.adoc §VALIDITY_KIND Enumeration
//   confidence: high
//   todos: 0
//   note: closed 3-value enum with a symbol() method carrying the spec's own lower-case name; P4 — serde derives added, per-variant #[serde(rename)] uses the same lower-case wire form as symbol() (enumeration values, unlike RM class names, do not use the uppercase _type convention).
// ─────────────────────────────────────────────
