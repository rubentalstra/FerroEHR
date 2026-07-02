//! `FEEDER_AUDIT_DETAILS` — audit details for one system in a feeder
//! chain.
//!
//! openEHR class: `FEEDER_AUDIT_DETAILS` (concrete), package
//! `common.archetyped`.
//!
//! Audit details for any system in a feeder system chain. Audit details
//! here means the general notion of who/where/when the information item
//! to which the audit is attached was created. None of the attributes is
//! defined as mandatory, however, in different scenarios, various
//! combinations of attributes will usually be mandatory. This can be
//! controlled by specifying feeder audit details in legacy archetypes.

// TODO(port): `DV_DATE_TIME` is RM 1.1.0 `data_types.date_time`,
// transcribed by a sibling agent in this same phase but not yet landed in
// this worktree. Forward-reference to its eventual module path.
use crate::data_types::date_time::dv_date_time::DvDateTime;
// TODO(port): `ITEM_STRUCTURE` is RM 1.1.0 `data_structures.item_structure`
// (abstract, closed subtype set per ADR-001 §4), transcribed by a sibling
// agent in this same phase but not yet landed in this worktree.
use crate::data_structures::item_structure::ItemStructure;

use crate::common::generic::party_identified::PartyIdentified;
use crate::common::generic::party_proxy::PartyProxy;

/// Canonical `_type` discriminator string for this class in serialized
/// form. Per ADR-001 refinements ("serde derives wait until P4"), a
/// `const` stands in for `#[serde(rename = ...)]` until serde lands as a
/// dependency of this crate.
pub const TYPE_NAME: &str = "FEEDER_AUDIT_DETAILS";

/// `FEEDER_AUDIT_DETAILS` declares no `Inherit` row in the spec table.
#[derive(Debug, Clone, PartialEq)]
pub struct FeederAuditDetails {
    /// `system_id`: `String`, cardinality `1..1`.
    ///
    /// Identifier of the system which handled the information item. This
    /// is the IT system owned by the organisation legally responsible for
    /// handling the data, and at which the data were previously created or
    /// passed by an earlier system.
    ///
    /// Invariant `System_id_valid`: `not system_id.is_empty`.
    ///
    /// TODO(port): invariant not yet enforced by a constructor/`Validate`
    /// impl; recorded here as a doc note pending the RM invariant
    /// framework (`.claude/rules/rm-transcription.md` "Invariants").
    pub system_id: String,

    /// `location`: `PARTY_IDENTIFIED`, cardinality `0..1`.
    ///
    /// Identifier of the particular site/facility within an organisation
    /// which handled the item. For computability, this identifier needs to
    /// be e.g. a PKI identifier which can be included in the identifier
    /// list of the `PARTY_IDENTIFIED` object.
    pub location: Option<PartyIdentified>,

    /// `subject`: `PARTY_PROXY`, cardinality `0..1`.
    ///
    /// Identifiers for subject of the received information item.
    pub subject: Option<PartyProxy>,

    /// `provider`: `PARTY_IDENTIFIED`, cardinality `0..1`.
    ///
    /// Optional provider(s) who created, committed, forwarded or otherwise
    /// handled the item.
    pub provider: Option<PartyIdentified>,

    /// `time`: `DV_DATE_TIME`, cardinality `0..1`.
    ///
    /// Time of handling the item. For an originating system, this will be
    /// time of creation, for an intermediate feeder system, this will be a
    /// time of accession or other time of handling, where available.
    pub time: Option<DvDateTime>,

    /// `version_id`: `String`, cardinality `0..1`.
    ///
    /// Any identifier used in the system such as "interim", "final", or
    /// numeric versions if available.
    pub version_id: Option<String>,

    /// `other_details`: `ITEM_STRUCTURE`, cardinality `0..1`.
    ///
    /// Optional attribute to carry any custom meta-data. May be
    /// archetyped.
    pub other_details: Option<ItemStructure>,
}

impl FeederAuditDetails {
    /// Invariant `System_id_valid`: `not system_id.is_empty`.
    ///
    /// TODO(port): not yet wired into a constructor or the RM `Validate`
    /// framework; this method lets a future `Validate` impl call the check
    /// directly once that framework lands.
    pub fn is_system_id_valid(&self) -> bool {
        !self.system_id.is_empty()
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 common.archetyped — docs/research/spec-cache/RM-1.1.0/uml_classes/feeder_audit_details.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: common/master03-archetyped_package.adoc §Meta-data / uml_classes/feeder_audit_details.adoc §FEEDER_AUDIT_DETAILS Class
//   confidence: high
//   todos: 1
//   note: System_id_valid invariant recorded as is_system_id_valid() but not yet Validate-enforced. Forward-refs DvDateTime and ItemStructure (data_types/data_structures, sibling-agent territory, not yet landed); PartyIdentified/PartyProxy reference this same task's sibling module (common::generic), written later in this same pass.
// ─────────────────────────────────────────────
