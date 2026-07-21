//! The production artifact tree (`artifacts/`) passes every machine gate —
//! the eight pilot encodings plus their `verified_by` targets, validated
//! against the vendored spec tree.
#![allow(clippy::panic, clippy::expect_used)] // test assertions/fixtures

use cnf_runner::artifacts::load_root;
use cnf_runner::validate::{Context, validate};

fn repo_root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("repo root")
        .to_owned()
}

#[test]
fn pilot_world_is_clean_under_all_gates() {
    let crate_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let specs = repo_root().join("docs/specs/openehr");
    assert!(
        specs.is_dir(),
        "vendored spec tree missing at {}",
        specs.display()
    );

    let loaded = load_root(&crate_dir.join("artifacts")).expect("schema compilation");
    let findings = validate(&Context {
        set: &loaded.set,
        load_errors: &loaded.errors,
        spec_root: Some(&specs),
    });
    assert!(
        findings.is_empty(),
        "the artifact tree must be clean, found:\n{}",
        findings
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("\n")
    );

    let ids: Vec<&str> = loaded
        .set
        .cases
        .iter()
        .map(|(_, c)| c.id.as_str())
        .collect();
    for pilot in [
        "I_EHR_SERVICE.create_ehr-main",
        "I_EHR_SERVICE.create_ehr-same_ehr_twice",
        "I_DEFINITION_ADL14.upload_opt-invalid_opt",
        "I_EHR_COMPOSITION.update_composition-event",
        "CONT-DV_QUANTITY-validate_property_units_mag",
        "SF-FLAT-commit_roundtrip_ctx_defaults",
        "I_QUERY_SERVICE.execute_ad_hoc_query-where_magnitude",
        "I_EHR_CONTRIBUTION.commit_contribution-valid_invalid_compositions",
        "I_EHR_STATUS.get_ehr_status-get_by_ehr_id",
        "I_DEFINITION_ADL14.get_opts-retrieve_all_no_opts",
    ] {
        assert!(
            ids.contains(&pilot),
            "pilot {pilot} missing from the schedule"
        );
    }
    assert!(loaded.set.bindings.len() >= 10, "binding set shrank");
}
