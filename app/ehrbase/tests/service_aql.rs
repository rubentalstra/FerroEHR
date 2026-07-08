//! End-to-end AQL engine tests against a real PostgreSQL 18 (testcontainers):
//! seed EHRs + COMPOSITIONs through the service, then execute AQL through the
//! `QueryService` seam and assert on the assembled ITS-REST 1.0.3 `RESULT_SET`.
//!
//! Fixtures: the vendored openEHR SDK `minimal_observation` composition
//! (`openehr-its/tests/vendor/openehr_sdk/composition/canonical_json`), with its
//! `template_id` stripped (so only the template-independent RM-invariant +
//! terminology validation runs) and the leaf ELEMENT value swapped to a
//! `DV_QUANTITY` with a controlled magnitude — giving deterministic values to
//! order/compare/aggregate over.
//!
//! Acceptance set (design §Testing; QUERY 1.1 + the CNF QUERY chapter
//! `docs/specs/openehr/CNF/docs/platform_test_schedule/master11-func_tc_querying.adoc`):
//! CONTAINS chains, WHERE magnitude comparison + ORDER BY magnitude, DISTINCT,
//! aggregates, LIMIT/OFFSET + REST fetch/offset, `$parameters`, `ehr_id` scoping,
//! NOT CONTAINS, VERSION uid/time selection, whole-COMPOSITION reassembly
//! equality, and LATEST_VERSION vs ALL_VERSIONS over a twice-updated object.
// `float_cmp`: the magnitudes are exact whole numbers seeded by the test and
// round-tripped losslessly, so exact comparison is intended.
#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::doc_markdown,
    clippy::float_cmp,
    clippy::too_many_lines
)]

use std::collections::BTreeMap;

use serde_json::{Value, json};
use sqlx::{AssertSqlSafe, Connection, PgConnection, PgPool};
use testcontainers::runners::AsyncRunner;
use testcontainers::{ContainerAsync, ImageExt};
use testcontainers_modules::postgres::Postgres;

use ehrbase::db::{self, DbSettings};
use ehrbase::service::EhrbaseService;
use ehrbase_rest::{AqlQueryRequest, EhrCompositionService, EhrService, QueryService};

const OBS_ARCHETYPE: &str = "openEHR-EHR-OBSERVATION.minimal.v1";
/// The magnitude leaf path used throughout (bp.v1-style descent to the ELEMENT).
const MAG_PATH: &str = "data[at0001]/events[at0002]/data[at0003]/items[at0004]/value/magnitude";

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

/// The base composition with its template stripped and a `DV_QUANTITY` leaf.
fn base_composition() -> Value {
    let path = format!(
        "{}/../../crates/openehr-its/tests/vendor/openehr_sdk/composition/canonical_json/minimal_observation.json",
        env!("CARGO_MANIFEST_DIR")
    );
    serde_json::from_str(&std::fs::read_to_string(&path).expect("read fixture"))
        .expect("parse fixture")
}

/// A composition variant with the given display name and leaf magnitude.
fn composition(name: &str, magnitude: f64) -> Value {
    let mut c = base_composition();
    // Strip the template id so only RM-invariant + terminology validation runs
    // (no archetype-conformance pass), and drop any incoming uid.
    if let Some(details) = c
        .get_mut("archetype_details")
        .and_then(Value::as_object_mut)
    {
        details.remove("template_id");
    }
    if let Some(obj) = c.as_object_mut() {
        obj.remove("uid");
    }
    c["name"] = json!({ "_type": "DV_TEXT", "value": name });
    // The content OBSERVATION is an archetype root, so it must carry
    // archetype_details (RM invariant `Is_archetypeRoot`); the vendored fixture
    // omits it.
    c["content"][0]["archetype_details"] = json!({
        "_type": "ARCHETYPED",
        "archetype_id": { "_type": "ARCHETYPE_ID", "value": OBS_ARCHETYPE },
        "rm_version": "1.1.0",
    });
    c["content"][0]["data"]["events"][0]["data"]["items"][0]["value"] =
        json!({ "_type": "DV_QUANTITY", "magnitude": magnitude, "units": "mm[Hg]" });
    c
}

async fn create_ehr(svc: &EhrbaseService) -> String {
    let ehr = svc
        .ehr_create(params(json!({})), None)
        .await
        .expect("ehr_create");
    ehr.body["ehr_id"]["value"]
        .as_str()
        .expect("ehr_id")
        .to_owned()
}

/// Create a composition in `ehr_id`, returning its `OBJECT_VERSION_ID`.
async fn create_comp(svc: &EhrbaseService, ehr_id: &str, name: &str, magnitude: f64) -> String {
    let created = svc
        .composition_create(
            params(json!({ "ehr_id": ehr_id })),
            composition(name, magnitude),
        )
        .await
        .unwrap_or_else(|e| panic!("composition_create ({name}, {magnitude}): {e:?}"));
    created.body["uid"]["value"]
        .as_str()
        .expect("uid")
        .to_owned()
}

async fn run_aql(svc: &EhrbaseService, aql: &str, request: AqlQueryRequest) -> Value {
    svc.query_execute_adhoc(aql.to_owned(), request)
        .await
        .unwrap_or_else(|e| panic!("query {aql:?}: {e:?}"))
        .result_set
}

fn rows(result: &Value) -> &Vec<Value> {
    result["rows"].as_array().expect("rows array")
}

fn ehr_scope(ehr_id: &str) -> AqlQueryRequest {
    AqlQueryRequest {
        ehr_id: Some(ehr_id.to_owned()),
        ..AqlQueryRequest::default()
    }
}

#[tokio::test]
async fn aql_acceptance_set() {
    let pg = Pg::start().await;
    let pool = pg.migrated_pool("aql_accept").await;
    let svc = EhrbaseService::new(pool);

    let ehr_id = create_ehr(&svc).await;
    // Magnitudes 80, 100, 120; two share the name "BP", one is "HR".
    create_comp(&svc, &ehr_id, "BP", 80.0).await;
    create_comp(&svc, &ehr_id, "BP", 120.0).await;
    create_comp(&svc, &ehr_id, "HR", 100.0).await;
    // A second EHR with one composition, to prove ehr_id scoping.
    let other = create_ehr(&svc).await;
    create_comp(&svc, &other, "BP", 999.0).await;

    // ── COUNT + ehr_id scope ────────────────────────────────────────────────
    let r = run_aql(
        &svc,
        "SELECT COUNT(*) FROM EHR e CONTAINS COMPOSITION c",
        ehr_scope(&ehr_id),
    )
    .await;
    assert_eq!(rows(&r)[0][0], json!(3), "3 compositions in the scoped EHR");
    assert_eq!(
        r["columns"][0]["name"],
        json!("#0"),
        "unaliased column gets a generated name"
    );

    // Without ehr_id: the population count spans both EHRs.
    let r = run_aql(
        &svc,
        "SELECT COUNT(*) FROM EHR e CONTAINS COMPOSITION c",
        AqlQueryRequest::default(),
    )
    .await;
    assert_eq!(rows(&r)[0][0], json!(4), "4 compositions across all EHRs");

    // ── CONTAINS chain (EHR→COMPOSITION→OBSERVATION) + ORDER BY magnitude ────
    let aql = format!(
        "SELECT o/{MAG_PATH} AS m \
         FROM EHR e CONTAINS COMPOSITION c CONTAINS OBSERVATION o[{OBS_ARCHETYPE}] \
         ORDER BY o/{MAG_PATH}"
    );
    let r = run_aql(&svc, &aql, ehr_scope(&ehr_id)).await;
    let mags: Vec<f64> = rows(&r)
        .iter()
        .map(|row| row[0].as_f64().expect("magnitude"))
        .collect();
    assert_eq!(
        mags,
        vec![80.0, 100.0, 120.0],
        "ordered ascending by magnitude"
    );
    assert_eq!(
        r["columns"][0]["name"],
        json!("m"),
        "AS alias becomes the column name"
    );

    // ── WHERE magnitude comparison ($parameter) ───────────────────────────────
    let aql = format!(
        "SELECT o/{MAG_PATH} AS m \
         FROM EHR e CONTAINS OBSERVATION o[{OBS_ARCHETYPE}] \
         WHERE o/{MAG_PATH} > $min ORDER BY o/{MAG_PATH}"
    );
    let mut req = ehr_scope(&ehr_id);
    req.parameters = BTreeMap::from([("min".to_owned(), json!(90))]);
    let r = run_aql(&svc, &aql, req).await;
    let mags: Vec<f64> = rows(&r)
        .iter()
        .map(|row| row[0].as_f64().unwrap())
        .collect();
    assert_eq!(mags, vec![100.0, 120.0], "$min = 90 keeps 100 and 120");

    // ── aggregates (MAX/MIN/AVG over magnitude) ───────────────────────────────
    let aql = format!(
        "SELECT MAX(o/{MAG_PATH}), MIN(o/{MAG_PATH}) \
         FROM EHR e CONTAINS OBSERVATION o[{OBS_ARCHETYPE}]"
    );
    let r = run_aql(&svc, &aql, ehr_scope(&ehr_id)).await;
    assert_eq!(rows(&r)[0][0].as_f64().unwrap(), 120.0, "MAX magnitude");
    assert_eq!(rows(&r)[0][1].as_f64().unwrap(), 80.0, "MIN magnitude");

    // ── DISTINCT over the composition name ─────────────────────────────────────
    let r = run_aql(
        &svc,
        "SELECT DISTINCT c/name/value FROM EHR e CONTAINS COMPOSITION c",
        ehr_scope(&ehr_id),
    )
    .await;
    let mut names: Vec<String> = rows(&r)
        .iter()
        .map(|row| row[0].as_str().unwrap().to_owned())
        .collect();
    names.sort();
    assert_eq!(
        names,
        vec!["BP", "HR"],
        "two distinct names (BP appears twice)"
    );

    // ── LIMIT/OFFSET (AQL) ─────────────────────────────────────────────────────
    let aql = format!(
        "SELECT o/{MAG_PATH} AS m FROM EHR e CONTAINS OBSERVATION o[{OBS_ARCHETYPE}] \
         ORDER BY o/{MAG_PATH} LIMIT 1 OFFSET 1"
    );
    let r = run_aql(&svc, &aql, ehr_scope(&ehr_id)).await;
    assert_eq!(rows(&r).len(), 1, "LIMIT 1");
    assert_eq!(
        rows(&r)[0][0].as_f64().unwrap(),
        100.0,
        "OFFSET 1 → the middle value"
    );

    // ── REST fetch/offset paging (no AQL LIMIT) ────────────────────────────────
    let aql = format!(
        "SELECT o/{MAG_PATH} AS m FROM EHR e CONTAINS OBSERVATION o[{OBS_ARCHETYPE}] \
         ORDER BY o/{MAG_PATH}"
    );
    let mut req = ehr_scope(&ehr_id);
    req.fetch = Some(2);
    req.offset = Some(1);
    let r = run_aql(&svc, &aql, req).await;
    let mags: Vec<f64> = rows(&r)
        .iter()
        .map(|row| row[0].as_f64().unwrap())
        .collect();
    assert_eq!(mags, vec![100.0, 120.0], "fetch=2 offset=1 → rows 2 and 3");

    // fetch + AQL LIMIT is a spec conflict (ITS-REST query Request) → rejected.
    let aql =
        format!("SELECT o/{MAG_PATH} FROM EHR e CONTAINS OBSERVATION o[{OBS_ARCHETYPE}] LIMIT 5");
    let mut req = ehr_scope(&ehr_id);
    req.fetch = Some(2);
    let err = svc
        .query_execute_adhoc(aql, req)
        .await
        .expect_err("fetch + AQL LIMIT must conflict");
    assert!(
        matches!(err, openehr_its::rest::runtime::ApiError::BadRequest(_)),
        "paging conflict is 400, got {err:?}"
    );

    // ── NOT CONTAINS (none contain a different archetype) ──────────────────────
    let r = run_aql(
        &svc,
        "SELECT COUNT(*) FROM EHR e CONTAINS COMPOSITION c \
         NOT CONTAINS OBSERVATION o[openEHR-EHR-OBSERVATION.other.v1]",
        ehr_scope(&ehr_id),
    )
    .await;
    assert_eq!(rows(&r)[0][0], json!(3), "all 3 lack the other archetype");

    // ── VERSION metadata selection (uid + time_committed) ──────────────────────
    let r = run_aql(
        &svc,
        "SELECT v/uid/value, v/commit_audit/time_committed \
         FROM EHR e CONTAINS VERSION v CONTAINS COMPOSITION c",
        ehr_scope(&ehr_id),
    )
    .await;
    assert_eq!(rows(&r).len(), 3, "one latest version per composition");
    for row in rows(&r) {
        assert!(
            row[0].as_str().is_some_and(|u| u.contains("::")),
            "uid is an OBJECT_VERSION_ID: {:?}",
            row[0]
        );
        assert!(
            row[1].as_str().is_some(),
            "time_committed present: {:?}",
            row[1]
        );
    }

    // ── whole-COMPOSITION select reassembles equal to composition_get ──────────
    let single = create_ehr(&svc).await;
    let ovid = create_comp(&svc, &single, "solo", 55.0).await;
    let vo_id = ovid.split("::").next().unwrap();
    let r = run_aql(
        &svc,
        "SELECT c FROM EHR e CONTAINS COMPOSITION c",
        ehr_scope(&single),
    )
    .await;
    assert_eq!(rows(&r).len(), 1, "one composition in the single-comp EHR");
    let queried = &rows(&r)[0][0];
    let fetched = svc
        .composition_get(params(json!({ "ehr_id": single, "uid_based_id": vo_id })))
        .await
        .expect("composition_get");
    // The query reassembles the stored canonical JSON (no injected uid); compare
    // against the fetched body with its service-injected uid removed.
    let mut expected = fetched.body.clone();
    expected.as_object_mut().unwrap().remove("uid");
    assert_eq!(
        queried, &expected,
        "whole-object reassembly equals composition_get"
    );
}

#[tokio::test]
async fn latest_versus_all_versions() {
    let pg = Pg::start().await;
    let pool = pg.migrated_pool("aql_versions").await;
    let svc = EhrbaseService::new(pool);

    let ehr_id = create_ehr(&svc).await;
    let ovid = create_comp(&svc, &ehr_id, "v", 10.0).await;
    let vo_id = ovid.split("::").next().unwrap().to_owned();

    // Update the composition twice → sys_version 2 and 3. Each update supplies
    // the current version_uid as `If-Match` (optimistic concurrency).
    let mut current = ovid.clone();
    for magnitude in [20.0, 30.0] {
        let resp = svc
            .composition_update(
                params(json!({
                    "ehr_id": ehr_id,
                    "uid_based_id": vo_id,
                    "If-Match": current,
                })),
                composition("v", magnitude),
            )
            .await
            .unwrap_or_else(|e| panic!("composition_update {magnitude}: {e:?}"));
        current = resp.meta.expect("update meta").uid;
    }

    // LATEST_VERSION (the default) sees one version.
    let r = run_aql(
        &svc,
        "SELECT COUNT(*) FROM EHR e CONTAINS COMPOSITION c",
        ehr_scope(&ehr_id),
    )
    .await;
    assert_eq!(
        rows(&r)[0][0],
        json!(1),
        "LATEST_VERSION → 1 current version"
    );

    // ALL_VERSIONS sees all three — a capability EHRbase never had (ADR-008).
    let r = run_aql(
        &svc,
        "SELECT COUNT(*) FROM EHR e CONTAINS VERSION v[ALL_VERSIONS] CONTAINS COMPOSITION c",
        ehr_scope(&ehr_id),
    )
    .await;
    assert_eq!(rows(&r)[0][0], json!(3), "ALL_VERSIONS → all 3 versions");

    // The latest magnitude is the last update.
    let aql = format!(
        "SELECT c/content[{OBS_ARCHETYPE}]/{MAG_PATH} AS m FROM EHR e CONTAINS COMPOSITION c"
    );
    let r = run_aql(&svc, &aql, ehr_scope(&ehr_id)).await;
    assert_eq!(
        rows(&r)[0][0].as_f64().unwrap(),
        30.0,
        "latest magnitude is 30"
    );
}
