//! `OPENEHR_DEFINITIONS` — inheritance class providing access to constants
//! defined in other packages.
//!
//! openEHR class: `OPENEHR_DEFINITIONS`, package `base.base_types.definitions`.
//! Inherits: `BASIC_DEFINITIONS`.
//!
//! Inheritance class to provide access to constants defined in other
//! packages, plus one attribute of its own: the predefined terminology
//! identifier used to indicate that a terminology is local to the knowledge
//! resource in which it occurs (e.g. an archetype).
use super::basic_definitions::BasicDefinitions;

/// `OPENEHR_DEFINITIONS` is, like its `BASIC_DEFINITIONS` parent, a
/// constant/default-value provider rather than a class with meaningful
/// instances — every RM/AM class that "inherits" it in the spec does so
/// purely to gain in-scope access to its constants, not to hold state
/// through it. Transcribed the same way: a zero-field struct with
/// associated consts.
///
/// PORT NOTE: the spec's single attribute, `Local_terminology_id`, is
/// declared with a default value (`{default = "local"}`), not a fixed
/// constant — in the openEHR/Eiffel source this reads as an attribute a
/// subclass or instance could in principle override. No RM class
/// transcribed so far does override it, and the value the spec assigns is
/// itself the well-known, otherwise-undocumented sentinel string `"local"`
/// used throughout the RM/AM to mean "this terminology is local to the
/// archetype/template, not looked up externally" — so it is transcribed
/// here as an associated const (`LOCAL_TERMINOLOGY_ID`) alongside the
/// inherited `BasicDefinitions` consts rather than as a struct field with a
/// `Default` impl. Revisit if a later phase needs a genuinely overridable
/// per-instance value.
///
/// Rust has no struct-level inheritance, so the parent's constants are not
/// automatically in scope through `OpenehrDefinitions::` the way they would
/// be via `OPENEHR_DEFINITIONS::CR` in the source language; callers that
/// need both reach for `BasicDefinitions::CR` directly (see
/// `basic_definitions.rs`) alongside `OpenehrDefinitions::LOCAL_TERMINOLOGY_ID`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OpenehrDefinitions;

impl OpenehrDefinitions {
    /// `Local_terminology_id`: `String { default = "local" }`.
    ///
    /// Predefined terminology identifier to indicate it is local to the
    /// knowledge resource in which it occurs, e.g. an archetype.
    pub const LOCAL_TERMINOLOGY_ID: &'static str = "local";
}

// PORT NOTE: retained only to document the spec's `Inherit` relationship;
// `BasicDefinitions` carries no per-instance state to actually compose into
// this type (see the struct-level PORT NOTE above for why the constants
// themselves are not re-exported through this type).
#[allow(dead_code)]
type _InheritsBasicDefinitions = BasicDefinitions;

// ─────────────────────────────────────────────
// PORT STATUS
//   source: BASE 1.2.0 base_types.definitions — docs/research/spec-cache/BASE-1.2.0/uml_classes/openehr_definitions.adoc (Release-1.2.0 @ 9064413)
//   source_loc: master03-definitions_package.adoc §Class Definitions / openehr_definitions.adoc §OPENEHR_DEFINITIONS Class
//   confidence: medium
//   todos: 0
//   note: Local_terminology_id transcribed as an associated const (spec's default value is the RM/AM's well-known "local" sentinel, never observed overridden); the BASIC_DEFINITIONS inheritance is documentation-only since Rust has no struct inheritance and the parent has no state to embed.
// ─────────────────────────────────────────────
