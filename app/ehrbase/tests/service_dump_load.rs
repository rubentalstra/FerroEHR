//! End-to-end service tests for the ADMIN dump/load API against a real
//! `PostgreSQL` 18 (testcontainers).
//!
//! Spec: SM `I_ADMIN_DUMP_LOAD.export_ehrs`/`load_ehrs`
//! (`docs/specs/openehr/SM/docs/UML/classes/i_admin_dump_load.adoc`), with
//! `EXPORT_SPEC` (`export_spec.adoc`, `segment_split_size` kb) and
//! `DUMP_LOAD_FAIL_REPORT` (`dump_load_fail_report.adoc`). The two acceptance
//! properties:
//!
//! 1. **Round-trip fidelity** — export N EHRs from a source repository, load the
//!    archive into a *fresh* database, and every EHR's content reads back
//!    byte-equal at the canonical JSON level (`EHR_STATUS` + directory), with the
//!    same version/row counts.
//! 2. **Duplicate-id failure path** — loading an EHR whose id already exists is
//!    recorded in a `DUMP_LOAD_FAIL_REPORT` (`dump_status = false`) and skipped,
//!    never a crash, and leaves the repository unchanged.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::too_many_lines)]

use serde_json::{Value, json};
use sqlx::{AssertSqlSafe, Connection, PgConnection, PgPool};
use testcontainers::runners::AsyncRunner;
use testcontainers::{ContainerAsync, ImageExt};
use testcontainers_modules::postgres::Postgres;
use uuid::Uuid;

use openehr_base::prelude::TerminologyCode;
use openehr_rm::prelude::PartyProxy;

use ehrbase::db::{self, DbSettings};
use ehrbase::service::EhrbaseService;
use ehrbase_sm::types::{UpdateAudit, UpdateVersion};
use ehrbase_sm::{
    AdminDumpLoad, EhrDirectoryService, EhrService, EhrStatusService, ExportSpec, ItemTagAdapter,
};

struct Pg {
    #[allow(dead_code)]
    container: ContainerAsync<Postgres>,
    host: String,
    port: u16,
}

impl Pg {
    async fn start() -> Self {
        let container = Postgres::default()
            .with_tag("18")
            .start()
            .await
            .expect("start postgres:18 (is Docker running?)");
        let host = container.get_host().await.expect("host").to_string();
        let port = container.get_host_port_ipv4(5432).await.expect("port");
        Self {
            container,
            host,
            port,
        }
    }

    async fn migrated_pool(&self, name: &str) -> PgPool {
        let admin = format!(
            "postgres://postgres:postgres@{}:{}/postgres",
            self.host, self.port
        );
        let mut conn = PgConnection::connect(&admin).await.expect("admin connect");
        sqlx::raw_sql(AssertSqlSafe(format!("CREATE DATABASE {name}")))
            .execute(&mut conn)
            .await
            .expect("create db");
        let settings = DbSettings::new(format!(
            "postgres://postgres:postgres@{}:{}/{name}",
            self.host, self.port
        ));
        let pool = db::connect(&settings).await.expect("pool");
        db::run_migrations(&pool).await.expect("migrate");
        pool
    }
}

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
            committer: serde_json::from_value::<PartyProxy>(
                json!({ "_type": "PARTY_IDENTIFIED", "name": "conformance tester" }),
            )
            .expect("committer"),
        },
        signature: None,
    }
}

/// Seed an EHR carrying multiple versioned-object kinds and versions: `EHR_STATUS`
/// (create → update = two versions), an item tag on the `EHR_STATUS`, and a
/// directory `FOLDER` — the same fixture shape as `service_admin.rs` (avoids
/// `COMPOSITION`, which needs a template the shared fixtures do not supply, so the
/// dump/load path is exercised without a `template_store` dependency).
async fn seed_full_ehr(svc: &EhrbaseService) -> Uuid {
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
            json!({ "_type": "FOLDER", "name": { "_type": "DV_TEXT", "value": "root" } }),
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
async fn canonical_snapshot(svc: &EhrbaseService, ehr_id: Uuid) -> (String, String) {
    let status = svc
        .get_ehr_status_at_time(ehr_id, None)
        .await
        .expect("status read");
    let directory = svc
        .get_directory_at_time(ehr_id, None, None)
        .await
        .expect("directory read");
    (
        serde_json::to_string(&status).expect("status json"),
        serde_json::to_string(&directory).expect("directory json"),
    )
}

#[tokio::test]
async fn export_then_load_into_fresh_db_round_trips_byte_equal() {
    let pg = Pg::start().await;
    let src_pool = pg.migrated_pool("dumpload_src").await;
    let dst_pool = pg.migrated_pool("dumpload_dst").await;
    let source = EhrbaseService::new(src_pool.clone());
    let target = EhrbaseService::new(dst_pool.clone());

    // Seed two EHRs in the source repository.
    let ehr1 = seed_full_ehr(&source).await;
    let ehr2 = seed_full_ehr(&source).await;

    // Snapshot the source canonical content before export.
    let src1 = canonical_snapshot(&source, ehr1).await;
    let src2 = canonical_snapshot(&source, ehr2).await;
    let src_counts1 = counts(&src_pool, ehr1).await;
    let src_counts2 = counts(&src_pool, ehr2).await;

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
    assert_eq!(counts(&dst_pool, ehr1).await, src_counts1);
    assert_eq!(counts(&dst_pool, ehr2).await, src_counts2);

    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn load_duplicate_ehr_ids_is_reported_not_fatal() {
    let pg = Pg::start().await;
    let source = EhrbaseService::new(pg.migrated_pool("dumpload_dup_src").await);
    let dst_pool = pg.migrated_pool("dumpload_dup_dst").await;
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
    let after_first = counts(&dst_pool, ehr1).await;

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
        counts(&dst_pool, ehr1).await,
        after_first,
        "a duplicate re-load must not add or remove rows"
    );

    let _ = std::fs::remove_dir_all(&dir);
}
