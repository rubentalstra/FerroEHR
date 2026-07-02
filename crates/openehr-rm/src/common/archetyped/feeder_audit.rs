//! `FEEDER_AUDIT` — audit trail describing the origin of feeder-system
//! data committed into openEHR form.
//!
//! openEHR class: `FEEDER_AUDIT` (concrete), package `common.archetyped`.
//!
//! The data in any part of the EHR may be obtained from a feeder system,
//! i.e. a source system which does not obey the versioning, auditing and
//! content semantics of openEHR. The `FEEDER_AUDIT` class defines the
//! semantics of an audit trail which is constructed to describe the origin
//! of data that have been transformed into openEHR form and committed to
//! the system.
//!
//! Feeder audit information is attached to the `LOCATABLE` class via the
//! `feeder_audit` attribute (see [`super::locatable::LocatableData::feeder_audit`]),
//! even though it is preferable by design to have it attached to the
//! equivalent of Compositions or at least the equivalent of archetype
//! entities. Its usual usage is to attach it to the outermost object to
//! which it applies.

// TODO(port): `DV_IDENTIFIER`, `DV_ENCAPSULATED` are RM 1.1.0
// `data_types.basic`/`data_types.encapsulated`, transcribed by a sibling
// agent in this same phase but not yet landed in this worktree.
// Forward-references to their eventual module paths.
use crate::data_types::basic::dv_identifier::DvIdentifier;
use crate::data_types::encapsulated::dv_encapsulated::DvEncapsulated;
use openehr_foundation::serde_support::{TypeName, TypeTag};
use serde::{Deserialize, Serialize};

use super::feeder_audit_details::FeederAuditDetails;

/// Canonical `_type` discriminator string for this class in serialized
/// form. Single-sources the [`TypeName`] impl below (ADR-002).
pub const TYPE_NAME: &str = "FEEDER_AUDIT";

/// `FEEDER_AUDIT` declares no `Inherit` row in the spec table.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FeederAudit {
    /// Canonical `_type` discriminator (`"FEEDER_AUDIT"`), always
    /// serialized first; tolerated-absent and validated-if-present on
    /// input (ADR-002).
    #[serde(rename = "_type", default = "TypeTag::new")]
    pub type_tag: TypeTag<Self>,

    /// `originating_system_item_ids`: `List<DV_IDENTIFIER>`, cardinality
    /// `0..1`.
    ///
    /// Identifiers used for the item in the originating system, e.g.
    /// filler and placer ids.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub originating_system_item_ids: Option<Vec<DvIdentifier>>,

    /// `feeder_system_item_ids`: `List<DV_IDENTIFIER>`, cardinality
    /// `0..1`.
    ///
    /// Identifiers used for the item in the feeder system, where the
    /// feeder system is distinct from the originating system.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub feeder_system_item_ids: Option<Vec<DvIdentifier>>,

    /// `original_content`: `DV_ENCAPSULATED`, cardinality `0..1`.
    ///
    /// Optional inline inclusion of or reference to original content
    /// corresponding to the openEHR content at this node. Typically a URI
    /// reference to a document or message in a persistent store associated
    /// with the EHR.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub original_content: Option<DvEncapsulated>,

    /// `originating_system_audit`: `FEEDER_AUDIT_DETAILS`, cardinality
    /// `1..1`.
    ///
    /// Any audit information for the information item from the
    /// originating system.
    pub originating_system_audit: FeederAuditDetails,

    /// `feeder_system_audit`: `FEEDER_AUDIT_DETAILS`, cardinality `0..1`.
    ///
    /// Any audit information for the information item from the feeder
    /// system, if different from the originating system.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub feeder_system_audit: Option<FeederAuditDetails>,
}

impl TypeName for FeederAudit {
    const NAME: &'static str = TYPE_NAME;
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 common.archetyped — docs/research/spec-cache/RM-1.1.0/uml_classes/feeder_audit.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: common/master03-archetyped_package.adoc §Feeder System Audit / uml_classes/feeder_audit.adoc §FEEDER_AUDIT Class
//   confidence: high
//   todos: 0
//   note: Forward-refs DvIdentifier and DvEncapsulated (data_types, sibling-agent territory, not yet landed). No invariants published for this class. P4/ADR-002: self-tags via TypeName + first-field TypeTag<Self> (_type = "FEEDER_AUDIT").
// ─────────────────────────────────────────────
