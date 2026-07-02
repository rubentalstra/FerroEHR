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
use serde::{Deserialize, Serialize};

use super::ehr_status::EhrStatus;

/// Canonical `_type` discriminator string for this class per the spec's
/// class naming.
///
/// PORT NOTE (ADR-002, resolved): `VERSIONED_X` binding classes never emit
/// their own `_type` — the pinned ITS-JSON schema defines only
/// `VERSIONED_OBJECT` (self-tagged in the sibling `common.change_control`
/// wave), no `VERSIONED_X` entries. See the fuller note on
/// `versioned_composition::TYPE_NAME`. This const exists only as the spec
/// class name for non-serde callers (e.g. `OBJECT_REF.type` comparisons).
pub const TYPE_NAME: &str = "VERSIONED_EHR_STATUS";

/// `VERSIONED_EHR_STATUS` — `VERSIONED_OBJECT<EHR_STATUS>`.
///
/// See `versioned_ehr_access::VersionedEhrAccess` for the full rationale
/// behind the newtype-wrapper (rather than bare type-alias) shape used for
/// this class of binding.
///
/// PORT NOTE: `#[serde(transparent)]`, no `TypeName`/`TypeTag` of its own —
/// per ADR-002 the binding never emits `_type: "VERSIONED_EHR_STATUS"`; the
/// wire tag is the inner `VersionedObject`'s `_type: "VERSIONED_OBJECT"`.
/// See the identical note on
/// `versioned_composition::VersionedComposition`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct VersionedEhrStatus(pub VersionedObject<EhrStatus>);

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 ehr — docs/research/spec-cache/RM-1.1.0/ehr/uml_classes/versioned_ehr_status.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: master04-ehr_package.adoc §Class Descriptions / uml_classes/versioned_ehr_status.adoc §VERSIONED_EHR_STATUS Class
//   confidence: high
//   todos: 1
//   note: pure VERSIONED_OBJECT<T> binding with no added members; same newtype-wrapper shape as VersionedEhrAccess. P4/ADR-002 resolved: keeps #[serde(transparent)] and never emits its own _type — the pinned ITS-JSON schema defines only VERSIONED_OBJECT, no VERSIONED_X entries (matching versioned_composition.rs).
// ─────────────────────────────────────────────
