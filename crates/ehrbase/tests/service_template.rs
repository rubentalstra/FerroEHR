//! End-to-end service tests for OPT 1.4 operational-template ingestion against a
//! real `PostgreSQL` 18 (testcontainers): upload a corpus `.opt` template, list it,
//! retrieve its XML, and re-upload (idempotent replace) — driven through the
//! generated `DefinitionApi` trait exactly as the REST layer calls it.
#![allow(clippy::expect_used, clippy::unwrap_used)]

use serde_json::{Value, json};
use sqlx::{AssertSqlSafe, Connection, PgConnection, PgPool};
use testcontainers::runners::AsyncRunner;
use testcontainers::{ContainerAsync, ImageExt};
use testcontainers_modules::postgres::Postgres;

use ehrbase::db::{self, DbSettings};
use ehrbase::service::EhrbaseService;
use openehr_its::rest::generated::definition::DefinitionApi;

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

/// A representative corpus template (Ocean Template Designer OPT 1.4 XML).
const TEMPLATE_REL: &str = "tests/resources/service/knowledge/IDCR Allergies List.v0.opt";
const TEMPLATE_ID: &str = "IDCR Allergies List.v0";

fn corpus_opt(rel: &str) -> String {
    let path = format!("{}/{rel}", env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {path}: {e}"))
}

#[tokio::test]
async fn template_upload_list_get_roundtrip() {
    let pg = Pg::start().await;
    let svc = EhrbaseService::new(pg.migrated_pool("tpl").await);
    let xml = corpus_opt(TEMPLATE_REL);

    // Upload the OPT XML (arrives as a JSON string, as the lenient body reader hands it over).
    let desc = svc
        .definition_template_adl1_4_upload(params(json!({})), Value::String(xml.clone()))
        .await
        .expect("upload");
    assert_eq!(desc["template_id"], TEMPLATE_ID, "descriptor template_id");
    assert!(
        desc["archetype_id"]
            .as_str()
            .is_some_and(|a| a.contains("COMPOSITION")),
        "root archetype extracted: {desc}"
    );

    // List includes the uploaded template.
    let list = svc
        .definition_template_adl1_4_list(params(json!({})))
        .await
        .expect("list");
    assert!(
        list.iter().any(|t| t["template_id"] == TEMPLATE_ID),
        "list contains the template: {list:?}"
    );

    // Retrieve returns the stored OPT XML verbatim.
    let got = svc
        .definition_template_adl1_4_get(params(json!({ "template_id": TEMPLATE_ID })))
        .await
        .expect("get");
    let got_xml = got.as_str().expect("get returns the OPT XML string");
    assert_eq!(
        got_xml, xml,
        "retrieved OPT XML is byte-identical to the upload"
    );

    // Re-uploading an existing template_id is a 409 Conflict, not a silent
    // overwrite: OPTs are immutable on the adl1.4 endpoint (ITS-REST
    // `409_template_already_exists.yaml`; CNF `upload_opt-valid_opt_twice_conflict`).
    let conflict = svc
        .definition_template_adl1_4_upload(params(json!({})), Value::String(xml.clone()))
        .await
        .expect_err("re-upload of an existing template_id must conflict");
    assert!(
        matches!(conflict, openehr_its::rest::runtime::ApiError::Conflict(_)),
        "got {conflict:?}"
    );

    // The original template is untouched and there is still exactly one row.
    let list2 = svc
        .definition_template_adl1_4_list(params(json!({})))
        .await
        .expect("list after conflicting re-upload");
    assert_eq!(
        list2
            .iter()
            .filter(|t| t["template_id"] == TEMPLATE_ID)
            .count(),
        1,
        "conflicting re-upload did not duplicate"
    );
    let still = svc
        .definition_template_adl1_4_get(params(json!({ "template_id": TEMPLATE_ID })))
        .await
        .expect("get after conflict");
    assert_eq!(
        still.as_str().expect("xml"),
        xml,
        "conflicting re-upload did not overwrite the stored OPT"
    );
}

#[tokio::test]
async fn get_unknown_template_is_not_found() {
    let pg = Pg::start().await;
    let svc = EhrbaseService::new(pg.migrated_pool("tpl_missing").await);
    let err = svc
        .definition_template_adl1_4_get(params(json!({ "template_id": "does.not.exist.v0" })))
        .await
        .expect_err("expected not-found");
    // Maps to the ITS-REST 404 (ApiError::NotFound).
    assert!(
        matches!(err, openehr_its::rest::runtime::ApiError::NotFound(_)),
        "got {err:?}"
    );
}

#[tokio::test]
async fn invalid_opt_xml_is_rejected() {
    let pg = Pg::start().await;
    let svc = EhrbaseService::new(pg.migrated_pool("tpl_bad").await);
    let err = svc
        .definition_template_adl1_4_upload(
            params(json!({})),
            Value::String("<not-a-template/>".to_owned()),
        )
        .await
        .expect_err("expected rejection of a non-OPT body");
    assert!(
        matches!(err, openehr_its::rest::runtime::ApiError::Unprocessable(_)),
        "got {err:?}"
    );
}

// ── example generation (`adl1.4/{id}/example`) ───────────────────────────────

/// Varied real templates: an OBSERVATION with a history of events, an
/// EVALUATION-list, and one carrying an ACTION/INSTRUCTION structure.
const EXAMPLE_TEMPLATES: &[&str] = &[
    "tests/resources/service/knowledge/Vital Signs Encounter (Composition).opt",
    "tests/resources/service/knowledge/IDCR Allergies List.v0.opt",
    "tests/resources/service/knowledge/IDCR - Immunisation summary.v0.opt",
];

/// The (cached) `WebTemplate` built from an OPT file, as the service builds it.
fn web_template_of(rel: &str) -> openehr_flat::WebTemplate {
    let opt = openehr_its::opt14::from_xml(&corpus_opt(rel)).expect("parse OPT");
    openehr_flat::build_web_template(&opt).expect("build web template")
}

/// The generated `required` example is committable (passes the P15 validator)
/// and survives FLAT round-trip + canonical-XML serialization for real
/// templates. The example is fetched through the generated `DefinitionApi`
/// exactly as the REST layer calls it; validation/conversion use the same
/// `WebTemplate` the service caches.
#[tokio::test]
async fn required_example_validates_and_converts() {
    let pg = Pg::start().await;
    let svc = EhrbaseService::new(pg.migrated_pool("tpl_example").await);

    for (i, rel) in EXAMPLE_TEMPLATES.iter().enumerate() {
        let xml = corpus_opt(rel);
        let desc = svc
            .definition_template_adl1_4_upload(params(json!({})), Value::String(xml))
            .await
            .unwrap_or_else(|e| panic!("upload {rel}: {e:?}"));
        let template_id = desc["template_id"]
            .as_str()
            .unwrap_or_else(|| panic!("template_id for {rel}"))
            .to_owned();
        let wt = web_template_of(rel);

        // `required` example, via the generated trait (as the REST layer calls it).
        let comp = svc
            .definition_template_adl1_4_example_get(params(json!({
                "template_id": template_id,
                "detail_level": "required",
            })))
            .await
            .unwrap_or_else(|e| panic!("example for {rel}: {e:?}"));
        assert_eq!(
            comp.get("_type").and_then(Value::as_str),
            Some("COMPOSITION"),
            "{rel}: example is a COMPOSITION"
        );

        // Acceptance bar: the required example is committable — it passes the
        // full P15 validator (RM invariants + terminology + archetype
        // conformance) with no violations.
        let violations = openehr_flat::validate_composition(&comp, &wt);
        assert!(
            violations.is_empty(),
            "{rel}: required example must validate clean, got {} violation(s): {violations:?}",
            violations.len()
        );

        // FLAT round-trip is stable (canonical → FLAT → canonical → FLAT).
        let flat1 =
            openehr_flat::to_flat(&comp, &wt).unwrap_or_else(|e| panic!("{rel} to_flat: {e}"));
        let flat1_map: serde_json::Map<String, Value> = flat1.clone().into_iter().collect();
        let comp2 = openehr_flat::from_flat(&flat1_map, &wt)
            .unwrap_or_else(|e| panic!("{rel} from_flat: {e}"));
        let flat2 =
            openehr_flat::to_flat(&comp2, &wt).unwrap_or_else(|e| panic!("{rel} to_flat2: {e}"));
        assert_eq!(flat1, flat2, "{rel}: FLAT round-trip is stable");

        // Canonical-XML serialization succeeds (deserialises as an RM COMPOSITION
        // then emits canonical XML — the XML `Accept` path in the dispatcher).
        let typed: openehr_rm::prelude::Composition = serde_json::from_value(comp.clone())
            .unwrap_or_else(|e| panic!("{rel}: example deserialises as Composition: {e}"));
        let xml_out = openehr_its::xml::to_canonical_xml(&typed, "composition")
            .unwrap_or_else(|e| panic!("{rel}: canonical XML: {e}"));
        assert!(
            xml_out.contains("<composition"),
            "{rel}: XML has a composition root"
        );

        // The `output` form carries a deterministic uid; `input` does not.
        let output = svc
            .definition_template_adl1_4_example_get(params(json!({
                "template_id": template_id,
                "type": "output",
            })))
            .await
            .unwrap_or_else(|e| panic!("output example for {rel}: {e:?}"));
        assert!(
            output.pointer("/uid/value").is_some(),
            "{rel}: output example carries a uid"
        );
        assert!(comp.get("uid").is_none(), "{rel}: input example has no uid");

        // Distinct databases per iteration are unnecessary; template ids differ.
        let _ = i;
    }
}

#[tokio::test]
async fn example_for_unknown_template_is_not_found() {
    let pg = Pg::start().await;
    let svc = EhrbaseService::new(pg.migrated_pool("tpl_example_missing").await);
    let err = svc
        .definition_template_adl1_4_example_get(params(json!({
            "template_id": "does.not.exist.v0",
        })))
        .await
        .expect_err("expected not-found for an unknown template");
    assert!(
        matches!(err, openehr_its::rest::runtime::ApiError::NotFound(_)),
        "got {err:?}"
    );
}

#[tokio::test]
async fn example_with_invalid_detail_level_is_bad_request() {
    let pg = Pg::start().await;
    let svc = EhrbaseService::new(pg.migrated_pool("tpl_example_bad_level").await);
    // Upload a template so the failure is the detail_level, not a missing id.
    let xml = corpus_opt(TEMPLATE_REL);
    svc.definition_template_adl1_4_upload(params(json!({})), Value::String(xml))
        .await
        .expect("upload");
    let err = svc
        .definition_template_adl1_4_example_get(params(json!({
            "template_id": TEMPLATE_ID,
            "detail_level": "exhaustive",
        })))
        .await
        .expect_err("expected bad-request for an invalid detail_level");
    assert!(
        matches!(err, openehr_its::rest::runtime::ApiError::BadRequest(_)),
        "got {err:?}"
    );
}
