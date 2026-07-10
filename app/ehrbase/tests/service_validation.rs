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

use openehr_base::prelude::TerminologyCode;
use openehr_rm::prelude::PartyProxy;

use ehrbase::db::{self, DbSettings};
use ehrbase::service::EhrbaseService;
use ehrbase_sm::types::{UpdateAudit, UpdateVersion};
use ehrbase_sm::{CallStatusType, DefinitionAdapter, EhrCompositionService, EhrService, SmError};

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

/// An `openehr` terminology code (audit change type / lifecycle state).
fn term(code: &str) -> TerminologyCode {
    TerminologyCode {
        terminology_id: "openehr".to_owned(),
        terminology_version: None,
        code_string: code.to_owned(),
        uri: None,
    }
}

/// The SM `UPDATE_VERSION` commit envelope for a bare-RM write.
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

/// Read a workspace fixture relative to this crate's manifest dir.
fn fixture(rel: &str) -> String {
    let path = format!("{}/{rel}", env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {path}: {e}"))
}

fn composition(name: &str) -> Value {
    let rel = format!(
        "../../crates/openehr-its/tests/vendor/openehr_sdk/composition/canonical_json/{name}"
    );
    serde_json::from_str(&fixture(&rel)).unwrap_or_else(|e| panic!("parse {name}: {e}"))
}

const IPS_OPT: &str = "../../crates/openehr-flat/tests/fixtures/sdk/ips.v0.opt";

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
    svc.template_adl14_upload(fixture(IPS_OPT))
        .await
        .expect("upload IPS OPT");

    let ehr_id = svc.create_ehr(None).await.expect("create_ehr").to_string();
    let ehr_uuid = ehr_id.parse::<uuid::Uuid>().expect("ehr uuid");

    // ── valid composition → committed and retrievable ────────────────────────
    // PORT NOTE (ADR-011): `create_composition` returns the new version_uid.
    let ovid = svc
        .create_composition(ehr_uuid, uv(composition("ips_canonical.json"), "249", None))
        .await
        .expect("valid composition accepted (201)");
    let vo_id = ovid.split("::").next().unwrap().to_owned();

    let fetched = svc
        .get_composition_latest(ehr_uuid, vo_id.parse().expect("vo uuid"))
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
        .create_composition(ehr_uuid, uv(composition("ips_invalid.json"), "249", None))
        .await
        .expect_err("invalid composition rejected");
    // PORT NOTE (ADR-011): the SM boundary flattens `ServiceError::ValidationFailed`
    // to `SmError { status: ContentInvalid, message }`, the per-path violations
    // joined into `message` ("path: msg; …"). The structured per-path list is now
    // built at the protocol adapter's 422 body (`ehrbase-rest`); here we assert the
    // status + that the message carries the RM paths, preserving the intent.
    assert!(
        matches!(
            err,
            SmError {
                status: CallStatusType::ContentInvalid,
                ..
            }
        ),
        "expected content_invalid (422), got {err:?}"
    );
    assert!(
        err.message.contains('/'),
        "422 message carries the RM-path-keyed violations: {}",
        err.message
    );
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
        .create_composition(ehr_uuid, uv(unknown, "249", None))
        .await
        .expect_err("unknown template rejected");
    // PORT NOTE (ADR-011): `ServiceError::Unprocessable` → `ContentInvalid`; the
    // cause still rides in the message.
    assert!(
        matches!(
            err,
            SmError {
                status: CallStatusType::ContentInvalid,
                ..
            }
        ),
        "expected content_invalid (422) for unknown template, got {err:?}"
    );
    assert!(
        err.message.contains("not known"),
        "422 message names the cause: {}",
        err.message
    );
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

    svc.template_adl14_upload(fixture(IPS_OPT))
        .await
        .expect("upload IPS OPT");
    let ehr_id = svc.create_ehr(None).await.expect("create_ehr").to_string();
    let ehr_uuid = ehr_id.parse::<uuid::Uuid>().expect("ehr uuid");

    // Seed a valid v1.
    let ovid_v1 = svc
        .create_composition(ehr_uuid, uv(composition("ips_canonical.json"), "249", None))
        .await
        .expect("valid v1");
    let vo_id = ovid_v1.split("::").next().unwrap().to_owned();
    let vo_uuid = vo_id.parse::<uuid::Uuid>().expect("vo uuid");

    // An update whose body fails template validation is rejected (422) and the
    // stored current version stays at v1.
    let err = svc
        .update_composition(
            ehr_uuid,
            vo_uuid,
            uv(composition("ips_invalid.json"), "251", Some(&ovid_v1)),
        )
        .await
        .expect_err("invalid update rejected");
    // PORT NOTE (ADR-011): ValidationFailed → `ContentInvalid` with the violations
    // in `message`.
    assert!(
        matches!(
            err,
            SmError {
                status: CallStatusType::ContentInvalid,
                ..
            }
        ) && !err.message.is_empty(),
        "expected content_invalid (422), got {err:?}"
    );

    let current = svc
        .get_composition_latest(ehr_uuid, vo_uuid)
        .await
        .expect("current still readable");
    assert_eq!(
        current["uid"]["value"], ovid_v1,
        "the rejected update did not advance the version"
    );
}

/// A version-item `commit_audit.change_type` as a coded openEHR audit change type.
fn change_type_coded(code: &str, value: &str) -> Value {
    json!({
        "_type": "DV_CODED_TEXT", "value": value,
        "defining_code": {
            "_type": "CODE_PHRASE",
            "terminology_id": { "_type": "TERMINOLOGY_ID", "value": "openehr" },
            "code_string": code
        }
    })
}

/// A `553|incomplete|` CONTRIBUTION carrying a single `creation` version.
fn incomplete_creation_contribution(data: Value) -> Value {
    json!({
        "versions": [{
            "data": data,
            "commit_audit": { "change_type": change_type_coded("249", "creation") },
            "lifecycle_state": {
                "terminology_id": { "_type": "TERMINOLOGY_ID", "value": "openehr" },
                "code_string": "553"
            }
        }]
    })
}

#[tokio::test]
async fn incomplete_lifecycle_relaxes_lower_bounds_but_not_wrongness() {
    // RM common master06 §"Incomplete Content": a `553|incomplete|` commit
    // treats existence/cardinality lower limits as zero ("data may be missing"),
    // while every wrongness check still applies ("but it may not be wrong").
    let pg = Pg::start().await;
    let pool = pg.migrated_pool("incomplete_lifecycle").await;
    let svc = EhrbaseService::new(pool.clone());

    svc.template_adl14_upload(fixture(IPS_OPT))
        .await
        .expect("upload IPS OPT");
    let ehr_uuid = svc.create_ehr(None).await.expect("create_ehr");

    // A composition missing its mandatory sections (content emptied): under the
    // IPS template each SECTION has occurrences min >= 1, so this is a pure
    // lower-bound (Required/Occurrences) violation with no wrongness.
    let mut missing = composition("ips_canonical.json");
    missing["content"] = json!([]);

    // Committed as `532|complete|` (the direct create path is always complete),
    // the missing-section lower bound is enforced → 422.
    let strict = svc
        .create_composition(ehr_uuid, uv(missing.clone(), "249", None))
        .await;
    assert!(
        matches!(
            strict,
            Err(SmError {
                status: CallStatusType::ContentInvalid,
                ..
            })
        ),
        "a complete commit must reject the missing mandatory sections, got {strict:?}"
    );
    assert_eq!(
        composition_versions(&pool).await,
        0,
        "the rejected complete commit persisted nothing"
    );

    // The identical body committed as `553|incomplete|` is accepted: the lower
    // limits are treated as zero.
    svc.create_ehr_contribution(ehr_uuid, incomplete_creation_contribution(missing))
        .await
        .expect("an incomplete commit tolerates the missing mandatory sections");
    assert_eq!(
        composition_versions(&pool).await,
        1,
        "the incomplete commit persisted the composition"
    );

    // But an incomplete commit does NOT tolerate *wrong* data: ips_invalid has
    // out-of-range magnitudes / coded values outside the value set, which are
    // still rejected under 553 ("may be missing, but may not be wrong").
    let wrong = svc
        .create_ehr_contribution(
            ehr_uuid,
            incomplete_creation_contribution(composition("ips_invalid.json")),
        )
        .await;
    assert!(
        matches!(
            wrong,
            Err(SmError {
                status: CallStatusType::ContentInvalid,
                ..
            })
        ),
        "an incomplete commit must still reject wrong (out-of-range/coded) data, got {wrong:?}"
    );
    assert_eq!(
        composition_versions(&pool).await,
        1,
        "the wrong incomplete commit persisted nothing"
    );
}
