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
use openehr_foundation::serde_support::{TypeName, TypeTag};

use super::object_id::{ObjectId, ObjectIdApi};
use super::uid_based_id::{UidBasedId, UidBasedIdApi, UidBasedIdData};

/// Canonical `_type` discriminator string for this class in serialized
/// form (ITS-JSON/ITS-XML), per `.claude/rules/rm-transcription.md`.
///
/// P4/ADR-002 update: this const single-sources the string carried by the
/// struct's own self-tagging `type_tag` field below (via the [`TypeName`]
/// impl), so every serialized `HierObjectId` — bare or reached through the
/// `UidBasedId`/`ObjectId` enum wrappers — emits
/// `{"_type": "HIER_OBJECT_ID", ...}` itself.
pub const TYPE_NAME: &str = "HIER_OBJECT_ID";

/// `HIER_OBJECT_ID` declares no attribute or function of its own beyond
/// those inherited from `UID_BASED_ID`, so it embeds `UidBasedIdData`
/// verbatim (ADR-001 §3) and gains `root`/`extension`/`has_extension` via
/// the [`UidBasedIdApi`] default methods.
///
/// `#[serde(flatten)]` on the embedded `uid_based_id` field folds
/// `UidBasedIdData`'s single `value` attribute directly into this struct's
/// JSON object, so a `HierObjectId` serializes as
/// `{"_type": "HIER_OBJECT_ID", "value": "..."}` (ADR-002 self-tag).
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
pub struct HierObjectId {
    /// Canonical `_type` discriminator (`"HIER_OBJECT_ID"`), always
    /// serialized first; tolerated-absent and validated-if-present on input
    /// (ADR-002).
    #[serde(rename = "_type", default = "TypeTag::new")]
    pub type_tag: TypeTag<Self>,

    /// Embedded `UID_BASED_ID` state (the single `value` attribute).
    #[serde(flatten)]
    pub uid_based_id: UidBasedIdData,
}

impl TypeName for HierObjectId {
    const NAME: &'static str = TYPE_NAME;
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Round-trip check for the canonical `{"_type": "...", "value": "..."}`
    /// UID shape (`.claude/rules/serialization.md`), exercised through the
    /// (now untagged) `UidBasedId::HierObjectId` variant — the `_type` tag
    /// comes from the payload's own `TypeTag` (ADR-002), and untagged
    /// variant probing on input is tag-driven.
    #[test]
    fn hier_object_id_round_trips_through_uid_based_id_as_canonical_json() {
        let hier_object_id = HierObjectId {
            type_tag: TypeTag::new(),
            uid_based_id: UidBasedIdData {
                value: "8849182c-82ad-4088-a07f-48ead4180515".to_string(),
            },
        };
        let wrapped: UidBasedId = hier_object_id.clone().into();

        let json = serde_json::to_string(&wrapped).expect("serialize UidBasedId::HierObjectId");
        assert_eq!(
            json,
            r#"{"_type":"HIER_OBJECT_ID","value":"8849182c-82ad-4088-a07f-48ead4180515"}"#
        );

        let parsed: UidBasedId =
            serde_json::from_str(&json).expect("deserialize UidBasedId::HierObjectId");
        assert_eq!(parsed, wrapped);
        assert_eq!(parsed.value(), hier_object_id.value());
    }

    /// ADR-002 self-tag: a bare, non-enum-wrapped `HierObjectId` now emits
    /// its own `_type` (first in key order) and round-trips exactly.
    #[test]
    fn bare_hier_object_id_self_tags_and_round_trips() {
        let hier_object_id = HierObjectId {
            type_tag: TypeTag::new(),
            uid_based_id: UidBasedIdData {
                value: "ehr.example.com::1234".to_string(),
            },
        };

        let json = serde_json::to_string(&hier_object_id).expect("serialize bare HierObjectId");
        assert_eq!(
            json,
            r#"{"_type":"HIER_OBJECT_ID","value":"ehr.example.com::1234"}"#
        );

        let parsed: HierObjectId =
            serde_json::from_str(&json).expect("deserialize bare HierObjectId");
        assert_eq!(parsed, hier_object_id);

        // A wrong `_type` in a concrete-declared slot is rejected.
        let wrong: Result<HierObjectId, _> =
            serde_json::from_str(r#"{"_type":"OBJECT_VERSION_ID","value":"x"}"#);
        assert!(wrong.is_err());
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: BASE 1.2.0 base_types.identification §HIER_OBJECT_ID — docs/research/spec-cache/BASE-1.2.0/uml_classes/hier_object_id.adoc (Release-1.2.0 @ 9064413)
//   source_loc: master05-identification_package.adoc §Class Descriptions / hier_object_id.adoc §HIER_OBJECT_ID Class
//   confidence: high
//   todos: 0
//   note: pure UID_BASED_ID subtype with no added attributes/functions. P4/ADR-002: self-tags via TypeTag<Self> first field (NAME single-sourced from TYPE_NAME); inert struct-level #[serde(rename)] deleted; round-trip unit tests pin the exact canonical-JSON UID shape both bare and through the untagged UidBasedId enum.
// ─────────────────────────────────────────────
