//! `VERSIONED_EHR_ACCESS` — version container for `EHR_ACCESS` instances.
//!
//! openEHR class: `VERSIONED_EHR_ACCESS`, package `rm.ehr`.
//! Inherits: `VERSIONED_OBJECT<T>` (bound to `T = EHR_ACCESS`).
//!
//! Static type formed by binding the generic parameter of
//! `VERSIONED_OBJECT<T>` to `EHR_ACCESS`. The published class table
//! declares no attribute, function, or invariant of its own beyond what
//! `VERSIONED_OBJECT<T>` already provides — its entire purpose, per the
//! chapter narrative (`master04-ehr_package.adoc` §Overview), is to give a
//! named, non-generic class in languages lacking genericity: "Each versioned
//! object of type `X` is defined by a class `VERSIONED_X`, which is a
//! binding of the type `X` to the generic type parameter `T` in the generic
//! type `VERSIONED_COMPOSITION`" (the narrative names `VERSIONED_COMPOSITION`
//! as its own worked example of the pattern; `VERSIONED_EHR_ACCESS` is the
//! same pattern bound to `EHR_ACCESS`).
//!
//! Ground truth: `docs/research/spec-cache/RM-1.1.0/ehr/uml_classes/versioned_ehr_access.adoc`
//! (RM Release-1.1.0 @ 3cbd85b).

// TODO(port): forward-reference — `VERSIONED_OBJECT<T>` lives in
// rm.common.change_control (PORT_MASTER_PLAN.md §7.1), not yet transcribed.
use crate::common::change_control::versioned_object::VersionedObject;
use serde::{Deserialize, Serialize};

use super::ehr_access::EhrAccess;

/// Canonical `_type` discriminator string for this class per the spec's
/// class naming.
///
/// PORT NOTE (ADR-002, resolved): `VERSIONED_X` binding classes never emit
/// their own `_type` — the pinned ITS-JSON schema defines only
/// `VERSIONED_OBJECT` (self-tagged in the sibling `common.change_control`
/// wave), no `VERSIONED_X` entries. See the fuller note on
/// `versioned_composition::TYPE_NAME`. This const exists only as the spec
/// class name for non-serde callers (e.g. `OBJECT_REF.type` comparisons).
pub const TYPE_NAME: &str = "VERSIONED_EHR_ACCESS";

/// `VERSIONED_EHR_ACCESS` — `VERSIONED_OBJECT<EHR_ACCESS>`.
///
/// Per ADR-001 §5 (constrained generic → generic with trait bound),
/// `VersionedObject<T>` is expected to carry a bound such as
/// `T: LocatableApi` or similar once `common.change_control` is
/// transcribed; this binding class simply closes `T` to the concrete
/// `EhrAccess` type, matching the spec's own binding relationship
/// (`VERSIONED_EHR_ACCESS` inherits `VERSIONED_OBJECT<T>` with `T` bound to
/// `EHR_ACCESS`, not a fresh subclass with its own state).
///
/// Transcribed as a newtype-style wrapper rather than a bare
/// `type VersionedEhrAccess = VersionedObject<EhrAccess>;` alias so the
/// class remains a nameable, addressable type for downstream RM code
/// exactly as the spec intends ("a binding of the type X to the generic
/// type parameter T ... facilitate[s] implementation in languages lacking
/// genericity" — Rust has genericity, but the spec's own reason for minting
/// the class is preserved here for name-stability rather than erased into a
/// type alias).
///
/// PORT NOTE: `#[serde(transparent)]`, no `TypeName`/`TypeTag` of its own —
/// per ADR-002 the binding never emits `_type: "VERSIONED_EHR_ACCESS"`; the
/// wire tag is the inner `VersionedObject`'s `_type: "VERSIONED_OBJECT"`.
/// See the identical note on
/// `versioned_composition::VersionedComposition`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct VersionedEhrAccess(pub VersionedObject<EhrAccess>);

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 ehr — docs/research/spec-cache/RM-1.1.0/ehr/uml_classes/versioned_ehr_access.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: master04-ehr_package.adoc §Class Descriptions / uml_classes/versioned_ehr_access.adoc §VERSIONED_EHR_ACCESS Class
//   confidence: high
//   todos: 1
//   note: pure VERSIONED_OBJECT<T> binding with no added members; newtype wrapper (not a bare type alias) for name stability. P4/ADR-002 resolved: keeps #[serde(transparent)] and never emits its own _type — the pinned ITS-JSON schema defines only VERSIONED_OBJECT, no VERSIONED_X entries (matching versioned_composition.rs).
// ─────────────────────────────────────────────
