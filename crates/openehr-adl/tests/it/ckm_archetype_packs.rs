// SPDX-FileCopyrightText: FerroEHR contributors
// SPDX-License-Identifier: MIT

//! Breadth gate over the vendored real-world archetype packs — both dialects.
//!
//! Two packs, two sources, because the two dialects are published in
//! different places (`.claude/rules/vendored-corpora.md`):
//!
//! - `corpus/archetypes/ckm/adl14/` — every
//!   archetype the public openEHR CKM publishes, as **ADL 1.4**
//!   (`scripts/vendor/ckm-archetypes.sh`). CKM publishes no ADL 2 export.
//! - `corpus/archetypes/adl2/ckm-2013-12-09/` —
//!   upstream's own CKM export carrying `*.adls` (**ADL 2**) beside `*.adl`
//!   (ADL 1.4) twins of the same archetypes, pinned by commit
//!   (`scripts/vendor/adl2-archetypes.sh`).
//!
//! The claim here is deliberately narrow and total: every file in both packs
//! PARSES in its own dialect. That is the breadth net the hand-written
//! `adl14-cadl`/`adl14-dadl` trees cannot provide — real clinical archetypes
//! carry constraint spellings, translation blocks and terminology sections no
//! authored fixture reproduces. Deeper claims (phase-1 validation, flattening)
//! stay with the rule-code-keyed `adl2-reference` corpus, whose file names
//! encode the expected outcome.
//!
//! NOTE: no openEHR spec governs 1.4 tolerance — our own design
//! ([`openehr_adl::adl14`]). Outcomes here are pinned by the packs themselves.
//!
//! Corpus discipline: 100% exercised, adjudicated refusals only. A file our
//! reader rejects is listed in the pack's `ADJUDICATED` table with the reason;
//! it stays vendored so the refusal is pinned, and a reader that starts
//! accepting it fails this gate.
#![allow(
    clippy::expect_used,
    clippy::panic,
    reason = "integration-test assertions and fixture plumbing outside #[test] fns, which the clippy.toml allow-*-in-tests scoping does not reach"
)]

use std::path::{Path, PathBuf};

use openehr_adl::assemble::parse_artefact;
use openehr_adl::error::SyntaxErrorCode;
use openehr_adl::parse::Dialect;

/// Archetypes the conformant reader MUST refuse, with the syntax code the
/// refusal must carry and the adjudication behind it.
///
/// These are NEGATIVE tests, not a skip list: the gate asserts the refusal
/// happens AND carries the stated code. A file that starts parsing, or that
/// refuses with a different code, fails the gate — so a silently loosened
/// reader cannot hide here, and neither can a mis-coded error.
type Refusal = (&'static str, SyntaxErrorCode, &'static str);

/// One adjudicated defect family in the live CKM library.
///
/// A duplicate sibling container key in the terminology ODIN — incomplete
/// authoring, not a language feature. `LANG` ODIN `master05-content.adoc`
/// §VDOBU: "object identifier uniqueness: sibling objects occurring within a
/// container attribute must be uniquely identified with respect to each
/// other."
///
/// (The former second family — EMPTY inline dADL domain blocks, 9 files — was
/// re-adjudicated to ACCEPTANCE under #1465: the dADL chapter's own grammar
/// admits the empty block and §Empty Sections allows it anywhere, so it lowers
/// to the open constraint; see `adl14/domain.rs` `lower_adl14_domain`.)
const ADJUDICATED_CKM_ADL14: &[Refusal] = &[
    // ── duplicate sibling container key in terminology ODIN (VDOBU) ───────
    (
        "openEHR-DEMOGRAPHIC-ITEM_TREE.person_details.v0.adl",
        SyntaxErrorCode::Sdinv,
        "terminology ODIN repeats the sibling container key [\"at0310\"] (VDOBU)",
    ),
];

/// The upstream paired pack's adjudicated refusals — the same VDOBU defect
/// family, in three `.adl` files of the 2013 CKM export. The `.adls` half of
/// the directory has none, which is why the expected count is derived per file
/// set ([`expected_refusals`]) rather than from this table's length.
const ADJUDICATED_PAIRS: &[Refusal] = &[
    (
        "openEHR-EHR-CLUSTER.palpation-external_ear.v1.adl",
        SyntaxErrorCode::Sdinv,
        "terminology ODIN repeats the sibling container key [\"at0019\"] (VDOBU)",
    ),
    (
        "openEHR-EHR-CLUSTER.palpation-joint.v1.adl",
        SyntaxErrorCode::Sdinv,
        "terminology ODIN repeats the sibling container key [\"at0019\"] (VDOBU)",
    ),
    (
        "openEHR-EHR-OBSERVATION.lab_test-immunology-ANA.v1.adl",
        SyntaxErrorCode::Sdinv,
        "terminology ODIN repeats the sibling container key [\"at0.97\"] (VDOBU)",
    ),
];

fn artifacts_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../corpus/archetypes")
}

/// Every file with `ext` under `dir`, recursively, sorted.
fn files_with_extension(dir: &Path, ext: &str) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        let entries = match std::fs::read_dir(&d) {
            Ok(entries) => entries,
            Err(e) => panic!("read pack dir {}: {e}", d.display()),
        };
        for path in entries.flatten().map(|e| e.path()) {
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|e| e == ext) {
                out.push(path);
            }
        }
    }
    out.sort();
    out
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .and_then(std::ffi::OsStr::to_str)
        .unwrap_or("?")
        .to_owned()
}

/// The outcome of parsing a whole pack.
struct PackOutcome {
    /// Files that failed to parse with no adjudication covering them.
    failures: Vec<String>,
    /// Adjudicated refusals that now parse, or refuse with the wrong code —
    /// either way the adjudication no longer describes reality.
    broken_adjudications: Vec<String>,
    /// Files that parsed clean.
    clean: usize,
    /// Adjudicated refusals that refused with exactly the expected code.
    refused: usize,
}

/// Parse every file in `files` under `dialect`, asserting the positive claim
/// (parses clean) for ordinary files and the NEGATIVE claim (refuses, with the
/// stated code) for adjudicated ones.
fn parse_all(files: &[PathBuf], dialect: Dialect, adjudicated: &[Refusal]) -> PackOutcome {
    let mut out = PackOutcome {
        failures: Vec::new(),
        broken_adjudications: Vec::new(),
        clean: 0,
        refused: 0,
    };
    for path in files {
        let name = file_name(path);
        let expected = adjudicated
            .iter()
            .find(|(file, _, _)| *file == name)
            .map(|(_, code, reason)| (*code, *reason));
        let src = std::fs::read_to_string(path).expect("read archetype source");
        match (parse_artefact(&src, dialect), expected) {
            // ordinary file, parses — the positive claim
            (Ok(_), None) => out.clean += 1,
            // adjudicated refusal that now parses — the adjudication is stale
            (Ok(_), Some((code, reason))) => out.broken_adjudications.push(format!(
                "{name}: expected refusal {code} ({reason}) but the file now PARSES — \
                 remove the adjudication if that is a genuine fix, or investigate a \
                 loosened reader"
            )),
            // ordinary file that fails — a real, unadjudicated defect
            (Err(errors), None) => out.failures.push(format!("{name}: {errors:?}")),
            // adjudicated refusal — the refusal must carry the stated code
            (Err(errors), Some((code, reason))) => {
                if errors.iter().any(|e| e.code == code) {
                    out.refused += 1;
                } else {
                    out.broken_adjudications.push(format!(
                        "{name}: expected refusal {code} ({reason}) but got {:?}",
                        errors.iter().map(|e| e.code).collect::<Vec<_>>()
                    ));
                }
            }
        }
    }
    out
}

/// How many adjudicated refusals actually belong to this file set (the pairs
/// table is shared by the `.adls` and `.adl` halves of one directory).
fn expected_refusals(files: &[PathBuf], adjudicated: &[Refusal]) -> usize {
    adjudicated
        .iter()
        .filter(|(name, _, _)| files.iter().any(|p| file_name(p) == *name))
        .count()
}

fn assert_pack(label: &str, out: &PackOutcome, total: usize, expected_refusals: usize) {
    assert!(
        out.broken_adjudications.is_empty(),
        "{label}: adjudications that no longer describe reality:\n{}",
        out.broken_adjudications.join("\n")
    );
    assert!(
        out.failures.is_empty(),
        "{label}: {} of {total} files failed to parse and are not adjudicated:\n{}",
        out.failures.len(),
        out.failures.join("\n")
    );
    // Every adjudicated refusal was reached and refused with its stated code —
    // the negative half of the claim, asserted rather than assumed.
    assert_eq!(
        out.refused, expected_refusals,
        "{label}: {} of {expected_refusals} adjudicated refusals were exercised",
        out.refused
    );
    assert_eq!(
        out.clean + out.refused,
        total,
        "{label}: accounting mismatch over the pack ({} clean + {} refused != {total})",
        out.clean,
        out.refused
    );
}

/// Every archetype of the full CKM library parses in the ADL 1.4 dialect.
#[test]
fn ckm_adl14_pack_parses() {
    let dir = artifacts_root().join("ckm/adl14");
    let files = files_with_extension(&dir, "adl");
    assert!(
        files.len() >= 900,
        "the CKM archetype pack is missing: found {} files in {} — re-run \
         scripts/vendor/ckm-archetypes.sh",
        files.len(),
        dir.display()
    );
    let out = parse_all(&files, Dialect::Adl14, ADJUDICATED_CKM_ADL14);
    assert_pack(
        "CKM ADL 1.4 pack",
        &out,
        files.len(),
        expected_refusals(&files, ADJUDICATED_CKM_ADL14),
    );
}

/// Every ADL 2 archetype of the upstream paired pack parses in the ADL 2
/// dialect.
#[test]
fn upstream_adl2_pack_parses() {
    let dir = artifacts_root().join("adl2/ckm-2013-12-09");
    let files = files_with_extension(&dir, "adls");
    assert!(
        files.len() >= 300,
        "the upstream ADL 2 pack is missing: found {} files in {} — re-run \
         scripts/vendor/adl2-archetypes.sh",
        files.len(),
        dir.display()
    );
    let out = parse_all(&files, Dialect::Adl2, ADJUDICATED_PAIRS);
    assert_pack(
        "upstream ADL 2 pack",
        &out,
        files.len(),
        expected_refusals(&files, ADJUDICATED_PAIRS),
    );
}

/// The ADL 1.4 twins shipped beside those ADL 2 files parse in the 1.4
/// dialect — the same archetype, both dialects, both readable.
#[test]
fn upstream_adl14_twins_parse() {
    let dir = artifacts_root().join("adl2/ckm-2013-12-09");
    let files = files_with_extension(&dir, "adl");
    assert!(
        files.len() >= 300,
        "the upstream ADL 1.4 twins are missing: found {} files in {}",
        files.len(),
        dir.display()
    );
    let out = parse_all(&files, Dialect::Adl14, ADJUDICATED_PAIRS);
    assert_pack(
        "upstream ADL 1.4 twins",
        &out,
        files.len(),
        expected_refusals(&files, ADJUDICATED_PAIRS),
    );
}

/// The RM resource-meta rows over the whole CKM 1.4 pack.
///
/// Enforcement of the RM common ch.8 invariants on 1.4 sources
/// (`validate::resource_meta`) was audited clean against the vendored
/// real-world library first, so it newly rejects no previously-accepted
/// archetype — this sweep IS that record. Only the resource-meta codes are
/// asserted: other validity findings on real-world content are the
/// rule-code-keyed corpus' territory.
#[test]
fn ckm_adl14_pack_is_resource_meta_clean() {
    let resource_codes = [
        "AUTHORED_RESOURCE.Original_language_valid",
        "AUTHORED_RESOURCE.Translations_valid",
        "AUTHORED_RESOURCE.Description_valid",
        "TRANSLATION_DETAILS.Language_valid",
        "RESOURCE_DESCRIPTION.Original_author_valid",
        "RESOURCE_DESCRIPTION.Lifecycle_state_valid",
        "RESOURCE_DESCRIPTION.Details_valid",
        "RESOURCE_DESCRIPTION_ITEM.Language_valid",
        "RESOURCE_DESCRIPTION_ITEM.Purpose_valid",
        "RESOURCE_DESCRIPTION_ITEM.Use_valid",
        "RESOURCE_DESCRIPTION_ITEM.misuse_valid",
    ];
    let dir = artifacts_root().join("ckm/adl14");
    let files = files_with_extension(&dir, "adl");
    assert!(files.len() >= 900);
    let mut offenders = Vec::new();
    for path in &files {
        let src = std::fs::read_to_string(path).expect("archetype is readable");
        let Ok(issues) = openehr_adl::validate::validate_adl14_source(
            &src,
            &openehr_adl::validate::rm::ProductionRmModel,
        ) else {
            continue; // adjudicated parse refusals are ckm_adl14_pack_parses' claim
        };
        for issue in issues
            .iter()
            .filter(|i| i.severity == openehr_adl::validate::catalogue::Severity::Error)
            .filter(|i| resource_codes.contains(&i.code.mnemonic()))
        {
            offenders.push(format!(
                "{}: {} — {}",
                file_name(path),
                issue.code.mnemonic(),
                issue.message
            ));
        }
    }
    assert!(
        offenders.is_empty(),
        "CKM archetypes refused by the resource-meta pass (adjudicate, never silence):\n{}",
        offenders.join("\n")
    );
}
