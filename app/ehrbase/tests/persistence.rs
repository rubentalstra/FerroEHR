//! P09/P10 integration tests: the greenfield schema (ADR-008) applies
//! cleanly on a real `PostgreSQL` 18, the `ext` magnitude functions follow
//! the spec formulas, the temporal versioning model behaves, and the node
//! codec round-trips through the database.
//!
//! Requires Docker. Each test owns its container, so `Drop` removes it when
//! the test finishes — nothing is left running afterwards.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::path::Path;

use ehrbase::db::{self, DbSettings};
use ehrbase::storage::{NodeRow, decompose, reassemble};
use serde_json::Value;
use sqlx::{AssertSqlSafe, Connection, PgConnection, PgPool, Row};
use testcontainers::runners::AsyncRunner;
use testcontainers::{ContainerAsync, ImageExt};
use testcontainers_modules::postgres::Postgres;
use uuid::Uuid;

/// Debian-based image: ICU-enabled and ships the contrib extensions.
const PG_TAG: &str = "18";

struct Pg {
    #[allow(dead_code)] // held for its Drop (container removal)
    container: ContainerAsync<Postgres>,
    host: String,
    port: u16,
}

impl Pg {
    async fn start() -> Self {
        let container = Postgres::default()
            .with_tag(PG_TAG)
            .start()
            .await
            .expect("start postgres:18 container (is Docker running?)");
        let host = container
            .get_host()
            .await
            .expect("container host")
            .to_string();
        let port = container
            .get_host_port_ipv4(5432)
            .await
            .expect("mapped 5432");
        Self {
            container,
            host,
            port,
        }
    }

    async fn create_database(&self, name: &str) -> DbSettings {
        let Self { host, port, .. } = self;
        let admin_url = format!("postgres://postgres:postgres@{host}:{port}/postgres");
        let mut conn = PgConnection::connect(&admin_url)
            .await
            .expect("admin connect");
        sqlx::raw_sql(AssertSqlSafe(format!("CREATE DATABASE {name}")))
            .execute(&mut conn)
            .await
            .expect("create database");
        DbSettings::new(format!("postgres://postgres:postgres@{host}:{port}/{name}"))
    }

    async fn migrated_pool(&self, name: &str) -> PgPool {
        let settings = self.create_database(name).await;
        let pool = db::connect(&settings).await.expect("pool");
        db::run_migrations(&pool).await.expect("migrations apply");
        pool
    }
}

#[tokio::test]
async fn migrations_apply_cleanly_and_idempotently() {
    let pg = Pg::start().await;
    let pool = pg.migrated_pool("mig_apply").await;
    // running again must be a no-op, not an error
    db::run_migrations(&pool)
        .await
        .expect("migrations idempotent");

    let applied_ext: i64 = sqlx::query_scalar("SELECT count(*) FROM ext._sqlx_migrations")
        .fetch_one(&pool)
        .await
        .expect("ext bookkeeping");
    let applied_ehr: i64 = sqlx::query_scalar("SELECT count(*) FROM ehr._sqlx_migrations")
        .fetch_one(&pool)
        .await
        .expect("ehr bookkeeping");
    // ext: 0001_openehr_functions. ehr: 0001_schema + 0002_add_vo_version_signature
    // (version signing — RM common §"Digital Signature") +
    // 0003_demographic_party_storage (party kinds on the shared vo machinery;
    // no new tables) + 0004_vo_attestation (ATTESTATION storage — RM common
    // master06 §Change Control; adds the `vo_attestation` table below) +
    // 0005_archetype_store (SM-2 ADL 1.4 source archetypes, I_DEFINITION_ADL14;
    // adds the `archetype_store` table below) +
    // 0006_adl2_artefact (SM-2 ADL2 artefacts, I_DEFINITION_ADL2; adds the
    // `adl2_artefact` table below) +
    // 0007_party_relationship_and_ehr_index (SM-3: the PARTY_RELATIONSHIP kind
    // on the shared vo machinery + the `ehr_index` table below) +
    // 0008_creating_system_id_and_audit_invariant (storage-semantics audit
    // wave: the per-version `creating_system_id` column on `vo_version` [M2]
    // and the `audit_system_id_nonempty` CHECK on `audit` [m6]; adds no table).
    assert_eq!((applied_ext, applied_ehr), (1, 8));

    let tables: Vec<String> = sqlx::query_scalar(
        "SELECT table_name FROM information_schema.tables \
         WHERE table_schema = 'ehr' AND table_name <> '_sqlx_migrations' ORDER BY 1",
    )
    .fetch_all(&pool)
    .await
    .expect("tables");
    assert_eq!(
        tables,
        [
            "adl2_artefact",
            "archetype_store",
            "audit",
            "contribution",
            "ehr",
            "ehr_index",
            "item_tag",
            "node",
            "stored_query",
            "template_store",
            "vo_attestation",
            "vo_version",
        ]
    );
}

#[tokio::test]
async fn ext_magnitude_function_follows_the_spec_formulas() {
    let pg = Pg::start().await;
    let pool = pg.migrated_pool("ext_magnitude").await;

    let cases: &[(&str, f64)] = &[
        (
            r#"{"_type":"DV_QUANTITY","magnitude":117.0,"units":"mm[Hg]"}"#,
            117.0,
        ),
        (r#"{"_type":"DV_COUNT","magnitude":3}"#, 3.0),
        (r#"{"_type":"DV_ORDINAL","value":2}"#, 2.0),
        (
            r#"{"_type":"DV_PROPORTION","numerator":60.0,"denominator":100.0,"type":2}"#,
            0.6,
        ),
        // days since 0001-01-01: 1970-01-01 => 719162
        (r#"{"_type":"DV_DATE","value":"1970-01-01"}"#, 719_162.0),
        (r#"{"_type":"DV_DATE","value":"1970"}"#, 719_162.0),
        // seconds since 0001-01-01T00:00Z
        (
            r#"{"_type":"DV_DATE_TIME","value":"1970-01-01T00:00:00Z"}"#,
            62_135_596_800.0,
        ),
        (
            r#"{"_type":"DV_DATE_TIME","value":"1970-01-01T01:00:00+01:00"}"#,
            62_135_596_800.0,
        ),
        (r#"{"_type":"DV_TIME","value":"10:55:41"}"#, 39_341.0),
        (r#"{"_type":"DV_DURATION","value":"PT42M"}"#, 2_520.0),
        (
            r#"{"_type":"DV_DURATION","value":"P1Y"}"#,
            365.24 * 86_400.0,
        ),
        (r#"{"_type":"DV_DURATION","value":"-PT30S"}"#, -30.0),
    ];
    for (dv, expected) in cases {
        let got: Option<f64> = sqlx::query_scalar("SELECT openehr_magnitude($1::jsonb)::float8")
            .bind(dv)
            .fetch_one(&pool)
            .await
            .expect("magnitude call");
        let got = got.unwrap_or_else(|| panic!("NULL magnitude for {dv}"));
        assert!(
            (got - expected).abs() < 1e-6,
            "magnitude({dv}) = {got}, expected {expected}"
        );
    }
    // unknown/unparseable values yield NULL, never an error
    let none: Option<f64> =
        sqlx::query_scalar("SELECT openehr_magnitude('{\"_type\":\"DV_TEXT\"}'::jsonb)::float8")
            .fetch_one(&pool)
            .await
            .expect("null magnitude");
    assert!(none.is_none());
}

#[tokio::test]
async fn temporal_versioning_model_behaves() {
    let pg = Pg::start().await;
    let pool = pg.migrated_pool("temporal").await;
    let (vo, ehr_id) = seed_version(&pool).await;

    // an overlapping period is impossible at the database
    let overlap = sqlx::query(
        "INSERT INTO vo_version (vo_id, kind, ehr_id, sys_version, sys_period, contribution_id, audit_id)
         SELECT $1, 'COMPOSITION', $2, 2, tstzrange(now(), NULL), contribution_id, audit_id
         FROM vo_version WHERE vo_id = $1",
    )
    .bind(vo)
    .bind(ehr_id)
    .execute(&pool)
    .await;
    assert!(overlap.is_err(), "temporal PK must reject overlaps");

    // close v1, open v2 — adjacent periods are fine
    sqlx::query(
        "UPDATE vo_version SET sys_period = tstzrange(lower(sys_period), now())
         WHERE vo_id = $1 AND upper_inf(sys_period)",
    )
    .bind(vo)
    .execute(&pool)
    .await
    .expect("close v1");
    sqlx::query(
        "INSERT INTO vo_version (vo_id, kind, ehr_id, sys_version, sys_period, contribution_id, audit_id)
         SELECT $1, 'COMPOSITION', $2, 2, tstzrange(upper(sys_period), NULL), contribution_id, audit_id
         FROM vo_version WHERE vo_id = $1 AND sys_version = 1",
    )
    .bind(vo)
    .bind(ehr_id)
    .execute(&pool)
    .await
    .expect("open v2");

    // LATEST_VERSION = the upper_inf partial index; ALL_VERSIONS = unfiltered
    let current: i32 = sqlx::query_scalar(
        "SELECT sys_version FROM vo_version WHERE vo_id = $1 AND upper_inf(sys_period)",
    )
    .bind(vo)
    .fetch_one(&pool)
    .await
    .expect("current");
    assert_eq!(current, 2);
    let all: i64 = sqlx::query_scalar("SELECT count(*) FROM vo_version WHERE vo_id = $1")
        .bind(vo)
        .fetch_one(&pool)
        .await
        .expect("all versions");
    assert_eq!(all, 2);
}

#[tokio::test]
async fn node_codec_round_trips_through_the_database() {
    let pg = Pg::start().await;
    let pool = pg.migrated_pool("codec_db").await;

    // m5: round-trip EVERY corpus COMPOSITION through the real jsonb `node`
    // store (decompose → INSERT → SELECT → reassemble), not just one sample.
    // One container / one DB for speed; each composition gets its own vo.
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../crates/openehr-its/tests/vendor/openehr_sdk/composition/canonical_json");
    let mut checked = 0usize;
    for entry in std::fs::read_dir(&dir).expect("corpus dir") {
        let path = entry.expect("entry").path();
        if path.extension().is_none_or(|e| e != "json") {
            continue;
        }
        let text = std::fs::read_to_string(&path).expect("read corpus file");
        let Ok(composition) = serde_json::from_str::<Value>(&text) else {
            continue; // deliberately-invalid corpus files
        };
        if composition.get("_type").and_then(Value::as_str) != Some("COMPOSITION") {
            continue;
        }
        let (vo, ehr_id) = seed_version(&pool).await;
        let rows =
            decompose(composition.clone()).unwrap_or_else(|e| panic!("decompose {path:?}: {e}"));
        insert_nodes(&pool, vo, 1, ehr_id, &rows).await;

        let read: Vec<NodeRow> = sqlx::query(
            "SELECT num, num_cap, parent_num, citem_num, rm_type, archetype, name, path, data
             FROM node WHERE vo_id = $1 AND sys_version = 1 ORDER BY num",
        )
        .bind(vo)
        .fetch_all(&pool)
        .await
        .expect("read nodes")
        .into_iter()
        .map(|r| NodeRow {
            num: r.get("num"),
            num_cap: r.get("num_cap"),
            parent_num: r.get("parent_num"),
            citem_num: r.get("citem_num"),
            rm_type: r.get("rm_type"),
            archetype: r.get("archetype"),
            name: r.get("name"),
            path: r.get("path"),
            data: r.get("data"),
        })
        .collect();

        assert_eq!(read.len(), rows.len(), "node count for {path:?}");
        let reassembled = reassemble(&read).expect("reassemble");
        assert_eq!(
            reassembled, composition,
            "DB round-trip must be lossless for {path:?}"
        );
        checked += 1;
    }
    assert!(checked >= 50, "expected the full corpus, got {checked}");

    // The CONTAINS shape works against real rows (IPS sample).
    let (vo, ehr_id) = seed_version(&pool).await;
    let composition = corpus_sample();
    let rows = decompose(composition).expect("decompose");
    insert_nodes(&pool, vo, 1, ehr_id, &rows).await;
    let contains: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM node c
         JOIN node o ON o.vo_id = c.vo_id AND o.sys_version = c.sys_version
                    AND o.num BETWEEN c.num AND c.num_cap
         WHERE c.vo_id = $1 AND c.sys_version = 1 AND c.num = 0
           AND o.rm_type = 'OBSERVATION'",
    )
    .bind(vo)
    .fetch_one(&pool)
    .await
    .expect("contains query");
    let expected = i64::try_from(rows.iter().filter(|r| r.rm_type == "OBSERVATION").count())
        .expect("count fits");
    assert_eq!(contains, expected);
}

/// §6.2: the stored `vo_version.template_id` is read back through the version
/// read-back and surfaced by `EhrbaseService::template_of_version` (the ABAC
/// template attribute).
#[tokio::test]
async fn template_id_is_read_back_from_vo_version() {
    use ehrbase::service::EhrbaseService;

    let pg = Pg::start().await;
    let pool = pg.migrated_pool("authz_template_db").await;
    let (vo, ehr_id) = seed_version(&pool).await;
    // Production sets this on commit (service/vobject.rs); set it directly here.
    // vo_version.template_id has an FK into template_store — seed the template.
    seed_template(&pool, "org.openehr::vital_signs.v1").await;
    sqlx::query("UPDATE vo_version SET template_id = $2 WHERE vo_id = $1")
        .bind(vo)
        .bind("org.openehr::vital_signs.v1")
        .execute(&pool)
        .await
        .expect("set template_id");
    // Nodes so the read-back can reassemble the current version.
    let rows = decompose(corpus_sample()).expect("decompose");
    insert_nodes(&pool, vo, 1, ehr_id, &rows).await;

    let service = EhrbaseService::new(pool);
    // Current version.
    assert_eq!(
        service
            .template_of_version(vo, None)
            .await
            .expect("read template")
            .as_deref(),
        Some("org.openehr::vital_signs.v1")
    );
    // Explicit version 1.
    assert_eq!(
        service
            .template_of_version(vo, Some(1))
            .await
            .expect("read template v1")
            .as_deref(),
        Some("org.openehr::vital_signs.v1")
    );
    // Unknown object → None (not an error).
    assert_eq!(
        service
            .template_of_version(Uuid::now_v7(), None)
            .await
            .expect("unknown ok"),
        None
    );
}

/// §6.4 projection-independence regression (v1 defect #1): the ABAC query
/// subject-scope pre-filter restricts rows to the caller's patient EHRs, and the
/// executor collects the touched EHR/template sets, **even when the query
/// projects neither `ehr_id`/`value` nor a template path**.
#[tokio::test]
async fn query_subject_scope_filters_and_collects_projection_independently() {
    use ehrbase::service::EhrbaseService;
    use ehrbase_rest::{AqlQueryRequest, QueryService};

    let pg = Pg::start().await;
    let pool = pg.migrated_pool("authz_query_scope_db").await;

    // Two EHRs with distinct subjects, each holding one composition (same corpus
    // body) under a distinct template id.
    let (vo_a, ehr_a) = seed_version(&pool).await;
    let (vo_b, ehr_b) = seed_version(&pool).await;
    for (ehr, vo, subject, template) in [
        (ehr_a, vo_a, "SUBJ-A", "org.openehr::t_a.v1"),
        (ehr_b, vo_b, "SUBJ-B", "org.openehr::t_b.v1"),
    ] {
        sqlx::query("UPDATE ehr SET subject_id = $2 WHERE id = $1")
            .bind(ehr)
            .bind(subject)
            .execute(&pool)
            .await
            .expect("set subject");
        // vo_version.template_id has an FK into template_store — seed first.
        seed_template(&pool, template).await;
        sqlx::query("UPDATE vo_version SET template_id = $2 WHERE vo_id = $1")
            .bind(vo)
            .bind(template)
            .execute(&pool)
            .await
            .expect("set template");
        let rows = decompose(corpus_sample()).expect("decompose");
        insert_nodes(&pool, vo, 1, ehr, &rows).await;
    }

    let service = EhrbaseService::new(pool);
    // The projection is `c/name/value` — neither ehr_id nor a template path.
    let aql = "SELECT c/name/value FROM COMPOSITION c";

    // Unscoped: both compositions are visible (control).
    let all = service
        .query_execute_adhoc(aql.to_owned(), AqlQueryRequest::default())
        .await
        .expect("unscoped query");
    assert_eq!(row_count(&all.result_set), 2, "both compositions visible");

    // Scoped to SUBJ-A + collection on: only A's row is fetched, and the touched
    // EHR/template sets are collected despite the projection.
    let scoped = service
        .query_execute_adhoc(
            aql.to_owned(),
            AqlQueryRequest {
                subject_scope: Some("SUBJ-A".to_owned()),
                collect_attributes: true,
                ..Default::default()
            },
        )
        .await
        .expect("scoped query");
    assert_eq!(row_count(&scoped.result_set), 1, "only SUBJ-A row fetched");
    assert_eq!(scoped.ehr_ids, vec![ehr_a.to_string()]);
    assert_eq!(scoped.template_ids, vec!["org.openehr::t_a.v1".to_owned()]);
}

/// The number of `rows` in an ITS-REST `RESULT_SET`.
fn row_count(result_set: &Value) -> usize {
    result_set
        .get("rows")
        .and_then(Value::as_array)
        .map_or(0, Vec::len)
}

// ─── helpers ────────────────────────────────────────────────────────────────

/// Creates ehr + audit + contribution + an open v1 `vo_version`; returns
/// `(vo_id, ehr_id)`.
/// Seed a `template_store` row so `vo_version.template_id` (FK) can reference
/// it — production ingests the OPT before any commit can cite it.
async fn seed_template(pool: &PgPool, template_id: &str) {
    sqlx::query(
        "INSERT INTO template_store (template_id, content) VALUES ($1, '<test/>')
         ON CONFLICT (template_id) DO NOTHING",
    )
    .bind(template_id)
    .execute(pool)
    .await
    .expect("seed template_store");
}

async fn seed_version(pool: &PgPool) -> (Uuid, Uuid) {
    let ehr_id = Uuid::now_v7();
    let vo = Uuid::now_v7();
    sqlx::query("INSERT INTO ehr (id) VALUES ($1)")
        .bind(ehr_id)
        .execute(pool)
        .await
        .expect("ehr row");
    let audit_id: Uuid = sqlx::query_scalar(
        "INSERT INTO audit (system_id, change_type, committer)
         VALUES ('test.system', 'creation', '{\"_type\":\"PARTY_SELF\"}'::jsonb)
         RETURNING id",
    )
    .fetch_one(pool)
    .await
    .expect("audit row");
    let contribution_id: Uuid = sqlx::query_scalar(
        "INSERT INTO contribution (ehr_id, audit_id) VALUES ($1, $2) RETURNING id",
    )
    .bind(ehr_id)
    .bind(audit_id)
    .fetch_one(pool)
    .await
    .expect("contribution row");
    sqlx::query(
        "INSERT INTO vo_version (vo_id, kind, ehr_id, sys_version, sys_period, contribution_id, audit_id)
         VALUES ($1, 'COMPOSITION', $2, 1, tstzrange(now(), NULL), $3, $4)",
    )
    .bind(vo)
    .bind(ehr_id)
    .bind(contribution_id)
    .bind(audit_id)
    .execute(pool)
    .await
    .expect("vo_version row");
    // Every EHR has an EHR_STATUS from creation (RM ehr §"EHR Creation");
    // the AQL population gate keys off its `is_queryable` flag
    // (`i_query_service.adoc`), so a spec-realistic fixture must seed one —
    // a bare `ehr` row without a status is not a state the service can
    // produce.
    let status_vo = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO vo_version (vo_id, kind, ehr_id, sys_version, sys_period, contribution_id, audit_id)
         VALUES ($1, 'EHR_STATUS', $2, 1, tstzrange(now(), NULL), $3, $4)",
    )
    .bind(status_vo)
    .bind(ehr_id)
    .bind(contribution_id)
    .bind(audit_id)
    .execute(pool)
    .await
    .expect("ehr_status vo_version row");
    sqlx::query(
        "INSERT INTO node (vo_id, sys_version, num, num_cap, parent_num, rm_type, ehr_id, path, data)
         VALUES ($1, 1, 0, 0, 0, 'EHR_STATUS', $2, '',
                 '{\"_type\":\"EHR_STATUS\",\"is_queryable\":true,\"is_modifiable\":true}'::jsonb)",
    )
    .bind(status_vo)
    .bind(ehr_id)
    .execute(pool)
    .await
    .expect("ehr_status root node");
    (vo, ehr_id)
}

async fn insert_nodes(pool: &PgPool, vo: Uuid, sys_version: i32, ehr_id: Uuid, rows: &[NodeRow]) {
    for row in rows {
        sqlx::query(
            "INSERT INTO node (vo_id, sys_version, num, num_cap, parent_num, citem_num,
                               ehr_id, rm_type, archetype, name, path, data)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)",
        )
        .bind(vo)
        .bind(sys_version)
        .bind(row.num)
        .bind(row.num_cap)
        .bind(row.parent_num)
        .bind(row.citem_num)
        .bind(ehr_id)
        .bind(&row.rm_type)
        .bind(&row.archetype)
        .bind(&row.name)
        .bind(&row.path)
        .bind(&row.data)
        .execute(pool)
        .await
        .expect("insert node");
    }
}

/// A real corpus composition (the IPS — the largest one).
fn corpus_sample() -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(
        "../../crates/openehr-its/tests/vendor/openehr_sdk/composition/canonical_json/ips_canonical.json",
    );
    serde_json::from_str(&std::fs::read_to_string(path).expect("read ips_canonical.json"))
        .expect("parse composition")
}
