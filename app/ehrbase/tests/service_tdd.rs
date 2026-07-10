//! End-to-end service tests for the SM `I_TDD_SERVICE.import_tdd` /
//! `import_tdds` TDD (Template Data Document) import path against a real
//! `PostgreSQL` 18 (testcontainers).
//!
//! Spec: SM `docs/specs/openehr/SM/docs/UML/classes/i_tdd_service.adoc`
//! (included by `SM/docs/openehr_platform/master09-message_service.adoc`);
//! design `docs/design/sm-platform/10-message-integration.md` §2. Fixtures are
//! the vendored CNF corpus TDD instances
//! (`docs/specs/openehr/CNF/tests/platform/robot/_resources/test_data_sets/compositions/TDD/`)
//! and their matching OPT
//! (`.../valid_templates/minimal_persistent/persistent_minimal.opt`).
//!
//! Scope this wave (see `service::tdd` PORT NOTE): the TDD **envelope** —
//! namespace, `template_id`, target-EHR existence, and operational-template
//! resolution — is validated; the OPT-guided body → COMPOSITION conversion is
//! deferred, so a well-formed TDD for a provisioned template is rejected with a
//! typed `precondition_violation` (never a silent partial COMPOSITION). These
//! tests assert the typed rejections at each boundary.
#![allow(clippy::expect_used, clippy::unwrap_used)]

use sqlx::{AssertSqlSafe, Connection, PgConnection, PgPool};
use testcontainers::runners::AsyncRunner;
use testcontainers::{ContainerAsync, ImageExt};
use testcontainers_modules::postgres::Postgres;
use uuid::Uuid;

use ehrbase::db::{self, DbSettings};
use ehrbase::service::EhrbaseService;
use ehrbase_sm::{CallStatusType, DefinitionAdapter, EhrService, TddService};

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

const CORPUS: &str = "../../docs/specs/openehr/CNF/tests/platform/robot/_resources/test_data_sets";
const TEMPLATE_ID: &str = "persistent_minimal.en.v1";

fn read_fixture(rel: &str) -> String {
    let path = format!("{}/{rel}", env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {path}: {e}"))
}

fn tdd(name: &str) -> String {
    read_fixture(&format!("{CORPUS}/compositions/TDD/{name}"))
}

fn persistent_minimal_opt() -> String {
    read_fixture(&format!(
        "{CORPUS}/valid_templates/minimal_persistent/persistent_minimal.opt"
    ))
}

/// A malformed (non-XML) payload is rejected `content_invalid`, not a 500.
#[tokio::test]
async fn tdd_import_rejects_malformed_payload() {
    let pg = Pg::start().await;
    let svc = EhrbaseService::new(pg.migrated_pool("tdd_malformed").await);
    let ehr = svc.create_ehr(None).await.expect("ehr");

    let err = svc
        .import_tdd(ehr, "this is not a TDD at all".to_owned())
        .await
        .expect_err("a non-XML payload must be rejected");
    assert_eq!(
        err.status,
        CallStatusType::ContentInvalid,
        "malformed payload → content_invalid: {err:?}"
    );
}

/// A payload that is not in the Ocean templates namespace is not a TDD →
/// `precondition_violation`.
#[tokio::test]
async fn tdd_import_rejects_non_tdd_xml() {
    let pg = Pg::start().await;
    let svc = EhrbaseService::new(pg.migrated_pool("tdd_wrong_ns").await);
    let ehr = svc.create_ehr(None).await.expect("ehr");

    // Well-formed XML, but a canonical-openEHR (not templates) namespace.
    let not_tdd = r#"<composition xmlns="http://schemas.openehr.org/v1"><name/></composition>"#;
    let err = svc
        .import_tdd(ehr, not_tdd.to_owned())
        .await
        .expect_err("a non-TDD XML document must be rejected");
    assert_eq!(
        err.status,
        CallStatusType::PreconditionViolation,
        "wrong namespace → precondition_violation: {err:?}"
    );
}

/// A valid TDD targeting an EHR that does not exist is `ehr_id_does_not_exist`
/// (the design-filled `has_ehr` precondition).
#[tokio::test]
async fn tdd_import_rejects_unknown_ehr() {
    let pg = Pg::start().await;
    let svc = EhrbaseService::new(pg.migrated_pool("tdd_no_ehr").await);

    let err = svc
        .import_tdd(Uuid::now_v7(), tdd("persistent_minimal.en.v1__full.xml"))
        .await
        .expect_err("a TDD for a non-existent EHR must be rejected");
    assert_eq!(
        err.status,
        CallStatusType::EhrIdDoesNotExist,
        "unknown EHR → ehr_id_does_not_exist: {err:?}"
    );
}

/// A valid TDD whose `template_id` names an unprovisioned template is
/// `template_does_not_exist` — the corpus `..__invalid_opt_doesnt_exist` case
/// (its root carries `template_id="not_exist"`).
#[tokio::test]
async fn tdd_import_rejects_unknown_template() {
    let pg = Pg::start().await;
    let svc = EhrbaseService::new(pg.migrated_pool("tdd_no_tpl").await);
    let ehr = svc.create_ehr(None).await.expect("ehr");

    let err = svc
        .import_tdd(ehr, tdd("nested.en.v1__invalid_opt_doesnt_exist.xml"))
        .await
        .expect_err("a TDD for an unprovisioned template must be rejected");
    assert_eq!(
        err.status,
        CallStatusType::TemplateDoesNotExist,
        "unknown template → template_does_not_exist: {err:?}"
    );
}

/// A well-formed TDD for a **provisioned** template passes every envelope
/// precondition (namespace, `template_id`, EHR, template resolution +
/// `WebTemplate` build) and is then rejected with a typed `precondition_violation` for the
/// deferred OPT-guided body walk — never a silent partial COMPOSITION.
#[tokio::test]
async fn tdd_import_provisioned_template_body_deferred() {
    let pg = Pg::start().await;
    let pool = pg.migrated_pool("tdd_deferred").await;
    let svc = EhrbaseService::new(pool.clone());
    let ehr = svc.create_ehr(None).await.expect("ehr");

    // Provision the operational template the TDD instantiates.
    let desc = svc
        .template_adl14_upload(persistent_minimal_opt())
        .await
        .expect("opt upload");
    assert_eq!(desc["template_id"], TEMPLATE_ID, "opt template_id");

    let err = svc
        .import_tdd(ehr, tdd("persistent_minimal.en.v1__full.xml"))
        .await
        .expect_err("body conversion is deferred this wave");
    assert_eq!(
        err.status,
        CallStatusType::PreconditionViolation,
        "deferred body → precondition_violation: {err:?}"
    );
    assert!(
        err.to_string().contains("body conversion"),
        "message names the deferred capability: {err}"
    );

    // Nothing was committed: the EHR holds no COMPOSITION.
    let comps: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM vo_version WHERE ehr_id = $1 AND kind = 'COMPOSITION'",
    )
    .bind(ehr)
    .fetch_one(&pool)
    .await
    .expect("count");
    assert_eq!(
        comps, 0,
        "no COMPOSITION committed on a deferred TDD import"
    );
}

/// `import_tdds` is fail-fast: a batch containing an invalid TDD rejects with a
/// typed error and commits nothing.
#[tokio::test]
async fn tdd_import_tdds_batch_fail_fast() {
    let pg = Pg::start().await;
    let svc = EhrbaseService::new(pg.migrated_pool("tdd_batch").await);
    let ehr = svc.create_ehr(None).await.expect("ehr");

    let err = svc
        .import_tdds(ehr, vec![tdd("nested.en.v1__invalid_opt_doesnt_exist.xml")])
        .await
        .expect_err("a batch with an unprovisioned-template TDD must be rejected");
    assert_eq!(
        err.status,
        CallStatusType::TemplateDoesNotExist,
        "batch fail-fast surfaces the item's typed error: {err:?}"
    );
}
