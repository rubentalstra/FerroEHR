//! `ACCESS_GROUP_REF` — legacy reference to an access-control group.
//!
//! openEHR class: `ACCESS_GROUP_REF`, package
//! `base.base_types.identification`.
//!
//! The BASE 1.2.0 plan records this as a settled legacy hazard, but the
//! pinned ITS-JSON schema still exposes the class. It has the same wire
//! attributes as `OBJECT_REF`, so it is represented as its own concrete
//! reference type rather than weakening the coverage harness.
use super::object_id::ObjectId;
use openehr_foundation::serde_support::{TypeName, TypeTag};

/// `ACCESS_GROUP_REF` — a namespace, type name, and identifier for an
/// access-control group reference.
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
pub struct AccessGroupRef {
    /// Canonical `_type` discriminator (`"ACCESS_GROUP_REF"`), always
    /// serialized first; tolerated-absent and validated-if-present on
    /// input (ADR-002).
    #[serde(rename = "_type", default = "TypeTag::new")]
    pub type_tag: TypeTag<Self>,

    /// Identifier of the referenced access group.
    pub id: ObjectId,

    /// Namespace to which this identifier belongs.
    pub namespace: String,

    /// Type name of the referenced access-control object.
    pub r#type: String,
}

impl TypeName for AccessGroupRef {
    const NAME: &'static str = "ACCESS_GROUP_REF";
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: BASE 1.2.0 legacy identification class + ITS-JSON pinned commit 5acae056248e917a4b4c56f7e712f4fcfeb616a6 definition ACCESS_GROUP_REF
//   source_loc: docs/research/spec-cache/BASE-1.2.0/uml_classes/access_group_ref.adoc; openehr_rm_1.1.0_all.json#/definitions/ACCESS_GROUP_REF
//   confidence: medium
//   todos: 0
//   note: legacy class retained only because the pinned ITS-JSON schema requires coverage; same structural fields as OBJECT_REF, but kept as its own concrete type so `_type` dispatch remains exact.
// ─────────────────────────────────────────────
