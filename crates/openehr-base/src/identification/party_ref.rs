//! `PARTY_REF` — identifier for parties in a demographic or identity
//! service.
//!
//! openEHR class: `PARTY_REF`, package `base.base_types.identification`.
//! Inherits: `OBJECT_REF`.
//!
//! Identifier for parties in a demographic or identity service. There are
//! typically a number of subtypes of the `PARTY` class, including
//! `PERSON`, `ORGANISATION`, etc. Abstract supertypes are allowed if the
//! referenced object is of a type not known by the current implementation
//! of this class (in other words, if the demographic model is changed by
//! the addition of a new `PARTY` or `ACTOR` subtypes, valid `PARTY_REF`s
//! can still be constructed to them).
use super::object_ref::ObjectRef;

/// Canonical `_type` discriminator string for this class in serialized
/// form. See the `TODO(port)` on `hier_object_id::TYPE_NAME` for why this
/// is a `const` rather than a `#[serde(rename = ...)]` in this pass.
pub const TYPE_NAME: &str = "PARTY_REF";

/// The closed set of legal values for `ObjectRef.type` on a `PARTY_REF`,
/// per the `Type_validity` invariant: `type.is_equal("PERSON") or
/// type.is_equal("ORGANISATION") or type.is_equal("GROUP") or
/// type.is_equal("AGENT") or type.is_equal("ROLE") or
/// type.is_equal("PARTY") or type.is_equal("ACTOR")`.
///
/// PORT NOTE: the spec's attribute table does **not** mark `OBJECT_REF.type`
/// as `(redefined)` on `PARTY_REF` (contrast with `LOCATABLE_REF.id`, which
/// carries that marker — see `locatable_ref.rs` and ADR-001 §6). The
/// constraint here is expressed purely as an invariant over the inherited
/// `String`-typed `type` field, not a narrowed field type, so `type` stays
/// `String` on [`PartyRef`] (via the embedded [`ObjectRef`]) and this
/// constant plus [`PartyRef::is_type_valid`] record/check the invariant
/// instead of inventing an enum the spec does not declare at the field
/// level.
pub const VALID_TYPES: &[&str] = &[
    "PERSON",
    "ORGANISATION",
    "GROUP",
    "AGENT",
    "ROLE",
    "PARTY",
    "ACTOR",
];

/// `PARTY_REF` declares no new attribute of its own beyond those inherited
/// from `OBJECT_REF`, so it embeds `ObjectRef` verbatim (ADR-001 §3) and
/// constrains the inherited `type` field via the `Type_validity` invariant.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PartyRef {
    /// Embedded `OBJECT_REF` state (`namespace`, `type`, `id`).
    pub object_ref: ObjectRef,
}

impl PartyRef {
    /// Invariant `Type_validity`: `type.is_equal("PERSON") or
    /// type.is_equal("ORGANISATION") or type.is_equal("GROUP") or
    /// type.is_equal("AGENT") or type.is_equal("ROLE") or
    /// type.is_equal("PARTY") or type.is_equal("ACTOR")`.
    ///
    /// TODO(port): not yet wired into a constructor or the RM `Validate`
    /// framework (`.claude/rules/rm-transcription.md` "Invariants"); this
    /// method lets a future `Validate` impl call the check directly once
    /// that framework lands.
    pub fn is_type_valid(&self) -> bool {
        VALID_TYPES.contains(&self.object_ref.r#type.as_str())
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: BASE 1.2.0 base_types.identification §PARTY_REF — docs/research/spec-cache/BASE-1.2.0/uml_classes/party_ref.adoc (Release-1.2.0 @ 9064413)
//   source_loc: master05-identification_package.adoc §Class Descriptions / party_ref.adoc §PARTY_REF Class
//   confidence: medium
//   todos: 1
//   note: Type_validity invariant modelled as a VALID_TYPES const + is_type_valid() check rather than narrowing the `type` field to an enum, since the spec table does not mark OBJECT_REF.type as (redefined) here — ambiguity noted in the transcription report.
// ─────────────────────────────────────────────
