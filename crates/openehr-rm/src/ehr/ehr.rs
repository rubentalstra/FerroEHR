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

// TODO(port): forward-reference — `DV_DATE_TIME` lives in
// rm.data_types.date_time (PORT_MASTER_PLAN.md §7.1), not yet transcribed.
use crate::data_types::date_time::dv_date_time::DvDateTime;

/// Canonical `_type` discriminator string for this class in serialized form.
/// See the note on `ehr_status::TYPE_NAME` for why this is a `const` rather
/// than a `#[serde(rename = ...)]` in this pass.
pub const TYPE_NAME: &str = "EHR";

/// `EHR` — the root object and access point of an EHR for a subject of care.
///
/// Field order and cardinalities follow the published class table exactly
/// (`system_id`, `ehr_id`, `contributions`, `ehr_status`, `ehr_access`,
/// `compositions`, `directory`, `time_created`, `folders`), not the
/// narrative-prose ordering in `master04-ehr_package.adoc`'s "High-level EHR
/// structure" description, which lists `ehr_access`/`ehr_status` before
/// `folders`/`compositions` in a different sequence purely for exposition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ehr {
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
    pub folders: Option<Vec<ObjectRef>>,
}

impl Ehr {
    /// Invariant `Contributions_valid`:
    /// `for_all c in contributions | c.type.is_equal("CONTRIBUTION")`.
    ///
    /// TODO(port): not yet enforced by a constructor/`Validate` impl; awaits
    /// the RM invariant framework (`.claude/rules/rm-transcription.md`
    /// "Invariants").
    pub fn invariant_contributions_valid(&self) -> bool {
        todo!("port: for_all c in contributions | c.type.is_equal(\"CONTRIBUTION\")")
    }

    /// Invariant `Ehr_access_valid`:
    /// `ehr_access.type.is_equal("VERSIONED_EHR_ACCESS")`.
    ///
    /// TODO(port): not yet enforced.
    pub fn invariant_ehr_access_valid(&self) -> bool {
        todo!("port: ehr_access.type.is_equal(\"VERSIONED_EHR_ACCESS\")")
    }

    /// Invariant `Ehr_status_valid`:
    /// `ehr_status.type.is_equal("VERSIONED_EHR_STATUS")`.
    ///
    /// TODO(port): not yet enforced.
    pub fn invariant_ehr_status_valid(&self) -> bool {
        todo!("port: ehr_status.type.is_equal(\"VERSIONED_EHR_STATUS\")")
    }

    /// Invariant `Compositions_valid`:
    /// `for_all c in compositions | c.type.is_equal("VERSIONED_COMPOSITION")`.
    ///
    /// TODO(port): not yet enforced.
    pub fn invariant_compositions_valid(&self) -> bool {
        todo!("port: for_all c in compositions | c.type.is_equal(\"VERSIONED_COMPOSITION\")")
    }

    /// Invariant `Directory_valid`:
    /// `directory /= Void implies directory.type.is_equal("VERSIONED_FOLDER")`.
    ///
    /// TODO(port): not yet enforced.
    pub fn invariant_directory_valid(&self) -> bool {
        todo!("port: directory /= Void implies directory.type.is_equal(\"VERSIONED_FOLDER\")")
    }

    /// Invariant `Folders_valid`:
    /// `folders /= Void implies for_all f in folders | f.type.is_equal("VERSIONED_FOLDER")`.
    ///
    /// TODO(port): not yet enforced.
    pub fn invariant_folders_valid(&self) -> bool {
        todo!(
            "port: folders /= Void implies for_all f in folders | f.type.is_equal(\"VERSIONED_FOLDER\")"
        )
    }

    /// Invariant `Directory_in_folders`:
    /// `folders /= Void implies folders.item(1) = directory`.
    ///
    /// TODO(port): not yet enforced.
    pub fn invariant_directory_in_folders(&self) -> bool {
        todo!("port: folders /= Void implies folders.item(1) = directory")
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 ehr — docs/research/spec-cache/RM-1.1.0/ehr/uml_classes/ehr.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: master04-ehr_package.adoc §Class Descriptions / uml_classes/ehr.adoc §EHR Class
//   confidence: high
//   todos: 8
//   note: EHR has no Inherit row at all (not LOCATABLE, not PATHABLE) — verified against the published table, documented prominently; all 7 class invariants stubbed pending the RM Validate framework.
// ─────────────────────────────────────────────
