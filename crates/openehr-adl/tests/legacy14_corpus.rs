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
//! - `validity/legacy_adl_1.4/*.adl` — parse (1.4 dialect) clean, EXCEPT
//!   `FAIL_c_dv_quantity_minimal.v1.adl` which rejects with `SDINV` (its
//!   `regression` tag) — an empty `(C_DV_QUANTITY) <>` inline dADL block.
//! - `features/**/*.adl` — parse (1.4 dialect) clean.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use std::path::{Path, PathBuf};

use openehr_adl::adl14::convert::{ConvertConfig, parse_and_convert};
use openehr_adl::adl14::log::ConversionLog;
use openehr_adl::assemble::parse_artefact_adl14;
use openehr_adl::error::SyntaxErrorCode;

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
        files.len() >= 21,
        "expected the full 1.4 `.adl` census (>= 21), found {}",
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
            }
            claimed += 1;
        } else if r.contains("features/") {
            // The lone 1.4 features source (intervention_decisions.v0).
            parse_artefact_adl14(&src)
                .unwrap_or_else(|e| panic!("{r}: 1.4-tolerant parse failed: {e:?}"));
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
