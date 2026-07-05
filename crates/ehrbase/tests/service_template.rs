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

    // Re-upload replaces idempotently (ON CONFLICT), still one row, still retrievable.
    svc.definition_template_adl1_4_upload(params(json!({})), Value::String(xml.clone()))
        .await
        .expect("re-upload");
    let list2 = svc
        .definition_template_adl1_4_list(params(json!({})))
        .await
        .expect("list after re-upload");
    assert_eq!(
        list2
            .iter()
            .filter(|t| t["template_id"] == TEMPLATE_ID)
            .count(),
        1,
        "re-upload replaced, not duplicated"
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
