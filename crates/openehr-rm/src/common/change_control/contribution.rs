//! `CONTRIBUTION` — a change-set of one or more versions.
//!
//! openEHR class: `CONTRIBUTION`, package `common.change_control`.
//!
//! Documents a Contribution (change set) of one or more versions added to
//! a change-controlled repository. Contributions are similar to nested
//! transactions in database management terms: an attempt to commit a
//! Contribution should only succeed if each Version and/or Attestation in
//! the Contribution is committed successfully.
use openehr_base::identification::hier_object_id::HierObjectId;
use openehr_base::identification::object_ref::ObjectRef;
use openehr_foundation::serde_support::{TypeName, TypeTag};
use serde::{Deserialize, Serialize};

use crate::common::generic::audit_details::AuditDetails;

/// Canonical `_type` discriminator string for this class in serialized
/// form. Per ADR-001 (Refinements), `serde` derives wait until P4.
pub const TYPE_NAME: &str = "CONTRIBUTION";

/// `CONTRIBUTION` — a change-set of one or more `VERSION` commits.
///
/// `CONTRIBUTION` has no `Inherit` row in its own spec table (implicitly
/// `Any`, per crate-wide convention for un-derived root classes — see
/// `docs/ROSETTA.md`'s `Cardinality` row for the same inference pattern).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contribution {
    /// Canonical `_type` discriminator (`"CONTRIBUTION"`), always serialized
    /// first; tolerated-absent and validated-if-present on input (ADR-002).
    #[serde(rename = "_type", default = "TypeTag::new")]
    pub type_tag: TypeTag<Self>,

    /// `uid`: unique identifier for this Contribution.
    pub uid: HierObjectId,

    /// `versions`: set of references to Versions causing changes to this
    /// EHR. Each contribution contains a list of versions, which may
    /// include paths pointing to any number of versionable items, i.e.
    /// items of types such as `COMPOSITION` and `FOLDER`.
    pub versions: Vec<ObjectRef>,

    /// `audit`: audit trail corresponding to the committal of this
    /// Contribution.
    pub audit: AuditDetails,
}

// This class declares no Functions or Invariants tables of its own beyond
// its three attributes.

impl TypeName for Contribution {
    const NAME: &'static str = TYPE_NAME;
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 common.change_control §CONTRIBUTION — docs/research/spec-cache/RM-1.1.0/uml_classes/contribution.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: master06-change_control_package.adoc §Class Descriptions / contribution.adoc §CONTRIBUTION Class
//   confidence: high
//   todos: 0
//   note: plain leaf struct, no generics/enums/recursion; three attributes transcribed verbatim.
// ─────────────────────────────────────────────
