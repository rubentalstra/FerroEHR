//! Parsed-artefact identity summary — the small public accessor the REST/service
//! layer reads after [`crate::assemble::parse_artefact`], so it never re-derives
//! the HRID / concept / kind / specialisation parent from source text by hand.
//!
//! The fields are exactly what the ITS-REST `DEFINITION` group needs:
//! `TemplateMetadata.archetype_id` + `.concept` (the OPT list rows;
//! `docs/specs/openehr/ITS-REST/specifications/schemas/definition/TemplateMetadata.yaml`),
//! the storage `kind` (`archetype` / `template` / `operational_template` /
//! `template_overlay`; `AOM2/master07.04`), the `specialize` parent reference,
//! and the specialisation depth (VACSD; `AOM2/master08-validation.adoc` Phase 1).

use openehr_am::am24::aom2::archetype::archetype::Archetype;

use crate::artefact::view;
use crate::hrid::hrid_to_string;
use crate::source::ArtefactKind;

/// The identity fields of a parsed ADL2 artefact.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtefactSummary {
    /// The full physical `ARCHETYPE_HRID`
    /// (`[ns::]publisher-package-class.concept.vMAJOR.MINOR.PATCH[-status.build]`;
    /// `AOM2/master07.05`), rendered by [`hrid_to_string`].
    pub archetype_id: String,
    /// The concept identifier — the `concept` segment of the HRID
    /// (`AOM2/master07.05` §Physical Archetype Identifier).
    pub concept_id: String,
    /// The storage kind keyword: `archetype`, `template`, `operational_template`,
    /// or `template_overlay` (`AOM2/master07.04`).
    pub kind: &'static str,
    /// The `specialize` parent reference (`AuthoredArchetype.parent_archetype_id`),
    /// when the artefact specialises a parent.
    pub parent_archetype_id: Option<String>,
    /// The specialisation depth = the specialisation level of the root node id
    /// (`id1` → 0, `id1.1` → 1, …; `AOM2/master07` §Specialisation Depth).
    pub specialisation_depth: usize,
}

/// The storage-kind keyword of an [`ArtefactKind`] (`AOM2/master07.04`).
#[must_use]
fn kind_keyword(kind: ArtefactKind) -> &'static str {
    match kind {
        ArtefactKind::Archetype => "archetype",
        ArtefactKind::Template => "template",
        ArtefactKind::TemplateOverlay => "template_overlay",
        ArtefactKind::OperationalTemplate => "operational_template",
    }
}

/// Summarise a parsed [`Archetype`] into its identity fields.
#[must_use]
pub fn summarize(archetype: &Archetype) -> ArtefactSummary {
    let v = view(archetype);
    ArtefactSummary {
        archetype_id: hrid_to_string(v.archetype_id),
        concept_id: v.archetype_id.concept_id.clone(),
        kind: kind_keyword(v.kind),
        parent_archetype_id: v.parent_archetype_id.map(str::to_owned),
        specialisation_depth: v.specialisation_level(),
    }
}
