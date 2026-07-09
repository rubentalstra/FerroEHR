//! End-to-end tests for ATTESTATION support in the CONTRIBUTION path
//! (RM common `master06-change_control_package.adoc` §Change Control /
//! §Attestation; ITS-REST `UpdateVersion.yaml` + `UpdateAttestation.yaml`)
//! against a real `PostgreSQL` 18 (testcontainers).
//!
//! Covers: attestations committed with a NEW version
//! (`UPDATE_VERSION.attestations`, "Signing content at committal"); a later
//! `666|attestation|`-only contribution attaching an `ATTESTATION` to an
//! existing `ORIGINAL_VERSION` (no new version); their exposure on the served
//! `ORIGINAL_VERSION` and in `REVISION_HISTORY`; and `CONTRIBUTION.versions` +
//! the aggregate change-type semantics. Plus the error surface (400/422/404).

#![allow(clippy::expect_used, clippy::unwrap_used, clippy::too_many_lines)]

use ehrbase::db::{self, DbSettings};
use ehrbase::service::EhrbaseService;
use ehrbase_rest::{EhrCompositionService, EhrContributionService, EhrService};
use openehr_its::rest::runtime::ApiError;
use serde_json::{Value, json};
use sqlx::{AssertSqlSafe, Connection, PgConnection, PgPool};
use testcontainers::runners::AsyncRunner;
use testcontainers::{ContainerAsync, ImageExt};
use testcontainers_modules::postgres::Postgres;

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

/// A minimal *valid* RM COMPOSITION (mirrors the signing-test fixture).
fn composition(name: &str) -> Value {
    json!({
        "_type": "COMPOSITION",
        "archetype_node_id": "openEHR-EHR-COMPOSITION.encounter.v1",
        "archetype_details": {
            "_type": "ARCHETYPED",
            "archetype_id": {
                "_type": "ARCHETYPE_ID",
                "value": "openEHR-EHR-COMPOSITION.encounter.v1"
            },
            "rm_version": "1.2.0"
        },
        "name": { "_type": "DV_TEXT", "value": name },
        "language": {
            "_type": "CODE_PHRASE",
            "terminology_id": { "_type": "TERMINOLOGY_ID", "value": "ISO_639-1" },
            "code_string": "en"
        },
        "territory": {
            "_type": "CODE_PHRASE",
            "terminology_id": { "_type": "TERMINOLOGY_ID", "value": "ISO_3166-1" },
            "code_string": "NL"
        },
        "category": {
            "_type": "DV_CODED_TEXT",
            "value": "event",
            "defining_code": {
                "_type": "CODE_PHRASE",
                "terminology_id": { "_type": "TERMINOLOGY_ID", "value": "openehr" },
                "code_string": "433"
            }
        },
        "composer": { "_type": "PARTY_IDENTIFIED", "name": "conformance tester" }
    })
}

fn committer(name: &str) -> Value {
    json!({ "_type": "PARTY_IDENTIFIED", "name": name })
}

fn change_type(code: &str, value: &str) -> Value {
    json!({
        "_type": "DV_CODED_TEXT", "value": value,
        "defining_code": {
            "_type": "CODE_PHRASE",
            "terminology_id": { "_type": "TERMINOLOGY_ID", "value": "openehr" },
            "code_string": code
        }
    })
}

/// A wire `UPDATE_ATTESTATION` partial (the server completes it).
fn attestation(reason: &str, is_pending: bool) -> Value {
    json!({
        "_type": "UPDATE_ATTESTATION",
        "change_type": change_type("666", "attestation"),
        "committer": committer("attesting clinician"),
        "reason": { "_type": "DV_TEXT", "value": reason },
        "is_pending": is_pending,
        "proof": "proof-bytes"
    })
}

async fn create_ehr(svc: &EhrbaseService) -> String {
    let ehr = svc
        .ehr_create(params(json!({})), None)
        .await
        .expect("ehr_create");
    ehr.body["ehr_id"]["value"].as_str().unwrap().to_owned()
}

/// Read the served `ORIGINAL_VERSION` of a composition version.
async fn read_version(svc: &EhrbaseService, ehr_id: &str, vo: &str, ovid: &str) -> Value {
    svc.versioned_composition_version_get_by_id(params(json!({
        "ehr_id": ehr_id, "versioned_object_uid": vo, "version_uid": ovid
    })))
    .await
    .expect("versioned composition version")
    .body
}

/// The `OBJECT_VERSION_ID` of the first version listed in a `CONTRIBUTION`.
fn first_version_uid(contribution: &Value) -> String {
    contribution["versions"][0]["id"]["value"]
        .as_str()
        .expect("versions[0].id.value")
        .to_owned()
}

fn vo_of(ovid: &str) -> String {
    ovid.split("::").next().unwrap().to_owned()
}

#[tokio::test]
async fn accompanying_attestation_then_standalone_666_attestation() {
    let pg = Pg::start().await;
    let svc = EhrbaseService::new(pg.migrated_pool("attestation_flow").await);
    let ehr_id = create_ehr(&svc).await;

    // (1) A CONTRIBUTION that creates a COMPOSITION carrying an attestation
    // committed with the version (UPDATE_VERSION.attestations).
    let contribution = json!({
        "_type": "CONTRIBUTION",
        "versions": [{
            "_type": "ORIGINAL_VERSION",
            "commit_audit": {
                "change_type": change_type("249", "creation"),
                "committer": committer("author")
            },
            "lifecycle_state": change_type("532", "complete"),
            "data": composition("v1"),
            "attestations": [ attestation("witnessed", false) ]
        }],
        "audit": { "committer": committer("author") }
    });
    let created = svc
        .contribution_create(params(json!({ "ehr_id": ehr_id })), contribution)
        .await
        .expect("contribution_create with accompanying attestation → 201");
    let ovid_v1 = first_version_uid(&created.body);
    let vo_id = vo_of(&ovid_v1);

    // Reading the ORIGINAL_VERSION exposes the completed ATTESTATION.
    let ov = read_version(&svc, &ehr_id, &vo_id, &ovid_v1).await;
    let atts = ov["attestations"].as_array().expect("attestations array");
    assert_eq!(atts.len(), 1, "one attestation after step 1");
    let att = &atts[0];
    assert_eq!(att["_type"], "ATTESTATION");
    assert!(att["system_id"].is_string(), "server-completed system_id");
    assert_eq!(att["time_committed"]["_type"], "DV_DATE_TIME");
    assert_eq!(att["reason"]["value"], "witnessed");
    assert_eq!(att["is_pending"], json!(false));
    // The inherited change_type is the 666|attestation| code.
    assert_eq!(att["change_type"]["defining_code"]["code_string"], "666");

    // (2) A later 666-only CONTRIBUTION attesting that same version.
    let attest_contribution = json!({
        "versions": [{
            "preceding_version_uid": { "value": ovid_v1.clone() },
            "commit_audit": {
                "change_type": change_type("666", "attestation"),
                "committer": committer("senior reviewer"),
                "reason": { "_type": "DV_TEXT", "value": "authorised" },
                "is_pending": false
            }
        }]
    });
    let attest_created = svc
        .contribution_create(params(json!({ "ehr_id": ehr_id })), attest_contribution)
        .await
        .expect("666-only contribution → 201");

    // The 666 contribution's aggregate change_type is 666, and its versions
    // list the attested (existing) version.
    assert_eq!(
        attest_created.body["audit"]["change_type"]["defining_code"]["code_string"],
        "666"
    );
    let attest_versions = attest_created.body["versions"]
        .as_array()
        .expect("versions");
    assert_eq!(attest_versions.len(), 1, "attested version is listed");
    assert_eq!(
        attest_versions[0]["id"]["value"].as_str(),
        Some(ovid_v1.as_str())
    );

    // The ORIGINAL_VERSION now lists both attestations.
    let ov = read_version(&svc, &ehr_id, &vo_id, &ovid_v1).await;
    assert_eq!(
        ov["attestations"].as_array().map(Vec::len),
        Some(2),
        "two attestations after the standalone 666"
    );

    // REVISION_HISTORY: the single version's audits = commit audit + both
    // attestations (revision_history_item.adoc "there may also be further
    // attestations").
    let rh = svc
        .versioned_composition_revision_history(params(json!({
            "ehr_id": ehr_id, "versioned_object_uid": vo_id
        })))
        .await
        .expect("revision history")
        .body;
    let items = rh["items"].as_array().expect("items");
    assert_eq!(items.len(), 1, "one version");
    let audits = items[0]["audits"].as_array().expect("audits");
    assert_eq!(audits.len(), 3, "commit audit + 2 attestations");
    assert_eq!(audits[0]["_type"], "AUDIT_DETAILS");
    assert_eq!(audits[1]["_type"], "ATTESTATION");
    assert_eq!(audits[2]["_type"], "ATTESTATION");
}

#[tokio::test]
async fn attestation_error_cases() {
    let pg = Pg::start().await;
    let svc = EhrbaseService::new(pg.migrated_pool("attestation_errors").await);
    let ehr_id = create_ehr(&svc).await;

    // A real composition to attest.
    let v1 = svc
        .composition_create(params(json!({ "ehr_id": ehr_id })), composition("v1"))
        .await
        .expect("composition_create");
    let ovid_v1 = v1.body["uid"]["value"].as_str().unwrap().to_owned();

    let attempt = |body: Value| {
        let svc = svc.clone();
        let ehr = ehr_id.clone();
        async move {
            svc.contribution_create(params(json!({ "ehr_id": ehr })), body)
                .await
        }
    };

    // 666 without preceding_version_uid → 400 (cannot name its target).
    let err = attempt(json!({
        "versions": [{
            "commit_audit": {
                "change_type": change_type("666", "attestation"),
                "committer": committer("x"),
                "reason": { "_type": "DV_TEXT", "value": "authorised" },
                "is_pending": false
            }
        }]
    }))
    .await
    .expect_err("666 without preceding → error");
    assert!(matches!(err, ApiError::BadRequest(_)), "got {err:?}");

    // 666 carrying data → 422 (attestation adds no content).
    let err = attempt(json!({
        "versions": [{
            "preceding_version_uid": { "value": ovid_v1.clone() },
            "commit_audit": {
                "change_type": change_type("666", "attestation"),
                "committer": committer("x"),
                "reason": { "_type": "DV_TEXT", "value": "authorised" },
                "is_pending": false
            },
            "data": composition("nope")
        }]
    }))
    .await
    .expect_err("666 with data → error");
    assert!(matches!(err, ApiError::Unprocessable(_)), "got {err:?}");

    // Attestation missing reason → 422 (ATTESTATION.reason 1..1).
    let err = attempt(json!({
        "versions": [{
            "preceding_version_uid": { "value": ovid_v1.clone() },
            "commit_audit": {
                "change_type": change_type("666", "attestation"),
                "committer": committer("x"),
                "is_pending": false
            }
        }]
    }))
    .await
    .expect_err("missing reason → error");
    assert!(matches!(err, ApiError::Unprocessable(_)), "got {err:?}");

    // Attestation missing is_pending → 422 (ATTESTATION.is_pending 1..1).
    let err = attempt(json!({
        "versions": [{
            "preceding_version_uid": { "value": ovid_v1.clone() },
            "commit_audit": {
                "change_type": change_type("666", "attestation"),
                "committer": committer("x"),
                "reason": { "_type": "DV_TEXT", "value": "authorised" }
            }
        }]
    }))
    .await
    .expect_err("missing is_pending → error");
    assert!(matches!(err, ApiError::Unprocessable(_)), "got {err:?}");

    // Attestation of a non-existent version → 404.
    let ghost = "00000000-0000-7000-8000-000000000000::ehrbase-rs.local::1";
    let err = attempt(json!({
        "versions": [{
            "preceding_version_uid": { "value": ghost },
            "commit_audit": {
                "change_type": change_type("666", "attestation"),
                "committer": committer("x"),
                "reason": { "_type": "DV_TEXT", "value": "authorised" },
                "is_pending": false
            }
        }]
    }))
    .await
    .expect_err("non-existent version → error");
    assert!(matches!(err, ApiError::NotFound(_)), "got {err:?}");
}
