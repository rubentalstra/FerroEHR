//! Guard: every committed upstream adjudication register (`adjudications/*.toml`)
//! must load and validate. A malformed or uncited register would otherwise only
//! surface at an upstream run — CI catches it here.
#![allow(clippy::expect_used)]

use std::path::Path;

use conformance::adjudication::AdjudicationRegister;

#[test]
fn committed_registers_load_and_validate() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("adjudications");
    let mut checked = 0;
    for entry in std::fs::read_dir(&dir).expect("read adjudications/") {
        let path = entry.expect("dir entry").path();
        if path.extension().and_then(|e| e.to_str()) != Some("toml") {
            continue;
        }
        let reg = AdjudicationRegister::load(&path)
            .unwrap_or_else(|e| panic!("register {} is invalid: {e}", path.display()));
        assert!(
            !reg.is_empty(),
            "register {} has no rules — remove it or add cited entries",
            path.display()
        );
        checked += 1;
    }
    assert!(
        checked >= 1,
        "expected at least the seeded ehrbase-java register in adjudications/"
    );
}
