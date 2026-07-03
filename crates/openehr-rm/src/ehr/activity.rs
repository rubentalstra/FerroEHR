//! `ACTIVITY` — a single activity within an `INSTRUCTION`.
//!
//! openEHR class: `ACTIVITY`, package `rm.ehr.entry`.
//! Inherits: `LOCATABLE`.
//!
//! Defines a single activity within an Instruction, such as a medication
//! administration.
use std::sync::LazyLock;

use crate::common::archetyped::locatable::LocatableData;
use crate::data_structures::item_structure::ItemStructure;
use crate::data_types::encapsulated::dv_parsable::DvParsable;
use openehr_foundation::serde_support::{TypeName, TypeTag};
use regex::Regex;
use serde::{Deserialize, Serialize};

/// Canonical `_type` discriminator string for this class in serialized
/// form. Single-sourced into the `TypeName` impl below (ADR-002).
pub const TYPE_NAME: &str = "ACTIVITY";

/// Validates the `//`-delimited form of `ACTIVITY.action_archetype_id`
/// (spec: "Perl-compliant regular expression pattern, enclosed in `//`
/// delimiters"), capturing the inner Perl pattern between the delimiters in
/// the `pattern` group. Used by [`Activity::is_action_archetype_id_well_formed`]
/// and [`Activity::matches_action_archetype`].
static ACTION_ARCHETYPE_ID_FORM: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^/(?P<pattern>.*)/$").expect("action_archetype_id form regex is a valid pattern")
});

/// `ACTIVITY` — a single activity within an [`super::instruction::Instruction`].
///
/// Per ADR-001 §3 (abstract class with attributes → embedded struct + marker
/// trait), `LOCATABLE`'s state is embedded as `pub locatable: LocatableData`
/// rather than simulated via a Rust supertrait. `#[serde(flatten)]` folds
/// those six attributes into `ACTIVITY`'s own JSON object.
///
/// TODO(port): P4 — the flatten below requires `LocatableData` to itself
/// derive `Serialize`/`Deserialize` (sibling P4 wave over `common/`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Activity {
    /// Canonical `_type` discriminator (`"ACTIVITY"`), always serialized
    /// first; tolerated-absent and validated-if-present on input (ADR-002).
    #[serde(rename = "_type", default = "TypeTag::new")]
    pub type_tag: TypeTag<Self>,

    /// Embedded `LOCATABLE` state.
    #[serde(flatten)]
    pub locatable: LocatableData,

    /// `timing`: timing of the activity, in the form of a parsable string.
    /// If used, the preferred syntax is ISO8601 'R' format, but other
    /// formats may be used including HL7 GTS.
    ///
    /// May be omitted if:
    /// * timing is represented structurally in the `description` attribute
    ///   (e.g. via archetyped elements), or
    /// * unavailable, e.g. imported legacy data; in such cases,
    ///   `INSTRUCTION.narrative` should carry text that indicates the
    ///   timing of its `activities`.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub timing: Option<DvParsable>,

    /// `action_archetype_id`: Perl-compliant regular expression pattern,
    /// enclosed in `//` delimiters, indicating the valid identifiers of
    /// archetypes for Actions corresponding to this Activity specification.
    ///
    /// Defaults to `/.*/`, meaning any archetype.
    ///
    /// Invariant `Action_archetype_id_valid`: `not
    /// action_archetype_id.is_empty` — see
    /// [`Activity::invariant_action_archetype_id_valid`].
    pub action_archetype_id: String,

    /// `description`: description of the activity, in the form of an
    /// archetyped structure.
    pub description: ItemStructure,
}

impl TypeName for Activity {
    const NAME: &'static str = TYPE_NAME;
}

impl Activity {
    /// Invariant `Action_archetype_id_valid`: `not
    /// action_archetype_id.is_empty` (ADR-003 §8) — the literal published
    /// invariant on this class.
    #[must_use]
    pub fn invariant_action_archetype_id_valid(&self) -> bool {
        !self.action_archetype_id.is_empty()
    }

    /// `true` if `action_archetype_id` is a well-formed `//`-delimited
    /// pattern whose inner Perl regex compiles.
    ///
    /// PORT NOTE: beyond the literal `Action_archetype_id_valid` invariant
    /// (which is only `not is_empty`); realises the field's own description
    /// ("Perl-compliant regular expression pattern, enclosed in `//`
    /// delimiters") using the [`ACTION_ARCHETYPE_ID_FORM`] static and the
    /// `regex` crate.
    #[must_use]
    pub fn is_action_archetype_id_well_formed(&self) -> bool {
        Self::compiled_pattern(&self.action_archetype_id).is_some()
    }

    /// `true` if `candidate_archetype_id` matches this Activity's
    /// `action_archetype_id` pattern (the inner Perl regex, applied with
    /// Perl `=~` unanchored semantics). `false` if the pattern is not
    /// well-formed.
    ///
    /// PORT NOTE: realises the field's stated purpose — "indicating the
    /// valid identifiers of archetypes for Actions corresponding to this
    /// Activity specification". The default `/.*/` matches any archetype.
    #[must_use]
    pub fn matches_action_archetype(&self, candidate_archetype_id: &str) -> bool {
        Self::compiled_pattern(&self.action_archetype_id)
            .is_some_and(|re| re.is_match(candidate_archetype_id))
    }

    /// Strips the `//` delimiters and compiles the inner Perl pattern, or
    /// `None` if the value is not `//`-delimited or the inner pattern does
    /// not compile.
    fn compiled_pattern(action_archetype_id: &str) -> Option<Regex> {
        let inner = ACTION_ARCHETYPE_ID_FORM
            .captures(action_archetype_id)?
            .name("pattern")?
            .as_str();
        Regex::new(inner).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The `//`-delimited-form parser accepts well-formed patterns and
    /// rejects undelimited or uncompilable ones (exercises the same logic
    /// [`Activity::is_action_archetype_id_well_formed`] exposes).
    #[test]
    fn action_archetype_id_well_formedness() {
        assert!(Activity::compiled_pattern("/.*/").is_some()); // spec default
        assert!(Activity::compiled_pattern(r"/openEHR-EHR-ACTION\.medication\..*/").is_some());
        // Missing delimiters.
        assert!(Activity::compiled_pattern("openEHR-EHR-ACTION.medication.v1").is_none());
        // Delimited but the inner pattern is an invalid regex.
        assert!(Activity::compiled_pattern("/[/").is_none());
    }

    /// The inner Perl pattern matches candidate archetype ids with Perl
    /// `=~` (unanchored) semantics.
    #[test]
    fn action_archetype_pattern_matches_candidates() {
        let default = Activity::compiled_pattern("/.*/").expect("default compiles");
        assert!(default.is_match("openEHR-EHR-ACTION.medication.v1"));

        let specific = Activity::compiled_pattern(r"/openEHR-EHR-ACTION\.medication\..*/")
            .expect("specific pattern compiles");
        assert!(specific.is_match("openEHR-EHR-ACTION.medication.v1"));
        assert!(!specific.is_match("openEHR-EHR-ACTION.procedure.v1"));
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 ehr.entry — docs/research/spec-cache/RM-1.1.0/uml_classes/activity.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: master08-entry_package.adoc §Class Descriptions / activity.adoc §ACTIVITY Class
//   confidence: high
//   todos: 1
//   note: LOCATABLE parent embedded per ADR-001 §3. P5/ADR-003 §8: Action_archetype_id_valid implemented (not is_empty); plus, per task, the field's `//`-delimited Perl-regex semantics realised via a LazyLock<Regex> static (ACTION_ARCHETYPE_ID_FORM) + is_action_archetype_id_well_formed()/matches_action_archetype() (regex crate), pinned by accept/reject unit tests. The one remaining TODO(port) is the P4 LocatableData-flatten scaffolding note. Follows the item_tag.rs LazyLock<Regex> precedent (same workspace expect_used=warn). P4/ADR-002: self-tagging TypeTag<Self> + TypeName.
// ─────────────────────────────────────────────
