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
/// with the spec's exact lower-case symbol names preserved via
/// [`ValidityKind::symbol`].
///
/// PORT NOTE: `openehr-base` has no `serde` dependency yet (mirroring the
/// sibling `openehr-foundation::primitive_types` cluster, which is likewise
/// serde-free at this layer — the `_type`-discriminated canonical-JSON
/// mapping is an `openehr-serde`/RM-layer concern, not a BASE foundation/
/// definitions concern). `symbol()` renders the spec's own lower-case
/// identifier (`mandatory`, `optional`, `prohibited` — not the uppercase
/// `_type`-style discriminator used for RM/AM class names) so a later serde
/// impl at the RM layer has a single, spec-verified string to rename onto.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ValidityKind {
    /// `mandatory` — constant to indicate mandatory presence of something.
    Mandatory,

    /// `optional` — constant to indicate optional presence of something.
    Optional,

    /// `prohibited` — constant to indicate disallowed presence of something.
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
//   note: closed 3-value enum with a symbol() method carrying the spec's own lower-case name; no serde derive since this crate has no serde dependency yet (mirrors openehr-foundation::primitive_types, which is likewise serde-free at this layer).
// ─────────────────────────────────────────────
