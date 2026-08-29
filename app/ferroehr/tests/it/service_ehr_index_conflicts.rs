// SPDX-FileCopyrightText: FerroEHR contributors
// SPDX-License-Identifier: MIT

//! The advisory EHR-Index duplicate detection
//! (`FerroEhrService::index_conflicts`) against a real `PostgreSQL` 18 (shared
//! testkit harness).
//!
//! `master07 §Overview` names the two error states the index metadata exists
//! "to detect and rectify": multiple EHRs recorded for one subject, and
//! multiple subjects recorded for one EHR. The SM declares no detection
//! OPERATION, so the read itself is our own design — what these tests pin is
//! that it reports exactly those two states, carries every association of each,
//! is purely advisory (it mutates nothing and never refuses), and clears the
//! moment the operator rectifies through the SM write operations (I4/I5).

#![expect(
    clippy::expect_used,
    reason = "clippy's in-test lint scoping (clippy.toml `allow-*-in-tests`) only \
              reaches `#[test]`-annotated functions, so it misses this integration \
              module's helpers and async bodies; panicking assertions are the \
              intended shape here (the Rust Book ch11)"
)]

use ferroehr::service::FerroEhrService;
use ferroehr::service::ehr_index::types::{ResourceInstanceType, ResourceStatus, SubjectRef};

use crate::admin_fixture::repository;

// The reported conflict type lives in a `pub(crate)` module, so an integration
// test cannot name its variants; the derived `Debug` rendering is the only
// surface outside the crate that distinguishes them, and is what these tests
// read.

/// The `Debug` renderings of every conflict one scan reports.
async fn scan(svc: &FerroEhrService) -> Vec<String> {
    svc.index_conflicts()
        .await
        .expect("the advisory scan never refuses")
        .iter()
        .map(|conflict| format!("{conflict:?}"))
        .collect()
}

/// Associate `subject` with a freshly created EHR, returning that EHR's id.
async fn associate(svc: &FerroEhrService, subject: &SubjectRef) -> String {
    let ehr = svc.create_ehr(None).await.expect("ehr").to_string();
    svc.add_ehr_subject(ehr.clone(), subject.clone(), None, None)
        .await
        .expect("add subject");
    ehr
}

/// A clean index reports nothing: neither an empty index nor a set of
/// well-formed 1:1 associations is an error state.
#[tokio::test]
async fn a_one_to_one_index_reports_no_conflict() {
    let (_db, _pool, svc) = repository().await;

    assert_eq!(
        scan(&svc).await,
        Vec::<String>::new(),
        "an empty index has nothing to report"
    );

    associate(&svc, &SubjectRef::person("PID-A", "mpi")).await;
    associate(&svc, &SubjectRef::person("PID-B", "mpi")).await;

    assert_eq!(
        scan(&svc).await,
        Vec::<String>::new(),
        "two subjects on two EHRs is the normal state"
    );
}

/// The "multiple EHRs … created in different locations" state: one subject
/// associated with more than one EHR is reported once, carrying EVERY
/// association of that subject so the operator can pick the `Primary`.
#[tokio::test]
async fn one_subject_on_several_ehrs_is_reported_with_all_its_associations() {
    let (_db, _pool, svc) = repository().await;

    let subject = SubjectRef::person("PID-SHARED", "mpi");
    let first = associate(&svc, &subject).await;
    let second = associate(&svc, &subject).await;
    let third = associate(&svc, &subject).await;
    // An unrelated 1:1 association must not appear in the report.
    let lone = associate(&svc, &SubjectRef::person("PID-LONE", "mpi")).await;

    let conflicts = scan(&svc).await;
    assert_eq!(
        conflicts.len(),
        1,
        "one conflicting subject, got {conflicts:?}"
    );
    let reported = &conflicts[0];
    assert!(
        reported.starts_with("SubjectWithMultipleEhrs"),
        "a subject on several EHRs is that state, got {reported}"
    );
    assert!(
        reported.contains("PID-SHARED"),
        "the report names the conflicting subject, got {reported}"
    );
    for ehr in [&first, &second, &third] {
        assert!(
            reported.contains(ehr.as_str()),
            "every association is carried; {ehr} missing from {reported}"
        );
    }
    assert!(
        !reported.contains(lone.as_str()) && !reported.contains("PID-LONE"),
        "a 1:1 association is not part of the conflict, got {reported}"
    );
}

/// The conflicting subject is reported with its STORED type, not the `PERSON`
/// default the scan starts from — the grouping key is (id, namespace) only, so
/// an `ORGANISATION` subject must not be reported as a person.
#[tokio::test]
async fn the_reported_subject_carries_the_stored_type() {
    let (_db, _pool, svc) = repository().await;

    let subject = SubjectRef {
        id: "ORG-1".to_owned(),
        namespace: "registry".to_owned(),
        r#type: "ORGANISATION".to_owned(),
    };
    associate(&svc, &subject).await;
    associate(&svc, &subject).await;

    let conflicts = scan(&svc).await;
    assert_eq!(conflicts.len(), 1, "one conflicting subject");
    assert!(
        conflicts[0].contains("ORGANISATION"),
        "the stored subject type is reported, got {}",
        conflicts[0]
    );
    assert!(
        !conflicts[0].contains("PERSON"),
        "the PERSON default must not overwrite the stored type, got {}",
        conflicts[0]
    );
}

/// The "records merged … multiple subject ids" state: one EHR associated with
/// more than one subject is reported once, carrying every association with its
/// stored `RESOURCE_INSTANCE_TYPE`, so the operator can see which association
/// was already flagged `Duplicate`.
#[tokio::test]
async fn one_ehr_with_several_subjects_is_reported_with_all_its_associations() {
    let (_db, _pool, svc) = repository().await;

    let primary = SubjectRef::person("PID-1", "mpi");
    let duplicate = SubjectRef::person("PID-2", "mpi");
    let ehr = associate(&svc, &primary).await;
    svc.add_ehr_subject(
        ehr.clone(),
        duplicate.clone(),
        Some(ResourceStatus {
            instance_type: ResourceInstanceType::Duplicate,
            ..ResourceStatus::default()
        }),
        None,
    )
    .await
    .expect("second subject");

    let conflicts = scan(&svc).await;
    assert_eq!(conflicts.len(), 1, "one conflicting EHR, got {conflicts:?}");
    let reported = &conflicts[0];
    assert!(
        reported.starts_with("EhrWithMultipleSubjects"),
        "an EHR with several subjects is that state, got {reported}"
    );
    assert!(
        reported.contains(ehr.as_str()),
        "the report names the conflicting EHR, got {reported}"
    );
    assert!(
        reported.contains("PID-1") && reported.contains("PID-2"),
        "both subjects are carried, got {reported}"
    );
    assert!(
        reported.contains("Duplicate") && reported.contains("Primary"),
        "each association keeps its stored instance type, got {reported}"
    );

    // The SM read agrees, and fixes the order the scan's own query declares
    // (`ORDER BY subject_id, subject_namespace`).
    let entries = svc.ehr_subjects(ehr).await.expect("ehr subjects");
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].subject, primary);
    assert_eq!(entries[1].subject, duplicate);
}

/// An index in BOTH error states reports both, and the scan is advisory: it
/// never refuses, and it changes nothing — the associations read back exactly
/// as they were, and a second scan reports the same thing.
#[tokio::test]
async fn both_error_states_are_reported_together_and_the_scan_mutates_nothing() {
    let (_db, _pool, svc) = repository().await;

    let shared = SubjectRef::person("PID-SHARED", "mpi");
    let other = SubjectRef::person("PID-OTHER", "mpi");
    let first = associate(&svc, &shared).await;
    let second = associate(&svc, &shared).await;
    // …and the second EHR also carries a second subject.
    svc.add_ehr_subject(second.clone(), other.clone(), None, None)
        .await
        .expect("second subject");

    let before = svc.ehr_subjects(second.clone()).await.expect("read before");
    let conflicts = scan(&svc).await;
    assert_eq!(
        conflicts.len(),
        2,
        "both states reported, got {conflicts:?}"
    );
    assert!(
        conflicts
            .iter()
            .any(|c| c.starts_with("SubjectWithMultipleEhrs")
                && c.contains("PID-SHARED")
                && c.contains(first.as_str())
                && c.contains(second.as_str())),
        "the shared subject's multiple EHRs are reported, got {conflicts:?}"
    );
    assert!(
        conflicts
            .iter()
            .any(|c| c.starts_with("EhrWithMultipleSubjects")
                && c.contains(second.as_str())
                && c.contains("PID-OTHER")),
        "the second EHR's multiple subjects are reported, got {conflicts:?}"
    );

    let after = svc.ehr_subjects(second).await.expect("read after");
    assert_eq!(after, before, "detection is a read: it mutates nothing");
    assert_eq!(
        scan(&svc).await,
        conflicts,
        "a repeated scan reports the same states"
    );
}

/// Rectification through the SM write operations clears the report: the scan is
/// a live read over the index, never a stored flag. `remove_ehr_subject` (I4)
/// drops one association and `remove_subject` (I5) drops them all.
#[tokio::test]
async fn rectifying_through_the_sm_writes_clears_the_report() {
    let (_db, _pool, svc) = repository().await;

    let shared = SubjectRef::person("PID-SHARED", "mpi");
    let other = SubjectRef::person("PID-OTHER", "mpi");
    let first = associate(&svc, &shared).await;
    let second = associate(&svc, &shared).await;
    svc.add_ehr_subject(second.clone(), other.clone(), None, None)
        .await
        .expect("second subject");
    assert_eq!(scan(&svc).await.len(), 2);

    // I4: drop the extra subject from the second EHR — that state clears, the
    // subject-with-multiple-EHRs one remains.
    svc.remove_ehr_subject(second, other)
        .await
        .expect("remove the duplicate subject");
    let conflicts = scan(&svc).await;
    assert_eq!(conflicts.len(), 1, "only one state left, got {conflicts:?}");
    assert!(conflicts[0].starts_with("SubjectWithMultipleEhrs"));

    // I4 again: with one association left, a subject is no longer duplicated.
    svc.remove_ehr_subject(first, shared.clone())
        .await
        .expect("remove the duplicate association");
    assert_eq!(
        scan(&svc).await,
        Vec::<String>::new(),
        "a rectified index reports nothing"
    );

    // I5: dropping the subject entirely leaves the index clean too.
    svc.remove_subject(shared).await.expect("remove subject");
    assert_eq!(scan(&svc).await, Vec::<String>::new());
}
