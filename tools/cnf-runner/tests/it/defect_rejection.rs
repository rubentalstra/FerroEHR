// SPDX-FileCopyrightText: FerroEHR contributors
// SPDX-License-Identifier: MIT

//! Every seeded-defect fixture is rejected with the expected gate — the
//! validator's negative battery (one fixture per machine gate, schema-level
//! through cross-artifact).
//!
//! **One test per defect.** nextest runs each test in its own process, so the
//! battery fans out across the whole process pool instead of grinding through
//! ~40 full validation passes in a single serial test body, and a red row
//! names exactly one fixture instead of aborting the loop. The fixture set and
//! the per-defect tests are generated from ONE list by `defects!`, so they
//! cannot drift apart; [`every_defect_fixture_has_a_test`] then compares that
//! list against the fixture directory, so a fixture added on disk with no test
//! is a failing check, never a silent skip.
//!
//! **The world each defect validates against is materialized, not copied.** A
//! recursive copy of the artifact tree cost ~1.5–4 s per defect (the tree is
//! ~220 MB, dominated by the corpus) and evicted the page cache the next
//! defect's load depends on. [`overlay_world`] instead symlinks the pristine
//! tree and materializes real directories only along the overlaid file's own
//! path, so the loader sees a byte-identical tree with one file replaced or
//! added. Nothing in the battery writes to the tree — `load_root` and
//! `validate` only read — and the overlaid name is never symlinked, so the
//! pristine artifacts can never be written through a link.

#![expect(
    clippy::expect_used,
    reason = "test-support helpers (not `#[test]` fns, so the clippy.toml in-tests scoping does not reach them) are panic-idiomatic: a broken fixture must abort the test loudly, Book ch11"
)]

use std::path::{Path, PathBuf};

use cnf_runner::artifacts::load_root;
use cnf_runner::validate::{Context, validate};

/// The seeded-defect battery: one row per fixture, and one `#[test]` per row.
///
/// Each row is `<test name> => (<fixture file>, <overlay destination inside
/// the world>, <the gate that must reject it>)`. The gate is spelled with the
/// stable token `cnf_runner::validate::CheckId::token` publishes — that enum
/// is the vocabulary these strings come from.
macro_rules! defects {
    ($($test:ident => ($fixture:literal, $dest:literal, $gate:literal)),+ $(,)?) => {
        /// (fixture file, overlay destination inside the world, expected gate)
        const DEFECTS: &[(&str, &str, &str)] = &[$(($fixture, $dest, $gate)),+];

        $(
            #[test]
            fn $test() {
                assert_rejected($fixture, $dest, $gate);
            }
        )+
    };
}

defects! {
    // JSON-Schema-level rejections (the hard file-validation requirement)
    case_unknown_field => ("case-unknown-field.yaml", "schedule/zz-defect.yaml", "load"),
    case_bad_outcome_kind => ("case-bad-outcome-kind.yaml", "schedule/zz-defect.yaml", "load"),
    case_bad_kind => ("case-bad-kind.yaml", "schedule/zz-defect.yaml", "load"),
    binding_bad_status =>
        ("binding-bad-status.yaml", "bindings/its-rest/zz-defect.yaml", "load"),
    binding_unknown_outcome_key =>
        ("binding-unknown-outcome-key.yaml", "bindings/its-rest/zz-defect.yaml", "load"),
    binding_unrealized_and_realized =>
        ("binding-unrealized-and-realized.yaml", "bindings/its-rest/zz-defect.yaml", "load"),
    register_bad_disposition =>
        ("register-bad-disposition.yaml", "registers/ambiguities.yaml", "load"),
    register_option_select_without_options =>
        ("register-option-select-without-options.yaml", "registers/ambiguities.yaml", "load"),
    corpus_missing_provenance =>
        ("corpus-missing-provenance.yaml", "corpus/MANIFEST.yaml", "load"),
    corpus_invalid_without_defect =>
        ("corpus-invalid-without-defect.yaml", "corpus/MANIFEST.yaml", "load"),
    // A declared view with no registered evaluator fails corpus-integrity at
    // validate time, never at run time (#971).
    corpus_view_without_evaluator =>
        ("corpus-view-without-evaluator.yaml", "corpus/MANIFEST.yaml", "corpus-integrity"),
    matrix_bad_tier => ("matrix-bad-tier.yaml", "vocab/capability_matrix.yaml", "load"),
    outcomes_missing_kind => ("outcomes-missing-kind.yaml", "vocab/outcomes.yaml", "load"),

    // typed-model rejections
    case_bad_ref_form => ("case-bad-ref-form.yaml", "schedule/zz-defect.yaml", "load"),
    case_bad_capture_source => ("case-bad-capture-source.yaml", "schedule/zz-defect.yaml", "load"),
    case_duplicate_yaml_key =>
        ("case-duplicate-yaml-key.yaml", "schedule/zz-defect.yaml", "load"),

    // cross-artifact rejections
    case_duplicate_id => ("case-duplicate-id.yaml", "schedule/zz-defect.yaml", "id-uniqueness"),
    case_unresolved_sm_op =>
        ("case-unresolved-sm-op.yaml", "schedule/zz-defect.yaml", "sm-operation"),
    case_bad_spec_ref => ("case-bad-spec-ref.yaml", "schedule/zz-defect.yaml", "spec-ref"),
    case_unmapped_outcome =>
        ("case-unmapped-outcome.yaml", "schedule/zz-defect.yaml", "binding-completeness"),
    case_unmapped_capture =>
        ("case-unmapped-capture.yaml", "schedule/zz-defect.yaml", "binding-completeness"),
    case_unresolved_verified_by =>
        ("case-unresolved-verified-by.yaml", "schedule/zz-defect.yaml", "verified-by"),
    case_missing_corpus_key =>
        ("case-missing-corpus-key.yaml", "schedule/zz-defect.yaml", "corpus-integrity"),
    case_missing_view => ("case-missing-view.yaml", "schedule/zz-defect.yaml", "corpus-integrity"),
    // The EXTRACT a `requires.import` replays resolves like any other corpus
    // reference, so a dangling fixture name is caught before a SUT is composed.
    case_import_missing_corpus_key =>
        ("case-import-missing-corpus-key.yaml", "schedule/zz-defect.yaml", "corpus-integrity"),
    // The closed token vocabularies a bundled CONTRIBUTION member may spell
    // (change_type / _type / lifecycle_state) — an out-of-group state would
    // otherwise commit a version the case never asked for.
    case_bad_member_lifecycle_token =>
        ("case-bad-member-lifecycle-token.yaml", "schedule/zz-defect.yaml", "literal-grammar"),
    case_unresolved_ambiguity =>
        ("case-unresolved-ambiguity.yaml", "schedule/zz-defect.yaml", "ambiguity-link"),
    case_unresolved_option =>
        ("case-unresolved-option.yaml", "schedule/zz-defect.yaml", "option-tag"),
    case_unknown_capability =>
        ("case-unknown-capability.yaml", "schedule/zz-defect.yaml", "capability-tier"),
    case_profile_tier_mismatch =>
        ("case-profile-tier-mismatch.yaml", "schedule/zz-defect.yaml", "capability-tier"),
    case_aggregate_without_single_pass =>
        ("case-aggregate-without-single-pass.yaml", "schedule/zz-defect.yaml", "kind-shape"),
    case_two_field_predicates =>
        ("case-two-field-predicates.yaml", "schedule/zz-defect.yaml", "kind-shape"),
    case_signature_no_fact =>
        ("case-signature-no-fact.yaml", "schedule/zz-defect.yaml", "kind-shape"),
    case_undefined_capture_ref =>
        ("case-undefined-capture-ref.yaml", "schedule/zz-defect.yaml", "reference-grammar"),
    case_bad_literal => ("case-bad-literal.yaml", "schedule/zz-defect.yaml", "literal-grammar"),
    corpus_missing_source =>
        ("corpus-missing-source.yaml", "corpus/MANIFEST.yaml", "corpus-integrity"),
    outcomes_wrong_class => ("outcomes-wrong-class.yaml", "vocab/outcomes.yaml", "vocab-drift"),
    wire_surface_empty =>
        ("wire-surface-empty.yaml", "vocab/wire_surface.yaml", "surface-coverage"),
}

/// This crate's directory (`tools/cnf-runner`).
fn crate_dir() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

/// The seeded-defect fixture directory.
fn fixture_dir() -> PathBuf {
    crate_dir().join("tests/fixtures/defects")
}

/// The vendored spec tree the citation-resolution gates read.
fn spec_root() -> PathBuf {
    crate_dir()
        .ancestors()
        .nth(2)
        .expect("repo root")
        .join("docs/specs/openehr")
}

/// Build the world one defect validates against: the pristine artifact tree,
/// with `dest` replaced (or added) from `fixture`.
///
/// Every entry the overlaid path does not pass through is a symlink to the
/// pristine tree; only the directories along `dest` are real, and the overlaid
/// name itself is never linked, so the copy at the end always creates a fresh
/// file and can never write through into `artifacts/`.
fn overlay_world(dest: &str, fixture: &str) -> assert_fs::TempDir {
    let temp = assert_fs::TempDir::new().expect("temp dir");
    let mut source = crate_dir().join("artifacts");
    let mut target = temp.path().to_owned();

    for component in Path::new(dest) {
        std::fs::create_dir_all(&target).expect("mkdir");
        for entry in std::fs::read_dir(&source).expect("read_dir") {
            let entry = entry.expect("entry");
            if entry.file_name() == component {
                continue; // the overlaid path: materialized, never linked
            }
            std::os::unix::fs::symlink(entry.path(), target.join(entry.file_name()))
                .expect("symlink");
        }
        source = source.join(component);
        target = target.join(component);
    }

    assert!(
        !target.exists(),
        "{dest}: the overlay destination must not exist yet"
    );
    std::fs::copy(fixture_dir().join(fixture), &target).expect("overlay");
    temp
}

/// Load the world holding `fixture` at `dest` and assert the `expected` gate
/// rejected it.
fn assert_rejected(fixture: &str, dest: &str, expected: &str) {
    let world = overlay_world(dest, fixture);
    let specs = spec_root();

    let loaded = load_root(world.path()).expect("schema compilation");
    let findings = validate(&Context {
        set: &loaded.set,
        load_errors: &loaded.errors,
        spec_root: Some(&specs),
    });

    // The clean world produces zero findings (`artifact_gates.rs`), so any
    // finding at all is attributable to the seeded defect; the specific-gate
    // assertion pins WHICH gate caught it.
    assert!(
        !findings.is_empty(),
        "{fixture}: defect produced no findings at all"
    );
    assert!(
        findings.iter().any(|f| f.check.token() == expected),
        "{fixture}: expected a `{expected}` finding, got:\n{}",
        findings
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("\n")
    );
}

/// Every fixture on disk has a test — a fixture nobody exercises is a silent
/// hole in the negative battery, so it fails here rather than passing unseen.
#[test]
fn every_defect_fixture_has_a_test() {
    let mut on_disk: Vec<String> = std::fs::read_dir(fixture_dir())
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
}
