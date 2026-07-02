//! `VERSIONED_COMPOSITION` — version-controlled composition abstraction.
//!
//! openEHR class: `VERSIONED_COMPOSITION`, package `rm.ehr`.
//! Inherits: `VERSIONED_OBJECT<T>` (bound to `T = COMPOSITION`).
//!
//! Version-controlled composition abstraction, defined by inheriting
//! `VERSIONED_OBJECT<COMPOSITION>`. Unlike the other `VERSIONED_*` bindings
//! declared by this chapter (`VERSIONED_EHR_ACCESS`, `VERSIONED_EHR_STATUS`),
//! this class is not a bare binding — the published table adds one function
//! (`is_persistent()`) and two invariants of its own.
//!
//! Ground truth: `docs/research/spec-cache/RM-1.1.0/ehr/uml_classes/versioned_composition.adoc`
//! (RM Release-1.1.0 @ 3cbd85b).

// TODO(port): forward-reference — `VERSIONED_OBJECT<T>` lives in
// rm.common.change_control (PORT_MASTER_PLAN.md §7.1), not yet transcribed.
use crate::common::change_control::versioned_object::VersionedObject;

// TODO(port): forward-reference — `COMPOSITION` lives in rm.ehr.composition
// (PORT_MASTER_PLAN.md §7.1: "EHR (20): EHR, EHR_STATUS, EHR_ACCESS,
// COMPOSITION, ..."). A sibling transcription pass owns the
// composition/content/entry classes in this same `crates/openehr-rm/src/ehr/`
// directory; this file forward-references `Composition` rather than
// defining it.
use super::composition::Composition;
use serde::{Deserialize, Serialize};

/// Canonical `_type` discriminator string for this class per the spec's
/// class naming.
///
/// PORT NOTE (ADR-002, resolved): `VERSIONED_X` binding classes never emit
/// their own `_type`. The pinned ITS-JSON schema (commit
/// `5acae056248e917a4b4c56f7e712f4fcfeb616a6`,
/// `openehr_rm_1.1.0_all.json`) defines only `VERSIONED_OBJECT` — which
/// self-tags via its own `TypeTag` (sibling `common.change_control` wave) —
/// and has no `VERSIONED_COMPOSITION`/`VERSIONED_EHR_*` entries at all. So
/// this newtype keeps `#[serde(transparent)]`, gets no `TypeName`/`TypeTag`
/// of its own, and this const exists only as the spec class name for
/// non-serde callers (e.g. `OBJECT_REF.type` comparisons).
pub const TYPE_NAME: &str = "VERSIONED_COMPOSITION";

/// `VERSIONED_COMPOSITION` — `VERSIONED_OBJECT<COMPOSITION>` plus its own
/// `is_persistent()` function and two invariants.
///
/// See `versioned_ehr_access::VersionedEhrAccess` for the rationale behind
/// the newtype-wrapper (rather than bare type-alias) shape used for
/// `VERSIONED_OBJECT<T>` bindings in general; this class additionally
/// carries real behaviour of its own (below), reinforcing that the
/// newtype-with-inherent-impl shape (not a type alias, which could not
/// carry inherent methods distinct from `VersionedObject<Composition>`'s
/// own) is the right one here.
///
/// PORT NOTE: `#[serde(transparent)]` makes this newtype serialize/
/// deserialize identically to its single field,
/// `VersionedObject<Composition>` — whose own `TypeTag` emits
/// `_type: "VERSIONED_OBJECT"`. Per ADR-002 this binding never emits a
/// `_type: "VERSIONED_COMPOSITION"` of its own: the pinned ITS-JSON schema
/// defines no `VERSIONED_X` entries, only `VERSIONED_OBJECT` (see the
/// `TYPE_NAME` doc comment). Resolved — not a deferred P17 question.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct VersionedComposition(pub VersionedObject<Composition>);

impl VersionedComposition {
    /// Function `is_persistent` (): `Boolean`.
    ///
    /// Indicates whether this composition set is persistent; derived from
    /// the first version.
    ///
    /// Cardinality: `1..1`.
    ///
    /// TODO(port): depends on `VersionedObject::all_versions()` /
    /// `latest_version()`-style accessors (not yet transcribed on
    /// `VersionedObject<T>`) and on `Composition`'s own `is_persistent`
    /// derivation (not yet transcribed on `Composition`, owned by the
    /// sibling composition-package transcription pass).
    pub fn is_persistent(&self) -> bool {
        todo!(
            "port: derive from all_versions().first().data.is_persistent (or equivalent); \
             awaits VersionedObject<T> version-accessor methods and Composition::is_persistent"
        )
    }

    /// Invariant `Archetype_node_id_valid`:
    /// `for_all v in all_versions | v.archetype_node_id.is_equal(all_versions.first.archetype_node_id)`.
    ///
    /// All versions of the same `VERSIONED_COMPOSITION` share one
    /// archetype node id — the identity of the composition's archetype does
    /// not change across its version history.
    ///
    /// TODO(port): not yet enforced; awaits `VersionedObject::all_versions()`
    /// and the RM invariant framework
    /// (`.claude/rules/rm-transcription.md` "Invariants").
    pub fn invariant_archetype_node_id_valid(&self) -> bool {
        todo!(
            "port: for_all v in all_versions | v.archetype_node_id.is_equal(all_versions.first.archetype_node_id)"
        )
    }

    /// Invariant `Persistent_validity`:
    /// `for_all v in all_versions | v.is_persistent = all_versions.first.data.is_persistent`.
    ///
    /// TODO(port): not yet enforced; same dependencies as
    /// `invariant_archetype_node_id_valid`.
    pub fn invariant_persistent_validity(&self) -> bool {
        todo!(
            "port: for_all v in all_versions | v.is_persistent = all_versions.first.data.is_persistent"
        )
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 ehr — docs/research/spec-cache/RM-1.1.0/ehr/uml_classes/versioned_composition.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: master04-ehr_package.adoc §Class Descriptions / uml_classes/versioned_composition.adoc §VERSIONED_COMPOSITION Class
//   confidence: high
//   todos: 3
//   note: forward-references Composition (sibling agent's file, not created here); is_persistent() and both invariants stubbed pending VersionedObject<T> version-accessor methods. P4/ADR-002 resolved: binding keeps #[serde(transparent)] and never emits its own _type — the pinned ITS-JSON schema defines only VERSIONED_OBJECT (self-tagged in the sibling change_control wave), no VERSIONED_X entries; TYPE_NAME retained for non-serde callers only.
// ─────────────────────────────────────────────
