//! `EHR` — the root object and access point of an EHR for a subject of care.
//!
//! openEHR class: `EHR`, package `rm.ehr`.
//!
//! **No `Inherit` row.** The published RM 1.1.0 class table for `EHR`
//! (`docs/research/spec-cache/RM-1.1.0/ehr/uml_classes/ehr.adoc`) has no
//! `Inherit` heading at all — `EHR` is the one class in the `ehr` package
//! that is neither `LOCATABLE` nor even `PATHABLE`. It is not archetyped and
//! carries no `uid`/`archetype_node_id`/`name` triple. This is deliberate,
//! not an omission on this pass's part: `EHR` is described in
//! `master04-ehr_package.adoc` as "simply act[ing] as an access point for
//! the component parts of the EHR" — the substantive, archetypable content
//! lives one level down, in `EHR_STATUS`, `EHR_ACCESS`, and the
//! `VERSIONED_COMPOSITION`s it references. Accordingly `Ehr` below embeds no
//! `LocatableData` and no `PathableData`.
//!
//! Ground truth: `docs/research/spec-cache/RM-1.1.0/ehr/uml_classes/ehr.adoc`
//! (RM Release-1.1.0 @ 3cbd85b).

// TODO(port): forward-reference — `openehr-base::identification` is already
// transcribed (Phase 01), so this import is real, not a stand-in.
use openehr_base::identification::hier_object_id::HierObjectId;
use openehr_base::identification::object_ref::ObjectRef;
use openehr_foundation::serde_support::{TypeName, TypeTag};
use serde::{Deserialize, Serialize};

// TODO(port): forward-reference — `DV_DATE_TIME` lives in
// rm.data_types.date_time (PORT_MASTER_PLAN.md §7.1), not yet transcribed.
use crate::data_types::date_time::dv_date_time::DvDateTime;

/// Canonical `_type` discriminator string for this class in serialized form.
///
/// Single-sourced into the `TypeName` impl below (ADR-002); the
/// `TypeTag<Self>` first field on [`Ehr`] is what actually emits
/// `_type: "EHR"` on the wire (the former struct-level
/// `#[serde(rename = "EHR")]` was a verified no-op and has been deleted).
pub const TYPE_NAME: &str = "EHR";

/// `EHR` — the root object and access point of an EHR for a subject of care.
///
/// Field order and cardinalities follow the published class table exactly
/// (`system_id`, `ehr_id`, `contributions`, `ehr_status`, `ehr_access`,
/// `compositions`, `directory`, `time_created`, `folders`), not the
/// narrative-prose ordering in `master04-ehr_package.adoc`'s "High-level EHR
/// structure" description, which lists `ehr_access`/`ehr_status` before
/// `folders`/`compositions` in a different sequence purely for exposition.
// Eq dropped from the derive set: `time_created` is a `DV_DATE_TIME`, whose
// embedded quantity chain transitively carries `f64` accuracy fields
// (`PartialEq` only).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Ehr {
    /// Canonical `_type` discriminator (`"EHR"`), always serialized first;
    /// tolerated-absent and validated-if-present on input (ADR-002).
    #[serde(rename = "_type", default = "TypeTag::new")]
    pub type_tag: TypeTag<Self>,

    /// `system_id`: the identifier of the logical EHR management system in
    /// which this EHR was created.
    ///
    /// Cardinality: `1..1`. Immutable after creation (spec narrative,
    /// "Root EHR Object").
    pub system_id: HierObjectId,

    /// `ehr_id`: the unique identifier of this EHR.
    ///
    /// Cardinality: `1..1`. Immutable after creation.
    ///
    /// NOTE (spec): it is strongly recommended that a UUID always be used
    /// for this field.
    pub ehr_id: HierObjectId,

    /// `contributions`: list of contributions causing changes to this EHR.
    /// Each contribution contains a list of versions, which may include
    /// references to any number of `VERSION` instances, i.e. items of type
    /// `VERSIONED_COMPOSITION` and `VERSIONED_FOLDER`.
    ///
    /// Cardinality: `0..1` in the published table (`List<OBJECT_REF>`,
    /// optional). Modelled as `Option<Vec<ObjectRef>>` per the `List<T>` →
    /// `Vec<T>` convention (`resource::authored_resource` doc comment) and
    /// the table's own `0..1` row.
    ///
    /// Invariant `Contributions_valid`:
    /// `for_all c in contributions | c.type.is_equal("CONTRIBUTION")`.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub contributions: Option<Vec<ObjectRef>>,

    /// `ehr_status`: reference to `EHR_STATUS` object for this EHR.
    ///
    /// Cardinality: `1..1`.
    ///
    /// Invariant `Ehr_status_valid`:
    /// `ehr_status.type.is_equal("VERSIONED_EHR_STATUS")`.
    pub ehr_status: ObjectRef,

    /// `ehr_access`: reference to `EHR_ACCESS` object for this EHR.
    ///
    /// Cardinality: `1..1`.
    ///
    /// Invariant `Ehr_access_valid`:
    /// `ehr_access.type.is_equal("VERSIONED_EHR_ACCESS")`.
    pub ehr_access: ObjectRef,

    /// `compositions`: master list of all Versioned Composition references
    /// in this EHR.
    ///
    /// Cardinality: `0..1`.
    ///
    /// Invariant `Compositions_valid`:
    /// `for_all c in compositions | c.type.is_equal("VERSIONED_COMPOSITION")`.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub compositions: Option<Vec<ObjectRef>>,

    /// `directory`: optional directory structure for this EHR. If present,
    /// this is a reference to the first member of `folders`.
    ///
    /// Cardinality: `0..1`.
    ///
    /// Invariant `Directory_valid`:
    /// `directory /= Void implies directory.type.is_equal("VERSIONED_FOLDER")`.
    ///
    /// Invariant `Directory_in_folders`:
    /// `folders /= Void implies folders.item(1) = directory`.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub directory: Option<ObjectRef>,

    /// `time_created`: time of creation of the EHR.
    ///
    /// Cardinality: `1..1`. Immutable after creation.
    pub time_created: DvDateTime,

    /// `folders`: optional additional Folder structures for this EHR. If
    /// set, the `directory` attribute refers to the first member.
    ///
    /// Cardinality: `0..1`.
    ///
    /// PORT NOTE: this attribute (and its coexistence with `directory`) is
    /// an RM 1.1.0 addition over prior releases —
    /// `master04-ehr_package.adoc` §Folders states: "If `_folders_` is not
    /// Void, the `_directory_` attribute always contains a reference to the
    /// first member, for backward compatibility with pre-Release 1.1.0
    /// systems." Transcribed as declared in the 1.1.0 table, including both
    /// fields and the `Directory_in_folders` invariant linking them.
    ///
    /// Invariant `Folders_valid`:
    /// `folders /= Void implies for_all f in folders | f.type.is_equal("VERSIONED_FOLDER")`.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub folders: Option<Vec<ObjectRef>>,
}

impl TypeName for Ehr {
    const NAME: &'static str = TYPE_NAME;
}

/// `true` if every `OBJECT_REF` in `refs` (an optional `List<OBJECT_REF>`)
/// has `type = type_name`. A `None` list is `for_all` over the empty set,
/// which is vacuously true, matching the `0..1` cardinality of the
/// `contributions`/`compositions`/`folders` attributes.
fn all_refs_of_type(refs: Option<&Vec<ObjectRef>>, type_name: &str) -> bool {
    refs.is_none_or(|list| list.iter().all(|r| r.r#type == type_name))
}

impl Ehr {
    /// Invariant `Contributions_valid`:
    /// `for_all c in contributions | c.type.is_equal("CONTRIBUTION")`.
    ///
    /// Type-name check (ADR-003 §8), reading each `OBJECT_REF.type`.
    #[must_use]
    pub fn invariant_contributions_valid(&self) -> bool {
        all_refs_of_type(self.contributions.as_ref(), "CONTRIBUTION")
    }

    /// Invariant `Ehr_access_valid`:
    /// `ehr_access.type.is_equal("VERSIONED_EHR_ACCESS")`.
    #[must_use]
    pub fn invariant_ehr_access_valid(&self) -> bool {
        self.ehr_access.r#type == "VERSIONED_EHR_ACCESS"
    }

    /// Invariant `Ehr_status_valid`:
    /// `ehr_status.type.is_equal("VERSIONED_EHR_STATUS")`.
    #[must_use]
    pub fn invariant_ehr_status_valid(&self) -> bool {
        self.ehr_status.r#type == "VERSIONED_EHR_STATUS"
    }

    /// Invariant `Compositions_valid`:
    /// `for_all c in compositions | c.type.is_equal("VERSIONED_COMPOSITION")`.
    #[must_use]
    pub fn invariant_compositions_valid(&self) -> bool {
        all_refs_of_type(self.compositions.as_ref(), "VERSIONED_COMPOSITION")
    }

    /// Invariant `Directory_valid`:
    /// `directory /= Void implies directory.type.is_equal("VERSIONED_FOLDER")`.
    #[must_use]
    pub fn invariant_directory_valid(&self) -> bool {
        self.directory
            .as_ref()
            .is_none_or(|d| d.r#type == "VERSIONED_FOLDER")
    }

    /// Invariant `Folders_valid`:
    /// `folders /= Void implies for_all f in folders | f.type.is_equal("VERSIONED_FOLDER")`.
    #[must_use]
    pub fn invariant_folders_valid(&self) -> bool {
        all_refs_of_type(self.folders.as_ref(), "VERSIONED_FOLDER")
    }

    /// Invariant `Directory_in_folders`:
    /// `folders /= Void implies folders.item(1) = directory`.
    ///
    /// `folders.item(1)` is the first (1-indexed) member; when `folders` is
    /// present, `directory` must reference it (see the `folders`/`directory`
    /// doc comments and the RM 1.1.0 backward-compatibility note).
    #[must_use]
    pub fn invariant_directory_in_folders(&self) -> bool {
        match &self.folders {
            None => true,
            Some(folders) => self.directory.as_ref() == folders.first(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use openehr_base::identification::object_id::ObjectId;
    use openehr_base::identification::uid_based_id::{UidBasedId, UidBasedIdData};

    fn hier(value: &str) -> HierObjectId {
        HierObjectId {
            type_tag: TypeTag::new(),
            uid_based_id: UidBasedIdData {
                value: value.to_string(),
            },
        }
    }

    fn object_ref(r#type: &str) -> ObjectRef {
        ObjectRef {
            type_tag: TypeTag::new(),
            namespace: "local".to_string(),
            r#type: r#type.to_string(),
            id: ObjectId::UidBased(UidBasedId::HierObjectId(hier(
                "8849182c-82ad-4088-a07f-48ead4180515",
            ))),
        }
    }

    fn date_time(value: &str) -> DvDateTime {
        serde_json::from_value(serde_json::json!({ "value": value }))
            .expect("test DV_DATE_TIME literal deserializes")
    }

    fn ehr() -> Ehr {
        Ehr {
            type_tag: TypeTag::new(),
            system_id: hier("openEHR.system"),
            ehr_id: hier("7d44b88c-4199-4bad-97dc-d78268e01398"),
            contributions: None,
            ehr_status: object_ref("VERSIONED_EHR_STATUS"),
            ehr_access: object_ref("VERSIONED_EHR_ACCESS"),
            compositions: None,
            directory: None,
            time_created: date_time("2020-01-01T00:00:00"),
            folders: None,
        }
    }

    #[test]
    fn reference_type_invariants_check_object_ref_type_names() {
        let mut e = ehr();
        assert!(e.invariant_ehr_status_valid());
        assert!(e.invariant_ehr_access_valid());
        assert!(e.invariant_contributions_valid()); // None: vacuously true
        assert!(e.invariant_compositions_valid());

        e.ehr_status = object_ref("SOMETHING_ELSE");
        assert!(!e.invariant_ehr_status_valid());

        e.compositions = Some(vec![object_ref("VERSIONED_COMPOSITION")]);
        assert!(e.invariant_compositions_valid());
        e.compositions = Some(vec![object_ref("NOT_A_COMPOSITION")]);
        assert!(!e.invariant_compositions_valid());
    }

    #[test]
    fn directory_in_folders_requires_directory_to_be_the_first_folder() {
        let mut e = ehr();
        // folders = None: vacuously true.
        assert!(e.invariant_directory_in_folders());
        assert!(e.invariant_folders_valid());
        assert!(e.invariant_directory_valid());

        let folder = object_ref("VERSIONED_FOLDER");
        e.folders = Some(vec![folder.clone()]);
        e.directory = Some(folder.clone());
        assert!(e.invariant_directory_in_folders());
        assert!(e.invariant_folders_valid());
        assert!(e.invariant_directory_valid());

        // directory points elsewhere → invariant fails.
        e.directory = Some(object_ref("SOME_OTHER_FOLDER"));
        assert!(!e.invariant_directory_in_folders());
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 ehr — docs/research/spec-cache/RM-1.1.0/ehr/uml_classes/ehr.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: master04-ehr_package.adoc §Class Descriptions / uml_classes/ehr.adoc §EHR Class
//   confidence: high
//   todos: 2
//   note: EHR has no Inherit row at all (not LOCATABLE, not PATHABLE) — verified against the published table, documented prominently. P5/ADR-003 §8: all 7 class invariants implemented as OBJECT_REF type-name checks (Contributions/Ehr_access/Ehr_status/Compositions/Directory/Folders_valid + Directory_in_folders), pinned by unit tests. Remaining 2 TODO(port) are the two forward-ref import comments. P4/ADR-002: self-tagging TypeTag<Self> first field + TypeName impl.
// ─────────────────────────────────────────────
