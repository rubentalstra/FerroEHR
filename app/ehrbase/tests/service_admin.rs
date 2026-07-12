//! End-to-end service tests for the ADMIN API (physical EHR delete) against a
//! real `PostgreSQL` 18 (testcontainers).
//!
//! Spec: SM `I_ADMIN_SERVICE.physical_ehr_delete`
//! (`docs/specs/openehr/SM/docs/UML/classes/i_admin_service.adoc`) — precondition
//! `has_ehr`, error `ehr_id_does_not_exist`. The cascade contract is the CNF
//! Robot prior art
//! (`docs/specs/openehr/CNF/tests/platform/robot/I_ADMIN_SERVICE/001-EHR.robot`):
//! after delete, every backing table returns to its pre-EHR baseline count. We
//! assert **zero rows remain** for the deleted EHR across `ehr`, `vo_version`,
//! `node`, `contribution`, `audit`, and `item_tag`, while a second EHR is left
//! entirely untouched.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::too_many_lines)]

use serde_json::{Value, json};
use sqlx::{AssertSqlSafe, Connection, PgConnection, PgPool};
use testcontainers::runners::AsyncRunner;
use testcontainers::{ContainerAsync, ImageExt};
use testcontainers_modules::postgres::Postgres;
use uuid::Uuid;

use openehr_base::prelude::TerminologyCode;
use openehr_rm::prelude::PartyProxy;

use ehrbase::db::{self, DbSettings};
use ehrbase::service::EhrbaseService;
use ehrbase_sm::services::PartyRelationshipService;
use ehrbase_sm::{UpdateAudit, UpdateVersion};
use ehrbase_sm::{
    AdminArchive, AdminService, CallStatusType, DemographicService, EhrDirectoryService,
    EhrService, EhrStatusService, ItemTagAdapter, PartyKind, PlatformService, SmError,
};

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

/// The `uid.value` (`OBJECT_VERSION_ID`) of a versioned-object body.
fn uid(v: &Value) -> &str {
    v["uid"]["value"].as_str().expect("uid.value")
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

/// Row counts scoped to one EHR across every table a physical delete must clear.
/// `audit` has no `ehr_id`, so it is counted by the audit ids the EHR's
/// versions/contributions reference (the same set the delete captures).
#[derive(Debug, Default, PartialEq, Eq)]
struct EhrRows {
    ehr: i64,
    vo_version: i64,
    node: i64,
    contribution: i64,
    item_tag: i64,
    audit: i64,
}

impl EhrRows {
    fn is_empty(&self) -> bool {
        *self == EhrRows::default()
    }
}

async fn count(pool: &PgPool, sql: &'static str, ehr_id: Uuid) -> i64 {
    sqlx::query_scalar(sql)
        .bind(ehr_id)
        .fetch_one(pool)
        .await
        .expect("count")
}

async fn ehr_rows(pool: &PgPool, ehr_id: Uuid) -> EhrRows {
    let audit_ids: Vec<Uuid> = sqlx::query_scalar(
        "SELECT audit_id FROM vo_version WHERE ehr_id = $1 \
         UNION SELECT audit_id FROM contribution WHERE ehr_id = $1",
    )
    .bind(ehr_id)
    .fetch_all(pool)
    .await
    .expect("audit ids");
    let audit: i64 = sqlx::query_scalar("SELECT count(*) FROM audit WHERE id = ANY($1)")
        .bind(&audit_ids)
        .fetch_one(pool)
        .await
        .expect("audit count");
    EhrRows {
        ehr: count(pool, "SELECT count(*) FROM ehr WHERE id = $1", ehr_id).await,
        vo_version: count(
            pool,
            "SELECT count(*) FROM vo_version WHERE ehr_id = $1",
            ehr_id,
        )
        .await,
        node: count(pool, "SELECT count(*) FROM node WHERE ehr_id = $1", ehr_id).await,
        contribution: count(
            pool,
            "SELECT count(*) FROM contribution WHERE ehr_id = $1",
            ehr_id,
        )
        .await,
        item_tag: count(
            pool,
            "SELECT count(*) FROM item_tag WHERE ehr_id = $1",
            ehr_id,
        )
        .await,
        audit,
    }
}

/// Seed an EHR with enough content to exercise every FK the physical delete
/// must cascade through: `EHR_STATUS` (two versions) + `EHR_ACCESS` from
/// creation, a directory FOLDER, and an item tag on the `EHR_STATUS`.
///
/// PORT NOTE: this deliberately avoids COMPOSITION. On this base commit,
/// COMPOSITION validation is stricter than the shared test fixtures supply, so
/// the pre-existing `service_ehr::ehr_composition_lifecycle_end_to_end` fails
/// identically (a base issue, not the admin change). `EHR_STATUS`/FOLDER writes
/// populate the same `vo_version`/`node`/`contribution`/`audit`/`item_tag`
/// tables, so the cascade contract is fully covered without COMPOSITION.
async fn seed_full_ehr(svc: &EhrbaseService) -> Uuid {
    let ehr_uuid = svc.create_ehr(None).await.expect("ehr");

    // A second EHR_STATUS version (create → update): a multi-version vo.
    let mut updated = svc
        .get_ehr_status_at_time(ehr_uuid, None)
        .await
        .expect("status get");
    let status_ovid = uid(&updated).to_owned();
    let status_vo = status_ovid.split("::").next().unwrap().to_owned();
    updated.as_object_mut().expect("status obj").remove("uid");
    // An item tag on the EHR_STATUS.
    svc.target_tags_replace(
        ehr_uuid,
        status_vo,
        "EHR_STATUS",
        vec![json!({ "key": "priority", "value": "high" })],
    )
    .await
    .expect("tag");

    // A directory FOLDER (another versioned-object kind through the cascade).
    svc.create_directory(
        ehr_uuid,
        uv(
            json!({ "_type": "FOLDER", "archetype_node_id": "openEHR-EHR-FOLDER.generic.v1", "name": { "_type": "DV_TEXT", "value": "root" } }),
            "249",
            None,
        ),
    )
    .await
    .expect("directory");

    // Deactivate LAST: with the B2 write guard, content writes on an EHR whose
    // EHR_STATUS.is_modifiable = false are refused (RM ehr master04 §"EHR
    // Active Status"), so the non-modifiable status must be the final change —
    // the cascade still deletes an EHR carrying a deactivated status, which is
    // this fixture's point.
    updated["is_modifiable"] = json!(false);
    svc.replace_ehr_status(ehr_uuid, uv(updated, "251", Some(&status_ovid)))
        .await
        .expect("status update");

    ehr_uuid
}

#[tokio::test]
async fn admin_delete_cascades_and_leaves_other_ehr_untouched() {
    let pg = Pg::start().await;
    // One database, two handles: the service owns one clone, the test queries
    // the other directly to assert the cascade.
    let pool = pg.migrated_pool("admin_cascade").await;
    let svc = EhrbaseService::new(pool.clone());
    let pool = &pool;

    let ehr1 = seed_full_ehr(&svc).await;
    let ehr2 = seed_full_ehr(&svc).await;

    // Both EHRs have content in every table before the delete.
    let before1 = ehr_rows(pool, ehr1).await;
    let before2 = ehr_rows(pool, ehr2).await;
    assert!(!before1.is_empty(), "ehr1 must be populated: {before1:?}");
    // EHR_STATUS v1+v2, EHR_ACCESS v1, FOLDER v1 → ≥4 versions; ≥3 contributions
    // (ehr create, status update, directory create); 1 item tag.
    assert!(before1.ehr == 1 && before1.vo_version >= 4 && before1.node >= 4);
    assert!(before1.contribution >= 3 && before1.item_tag == 1 && before1.audit >= 3);

    // Physical delete via the ADMIN seam (SM physical_ehr_delete).
    svc.admin_ehr_delete(ehr1.to_string())
        .await
        .expect("admin delete");

    // Every trace of ehr1 is physically gone (CNF cascade contract).
    let after1 = ehr_rows(pool, ehr1).await;
    assert!(
        after1.is_empty(),
        "physical delete must clear every table for the EHR, got {after1:?}"
    );

    // ehr2 is entirely untouched.
    let after2 = ehr_rows(pool, ehr2).await;
    assert_eq!(after2, before2, "the other EHR must be untouched");
}

#[tokio::test]
async fn admin_delete_unknown_ehr_is_not_found() {
    let pg = Pg::start().await;
    let svc = EhrbaseService::new(pg.migrated_pool("admin_missing").await);

    // `has_ehr` is false → `ehr_id_does_not_exist` → NotFound (→ HTTP 404).
    let missing = Uuid::now_v7().to_string();
    let res = svc.admin_ehr_delete(missing).await;
    assert!(
        matches!(
            res,
            Err(SmError {
                status: CallStatusType::VersionedObjectDoesNotExist,
                ..
            })
        ),
        "unknown EHR must be NotFound, got {res:?}"
    );

    // A malformed id is a 400.
    let bad = svc.admin_ehr_delete("not-a-uuid".to_owned()).await;
    assert!(
        matches!(
            bad,
            Err(SmError {
                status: CallStatusType::PreconditionViolation,
                ..
            })
        ),
        "malformed id must be BadRequest, got {bad:?}"
    );
}

#[tokio::test]
async fn admin_delete_all_deletes_present_and_skips_missing() {
    let pg = Pg::start().await;
    let svc = EhrbaseService::new(pg.migrated_pool("admin_delete_all").await);

    let a = seed_full_ehr(&svc).await;
    let b = seed_full_ehr(&svc).await;

    // A two-id list of existing EHRs deletes both.
    let deleted = svc
        .admin_ehr_delete_all(vec![a.to_string(), b.to_string()])
        .await
        .expect("delete all");
    assert_eq!(deleted, 2, "both existing EHRs deleted");

    // A list mixing one existing and one missing id deletes only the existing
    // (idempotent bulk: missing ids are skipped).
    let c = seed_full_ehr(&svc).await;
    let missing = Uuid::now_v7().to_string();
    let deleted = svc
        .admin_ehr_delete_all(vec![c.to_string(), missing])
        .await
        .expect("delete all with a bogus id");
    assert_eq!(
        deleted, 1,
        "only the existing EHR is deleted; missing skipped"
    );

    // A malformed id in the list rejects the whole request (400), no deletion.
    let d = seed_full_ehr(&svc).await;
    let res = svc
        .admin_ehr_delete_all(vec![d.to_string(), "not-a-uuid".to_owned()])
        .await;
    assert!(
        matches!(
            res,
            Err(SmError {
                status: CallStatusType::PreconditionViolation,
                ..
            })
        ),
        "a malformed id must be BadRequest, got {res:?}"
    );
}

// ─── SM-4: statistics / physical_party_delete / archive ───────────────────────

/// A minimal valid demographic PERSON (PARTY invariant `Identities_valid`).
fn person(name: &str) -> Value {
    json!({
        "_type": "PERSON",
        "archetype_node_id": "openEHR-DEMOGRAPHIC-PERSON.person.v1",
        "name": { "_type": "DV_TEXT", "value": name },
        "identities": [{
            "_type": "PARTY_IDENTITY",
            "archetype_node_id": "at0001",
            "name": { "_type": "DV_TEXT", "value": "legal name" },
            "details": {
                "_type": "ITEM_TREE",
                "archetype_node_id": "at0002",
                "name": { "_type": "DV_TEXT", "value": "structure" },
                "items": []
            }
        }]
    })
}

/// A `PARTY_RELATIONSHIP` from `source` to `target` (bare versioned-object ids).
fn relationship(name: &str, source: &str, target: &str) -> Value {
    json!({
        "_type": "PARTY_RELATIONSHIP",
        "archetype_node_id": "openEHR-DEMOGRAPHIC-PARTY_RELATIONSHIP.relationship.v1",
        "name": { "_type": "DV_TEXT", "value": name },
        "source": {
            "_type": "PARTY_REF", "namespace": "demographic", "type": "PERSON",
            "id": { "_type": "HIER_OBJECT_ID", "value": source }
        },
        "target": {
            "_type": "PARTY_REF", "namespace": "demographic", "type": "PERSON",
            "id": { "_type": "HIER_OBJECT_ID", "value": target }
        }
    })
}

/// Create a PERSON and return its bare versioned-object UUID string.
async fn make_person(svc: &EhrbaseService, name: &str) -> String {
    let created = svc
        .party_create(PartyKind::Person, person(name))
        .await
        .expect("create person");
    created.body["uid"]["value"]
        .as_str()
        .expect("uid")
        .split("::")
        .next()
        .expect("vo uuid")
        .to_owned()
}

/// The number of `vo_version` rows for one versioned object (0 = physically gone).
async fn vo_version_rows(pool: &PgPool, vo: &str) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM vo_version WHERE vo_id = $1::uuid")
        .bind(vo)
        .fetch_one(pool)
        .await
        .expect("vo_version count")
}

#[tokio::test]
async fn admin_statistics_per_service_and_time_range() {
    let pg = Pg::start().await;
    let pool = pg.migrated_pool("admin_stats").await;
    let svc = EhrbaseService::new(pool.clone());
    let pool = &pool;

    let ehr = seed_full_ehr(&svc).await; // several EHR-scoped contributions
    make_person(&svc, "P").await; // one demographic (ehr-less) contribution

    // Directly seed COMPOSITIONs (the service fixtures here avoid COMPOSITION):
    // one vo with two versions + one vo with a single version → 2 versioned
    // compositions across 3 version rows. Reuse an existing audit/contribution.
    let (cid, aid): (Uuid, Uuid) =
        sqlx::query_as("SELECT id, audit_id FROM contribution WHERE ehr_id = $1 LIMIT 1")
            .bind(ehr)
            .fetch_one(pool)
            .await
            .expect("an EHR contribution");
    let vo_x = Uuid::now_v7();
    let vo_y = Uuid::now_v7();
    for (vo, ver, period) in [
        (
            vo_x,
            1,
            "tstzrange(now() - interval '2 seconds', now() - interval '1 second', '[)')",
        ),
        (
            vo_x,
            2,
            "tstzrange(now() - interval '1 second', NULL, '[)')",
        ),
        (
            vo_y,
            1,
            "tstzrange(now() - interval '1 second', NULL, '[)')",
        ),
    ] {
        sqlx::query(AssertSqlSafe(format!(
            "INSERT INTO vo_version (vo_id, kind, ehr_id, sys_version, trunk_version, sys_period, contribution_id, audit_id, creating_system_id) \
             VALUES ($1, 'COMPOSITION', $2, $3, $3, {period}, $4, $5, 'ehrbase-rs.test')"
        )))
        .bind(vo)
        .bind(ehr)
        .bind(ver)
        .bind(cid)
        .bind(aid)
        .execute(pool)
        .await
        .expect("seed composition version");
    }

    // Contribution counts: Ehr scope has the seeded EHR's contributions;
    // Demographic scope has the party create; a non-content service has none.
    let ehr_count = svc
        .admin_contribution_count(PlatformService::Ehr, None)
        .await
        .expect("ehr count");
    assert!(
        ehr_count >= 3,
        "expected the EHR's contributions, got {ehr_count}"
    );
    let demo_count = svc
        .admin_contribution_count(PlatformService::Demographic, None)
        .await
        .expect("demo count");
    assert_eq!(demo_count, 1, "one demographic contribution (the party)");
    let query_count = svc
        .admin_contribution_count(PlatformService::Query, None)
        .await
        .expect("query count");
    assert_eq!(query_count, 0, "Query is not a versioned-content service");

    // list_contributions agrees with the count for the Ehr scope.
    let listed = svc
        .admin_list_contributions(PlatformService::Ehr, None)
        .await
        .expect("list contributions");
    assert_eq!(i64::try_from(listed.len()).unwrap(), ehr_count);

    // Composition statistics: 2 versioned compositions, 3 version rows (Ehr);
    // a non-Ehr service sees neither.
    assert_eq!(
        svc.versioned_composition_count(PlatformService::Ehr, None)
            .await
            .expect("versioned comp count"),
        2
    );
    assert_eq!(
        svc.composition_version_count(PlatformService::Ehr, None)
            .await
            .expect("comp version count"),
        3
    );
    assert_eq!(
        svc.versioned_composition_count(PlatformService::Demographic, None)
            .await
            .expect("demo versioned comp"),
        0
    );

    // Time range: an upper bound before everything → 0; a lower bound in the
    // future → 0; open bounds → all.
    let past = Some((None, Some("2000-01-01T00:00:00Z".to_owned())));
    assert_eq!(
        svc.admin_contribution_count(PlatformService::Ehr, past)
            .await
            .expect("past range"),
        0
    );
    let future = Some((Some("2999-01-01T00:00:00Z".to_owned()), None));
    assert_eq!(
        svc.admin_contribution_count(PlatformService::Ehr, future)
            .await
            .expect("future range"),
        0
    );

    // An invalid ISO bound → 400 (validated at the adapter before the query).
    let bad = svc
        .admin_contribution_count(
            PlatformService::Ehr,
            Some((Some("not-a-date".to_owned()), None)),
        )
        .await;
    assert!(
        matches!(
            bad,
            Err(SmError {
                status: CallStatusType::PreconditionViolation,
                ..
            })
        ),
        "invalid ISO bound must be BadRequest, got {bad:?}"
    );
}

#[tokio::test]
async fn physical_party_delete_cascades_relationships_and_spares_partner() {
    let pg = Pg::start().await;
    let pool = pg.migrated_pool("admin_party_delete").await;
    let svc = EhrbaseService::new(pool.clone());
    let pool = &pool;

    let p1 = make_person(&svc, "P1").await;
    let p2 = make_person(&svc, "P2").await;
    let p3 = make_person(&svc, "P3").await;

    // R1: p1 → p2 (references p1 as source). R2: p2 → p1 (references p1 as
    // target). R3: p2 → p3 (does NOT reference p1).
    let r1 = svc
        .party_relationship_create(relationship("r1", &p1, &p2))
        .await
        .expect("r1");
    let r1 = r1.body["uid"]["value"]
        .as_str()
        .unwrap()
        .split("::")
        .next()
        .unwrap()
        .to_owned();
    let r2 = svc
        .party_relationship_create(relationship("r2", &p2, &p1))
        .await
        .expect("r2");
    let r2 = r2.body["uid"]["value"]
        .as_str()
        .unwrap()
        .split("::")
        .next()
        .unwrap()
        .to_owned();
    let r3 = svc
        .party_relationship_create(relationship("r3", &p2, &p3))
        .await
        .expect("r3");
    let r3 = r3.body["uid"]["value"]
        .as_str()
        .unwrap()
        .split("::")
        .next()
        .unwrap()
        .to_owned();

    // No orphaned audits before the delete: every audit row is referenced by a
    // vo_version or contribution.
    let orphan_audits_before: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM audit a \
         WHERE NOT EXISTS (SELECT 1 FROM vo_version v WHERE v.audit_id = a.id) \
           AND NOT EXISTS (SELECT 1 FROM contribution c WHERE c.audit_id = a.id)",
    )
    .fetch_one(pool)
    .await
    .expect("orphan audits before");
    assert_eq!(orphan_audits_before, 0);

    // Delete p1 physically (SM physical_party_delete).
    svc.physical_party_delete(p1.clone())
        .await
        .expect("physical party delete");

    // p1 and both relationships referencing it are physically gone.
    assert_eq!(vo_version_rows(pool, &p1).await, 0, "p1 gone");
    assert_eq!(
        vo_version_rows(pool, &r1).await,
        0,
        "r1 (p1 as source) gone"
    );
    assert_eq!(
        vo_version_rows(pool, &r2).await,
        0,
        "r2 (p1 as target) gone"
    );

    // The partner party p2 and the unrelated relationship r3 survive.
    assert!(vo_version_rows(pool, &p2).await > 0, "partner p2 survives");
    assert!(
        vo_version_rows(pool, &r3).await > 0,
        "unrelated r3 survives"
    );
    assert!(
        svc.party_get(PartyKind::Person, p2.clone(), None)
            .await
            .expect("p2 still readable")
            .body["_type"]
            == "PERSON"
    );

    // No orphaned audit rows were left behind (audits swept in the cascade).
    let orphan_audits_after: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM audit a \
         WHERE NOT EXISTS (SELECT 1 FROM vo_version v WHERE v.audit_id = a.id) \
           AND NOT EXISTS (SELECT 1 FROM contribution c WHERE c.audit_id = a.id)",
    )
    .fetch_one(pool)
    .await
    .expect("orphan audits after");
    assert_eq!(orphan_audits_after, 0, "no orphaned audits after cascade");

    // An unknown party id → 404 (party_id_does_not_exist).
    let unknown = svc.physical_party_delete(Uuid::now_v7().to_string()).await;
    assert!(
        matches!(
            unknown,
            Err(SmError {
                status: CallStatusType::VersionedObjectDoesNotExist,
                ..
            })
        ),
        "unknown party must be NotFound, got {unknown:?}"
    );

    // A malformed id → 400.
    let bad = svc.physical_party_delete("not-a-uuid".to_owned()).await;
    assert!(matches!(
        bad,
        Err(SmError {
            status: CallStatusType::PreconditionViolation,
            ..
        })
    ));
}

#[tokio::test]
async fn archive_marks_vos_idempotently_and_reads_stay_unchanged() {
    let pg = Pg::start().await;
    let pool = pg.migrated_pool("admin_archive").await;
    let svc = EhrbaseService::new(pool.clone());
    let pool = &pool;

    let ehr = seed_full_ehr(&svc).await;
    let person = make_person(&svc, "Archie").await;

    // archive_ehrs marks every VO of the EHR.
    svc.archive_ehrs(vec![ehr.to_string()])
        .await
        .expect("archive ehr");
    let ehr_vo_count: i64 =
        sqlx::query_scalar("SELECT count(DISTINCT vo_id) FROM vo_version WHERE ehr_id = $1")
            .bind(ehr)
            .fetch_one(pool)
            .await
            .expect("ehr vo count");
    let archived: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM vo_archive va \
         WHERE EXISTS (SELECT 1 FROM vo_version v WHERE v.vo_id = va.vo_id AND v.ehr_id = $1)",
    )
    .bind(ehr)
    .fetch_one(pool)
    .await
    .expect("archived count");
    assert_eq!(archived, ehr_vo_count, "every EHR VO is marked archived");

    // Idempotent: a second archive_ehrs adds nothing and does not error.
    svc.archive_ehrs(vec![ehr.to_string()])
        .await
        .expect("re-archive ehr");
    let archived_again: i64 = sqlx::query_scalar("SELECT count(*) FROM vo_archive")
        .fetch_one(pool)
        .await
        .expect("total archived");
    assert_eq!(archived_again, ehr_vo_count, "re-archive is idempotent");

    // archive_parties marks the party VO.
    svc.archive_parties(vec![person.clone()])
        .await
        .expect("archive party");
    let party_marked: i64 =
        sqlx::query_scalar("SELECT count(*) FROM vo_archive WHERE vo_id = $1::uuid")
            .bind(&person)
            .fetch_one(pool)
            .await
            .expect("party archived");
    assert_eq!(party_marked, 1);

    // Reads are UNCHANGED after archival (zero wire drift): the EHR_STATUS and
    // the party still serve their content.
    let status = svc
        .get_ehr_status_at_time(ehr, None)
        .await
        .expect("status still readable after archive");
    assert_eq!(status["_type"], "EHR_STATUS");
    let party = svc
        .party_get(PartyKind::Person, person.clone(), None)
        .await
        .expect("party still readable after archive");
    assert_eq!(party.body["_type"], "PERSON");

    // All-or-nothing: a batch with one unknown EHR → 404 and nothing new is
    // archived (a fresh EHR paired with a bogus id stays unmarked).
    let fresh = seed_full_ehr(&svc).await;
    let before: i64 = sqlx::query_scalar("SELECT count(*) FROM vo_archive")
        .fetch_one(pool)
        .await
        .expect("before");
    let res = svc
        .archive_ehrs(vec![fresh.to_string(), Uuid::now_v7().to_string()])
        .await;
    assert!(
        matches!(
            res,
            Err(SmError {
                status: CallStatusType::VersionedObjectDoesNotExist,
                ..
            })
        ),
        "an unknown EHR aborts the batch, got {res:?}"
    );
    let after: i64 = sqlx::query_scalar("SELECT count(*) FROM vo_archive")
        .fetch_one(pool)
        .await
        .expect("after");
    assert_eq!(before, after, "all-or-nothing: nothing archived on failure");

    // Unknown party → 404.
    let bad_party = svc.archive_parties(vec![Uuid::now_v7().to_string()]).await;
    assert!(matches!(
        bad_party,
        Err(SmError {
            status: CallStatusType::VersionedObjectDoesNotExist,
            ..
        })
    ));
}
