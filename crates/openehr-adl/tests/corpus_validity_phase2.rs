//! Phase-2 specialisation validation corpus harness.
//!
//! Walks the `validity/specialisation/**` corpus (plus the four slot-
//! specialisation fixtures under `validity/slots/`) and asserts the phase-2
//! validator's behaviour against each file's authoritative `regression` tag
//! (the oracle per `tests/corpus/INVENTORY.md`, never the filename).
//!
//! The full gated pipeline (`validate_source`: phase 1 → RM → phase-2
//! specialisation) runs with a repository built over the whole corpus so the
//! flat parent and external references resolve. A level-0 parent is its own flat
//! form (`ADL2/master09.02` §Differential and Flat Forms); a specialised parent
//! needs the flattener and is adjudicated ([`FlatParent::NeedsFlattener`]).
//!
//! Spec oracle for the codes:
//! `docs/specs/openehr/AM/docs/AOM2/master04.5-constraint_model-class_definitions.adoc`
//! §Validity Rules + `master08-validation.adoc` §Phase 2.

// A test harness: vendored-fixture reads/parses are asserted to succeed.
#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::print_stderr
)]

use std::path::{Path, PathBuf};

use openehr_adl::assemble::parse_artefact;
use openehr_adl::validate::rm::ProductionRmModel;
use openehr_adl::validate::{
    ArchetypeRepository, FlatParent, Severity, resolve_flat_parent, validate_source,
};

const CORPUS: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/corpus/adl2-reference");

/// The phase-2 (and gated phase-1) codes this harness asserts a tag names is
/// raised on its fixture.
const PHASE2_FIRING: &[&str] = &[
    "VSANCE", "VSANCC", "VSONIN", "VSSM", "VDIFP", "VCORMT", "VARXID", "VARXS", "VARXR", "VDSSID",
    "VSONCO", // gated phase-1 codes reached through the same pipeline:
    "VACSD", "VTSD",
];

/// The extra slot-specialisation fixtures claimed from `validity/slots/`.
const SLOT_FIXTURES: &[&str] = &[
    "validity/slots/openEHR-EHR-SECTION.VARXID_filler_id_not_valid.v1.0.0.adls",
    "validity/slots/openEHR-EHR-SECTION.VARXS_slot_id_mismatch.v1.0.0.adls",
    "validity/slots/openEHR-EHR-SECTION.VARXR_slot_id_match_but_not_found.v1.0.0.adls",
    "validity/slots/openEHR-EHR-SECTION.VDSSID_slot_redefine_bad_id.v1.0.0.adls",
];

/// Documented adjudications — files skipped with a spec-cited reason (never a
/// silent exclusion).
fn adjudicated(name: &str) -> Option<&'static str> {
    if name.ends_with("VSONCO_redefine_occurrences.v1.0.0.adls") {
        // The declared parent `redefine_occurrences.v1` is itself a specialised
        // archetype (root id1.1), so its deep flat form needs the flattener
        // (`ADL2/master09.02` §Differential and Flat Forms — only a top-level
        // parent is its own flat form). VSONCO's collective-occurrences rule
        // (master04.5 §C_OBJECT VSONCO L359-379) is exercised by the level-0
        // `new_VSONCO-redef_to_multiple_singles-FAIL` fixture instead.
        // TODO: flatten a specialised parent, then un-adjudicate this file.
        Some(
            "parent is specialised — deep flat form needs the flattener (VSONCO covered by new_VSONCO-…-FAIL)",
        )
    } else if name.ends_with("VCORMT_illegal_redef_of_ac_code_node.v1.0.0.adls") {
        // The child redefines a terminology-code leaf (`defining_code`) to a
        // C_STRING under the ac-coded node of the flat parent (spec_test_obs2).
        // In our model this reduces to a C_TERMINOLOGY_CODE→C_STRING meta-type
        // change (VSONT, master04.5 §C_OBJECT L342), which the corpus tags
        // VCORMT; resolving the ac-code leaf's RM type (CODE_PHRASE) needs the
        // deep terminology-code redefinition machinery.
        // TODO: map primitive-leaf RM types so the ac-code redefinition raises
        // VCORMT (master04.5 §C_OBJECT VCORMT L327-328).
        Some(
            "ac-code-leaf → C_STRING redefinition needs primitive-leaf RM typing (VCORMT vs VSONT)",
        )
    } else if name.ends_with("VPOV_redef_ac_code_node_to_local_codes.v1.0.0.adls") {
        // VPOV here requires comparing the child's ac-code value-set expansion to
        // the flat parent's (`value_set_expanded`, master04.5 §C_TERMINOLOGY_NODE
        // L663-699); the flat terminology comes from the flattener.
        // TODO: value-set-expansion subset once the flattener supplies the flat
        // terminology.
        Some("VPOV needs value-set-expansion subset from the flattened terminology")
    } else {
        None
    }
}

fn normalise_tag(tag: &str) -> String {
    match tag {
        "VDIFP1" => "VDIFP".to_owned(),
        "VSONCOm" => "VSONCO".to_owned(),
        other => other.to_owned(),
    }
}

fn read_tag_raw(src: &str) -> Option<String> {
    let idx = src.find("regression")?;
    let rest = src.get(idx..)?;
    let open = rest.find("<\"")? + 2;
    let after = rest.get(open..)?;
    let end = after.find('"')?;
    after.get(..end).map(str::to_owned)
}

fn adls_files(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&d) else {
            continue;
        };
        for entry in rd.flatten() {
            let p = entry.path();
            if p.is_dir() {
                stack.push(p);
            } else if p.extension().is_some_and(|e| e == "adls") {
                out.push(p);
            }
        }
    }
    out.sort();
    out
}

fn build_repository() -> ArchetypeRepository {
    let mut repo = ArchetypeRepository::new();
    for path in adls_files(Path::new(CORPUS)) {
        let Ok(src) = std::fs::read_to_string(&path) else {
            continue;
        };
        if let Ok(art) = parse_artefact(&src) {
            repo.insert(art);
        }
    }
    repo
}

#[derive(Default)]
struct Counts {
    exact_code: usize,
    pass_clean: usize,
    parent_not_found: usize,
    adjudicated: usize,
    non_specialised: usize,
}

#[test]
fn corpus_phase2_outcomes() {
    let repo = build_repository();
    let rm = ProductionRmModel;
    let mut counts = Counts::default();
    let mut violations: Vec<String> = Vec::new();

    let mut paths = adls_files(&PathBuf::from(format!("{CORPUS}/validity/specialisation")));
    for extra in SLOT_FIXTURES {
        paths.push(PathBuf::from(format!("{CORPUS}/{extra}")));
    }
    paths.sort();

    for path in &paths {
        let name = path
            .strip_prefix(CORPUS)
            .unwrap_or(path)
            .display()
            .to_string();
        if let Some(reason) = adjudicated(&name) {
            counts.adjudicated += 1;
            eprintln!("adjudicated {name}: {reason}");
            continue;
        }
        let Ok(src) = std::fs::read_to_string(path) else {
            violations.push(format!("{name}: unreadable"));
            continue;
        };
        let tag = read_tag_raw(&src).map(|t| normalise_tag(&t));

        let Ok(archetype) = parse_artefact(&src) else {
            // A non-parsing fixture is owned by the parse gates; a FAIL tag is a
            // valid outcome here.
            if tag.as_deref() == Some("FAIL") {
                counts.parent_not_found += 1;
            } else {
                violations.push(format!("{name}: failed to parse (tag {tag:?})"));
            }
            continue;
        };

        // FAIL fixtures whose parent is deliberately absent assert the typed
        // parent-not-found outcome (INVENTORY §5).
        if tag.as_deref() == Some("FAIL") {
            match resolve_flat_parent(&archetype, &repo) {
                FlatParent::NotFound => {
                    counts.parent_not_found += 1;
                }
                other => violations.push(format!(
                    "{name}: FAIL fixture expected parent-not-found, got {other:?}"
                )),
            }
            continue;
        }

        // Non-specialised support archetypes (the level-0 parents that live in
        // this directory) are not phase-2 subjects — their RM/phase-1 validation
        // is the reference-model / phase-1 harnesses' concern (and some exercise
        // an emit-rm-model gap, e.g. spec_test_obs3's DV_QUANTITY attributes).
        if matches!(
            resolve_flat_parent(&archetype, &repo),
            FlatParent::NotSpecialised
        ) {
            counts.non_specialised += 1;
            continue;
        }

        let Ok(issues) = validate_source(&src, Some(&repo), &rm) else {
            violations.push(format!("{name}: validate_source parse error"));
            continue;
        };
        let error_codes: Vec<String> = issues
            .iter()
            .filter(|i| i.severity == Severity::Error)
            .map(|i| i.code.mnemonic().to_owned())
            .collect();

        match tag.as_deref() {
            None | Some("PASS") => {
                if error_codes.is_empty() {
                    counts.pass_clean += 1;
                } else {
                    violations.push(format!("{name}: PASS/untagged but raised {error_codes:?}"));
                }
            }
            Some(t) if PHASE2_FIRING.contains(&t) => {
                if error_codes.iter().any(|c| c == t) {
                    counts.exact_code += 1;
                } else {
                    violations.push(format!("{name}: expected {t} but raised {error_codes:?}"));
                }
            }
            Some(t) => {
                violations.push(format!(
                    "{name}: unhandled tag {t} (raised {error_codes:?})"
                ));
            }
        }
    }

    eprintln!(
        "phase-2 corpus: exact={} pass_clean={} parent_not_found={} adjudicated={} non_specialised={} ({} files)",
        counts.exact_code,
        counts.pass_clean,
        counts.parent_not_found,
        counts.adjudicated,
        counts.non_specialised,
        paths.len(),
    );

    assert!(
        violations.is_empty(),
        "phase-2 corpus violations ({}):\n{}",
        violations.len(),
        violations.join("\n")
    );
}
