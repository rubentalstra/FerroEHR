//! Feature-gated end-to-end proof (design §8 step 2–3): boot the self-hosted SUT
//! (testcontainers PG18 + the real app in-process) and run the transcribed cases
//! against it — proving the whole pipeline (SUT lifecycle → transport → cases →
//! assertions → results) with real fixtures, under **both** JSON and XML so the
//! format-parameterized composition cases (master07) execute.
//!
//! Requires Docker; run with `cargo test -p ehrbase-conformance --features self-host`.
#![cfg(feature = "self-host")]
#![allow(clippy::expect_used)]

use ehrbase_conformance::case::{Format, Profile};
use ehrbase_conformance::results::CaseStatus;
use ehrbase_conformance::run::{RunConfig, run};
use ehrbase_conformance::sut::Sut;

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn master06_ehr_cases_run_against_self_hosted_sut() {
    let sut = Sut::self_hosted()
        .await
        .expect("boot self-hosted SUT (is Docker running?)");

    let config = RunConfig {
        filter: None,
        profile: None,
        formats: vec![Format::Json, Format::Xml],
        rm_version: "1.2.0".to_owned(),
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
    assert_eq!(results.identified(), 322);
}
