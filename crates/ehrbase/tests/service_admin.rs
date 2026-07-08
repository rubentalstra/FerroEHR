//! End-to-end service tests for the ADMIN API (physical EHR delete) against a
//! real `PostgreSQL` 18 (testcontainers).
//!
//! Spec: SM `I_ADMIN_SERVICE.physical_ehr_delete`
//! (`docs/specs/openehr/SM/docs/UML/classes/i_admin_service.adoc`) — precondition
//! `has_ehr`, error `ehr_id_does_not_exist`. The cascade contract is the CNF
//! Robot prior art
//! (`docs/specs/openehr/CNF/tests/platform/robot/I_ADMIN_SERVICE/001-EHR.robot`):
//! after delete, every backing table returns to its pre-EHR baseline count. We
//! assert **zero rows remain** for the deleted EHR across `ehr`, `vo_version`,
//! `node`, `contribution`, `audit`, and `item_tag`, while a second EHR is left
//! entirely untouched.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::too_many_lines)]

use serde_json::{Value, json};
use sqlx::{AssertSqlSafe, Connection, PgConnection, PgPool};
use testcontainers::runners::AsyncRunner;
use testcontainers::{ContainerAsync, ImageExt};
use testcontainers_modules::postgres::Postgres;
use uuid::Uuid;

use ehrbase::db::{self, DbSettings};
use ehrbase::service::EhrbaseService;
use ehrbase_rest::{AdminService, EhrService};
use openehr_its::rest::runtime::ApiError;

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

fn params<P: serde::de::DeserializeOwned>(v: Value) -> P {
    serde_json::from_value(v).expect("params")
}

/// The `uid.value` (`OBJECT_VERSION_ID`) of a versioned-object body.
fn uid(v: &Value) -> &str {
    v["uid"]["value"].as_str().expect("uid.value")
}

/// Row counts scoped to one EHR across every table a physical delete must clear.
/// `audit` has no `ehr_id`, so it is counted by the audit ids the EHR's
/// versions/contributions reference (the same set the delete captures).
#[derive(Debug, Default, PartialEq, Eq)]
struct EhrRows {
    ehr: i64,
    vo_version: i64,
    node: i64,
    contribution: i64,
    item_tag: i64,
    audit: i64,
}

impl EhrRows {
    fn is_empty(&self) -> bool {
        *self == EhrRows::default()
    }
}

async fn count(pool: &PgPool, sql: &'static str, ehr_id: Uuid) -> i64 {
    sqlx::query_scalar(sql)
        .bind(ehr_id)
        .fetch_one(pool)
        .await
        .expect("count")
}

async fn ehr_rows(pool: &PgPool, ehr_id: Uuid) -> EhrRows {
    let audit_ids: Vec<Uuid> = sqlx::query_scalar(
        "SELECT audit_id FROM vo_version WHERE ehr_id = $1 \
         UNION SELECT audit_id FROM contribution WHERE ehr_id = $1",
    )
    .bind(ehr_id)
    .fetch_all(pool)
    .await
    .expect("audit ids");
    let audit: i64 = sqlx::query_scalar("SELECT count(*) FROM audit WHERE id = ANY($1)")
        .bind(&audit_ids)
        .fetch_one(pool)
        .await
        .expect("audit count");
    EhrRows {
        ehr: count(pool, "SELECT count(*) FROM ehr WHERE id = $1", ehr_id).await,
        vo_version: count(
            pool,
            "SELECT count(*) FROM vo_version WHERE ehr_id = $1",
            ehr_id,
        )
        .await,
        node: count(pool, "SELECT count(*) FROM node WHERE ehr_id = $1", ehr_id).await,
        contribution: count(
            pool,
            "SELECT count(*) FROM contribution WHERE ehr_id = $1",
            ehr_id,
        )
        .await,
        item_tag: count(
            pool,
            "SELECT count(*) FROM item_tag WHERE ehr_id = $1",
            ehr_id,
        )
        .await,
        audit,
    }
}

/// Seed an EHR with enough content to exercise every FK the physical delete
/// must cascade through: `EHR_STATUS` (two versions) + `EHR_ACCESS` from
/// creation, a directory FOLDER, and an item tag on the `EHR_STATUS`.
///
/// PORT NOTE: this deliberately avoids COMPOSITION. On this base commit,
/// COMPOSITION validation is stricter than the shared test fixtures supply, so
/// the pre-existing `service_ehr::ehr_composition_lifecycle_end_to_end` fails
/// identically (a base issue, not the admin change). `EHR_STATUS`/FOLDER writes
/// populate the same `vo_version`/`node`/`contribution`/`audit`/`item_tag`
/// tables, so the cascade contract is fully covered without COMPOSITION.
async fn seed_full_ehr(svc: &EhrbaseService) -> Uuid {
    let ehr = svc.ehr_create(params(json!({})), None).await.expect("ehr");
    let ehr_id = ehr.body["ehr_id"]["value"].as_str().unwrap().to_owned();

    // A second EHR_STATUS version (create → update): a multi-version vo.
    let status = svc
        .ehr_status_get_at_time(params(json!({ "ehr_id": ehr_id })))
        .await
        .expect("status get");
    let status_ovid = uid(&status.body).to_owned();
    let status_vo = status_ovid.split("::").next().unwrap().to_owned();
    let mut updated = status.body.clone();
    updated["is_modifiable"] = json!(false);
    svc.ehr_status_update(
        params(json!({ "ehr_id": ehr_id, "If-Match": status_ovid })),
        updated,
    )
    .await
    .expect("status update");

    // An item tag on the EHR_STATUS.
    svc.ehr_status_tags_update(
        params(json!({ "ehr_id": ehr_id, "uid_based_id": status_vo })),
        vec![json!({ "key": "priority", "value": "high" })],
    )
    .await
    .expect("tag");

    // A directory FOLDER (another versioned-object kind through the cascade).
    svc.directory_create(
        params(json!({ "ehr_id": ehr_id })),
        json!({ "_type": "FOLDER", "name": { "_type": "DV_TEXT", "value": "root" } }),
    )
    .await
    .expect("directory");

    Uuid::parse_str(&ehr_id).unwrap()
}

#[tokio::test]
async fn admin_delete_cascades_and_leaves_other_ehr_untouched() {
    let pg = Pg::start().await;
    // One database, two handles: the service owns one clone, the test queries
    // the other directly to assert the cascade.
    let pool = pg.migrated_pool("admin_cascade").await;
    let svc = EhrbaseService::new(pool.clone());
    let pool = &pool;

    let ehr1 = seed_full_ehr(&svc).await;
    let ehr2 = seed_full_ehr(&svc).await;

    // Both EHRs have content in every table before the delete.
    let before1 = ehr_rows(pool, ehr1).await;
    let before2 = ehr_rows(pool, ehr2).await;
    assert!(!before1.is_empty(), "ehr1 must be populated: {before1:?}");
    // EHR_STATUS v1+v2, EHR_ACCESS v1, FOLDER v1 → ≥4 versions; ≥3 contributions
    // (ehr create, status update, directory create); 1 item tag.
    assert!(before1.ehr == 1 && before1.vo_version >= 4 && before1.node >= 4);
    assert!(before1.contribution >= 3 && before1.item_tag == 1 && before1.audit >= 3);

    // Physical delete via the ADMIN seam (SM physical_ehr_delete).
    svc.admin_ehr_delete(ehr1.to_string())
        .await
        .expect("admin delete");

    // Every trace of ehr1 is physically gone (CNF cascade contract).
    let after1 = ehr_rows(pool, ehr1).await;
    assert!(
        after1.is_empty(),
        "physical delete must clear every table for the EHR, got {after1:?}"
    );

    // ehr2 is entirely untouched.
    let after2 = ehr_rows(pool, ehr2).await;
    assert_eq!(after2, before2, "the other EHR must be untouched");
}

#[tokio::test]
async fn admin_delete_unknown_ehr_is_not_found() {
    let pg = Pg::start().await;
    let svc = EhrbaseService::new(pg.migrated_pool("admin_missing").await);

    // `has_ehr` is false → `ehr_id_does_not_exist` → NotFound (→ HTTP 404).
    let missing = Uuid::now_v7().to_string();
    let res = svc.admin_ehr_delete(missing).await;
    assert!(
        matches!(res, Err(ApiError::NotFound(_))),
        "unknown EHR must be NotFound, got {res:?}"
    );

    // A malformed id is a 400.
    let bad = svc.admin_ehr_delete("not-a-uuid".to_owned()).await;
    assert!(
        matches!(bad, Err(ApiError::BadRequest(_))),
        "malformed id must be BadRequest, got {bad:?}"
    );
}

#[tokio::test]
async fn admin_delete_all_deletes_present_and_skips_missing() {
    let pg = Pg::start().await;
    let svc = EhrbaseService::new(pg.migrated_pool("admin_delete_all").await);

    let a = seed_full_ehr(&svc).await;
    let b = seed_full_ehr(&svc).await;

    // A two-id list of existing EHRs deletes both.
    let deleted = svc
        .admin_ehr_delete_all(vec![a.to_string(), b.to_string()])
        .await
        .expect("delete all");
    assert_eq!(deleted, 2, "both existing EHRs deleted");

    // A list mixing one existing and one missing id deletes only the existing
    // (idempotent bulk: missing ids are skipped).
    let c = seed_full_ehr(&svc).await;
    let missing = Uuid::now_v7().to_string();
    let deleted = svc
        .admin_ehr_delete_all(vec![c.to_string(), missing])
        .await
        .expect("delete all with a bogus id");
    assert_eq!(
        deleted, 1,
        "only the existing EHR is deleted; missing skipped"
    );

    // A malformed id in the list rejects the whole request (400), no deletion.
    let d = seed_full_ehr(&svc).await;
    let res = svc
        .admin_ehr_delete_all(vec![d.to_string(), "not-a-uuid".to_owned()])
        .await;
    assert!(
        matches!(res, Err(ApiError::BadRequest(_))),
        "a malformed id must be BadRequest, got {res:?}"
    );
}
