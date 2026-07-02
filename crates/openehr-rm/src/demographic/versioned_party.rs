//! `VERSIONED_PARTY` — the versioned container for `PARTY` demographic
//! objects.
//!
//! openEHR class: `VERSIONED_PARTY` (concrete), package `rm.demographic`.
//!
//! Static type formed by binding the generic parameter of
//! `VERSIONED_OBJECT<T>` to `PARTY`. Bound in the demographic package's own
//! `Class Definitions` list (`master02-demographic_package.adoc`,
//! immediately after `PARTY`).
use super::party::Party;

/// `VERSIONED_PARTY` — a type alias binding `VERSIONED_OBJECT<T>`'s generic
/// parameter to `Party`, per ADR-001 §5 (constrained generic → generic with
/// a trait bound; here the binding is a closed monomorphisation rather than
/// a further-bounded open parameter, so a type alias is the direct
/// transcription of "static type formed by binding generic parameter... to
/// PARTY").
///
/// TODO(port): forward-reference to `crate::common::change_control::versioned_object::VersionedObject<T>`
/// (sibling agent owns `common`); this alias cannot resolve until that
/// generic type lands, and both the type name (`VersionedObject`) and its
/// generic parameter's trait bound are conventions this file does not
/// control.
pub type VersionedParty = crate::common::change_control::versioned_object::VersionedObject<Party>;

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 demographic §Class Definitions VERSIONED_PARTY — docs/research/spec-cache/RM-1.1.0/uml_classes/versioned_party.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: master02-demographic_package.adoc §Class Definitions / uml_classes/versioned_party.adoc §VERSIONED_PARTY Class
//   confidence: low
//   todos: 1
//   note: type alias forward-references crate::common::change_control::versioned_object::VersionedObject<T>, not yet landed by the sibling agent owning common/ — exact path/bound unverifiable until then.
// ─────────────────────────────────────────────
