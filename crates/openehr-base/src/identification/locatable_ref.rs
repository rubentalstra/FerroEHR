//! `LOCATABLE_REF` — reference to a `LOCATABLE` instance inside a
//! versioned content structure.
//!
//! openEHR class: `LOCATABLE_REF`, package
//! `base.base_types.identification`.
//! Inherits: `OBJECT_REF`.
//!
//! Reference to a `LOCATABLE` instance inside the top-level content
//! structure inside a `VERSION<T>`; the path attribute is applied to the
//! object that `VERSION.data` points to.
use super::object_id::ObjectId;
use super::uid_based_id::UidBasedId;

/// Canonical `_type` discriminator string for this class in serialized
/// form. See the `TODO(port)` on `hier_object_id::TYPE_NAME` for why this
/// is a `const` rather than a `#[serde(rename = ...)]` in this pass.
pub const TYPE_NAME: &str = "LOCATABLE_REF";

/// `LOCATABLE_REF` inherits `OBJECT_REF` but the spec's attribute table
/// marks `id` `*1..1 (redefined)*`, narrowing its declared type from
/// `OBJECT_ID` (on `OBJECT_REF`) to `UID_BASED_ID`. Per ADR-001 §6
/// (covariant redefinition → narrowed type on the concrete struct), this
/// struct does **not** embed [`super::object_ref::ObjectRef`] wholesale —
/// doing so would keep `id: ObjectId`, silently losing the narrowing — and
/// instead re-declares `namespace`, `type`, and `id` directly, with `id`
/// typed as [`UidBasedId`] (the enum encoding of `UID_BASED_ID`, ADR-001
/// §4) rather than the wider [`ObjectId`].
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LocatableRef {
    /// `namespace`, inherited unchanged from `OBJECT_REF`. See
    /// `object_ref::ObjectRef::namespace` for the legal-value constraint.
    pub namespace: String,

    /// `type`, inherited unchanged from `OBJECT_REF`.
    ///
    /// PORT NOTE: named `r#type` because `type` is a Rust reserved
    /// keyword, matching `object_ref::ObjectRef::r#type`.
    pub r#type: String,

    /// `id`: globally unique id of an object, regardless of where it is
    /// stored.
    ///
    /// **Covariant redefinition** (ADR-001 §6): the spec's attribute table
    /// marks this `*1..1 (redefined)*`, narrowing the declared type from
    /// `OBJECT_ID` (as declared on the parent `OBJECT_REF`) to
    /// `UID_BASED_ID`. Encoded directly as [`UidBasedId`], not the wider
    /// [`ObjectId`] enum.
    pub id: UidBasedId,

    /// `path`: the path to an instance in question, as an absolute path
    /// with respect to the object found at `VERSION.data`. An empty path
    /// means that the object referred to by `id` is being specified.
    ///
    /// PORT NOTE: the spec's attribute table gives this cardinality `0..1`;
    /// modelled as `Option<String>` rather than an always-present
    /// possibly-empty `String`, since `None` cleanly represents "no path
    /// component was given" while an empty-but-present `Some(String::new())`
    /// is the spec's own "refers to `id` directly" case — both states are
    /// representable and distinguishable this way, matching the `0..1`
    /// cardinality more literally than collapsing them.
    pub path: Option<String>,
}

impl LocatableRef {
    /// `as_uri(): String`.
    ///
    /// A URI form of the reference, created by concatenating:
    /// * scheme, e.g. `ehr:`, derived from `namespace`;
    /// * `id.value`;
    /// * `/` + `path`, where `path` is non-empty.
    ///
    /// TODO(port): the mapping from `namespace` to a URI scheme (e.g.
    /// `namespace = "local"` → `ehr:` in the spec's own example) is not
    /// spelled out in the class table beyond the one worked example; left
    /// as `todo!()` pending clarification of the full namespace→scheme
    /// mapping rather than guessing a general rule from a single example.
    pub fn as_uri(&self) -> String {
        todo!("LocatableRef::as_uri: namespace-to-URI-scheme mapping not fully specified")
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: BASE 1.2.0 base_types.identification §LOCATABLE_REF — docs/research/spec-cache/BASE-1.2.0/uml_classes/locatable_ref.adoc (Release-1.2.0 @ 9064413)
//   source_loc: master05-identification_package.adoc §Class Descriptions / locatable_ref.adoc §LOCATABLE_REF Class
//   confidence: medium
//   todos: 1
//   note: id field is the ADR-001 §6 covariant-redefinition worked example (OBJECT_ID narrowed to UID_BASED_ID), so namespace/type/id are re-declared flat rather than embedding ObjectRef wholesale; as_uri()'s namespace-to-scheme mapping left as todo!() pending a fuller worked example than the spec's single "ehr:" case.
// ─────────────────────────────────────────────
