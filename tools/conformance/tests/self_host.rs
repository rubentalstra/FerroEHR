//! Feature-gated end-to-end proof (design §8 step 2–3): boot the self-hosted SUT
//! (testcontainers PG18 + the real app in-process) and run the transcribed cases
//! against it — proving the whole pipeline (SUT lifecycle → transport → cases →
//! assertions → results) with real fixtures, under **both** JSON and XML so the
//! format-parameterized composition cases (master07) execute.
//!
//! Requires Docker; run with `cargo test -p conformance --features self-host`.
#![cfg(feature = "self-host")]
#![allow(clippy::expect_used)]

use conformance::case::Format;
use conformance::results::CaseStatus;
use conformance::run::{RunConfig, run};
use conformance::sut::Sut;

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn master06_ehr_cases_run_against_self_hosted_sut() {
    let sut = Sut::self_hosted()
        .await
        .expect("boot self-hosted SUT (is Docker running?)");

    let config = RunConfig {
        filter: None,
        profile: None,
        formats: vec![Format::Json, Format::Xml],
        versions: conformance::version::SpecVersions::latest(),
        auth_mode: "basic (self-host, RBAC off)".to_owned(),
    };
    let results = run(sut.transport(), &config).await.expect("run");

    // Print every case outcome for visibility.
    for c in &results.cases {
        println!(
            "{:<52} {:?} {}/{} {}",
            c.id,
            c.status,
            c.passed_data_sets,
            c.total_data_sets,
            c.message.as_deref().unwrap_or("")
        );
    }

    // The pipeline itself must not error (no transport faults).
    assert!(
        !results
            .cases
            .iter()
            .any(|c| c.status == CaseStatus::Errored),
        "no case should error at the transport level"
    );

    // The heart case must pass all 16 data sets.
    let main = results
        .cases
        .iter()
        .find(|c| c.id == "I_EHR_SERVICE.create_ehr-main")
        .expect("create_ehr-main ran");
    assert_eq!(main.status, CaseStatus::Passed, "{:?}", main.message);
    assert_eq!(main.passed_data_sets, 16);

    // The inventory is still fully classified.
    assert!(results.executed() > 0, "the selection executed cases");
}

/// The content-chapter (master15/16/17.x) data-validation cases against the
/// self-hosted SUT. Prints every outcome for visibility; asserts only that no
/// case errors at the transport level. The pass/fail split is the finding
/// surface (design §4.5): a driven constraint case that FAILS is an open finding
/// (the SUT accepted a value the truth table rejects), not a masked skip.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn content_cases_run_against_self_hosted_sut() {
    let sut = Sut::self_hosted()
        .await
        .expect("boot self-hosted SUT (is Docker running?)");

    let config = RunConfig {
        filter: Some("CONT-".to_owned()),
        profile: None,
        formats: vec![Format::Json],
        versions: conformance::version::SpecVersions::latest(),
        auth_mode: "basic (self-host, RBAC off)".to_owned(),
    };
    let results = run(sut.transport(), &config).await.expect("run");

    let mut passed = 0u32;
    let mut failed = 0u32;
    let mut skipped = 0u32;
    for c in &results.cases {
        match c.status {
            CaseStatus::Passed => passed += 1,
            CaseStatus::Failed => failed += 1,
            CaseStatus::Skipped => skipped += 1,
            CaseStatus::Errored => {}
        }
        println!(
            "{:<48} {:?} {}/{} {}",
            c.id,
            c.status,
            c.passed_data_sets,
            c.total_data_sets,
            c.message.as_deref().unwrap_or("")
        );
    }
    println!("CONTENT SUMMARY: {passed} passed, {failed} failed (findings), {skipped} skipped");

    assert!(
        !results
            .cases
            .iter()
            .any(|c| c.status == CaseStatus::Errored),
        "no content case should error at the transport level"
    );

    let status = |id: &str| {
        results
            .cases
            .iter()
            .find(|c| c.id == id)
            .unwrap_or_else(|| panic!("{id} ran"))
            .status
    };

    // Archetype constraints our validator ENFORCES → the driven case PASSES (both
    // the vendored valid composition accepted and the constraint-violating copy
    // rejected). These lock in correct behaviour; a regression flips them.
    for id in [
        "CONT-DV_QUANTITY-validate_property_units",
        "CONT-DV_ORDINAL-validate_constraint",
        "CONT-DV_CODED_TEXT-validate_local_codes",
        // ITEM_STRUCTURE ITEM_TREE narrowing: the strict typed RM validation on
        // commit now rejects an ITEM_LIST committed into an ITEM_TREE-narrowed slot
        // (the sibling ITEM_STR type_item_list/table/single cases remain open
        // findings — those swap directions are not yet rejected, see below).
        "CONT-ITEM_STR-type_item_tree",
    ] {
        assert_eq!(status(id), CaseStatus::Passed, "{id} must pass (enforced)");
    }

    // Archetype constraints our validator does NOT yet enforce → the driven case
    // FAILS: the SUT accepts (201) a value the truth table rejects (422). Recorded
    // as open findings (F-open-30/31), never masked as a skip; each assertion
    // flips to Passed when the validator gap is closed.
    for id in [
        // F-open-30: C_DATE_TIME field-validity pattern not enforced.
        "CONT-DV_DATE_TIME-validate_constraint",
        // F-open-31: ITEM_STRUCTURE type narrowing (Class not allowed) not enforced
        // for these swap directions (ITEM_TREE into a LIST/TABLE/SINGLE-narrowed
        // slot is accepted); the ITEM_TREE-narrowed direction is now enforced (above).
        "CONT-ITEM_STR-type_item_list",
        "CONT-ITEM_STR-type_item_table",
        "CONT-ITEM_STR-type_item_single",
        // F-open-40: DV_PROPORTION `type` C_INTEGER.list not enforced. The
        // `minimal_action_2` OPT constrains `type` to {3,4}; the SUT accepts
        // `type=0` (201) where master17.3 §validate_any_fraction rejects it (422).
        "CONT-DV_PROPORTION-validate_any_fraction",
        // F-open: `from_flat` drops COMPOSITION.territory when the FLAT fixture
        // stores it root-prefixed (`event_series/territory|code`) instead of
        // `ctx/territory` (openehr-flat context.rs `ctx_get` only reads `ctx/`), so
        // the converted `time_series` baseline lacks the mandatory `territory` and
        // the (now strict, RM-typed) commit validation rejects it. The DV_QUANTITY
        // magnitude-range constraint itself is still enforced — this finding is the
        // FLAT-converter gap, tracked for the from_flat root-prefix fix.
        "CONT-DV_QUANTITY-validate_property_units_mag",
        // F1: the WebTemplate builder's `requires_cardinality` returns false for a
        // min<=1 interval, so COMPOSITION.content / HISTORY.events cardinality lower
        // bound 1 (`1..*`, `1..1`) and upper bounds (`0..1`, `1..1`) are not
        // enforced — only `min>1` (`3..*`, `3..5`) surfaces. Representative rows:
        "CONT-COMP-content_card_1plus-context_any",
        "CONT-COMP-content_card_opt-context_any",
        "CONT-COMP-content_card_mand-context_any",
        // F2: nested HISTORY.summary existence (1..1) not enforced.
        "CONT-HIST-events_card_any-summary_ex_mand",
        // F3: DV_COUNT C_INTEGER.list not enforced (validation::leaf checks count
        // range only, not an enumerated list).
        "CONT-DV_COUNT-validate_list",
    ] {
        assert_eq!(
            status(id),
            CaseStatus::Failed,
            "{id} is an open finding (SUT accepts a value the truth table rejects)"
        );
    }
}

/// The runner-defined SIGN-* capability cases (design §4.6) against the
/// self-hosted SUT: the four digest cases must PASS (the SUT signs by default,
/// `digest` mode — version-signing.md §3.4) and `SIGN-pgp-verifies` must be
/// SKIPPED (the self-hosted SUT is not in `pgp` mode).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn sign_capability_cases_run_against_self_hosted_sut() {
    let sut = Sut::self_hosted()
        .await
        .expect("boot self-hosted SUT (is Docker running?)");

    let config = RunConfig {
        filter: Some("SIGN-".to_owned()),
        profile: None,
        formats: vec![Format::Json, Format::Xml],
        versions: conformance::version::SpecVersions::latest(),
        auth_mode: "basic (self-host, RBAC off)".to_owned(),
    };
    let results = run(sut.transport(), &config).await.expect("run");

    for c in &results.cases {
        println!(
            "{:<24} {:<5} {:?} {}/{} {}",
            c.id,
            c.format,
            c.status,
            c.passed_data_sets,
            c.total_data_sets,
            c.message.as_deref().unwrap_or("")
        );
    }

    // No transport-level errors.
    assert!(
        !results
            .cases
            .iter()
            .any(|c| c.status == CaseStatus::Errored),
        "no SIGN case should error at the transport level"
    );

    let outcome = |id: &str, fmt: &str| {
        results
            .cases
            .iter()
            .find(|c| c.id == id && c.format == fmt)
            .unwrap_or_else(|| panic!("{id} ({fmt}) ran"))
    };

    // The four digest cases prove the capability in canonical JSON.
    for id in [
        "SIGN-digest-present",
        "SIGN-digest-recomputes",
        "SIGN-all-kinds",
        "SIGN-client-verbatim",
    ] {
        let c = outcome(id, "json");
        assert_eq!(c.status, CaseStatus::Passed, "{id} (json): {:?}", c.message);
    }

    // SIGN-digest-present in canonical XML surfaces the SAME known gap as F-open-6:
    // the versioned-object VERSION endpoints have no canonical-XML serializer yet
    // (406 "canonical XML … once typed payloads land (P12)"). The RM/serialization
    // layer already emits the signature in XML (ehrbase `service_signing::
    // canonical_xml_carries_the_signature`); only the REST negotiation is missing.
    // Recorded as a finding, not weakened away — this assertion flips when the gap
    // is fixed.
    let xml = outcome("SIGN-digest-present", "xml");
    assert_eq!(
        xml.status,
        CaseStatus::Failed,
        "SIGN-digest-present (xml) is expected to fail on the F-open-6 gap: {:?}",
        xml.message
    );
    assert!(
        xml.message.as_deref().unwrap_or_default().contains("406"),
        "the XML failure must be the F-open-6 406, got {:?}",
        xml.message
    );

    // The pgp case is skipped (digest-mode SUT, no configured OpenPGP key).
    let pgp = results
        .cases
        .iter()
        .find(|c| c.id == "SIGN-pgp-verifies")
        .expect("SIGN-pgp-verifies ran");
    assert_eq!(pgp.status, CaseStatus::Skipped, "{:?}", pgp.message);
}
