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
use super::uid_based_id::{UidBasedId, UidBasedIdApi};
use openehr_foundation::serde_support::{TypeName, TypeTag};

/// Canonical `_type` discriminator string for this class in serialized
/// form. `LocatableRef` is not currently reached through any tagged enum in
/// this crate, so the struct-level `#[serde(rename = "LOCATABLE_REF")]`
/// below is inert for this standalone struct under `#[derive(Serialize)]`;
/// see the caveat on `hier_object_id::TYPE_NAME`.
pub const TYPE_NAME: &str = "LOCATABLE_REF";

/// `LOCATABLE_REF` inherits `OBJECT_REF` but the spec's attribute table
/// marks `id` `*1..1 (redefined)*`, narrowing its declared type from
/// `OBJECT_ID` (on `OBJECT_REF`) to `UID_BASED_ID`. Per ADR-001 §6
/// (covariant redefinition → narrowed type on the concrete struct), this
/// struct does **not** embed [`super::object_ref::ObjectRef`] wholesale —
/// doing so would keep `id: ObjectId`, silently losing the narrowing — and
/// instead re-declares `namespace`, `type`, and `id` directly, with `id`
/// typed as [`UidBasedId`] (the enum encoding of `UID_BASED_ID`, ADR-001
/// §4) rather than the wider [`ObjectId`](super::object_id::ObjectId).
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
pub struct LocatableRef {
    /// Canonical `_type` discriminator (`"LOCATABLE_REF"`), always
    /// serialized first; tolerated-absent and validated-if-present on
    /// input (ADR-002).
    #[serde(rename = "_type", default = "TypeTag::new")]
    pub type_tag: TypeTag<Self>,

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
    /// [`ObjectId`](super::object_id::ObjectId) enum.
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
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub path: Option<String>,
}

impl TypeName for LocatableRef {
    const NAME: &'static str = TYPE_NAME;
}

impl LocatableRef {
    /// The URI scheme derived from a `LOCATABLE_REF.namespace` value, used
    /// by [`LocatableRef::as_uri`].
    ///
    /// PORT NOTE (documented reading of an under-specified table): the
    /// spec's `as_uri` row says the scheme is "derived from `namespace`"
    /// but gives only one worked example (`ehr:`). The rule implemented
    /// here is:
    ///
    /// * the special namespaces `"local"` and `"unknown"` map to `ehr` —
    ///   a `LOCATABLE_REF` always addresses content inside a
    ///   `VERSION.data` of an EHR, so the local/unqualified case is the
    ///   spec's own `ehr:` example;
    /// * any other namespace is used as the scheme verbatim when it is
    ///   itself lexically a legal RFC 3986 scheme
    ///   (`[a-zA-Z][a-zA-Z0-9+.-]*`) — the identity mapping being the only
    ///   derivation the table's wording supports without inventing a
    ///   registry;
    /// * a namespace that cannot serve as a scheme (legal `OBJECT_REF`
    ///   namespaces admit `:`/`/`/`?` etc., which RFC 3986 schemes do not)
    ///   falls back to `ehr`.
    #[must_use]
    fn scheme_from_namespace(namespace: &str) -> &str {
        fn is_rfc3986_scheme(candidate: &str) -> bool {
            let mut chars = candidate.chars();
            chars
                .next()
                .is_some_and(|first| first.is_ascii_alphabetic())
                && chars.all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '+' | '.' | '-'))
        }

        match namespace {
            "local" | "unknown" => "ehr",
            other if is_rfc3986_scheme(other) => other,
            _ => "ehr",
        }
    }

    /// `as_uri(): String`.
    ///
    /// A URI form of the reference, created by concatenating:
    /// * scheme, e.g. `ehr:`, derived from `namespace` (see
    ///   [`LocatableRef::scheme_from_namespace`]);
    /// * `id.value`;
    /// * `/` + `path`, where `path` is non-empty.
    ///
    /// PORT NOTE: the table's third bullet prepends `/` to a non-empty
    /// `path`, but RM paths are themselves absolute (they already begin
    /// with `/`); the separator is therefore added only when the stored
    /// path does not already start with one, so the spec's intended
    /// single-`/` join is produced instead of a spurious `//`. An absent
    /// (`None`) or empty path contributes nothing, per the class's own
    /// "empty path means the object referred to by `id`" wording.
    pub fn as_uri(&self) -> String {
        let mut uri = format!(
            "{}:{}",
            Self::scheme_from_namespace(&self.namespace),
            self.id.value()
        );
        if let Some(path) = self.path.as_deref()
            && !path.is_empty()
        {
            if !path.starts_with('/') {
                uri.push('/');
            }
            uri.push_str(path);
        }
        uri
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identification::hier_object_id::HierObjectId;
    use crate::identification::uid_based_id::UidBasedIdData;

    fn id(value: &str) -> UidBasedId {
        UidBasedId::HierObjectId(HierObjectId {
            type_tag: TypeTag::new(),
            uid_based_id: UidBasedIdData {
                value: value.to_string(),
            },
        })
    }

    fn locatable_ref(namespace: &str, id_value: &str, path: Option<&str>) -> LocatableRef {
        LocatableRef {
            type_tag: TypeTag::new(),
            namespace: namespace.to_string(),
            r#type: "COMPOSITION".to_string(),
            id: id(id_value),
            path: path.map(str::to_string),
        }
    }

    #[test]
    fn as_uri_uses_the_spec_example_ehr_scheme_for_local() {
        let reference = locatable_ref(
            "local",
            "87284370-2D4B-4e3d-A3F3-F303D2F4F34B::uk.nhs.ehr1",
            None,
        );
        assert_eq!(
            reference.as_uri(),
            "ehr:87284370-2D4B-4e3d-A3F3-F303D2F4F34B::uk.nhs.ehr1"
        );
    }

    #[test]
    fn as_uri_appends_a_non_empty_path_with_a_single_separator() {
        let reference = locatable_ref("unknown", "uk.nhs.ehr1", Some("/content[at0001]"));
        assert_eq!(reference.as_uri(), "ehr:uk.nhs.ehr1/content[at0001]");

        let relative = locatable_ref("unknown", "uk.nhs.ehr1", Some("content[at0001]"));
        assert_eq!(relative.as_uri(), "ehr:uk.nhs.ehr1/content[at0001]");
    }

    #[test]
    fn as_uri_ignores_an_empty_or_absent_path() {
        let empty = locatable_ref("local", "uk.nhs.ehr1", Some(""));
        assert_eq!(empty.as_uri(), "ehr:uk.nhs.ehr1");
        let absent = locatable_ref("local", "uk.nhs.ehr1", None);
        assert_eq!(absent.as_uri(), "ehr:uk.nhs.ehr1");
    }

    #[test]
    fn as_uri_derives_the_scheme_from_scheme_shaped_namespaces() {
        let reference = locatable_ref("demographic", "uk.nhs.ehr1", None);
        assert_eq!(reference.as_uri(), "demographic:uk.nhs.ehr1");

        // A legal OBJECT_REF namespace that is not a legal URI scheme
        // falls back to the spec's own "ehr" example.
        let fallback = locatable_ref("ns:sub/x", "uk.nhs.ehr1", None);
        assert_eq!(fallback.as_uri(), "ehr:uk.nhs.ehr1");
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: BASE 1.2.0 base_types.identification §LOCATABLE_REF — docs/research/spec-cache/BASE-1.2.0/uml_classes/locatable_ref.adoc (Release-1.2.0 @ 9064413)
//   source_loc: master05-identification_package.adoc §Class Descriptions / locatable_ref.adoc §LOCATABLE_REF Class
//   confidence: medium
//   todos: 0
//   note: id field is the ADR-001 §6 covariant-redefinition worked example (OBJECT_ID narrowed to UID_BASED_ID), so namespace/type/id are re-declared flat rather than embedding ObjectRef wholesale; as_uri() implemented per the spec's concatenation with a PORT-NOTEd namespace→scheme reading (local/unknown→ehr, scheme-shaped namespaces verbatim, ehr fallback).
// ─────────────────────────────────────────────
