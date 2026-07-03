//! `OBJECT_REF` — reference to another object.
//!
//! openEHR class: `OBJECT_REF`, package `base.base_types.identification`.
//!
//! Class describing a reference to another object, which may exist locally
//! or be maintained outside the current namespace, e.g. in another
//! service. Services are usually external, e.g. available in a LAN
//! (including on the same host) or the internet via Corba, SOAP, or some
//! other distributed protocol. However, in small systems they may be part
//! of the same executable as the data containing the Id.
use super::object_id::ObjectId;
use openehr_foundation::serde_support::{TypeName, TypeTag};

/// Canonical `_type` discriminator string for this class in serialized
/// form. `ObjectRef` is embedded by value (not enum-wrapped) into
/// `PartyRef`, so — as with the `OBJECT_ID` leaves in this package — the
/// struct-level `#[serde(rename = "OBJECT_REF")]` below is inert for a
/// standalone struct under `#[derive(Serialize)]`; see the caveat on
/// `hier_object_id::TYPE_NAME`.
pub const TYPE_NAME: &str = "OBJECT_REF";

/// `OBJECT_REF` — a namespace, a type name, and the `OBJECT_ID` of the
/// referenced object.
///
/// `PARTY_REF` and `LOCATABLE_REF` both inherit `OBJECT_REF`; per ADR-001
/// §3 they embed `ObjectRef` by composition rather than a Rust trait-based
/// inheritance simulation. `LOCATABLE_REF` additionally redefines the `id`
/// field's type (`OBJECT_ID` narrowed to `UID_BASED_ID`) — see
/// `locatable_ref.rs` and ADR-001 §6.
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
pub struct ObjectRef {
    /// Canonical `_type` discriminator (`"OBJECT_REF"`), always
    /// serialized first; tolerated-absent and validated-if-present on
    /// input (ADR-002).
    #[serde(rename = "_type", default = "TypeTag::new")]
    pub type_tag: TypeTag<Self>,

    /// `namespace`: namespace to which this identifier belongs in the
    /// local system context (and possibly in any other openEHR compliant
    /// environment) e.g. `terminology`, `demographic`. These names are not
    /// yet standardised.
    ///
    /// Legal values, per the class description: `"local"`, `"unknown"`, or
    /// a string matching the standard regex
    /// `[a-zA-Z][a-zA-Z0-9_.:\/&?=+-]*` (the first two values are just
    /// special cases already matched by the regex).
    ///
    /// Enforced by [`ObjectRef::new`] (ADR-003 decision 8: cheap invariants
    /// move into fallible constructors now; the deep walker/accumulator
    /// validation framework remains the P11 deliverable). Struct-literal
    /// construction remains possible for unchecked wire data and is
    /// re-checkable via [`is_namespace_valid`].
    pub namespace: String,

    /// `type`: name of the class (concrete or abstract) of object to which
    /// this identifier type refers, e.g. `PARTY`, `PERSON`, `GUIDELINE`
    /// etc. These class names are from the relevant reference model. The
    /// type name `ANY` can be used to indicate that any type is accepted
    /// (e.g. if the type is unknown).
    ///
    /// PORT NOTE: named `r#type` because `type` is a Rust reserved keyword;
    /// the field is still serialized/documented under the spec's own name
    /// `type`.
    pub r#type: String,

    /// `id`: globally unique id of an object, regardless of where it is
    /// stored.
    pub id: ObjectId,
}

impl TypeName for ObjectRef {
    const NAME: &'static str = TYPE_NAME;
}

/// Error raised by [`ObjectRef::new`] (and reused by the `OBJECT_REF`
/// descendants `PARTY_REF` / `LOCATABLE_REF` for their inherited
/// `namespace` check).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ObjectRefError {
    /// The `namespace` attribute does not match the spec's legal-value
    /// pattern `[a-zA-Z][a-zA-Z0-9_.:\/&?=+-]*`.
    #[error(
        "invalid OBJECT_REF namespace {0:?}: must match [a-zA-Z][a-zA-Z0-9_.:\\/&?=+-]* (spec legal values)"
    )]
    InvalidNamespace(String),
}

/// `true` when `namespace` matches the spec's legal-value pattern for
/// `OBJECT_REF.namespace`: `[a-zA-Z][a-zA-Z0-9_.:\/&?=+-]*`.
///
/// The spec's two named special values `"local"` and `"unknown"` are, as
/// the class description itself notes, already matched by the pattern, so
/// no separate case is needed.
///
/// PORT NOTE: implemented with plain character checks rather than the
/// `regex` crate — the pattern is a single character class, so a dependency
/// is not warranted.
#[must_use]
pub fn is_namespace_valid(namespace: &str) -> bool {
    let mut chars = namespace.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !first.is_ascii_alphabetic() {
        return false;
    }
    chars.all(|ch| {
        ch.is_ascii_alphanumeric()
            || matches!(ch, '_' | '.' | ':' | '/' | '&' | '?' | '=' | '+' | '-')
    })
}

impl ObjectRef {
    /// Fallible constructor enforcing the `namespace` legal-value
    /// constraint (ADR-003 decision 8).
    ///
    /// PORT NOTE: the spec's `OBJECT_REF` declares no constructor; this is
    /// the standing Java-constructor-that-throws → `fn new(...) ->
    /// Result<Self, E>` idiom applied to the class's declared value
    /// constraint (`docs/PORTING.md` §2/§5).
    pub fn new(
        namespace: impl Into<String>,
        r#type: impl Into<String>,
        id: ObjectId,
    ) -> Result<Self, ObjectRefError> {
        let namespace = namespace.into();
        if !is_namespace_valid(&namespace) {
            return Err(ObjectRefError::InvalidNamespace(namespace));
        }
        Ok(Self {
            type_tag: TypeTag::new(),
            namespace,
            r#type: r#type.into(),
            id,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identification::generic_id::GenericId;
    use crate::identification::object_id::ObjectId;

    fn some_id() -> ObjectId {
        ObjectId::GenericId(GenericId {
            type_tag: TypeTag::new(),
            object_id: crate::identification::object_id::ObjectIdData {
                value: "id-1".to_string(),
            },
            scheme: "test".to_string(),
        })
    }

    #[test]
    fn namespace_legal_values_follow_the_spec_pattern() {
        // The two named special values are plain matches of the pattern.
        assert!(is_namespace_valid("local"));
        assert!(is_namespace_valid("unknown"));
        // Regular pattern matches, including every punctuation character
        // the class description's regex allows.
        assert!(is_namespace_valid("terminology"));
        assert!(is_namespace_valid("a"));
        assert!(is_namespace_valid("ns_1.sub:part/x&y?q=v+w-z"));
        // Rejections: empty, leading non-alpha, illegal characters.
        assert!(!is_namespace_valid(""));
        assert!(!is_namespace_valid("9abc"));
        assert!(!is_namespace_valid("_abc"));
        assert!(!is_namespace_valid("has space"));
        assert!(!is_namespace_valid("ünïcode"));
    }

    #[test]
    fn new_enforces_namespace_validity() {
        assert!(ObjectRef::new("demographic", "PARTY", some_id()).is_ok());
        assert_eq!(
            ObjectRef::new("1bad", "PARTY", some_id()),
            Err(ObjectRefError::InvalidNamespace("1bad".to_string()))
        );
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: BASE 1.2.0 base_types.identification §OBJECT_REF — docs/research/spec-cache/BASE-1.2.0/uml_classes/object_ref.adoc (Release-1.2.0 @ 9064413)
//   source_loc: master05-identification_package.adoc §Class Descriptions / object_ref.adoc §OBJECT_REF Class
//   confidence: high
//   todos: 0
//   note: namespace legal-value pattern now enforced by ObjectRef::new + is_namespace_valid (ADR-003 §8); type field named r#type since `type` is a Rust keyword — verified empirically that serde strips the `r#` prefix automatically (field serializes as "type" with no explicit rename needed). No Option fields on this class; all three attributes are 1..1.
// ─────────────────────────────────────────────
