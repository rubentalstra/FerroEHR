//! End-to-end composition-validation tests against a real PostgreSQL 18
//! (testcontainers): a COMPOSITION committed via the ITS-REST create/update
//! path is validated against its operational template *before* persistence.
//!
//! Oracle + fixtures: the vendored Apache-2.0 openEHR SDK corpus — the IPS
//! operational template (`openehr-flat/tests/fixtures/sdk/ips.v0.opt`,
//! `template_id` "International Patient Summary") paired with its canonical-JSON
//! compositions (`openehr-its/tests/vendor/openehr_sdk/composition/…`):
//! `ips_canonical.json` (valid) and `ips_invalid.json` (out-of-range magnitudes
//! and coded values outside the value set). Same pairing the `openehr-flat`
//! validator's own corpus tests use (`openehr-flat/tests/validation.rs`).
//!
//! Spec: openEHR ITS-REST 1.0.3 —
//! `docs/specs/openehr/ITS-REST/specifications/responses/422_COMPOSITION.yaml`
//! ("content could be converted to a COMPOSITION, but there are semantic
//! validation errors, such as the underlying template is not known or is not
//! validating the supplied COMPOSITION" → `422`). CNF cross-check:
//! `docs/specs/openehr/CNF/docs/platform_test_schedule/master07-func_tc_ehr_composition.adoc`
//! (`create_composition-event_bad_opt` → 422).
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::doc_markdown)]

use serde_json::{Value, json};
use sqlx::{AssertSqlSafe, Connection, PgConnection, PgPool};
use testcontainers::runners::AsyncRunner;
use testcontainers::{ContainerAsync, ImageExt};
use testcontainers_modules::postgres::Postgres;

use ehrbase::db::{self, DbSettings};
use ehrbase::service::EhrbaseService;
use openehr_its::rest::generated::definition::DefinitionApi;
use openehr_its::rest::generated::ehr::EhrApi;
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

/// Read a workspace fixture relative to this crate's manifest dir.
fn fixture(rel: &str) -> String {
    let path = format!("{}/{rel}", env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {path}: {e}"))
}

fn composition(name: &str) -> Value {
    let rel = format!("../openehr-its/tests/vendor/openehr_sdk/composition/canonical_json/{name}");
    serde_json::from_str(&fixture(&rel)).unwrap_or_else(|e| panic!("parse {name}: {e}"))
}

const IPS_OPT: &str = "../openehr-flat/tests/fixtures/sdk/ips.v0.opt";

/// Count the persisted COMPOSITION versions (kind discriminator on `vo_version`).
async fn composition_versions(pool: &PgPool) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM vo_version WHERE kind = 'COMPOSITION'")
        .fetch_one(pool)
        .await
        .expect("count compositions")
}

#[tokio::test]
async fn composition_validation_gates_persistence() {
    let pg = Pg::start().await;
    let pool = pg.migrated_pool("validation").await;
    let svc = EhrbaseService::new(pool.clone());

    // Ingest the IPS operational template (the validation target).
    svc.definition_template_adl1_4_upload(params(json!({})), Value::String(fixture(IPS_OPT)))
        .await
        .expect("upload IPS OPT");

    let ehr = svc
        .ehr_create(params(json!({})), None)
        .await
        .expect("ehr_create");
    let ehr_id = ehr["ehr_id"]["value"].as_str().expect("ehr_id").to_owned();

    // ── valid composition → committed and retrievable ────────────────────────
    let created = svc
        .composition_create(
            params(json!({ "ehr_id": ehr_id })),
            composition("ips_canonical.json"),
        )
        .await
        .expect("valid composition accepted (201)");
    let ovid = created["uid"]["value"].as_str().expect("uid").to_owned();
    let vo_id = ovid.split("::").next().unwrap().to_owned();

    let fetched = svc
        .composition_get(params(json!({ "ehr_id": ehr_id, "uid_based_id": vo_id })))
        .await
        .expect("valid composition persisted");
    assert_eq!(
        fetched["uid"]["value"], ovid,
        "persisted composition round-trips"
    );
    assert_eq!(
        composition_versions(&pool).await,
        1,
        "exactly the one valid composition is stored"
    );

    // ── invalid composition → 422 with per-path violations, NOT persisted ─────
    let err = svc
        .composition_create(
            params(json!({ "ehr_id": ehr_id })),
            composition("ips_invalid.json"),
        )
        .await
        .expect_err("invalid composition rejected");
    match err {
        ApiError::ValidationFailed(violations) => {
            assert!(!violations.is_empty(), "422 body carries the violations");
            assert!(
                violations.iter().all(|v| !v.path.is_empty()),
                "every violation is keyed by an RM path: {violations:?}"
            );
        }
        other => panic!("expected ValidationFailed (422), got {other:?}"),
    }
    // Validation runs before the write transaction, so nothing was persisted.
    assert_eq!(
        composition_versions(&pool).await,
        1,
        "the rejected composition was not persisted"
    );

    // ── unknown template → 422 "template not known" ──────────────────────────
    let mut unknown = composition("ips_canonical.json");
    unknown["archetype_details"]["template_id"]["value"] =
        Value::String("no.such.template.v0".to_owned());
    let err = svc
        .composition_create(params(json!({ "ehr_id": ehr_id })), unknown)
        .await
        .expect_err("unknown template rejected");
    match err {
        ApiError::Unprocessable(msg) => {
            assert!(
                msg.contains("not known"),
                "422 message names the cause: {msg}"
            );
        }
        other => panic!("expected Unprocessable (422) for unknown template, got {other:?}"),
    }
    assert_eq!(
        composition_versions(&pool).await,
        1,
        "the unknown-template composition was not persisted"
    );
}

#[tokio::test]
async fn composition_update_is_validated() {
    let pg = Pg::start().await;
    let pool = pg.migrated_pool("validation_update").await;
    let svc = EhrbaseService::new(pool.clone());

    svc.definition_template_adl1_4_upload(params(json!({})), Value::String(fixture(IPS_OPT)))
        .await
        .expect("upload IPS OPT");
    let ehr = svc
        .ehr_create(params(json!({})), None)
        .await
        .expect("ehr_create");
    let ehr_id = ehr["ehr_id"]["value"].as_str().expect("ehr_id").to_owned();

    // Seed a valid v1.
    let v1 = svc
        .composition_create(
            params(json!({ "ehr_id": ehr_id })),
            composition("ips_canonical.json"),
        )
        .await
        .expect("valid v1");
    let ovid_v1 = v1["uid"]["value"].as_str().expect("uid").to_owned();
    let vo_id = ovid_v1.split("::").next().unwrap().to_owned();

    // An update whose body fails template validation is rejected (422) and the
    // stored current version stays at v1.
    let err = svc
        .composition_update(
            params(json!({ "ehr_id": ehr_id, "uid_based_id": vo_id, "If-Match": ovid_v1 })),
            composition("ips_invalid.json"),
        )
        .await
        .expect_err("invalid update rejected");
    assert!(
        matches!(err, ApiError::ValidationFailed(ref v) if !v.is_empty()),
        "expected ValidationFailed (422), got {err:?}"
    );

    let current = svc
        .composition_get(params(json!({ "ehr_id": ehr_id, "uid_based_id": vo_id })))
        .await
        .expect("current still readable");
    assert_eq!(
        current["uid"]["value"], ovid_v1,
        "the rejected update did not advance the version"
    );
}
