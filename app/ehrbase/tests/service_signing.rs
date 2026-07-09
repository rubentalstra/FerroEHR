//! End-to-end tests for version signing (`VERSION.signature`, RM common
//! §"Digital Signature"; design `docs/design/version-signing.md` §6.3–6.4)
//! against a real `PostgreSQL` 18 (testcontainers).
//!
//! The strongest assertion (§6.3): the digest recomputes from the **served**
//! `ORIGINAL_VERSION`'s `canonical_form` — proving commit-time and read-time
//! object identity. Also: `EHR_STATUS` / FOLDER / contribution versions are all
//! signed; client-supplied signatures are stored verbatim; `verify_on_read =
//! strict` turns a tampered row into a 5xx; canonical XML carries the signature.

#![allow(clippy::expect_used, clippy::unwrap_used, clippy::too_many_lines)]

use std::sync::Arc;

use ehrbase::db::{self, DbSettings};
use ehrbase::service::EhrbaseService;
use ehrbase_rest::{
    EhrCompositionService, EhrContributionService, EhrDirectoryService, EhrService,
    EhrStatusService,
};
use ehrbase_signing::{Mode, Signer, SigningConfig, Verdict, VerifyOnRead};
use openehr_its::rest::runtime::ApiError;
use openehr_rm::common::change_control::version_impl::canonical_form_of_json;
use serde_json::{Value, json};
use sqlx::{AssertSqlSafe, Connection, PgConnection, PgPool, Row};
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

fn uid(v: &Value) -> &str {
    v["uid"]["value"].as_str().expect("uid.value")
}

/// A minimal *valid* RM COMPOSITION: `language`, `territory`, `category`, and
/// `composer` are all `1..1` (RM ehr, COMPOSITION class), so the typed RM
/// validation rejects a fixture without them.
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

/// Assert a served `ORIGINAL_VERSION` carries a server digest signature that
/// recomputes from its own `canonical_form` — the strongest test (§6.3).
fn assert_digest_recomputes(ov: &Value) {
    assert_eq!(ov["_type"], "ORIGINAL_VERSION");
    let signature = ov["signature"]
        .as_str()
        .expect("ORIGINAL_VERSION.signature");
    assert!(
        signature.starts_with("sha256:"),
        "expected a digest signature, got {signature}"
    );
    // canonical_form_of_json strips the signature key + JCS-canonicalises — the
    // exact bytes that were signed at commit. digest_default().verify recomputes.
    let canonical = canonical_form_of_json(ov).expect("canonical form");
    assert_eq!(
        Signer::digest_default().verify(&canonical, signature),
        Verdict::DigestMatch,
        "digest must recompute from the served version's canonical form"
    );
}

async fn create_ehr(svc: &EhrbaseService) -> String {
    let ehr = svc
        .ehr_create(params(json!({})), None)
        .await
        .expect("ehr_create");
    ehr.body["ehr_id"]["value"].as_str().unwrap().to_owned()
}

#[tokio::test]
async fn composition_version_is_signed_and_digest_recomputes_from_served_version() {
    let pg = Pg::start().await;
    let svc = EhrbaseService::new(pg.migrated_pool("signing_comp").await);
    let ehr_id = create_ehr(&svc).await;

    // Create a composition, then commit a second version.
    let v1 = svc
        .composition_create(params(json!({ "ehr_id": ehr_id })), composition("v1"))
        .await
        .expect("composition_create");
    let ovid_v1 = uid(&v1.body).to_owned();
    let vo_id = ovid_v1.split("::").next().unwrap().to_owned();
    let v2 = svc
        .composition_update(
            params(json!({ "ehr_id": ehr_id, "uid_based_id": vo_id, "If-Match": ovid_v1 })),
            composition("v2"),
        )
        .await
        .expect("composition_update");
    let ovid_v2 = uid(&v2.body).to_owned();

    for ovid in [&ovid_v1, &ovid_v2] {
        let ov = svc
            .versioned_composition_version_get_by_id(params(json!({
                "ehr_id": ehr_id, "versioned_object_uid": vo_id, "version_uid": ovid
            })))
            .await
            .expect("versioned composition version")
            .body;
        assert_digest_recomputes(&ov);
    }
}

#[tokio::test]
async fn ehr_status_versions_are_signed_and_every_vo_version_carries_a_digest() {
    let pg = Pg::start().await;
    let pool = pg.migrated_pool("signing_status").await;
    let svc = EhrbaseService::new(pool.clone());
    let ehr_id = create_ehr(&svc).await;

    // Update EHR_STATUS → v2, then read the ORIGINAL_VERSION of v1.
    let status = svc
        .ehr_status_get_at_time(params(json!({ "ehr_id": ehr_id })))
        .await
        .expect("status get");
    let status_ovid_v1 = uid(&status.body).to_owned();
    let mut body = status.body.clone();
    body["is_modifiable"] = json!(false);
    svc.ehr_status_update(
        params(json!({ "ehr_id": ehr_id, "If-Match": status_ovid_v1 })),
        body,
    )
    .await
    .expect("status update");

    let status_vo = status_ovid_v1.split("::").next().unwrap().to_owned();
    let ov = svc
        .versioned_ehr_status_version_get_by_id(params(json!({
            "ehr_id": ehr_id, "versioned_object_uid": status_vo, "version_uid": status_ovid_v1
        })))
        .await
        .expect("versioned ehr_status version")
        .body;
    assert_eq!(ov["data"]["_type"], "EHR_STATUS");
    assert_digest_recomputes(&ov);

    // Also commit a directory (FOLDER) — its version is signed even though the
    // FOLDER endpoints serve the bare folder (no ORIGINAL_VERSION wrapper).
    let folder = json!({
        "_type": "FOLDER",
        "archetype_node_id": "openEHR-EHR-FOLDER.generic.v1",
        "name": { "_type": "DV_TEXT", "value": "root" }
    });
    svc.directory_create(params(json!({ "ehr_id": ehr_id })), folder)
        .await
        .expect("directory_create");

    // Sweep: EHR_STATUS (x2), EHR_ACCESS, FOLDER — every stored version is signed
    // with a digest (design §3.4: signing on by default).
    let rows = sqlx::query("SELECT kind, signature FROM vo_version ORDER BY kind, sys_version")
        .fetch_all(&pool)
        .await
        .expect("select vo_version");
    assert!(!rows.is_empty());
    for row in &rows {
        let kind: String = row.try_get("kind").unwrap();
        let sig: Option<String> = row.try_get("signature").unwrap();
        let sig = sig.unwrap_or_else(|| panic!("{kind} version is unsigned"));
        assert!(
            sig.starts_with("sha256:"),
            "{kind} version signature is not a digest: {sig}"
        );
    }
}

#[tokio::test]
async fn contribution_versions_are_signed() {
    let pg = Pg::start().await;
    let svc = EhrbaseService::new(pg.migrated_pool("signing_contrib").await);
    let ehr_id = create_ehr(&svc).await;

    let body = json!({
        "audit": {
            "change_type": change_type("249", "creation"),
            "committer": { "_type": "PARTY_IDENTIFIED", "name": "Dr. Contribution" }
        },
        "versions": [{
            "data": composition("Via contribution"),
            "commit_audit": { "change_type": change_type("249", "creation") }
        }]
    });
    let contribution = svc
        .contribution_create(params(json!({ "ehr_id": ehr_id })), body)
        .await
        .expect("contribution_create");
    let ovid = contribution.body["versions"][0]["id"]["value"]
        .as_str()
        .unwrap()
        .to_owned();
    let vo_id = ovid.split("::").next().unwrap().to_owned();

    let ov = svc
        .versioned_composition_version_get_by_id(params(json!({
            "ehr_id": ehr_id, "versioned_object_uid": vo_id, "version_uid": ovid
        })))
        .await
        .expect("versioned composition version")
        .body;
    assert_digest_recomputes(&ov);
}

#[tokio::test]
async fn client_supplied_signature_is_stored_verbatim() {
    // A CONTRIBUTION creation version carrying an author-generated signature —
    // stored verbatim, never re-signed (design §3.3).
    const CLIENT_SIG: &str =
        "-----BEGIN PGP SIGNATURE-----\nauthored-elsewhere\n-----END PGP SIGNATURE-----";
    let pg = Pg::start().await;
    let svc = EhrbaseService::new(pg.migrated_pool("signing_client").await);
    let ehr_id = create_ehr(&svc).await;

    let body = json!({
        "audit": {
            "change_type": change_type("249", "creation"),
            "committer": { "_type": "PARTY_IDENTIFIED", "name": "Dr. Author" }
        },
        "versions": [{
            "data": composition("Client signed"),
            "commit_audit": { "change_type": change_type("249", "creation") },
            "signature": CLIENT_SIG
        }]
    });
    let contribution = svc
        .contribution_create(params(json!({ "ehr_id": ehr_id })), body)
        .await
        .expect("contribution_create");
    let ovid = contribution.body["versions"][0]["id"]["value"]
        .as_str()
        .unwrap()
        .to_owned();
    let vo_id = ovid.split("::").next().unwrap().to_owned();

    let ov = svc
        .versioned_composition_version_get_by_id(params(json!({
            "ehr_id": ehr_id, "versioned_object_uid": vo_id, "version_uid": ovid
        })))
        .await
        .expect("versioned composition version")
        .body;
    assert_eq!(
        ov["signature"].as_str(),
        Some(CLIENT_SIG),
        "client-supplied signature must be stored verbatim"
    );
}

#[tokio::test]
async fn strict_verify_on_read_rejects_a_tampered_row() {
    let pg = Pg::start().await;
    let pool = pg.migrated_pool("signing_strict").await;
    // A strict-verify service: a signature that does not match the served
    // canonical form is a 5xx integrity failure (design §3.5).
    let config = SigningConfig {
        enabled: true,
        mode: Mode::Digest,
        key_path: None,
        key_passphrase: None,
        verify_on_read: VerifyOnRead::Strict,
    };
    let signer = Signer::from_config(&config).expect("strict signer");
    let svc = EhrbaseService::new(pool.clone()).with_signer(Arc::new(signer));
    let ehr_id = create_ehr(&svc).await;

    let v1 = svc
        .composition_create(params(json!({ "ehr_id": ehr_id })), composition("tamper"))
        .await
        .expect("composition_create");
    let ovid = uid(&v1.body).to_owned();
    let vo_id = ovid.split("::").next().unwrap().to_owned();

    // A clean read verifies fine.
    svc.versioned_composition_version_get_by_id(params(json!({
        "ehr_id": ehr_id, "versioned_object_uid": vo_id, "version_uid": ovid
    })))
    .await
    .expect("clean read verifies");

    // Tamper the stored signature via SQL.
    sqlx::query(
        "UPDATE vo_version SET signature = 'sha256:dGFtcGVyZWQ=' WHERE kind = 'COMPOSITION'",
    )
    .execute(&pool)
    .await
    .expect("tamper");

    let tampered = svc
        .versioned_composition_version_get_by_id(params(json!({
            "ehr_id": ehr_id, "versioned_object_uid": vo_id, "version_uid": ovid
        })))
        .await;
    assert!(
        matches!(tampered, Err(ApiError::Internal(_))),
        "strict verify_on_read must 5xx a tampered signature, got {tampered:?}"
    );
}

#[tokio::test]
async fn canonical_xml_carries_the_signature() {
    // The generated canonical-XML serialization (the same `to_canonical_xml` the
    // REST negotiate path uses) emits the `signature` element (design §4.4/§6.4).
    use openehr_rm::common::change_control::original_version::OriginalVersion;
    use openehr_rm::composition::composition::Composition;

    // A full corpus COMPOSITION (all mandatory fields) as the version data.
    const CORPUS: &str = include_str!(
        "../../../crates/openehr-its/tests/vendor/openehr_sdk/composition/canonical_json/minimal_persistent.json"
    );
    let data: Composition = serde_json::from_str(CORPUS).expect("typed composition");
    let ov: OriginalVersion<Composition> = serde_json::from_value(json!({
        "_type": "ORIGINAL_VERSION",
        "contribution": {
            "_type": "OBJECT_REF", "namespace": "local", "type": "CONTRIBUTION",
            "id": { "_type": "HIER_OBJECT_ID", "value": "0198f4a5-9df1-7d1e-8b6f-2b8c00000abc" }
        },
        "commit_audit": {
            "_type": "AUDIT_DETAILS",
            "system_id": "ehrbase-rs.local",
            "time_committed": { "_type": "DV_DATE_TIME", "value": "2026-07-07T10:11:12Z" },
            "change_type": change_type("249", "creation"),
            "committer": { "_type": "PARTY_IDENTIFIED", "name": "EHRbase" }
        },
        "uid": {
            "_type": "OBJECT_VERSION_ID",
            "value": "0198f4a5-9df1-7d1e-8b6f-2b8c00000001::ehrbase-rs.local::1"
        },
        "lifecycle_state": change_type("532", "complete"),
        "signature": "sha256:jtWX/CULavvzX0ehjowv2XZPICTQhN1t0+AXHfbEaNc=",
        "data": data
    }))
    .expect("typed ORIGINAL_VERSION");

    let xml =
        openehr_its::xml::to_canonical_xml(&ov, "original_version").expect("to_canonical_xml");
    assert!(
        xml.contains("sha256:jtWX/CULavvzX0ehjowv2XZPICTQhN1t0+AXHfbEaNc="),
        "canonical XML must carry the signature; got:\n{xml}"
    );
    assert!(xml.contains("signature"), "expected a <signature> element");
}

#[tokio::test]
async fn creating_system_id_and_signature_survive_a_system_id_change() {
    // M2 (RM common master06 §"Distributed Versioning"): the OBJECT_VERSION_ID's
    // creating_system_id is per-version immutable and reconstructed from
    // storage, never from the live service config. A later `with_system_id`
    // change must not mutate a historical version's uid nor invalidate the
    // signature that was computed over it.
    let pg = Pg::start().await;
    let pool = pg.migrated_pool("signing_csid").await;

    // Commit a composition under system id "sys-origin".
    let svc_a = EhrbaseService::new(pool.clone()).with_system_id("sys-origin");
    let ehr_id = create_ehr(&svc_a).await;
    let v1 = svc_a
        .composition_create(params(json!({ "ehr_id": ehr_id })), composition("v1"))
        .await
        .expect("composition_create");
    let ovid = uid(&v1.body).to_owned();
    let vo_id = ovid.split("::").next().unwrap().to_owned();
    // The uid's middle part is the creating system id.
    assert_eq!(
        ovid.split("::").nth(1),
        Some("sys-origin"),
        "uid carries the committing system id"
    );

    // A second service over the SAME pool with a DIFFERENT system id.
    let svc_b = EhrbaseService::new(pool.clone()).with_system_id("sys-changed");

    // Reading the composition back: the injected uid still carries "sys-origin"
    // (the stored creating_system_id), not the new config value.
    let read = svc_b
        .composition_get(params(json!({ "ehr_id": ehr_id, "uid_based_id": vo_id })))
        .await
        .expect("composition_get")
        .body;
    assert_eq!(
        uid(&read),
        ovid,
        "uid must be stable across a system-id change"
    );

    // The served ORIGINAL_VERSION still verifies — the signature was computed
    // over the stored creating_system_id, which the read path reconstructs.
    let ov = svc_b
        .versioned_composition_version_get_by_id(params(json!({
            "ehr_id": ehr_id, "versioned_object_uid": vo_id, "version_uid": ovid
        })))
        .await
        .expect("versioned composition version")
        .body;
    assert_eq!(ov["uid"]["value"], ovid);
    assert_digest_recomputes(&ov);
}
