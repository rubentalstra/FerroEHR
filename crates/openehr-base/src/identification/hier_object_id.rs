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
/// P4 update: `openehr-base` now depends on `serde`
/// (`PORT_MASTER_PLAN.md` §10), and `UidBasedId::HierObjectId` (the enum
/// variant most call sites reach this struct through — see
/// `uid_based_id.rs`) now carries `#[serde(rename = "HIER_OBJECT_ID")]`, so
/// the `_type` tag is emitted whenever a `HierObjectId` is serialized via
/// the `UidBasedId`/`ObjectId`/`Uid`-adjacent enum wrappers that reach it.
/// This `const` is kept alongside the struct-level `#[serde(rename)]` below
/// (added purely for documentation/consistency with the sibling `resource/`
/// classes — a struct-level rename has no serialization effect for a
/// standalone, non-enum-wrapped struct under `#[derive(Serialize)]`) as the
/// stable place other code can reference the discriminator string without
/// depending on serde internals.
pub const TYPE_NAME: &str = "HIER_OBJECT_ID";

/// `HIER_OBJECT_ID` declares no attribute or function of its own beyond
/// those inherited from `UID_BASED_ID`, so it embeds `UidBasedIdData`
/// verbatim (ADR-001 §3) and gains `root`/`extension`/`has_extension` via
/// the [`UidBasedIdApi`] default methods.
///
/// `#[serde(flatten)]` on the embedded `uid_based_id` field folds
/// `UidBasedIdData`'s single `value` attribute directly into this struct's
/// JSON object, so a bare `HierObjectId` serializes as `{"value": "..."}`
/// (or, tagged via the `UidBasedId` enum, `{"_type": "HIER_OBJECT_ID",
/// "value": "..."}`).
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
#[serde(rename = "HIER_OBJECT_ID")]
pub struct HierObjectId {
    /// Embedded `UID_BASED_ID` state (the single `value` attribute).
    #[serde(flatten)]
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Round-trip check for the canonical `{"_type": "...", "value": "..."}`
    /// UID shape (`.claude/rules/serialization.md`), exercised through the
    /// `UidBasedId::HierObjectId` variant so the `_type` tag is actually
    /// emitted (a bare, non-enum-wrapped `HierObjectId` would not carry
    /// `_type` on the wire — see the P4 doc note on `TYPE_NAME` above).
    #[test]
    fn hier_object_id_round_trips_through_uid_based_id_as_canonical_json() {
        let hier_object_id = HierObjectId {
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
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: BASE 1.2.0 base_types.identification §HIER_OBJECT_ID — docs/research/spec-cache/BASE-1.2.0/uml_classes/hier_object_id.adoc (Release-1.2.0 @ 9064413)
//   source_loc: master05-identification_package.adoc §Class Descriptions / hier_object_id.adoc §HIER_OBJECT_ID Class
//   confidence: high
//   todos: 0
//   note: P4 — serde derives added (openehr-base now genuinely depends on serde, not just decorative derive paths); _type is emitted via #[serde(rename)] on the UidBasedId::HierObjectId enum variant (functional) plus a matching struct-level rename on HierObjectId itself (inert outside an enum wrapper, kept for documentation/precedent-consistency with resource/); pure UID_BASED_ID subtype with no added attributes/functions. Round-trip unit test added exercising the exact canonical-JSON UID shape.
// ─────────────────────────────────────────────
