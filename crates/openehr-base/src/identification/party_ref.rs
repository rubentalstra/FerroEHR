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
use super::object_id::ObjectId;
use openehr_foundation::serde_support::{TypeName, TypeTag};

/// Canonical `_type` discriminator string for this class in serialized
/// form. `PartyRef` is not currently reached through any tagged enum in
/// this crate, so the struct-level `#[serde(rename = "PARTY_REF")]` below
/// is inert for this standalone struct under `#[derive(Serialize)]`; see
/// the caveat on `hier_object_id::TYPE_NAME`.
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
///
/// `#[serde(flatten)]` on the embedded `object_ref` field folds
/// `ObjectRef`'s three attributes (`namespace`, `type`, `id`) directly into
/// this struct's JSON object rather than nesting them under an `object_ref`
/// key.
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
pub struct PartyRef {
    /// Canonical `_type` discriminator (`"PARTY_REF"`), always
    /// serialized first; tolerated-absent and validated-if-present on
    /// input (ADR-002).
    #[serde(rename = "_type", default = "TypeTag::new")]
    pub type_tag: TypeTag<Self>,

    /// `namespace`, inherited unchanged from `OBJECT_REF`.
    ///
    /// PORT NOTE (ADR-002): `PARTY_REF` previously embedded the full
    /// [`ObjectRef`] via `#[serde(flatten)]`, but `OBJECT_REF` is itself a
    /// concrete, self-tagged class — flattening it leaks an inner
    /// `_type: "OBJECT_REF"` that collides with this struct's own tag. The
    /// three inherited fields are therefore re-declared directly, matching
    /// the `locatable_ref.rs` precedent.
    pub namespace: String,

    /// `type`, inherited unchanged from `OBJECT_REF`. Constrained by the
    /// `Type_validity` invariant (see [`VALID_TYPES`]).
    #[serde(rename = "type")]
    pub r#type: String,

    /// `id`, inherited unchanged from `OBJECT_REF`.
    pub id: ObjectId,
}

impl TypeName for PartyRef {
    const NAME: &'static str = TYPE_NAME;
}

/// Error raised by [`PartyRef::new`].
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PartyRefError {
    /// The inherited `namespace` attribute fails `OBJECT_REF`'s
    /// legal-value pattern (see
    /// [`super::object_ref::is_namespace_valid`]).
    #[error(
        "invalid PARTY_REF namespace {0:?}: must match [a-zA-Z][a-zA-Z0-9_.:\\/&?=+-]* (spec legal values)"
    )]
    InvalidNamespace(String),
    /// The `type` attribute violates the `Type_validity` invariant (must
    /// be one of [`VALID_TYPES`]).
    #[error(
        "invalid PARTY_REF type {0:?}: Type_validity requires one of PERSON, ORGANISATION, GROUP, AGENT, ROLE, PARTY, ACTOR"
    )]
    InvalidType(String),
}

impl PartyRef {
    /// Fallible constructor enforcing the `Type_validity` invariant plus
    /// the inherited `OBJECT_REF.namespace` legal-value constraint
    /// (ADR-003 decision 8: cheap invariants move into fallible
    /// constructors now; the deep walker/accumulator validation framework
    /// remains the P11 deliverable). Struct-literal construction remains
    /// possible for unchecked wire data and is re-checkable via
    /// [`PartyRef::is_type_valid`].
    pub fn new(
        namespace: impl Into<String>,
        r#type: impl Into<String>,
        id: ObjectId,
    ) -> Result<Self, PartyRefError> {
        let namespace = namespace.into();
        if !super::object_ref::is_namespace_valid(&namespace) {
            return Err(PartyRefError::InvalidNamespace(namespace));
        }
        let r#type = r#type.into();
        if !VALID_TYPES.contains(&r#type.as_str()) {
            return Err(PartyRefError::InvalidType(r#type));
        }
        Ok(Self {
            type_tag: TypeTag::new(),
            namespace,
            r#type,
            id,
        })
    }

    /// Invariant `Type_validity`: `type.is_equal("PERSON") or
    /// type.is_equal("ORGANISATION") or type.is_equal("GROUP") or
    /// type.is_equal("AGENT") or type.is_equal("ROLE") or
    /// type.is_equal("PARTY") or type.is_equal("ACTOR")`.
    ///
    /// Enforced at construction by [`PartyRef::new`]; kept as a working
    /// re-check so the P11 `Validate` framework (and unchecked
    /// struct-literal values) can call it directly.
    pub fn is_type_valid(&self) -> bool {
        VALID_TYPES.contains(&self.r#type.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identification::generic_id::GenericId;
    use crate::identification::object_id::ObjectIdData;

    fn some_id() -> ObjectId {
        ObjectId::GenericId(GenericId {
            type_tag: TypeTag::new(),
            object_id: ObjectIdData {
                value: "id-1".to_string(),
            },
            scheme: "test".to_string(),
        })
    }

    #[test]
    fn new_accepts_every_spec_listed_party_type() {
        for r#type in VALID_TYPES {
            let party_ref = PartyRef::new("demographic", *r#type, some_id());
            assert!(party_ref.is_ok(), "expected {type:?} to be accepted");
            assert!(party_ref.is_ok_and(|p| p.is_type_valid()));
        }
    }

    #[test]
    fn new_rejects_type_validity_violations() {
        // Type_validity is an exact, case-sensitive is_equal comparison.
        assert_eq!(
            PartyRef::new("demographic", "person", some_id()),
            Err(PartyRefError::InvalidType("person".to_string()))
        );
        assert_eq!(
            PartyRef::new("demographic", "GUIDELINE", some_id()),
            Err(PartyRefError::InvalidType("GUIDELINE".to_string()))
        );
    }

    #[test]
    fn new_rejects_inherited_namespace_violations() {
        assert_eq!(
            PartyRef::new("9bad", "PERSON", some_id()),
            Err(PartyRefError::InvalidNamespace("9bad".to_string()))
        );
    }

    #[test]
    fn is_type_valid_re_checks_unchecked_values() {
        let mut party_ref = PartyRef::new("local", "PERSON", some_id()).expect("valid");
        assert!(party_ref.is_type_valid());
        party_ref.r#type = "WIDGET".to_string();
        assert!(!party_ref.is_type_valid());
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: BASE 1.2.0 base_types.identification §PARTY_REF — docs/research/spec-cache/BASE-1.2.0/uml_classes/party_ref.adoc (Release-1.2.0 @ 9064413)
//   source_loc: master05-identification_package.adoc §Class Descriptions / party_ref.adoc §PARTY_REF Class
//   confidence: high
//   todos: 0
//   note: Type_validity invariant modelled as a VALID_TYPES const + is_type_valid() check rather than narrowing the `type` field to an enum, since the spec table does not mark OBJECT_REF.type as (redefined) here; PartyRef::new enforces Type_validity + the inherited namespace pattern (ADR-003 §8).
// ─────────────────────────────────────────────
