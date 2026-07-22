//! The committed comparison report (`docs/conformance/cnf-comparison.md`)
//! must always equal a fresh regeneration from the committed map + catalogue
//! (hand-typed numbers are a failure, not a style issue). The gate itself
//! (zero pending) is asserted only at cutover — until then the report
//! honestly prints the open gap.
#![allow(clippy::panic, clippy::expect_used)] // test assertions/fixtures

use cnf_runner::artifacts::load_root;
use cnf_runner::compare;

#[test]
fn committed_comparison_report_matches_regeneration() {
    let crate_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let repo = crate_dir.ancestors().nth(2).expect("repo root");

    let loaded = load_root(&crate_dir.join("artifacts")).expect("schema compilation");
    assert!(
        loaded.errors.is_empty(),
        "artifact tree must load cleanly for the comparison: {:?}",
        loaded.errors.first()
    );
    let (_, report) = compare::run(
        &crate_dir.join("comparison/ecc-catalog.tsv"),
        &crate_dir.join("comparison/ecc-map.yaml"),
        &loaded.set,
    )
    .expect("comparison inputs");

    let committed = std::fs::read_to_string(repo.join("docs/conformance/cnf-comparison.md"))
        .expect("committed report");
    assert_eq!(
        committed, report,
        "docs/conformance/cnf-comparison.md drifted — regenerate with `cnf-runner compare-ecc`"
    );
}
