//! `UID` — abstract parent of unique identifier classes.
//!
//! openEHR class: `UID` (abstract), package `base.base_types.identification`.
//!
//! Abstract parent of classes representing unique identifiers which
//! identify information entities in a durable way. UIDs only ever identify
//! one IE in time or space and are never re-used.
use super::internet_id::InternetId;
use super::iso_oid::IsoOid;
use super::uuid::Uuid;

/// Shared attribute state of `UID` and its descendants.
///
/// Per ADR-001 §3 (abstract class with attributes → embedded struct + marker
/// trait), every concrete `UID` subtype (`IsoOid`, `Uuid`, `InternetId`)
/// embeds this struct rather than inheriting from it, since Rust has no
/// class inheritance. None of the three concrete subtypes adds any
/// attribute or function of its own beyond what `UID` declares, so each
/// concrete file wraps `UidData` directly.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UidData {
    /// `value`: the value of the id.
    ///
    /// Invariant `Value_valid`: `not value.empty`.
    ///
    /// TODO(port): invariant not yet enforced by a constructor/`Validate`
    /// impl; recorded here as a doc note pending the RM invariant framework
    /// (`.claude/rules/rm-transcription.md` "Invariants").
    pub value: String,
}

/// `UID` is abstract in the spec and is used polymorphically wherever an
/// attribute is declared of type `UID` (e.g. `UID_BASED_ID.root()`,
/// `OBJECT_VERSION_ID.object_id()`/`creating_system_id()`). Per ADR-001 §4
/// (closed subtype set → enum), the three concrete subtypes `ISO_OID`,
/// `UUID`, and `INTERNET_ID` are collected into this closed `enum` so a
/// field or return type can be declared `Uid` exactly where the spec
/// declares it `UID`.
///
/// The spec notes (BASE 1.2.0 identification package, "Primitive
/// Identifiers") that the three subtypes have "mutually exclusive string
/// patterns" and so can always be distinguished by inspecting the string
/// form alone — justifying the closed, exhaustively-matchable enum shape
/// used here rather than a trait object.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Uid {
    /// `ISO_OID`.
    IsoOid(IsoOid),
    /// `UUID`.
    ///
    /// PORT NOTE: named `Uuid` (PascalCase of the spec's `UUID`), which is a
    /// distinct type from the `uuid` crate's `Uuid` — see the doc comment on
    /// `uuid::Uuid` in `uuid.rs` for the disambiguation. No external `uuid`
    /// crate dependency is introduced by this transcription.
    Uuid(Uuid),
    /// `INTERNET_ID`.
    InternetId(InternetId),
}

/// Marker/accessor trait shared by every `UID` descendant, exposing the
/// abstract class's sole attribute uniformly whether the caller holds a
/// concrete type or a `Uid` enum value.
pub trait UidApi {
    /// `value`: the value of the id.
    fn value(&self) -> &str;
}

impl UidApi for Uid {
    fn value(&self) -> &str {
        match self {
            Uid::IsoOid(v) => v.value(),
            Uid::Uuid(v) => v.value(),
            Uid::InternetId(v) => v.value(),
        }
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: BASE 1.2.0 base_types.identification §UID — docs/research/spec-cache/BASE-1.2.0/uml_classes/uid.adoc (Release-1.2.0 @ 9064413)
//   source_loc: master05-identification_package.adoc §Class Descriptions / uid.adoc §UID Class
//   confidence: high
//   todos: 1
//   note: Value_valid invariant (not value.empty) recorded but not yet enforced; awaits the RM Validate-trait framework.
// ─────────────────────────────────────────────
