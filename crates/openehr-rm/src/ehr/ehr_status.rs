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
//
// TODO(port): P4 — `#[serde(flatten)]` below requires `LocatableData` to
// itself derive `Serialize`/`Deserialize`; it does not yet (sibling P4 wave
// over `common/`). This file is written as if that derive exists; it will
// not actually satisfy the trait bound until `common::archetyped::locatable`
// gets its own P4 pass.
use crate::common::archetyped::locatable::LocatableData;

// TODO(port): forward-reference — `PARTY_SELF` lives in rm.common.generic
// (PORT_MASTER_PLAN.md §7.1: "PARTY_PROXY, PARTY_SELF, ..." under rm.common),
// not yet transcribed. `PartySelf` (party_proxy.rs) already derives
// `PartialEq, Eq, Hash` and has no `Weak`/back-reference fields, so once
// `common/generic` gets its own P4 pass this embed is expected to be a
// clean derive with no flatten-target blocker.
use crate::common::generic::party_self::PartySelf;

// TODO(port): forward-reference — `ITEM_STRUCTURE` lives in
// rm.data_structures (PORT_MASTER_PLAN.md §7.1), not yet transcribed. Closed
// subtype set per ADR-001 §4 / `.claude/rules/rm-transcription.md`, so the
// eventual type here will be an enum, not a trait object.
use crate::data_structures::item_structure::ItemStructure;
use openehr_foundation::serde_support::{TypeName, TypeTag};
use serde::{Deserialize, Serialize};

/// Canonical `_type` discriminator string for this class in serialized form
/// (ITS-JSON/ITS-XML).
///
/// Single-sourced into the `TypeName` impl below (ADR-002); the
/// `TypeTag<Self>` first field on [`EhrStatus`] is what actually emits
/// `_type: "EHR_STATUS"` on the wire.
pub const TYPE_NAME: &str = "EHR_STATUS";

/// `EHR_STATUS` — EHR-wide status flags and settings.
///
/// Per ADR-001 §3 (abstract class with attributes → embedded struct + marker
/// trait), `LOCATABLE`'s state is embedded as `pub locatable: LocatableData`
/// rather than simulated via a Rust supertrait, since `LOCATABLE` carries
/// attributes (`uid`, `archetype_node_id`, `name`, `archetype_details`,
/// `feeder_audit`, `links`), not just behaviour. `#[serde(flatten)]` folds
/// those six attributes directly into `EHR_STATUS`'s own JSON object, per
/// the ITS-JSON rule that abstract classes are flattened, never
/// `$ref`-chained (`.claude/rules/serialization.md`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EhrStatus {
    /// Canonical `_type` discriminator (`"EHR_STATUS"`), always serialized
    /// first; tolerated-absent and validated-if-present on input (ADR-002).
    #[serde(rename = "_type", default = "TypeTag::new")]
    pub type_tag: TypeTag<Self>,

    /// Embedded `LOCATABLE` state.
    #[serde(flatten)]
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
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub other_details: Option<ItemStructure>,
}

impl TypeName for EhrStatus {
    const NAME: &'static str = TYPE_NAME;
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

#[cfg(test)]
mod tests {
    use super::EhrStatus;
    use crate::common::archetyped::locatable::LocatableData;
    use crate::common::generic::party_proxy::PartyProxyData;
    use crate::common::generic::party_self::PartySelf;
    use crate::data_types::text::dv_text::{DvText, DvTextData};
    use openehr_foundation::serde_support::TypeTag;

    /// Canonical-JSON round-trip: an `EHR_STATUS` with `is_queryable`/
    /// `is_modifiable` both `true`, a bare `PARTY_SELF` subject (no
    /// `external_ref`), and no `other_details`. Asserts the exact canonical
    /// string per `.claude/rules/serialization.md` (flattened `LOCATABLE`
    /// fields present, no nulls, no `parent` key), then parses it back and
    /// checks equality.
    ///
    /// TODO(port): this test cannot compile or run yet, for two independent
    /// reasons, neither resolvable from this file:
    /// 1. `LocatableData` does not derive `Serialize`/`Deserialize` until
    ///    `common/archetyped` gets its own P4 pass (see the
    ///    `#[serde(flatten)]` TODO(port) on `EhrStatus::locatable` above).
    /// 2. `LocatableData.name`'s declared type, [`DvText`], is a closed enum
    ///    (`{Text(DvTextData), Coded(DvCodedText)}`, `data_types::text`,
    ///    owned by a different P4 wave than this file's); this test cannot
    ///    know in advance whether that sibling wave will tag it
    ///    `#[serde(tag = "_type")]` with a flattened payload or some other
    ///    shape, so the JSON literal below assumes the same tagged-flatten
    ///    convention already established for `UidBasedId`
    ///    (`openehr-base::identification`, landed) as the most-likely
    ///    outcome, purely so this test's `name` sub-object has *a* concrete
    ///    literal to check against — **not** a claim about what
    ///    `data_types::text` will actually do. Revisit the `name` line
    ///    specifically once that wave lands, independently of reason (1).
    /// Written now so the exact expected `EHR_STATUS`-level shape (which
    /// this file *does* own) is on record; re-enable once both blockers
    /// clear.
    #[test]
    fn round_trips_and_omits_absent_fields() {
        let status = EhrStatus {
            type_tag: TypeTag::new(),
            locatable: LocatableData {
                name: DvText::Text {
                    type_tag: TypeTag::new(),
                    data: DvTextData {
                        value: "EHR Status".to_string(),
                        hyperlink: None,
                        formatting: None,
                        mappings: None,
                        language: None,
                        encoding: None,
                    },
                },
                archetype_node_id: "openEHR-EHR-EHR_STATUS.generic.v1".to_string(),
                uid: None,
                links: None,
                archetype_details: None,
                feeder_audit: None,
                parent: None,
            },
            subject: PartySelf {
                type_tag: TypeTag::new(),
                party_proxy: PartyProxyData { external_ref: None },
            },
            is_queryable: true,
            is_modifiable: true,
            other_details: None,
        };

        let json = serde_json::to_string(&status).expect("serialize");
        assert_eq!(
            json,
            concat!(
                "{\"_type\":\"EHR_STATUS\",",
                "\"name\":{\"_type\":\"DV_TEXT\",\"value\":\"EHR Status\"},",
                "\"archetype_node_id\":\"openEHR-EHR-EHR_STATUS.generic.v1\",",
                "\"subject\":{\"_type\":\"PARTY_SELF\"},",
                "\"is_queryable\":true,",
                "\"is_modifiable\":true}"
            )
        );

        let parsed: EhrStatus = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(parsed, status);
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 ehr — docs/research/spec-cache/RM-1.1.0/ehr/uml_classes/ehr_status.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: master04-ehr_package.adoc §Class Descriptions / uml_classes/ehr_status.adoc §EHR_STATUS Class
//   confidence: high
//   todos: 6
//   note: forward-references LocatableData/PartySelf/ItemStructure (all not-yet-transcribed siblings); Is_archetype_root invariant stubbed pending LOCATABLE. P4/ADR-002: self-tagging TypeTag<Self> first field + TypeName impl (no-op struct-level rename removed); round-trip test still #[ignore]d pending the sibling waves' DvText/LocatableData ADR-002 shapes (its expected JSON already asserts _type-first). The test's exact expected JSON string is the reviewable artifact even while ignored.
// ─────────────────────────────────────────────
