#![allow(
    clippy::panic,
    clippy::print_stdout,
    clippy::print_stderr,
    let_underscore_drop
)] // test assertions/diagnostics/fixtures
//! End-to-end service tests for the ADMIN dump/load API against a real
//! `PostgreSQL` 18 (shared testkit harness).
//!
//! Spec: SM `I_ADMIN_DUMP_LOAD.export_ehrs`/`load_ehrs`
//! (`docs/specs/openehr/SM/docs/UML/classes/i_admin_dump_load.adoc`), with
//! `EXPORT_SPEC` (`export_spec.adoc`, `segment_split_size` kb) and
//! `DUMP_LOAD_FAIL_REPORT` (`dump_load_fail_report.adoc`). The acceptance
//! properties:
//!
//! 1. **Round-trip fidelity** — export N EHRs from a source repository, load the
//!    archive into a *fresh* database, and every EHR's content reads back
//!    byte-equal at the canonical JSON level (`EHR_STATUS` + directory), with the
//!    same version/row counts.
//! 2. **Duplicate-id failure path** — loading an EHR whose id already exists is
//!    recorded in a `DUMP_LOAD_FAIL_REPORT` (`dump_status = false`) and skipped,
//!    never a crash, and leaves the repository unchanged.
//! 3. **Promoted-column re-derivation** — the `ehr.subject_*` columns are a
//!    projection of `EHR_STATUS.subject` (RM ehr master04 §EHR Status), so the
//!    load re-derives them from the loaded status: a loaded EHR is found by the
//!    subject lookup (SM `I_EHR_SERVICE.get_ehrs_for_subject`,
//!    `operations/ehr_get_by_subject.yaml`) even when the archive's projection
//!    was stale, and a PARTIAL load carrying a subject this repository already
//!    holds is reported like a duplicate id, never silently duplicated
//!    (one EHR per subject).
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::too_many_lines)]

use serde_json::{Value, json};
use sqlx::PgPool;
use uuid::Uuid;

use openehr_base::prelude::TerminologyCode;
use openehr_rm::prelude::PartyProxy;

use ehrbase::service::EhrbaseService;

use ehrbase::service::admin::types::{CompressionFormat, ExportFormat, ExportSpec};
use ehrbase::service::ehr_index::types::SubjectRef;
use ehrbase::service::status::CallStatusType;
use ehrbase::service::version_update::{UpdateAudit, UpdateVersion};

/// A unique temporary directory path for one archive (best-effort cleaned up by
/// the OS temp dir; the round-trip does not depend on removal).
fn archive_dir() -> String {
    std::env::temp_dir()
        .join(format!("ehrbase-dumpload-{}", Uuid::now_v7()))
        .to_string_lossy()
        .into_owned()
}

fn uid(v: &Value) -> &str {
    v["uid"]["value"].as_str().expect("uid.value")
}

fn term(code: &str) -> TerminologyCode {
    TerminologyCode {
        terminology_id: "openehr".to_owned(),
        terminology_version: None,
        code_string: code.to_owned(),
        uri: None,
    }
}

fn uv(data: Value, change_code: &str, preceding: Option<&str>) -> UpdateVersion {
    UpdateVersion {
        preceding_version_uid: preceding.map(|p| p.parse().expect("OBJECT_VERSION_ID")),
        lifecycle_state: term("532"),
        attestations: None,
        data,
        audit: UpdateAudit {
            change_type: term(change_code),
            description: None,
            committer: openehr_its::json::from_canonical_value::<PartyProxy>(
                &json!({ "_type": "PARTY_IDENTIFIED", "name": "conformance tester" }),
            )
            .expect("committer"),
            system_id: None,
        },
        signature: None,
    }
}

/// Seed an EHR carrying multiple versioned-object kinds and versions: `EHR_STATUS`
/// (create → update = two versions), an item tag on the `EHR_STATUS`, and a
/// directory `FOLDER` — the same fixture shape as `service_admin.rs` (avoids
/// `COMPOSITION`, which needs a template the shared fixtures do not supply, so the
/// dump/load path is exercised without a `template_store` dependency).
async fn seed_full_ehr(svc: &EhrbaseService) -> ehrbase::ids::EhrId {
    let ehr_uuid = svc.create_ehr(None).await.expect("ehr");

    let mut updated = svc
        .get_ehr_status_at_time(ehr_uuid, None)
        .await
        .expect("status get");
    let status_ovid = uid(&updated).to_owned();
    let status_vo = status_ovid.split("::").next().unwrap().to_owned();
    updated.as_object_mut().expect("status obj").remove("uid");
    svc.target_tags_replace(
        ehr_uuid,
        status_vo,
        "EHR_STATUS",
        vec![json!({ "key": "priority", "value": "high" })],
    )
    .await
    .expect("tag");

    svc.create_directory(
        ehr_uuid,
        uv(
            json!({ "_type": "FOLDER", "archetype_node_id": "openEHR-EHR-FOLDER.generic.v1", "name": { "_type": "DV_TEXT", "value": "root" } }),
            "249",
            None,
        ),
    )
    .await
    .expect("directory");

    updated["is_modifiable"] = json!(false);
    svc.replace_ehr_status(ehr_uuid, uv(updated, "251", Some(&status_ovid)))
        .await
        .expect("status update");

    ehr_uuid
}

/// An `EHR_STATUS` whose `PARTY_SELF` subject carries an `external_ref` — RM
/// ehr master04 §EHR Status (the subject 0..1 identifies the EHR).
fn status_for_subject(subject_id: &str) -> Value {
    json!({
        "_type": "EHR_STATUS",
        "archetype_node_id": "openEHR-EHR-EHR_STATUS.generic.v1",
        "archetype_details": {
            "_type": "ARCHETYPED",
            "archetype_id": { "_type": "ARCHETYPE_ID",
                              "value": "openEHR-EHR-EHR_STATUS.generic.v1" },
            "rm_version": "1.2.0"
        },
        "name": { "_type": "DV_TEXT", "value": "EHR Status" },
        "subject": {
            "_type": "PARTY_SELF",
            "external_ref": {
                "_type": "PARTY_REF",
                "namespace": "patients",
                "type": "PERSON",
                "id": { "_type": "HIER_OBJECT_ID", "value": subject_id }
            }
        },
        "is_queryable": true,
        "is_modifiable": true
    })
}

/// The per-EHR row counts across the versioned-object tables (used to prove the
/// load reproduced the same storage shape).
async fn counts(pool: &PgPool, ehr_id: Uuid) -> (i64, i64, i64, i64) {
    let one = |sql: &'static str| async move {
        sqlx::query_scalar::<_, i64>(sql)
            .bind(ehr_id)
            .fetch_one(pool)
            .await
            .expect("count")
    };
    (
        one("SELECT count(*) FROM vo_version WHERE ehr_id = $1").await,
        one("SELECT count(*) FROM node WHERE ehr_id = $1").await,
        one("SELECT count(*) FROM contribution WHERE ehr_id = $1").await,
        one("SELECT count(*) FROM item_tag WHERE ehr_id = $1").await,
    )
}

/// Read the current `EHR_STATUS` and directory `FOLDER` of `ehr_id` as canonical
/// JSON, serialized in storage (jsonb-normalized) key order — the byte-equal
/// comparison surface.
async fn canonical_snapshot(svc: &EhrbaseService, ehr_id: ehrbase::ids::EhrId) -> (String, String) {
    let status = svc
        .get_ehr_status_at_time(ehr_id, None)
        .await
        .expect("status read");
    let directory = svc
        .get_directory_at_time(ehr_id, None, None)
        .await
        .expect("directory read")
        .body;
    (
        serde_json::to_string(&status).expect("status json"),
        serde_json::to_string(&directory).expect("directory json"),
    )
}

#[tokio::test]
async fn export_then_load_into_fresh_db_round_trips_byte_equal() {
    let src_db = testkit::db().await.expect("testkit database");
    let src_pool = src_db.pool();
    let dst_db = testkit::db().await.expect("testkit database");
    let dst_pool = dst_db.pool();
    let source = EhrbaseService::new(src_pool.clone());
    let target = EhrbaseService::new(dst_pool.clone());

    // Seed two EHRs in the source repository.
    let ehr1 = seed_full_ehr(&source).await;
    let ehr2 = seed_full_ehr(&source).await;

    // Snapshot the source canonical content before export.
    let src1 = canonical_snapshot(&source, ehr1).await;
    let src2 = canonical_snapshot(&source, ehr2).await;
    let src_counts1 = counts(&src_pool, ehr1.into()).await;
    let src_counts2 = counts(&src_pool, ehr2.into()).await;

    // Export to a canonical-JSON archive (small segment size forces multiple
    // segment files, exercising the segmenting path).
    let dir = archive_dir();
    let export_reports = source
        .export_ehrs(dir.clone(), ExportSpec::canonical_json(1))
        .await
        .expect("export");
    assert!(
        export_reports.is_empty(),
        "a clean export reports no failures, got {export_reports:?}"
    );
    // The archive exists on disk (manifest + at least one segment).
    assert!(
        std::path::Path::new(&dir).join("manifest.json").exists(),
        "manifest written"
    );

    // Load into the fresh target repository.
    let load_reports = target.load_ehrs(dir.clone()).await.expect("load");
    assert!(
        load_reports.is_empty(),
        "loading into an empty repo reports no failures, got {load_reports:?}"
    );

    // Both EHRs read back byte-equal at the canonical JSON level.
    assert_eq!(
        canonical_snapshot(&target, ehr1).await,
        src1,
        "ehr1 content must round-trip byte-equal"
    );
    assert_eq!(
        canonical_snapshot(&target, ehr2).await,
        src2,
        "ehr2 content must round-trip byte-equal"
    );

    // And the storage shape (version/node/contribution/tag counts) matches.
    assert_eq!(counts(&dst_pool, ehr1.into()).await, src_counts1);
    assert_eq!(counts(&dst_pool, ehr2.into()).await, src_counts2);

    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn load_duplicate_ehr_ids_is_reported_not_fatal() {
    let src_db = testkit::db().await.expect("testkit database");
    let source = EhrbaseService::new(src_db.pool());
    let dst_db = testkit::db().await.expect("testkit database");
    let dst_pool = dst_db.pool();
    let target = EhrbaseService::new(dst_pool.clone());

    let ehr1 = seed_full_ehr(&source).await;
    let ehr2 = seed_full_ehr(&source).await;

    let dir = archive_dir();
    source
        .export_ehrs(dir.clone(), ExportSpec::canonical_json(1024))
        .await
        .expect("export");

    // First load into the empty target: both EHRs land, no failures.
    let first = target.load_ehrs(dir.clone()).await.expect("first load");
    assert!(first.is_empty(), "first load is clean, got {first:?}");
    let after_first = counts(&dst_pool, ehr1.into()).await;

    // Second load of the same archive: every EHR id now already exists, so each
    // is recorded in a DUMP_LOAD_FAIL_REPORT (dump_status = false) and skipped —
    // no crash, no duplicate rows.
    let second = target.load_ehrs(dir.clone()).await.expect("second load");
    assert_eq!(second.len(), 2, "both EHRs reported as duplicates");
    for report in &second {
        assert_eq!(report.entity_type, "EHR");
        assert!(
            !report.dump_status,
            "a duplicate load fails for that entity"
        );
        assert!(report.error.is_some(), "with an explanatory error");
    }
    let reported: std::collections::BTreeSet<String> =
        second.iter().map(|r| r.entity_id.clone()).collect();
    assert!(reported.contains(&ehr1.to_string()));
    assert!(reported.contains(&ehr2.to_string()));

    // The repository is unchanged by the failed re-load (idempotent skip).
    assert_eq!(
        counts(&dst_pool, ehr1.into()).await,
        after_first,
        "a duplicate re-load must not add or remove rows"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn load_promotes_the_subject_from_the_loaded_status() {
    // The promoted `ehr.subject_*` columns are only a projection of
    // `EHR_STATUS.subject` (RM ehr master04 §EHR Status), so the load re-derives
    // them from the loaded status instead of trusting the archived projection:
    // an archive whose projection is absent — the shape an EHR landed by a path
    // that never promoted has — still yields an EHR the subject lookup finds
    // (SM `I_EHR_SERVICE.get_ehrs_for_subject`).
    let src_db = testkit::db().await.expect("testkit database");
    let src_pool = src_db.pool();
    let source = EhrbaseService::new(src_pool.clone());
    let dst_db = testkit::db().await.expect("testkit database");
    let target = EhrbaseService::new(dst_db.pool());

    let ehr = source
        .create_ehr(Some(status_for_subject("patient-dump-1")))
        .await
        .expect("source EHR for the subject");

    // Fixture: clear the promoted projection, leaving the subject only in the
    // EHR_STATUS content — what an EHR landed without promotion looks like.
    sqlx::query("UPDATE ehr SET subject_id = NULL, subject_namespace = NULL WHERE id = $1")
        .bind(Uuid::from(ehr))
        .execute(&src_pool)
        .await
        .expect("clear the promoted subject columns");
    assert!(
        source
            .get_ehrs_for_subject(SubjectRef::person("patient-dump-1", "patients"))
            .await
            .expect("source lookup")
            .is_empty(),
        "fixture: the source no longer finds the EHR by subject"
    );

    let dir = archive_dir();
    source
        .export_ehrs(dir.clone(), ExportSpec::canonical_json(1024))
        .await
        .expect("export");
    let reports = target.load_ehrs(dir.clone()).await.expect("load");
    assert!(reports.is_empty(), "clean load, got {reports:?}");

    let found = target
        .get_ehrs_for_subject(SubjectRef::person("patient-dump-1", "patients"))
        .await
        .expect("target lookup");
    assert_eq!(
        found.len(),
        1,
        "the loaded EHR must be found by the subject its EHR_STATUS names"
    );
    assert_eq!(found[0].ehr_id, ehr.to_string());

    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn load_reports_an_ehr_whose_subject_the_repository_already_holds() {
    // A dump of a self-consistent repository cannot clash with itself, but a
    // PARTIAL load into a NON-EMPTY repository can: one EHR per subject (RM ehr
    // master04 §EHR Status). The clash is reported per record
    // (`DUMP_LOAD_FAIL_REPORT`, `dump_status = false`) and skipped exactly like
    // a duplicate EHR id — never fatal, never a silent duplicate — and the rest
    // of the archive still loads.
    let src_db = testkit::db().await.expect("testkit database");
    let source = EhrbaseService::new(src_db.pool());
    let dst_db = testkit::db().await.expect("testkit database");
    let target = EhrbaseService::new(dst_db.pool());

    let clashing = source
        .create_ehr(Some(status_for_subject("patient-dump-2")))
        .await
        .expect("source EHR for the subject");
    let innocent = seed_full_ehr(&source).await;

    // The target already holds that subject under a DIFFERENT EHR.
    let owner = target
        .create_ehr(Some(status_for_subject("patient-dump-2")))
        .await
        .expect("target EHR for the same subject");

    let dir = archive_dir();
    source
        .export_ehrs(dir.clone(), ExportSpec::canonical_json(1024))
        .await
        .expect("export");
    let reports = target.load_ehrs(dir.clone()).await.expect("load");

    assert_eq!(
        reports.len(),
        1,
        "only the subject clash fails, got {reports:?}"
    );
    assert_eq!(reports[0].entity_type, "EHR");
    assert_eq!(reports[0].entity_id, clashing.to_string());
    assert!(!reports[0].dump_status);
    let error = reports[0].error.as_deref().expect("an explanatory error");
    assert!(
        error.contains("patient-dump-2"),
        "the report must name the clashing subject, got: {error}"
    );

    // The clashing record was skipped whole; everything else loaded.
    assert!(
        !target.has_ehr(clashing).await.expect("has_ehr"),
        "a reported record must not be partially loaded"
    );
    assert!(
        target.has_ehr(innocent).await.expect("has_ehr"),
        "the rest of the archive still loads"
    );
    let found = target
        .get_ehrs_for_subject(SubjectRef::person("patient-dump-2", "patients"))
        .await
        .expect("target lookup");
    assert_eq!(found.len(), 1, "the subject still resolves to its holder");
    assert_eq!(found[0].ehr_id, owner.to_string());

    let _ = std::fs::remove_dir_all(&dir);
}

/// SM `compression_format.adoc` member `zip`: the archive is one `archive.zip`
/// container carrying the identical entry set, and `load_ehrs` — which is
/// passed no format (`i_admin_dump_load.adoc`: `load_ehrs(file_sys_loc)`) —
/// detects and reads it, round-tripping byte-equal.
#[tokio::test]
async fn zip_compressed_export_round_trips_through_the_detected_container() {
    let src_db = testkit::db().await.expect("testkit database");
    let src_pool = src_db.pool();
    let source = EhrbaseService::new(src_pool.clone());
    let dst_db = testkit::db().await.expect("testkit database");
    let dst_pool = dst_db.pool();
    let target = EhrbaseService::new(dst_pool.clone());

    let ehr = seed_full_ehr(&source).await;
    let src = canonical_snapshot(&source, ehr).await;
    let src_counts = counts(&src_pool, ehr.into()).await;

    let dir = archive_dir();
    let spec = ExportSpec {
        logical_format: Some(ExportFormat::OpenehrCanonicalJson),
        compression_format: Some(CompressionFormat::Zip),
        // A small split forces several segment entries into the one container.
        segment_split_size: 1,
    };
    let reports = source
        .export_ehrs(dir.clone(), spec)
        .await
        .expect("zip export");
    assert!(reports.is_empty(), "a clean export reports no failures");

    // The packed container is the ONLY thing written — no loose manifest.
    let root = std::path::Path::new(&dir);
    assert!(root.join("archive.zip").is_file(), "archive.zip written");
    assert!(
        !root.join("manifest.json").exists(),
        "the packed form writes no loose manifest"
    );

    let load_reports = target.load_ehrs(dir.clone()).await.expect("zip load");
    assert!(
        load_reports.is_empty(),
        "loading into an empty repo reports no failures, got {load_reports:?}"
    );
    assert_eq!(
        canonical_snapshot(&target, ehr).await,
        src,
        "a zip round-trip must be byte-equal at the canonical JSON level"
    );
    assert_eq!(counts(&dst_pool, ehr.into()).await, src_counts);

    let _ = std::fs::remove_dir_all(&dir);
}

/// SM `compression_format.adoc` member `7z` (owner-approved 2026-07-29,
/// `sevenz-rust2`): the archive is one `archive.7z` container carrying the
/// identical entry set, and the format-less `load_ehrs` detects and reads it,
/// round-tripping byte-equal — the exact mirror of the `zip` sibling above.
#[tokio::test]
async fn sevenz_compressed_export_round_trips_through_the_detected_container() {
    let src_db = testkit::db().await.expect("testkit database");
    let src_pool = src_db.pool();
    let source = EhrbaseService::new(src_pool.clone());
    let dst_db = testkit::db().await.expect("testkit database");
    let dst_pool = dst_db.pool();
    let target = EhrbaseService::new(dst_pool.clone());

    let ehr = seed_full_ehr(&source).await;
    let src = canonical_snapshot(&source, ehr).await;
    let src_counts = counts(&src_pool, ehr.into()).await;

    let dir = archive_dir();
    let spec = ExportSpec {
        logical_format: Some(ExportFormat::OpenehrCanonicalJson),
        compression_format: Some(CompressionFormat::SevenZip),
        // A small split forces several segment entries into the one container.
        segment_split_size: 1,
    };
    let reports = source
        .export_ehrs(dir.clone(), spec)
        .await
        .expect("7z export");
    assert!(reports.is_empty(), "a clean export reports no failures");

    // The packed container is the ONLY thing written — no loose manifest.
    let root = std::path::Path::new(&dir);
    assert!(root.join("archive.7z").is_file(), "archive.7z written");
    assert!(
        !root.join("manifest.json").exists(),
        "the packed form writes no loose manifest"
    );

    let load_reports = target.load_ehrs(dir.clone()).await.expect("7z load");
    assert!(
        load_reports.is_empty(),
        "loading into an empty repo reports no failures, got {load_reports:?}"
    );
    assert_eq!(
        canonical_snapshot(&target, ehr).await,
        src,
        "a 7z round-trip must be byte-equal at the canonical JSON level"
    );
    assert_eq!(counts(&dst_pool, ehr.into()).await, src_counts);

    let _ = std::fs::remove_dir_all(&dir);
}

/// The one `EXPORT_FORMAT` member this service does not realize
/// (`openehr_canonical_xml` — the archive envelope has no XML design yet) is
/// refused as `not_implemented` (RFC 9110 §15.6.2 `501`) — a valid SM
/// enumeration member is never reported as a malformed request, and never
/// silently downgraded to a format the caller did not ask for.
/// (`COMPRESSION_FORMAT` is realized in full since the 7z approval —
/// the round-trip test below covers it.)
#[tokio::test]
async fn unrealized_format_members_are_not_implemented_and_write_nothing() {
    let db = testkit::db().await.expect("testkit database");
    let service = EhrbaseService::new(db.pool());
    let _ehr = seed_full_ehr(&service).await;

    for (label, spec) in [(
        "openehr_canonical_xml",
        ExportSpec {
            logical_format: Some(ExportFormat::OpenehrCanonicalXml),
            compression_format: None,
            segment_split_size: 1024,
        },
    )] {
        let dir = archive_dir();
        let err = service
            .export_ehrs(dir.clone(), spec)
            .await
            .expect_err("an unrealized member is refused");
        assert_eq!(
            err.status,
            CallStatusType::NotImplemented,
            "{label} must be not_implemented, got {err:?}"
        );
        let path = std::path::Path::new(&dir);
        let empty = !path.exists()
            || std::fs::read_dir(path)
                .expect("read the archive dir")
                .next()
                .is_none();
        assert!(empty, "{label} must leave no partial archive behind");
        let _ = std::fs::remove_dir_all(&dir);
    }
}

/// `EXPORT_SPEC.segment_split_size` is `Integer [1..1]` in kb
/// (`export_spec.adoc`); a non-positive size names no segment size at all and
/// is a `precondition_violation`, distinct from the unrealized-member branch.
#[tokio::test]
async fn non_positive_segment_split_size_is_a_precondition_violation() {
    let db = testkit::db().await.expect("testkit database");
    let service = EhrbaseService::new(db.pool());

    let dir = archive_dir();
    let err = service
        .export_ehrs(
            dir.clone(),
            ExportSpec {
                logical_format: None,
                compression_format: None,
                segment_split_size: 0,
            },
        )
        .await
        .expect_err("a non-positive split size is refused");
    assert_eq!(err.status, CallStatusType::PreconditionViolation);
    let _ = std::fs::remove_dir_all(&dir);
}

/// `load_ehrs` against a location holding neither container is the SM's own
/// `file_not_writable` (the only error `i_admin_dump_load.adoc` declares),
/// never a panic or a silent empty load.
#[tokio::test]
async fn load_from_a_location_with_no_archive_is_file_not_writable() {
    let db = testkit::db().await.expect("testkit database");
    let service = EhrbaseService::new(db.pool());

    let err = service
        .load_ehrs(archive_dir())
        .await
        .expect_err("no archive at the location");
    assert_eq!(err.status, CallStatusType::FileNotWritable);
}
