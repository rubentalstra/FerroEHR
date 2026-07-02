//! `OBJECT_VERSION_ID` — globally unique identifier for one version of a
//! versioned object.
//!
//! openEHR class: `OBJECT_VERSION_ID`, package
//! `base.base_types.identification`.
//! Inherits: `UID_BASED_ID`.
//!
//! Globally unique identifier for one version of a versioned object;
//! lexical form: `object_id '::' creating_system_id '::' version_tree_id`.
use openehr_foundation::serde_support::{TypeName, TypeTag};

use super::object_id::{ObjectId, ObjectIdApi};
use super::uid::Uid;
use super::uid_based_id::{UidBasedId, UidBasedIdApi, UidBasedIdData};
use super::version_tree_id::VersionTreeId;

/// Canonical `_type` discriminator string for this class in serialized
/// form. See the P4/ADR-002 note on `hier_object_id::TYPE_NAME` — this
/// const single-sources the string carried by the struct's own self-tagging
/// `type_tag` field (via the [`TypeName`] impl).
pub const TYPE_NAME: &str = "OBJECT_VERSION_ID";

/// `OBJECT_VERSION_ID` declares no attribute of its own beyond the
/// inherited `value: String` from `UID_BASED_ID`; it layers three
/// additional functions (`object_id`, `creating_system_id`,
/// `version_tree_id`) plus `is_branch` on top of that single attribute, so
/// it embeds `UidBasedIdData` verbatim (ADR-001 §3) rather than adding new
/// fields.
///
/// `#[serde(flatten)]` on the embedded `uid_based_id` field folds
/// `UidBasedIdData`'s single `value` attribute directly into this struct's
/// JSON object, matching the convention on `HierObjectId`, so an
/// `ObjectVersionId` serializes as `{"_type": "OBJECT_VERSION_ID",
/// "value": "..."}` (ADR-002 self-tag).
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
pub struct ObjectVersionId {
    /// Canonical `_type` discriminator (`"OBJECT_VERSION_ID"`), always
    /// serialized first; tolerated-absent and validated-if-present on input
    /// (ADR-002).
    #[serde(rename = "_type", default = "TypeTag::new")]
    pub type_tag: TypeTag<Self>,

    /// Embedded `UID_BASED_ID` state (the single `value` attribute), in
    /// the lexical form `object_id '::' creating_system_id '::'
    /// version_tree_id`.
    #[serde(flatten)]
    pub uid_based_id: UidBasedIdData,
}

impl TypeName for ObjectVersionId {
    const NAME: &'static str = TYPE_NAME;
}

impl ObjectVersionId {
    /// `value`: the raw `object_id '::' creating_system_id '::'
    /// version_tree_id` string.
    pub fn value(&self) -> &str {
        &self.uid_based_id.value
    }

    /// `object_id(): UID`.
    ///
    /// Unique identifier for the logical object of which this identifier
    /// identifies one version; normally the `object_id` will be the unique
    /// identifier of the version container containing the version referred
    /// to by this `OBJECT_VERSION_ID` instance. The first of the three
    /// `::`-separated parts.
    ///
    /// TODO(port): returning a parsed [`Uid`] requires the same
    /// format-sniffing UID sub-parser noted as `todo!()` on
    /// `UidBasedIdApi::root` in `uid_based_id.rs`.
    pub fn object_id(&self) -> Uid {
        todo!("ObjectVersionId::object_id: format-sniffing UID parser not yet implemented")
    }

    /// `creating_system_id(): UID`.
    ///
    /// Identifier of the system that created the Version corresponding to
    /// this Object version id. The second of the three `::`-separated
    /// parts.
    ///
    /// TODO(port): see [`ObjectVersionId::object_id`] — same deferred UID
    /// sub-parser.
    pub fn creating_system_id(&self) -> Uid {
        todo!("ObjectVersionId::creating_system_id: format-sniffing UID parser not yet implemented")
    }

    /// `version_tree_id(): VERSION_TREE_ID`.
    ///
    /// Tree identifier of this version with respect to other versions in
    /// the same version tree, as either 1 or 3 part dot-separated numbers,
    /// e.g. `1`, `2.1.4`. The third of the three `::`-separated parts.
    pub fn version_tree_id(&self) -> VersionTreeId {
        let raw = self
            .uid_based_id
            .value
            .rsplit_once("::")
            .map(|(_head, tail)| tail.to_string())
            .unwrap_or_default();
        VersionTreeId {
            type_tag: TypeTag::new(),
            value: raw,
        }
    }

    /// `is_branch(): Boolean`.
    ///
    /// True if this version identifier represents a branch.
    ///
    /// Delegates to `VERSION_TREE_ID.is_branch()` per the identification
    /// package's description of the version-tree-id component.
    pub fn is_branch(&self) -> bool {
        self.version_tree_id().is_branch()
    }
}

impl UidBasedIdApi for ObjectVersionId {
    fn value(&self) -> &str {
        &self.uid_based_id.value
    }
}

impl ObjectIdApi for ObjectVersionId {
    fn value(&self) -> &str {
        &self.uid_based_id.value
    }
}

impl From<ObjectVersionId> for UidBasedId {
    fn from(value: ObjectVersionId) -> Self {
        UidBasedId::ObjectVersionId(value)
    }
}

impl From<ObjectVersionId> for ObjectId {
    fn from(value: ObjectVersionId) -> Self {
        ObjectId::UidBased(UidBasedId::ObjectVersionId(value))
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: BASE 1.2.0 base_types.identification §OBJECT_VERSION_ID — docs/research/spec-cache/BASE-1.2.0/uml_classes/object_version_id.adoc (Release-1.2.0 @ 9064413)
//   source_loc: master05-identification_package.adoc §Class Descriptions / object_version_id.adoc §OBJECT_VERSION_ID Class
//   confidence: medium
//   todos: 2
//   note: object_id()/creating_system_id() need the same deferred format-sniffing UID parser as UID_BASED_ID.root(); version_tree_id() parsing implemented directly since VERSION_TREE_ID's own value is just the raw substring, no UID sub-parse required. P4/ADR-002: self-tags via TypeTag<Self> first field (NAME single-sourced from TYPE_NAME); inert struct-level #[serde(rename)] deleted.
// ─────────────────────────────────────────────
