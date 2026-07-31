//! Template / external-reference filler validation (VTPL + VARXR).
//!
//! These two checks both need the supplier repository (a filler is resolved,
//! flattened, and inspected), so they run only when a repository is available —
//! separately from the standalone phase-1/2 passes. They un-defer:
//!
//! * **VTPL** — template/filler language consistency (`master03` §Validity
//!   Rules, VTPL): every filler flattened into a *template* must support the
//!   template's `original_language`, so the flattened OPT can carry a common
//!   language (`OPT2/master03` §Terminology). Only templates are subject to
//!   VTPL — a plain archetype that merely references a mono-lingual filler is
//!   still valid, since it is not (yet) being flattened into an OPT.
//! * **VARXR** — external reference resolution (`master08` §Phase 2 → Validate
//!   Specialised Definition; `master04.5` §`C_ARCHETYPE_ROOT`): every
//!   `use_archetype` external reference / slot-filler must resolve to an
//!   archetype in the repository.
//!
//! Both walk the *flattened* artefact (`crate::flatten::flat_form`) so fillers
//! inherited from a specialisation parent are seen.

use openehr_am::am24::aom2::archetype::archetype::Archetype;
use openehr_am::am24::aom2::constraint_model::c_archetype_root::CArchetypeRoot;
use openehr_am::am24::aom2::constraint_model::c_attribute::CAttribute;
use openehr_am::am24::aom2::constraint_model::c_complex_object::CComplexObject;
use openehr_am::am24::aom2::constraint_model::c_object::CObject;

use super::{ValidationCode, ValidationIssue};
use crate::artefact::{ArchetypeRepository, ArchetypeView, view};
use crate::flatten::flat_form;
use crate::source::ArtefactKind;

/// Validate an archetype's `use_archetype` fillers against `repo`: VARXR (a
/// reference that does not resolve) and — for templates only — VTPL (a filler
/// that does not support the template's `original_language`).
///
/// The artefact is flattened first so inherited fillers are seen; a flatten
/// failure yields no filler issues (the flattener's own errors surface through
/// the specialisation validator).
#[must_use]
pub fn validate_fillers(archetype: &Archetype, repo: &ArchetypeRepository) -> Vec<ValidationIssue> {
    let mut issues = Vec::new();
    let Ok(flat) = flat_form(archetype, repo) else {
        return issues;
    };
    let v = view(&flat);
    let is_template = matches!(
        v.kind,
        ArtefactKind::Template | ArtefactKind::TemplateOverlay
    );
    let root_language = template_language(&v);
    let mut roots = Vec::new();
    collect_roots(v.definition, &mut roots);
    for root in roots {
        check_filler(
            root,
            repo,
            is_template,
            root_language.as_deref(),
            &mut issues,
        );
    }
    issues
}

/// The template's declared original language code (`master07.13`
/// `ARCHETYPE_TERMINOLOGY.original_language` / the language section), for the
/// VTPL comparison.
fn template_language(v: &ArchetypeView<'_>) -> Option<String> {
    if !v.terminology.original_language.is_empty() {
        return Some(v.terminology.original_language.clone());
    }
    v.original_language.map(|c| c.code_string.clone())
}

/// Check one filler root: VARXR if it does not resolve; VTPL (templates only) if
/// the resolved filler's flat languages do not include `root_language`.
fn check_filler(
    root: &CArchetypeRoot,
    repo: &ArchetypeRepository,
    is_template: bool,
    root_language: Option<&str>,
    issues: &mut Vec<ValidationIssue>,
) {
    if root.archetype_ref.is_empty() {
        return;
    }
    let Some(constituent) = repo.get(&root.archetype_ref) else {
        issues.push(
            ValidationIssue::new(
                ValidationCode::Varxr,
                format!(
                    "external reference {:?} does not resolve to an archetype in the repository",
                    root.archetype_ref
                ),
            )
            .at_path(root.node_id.clone()),
        );
        return;
    };
    if !is_template {
        return;
    }
    let Some(lang) = root_language else {
        return;
    };
    // The filler must support the template's language in its flat terminology
    // (VTPL). Flatten the filler so inherited translations are counted.
    let Ok(flat_filler) = flat_form(constituent, repo) else {
        return;
    };
    if !filler_supports_language(&view(&flat_filler), lang) {
        issues.push(
            ValidationIssue::new(
                ValidationCode::Vtpl,
                format!(
                    "slot filler {:?} does not support the template language {lang:?}",
                    root.archetype_ref
                ),
            )
            .at_path(root.node_id.clone()),
        );
    }
}

/// True if a filler's flat terminology carries `lang` (as its original
/// language, a translation, or a `term_definitions` language bucket).
fn filler_supports_language(v: &ArchetypeView<'_>, lang: &str) -> bool {
    if v.terminology.original_language == lang {
        return true;
    }
    if v.terminology.term_definitions.contains_key(lang) {
        return true;
    }
    v.original_language.is_some_and(|c| c.code_string == lang)
        || v.translations.is_some_and(|t| t.contains_key(lang))
}

/// Collect every `C_ARCHETYPE_ROOT` in the definition tree (the fillers /
/// external references).
fn collect_roots<'a>(def: &'a CComplexObject, out: &mut Vec<&'a CArchetypeRoot>) {
    match def {
        CComplexObject::CComplexObject(d) => {
            for attr in &d.attributes {
                collect_roots_attr(attr, out);
            }
        }
        CComplexObject::CArchetypeRoot(r) => {
            out.push(r);
            for attr in &r.attributes {
                collect_roots_attr(attr, out);
            }
        }
    }
}

fn collect_roots_attr<'a>(attr: &'a CAttribute, out: &mut Vec<&'a CArchetypeRoot>) {
    for child in &attr.children {
        if let CObject::CComplexObject(cco) = child {
            collect_roots(cco, out);
        }
    }
}
