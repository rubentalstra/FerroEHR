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

use openehr_base::prelude::TerminologyCode;
use openehr_rm::prelude::PartyProxy;
use uuid::Uuid;

use ehrbase::db::{self, DbSettings};
use ehrbase::service::EhrbaseService;
use ehrbase_sm::types::{UpdateAudit, UpdateVersion};
use ehrbase_sm::{
    AqlQueryRequest, CallStatusType, EhrCompositionService, EhrService, EhrStatusService,
    QueryService, SmError,
};

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

/// An `openehr` terminology code (audit change type / lifecycle state).
fn term(code: &str) -> TerminologyCode {
    TerminologyCode {
        terminology_id: "openehr".to_owned(),
        terminology_version: None,
        code_string: code.to_owned(),
        uri: None,
    }
}

/// The SM `UPDATE_VERSION` commit envelope for a bare-RM write, mirroring the
/// adapter's `mk_update_version` (the RM object is the `data`, `If-Match` is the
/// `preceding_version_uid`, and the audit carries the change type + committer).
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
    // PORT NOTE (ADR-011): the SM `create_ehr` returns the new `UUID`, not the
    // old `ServiceResponse` RM `EHR` envelope; the EHR's `ehr_id.value` is that
    // uuid, so the string form is the same id the old test read from `.body`.
    svc.create_ehr(None).await.expect("create_ehr").to_string()
}

/// Create a composition in `ehr_id`, returning its `OBJECT_VERSION_ID`.
async fn create_comp(svc: &EhrbaseService, ehr_id: &str, name: &str, magnitude: f64) -> String {
    // PORT NOTE (ADR-011): the SM `create_composition` returns the new
    // `version_uid` directly (what the old test extracted from `.body.uid.value`
    // / `.meta.uid`).
    svc.create_composition(
        ehr_id.parse().expect("ehr_id uuid"),
        uv(composition(name, magnitude), "249", None),
    )
    .await
    .unwrap_or_else(|e| panic!("create_composition ({name}, {magnitude}): {e:?}"))
}

async fn run_aql(svc: &EhrbaseService, aql: &str, request: AqlQueryRequest) -> Value {
    svc.query_execute_adhoc(aql.to_owned(), request)
        .await
        .unwrap_or_else(|e| panic!("query {aql:?}: {e:?}"))
        .result_set
}

/// Flip an EHR's `EHR_STATUS.is_queryable` to `false` through the service's
/// canonical status-update path (`ehr_status_update`), supplying the current
/// version `uid` as the `If-Match` precondition.
async fn set_not_queryable(svc: &EhrbaseService, ehr_id: &str) {
    // Read the current EHR_STATUS — its body carries the `uid` we need for the
    // optimistic-concurrency precondition, plus the mandatory RM fields we keep.
    let ehr_uuid: Uuid = ehr_id.parse().expect("ehr_id uuid");
    let mut body = svc
        .get_ehr_status_at_time(ehr_uuid, None)
        .await
        .expect("get_ehr_status_at_time");
    let if_match = body["uid"]["value"]
        .as_str()
        .expect("EHR_STATUS uid")
        .to_owned();
    let obj = body.as_object_mut().expect("EHR_STATUS object");
    obj.remove("uid");
    obj.insert("is_queryable".to_owned(), json!(false));
    svc.replace_ehr_status(ehr_uuid, uv(body, "251", Some(&if_match)))
        .await
        .expect("replace_ehr_status is_queryable=false");
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
    // PORT NOTE (ADR-011): the paging conflict is now the SM
    // `precondition_violation` (`SmError::precondition`), which the adapter maps
    // to the same wire `400` the old `ApiError::BadRequest` produced.
    assert!(
        matches!(
            err,
            SmError {
                status: CallStatusType::PreconditionViolation,
                ..
            }
        ),
        "paging conflict is precondition_violation, got {err:?}"
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
        .get_composition_latest(
            single.parse().expect("ehr uuid"),
            vo_id.parse().expect("vo uuid"),
        )
        .await
        .expect("get_composition_latest");
    // The query reassembles the stored canonical JSON (no injected uid); compare
    // against the fetched body with its service-injected uid removed.
    let mut expected = fetched.clone();
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
    let ehr_uuid: Uuid = ehr_id.parse().expect("ehr_id uuid");
    let vo_uuid: Uuid = vo_id.parse().expect("vo_id uuid");
    let mut current = ovid.clone();
    for magnitude in [20.0, 30.0] {
        // PORT NOTE (ADR-011): the SM `update_composition` returns the new
        // `version_uid` directly (the old `.meta.uid`).
        current = svc
            .update_composition(
                ehr_uuid,
                vo_uuid,
                uv(composition("v", magnitude), "251", Some(&current)),
            )
            .await
            .unwrap_or_else(|e| panic!("update_composition {magnitude}: {e:?}"));
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

/// The query-population gate, mandated by the SM `I_QUERY_SERVICE` interface for
/// both `execute_stored_query` and `execute_ad_hoc_query`. The `ehr_ids`
/// parameter doc reads (verbatim,
/// `docs/specs/openehr/SM/docs/UML/classes/i_query_service.adoc`):
///
/// > Specific set of EHRs on which to execute the query. If none supplied, a
/// > full population query will be performed on all EHRs whose status has the
/// > `is_queryable` flag set to `True`.
///
/// So an ad-hoc population query (no `ehr_id` scope) must include EHRs whose
/// current EHR_STATUS is queryable and exclude those whose current EHR_STATUS
/// has `is_queryable = False`.
#[tokio::test]
async fn population_query_excludes_not_queryable_ehrs() {
    let pg = Pg::start().await;
    let pool = pg.migrated_pool("aql_queryable_gate").await;
    let svc = EhrbaseService::new(pool);

    // Two EHRs; both start queryable (the default EHR_STATUS). Flip one off
    // through the canonical EHR_STATUS update path.
    let queryable = create_ehr(&svc).await;
    let hidden = create_ehr(&svc).await;
    set_not_queryable(&svc, &hidden).await;

    // A full population query: NO ehr_id scope supplied (`ehr_ids` = none).
    let r = run_aql(
        &svc,
        "SELECT e/ehr_id/value FROM EHR e",
        AqlQueryRequest::default(),
    )
    .await;
    let ids: Vec<String> = rows(&r)
        .iter()
        .map(|row| row[0].as_str().expect("ehr_id cell").to_owned())
        .collect();

    assert!(
        ids.contains(&queryable),
        "the queryable EHR is in the population result set: {ids:?}"
    );
    assert!(
        !ids.contains(&hidden),
        "the non-queryable EHR is excluded from the population result set \
         (is_queryable = True gate): {ids:?}"
    );
}

/// The assembled query response carries the ITS-REST 1.0.3 `RESULT_SET` shape.
///
/// Asserts only what the schema requires:
/// * `rows` is present and an array — the sole `required` field of
///   `docs/specs/openehr/ITS-REST/specifications/schemas/query/ResultSet.yaml`.
/// * every `columns[]` entry carries a `name` — the sole `required` field of
///   `schemas/query/ResultSetColumn.yaml`.
/// * each row is an array whose length equals the number of columns —
///   `schemas/query/ResultSetRow.yaml`: "A set of cells representing a
///   RESULT_SET row, one cell for each column."
#[tokio::test]
async fn result_set_carries_the_its_rest_shape() {
    let pg = Pg::start().await;
    let pool = pg.migrated_pool("aql_result_set_shape").await;
    let svc = EhrbaseService::new(pool);

    let ehr_id = create_ehr(&svc).await;
    create_comp(&svc, &ehr_id, "BP", 120.0).await;

    // A two-column projection over the seeded composition (deterministic via the
    // ehr_id scope), so the row/column relationship is meaningful.
    let r = run_aql(
        &svc,
        "SELECT e/ehr_id/value, c/name/value FROM EHR e CONTAINS COMPOSITION c",
        ehr_scope(&ehr_id),
    )
    .await;

    // ResultSet.yaml `required: [rows]`.
    let rows = r["rows"].as_array().expect("`rows` is a (required) array");
    assert!(
        !rows.is_empty(),
        "the seeded composition yields at least one row"
    );

    // ResultSetColumn.yaml `required: [name]`.
    let columns = r["columns"].as_array().expect("`columns` is an array");
    for col in columns {
        assert!(
            col["name"].as_str().is_some(),
            "every column carries a `name`: {col:?}"
        );
    }

    // ResultSetRow.yaml: one cell per column.
    for row in rows {
        let cells = row.as_array().expect("each row is an array of cells");
        assert_eq!(
            cells.len(),
            columns.len(),
            "row cell count equals column count (one cell per column)"
        );
    }
}

/// `e/ehr_status` on an EHR-typed variable (B6 cluster 2; ECC-QRY-006/010, the
/// A/106 `get_ehrs` golden). EHR is not a `node` in the store and `EHR_STATUS`
/// is a *separate* versioned object (RM 1.2.0 `EHR.ehr_status`), so the engine
/// resolves the path by joining the EHR's current `EHR_STATUS` VO and
/// reassembling it — rather than rejecting the query. Also exercises the exact
/// A/106 SELECT list (`ehr_id`, `time_created`, `system_id`, `ehr_status`) and
/// leaf extraction under `ehr_status`.
///
/// Golden columns:
/// `docs/specs/openehr/CNF/tests/platform/robot/_resources/test_data_sets/`
/// `query/expected_results/{empty_db,loaded_db}/A/106_get_ehrs.json`.
#[tokio::test]
async fn ehr_status_on_ehr_variable() {
    let pg = Pg::start().await;
    let pool = pg.migrated_pool("aql_ehr_status").await;
    let svc = EhrbaseService::new(pool);

    let ehr_id = create_ehr(&svc).await;

    // (1) Whole EHR_STATUS reassembled through the engine-level join.
    let r = run_aql(&svc, "SELECT e/ehr_status FROM EHR e", ehr_scope(&ehr_id)).await;
    assert_eq!(rows(&r).len(), 1, "one EHR in the scoped result set");
    let status = &rows(&r)[0][0];
    assert_eq!(
        status["_type"], "EHR_STATUS",
        "the cell is a reassembled EHR_STATUS object: {status:?}"
    );
    assert!(
        status.get("subject").is_some(),
        "the reassembled EHR_STATUS carries its mandatory `subject`: {status:?}"
    );
    assert_eq!(
        status["is_queryable"],
        json!(true),
        "default EHR_STATUS is queryable: {status:?}"
    );

    // (2) The exact A/106 SELECT list resolves (previously a 400 reject). The
    // golden's column metadata (name `#i` + path) is the data-independent
    // oracle.
    let r = run_aql(
        &svc,
        "SELECT e/ehr_id, e/time_created, e/system_id, e/ehr_status FROM EHR e",
        ehr_scope(&ehr_id),
    )
    .await;
    let cols = r["columns"].as_array().expect("columns array");
    let names: Vec<&str> = cols.iter().map(|c| c["name"].as_str().unwrap()).collect();
    let paths: Vec<&str> = cols.iter().map(|c| c["path"].as_str().unwrap()).collect();
    assert_eq!(names, vec!["#0", "#1", "#2", "#3"], "A/106 column names");
    assert_eq!(
        paths,
        vec!["/ehr_id", "/time_created", "/system_id", "/ehr_status"],
        "A/106 column paths"
    );
    assert_eq!(rows(&r).len(), 1, "one row for the scoped EHR");
    assert_eq!(
        rows(&r)[0][3]["_type"],
        "EHR_STATUS",
        "the fourth column is the EHR_STATUS object"
    );

    // (3) Leaf extraction under ehr_status: an inline scalar (`is_queryable`)
    // and an inline object attribute (`subject`, a PARTY_PROXY kept inline in
    // the root fragment).
    let r = run_aql(
        &svc,
        "SELECT e/ehr_status/is_queryable, e/ehr_status/subject FROM EHR e",
        ehr_scope(&ehr_id),
    )
    .await;
    assert_eq!(rows(&r).len(), 1, "one row");
    assert_eq!(
        rows(&r)[0][0],
        json!(true),
        "e/ehr_status/is_queryable extracts the boolean leaf"
    );
    assert!(
        rows(&r)[0][1]["_type"].as_str().is_some(),
        "e/ehr_status/subject extracts the PARTY_PROXY object: {:?}",
        rows(&r)[0][1]
    );
}

/// The empty-DB shape of the A/106 golden: the query must resolve (200, not a
/// 400 reject) and return the four columns with **no** rows when no EHR exists.
/// Matches `expected_results/empty_db/A/106_get_ehrs.json` (columns + `rows: []`).
#[tokio::test]
async fn ehr_status_query_empty_db() {
    let pg = Pg::start().await;
    let pool = pg.migrated_pool("aql_ehr_status_empty").await;
    let svc = EhrbaseService::new(pool);

    let r = run_aql(
        &svc,
        "SELECT e/ehr_id, e/time_created, e/system_id, e/ehr_status FROM EHR e",
        AqlQueryRequest::default(),
    )
    .await;
    let cols = r["columns"].as_array().expect("columns array");
    let paths: Vec<&str> = cols.iter().map(|c| c["path"].as_str().unwrap()).collect();
    assert_eq!(
        paths,
        vec!["/ehr_id", "/time_created", "/system_id", "/ehr_status"],
        "A/106 columns present on the empty DB"
    );
    assert!(rows(&r).is_empty(), "no EHRs → empty result set");
}
