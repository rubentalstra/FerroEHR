//! `ARCHETYPED` — archetype identification for a `LOCATABLE` root point.
//!
//! openEHR class: `ARCHETYPED` (concrete), package `common.archetyped`.
//!
//! Archetypes act as the configuration basis for the particular structures
//! of instances defined by the reference model. To enable archetypes to be
//! used to create valid data, key classes in the reference model act as
//! root points for archetyping; accordingly, these classes have the
//! `archetype_details` attribute set.
//!
//! An instance of the class `ARCHETYPED` contains the relevant archetype
//! identification information, allowing generating archetypes to be
//! matched up with data instances.
use openehr_base::identification::archetype_id::ArchetypeId;
use openehr_base::identification::template_id::TemplateId;

/// Canonical `_type` discriminator string for this class in serialized
/// form. Per ADR-001 refinements ("serde derives wait until P4"), a
/// `const` stands in for `#[serde(rename = ...)]` until serde lands as a
/// dependency of this crate.
pub const TYPE_NAME: &str = "ARCHETYPED";

/// `ARCHETYPED` declares no `Inherit` row in the spec table (its
/// superclass is implicitly `Any`, per BASE foundation_types convention —
/// see the same inference flagged for `Cardinality` in
/// `openehr-foundation::interval::cardinality`), so this is a plain
/// struct with no embedded parent state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Archetyped {
    /// `archetype_id`: `ARCHETYPE_ID`, cardinality `1..1`.
    ///
    /// Globally unique archetype identifier.
    pub archetype_id: ArchetypeId,

    /// `template_id`: `TEMPLATE_ID`, cardinality `0..1`.
    ///
    /// Globally unique template identifier, if a template was active at
    /// this point in the structure. Normally, a template would only be
    /// used at the top of a top-level structure, but the possibility
    /// exists for templates at lower levels.
    pub template_id: Option<TemplateId>,

    /// `rm_version`: `String`, cardinality `1..1`.
    ///
    /// Version of the openEHR reference model used to create this object.
    /// Expressed in terms of the release version string, e.g. `1.0`,
    /// `1.2.4`.
    ///
    /// Invariant `Rm_version_valid`: `not rm_version.is_empty`.
    ///
    /// TODO(port): invariant not yet enforced by a constructor/`Validate`
    /// impl; recorded here as a doc note pending the RM invariant
    /// framework (`.claude/rules/rm-transcription.md` "Invariants").
    pub rm_version: String,
}

impl Archetyped {
    /// Invariant `Rm_version_valid`: `not rm_version.is_empty`.
    ///
    /// TODO(port): not yet wired into a constructor or the RM `Validate`
    /// framework; this method lets a future `Validate` impl call the check
    /// directly once that framework lands.
    pub fn is_rm_version_valid(&self) -> bool {
        !self.rm_version.is_empty()
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 common.archetyped — docs/research/spec-cache/RM-1.1.0/uml_classes/archetyped.adoc (Release-1.1.0 @ 3cbd85b)
//   source_loc: common/master03-archetyped_package.adoc §Class Definitions / uml_classes/archetyped.adoc §ARCHETYPED Class
//   confidence: high
//   todos: 1
//   note: Rm_version_valid invariant recorded as is_rm_version_valid() but not yet Validate-enforced. No ancestor inference needed beyond the implicit Any convention already used elsewhere in the port.
// ─────────────────────────────────────────────
