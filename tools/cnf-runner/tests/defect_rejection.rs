//! Every seeded-defect fixture is rejected with the expected gate — the
//! validator's negative battery (one fixture per machine gate, schema-level
//! through cross-artifact).
#![allow(clippy::panic, clippy::expect_used)] // test assertions/fixtures

use cnf_runner::artifacts::load_root;
use cnf_runner::validate::{CheckId, Context, validate};

/// (fixture file, overlay destination inside the copied world, expected gate)
const DEFECTS: &[(&str, &str, &str)] = &[
    // JSON-Schema-level rejections (the hard file-validation requirement)
    ("case-unknown-field.yaml", "schedule/zz-defect.yaml", "load"),
    (
        "case-bad-outcome-kind.yaml",
        "schedule/zz-defect.yaml",
        "load",
    ),
    ("case-bad-kind.yaml", "schedule/zz-defect.yaml", "load"),
    (
        "binding-bad-status.yaml",
        "bindings/its-rest/zz-defect.yaml",
        "load",
    ),
    (
        "binding-unknown-outcome-key.yaml",
        "bindings/its-rest/zz-defect.yaml",
        "load",
    ),
    (
        "binding-unrealized-and-realized.yaml",
        "bindings/its-rest/zz-defect.yaml",
        "load",
    ),
    (
        "register-bad-disposition.yaml",
        "registers/ambiguities.yaml",
        "load",
    ),
    (
        "register-option-select-without-options.yaml",
        "registers/ambiguities.yaml",
        "load",
    ),
    (
        "corpus-missing-provenance.yaml",
        "corpus/MANIFEST.yaml",
        "load",
    ),
    (
        "corpus-invalid-without-defect.yaml",
        "corpus/MANIFEST.yaml",
        "load",
    ),
    (
        "matrix-bad-tier.yaml",
        "vocab/capability_matrix.yaml",
        "load",
    ),
    ("outcomes-missing-kind.yaml", "vocab/outcomes.yaml", "load"),
    // typed-model rejections
    ("case-bad-ref-form.yaml", "schedule/zz-defect.yaml", "load"),
    (
        "case-bad-capture-source.yaml",
        "schedule/zz-defect.yaml",
        "load",
    ),
    (
        "case-duplicate-yaml-key.yaml",
        "schedule/zz-defect.yaml",
        "load",
    ),
    // cross-artifact rejections
    (
        "case-duplicate-id.yaml",
        "schedule/zz-defect.yaml",
        "id-uniqueness",
    ),
    (
        "case-unresolved-sm-op.yaml",
        "schedule/zz-defect.yaml",
        "sm-operation",
    ),
    (
        "case-bad-spec-ref.yaml",
        "schedule/zz-defect.yaml",
        "spec-ref",
    ),
    (
        "case-unmapped-outcome.yaml",
        "schedule/zz-defect.yaml",
        "binding-completeness",
    ),
    (
        "case-unmapped-capture.yaml",
        "schedule/zz-defect.yaml",
        "binding-completeness",
    ),
    (
        "case-unresolved-verified-by.yaml",
        "schedule/zz-defect.yaml",
        "verified-by",
    ),
    (
        "case-missing-corpus-key.yaml",
        "schedule/zz-defect.yaml",
        "corpus-integrity",
    ),
    (
        "case-missing-view.yaml",
        "schedule/zz-defect.yaml",
        "corpus-integrity",
    ),
    (
        "case-unresolved-ambiguity.yaml",
        "schedule/zz-defect.yaml",
        "ambiguity-link",
    ),
    (
        "case-unresolved-option.yaml",
        "schedule/zz-defect.yaml",
        "option-tag",
    ),
    (
        "case-unknown-capability.yaml",
        "schedule/zz-defect.yaml",
        "capability-tier",
    ),
    (
        "case-profile-tier-mismatch.yaml",
        "schedule/zz-defect.yaml",
        "capability-tier",
    ),
    (
        "case-aggregate-without-single-pass.yaml",
        "schedule/zz-defect.yaml",
        "kind-shape",
    ),
    (
        "case-two-field-predicates.yaml",
        "schedule/zz-defect.yaml",
        "kind-shape",
    ),
    (
        "case-signature-no-fact.yaml",
        "schedule/zz-defect.yaml",
        "kind-shape",
    ),
    (
        "case-undefined-capture-ref.yaml",
        "schedule/zz-defect.yaml",
        "reference-grammar",
    ),
    (
        "case-bad-literal.yaml",
        "schedule/zz-defect.yaml",
        "literal-grammar",
    ),
    (
        "corpus-missing-source.yaml",
        "corpus/MANIFEST.yaml",
        "corpus-integrity",
    ),
    (
        "outcomes-wrong-class.yaml",
        "vocab/outcomes.yaml",
        "vocab-drift",
    ),
];

fn copy_tree(from: &std::path::Path, to: &std::path::Path) {
    std::fs::create_dir_all(to).expect("mkdir");
    for entry in std::fs::read_dir(from).expect("read_dir") {
        let entry = entry.expect("entry");
        let target = to.join(entry.file_name());
        if entry.path().is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), &target).expect("copy");
        }
    }
}

#[test]
fn every_seeded_defect_is_rejected_by_its_gate() {
    let crate_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let specs = crate_dir
        .ancestors()
        .nth(2)
        .expect("repo root")
        .join("docs/specs/openehr");
    let fixtures = crate_dir.join("tests/fixtures/defects");

    // Every fixture on disk must be in the table — no silent dead fixtures.
    let mut on_disk: Vec<String> = std::fs::read_dir(&fixtures)
        .expect("defects dir")
        .map(|e| e.expect("entry").file_name().to_string_lossy().into_owned())
        .collect();
    on_disk.sort();
    let mut in_table: Vec<String> = DEFECTS.iter().map(|(f, _, _)| (*f).to_owned()).collect();
    in_table.sort();
    in_table.dedup();
    assert_eq!(
        on_disk, in_table,
        "defect fixtures and the expectation table diverge"
    );

    for (fixture, dest, expected) in DEFECTS {
        let temp = assert_fs::TempDir::new().expect("temp dir");
        copy_tree(&crate_dir.join("artifacts"), temp.path());
        let dest_path = temp.path().join(dest);
        std::fs::create_dir_all(dest_path.parent().expect("parent")).expect("mkdir");
        std::fs::copy(fixtures.join(fixture), &dest_path).expect("overlay");

        let loaded = load_root(temp.path()).expect("schema compilation");
        let findings = validate(&Context {
            set: &loaded.set,
            load_errors: &loaded.errors,
            spec_root: Some(&specs),
        });
        let hit = findings.iter().any(|f| f.check.token() == *expected);
        assert!(
            hit,
            "{fixture}: expected a `{expected}` finding, got:\n{}",
            findings
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join("\n")
        );
        // The clean world produced zero findings, so any finding at all is
        // attributable to the seeded defect; the specific-gate assertion
        // above pins WHICH gate caught it.
        assert!(
            !findings.is_empty(),
            "{fixture}: defect produced no findings at all"
        );
        let _ = CheckId::Load; // the enum is the vocabulary the tokens come from
    }
}
