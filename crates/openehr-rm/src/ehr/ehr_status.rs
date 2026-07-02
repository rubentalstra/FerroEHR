//! `EHR_STATUS` — EHR-wide status flags and settings.
//!
//! openEHR class: `EHR_STATUS`, package `rm.ehr`.
//! Inherits: `LOCATABLE`.
//!
//! Single object per EHR containing various EHR-wide status flags and
//! settings, including whether this EHR can be queried, modified etc. This
//! object is always modifiable, in order to change the status of the EHR as
//! a whole.
//!
//! NOTE (spec): it is strongly recommended that the inherited attribute
//! `_uid_` be populated in `EHR_STATUS` objects, using the UID copied from
//! the `object_id()` of the `_uid_` field of the enclosing `VERSION` object.
//! For example, the `ORIGINAL_VERSION.uid`
//! `87284370-2D4B-4e3d-A3F3-F303D2F4F34B::uk.nhs.ehr1::2` would be copied to
//! the `_uid_` field of the `EHR_STATUS` object.
//!
//! Ground truth: `docs/research/spec-cache/RM-1.1.0/ehr/uml_classes/ehr_status.adoc`
//! (RM Release-1.1.0 @ 3cbd85b).

// TODO(port): forward-reference — `common` package (rm.common) is not yet
// transcribed (Phase 03 task order places `ehr` after `common`, but this
// class is being transcribed ahead of it per an explicit invocation). Path
// per ADR-001 §9 (one class per spec-package directory); `LocatableData` is
// the ADR-001 §3 embedded-struct half of the abstract `LOCATABLE` class.
use crate::common::archetyped::locatable::LocatableData;

// TODO(port): forward-reference — `PARTY_SELF` lives in rm.common.generic
// (PORT_MASTER_PLAN.md §7.1: "PARTY_PROXY, PARTY_SELF, ..." under rm.common),
// not yet transcribed.
use crate::common::generic::party_self::PartySelf;

// TODO(port): forward-reference — `ITEM_STRUCTURE` lives in
// rm.data_structures (PORT_MASTER_PLAN.md §7.1), not yet transcribed. Closed
// subtype set per ADR-001 §4 / `.claude/rules/rm-transcription.md`, so the
// eventual type here will be an enum, not a trait object.
use crate::data_structures::item_structure::ItemStructure;

/// Canonical `_type` discriminator string for this class in serialized form
/// (ITS-JSON/ITS-XML). `openehr-rm` has no `serde` dependency wired in yet
/// (canonical JSON/XML serialization is Phases 04-05 —
/// `PORT_MASTER_PLAN.md` §10); this constant records the discriminator value
/// in the meantime, matching the `TYPE_NAME` convention already used in
/// `openehr-base` (see e.g. `identification::hier_object_id::TYPE_NAME`).
pub const TYPE_NAME: &str = "EHR_STATUS";

/// `EHR_STATUS` — EHR-wide status flags and settings.
///
/// Per ADR-001 §3 (abstract class with attributes → embedded struct + marker
/// trait), `LOCATABLE`'s state is embedded as `pub locatable: LocatableData`
/// rather than simulated via a Rust supertrait, since `LOCATABLE` carries
/// attributes (`uid`, `archetype_node_id`, `name`, `archetype_details`,
/// `feeder_audit`, `links`), not just behaviour.
#[derive(Debug, Clone, PartialEq)]
pub struct EhrStatus {
    /// Embedded `LOCATABLE` state.
    pub locatable: LocatableData,

    /// `subject`: the subject of this EHR. The `_external_ref_` attribute
    /// can be used to contain a direct reference to the subject in a
    /// demographic or identity service. Alternatively, the association
    /// between patients and their records may be done elsewhere for
    /// security reasons.
    ///
    /// Cardinality: `1..1`.
    pub subject: PartySelf,

    /// `is_queryable`: `True` if this EHR should be included in population
    /// queries, i.e. if this EHR is considered active in the population.
    ///
    /// Cardinality: `1..1`.
    pub is_queryable: bool,

    /// `is_modifiable`: `True` if the EHR, other than the `EHR_STATUS`
    /// object, is allowed to be written to. The `EHR_STATUS` object itself
    /// can always be written to.
    ///
    /// Cardinality: `1..1`.
    pub is_modifiable: bool,

    /// `other_details`: any other details of the EHR summary object, in the
    /// form of an archetyped `ITEM_STRUCTURE`.
    ///
    /// Cardinality: `0..1`.
    pub other_details: Option<ItemStructure>,
}

impl EhrStatus {
    /// Invariant `Is_archetype_root`: `is_archetype_root`.
    ///
    /// The spec's sole class invariant for `EHR_STATUS` is inherited
    /// unchanged from `LOCATABLE` (`is_archetype_root`), restated here so
    /// its presence on this class is not lost during transcription.
    ///
    /// TODO(port): delegates to `LOCATABLE.is_archetype_root()`, not yet
    /// implemented; awaits the `common::archetyped::locatable` transcription
    /// and the RM invariant framework (`.claude/rules/rm-transcription.md`
    /// "Invariants").
    pub fn invariant_is_archetype_root(&self) -> bool {
        todo!(
            "port: delegate to LocatableData::is_archetype_root() once common::archetyped::locatable lands"
        )
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 ehr — docs/research/spec-cache/RM-1.1.0/ehr/uml_classes/ehr_status.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: master04-ehr_package.adoc §Class Descriptions / uml_classes/ehr_status.adoc §EHR_STATUS Class
//   confidence: high
//   todos: 4
//   note: forward-references LocatableData/PartySelf/ItemStructure (all not-yet-transcribed siblings); Is_archetype_root invariant stubbed pending LOCATABLE.
// ─────────────────────────────────────────────
