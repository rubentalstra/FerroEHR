//! `VERSIONED_EHR_STATUS` — version container for `EHR_STATUS` instances.
//!
//! openEHR class: `VERSIONED_EHR_STATUS`, package `rm.ehr`.
//! Inherits: `VERSIONED_OBJECT<T>` (bound to `T = EHR_STATUS`).
//!
//! Static type formed by binding the generic parameter of
//! `VERSIONED_OBJECT<T>` to `EHR_STATUS`. As with `VERSIONED_EHR_ACCESS`
//! (see that file's doc comment for the fuller rationale quoted from the
//! chapter narrative), the published class table declares no attribute,
//! function, or invariant of its own beyond what `VERSIONED_OBJECT<T>`
//! already provides.
//!
//! Ground truth: `docs/research/spec-cache/RM-1.1.0/ehr/uml_classes/versioned_ehr_status.adoc`
//! (RM Release-1.1.0 @ 3cbd85b).

// TODO(port): forward-reference — `VERSIONED_OBJECT<T>` lives in
// rm.common.change_control (PORT_MASTER_PLAN.md §7.1), not yet transcribed.
use crate::common::change_control::versioned_object::VersionedObject;

use super::ehr_status::EhrStatus;

/// Canonical `_type` discriminator string for this class in serialized form.
pub const TYPE_NAME: &str = "VERSIONED_EHR_STATUS";

/// `VERSIONED_EHR_STATUS` — `VERSIONED_OBJECT<EHR_STATUS>`.
///
/// See `versioned_ehr_access::VersionedEhrAccess` for the full rationale
/// behind the newtype-wrapper (rather than bare type-alias) shape used for
/// this class of binding.
#[derive(Debug, Clone, PartialEq)]
pub struct VersionedEhrStatus(pub VersionedObject<EhrStatus>);

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 ehr — docs/research/spec-cache/RM-1.1.0/ehr/uml_classes/versioned_ehr_status.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: master04-ehr_package.adoc §Class Descriptions / uml_classes/versioned_ehr_status.adoc §VERSIONED_EHR_STATUS Class
//   confidence: high
//   todos: 1
//   note: pure VERSIONED_OBJECT<T> binding with no added members; same newtype-wrapper shape as VersionedEhrAccess.
// ─────────────────────────────────────────────
