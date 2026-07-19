//! The AOM2 validation catalogue.
//!
//! A [`Validator`] walks an assembled `openehr_am::am24::aom2` [`Archetype`]
//! and produces typed [`ValidationIssue`]s, each carrying a [`ValidationCode`]
//! (one variant per AOM2 validity code), a [`Severity`], a message, and — where
//! derivable — the archetype path the issue is anchored at.
//!
//! Phase 1 (basic integrity, standalone) is implemented here
//! ([`validate_phase1`] / [`validate_source_phase1`]); phases 2 (RM +
//! specialised-vs-flat-parent) and 3 (flat form) land in later phases. The
//! phase-1 catalogue and its orchestration are defined in
//! `docs/specs/openehr/AM/docs/AOM2/master08-validation.adoc` §Phase 1 - Basic
//! Integrity, with the full rule texts in `master03-archetype_package.adoc`,
//! `master04.5-constraint_model-class_definitions.adoc`,
//! `master06-rm_overlay.adoc`, and `master07-terminology_package.adoc` (cited
//! per check in [`phase1`]).
//!
//! Parent seam: parent-dependent checks (VACSD's depth comparison, VASID,
//! VALC, VTPL) take an optional [`ArchetypeRepository`]; when a parent is not
//! supplied they degrade to the standalone half they can compute (or are
//! skipped), so phase 2 (A6) slots in without re-architecture.

mod phase1;

use std::collections::HashMap;

use openehr_am::am24::aom2::archetype::archetype::Archetype;
use openehr_am::am24::aom2::archetype::archetype_hrid::ArchetypeHrid;
use openehr_am::am24::aom2::archetype::authored_archetype::AuthoredArchetype;
use openehr_am::am24::aom2::constraint_model::c_complex_object::CComplexObject;
use openehr_am::am24::aom2::rm_overlay::rm_overlay::RmOverlay;
use openehr_am::am24::aom2::terminology::archetype_terminology::ArchetypeTerminology;
use openehr_am::am24::resource::resource_description::ResourceDescription;
use openehr_base::prelude::{ResourceAnnotations, TerminologyCode};

use crate::error::SyntaxError;
use crate::source::{ArtefactKind, parse_source};

/// The severity of a [`ValidationIssue`].
///
/// The `W`-prefixed codes (WACMCL, WOUC) are warnings; every other code is an
/// error. master08 assigns no explicit severity column, so this follows the
/// `V`/`W` naming convention (the `W` prefix = advisory "should"; see
/// `master04.5` WACMCL "should be" vs VACMCU "must").
///
/// NOTE: no openEHR spec states the `W`→Warning convention normatively; it is
/// inferred from the code naming (master08-validation is silent on severity).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Severity {
    /// A validity error — the archetype is invalid.
    Error,
    /// A validity warning — advisory, does not invalidate the archetype.
    Warning,
}

/// An AOM2 validation code (one typed variant per validity rule).
///
/// Each variant's doc comment names the spec file + section that defines it.
/// The catalogue is the phase-1 set of `docs/specs/openehr/AM/docs/AOM2/`
/// plus the two corpus-adjudicated additions (VRDLA, WOUC — archie parity, no
/// full vendored text, NOTE-flagged). Deferred variants (their check needs the
/// RM model, the flat parent, or an external terminology service) are present
/// as the vocabulary but not raised in phase 1 — see [`phase1`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ValidationCode {
    /// VARDT — archetype definition typename validity (master03 §Validity Rules).
    Vardt,
    /// VARCN — archetype concept validity (master03 §Validity Rules).
    Varcn,
    /// STCNT — missing mandatory part, e.g. terminology (master08 §Phase 1; no
    /// full vendored text — NOTE-flagged).
    Stcnt,
    /// VACSD — archetype concept specialisation depth (master03 §Validity Rules).
    Vacsd,
    /// VOLT — original language available in terminology (master08 §Phase 1; no
    /// full vendored text — NOTE-flagged).
    Volt,
    /// VARAV — ADL version validity (master03 §Validity Rules).
    Varav,
    /// VARRV — RM release validity (master03 §Validity Rules).
    Varrv,
    /// VOTM — terminology translations validity (master03 §Validity Rules).
    Votm,
    /// VDIFV — differential path only in specialised archetype (master04.5
    /// §`C_ATTRIBUTE`).
    Vdifv,
    /// VDIFP — differential path exists in flat parent (master04.5 §`C_ATTRIBUTE`;
    /// deferred — needs the flat parent + RM).
    Vdifp,
    /// VATCV — terminology code format validity (master08 §Phase 1; no full
    /// vendored text — NOTE-flagged).
    Vatcv,
    /// VTSD — specialisation level of codes (master07 §Validity Rules).
    Vtsd,
    /// VTLC — terminology language consistency (master07 §Validity Rules).
    Vtlc,
    /// VTTBK — term binding key valid (master07 §Validity Rules).
    Vttbk,
    /// VTCBK — constraint binding key valid (master07 §Validity Rules).
    Vtcbk,
    /// VETDF — external term validity (master03 §Validity Rules; deferred —
    /// needs an external terminology service).
    Vetdf,
    /// VTVSID — value-set id defined (master07 §Validity Rules).
    Vtvsid,
    /// VTVSMD — value-set members defined (master07 §Validity Rules).
    Vtvsmd,
    /// VTVSUQ — value-set members unique (master07 §Validity Rules).
    Vtvsuq,
    /// VDSEV — slot 'exclude' constraint validity (master04.5 §`ARCHETYPE_SLOT`).
    Vdsev,
    /// VDSIV — slot 'include' constraint validity (master04.5 §`ARCHETYPE_SLOT`).
    Vdsiv,
    /// VARXRA — `C_ARCHETYPE_ROOT` validity set (master08 §Phase 1; umbrella for
    /// VARXNC/VARXAV/VARXR — no full vendored text, NOTE-flagged).
    Varxra,
    /// VARXNC — `C_ARCHETYPE_ROOT` node-id conformance (master08 §Phase 1).
    Varxnc,
    /// VARXAV — `C_ARCHETYPE_ROOT` archetype-ref validity (master08 §Phase 1).
    Varxav,
    /// VARXR — external reference resolution (master08 §Phase 2; deferred —
    /// needs the supplier repository).
    Varxr,
    /// VARXTV — `C_ARCHETYPE_ROOT` type validity (master08 §Phase 1).
    Varxtv,
    /// VATID — all definition codes defined in terminology (master08 §Phase 1;
    /// no full vendored text — NOTE-flagged).
    Vatid,
    /// VATCD — archetype code specialisation level validity (master03 §Validity
    /// Rules).
    Vatcd,
    /// VATDF — value code (at-code) validity (master03 §Validity Rules; the
    /// flat-parent half is NOTE-deferred for specialised archetypes).
    Vatdf,
    /// VACDF — constraint code (ac-code) validity (master03 §Validity Rules).
    Vacdf,
    /// VATDA — value-set assumed value code validity (master03 §Validity Rules).
    Vatda,
    /// VRANP — annotation path valid (master03 §Validity Rules; the RM-path half
    /// is NOTE-deferred to the RM checks).
    Vranp,
    /// VOKU — object key unique in keyed lists (master03 §Validity Rules).
    Voku,
    /// VARID — archetype identifier validity (master03 §Validity Rules).
    Varid,
    /// VDEOL — original language specified (master03 §Validity Rules).
    Vdeol,
    /// VARD — description specified (master03 §Validity Rules).
    Vard,
    /// VASID — specialisation parent identifier validity (master03 §Validity
    /// Rules; fires only when the parent is supplied).
    Vasid,
    /// VALC — archetype language conformance (master03 §Validity Rules; fires
    /// only when the parent is supplied).
    Valc,
    /// VTPL — template/filler language consistency (master03 §Validity Rules;
    /// deferred — needs the resolved fillers).
    Vtpl,
    /// VRRLP — rule path valid (master03 §Validity Rules; the RM-extension half
    /// is NOTE-deferred to the RM checks).
    Vrrlp,
    /// VCOCD — object constraint definition validity (master04.5 §`C_OBJECT`).
    Vcocd,
    /// VCOID — object node identifier present (master04.5 §`C_OBJECT`).
    Vcoid,
    /// VCOSU — object node identifier unique (master04.5 §`C_OBJECT`).
    Vcosu,
    /// VCATU — sibling attribute uniqueness (master04.5 §`C_COMPLEX_OBJECT`).
    Vcatu,
    /// VDFAI — archetype id validity in slot definition (master04.5
    /// §`ARCHETYPE_SLOT`).
    Vdfai,
    /// VOBAV — object node assumed value validity (master04.5
    /// §`C_PRIMITIVE_OBJECT`).
    Vobav,
    /// VRMVP — RM-visibility path validity (master06 §Validity).
    Vrmvp,
    /// VRMVAV — RM-visibility alias validity (master06 §Validity).
    Vrmvav,
    /// VACSO — single-valued attribute child occurrences validity (master04.5
    /// §`C_ATTRIBUTE`).
    Vacso,
    /// VACMCU — cardinality/occurrences upper bound validity (master04.5
    /// §`C_ATTRIBUTE`).
    Vacmcu,
    /// VSONIF — object node identification validity in flat siblings (master04.5
    /// §`ARCHETYPE_SLOT`; deferred — needs the flat parent; refs undefined VACMI).
    Vsonif,
    /// VRDLA — resource-description language-code consistency (archie parity; no
    /// openEHR spec governs this — our own design/extension, NOTE-flagged).
    Vrdla,
    /// WACMCL — cardinality/occurrences lower bound warning (master04.5
    /// §`C_ATTRIBUTE`; WARNING).
    Wacmcl,
    /// WOUC — defined terminology code unused in the definition (archie parity;
    /// no openEHR spec governs this — our own design/extension; WARNING).
    Wouc,
}

impl ValidationCode {
    /// The bare mnemonic (e.g. `"VARDT"`), as used in the spec catalogue and
    /// the conformance-corpus `regression` tags.
    #[must_use]
    #[allow(clippy::too_many_lines)] // one arm per catalogue code
    pub fn mnemonic(self) -> &'static str {
        match self {
            Self::Vardt => "VARDT",
            Self::Varcn => "VARCN",
            Self::Stcnt => "STCNT",
            Self::Vacsd => "VACSD",
            Self::Volt => "VOLT",
            Self::Varav => "VARAV",
            Self::Varrv => "VARRV",
            Self::Votm => "VOTM",
            Self::Vdifv => "VDIFV",
            Self::Vdifp => "VDIFP",
            Self::Vatcv => "VATCV",
            Self::Vtsd => "VTSD",
            Self::Vtlc => "VTLC",
            Self::Vttbk => "VTTBK",
            Self::Vtcbk => "VTCBK",
            Self::Vetdf => "VETDF",
            Self::Vtvsid => "VTVSID",
            Self::Vtvsmd => "VTVSMD",
            Self::Vtvsuq => "VTVSUQ",
            Self::Vdsev => "VDSEV",
            Self::Vdsiv => "VDSIV",
            Self::Varxra => "VARXRA",
            Self::Varxnc => "VARXNC",
            Self::Varxav => "VARXAV",
            Self::Varxr => "VARXR",
            Self::Varxtv => "VARXTV",
            Self::Vatid => "VATID",
            Self::Vatcd => "VATCD",
            Self::Vatdf => "VATDF",
            Self::Vacdf => "VACDF",
            Self::Vatda => "VATDA",
            Self::Vranp => "VRANP",
            Self::Voku => "VOKU",
            Self::Varid => "VARID",
            Self::Vdeol => "VDEOL",
            Self::Vard => "VARD",
            Self::Vasid => "VASID",
            Self::Valc => "VALC",
            Self::Vtpl => "VTPL",
            Self::Vrrlp => "VRRLP",
            Self::Vcocd => "VCOCD",
            Self::Vcoid => "VCOID",
            Self::Vcosu => "VCOSU",
            Self::Vcatu => "VCATU",
            Self::Vdfai => "VDFAI",
            Self::Vobav => "VOBAV",
            Self::Vrmvp => "VRMVP",
            Self::Vrmvav => "VRMVAV",
            Self::Vacso => "VACSO",
            Self::Vacmcu => "VACMCU",
            Self::Vsonif => "VSONIF",
            Self::Vrdla => "VRDLA",
            Self::Wacmcl => "WACMCL",
            Self::Wouc => "WOUC",
        }
    }

    /// The severity of this code: [`Severity::Warning`] for the `W`-prefixed
    /// codes, [`Severity::Error`] otherwise (see [`Severity`]).
    #[must_use]
    pub fn severity(self) -> Severity {
        match self {
            Self::Wacmcl | Self::Wouc => Severity::Warning,
            _ => Severity::Error,
        }
    }
}

impl std::fmt::Display for ValidationCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.mnemonic())
    }
}

/// A single validation finding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationIssue {
    /// The catalogue code.
    pub code: ValidationCode,
    /// The severity (mirrors [`ValidationCode::severity`]).
    pub severity: Severity,
    /// A concrete, human-readable description of the specific violation.
    pub message: String,
    /// The archetype path the issue is anchored at, where derivable.
    pub path: Option<String>,
    /// A source byte span, for source-level checks (e.g. VOKU) where derivable.
    pub span: Option<std::ops::Range<usize>>,
}

impl ValidationIssue {
    /// Build an issue for `code` with `message`, deriving the severity.
    pub(crate) fn new(code: ValidationCode, message: impl Into<String>) -> Self {
        Self {
            code,
            severity: code.severity(),
            message: message.into(),
            path: None,
            span: None,
        }
    }

    /// Attach an archetype path.
    #[must_use]
    pub(crate) fn at_path(mut self, path: impl Into<String>) -> Self {
        self.path = Some(path.into());
        self
    }
}

/// A minimal in-memory archetype repository — the parent/supplier seam the
/// specialisation-aware checks resolve against.
///
/// Keyed on the `publisher-package-class.concept` portion of the HRID (version
/// family and namespace are ignored for lookup), so a child's
/// `parent_archetype_id` (`…redefine_occurrences.v1`) resolves to the parsed
/// parent (`…redefine_occurrences.v1.0.0`).
#[derive(Debug, Default)]
pub struct ArchetypeRepository {
    by_id: HashMap<String, Archetype>,
}

impl ArchetypeRepository {
    /// A new, empty repository.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a parsed archetype under its HRID key.
    pub fn insert(&mut self, archetype: Archetype) {
        let key = hrid_lookup_key(view(&archetype).archetype_id);
        self.by_id.insert(key, archetype);
    }

    /// Resolve a raw archetype-id reference (as it appears in a
    /// `parent_archetype_id` / external ref) to a registered archetype.
    #[must_use]
    pub fn get(&self, raw_id: &str) -> Option<&Archetype> {
        self.by_id.get(&raw_id_lookup_key(raw_id))
    }
}

/// The `publisher-package-class.concept` lookup key of an [`ArchetypeHrid`].
fn hrid_lookup_key(h: &ArchetypeHrid) -> String {
    format!(
        "{}-{}-{}.{}",
        h.rm_publisher, h.rm_package, h.rm_class, h.concept_id
    )
}

/// The lookup key of a raw archetype-id string (strips an optional `ns::`
/// namespace prefix and the trailing `.vN…` version).
fn raw_id_lookup_key(raw: &str) -> String {
    let no_ns = raw.rsplit("::").next().unwrap_or(raw);
    match version_marker(no_ns) {
        Some(idx) => no_ns.get(..idx).unwrap_or(no_ns).to_owned(),
        None => no_ns.to_owned(),
    }
}

/// The byte index of the version marker in an archetype id — the first `.v`
/// immediately followed by a digit (so a concept id starting with `v`, e.g.
/// `…ENTRY.valc_parent…`, is not mistaken for the version).
fn version_marker(s: &str) -> Option<usize> {
    let bytes = s.as_bytes();
    (0..bytes.len().saturating_sub(2))
        .find(|&i| bytes[i] == b'.' && bytes[i + 1] == b'v' && bytes[i + 2].is_ascii_digit())
}

/// Validate an assembled [`Archetype`] against the AOM2 **phase-1** catalogue.
///
/// `repo`, when supplied, resolves the archetype's parent (and suppliers) so
/// the parent-dependent phase-1 checks (VACSD depth comparison, VASID, VALC,
/// VTPL) can run; when `None`, those checks compute only their standalone half
/// or are skipped. Source-level checks that need the raw ODIN text (VOKU) are
/// not run here — use [`validate_source_phase1`] for those.
#[must_use]
pub fn validate_phase1(
    archetype: &Archetype,
    repo: Option<&ArchetypeRepository>,
) -> Vec<ValidationIssue> {
    let mut issues = Vec::new();
    phase1::run(&view(archetype), repo, None, &mut issues);
    issues
}

/// Parse ADL2 `src` and validate it against the AOM2 **phase-1** catalogue,
/// including the source-level checks (VOKU keyed-list uniqueness) that need the
/// raw ODIN text.
///
/// # Errors
/// Returns the parse [`SyntaxError`]s if `src` does not parse into an
/// [`Archetype`]; validation runs only on a successful parse.
pub fn validate_source_phase1(
    src: &str,
    repo: Option<&ArchetypeRepository>,
) -> Result<Vec<ValidationIssue>, Vec<SyntaxError>> {
    let source = parse_source(src)?;
    let archetype = crate::assemble::assemble(&source, src)?;
    let mut issues = Vec::new();
    phase1::run(&view(&archetype), repo, Some((&source, src)), &mut issues);
    Ok(issues)
}

/// A borrowed, artefact-kind-agnostic view of an [`Archetype`]'s common fields
/// — the single access point the checks read through.
pub(crate) struct ArchetypeView<'a> {
    pub(crate) kind: ArtefactKind,
    pub(crate) archetype_id: &'a ArchetypeHrid,
    pub(crate) parent_archetype_id: Option<&'a str>,
    pub(crate) definition: &'a CComplexObject,
    pub(crate) terminology: &'a ArchetypeTerminology,
    pub(crate) rm_overlay: Option<&'a RmOverlay>,
    pub(crate) original_language: Option<&'a TerminologyCode>,
    pub(crate) description: Option<&'a ResourceDescription>,
    pub(crate) translations:
        Option<&'a std::collections::BTreeMap<String, openehr_base::prelude::TranslationDetails>>,
    pub(crate) annotations: Option<&'a ResourceAnnotations>,
    pub(crate) adl_version: Option<&'a str>,
    pub(crate) rm_release: &'a str,
    pub(crate) is_differential: bool,
}

impl ArchetypeView<'_> {
    /// True if this archetype specialises a parent.
    pub(crate) fn is_specialised(&self) -> bool {
        self.parent_archetype_id.is_some()
    }

    /// The archetype's specialisation level = the specialisation depth of its
    /// root node id (master07 §Specialisation Depth; VARCN).
    pub(crate) fn specialisation_level(&self) -> usize {
        crate::codes::specialisation_depth(crate::paths::complex_node_id(self.definition))
            .unwrap_or(0)
    }
}

/// Build an [`ArchetypeView`] over any [`Archetype`] variant.
pub(crate) fn view(archetype: &Archetype) -> ArchetypeView<'_> {
    match archetype {
        Archetype::AuthoredArchetype(a) => match a.as_ref() {
            AuthoredArchetype::AuthoredArchetype(d) => ArchetypeView {
                kind: ArtefactKind::Archetype,
                archetype_id: &d.archetype_id,
                parent_archetype_id: d.parent_archetype_id.as_deref(),
                definition: &d.definition,
                terminology: &d.terminology,
                rm_overlay: d.rm_overlay.as_ref(),
                original_language: Some(&d.original_language),
                description: d.description.as_deref(),
                translations: d.translations.as_ref(),
                annotations: d.annotations.as_ref(),
                adl_version: d.adl_version.as_deref(),
                rm_release: &d.rm_release,
                is_differential: d.is_differential,
            },
            AuthoredArchetype::Template(t) => ArchetypeView {
                kind: ArtefactKind::Template,
                archetype_id: &t.archetype_id,
                parent_archetype_id: t.parent_archetype_id.as_deref(),
                definition: &t.definition,
                terminology: &t.terminology,
                rm_overlay: t.rm_overlay.as_ref(),
                original_language: Some(&t.original_language),
                description: t.description.as_ref(),
                translations: t.translations.as_ref(),
                annotations: t.annotations.as_ref(),
                adl_version: t.adl_version.as_deref(),
                rm_release: &t.rm_release,
                is_differential: t.is_differential,
            },
            AuthoredArchetype::OperationalTemplate(o) => ArchetypeView {
                kind: ArtefactKind::OperationalTemplate,
                archetype_id: &o.archetype_id,
                parent_archetype_id: o.parent_archetype_id.as_deref(),
                definition: &o.definition,
                terminology: &o.terminology,
                rm_overlay: o.rm_overlay.as_ref(),
                original_language: Some(&o.original_language),
                description: o.description.as_ref(),
                translations: o.translations.as_ref(),
                annotations: o.annotations.as_ref(),
                adl_version: o.adl_version.as_deref(),
                rm_release: &o.rm_release,
                is_differential: o.is_differential,
            },
        },
        Archetype::TemplateOverlay(t) => ArchetypeView {
            kind: ArtefactKind::TemplateOverlay,
            archetype_id: &t.archetype_id,
            parent_archetype_id: t.parent_archetype_id.as_deref(),
            definition: &t.definition,
            terminology: &t.terminology,
            rm_overlay: t.rm_overlay.as_ref(),
            original_language: None,
            description: None,
            translations: None,
            annotations: None,
            adl_version: None,
            rm_release: "",
            is_differential: t.is_differential,
        },
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn every_code_has_a_unique_mnemonic_and_severity() {
        let all = [
            ValidationCode::Vardt,
            ValidationCode::Varcn,
            ValidationCode::Stcnt,
            ValidationCode::Vacsd,
            ValidationCode::Volt,
            ValidationCode::Varav,
            ValidationCode::Varrv,
            ValidationCode::Votm,
            ValidationCode::Vdifv,
            ValidationCode::Vdifp,
            ValidationCode::Vatcv,
            ValidationCode::Vtsd,
            ValidationCode::Vtlc,
            ValidationCode::Vttbk,
            ValidationCode::Vtcbk,
            ValidationCode::Vetdf,
            ValidationCode::Vtvsid,
            ValidationCode::Vtvsmd,
            ValidationCode::Vtvsuq,
            ValidationCode::Vdsev,
            ValidationCode::Vdsiv,
            ValidationCode::Varxra,
            ValidationCode::Varxnc,
            ValidationCode::Varxav,
            ValidationCode::Varxr,
            ValidationCode::Varxtv,
            ValidationCode::Vatid,
            ValidationCode::Vatcd,
            ValidationCode::Vatdf,
            ValidationCode::Vacdf,
            ValidationCode::Vatda,
            ValidationCode::Vranp,
            ValidationCode::Voku,
            ValidationCode::Varid,
            ValidationCode::Vdeol,
            ValidationCode::Vard,
            ValidationCode::Vasid,
            ValidationCode::Valc,
            ValidationCode::Vtpl,
            ValidationCode::Vrrlp,
            ValidationCode::Vcocd,
            ValidationCode::Vcoid,
            ValidationCode::Vcosu,
            ValidationCode::Vcatu,
            ValidationCode::Vdfai,
            ValidationCode::Vobav,
            ValidationCode::Vrmvp,
            ValidationCode::Vrmvav,
            ValidationCode::Vacso,
            ValidationCode::Vacmcu,
            ValidationCode::Vsonif,
            ValidationCode::Vrdla,
            ValidationCode::Wacmcl,
            ValidationCode::Wouc,
        ];
        let mut seen = std::collections::HashSet::new();
        for c in all {
            assert!(seen.insert(c.mnemonic()), "duplicate mnemonic {c}");
            let expected = if c.mnemonic().starts_with('W') {
                Severity::Warning
            } else {
                Severity::Error
            };
            assert_eq!(c.severity(), expected, "{c} severity");
        }
        assert_eq!(seen.len(), 54);
    }
}
