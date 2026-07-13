//! Terminology-side artefact validity for the OPT 1.4 pass (T17): the codes,
//! bindings and language-consistency rules of the AOM2 terminology package.
//!
//! Rules enforced (`AOM2/master07-terminology_package.adoc`,
//! `AOM2/master08-validation.adoc`):
//! - **VATID** — every at-code used as a `node_id` in the definition is
//!   defined in terminology (master08 line 56).
//! - **VTTBK** — every term-binding key is a defined archetype term or a path
//!   (master07 line 77).
//! - **VTCBK** — every constraint-binding key is a defined ac-code
//!   (master07 line 80).
//! - **VTLC** — all term/constraint codes exist in all languages
//!   (master07 line 74).
//!
//! This module also owns the global code-collection helpers and the flattened
//! ontology / nested-root accessors the whole pass shares.

use std::collections::HashSet;

use openehr_its::opt14::{
    CArchetypeRoot, CAttribute, CObject, Codedefinitionset, FlatArchetypeOntology,
    OperationalTemplate,
};

use super::Violation;

// ─── VATID (node-id codes defined in terminology) ───────────────────────────────

/// VATID: "check that all codes mentioned in `definition` are defined in
/// terminology" (`AOM2/master08-validation.adoc` line 56). Applied to at-code
/// `node_id`s (the addressable, sibling-identifying codes, AOM14/04 §`Node_id`).
/// Empty `node_ids` (non-addressable leaves) and non-`at`/`id` codes are exempt.
/// The defined-code set is collected globally across the flattened OPT (the
/// definition roots + every `component_ontologies` set), which is deliberately
/// lenient about per-archetype scoping — it still catches a `node_id` that is
/// defined nowhere while never mis-rejecting a correctly-scoped code.
pub(super) fn check_node_id(node_id: &str, defined_at: &HashSet<String>) -> Result<(), Violation> {
    if !is_at_code(node_id) {
        return Ok(());
    }
    if !defined_at.contains(node_id) {
        return Err(Violation::new(
            "VATID",
            format!("node_id '{node_id}' is used in the definition but not defined in terminology"),
        ));
    }
    Ok(())
}

/// An addressable archetype term code: `at0000`, `at0001.1`, or the ADL2 `id`
/// form. A bare, empty, or free-text `node_id` is not an at-code.
pub(super) fn is_at_code(code: &str) -> bool {
    let rest = code
        .strip_prefix("at")
        .or_else(|| code.strip_prefix("id"))
        .unwrap_or("");
    rest.starts_with(|c: char| c.is_ascii_digit())
}

// ─── VTTBK / VTCBK (binding key validity) ───────────────────────────────────────

/// VTTBK: "terminology term binding key valid. Every term binding must be to
/// either a defined archetype term ('at-code') or to a path that is valid in the
/// flat archetype." (`AOM2/master07-terminology_package.adoc` line 77.) A `/`
/// path key is accepted without full path resolution (conservative — never
/// mis-reject a real flat path).
pub(super) fn check_term_bindings(
    opt: &OperationalTemplate,
    defined_at: &HashSet<String>,
) -> Result<(), Violation> {
    let check = |code: &str| -> Result<(), Violation> {
        // PORT NOTE (flattened-OPT tolerance): a *specialised* at-code
        // (`at0.23`, dot-notation — AOM2 §specialisation depth) may be bound
        // without a re-emitted local term definition: archie-era flattening
        // keeps parent-archetype bindings whose definitions live in the parent
        // (the vendored blood-pressure corpus OPTs carry these). A dotted
        // at-code is therefore accepted as a valid binding key.
        let specialised = code.starts_with("at") && code.contains('.');
        if code.starts_with('/') || !is_at_code(code) || specialised || defined_at.contains(code) {
            return Ok(());
        }
        Err(Violation::new(
            "VTTBK",
            format!(
                "term binding key '{code}' is neither a defined archetype term (at-code) nor a path"
            ),
        ))
    };
    for set in &opt.definition.term_bindings {
        for item in &set.items {
            check(&item.code)?;
        }
    }
    for onto in flat_ontologies(opt) {
        for set in &onto.term_bindings {
            for item in &set.items {
                check(&item.code)?;
            }
        }
    }
    // Nested C_ARCHETYPE_ROOTs carry their own term_bindings.
    for root in nested_roots(opt) {
        for set in &root.term_bindings {
            for item in &set.items {
                check(&item.code)?;
            }
        }
    }
    Ok(())
}

/// VTCBK: "terminology constraint binding key valid. Every constraint binding
/// must be to a defined archetype constraint code ('ac-code')."
/// (`AOM2/master07-terminology_package.adoc` line 80.)
pub(super) fn check_constraint_bindings(
    opt: &OperationalTemplate,
    defined_ac: &HashSet<String>,
) -> Result<(), Violation> {
    for onto in flat_ontologies(opt) {
        for set in &onto.constraint_bindings {
            for item in &set.items {
                if !defined_ac.contains(&item.code) {
                    return Err(Violation::new(
                        "VTCBK",
                        format!(
                            "constraint binding key '{}' is not a defined archetype constraint \
                             code (ac-code)",
                            item.code
                        ),
                    ));
                }
            }
        }
    }
    Ok(())
}

// ─── VTLC (language consistency) ────────────────────────────────────────────────

/// VTLC: "language consistency. Languages consistent: all term codes and
/// constraint codes exist in all languages."
/// (`AOM2/master07-terminology_package.adoc` line 74.) Applied to each
/// `FlatArchetypeOntology` whose `term_definitions` / `constraint_definitions`
/// are grouped per language: every language must define the same code set.
///
/// PORT NOTE: the per-`C_ARCHETYPE_ROOT` `term_definitions` (a flat
/// `Vec<ARCHETYPE_TERM>`, single-language) carry no language grouping, so VTLC
/// is inert for a single-language OPT — the multi-language code sets live only
/// in `ontology` / `component_ontologies`.
pub(super) fn check_language_consistency(opt: &OperationalTemplate) -> Result<(), Violation> {
    for onto in flat_ontologies(opt) {
        language_consistent(&codes_by_language(&onto.term_definitions), "term")?;
        language_consistent(
            &codes_by_language(&onto.constraint_definitions),
            "constraint",
        )?;
    }
    Ok(())
}

fn codes_by_language(sets: &[Codedefinitionset]) -> Vec<(String, HashSet<String>)> {
    sets.iter()
        .map(|s| {
            (
                s.language.clone(),
                s.items.iter().map(|t| t.code.clone()).collect(),
            )
        })
        .collect()
}

fn language_consistent(by_lang: &[(String, HashSet<String>)], kind: &str) -> Result<(), Violation> {
    if by_lang.len() < 2 {
        return Ok(());
    }
    let (ref_lang, ref_codes) = &by_lang[0];
    for (lang, codes) in &by_lang[1..] {
        if codes != ref_codes {
            let missing: Vec<&str> = ref_codes
                .symmetric_difference(codes)
                .map(String::as_str)
                .collect();
            return Err(Violation::new(
                "VTLC",
                format!(
                    "the {kind} code set differs between languages '{ref_lang}' and '{lang}' \
                     (e.g. {missing:?}); all codes must exist in all languages"
                ),
            ));
        }
    }
    Ok(())
}

// ─── code collection + ontology / nested-root accessors ─────────────────────────

/// Every archetype term (`at`/`id`) code defined anywhere in the flattened OPT.
pub(super) fn collect_defined_at_codes(opt: &OperationalTemplate) -> HashSet<String> {
    let mut out = HashSet::new();
    out.extend(
        opt.definition
            .term_definitions
            .iter()
            .map(|t| t.code.clone()),
    );
    for root in nested_roots(opt) {
        out.extend(root.term_definitions.iter().map(|t| t.code.clone()));
    }
    for onto in flat_ontologies(opt) {
        for set in &onto.term_definitions {
            out.extend(set.items.iter().map(|t| t.code.clone()));
        }
    }
    out
}

/// Every archetype constraint (`ac`) code defined in the flattened OPT.
pub(super) fn collect_defined_ac_codes(opt: &OperationalTemplate) -> HashSet<String> {
    let mut out = HashSet::new();
    for onto in flat_ontologies(opt) {
        for set in &onto.constraint_definitions {
            out.extend(set.items.iter().map(|t| t.code.clone()));
        }
    }
    out
}

/// `ontology` + every `component_ontologies` entry.
pub(super) fn flat_ontologies(opt: &OperationalTemplate) -> Vec<&FlatArchetypeOntology> {
    opt.ontology
        .iter()
        .chain(opt.component_ontologies.iter())
        .collect()
}

/// Every nested `C_ARCHETYPE_ROOT` under the definition (the flattened slot
/// fillers), excluding the root definition itself.
fn nested_roots(opt: &OperationalTemplate) -> Vec<&CArchetypeRoot> {
    let mut out = Vec::new();
    for attr in &opt.definition.attributes {
        collect_roots_in_attr(attr, &mut out);
    }
    out
}

fn collect_roots_in_attr<'a>(attr: &'a CAttribute, out: &mut Vec<&'a CArchetypeRoot>) {
    let children = match attr {
        CAttribute::CSingleAttribute(a) => &a.children,
        CAttribute::CMultipleAttribute(a) => &a.children,
    };
    for child in children {
        if let CObject::CArchetypeRoot(root) = child {
            out.push(root);
            for a in &root.attributes {
                collect_roots_in_attr(a, out);
            }
        } else {
            for a in super::co_attributes(child) {
                collect_roots_in_attr(a, out);
            }
        }
    }
}
