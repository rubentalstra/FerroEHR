//! Guard: every committed adjudication register must load and validate — a
//! malformed or uncited register would otherwise only surface mid-run.
//!
//! Two register kinds live under `adjudications/`:
//! - `ecc-own.toml` — the own-corpus register (vendored-data defects,
//! spec-cited; standing rule 3).
//! - `<sut>*.toml` — foreign-SUT fairness registers (X1 absorption).
#![allow(clippy::expect_used)]

use std::path::Path;

use conformance::adjudication::OwnRegister;
use conformance::fairness::AdjudicationRegister;

#[test]
fn committed_registers_load_and_validate() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("adjudications");
    let mut fairness_checked = 0;
    let mut own_checked = 0;
    for entry in std::fs::read_dir(&dir).expect("read adjudications/") {
        let path = entry.expect("dir entry").path();
        if path.extension().and_then(|e| e.to_str()) != Some("toml") {
            continue;
        }
        if path.file_name().and_then(|n| n.to_str()) == Some("ecc-own.toml") {
            let reg = OwnRegister::load(&path)
                .unwrap_or_else(|e| panic!("own register {} is invalid: {e}", path.display()));
            assert!(
                !reg.is_empty(),
                "own register {} has no entries — remove it or add cited entries",
                path.display()
            );
            own_checked += 1;
        } else {
            let reg = AdjudicationRegister::load(&path)
                .unwrap_or_else(|e| panic!("register {} is invalid: {e}", path.display()));
            assert!(
                !reg.is_empty(),
                "register {} has no rules — remove it or add cited entries",
                path.display()
            );
            fairness_checked += 1;
        }
    }
    assert!(
        fairness_checked >= 1,
        "expected at least the seeded ehrbase-java fairness register in adjudications/"
    );
    assert!(
        own_checked >= 1,
        "expected the own-corpus register adjudications/ecc-own.toml (the golden-dialect entries)"
    );
}
