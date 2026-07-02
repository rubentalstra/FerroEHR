//! `HIER_OBJECT_ID` — hierarchical identifier.
//!
//! openEHR class: `HIER_OBJECT_ID`, package
//! `base.base_types.identification`.
//! Inherits: `UID_BASED_ID`.
//!
//! Concrete type corresponding to hierarchical identifiers of the form
//! defined by `UID_BASED_ID`: `root '::' extension`. Used both by openEHR
//! and many other organisations, often based on UUIDs or other similar
//! machine-readable and -resolvable schemes.
use super::object_id::{ObjectId, ObjectIdApi};
use super::uid_based_id::{UidBasedId, UidBasedIdApi, UidBasedIdData};

/// Canonical `_type` discriminator string for this class in serialized
/// form (ITS-JSON/ITS-XML), per `.claude/rules/rm-transcription.md`.
///
/// TODO(port): no `serde` dependency is wired into `openehr-base` yet
/// (canonical JSON serialization is a separate phase, P4 —
/// `PORT_MASTER_PLAN.md` §10). `#[serde(rename = "HIER_OBJECT_ID")]` will
/// be added once that dependency and phase land; this constant records the
/// discriminator value in the meantime so the naming decision does not need
/// to be re-derived later.
pub const TYPE_NAME: &str = "HIER_OBJECT_ID";

/// `HIER_OBJECT_ID` declares no attribute or function of its own beyond
/// those inherited from `UID_BASED_ID`, so it embeds `UidBasedIdData`
/// verbatim (ADR-001 §3) and gains `root`/`extension`/`has_extension` via
/// the [`UidBasedIdApi`] default methods.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HierObjectId {
    /// Embedded `UID_BASED_ID` state (the single `value` attribute).
    pub uid_based_id: UidBasedIdData,
}

impl HierObjectId {
    /// `value`: the raw `root [ '::' extension ]` string.
    pub fn value(&self) -> &str {
        &self.uid_based_id.value
    }
}

impl UidBasedIdApi for HierObjectId {
    fn value(&self) -> &str {
        &self.uid_based_id.value
    }
}

impl ObjectIdApi for HierObjectId {
    fn value(&self) -> &str {
        &self.uid_based_id.value
    }
}

impl From<HierObjectId> for UidBasedId {
    fn from(value: HierObjectId) -> Self {
        UidBasedId::HierObjectId(value)
    }
}

impl From<HierObjectId> for ObjectId {
    fn from(value: HierObjectId) -> Self {
        ObjectId::UidBased(UidBasedId::HierObjectId(value))
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: BASE 1.2.0 base_types.identification §HIER_OBJECT_ID — docs/research/spec-cache/BASE-1.2.0/uml_classes/hier_object_id.adoc (Release-1.2.0 @ 9064413)
//   source_loc: master05-identification_package.adoc §Class Descriptions / hier_object_id.adoc §HIER_OBJECT_ID Class
//   confidence: high
//   todos: 1
//   note: _type discriminator recorded as a TYPE_NAME const, not a #[serde(rename)], since serde is not yet a dependency of openehr-base (deferred to P4); pure UID_BASED_ID subtype with no added attributes/functions.
// ─────────────────────────────────────────────
