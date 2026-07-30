//! The ADL 1.4 `.adl` corpus coverage harness — every `.adl` source file under
//! `tests/corpus` is exercised with an asserted outcome, closing the `.adl`
//! walker gap the pre-existing `.adls`-only corpus harnesses left.
//!
//! NOTE: no openEHR spec governs 1.4 tolerance/conversion — our own design (see
//! [`openehr_adl::adl14`]). Outcomes are pinned by the corpus (the in-file
//! `regression` tag / directory convention), not a spec clause.
//!
//! Per-category outcome:
//! - `upgrade/upgrade_from_14/*.adl` — converts via [`adl14::convert`] without
//!   error (the structural compare vs the paired `.adls` is in
//!   `adl14_conversion.rs`).
//! - `validity/legacy_adl_1.4/*.adl` — parse (1.4 dialect) clean AND validate
//!   clean under the ADL 1.4 phase-1 subset
//!   ([`openehr_adl::validate::validate_source_phase1_adl14`]; every file here
//!   is `regression`-tagged PASS), EXCEPT `FAIL_c_dv_quantity_minimal.v1.adl`
//!   which rejects at parse with `SDINV` (its `regression` tag) — an empty
//!   `(C_DV_QUANTITY) <>` inline dADL block.
//! - `features/**/*.adl` — parse (1.4 dialect) clean.
//! - `adl14-dadl/*.adl` — the hand-written dADL breadth tree, claimed by
//!   `adl14_dadl_breadth.rs` (accept/refuse per fixture, leaf values
//!   asserted).
//! - `adl14-cadl/*.adl` — the hand-written cADL tree (dialect gates, domain
//!   lowering, VATDF/VACDF, VCOC), claimed by `adl14_cadl_gates.rs`.
#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    reason = "integration-test assertions, diagnostics and fixture plumbing outside #[test] fns, which the clippy.toml allow-*-in-tests scoping does not reach"
)]

use std::path::{Path, PathBuf};

use openehr_adl::adl14::convert::{ConvertConfig, parse_and_convert};
use openehr_adl::adl14::log::ConversionLog;
use openehr_adl::assemble::parse_artefact_adl14;
use openehr_adl::error::SyntaxErrorCode;
use openehr_adl::validate::{Severity, validate_source_phase1_adl14};

fn corpus_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/corpus")
}

fn all_adl_files() -> Vec<PathBuf> {
    let mut out = Vec::new();
    walk(&corpus_root(), &mut out);
    out.retain(|p| p.extension().is_some_and(|e| e == "adl"));
    out.sort();
    out
}

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for e in entries.flatten() {
        let p = e.path();
        if p.is_dir() {
            walk(&p, out);
        } else {
            out.push(p);
        }
    }
}

fn rel(p: &Path) -> String {
    p.strip_prefix(corpus_root())
        .unwrap_or(p)
        .to_string_lossy()
        .replace('\\', "/")
}

#[test]
fn every_adl_file_is_claimed_with_an_outcome() {
    let files = all_adl_files();
    assert!(
        files.len() >= 37,
        "expected the full 1.4 `.adl` census (>= 37), found {}",
        files.len()
    );

    let mut claimed = 0usize;
    for path in &files {
        let src = std::fs::read_to_string(path).expect("read");
        let r = rel(path);
        if r.contains("upgrade/upgrade_from_14/") {
            // Converts without error (structural compare is in adl14_conversion).
            let mut log = ConversionLog::new();
            parse_and_convert(&src, &ConvertConfig::default(), &mut log)
                .unwrap_or_else(|e| panic!("{r}: 1.4 conversion failed: {e}"));
            claimed += 1;
        } else if r.contains("validity/legacy_adl_1.4/") {
            if r.contains("FAIL_c_dv_quantity_minimal") {
                // The SDINV reject: an empty `(C_DV_QUANTITY) <>` domain block.
                let err =
                    parse_artefact_adl14(&src).expect_err("FAIL_c_dv_quantity_minimal must reject");
                assert!(
                    err.iter().any(|e| e.code == SyntaxErrorCode::Sdinv),
                    "{r}: expected SDINV, got {:?}",
                    err.iter().map(|e| e.code).collect::<Vec<_>>()
                );
            } else {
                parse_artefact_adl14(&src)
                    .unwrap_or_else(|e| panic!("{r}: 1.4-tolerant parse failed: {e:?}"));
                // Every legacy_adl_1.4 fixture is `regression`-tagged PASS: it
                // must validate clean under the ADL 1.4 phase-1 subset (no
                // AOM2-only rule may false-reject a valid 1.4 archetype).
                let issues = validate_source_phase1_adl14(&src)
                    .unwrap_or_else(|e| panic!("{r}: 1.4 validation parse failed: {e:?}"));
                let errs: Vec<_> = issues
                    .iter()
                    .filter(|i| i.severity == Severity::Error)
                    .map(|i| (i.code.mnemonic(), i.message.clone()))
                    .collect();
                assert!(
                    errs.is_empty(),
                    "{r}: expected clean 1.4 validation, got errors {errs:?}"
                );
            }
            claimed += 1;
        } else if r.contains("features/") {
            // The lone 1.4 features source (intervention_decisions.v0).
            parse_artefact_adl14(&src)
                .unwrap_or_else(|e| panic!("{r}: 1.4-tolerant parse failed: {e:?}"));
            claimed += 1;
        } else if r.starts_with("adl14-dadl/") {
            // The hand-written dADL breadth tree, owned by
            // `adl14_dadl_breadth.rs` (which asserts each fixture's declared
            // accept/refuse outcome and its leaf values).
            claimed += 1;
        } else if r.starts_with("adl14-cadl/") {
            // The hand-written cADL tree, owned by `adl14_cadl_gates.rs` (which
            // asserts each fixture's declared accept/refuse/invalid outcome).
            claimed += 1;
        } else {
            panic!("unclaimed .adl file (no coverage category): {r}");
        }
    }
    assert_eq!(
        claimed,
        files.len(),
        "every .adl file must be claimed exactly once"
    );
}
