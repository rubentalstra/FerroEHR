//! `ARCHETYPE_HRID` — human-readable archetype identifier.
//!
//! openEHR class: `ARCHETYPE_HRID`.
//!
//! This class belongs to the archetype identifier surface rather than the
//! core RM object graph, but the pinned ITS-JSON schema includes it. The
//! fields mirror that schema definition directly.
use crate::definitions::version_status::VersionStatus;
use openehr_foundation::serde_support::{TypeName, TypeTag};

/// `ARCHETYPE_HRID` — canonical human-readable archetype id components.
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct ArchetypeHrid {
    /// Canonical `_type` discriminator (`"ARCHETYPE_HRID"`), always
    /// serialized first; tolerated-absent and validated-if-present on
    /// input (ADR-002).
    #[serde(rename = "_type", default = "TypeTag::new")]
    pub type_tag: TypeTag<Self>,

    pub namespace: String,
    pub rm_publisher: String,
    pub rm_package: String,
    pub rm_class: String,
    pub concept_id: String,
    pub release_version: String,
    pub version_status: VersionStatus,
    pub build_count: String,
}

impl TypeName for ArchetypeHrid {
    const NAME: &'static str = "ARCHETYPE_HRID";
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: ITS-JSON pinned commit 5acae056248e917a4b4c56f7e712f4fcfeb616a6 definition ARCHETYPE_HRID
//   source_loc: openehr_rm_1.1.0_all.json#/definitions/ARCHETYPE_HRID
//   confidence: medium
//   todos: 0
//   note: schema-coverage type for the AM/archetype-id surface exposed by the pinned ITS-JSON bundle; fields mirror the schema definition so VERSION_STATUS is validated through its object-form serde.
// ─────────────────────────────────────────────
