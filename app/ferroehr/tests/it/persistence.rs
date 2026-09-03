// SPDX-FileCopyrightText: Ruben Talstra
// SPDX-License-Identifier: BUSL-1.1

//! Persistence integration tests: the greenfield schema applies
//! cleanly on a real `PostgreSQL` 18, the `ext` magnitude functions follow
//! the spec formulas, the temporal versioning model behaves, and the node
//! codec round-trips through the database.
//!
//! Each test takes a fresh, fully-migrated database from the shared `testkit`
//! harness (`tools/testkit`); the returned guard releases the clone on drop.

#![expect(
    clippy::expect_used,
    reason = "clippy's in-test lint scoping (clippy.toml `allow-*-in-tests`) only \
              reaches `#[test]`-annotated functions, so it misses this integration \
              module's helpers and async bodies; panicking assertions and direct \
              fixture indexing are the intended shape here (the Rust Book ch11)"
)]

use std::path::Path;

use ferroehr::db;
use ferroehr::storage::codec::{decompose, reassemble};
use ferroehr::storage::row::NodeRow;
use serde_json::Value;
use sqlx::{PgPool, Row};
use uuid::Uuid;

#[tokio::test]
async fn migrations_apply_cleanly_and_idempotently() {
    let db = testkit::db().await.expect("testkit database");
    let pool = db.pool();
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
    // Single squashed baseline per schema, plus one file per own-extension
    // table set (pre-production rule: schema changes edit the baseline
    // directly). ext: 0001_openehr_functions (functions incl. openehr_timestamp
    // + roles + grants) + 0002_tenant_context. ehr: 0001_baseline (all core
    // tables) + 0002_event_outbox + 0003_event_subscription +
    // 0004_multitenancy + 0005_fhir_mapping + 0006_fhir_outbound_cursor +
    // 0007_cold_archive_tier + 0008_spec_profile_stable_compatible_stamp.
    assert_eq!((applied_ext, applied_ehr), (2, 8));

    let tables: Vec<String> = sqlx::query_scalar(
        "SELECT table_name FROM information_schema.tables \
         WHERE table_schema = 'ehr' AND table_type = 'BASE TABLE' \
           AND table_name <> '_sqlx_migrations' ORDER BY 1",
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
            "ehr_folder",
            "ehr_index",
            "event_outbox",
            "event_subscription",
            "fhir_mapping",
            "fhir_outbound_cursor",
            "item_tag",
            "node",
            "sp_binding",
            "sp_data_frame",
            "sp_data_set",
            "sp_sample",
            "sp_subject",
            "sp_variable",
            "stored_query",
            "template_ref",
            "template_store",
            "tenant",
            "vo_archive",
            "vo_attestation",
            "vo_version",
        ]
    );

    // The cold archival tier (0007): one mirror per moved relation, plus the
    // both-tier union views the whole-repository readers use. No openEHR spec
    // governs storage tiering — our own design/extension.
    let views: Vec<String> = sqlx::query_scalar(
        "SELECT table_name FROM information_schema.tables \
         WHERE table_schema = 'ehr' AND table_type = 'VIEW' ORDER BY 1",
    )
    .fetch_all(&pool)
    .await
    .expect("views");
    assert_eq!(views, ["node_all", "vo_attestation_all", "vo_version_all"]);

    let cold: Vec<String> = sqlx::query_scalar(
        "SELECT table_name FROM information_schema.tables \
         WHERE table_schema = 'cold' AND table_type = 'BASE TABLE' ORDER BY 1",
    )
    .fetch_all(&pool)
    .await
    .expect("cold tables");
    assert_eq!(cold, ["node", "vo_attestation", "vo_version"]);
}

/// A wipe of the `ehr` schema alone leaves the cold archival tier standing, and
/// the next boot refuses with the remedy rather than looping on a bare
/// `relation "vo_version" already exists`.
///
/// This is the exact sequence observed on a live cluster: `0007_cold_archive_tier`
/// is the only migration in the set whose objects live outside `ehr`, so
/// `DROP SCHEMA ehr CASCADE` takes the bookkeeping and leaves the mirrors. The
/// refusal is deliberate — adopting a surviving mirror would accept a shape copied
/// from the primary tables as they were before the wipe, and re-attach clinical
/// rows to a repository that no longer exists.
#[tokio::test]
async fn a_cold_tier_that_outlived_its_primary_tier_is_refused_with_the_remedy() {
    let db = testkit::db().await.expect("testkit database");
    let pool = db.pool();

    sqlx::query("DROP SCHEMA ehr CASCADE")
        .execute(&pool)
        .await
        .expect("drop the primary tier alone");

    // The mirrors are in a different schema, so they are still here — which is
    // the whole cause.
    let survivors: i64 =
        sqlx::query_scalar("SELECT count(*) FROM pg_tables WHERE schemaname = 'cold'")
            .fetch_one(&pool)
            .await
            .expect("count the surviving mirrors");
    assert_eq!(survivors, 3, "the cold tier must survive a wipe of `ehr`");

    let error = db::run_migrations(&pool)
        .await
        .expect_err("re-migrating over an orphaned cold tier must be refused");
    assert!(
        matches!(error, db::DbError::OrphanedArchiveTier),
        "the refusal must be the typed one, not a bare relation-exists error: {error}"
    );
    let message = error.to_string();
    assert!(
        message.contains("DROP SCHEMA cold CASCADE"),
        "the refusal must name the remedy: {message}"
    );

    // And the remedy actually works: with the orphan removed, the set applies
    // from scratch. Without this half the test would pin a refusal with no way out.
    sqlx::query("DROP SCHEMA cold CASCADE")
        .execute(&pool)
        .await
        .expect("apply the remedy");
    db::run_migrations(&pool)
        .await
        .expect("the migrations apply once the orphaned tier is gone");
}

#[tokio::test]
async fn ext_magnitude_function_follows_the_spec_formulas() {
    let db = testkit::db().await.expect("testkit database");
    let pool = db.pool();

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
    let db = testkit::db().await.expect("testkit database");
    let pool = db.pool();
    let (vo, ehr_id) = seed_version(&pool).await;

    // an overlapping period is impossible at the database
    let overlap = sqlx::query(
        "INSERT INTO vo_version (vo_id, kind, ehr_id, sys_version, trunk_version, sys_period, contribution_id, audit_id, creating_system_id)
         SELECT $1, 'COMPOSITION', $2, 2, 2, tstzrange(now(), NULL), contribution_id, audit_id, creating_system_id
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
        "INSERT INTO vo_version (vo_id, kind, ehr_id, sys_version, trunk_version, sys_period, contribution_id, audit_id, creating_system_id)
         SELECT $1, 'COMPOSITION', $2, 2, 2, tstzrange(upper(sys_period), NULL), contribution_id, audit_id, creating_system_id
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

/// A TRUNK position is unique across creating systems; a BRANCH id is not.
///
/// RM common `master06-change_control_package.adoc` §Version Identification
/// §Distributed Versioning identifies a version globally by the tuple
/// `{object_id, creating_system_id, version_tree_id}` — but that tuple alone
/// would admit two versions of ONE container both claiming trunk position 2,
/// one per system. The model forbids it: §Copying §Subsequent Local
/// Modifications makes a second system BRANCH rather than extend the trunk
/// ("the local system id is recorded in the `uid.creating_system_id()`
/// attribute, while branching numbering is used in the
/// `uid.version_tree_id()`"), and §Moving Version Containers has the trunk
/// CONTINUE its increment under the new system's id. Either way the trunk is
/// one global sequence.
///
/// BRANCH ids, by contrast, legitimately collide across systems — each system
/// allocates its branch numbers locally, which is precisely what the 3-part
/// identifier exists to disambiguate ("Two places are indicated on the diagram
/// where identification clashes could have occurred, but are prevented due to
/// the use of the 3-part unique Version identifier scheme"). So the tuple
/// constraint keeps `creating_system_id` and the trunk-position index does not.
///
/// The two live write paths derive the tree position from the container's own
/// tip and cannot produce a duplicate; the archive load replays arbitrary file
/// input, so it is guarded — and this pins both the typed refusal and the
/// database backstop behind it.
#[tokio::test]
async fn a_trunk_position_is_unique_across_creating_systems_but_a_branch_id_is_not() {
    use ferroehr::ids::{EhrId, VoId};
    use ferroehr::storage::error::StorageError;
    use ferroehr::storage::version_repo::import::{VerbatimVersionRow, insert_version_verbatim};

    let db = testkit::db().await.expect("testkit database");
    let pool = db.pool();
    let (vo, ehr_id) = seed_version(&pool).await;
    // The seeded row is trunk 1 created by `ferroehr.test`.
    let (contribution_id, audit_id): (Uuid, Uuid) =
        sqlx::query_as("SELECT contribution_id, audit_id FROM vo_version WHERE vo_id = $1")
            .bind(vo)
            .fetch_one(&pool)
            .await
            .expect("seeded provenance");

    let row = |trunk: i32, branch: i32, branch_version: i32, system: &'static str, ord: i32| {
        VerbatimVersionRow {
            vo_id: VoId(vo),
            kind: "COMPOSITION",
            ehr_id: Some(EhrId(ehr_id)),
            sys_version: ord,
            trunk_version: trunk,
            branch_number: branch,
            branch_version,
            preceding_version_uid: None,
            other_input_version_uids: None,
            sys_period_lower: Some("2026-01-01T00:00:00Z"),
            sys_period_upper: Some("2026-01-02T00:00:00Z"),
            lifecycle_state: "532",
            contribution_id,
            audit_id,
            template_id: None,
            signature: None,
            signature_client_supplied: false,
            creating_system_id: system,
            wrapped_original: None,
            body: None,
        }
    };

    let mut conn = pool.acquire().await.expect("connection");

    // A second creating system claiming the SAME trunk position is refused,
    // and the refusal names the container, the position and the holder.
    let clash = insert_version_verbatim(&mut conn, &row(1, 0, 0, "sysB.example.org", 2)).await;
    match clash {
        Err(StorageError::TrunkPositionInUse {
            vo_id,
            trunk_version,
            held_by,
        }) => {
            assert_eq!(vo_id, vo);
            assert_eq!(trunk_version, 1);
            assert_eq!(held_by, "ferroehr.test");
        }
        other => panic!("expected a typed trunk-position conflict, got {other:?}"),
    }

    // The database is the backstop behind the guard: the same row written past
    // the repository layer still cannot land.
    let raw = sqlx::query(
        "INSERT INTO vo_version (vo_id, kind, ehr_id, sys_version, trunk_version, sys_period, \
         contribution_id, audit_id, creating_system_id) \
         VALUES ($1, 'COMPOSITION', $2, 2, 1, \
                 tstzrange('2026-01-01T00:00:00Z'::timestamptz, '2026-01-02T00:00:00Z'::timestamptz), \
                 $3, $4, 'sysB.example.org')",
    )
    .bind(vo)
    .bind(ehr_id)
    .bind(contribution_id)
    .bind(audit_id)
    .execute(&pool)
    .await;
    assert!(
        raw.is_err(),
        "uq_vo_version_trunk_position must reject a second trunk row at one position"
    );

    // A BRANCH id, however, may repeat across creating systems: `1.1.1` minted
    // by two different systems off trunk node 1 are two distinct versions.
    insert_version_verbatim(&mut conn, &row(1, 1, 1, "sysB.example.org", 3))
        .await
        .expect("a foreign branch off trunk 1");
    insert_version_verbatim(&mut conn, &row(1, 1, 1, "sysC.example.org", 4))
        .await
        .expect("another system's branch with the SAME branch id is a distinct version");
    let branches: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM vo_version WHERE vo_id = $1 AND branch_number = 1",
    )
    .bind(vo)
    .fetch_one(&pool)
    .await
    .expect("branch rows");
    assert_eq!(
        branches, 2,
        "cross-system branch-id collisions stay admitted (the 3-part identifier disambiguates)"
    );
}

/// An as-of read resolves along the TRUNK.
///
/// `VERSIONED_OBJECT.version_at_time (a_time): VERSION[1]`
/// (`UML/classes/org.openehr.rm.common.versioned_object.adoc` §Functions)
/// returns exactly one version, yet a container may have several valid tips at
/// one instant — the trunk tip plus one per open branch — so only the trunk
/// makes the answer unique. The class draws the same line elsewhere:
/// `latest_version` is "the most recently added version (i.e. on trunk or any
/// branch)" while `latest_trunk_version` and `trunk_lifecycle_state` read the
/// trunk alone, the latter being how the spec says to decide "if the version
/// container is logically deleted".
///
/// The corollary this pins on the other side: a container holding branches but
/// no trunk version has no as-of answer — and that container is a state RM
/// common `master06-change_control_package.adoc` §Copying §Subsequent Local
/// Modifications rules out, since "branch versions … cannot be copied without
/// their corresponding preceding versions on the same branch (if any) and trunk
/// versions also being copied".
#[tokio::test]
async fn an_as_of_read_resolves_along_the_trunk() {
    use ferroehr::ids::{EhrId, VoId};
    use ferroehr::storage::version_repo::import::{VerbatimVersionRow, insert_version_verbatim};
    use ferroehr::storage::version_repo::read::version_at;

    let db = testkit::db().await.expect("testkit database");
    let pool = db.pool();
    let (vo, ehr_id) = seed_version(&pool).await;
    let (contribution_id, audit_id): (Uuid, Uuid) =
        sqlx::query_as("SELECT contribution_id, audit_id FROM vo_version WHERE vo_id = $1")
            .bind(vo)
            .fetch_one(&pool)
            .await
            .expect("seeded provenance");

    let branch_row = |target: VoId, ord: i32| VerbatimVersionRow {
        vo_id: target,
        kind: "COMPOSITION",
        ehr_id: Some(EhrId(ehr_id)),
        sys_version: ord,
        trunk_version: 1,
        branch_number: 1,
        branch_version: 1,
        preceding_version_uid: None,
        other_input_version_uids: None,
        sys_period_lower: Some("2020-01-01T00:00:00Z"),
        sys_period_upper: None,
        lifecycle_state: "532",
        contribution_id,
        audit_id,
        template_id: None,
        signature: None,
        signature_client_supplied: false,
        creating_system_id: "sysB.example.org",
        wrapped_original: None,
        body: None,
    };
    let mut conn = pool.acquire().await.expect("connection");

    // The seeded container's trunk version 1 is open from `now()`; a branch
    // tip open across the same instant does not displace it.
    insert_version_verbatim(&mut conn, &branch_row(VoId(vo), 2))
        .await
        .expect("branch beside the trunk");
    let at = jiff::Timestamp::now();
    let read = version_at(&pool, VoId(vo), at)
        .await
        .expect("as-of read")
        .expect("the trunk version is current at this instant");
    assert_eq!(
        (read.trunk_version, read.branch_number, read.branch_version),
        (1, 0, 0),
        "an as-of read returns the TRUNK version, never a branch tip"
    );

    // A container with branches and no trunk version has no as-of answer — the
    // copy-closure rule says such a container should not exist.
    let branch_only = VoId(Uuid::now_v7());
    insert_version_verbatim(&mut conn, &branch_row(branch_only, 1))
        .await
        .expect("a branch-only container");
    assert!(
        version_at(&pool, branch_only, at)
            .await
            .expect("as-of read")
            .is_none(),
        "a container with no trunk version has no as-of answer to give"
    );
}

#[tokio::test]
async fn node_codec_round_trips_through_the_database() {
    let db = testkit::db().await.expect("testkit database");
    let pool = db.pool();

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
            // arch_* are query-only promoted columns, unused by `reassemble`.
            arch_entity: None,
            arch_concept: None,
            arch_major: None,
            name: r.get("name"),
            path: r.get("path"),
            data: r.get("data"),
            // Promoted-leaf columns are query-only and unused by `reassemble`.
            promoted: Vec::new(),
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

/// The stored `vo_version.template_id` is read back through the version
/// read-back and surfaced by `FerroEhrService::template_of_version` (the ABAC
/// template attribute).
#[tokio::test]
async fn template_id_is_read_back_from_vo_version() {
    use ferroehr::service::FerroEhrService;

    let db = testkit::db().await.expect("testkit database");
    let pool = db.pool();
    let (vo, ehr_id) = seed_version(&pool).await;
    // Production sets this on commit (service/vobject.rs); set it directly here.
    // vo_version.template_id has an FK into template_ref — seed the template.
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

    let service = FerroEhrService::new(pool);
    // Current version.
    assert_eq!(
        service
            .template_of_version(ferroehr::ids::VoId(vo), None)
            .await
            .expect("read template")
            .as_deref(),
        Some("org.openehr::vital_signs.v1")
    );
    // Explicit version 1.
    assert_eq!(
        service
            .template_of_version(ferroehr::ids::VoId(vo), Some("1"))
            .await
            .expect("read template v1")
            .as_deref(),
        Some("org.openehr::vital_signs.v1")
    );
    // Unknown object → None (not an error).
    assert_eq!(
        service
            .template_of_version(ferroehr::ids::VoId::new(), None)
            .await
            .expect("unknown ok"),
        None
    );
}

/// Projection-independence regression (EHRbase v1 read these attributes off the
/// SELECT columns): the ABAC query subject-scope pre-filter restricts rows to
/// the caller's patient EHRs, and the executor collects the touched
/// EHR/template sets, **even when the query projects neither `ehr_id`/`value`
/// nor a template path**.
#[tokio::test]
async fn query_subject_scope_filters_and_collects_projection_independently() {
    use ferroehr::service::FerroEhrService;
    use ferroehr::service::query::request::AqlQueryRequest;

    let db = testkit::db().await.expect("testkit database");
    let pool = db.pool();

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

    let service = FerroEhrService::new(pool);
    // The projection is `c/name/value` — neither ehr_id nor a template path.
    let aql = "SELECT c/name/value FROM COMPOSITION c";

    // Unscoped: both compositions are visible (control).
    let all = service
        .execute_ad_hoc_query(aql.to_owned(), AqlQueryRequest::default())
        .await
        .expect("unscoped query");
    assert_eq!(row_count(&all.result_set), 2, "both compositions visible");

    // Scoped to SUBJ-A + collection on: only A's row is fetched, and the touched
    // EHR/template sets are collected despite the projection.
    let scoped = service
        .execute_ad_hoc_query(
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
    // Register the wire address exactly as `store_template` does — the
    // vo_version.template_id FK targets the template_ref registry.
    sqlx::query("INSERT INTO template_ref (template_id) VALUES ($1) ON CONFLICT DO NOTHING")
        .bind(template_id)
        .execute(pool)
        .await
        .expect("seed template_ref");
}

async fn seed_version(pool: &PgPool) -> (Uuid, Uuid) {
    let ehr_id = Uuid::now_v7();
    let vo = Uuid::now_v7();
    // ehr.system_id is NOT NULL.
    sqlx::query("INSERT INTO ehr (id, system_id) VALUES ($1, 'ferroehr.test')")
        .bind(ehr_id)
        .execute(pool)
        .await
        .expect("ehr row");
    // audit.change_type is a coded audit_change_type value ('249' creation),
    // enforced by ck_audit_change_type — not the rubric.
    let audit_id: Uuid = sqlx::query_scalar(
        "INSERT INTO audit (system_id, change_type, committer)
         VALUES ('test.system', '249', '{\"_type\":\"PARTY_SELF\"}'::jsonb)
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
    // creating_system_id is NOT NULL.
    sqlx::query(
        "INSERT INTO vo_version (vo_id, kind, ehr_id, sys_version, trunk_version, sys_period, contribution_id, audit_id, creating_system_id)
         VALUES ($1, 'COMPOSITION', $2, 1, 1, tstzrange(now(), NULL), $3, $4, 'ferroehr.test')",
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
        "INSERT INTO vo_version (vo_id, kind, ehr_id, sys_version, trunk_version, sys_period, contribution_id, audit_id, creating_system_id)
         VALUES ($1, 'EHR_STATUS', $2, 1, 1, tstzrange(now(), NULL), $3, $4, 'ferroehr.test')",
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

/// The materialized `vo_version.body` is byte-identical to the node-row
/// reassembly on a REAL service commit — the parity the body column's whole
/// design rests on (reads serve `body`; AQL reads the nodes; both must be the
/// same canonical value, RM common master06 §Copying: a stored version is
/// served verbatim).
#[tokio::test]
async fn materialized_body_matches_node_reassembly_on_a_real_commit() {
    use ferroehr::service::FerroEhrService;
    use ferroehr::storage::node_repo::read_version_canonical;

    let db = testkit::db().await.expect("testkit database");
    let pool = db.pool();
    let service = FerroEhrService::new(pool.clone());
    let ehr_id = service.create_ehr(None).await.expect("ehr create");

    // The EHR create commits an EHR_STATUS through the full commit path.
    let (vo, body): (Uuid, Option<String>) = sqlx::query_as(
        "SELECT vo_id, body FROM vo_version WHERE ehr_id = $1 AND kind = 'EHR_STATUS'",
    )
    .bind(ehr_id.0)
    .fetch_one(&pool)
    .await
    .expect("status version row");
    let body: Value =
        serde_json::from_str(&body.expect("a content-bearing version materializes its body"))
            .expect("the stored body text parses");
    let reassembled = read_version_canonical(&pool, ferroehr::ids::VoId(vo), 1)
        .await
        .expect("node reassembly");
    assert_eq!(
        body, reassembled,
        "vo_version.body must equal the node-row reassembly"
    );
    assert_eq!(
        body.get("_type").and_then(Value::as_str),
        Some("EHR_STATUS")
    );
}

/// The fixed-text `unnest` node insert has no per-row parameter cost, so a
/// composition decomposing to more than 4,095 node rows — past the 65,535
/// extended-protocol parameter cap the old per-row shape hit — commits in one
/// statement (#2668).
#[tokio::test]
async fn write_nodes_survives_more_than_4095_rows() {
    let db = testkit::db().await.expect("testkit database");
    let pool = db.pool();
    let (vo, ehr_id) = seed_version(&pool).await;

    let n = 5_000;
    let mut rows = Vec::with_capacity(n);
    for i in 0..n {
        let num = i32::try_from(i).expect("row ordinal fits i32");
        rows.push(NodeRow {
            num,
            num_cap: num,
            parent_num: 0,
            citem_num: None,
            rm_type: "ELEMENT".to_owned(),
            archetype: None,
            arch_entity: None,
            arch_concept: None,
            arch_major: None,
            name: None,
            path: if i == 0 {
                String::new()
            } else {
                format!("items{i}.")
            },
            data: serde_json::json!({"_type": "ELEMENT", "archetype_node_id": "at0001"}),
            promoted: Vec::new(),
        });
    }
    let mut tx = pool.begin().await.expect("begin");
    ferroehr::storage::node_repo::write_nodes(
        &mut tx,
        ferroehr::ids::VoId(vo),
        1,
        Some(ferroehr::ids::EhrId(ehr_id)),
        &rows,
    )
    .await
    .expect("a 5,000-row version must write in one statement");
    tx.commit().await.expect("commit");

    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM node WHERE vo_id = $1")
        .bind(vo)
        .fetch_one(&pool)
        .await
        .expect("count");
    assert_eq!(count, 5_000);
}

/// The tenant-scoped pool applies the SAME session settings the base pool
/// does — `statement_timeout` included (#2669: the tenant `after_connect`
/// previously replaced the base hook and silently dropped the DB-side
/// runaway-query guard).
#[tokio::test]
async fn tenant_scoped_pool_applies_statement_timeout() {
    let db = testkit::db().await.expect("testkit database");
    let settings = db::DbConfig {
        url: ferroehr::config::secret::SecretUrl::new(db.url()),
        statement_timeout_ms: 12_345,
        max_connections: 2,
        min_connections: 0,
        ..db::DbConfig::default()
    };

    let pool = db::connect_tenant_scoped(&settings)
        .await
        .expect("tenant-scoped pool");
    let timeout: String = sqlx::query_scalar("SHOW statement_timeout")
        .fetch_one(&pool)
        .await
        .expect("read timeout");
    assert_eq!(
        timeout, "12345ms",
        "the tenant-scoped connection must carry the configured statement_timeout"
    );
}
