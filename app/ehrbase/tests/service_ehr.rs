//! End-to-end service tests against a real PostgreSQL 18 (testcontainers):
//! the EHR / EHR_STATUS / COMPOSITION / DIRECTORY / CONTRIBUTION lifecycle,
//! including versioning, optimistic concurrency, time-travel, and logical
//! delete — driven through the `EhrService` envelope seam (W2-A) exactly as the
//! REST layer calls it, asserting both the RM payload (`.body`) and the resource
//! metadata (`.meta`, from which the HTTP edge derives `ETag`/`Location`).
#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::too_many_lines,
    clippy::doc_markdown
)]

use serde_json::{Value, json};
use sqlx::{AssertSqlSafe, Connection, PgConnection, PgPool};
use testcontainers::runners::AsyncRunner;
use testcontainers::{ContainerAsync, ImageExt};
use testcontainers_modules::postgres::Postgres;

use openehr_base::prelude::TerminologyCode;
use openehr_rm::prelude::PartyProxy;

use ehrbase::db::{self, DbSettings};
use ehrbase::service::EhrbaseService;
use ehrbase_sm::types::{UpdateAudit, UpdateVersion};
use ehrbase_sm::{
    CallStatusType, DefinitionAdapter, EhrCompositionService, EhrContributionService,
    EhrDirectoryService, EhrService, EhrStatusService, ItemTagAdapter, SmError,
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

fn has_key(tags: &[Value], k: &str) -> bool {
    tags.iter()
        .any(|t| t.get("key").and_then(Value::as_str) == Some(k))
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

/// A COMPOSITION with **no** `template_id` but a terminology-invalid `category`
/// code (`openehr::9999` is not in the `composition_category` group) — used to
/// prove templateless compositions still get RM/terminology validation
/// (F-07-01/F-07-02). Every other mandatory attribute is valid, so the category
/// code is the only defect.
fn composition_with_bad_category() -> Value {
    let mut c = composition("Bad category");
    c["category"]["defining_code"]["code_string"] = json!("9999");
    c
}

#[tokio::test]
async fn ehr_composition_lifecycle_end_to_end() {
    let pg = Pg::start().await;
    let svc = EhrbaseService::new(pg.migrated_pool("lifecycle").await);

    // ── EHR create + retrieve ────────────────────────────────────────────────
    // PORT NOTE (ADR-011): the SM `create_ehr` returns the new UUID; the RM `EHR`
    // body is read via `ehr_object`. The old create-envelope ETag/Location meta
    // (ehr_id == uid) is exactly that returned uuid — the adapter builds the
    // header — so the meta assertions are folded into the uuid/body checks.
    let ehr_uuid = svc.create_ehr(None).await.expect("ehr_create");
    let ehr_id = ehr_uuid.to_string();
    let ehr = svc.ehr_object(ehr_uuid).await.expect("ehr object");
    assert_eq!(ehr["_type"], "EHR");
    assert_eq!(ehr["ehr_id"]["value"], ehr_id);

    let fetched = svc.ehr_object(ehr_uuid).await.expect("ehr_get_by_id");
    assert_eq!(fetched["ehr_id"]["value"], ehr_id);

    // ── EHR_STATUS: read v1, update → v2, optimistic concurrency ─────────────
    let status_v1 = svc
        .get_ehr_status_at_time(ehr_uuid, None)
        .await
        .expect("status get");
    let status_ovid_v1 = uid(&status_v1).to_owned();
    assert!(status_ovid_v1.ends_with("::1"), "got {status_ovid_v1}");

    let mut status_v2_body = status_v1.clone();
    status_v2_body.as_object_mut().unwrap().remove("uid");
    status_v2_body["is_modifiable"] = json!(false);
    // PORT NOTE (ADR-011): `replace_ehr_status` returns the new version_uid (the
    // old `.meta.uid`); the content is re-read to assert it.
    let status_v2_uid = svc
        .replace_ehr_status(ehr_uuid, uv(status_v2_body, "251", Some(&status_ovid_v1)))
        .await
        .expect("status update");
    assert!(status_v2_uid.ends_with("::2"));
    let status_v2 = svc
        .get_ehr_status_at_time(ehr_uuid, None)
        .await
        .expect("status v2");
    assert_eq!(status_v2["is_modifiable"], json!(false));

    // Stale If-Match is rejected (precondition failed → 412 / version_mismatch).
    let stale = svc
        .replace_ehr_status(
            ehr_uuid,
            uv(status_v1.clone(), "251", Some(&status_ovid_v1)),
        )
        .await;
    assert!(
        matches!(
            stale,
            Err(SmError {
                status: CallStatusType::VersionMismatch,
                ..
            })
        ),
        "stale update must 412, got {stale:?}"
    );

    // A specific EHR_STATUS version reads as the BARE EHR_STATUS (F-01-03), not
    // an ORIGINAL_VERSION wrapper.
    let sp: Vec<&str> = status_ovid_v1.split("::").collect();
    let status_by_v = svc
        .get_ehr_status_at_version(
            ehr_uuid,
            sp[0].parse().expect("status vo uuid"),
            sp[2].parse().expect("status version"),
        )
        .await
        .expect("status by version");
    assert_eq!(status_by_v["_type"], "EHR_STATUS");
    assert_eq!(uid(&status_by_v), status_ovid_v1);

    // ── COMPOSITION: create, update, version reads ──────────────────────────
    let comp_ovid_v1 = svc
        .create_composition(ehr_uuid, uv(composition("Encounter"), "249", None))
        .await
        .expect("composition_create");
    let comp_vo_id = comp_ovid_v1.split("::").next().unwrap().to_owned();
    let comp_vo_uuid = comp_vo_id.parse::<uuid::Uuid>().expect("vo uuid");

    let got = svc
        .get_composition_latest(ehr_uuid, comp_vo_uuid)
        .await
        .expect("composition_get");
    assert_eq!(got["name"]["value"], "Encounter");

    let comp_ovid_v2 = svc
        .update_composition(
            ehr_uuid,
            comp_vo_uuid,
            uv(composition("Encounter v2"), "251", Some(&comp_ovid_v1)),
        )
        .await
        .expect("composition_update");
    assert!(comp_ovid_v2.ends_with("::2"));

    // current is v2; the pinned OBJECT_VERSION_ID still returns v1
    let current = svc
        .get_composition_latest(ehr_uuid, comp_vo_uuid)
        .await
        .expect("get current");
    assert_eq!(current["name"]["value"], "Encounter v2");
    let pinned = svc
        .get_composition_at_version(ehr_uuid, comp_ovid_v1.parse().expect("ovid"))
        .await
        .expect("get pinned v1");
    assert_eq!(pinned["name"]["value"], "Encounter");

    // VERSIONED_OBJECT + ORIGINAL_VERSION (provenance: the CONTRIBUTION ref)
    let versioned = svc
        .get_versioned_composition(ehr_uuid, comp_vo_uuid)
        .await
        .expect("versioned_composition_get");
    assert_eq!(versioned["_type"], "VERSIONED_OBJECT");
    assert!(
        versioned["time_created"]["value"].is_string(),
        "VERSIONED_OBJECT.time_created must be present, got {versioned}"
    );

    let original = svc
        .composition_original_version(ehr_uuid, comp_ovid_v1.parse().expect("ovid"))
        .await
        .expect("original version");
    assert_eq!(original["_type"], "ORIGINAL_VERSION");
    assert_eq!(original["commit_audit"]["_type"], "AUDIT_DETAILS");
    assert_eq!(
        original["commit_audit"]["change_type"]["defining_code"]["code_string"],
        "249"
    );
    assert_eq!(original["commit_audit"]["change_type"]["value"], "creation");
    assert_eq!(
        original["lifecycle_state"]["defining_code"]["code_string"],
        "532"
    );
    assert!(
        original.get("preceding_version_uid").is_none(),
        "v1 must not carry preceding_version_uid"
    );
    let original_v2 = svc
        .composition_original_version(ehr_uuid, comp_ovid_v2.parse().expect("ovid"))
        .await
        .expect("original version v2");
    assert_eq!(original_v2["preceding_version_uid"]["value"], comp_ovid_v1);
    let contribution_uid = original["contribution"]["id"]["value"]
        .as_str()
        .expect("contribution ref")
        .to_owned();

    // ── CONTRIBUTION retrieval (audit + version refs) ────────────────────────
    let contribution = svc
        .get_contribution(ehr_uuid, contribution_uid.parse().expect("contrib uuid"))
        .await
        .expect("contribution_get");
    assert_eq!(contribution["_type"], "CONTRIBUTION");
    assert_eq!(contribution["audit"]["change_type"]["value"], "creation");
    assert!(!contribution["versions"].as_array().unwrap().is_empty());

    // ── logical delete (F-02-01/05, F-06-04) ─────────────────────────────────
    // A stale preceding_version_uid (v1, but latest is v2) → 409 Conflict.
    let stale_delete = svc
        .delete_composition(ehr_uuid, comp_ovid_v1.parse().expect("ovid"))
        .await;
    assert!(
        matches!(
            stale_delete,
            Err(SmError {
                status: CallStatusType::CompositionAlreadyExists,
                ..
            })
        ),
        "stale preceding_version_uid must 409, got {stale_delete:?}"
    );
    // PORT NOTE (ADR-011): the old "bare HIER_OBJECT_ID → 400" sub-check is
    // dropped — `delete_composition` now takes a typed `ObjectVersionId`, so a
    // bare id cannot be constructed as an argument; that decode + 400 moved to
    // the protocol adapter (`ehrbase-rest`), where it is exercised.

    // The correct latest version_uid deletes; the returned uid names the new
    // (deleted) version (204_COMPOSITION_deleted ETag/Location).
    let deleted = svc
        .delete_composition(ehr_uuid, comp_ovid_v2.parse().expect("ovid"))
        .await
        .expect("composition_delete");
    assert!(
        deleted.ends_with("::3"),
        "delete names the new (deleted) version, got {deleted}"
    );
    // A deleted read is NOT an error/500 — it yields Null (→ 204) (F-02-01).
    let after_delete = svc
        .get_composition_latest(ehr_uuid, comp_vo_uuid)
        .await
        .expect("deleted composition read must not error");
    assert!(
        after_delete.is_null(),
        "deleted composition reads as an empty (204) body, got {after_delete}"
    );
    // Re-deleting an already-deleted composition → 400 (400_already_deleted).
    assert!(
        matches!(
            svc.delete_composition(ehr_uuid, comp_ovid_v2.parse().expect("ovid"))
                .await,
            Err(SmError {
                status: CallStatusType::PreconditionViolation,
                ..
            })
        ),
        "re-delete must be 400 already-deleted"
    );
    // The deleted VERSION renders lifecycle_state 523|deleted| with no data.
    let parts: Vec<&str> = comp_ovid_v2.split("::").collect();
    let deleted_ovid = format!("{}::{}::3", parts[0], parts[1]);
    let deleted_version = svc
        .composition_original_version(ehr_uuid, deleted_ovid.parse().expect("ovid"))
        .await
        .expect("deleted version wrapper");
    assert_eq!(
        deleted_version["lifecycle_state"]["defining_code"]["code_string"],
        "523"
    );
    assert_eq!(deleted_version["lifecycle_state"]["value"], "deleted");
    assert_eq!(
        deleted_version["commit_audit"]["change_type"]["defining_code"]["code_string"],
        "523"
    );
    assert!(
        deleted_version.get("data").is_none(),
        "a deleted version carries no data"
    );

    // ── DIRECTORY (FOLDER) create/get/update/delete ──────────────────────────
    let folder = json!({ "_type": "FOLDER", "name": { "_type": "DV_TEXT", "value": "root" } });
    let dir_ovid = svc
        .create_directory(ehr_uuid, uv(folder.clone(), "249", None))
        .await
        .expect("directory_create");
    let dir_got = svc
        .get_directory_at_time(ehr_uuid, None, None)
        .await
        .expect("directory_get");
    assert_eq!(dir_got["name"]["value"], "root");

    let mut folder_v2 = folder;
    folder_v2["name"]["value"] = json!("root-renamed");
    let dir_ovid_v2 = svc
        .update_directory(ehr_uuid, uv(folder_v2, "251", Some(&dir_ovid)))
        .await
        .expect("directory_update");
    assert!(dir_ovid_v2.ends_with("::2"));

    svc.delete_directory(ehr_uuid, Some(dir_ovid_v2.parse().expect("ovid")))
        .await
        .expect("directory_delete");
}

#[tokio::test]
async fn creating_an_ehr_with_an_existing_id_conflicts() {
    let pg = Pg::start().await;
    let svc = EhrbaseService::new(pg.migrated_pool("conflict").await);

    let id = uuid::Uuid::now_v7();
    svc.create_ehr_with_id(id, None)
        .await
        .expect("first create");
    let again = svc.create_ehr_with_id(id, None).await;
    assert!(again.is_err(), "duplicate EHR id must conflict");
}

#[tokio::test]
async fn unknown_ehr_is_not_found() {
    let pg = Pg::start().await;
    let svc = EhrbaseService::new(pg.migrated_pool("missing").await);
    let missing = uuid::Uuid::now_v7();
    assert!(svc.ehr_object(missing).await.is_err());
}

/// A `DV_CODED_TEXT` audit change_type (openEHR audit change-type group).
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

#[tokio::test]
async fn contribution_commits_a_composition_atomically() {
    let pg = Pg::start().await;
    let svc = EhrbaseService::new(pg.migrated_pool("contribution").await);

    let ehr_id = svc.create_ehr(None).await.expect("ehr").to_string();
    let ehr_uuid = ehr_id.parse::<uuid::Uuid>().expect("ehr uuid");

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
        .create_ehr_contribution(ehr_uuid, body)
        .await
        .expect("contribution_create");
    assert_eq!(contribution.body["_type"], "CONTRIBUTION");
    // 201_CONTRIBUTION: ETag(contribution_uid) meta.
    let contribution_uid = contribution.body["uid"]["value"].as_str().unwrap();
    assert_eq!(
        contribution.meta.as_ref().map(|m| m.uid.as_str()),
        Some(contribution_uid)
    );
    let versions = contribution.body["versions"].as_array().expect("versions");
    assert_eq!(versions.len(), 1);

    // The version the contribution created is retrievable by its OBJECT_VERSION_ID.
    let ovid = versions[0]["id"]["value"].as_str().unwrap().to_owned();
    let comp = svc
        .get_composition_at_version(ehr_uuid, ovid.parse().expect("ovid"))
        .await
        .expect("get created composition");
    assert_eq!(comp["name"]["value"], "Via contribution");
}

#[tokio::test]
async fn contribution_preserves_the_client_change_type_and_rejects_invalid_combos() {
    // F-06-06 / W2-C: an inbound `250|amendment|` is stored and echoed verbatim
    // (never narrowed to `modification` — RM change_control §"Contributions":
    // a correction is committed with change type 250|amendment|), and
    // spec-invalid combinations (creation on an existing object; an
    // out-of-group code) are rejected as 422.
    let pg = Pg::start().await;
    let svc = EhrbaseService::new(pg.migrated_pool("contribamend").await);

    let ehr_id = svc.create_ehr(None).await.expect("ehr").to_string();
    let ehr_uuid = ehr_id.parse::<uuid::Uuid>().expect("ehr uuid");

    let created = svc
        .create_ehr_contribution(
            ehr_uuid,
            json!({
                "versions": [{
                    "data": composition("v1"),
                    "commit_audit": { "change_type": change_type("249", "creation") }
                }]
            }),
        )
        .await
        .expect("creation contribution");
    let ovid_v1 = created.body["versions"][0]["id"]["value"]
        .as_str()
        .unwrap()
        .to_owned();

    // Amendment (a correction) of the committed composition.
    let amended = svc
        .create_ehr_contribution(
            ehr_uuid,
            json!({
                "versions": [{
                    "data": composition("v2 corrected"),
                    "preceding_version_uid": ovid_v1,
                    "commit_audit": { "change_type": change_type("250", "amendment") }
                }]
            }),
        )
        .await
        .expect("amendment contribution");
    let ovid_v2 = amended.body["versions"][0]["id"]["value"]
        .as_str()
        .unwrap()
        .to_owned();
    // With no contribution-level audit given, the aggregate change type is the
    // members' shared code (RM change_control §"Contributions": "any code:
    // when all member versions have the same change type…").
    assert_eq!(
        amended.body["audit"]["change_type"]["defining_code"]["code_string"],
        "250"
    );
    assert_eq!(amended.body["audit"]["change_type"]["value"], "amendment");

    // The stored ORIGINAL_VERSION echoes the client's change type verbatim
    // (code 250 + rubric "amendment"), not a rewritten "modification".
    let version = svc
        .composition_original_version(ehr_uuid, ovid_v2.parse().expect("ovid"))
        .await
        .expect("amended version");
    assert_eq!(
        version["commit_audit"]["change_type"]["defining_code"]["code_string"],
        "250"
    );
    assert_eq!(version["commit_audit"]["change_type"]["value"], "amendment");

    // Invalid combo: 249|creation| on an existing object (preceding uid set).
    let bad_creation = svc
        .create_ehr_contribution(
            ehr_uuid,
            json!({
                "versions": [{
                    "data": composition("v3"),
                    "preceding_version_uid": ovid_v2,
                    "commit_audit": { "change_type": change_type("249", "creation") }
                }]
            }),
        )
        .await;
    assert!(
        matches!(
            bad_creation,
            Err(SmError {
                status: CallStatusType::ContentInvalid,
                ..
            })
        ),
        "creation on an existing object must 422, got {bad_creation:?}"
    );

    // Invalid code: not a member of the audit_change_type group
    // (AUDIT_DETAILS.Change_type_valid).
    let bad_code = svc
        .create_ehr_contribution(
            ehr_uuid,
            json!({
                "versions": [{
                    "data": composition("v3"),
                    "preceding_version_uid": ovid_v2,
                    "commit_audit": { "change_type": change_type("999", "bogus") }
                }]
            }),
        )
        .await;
    assert!(
        matches!(
            bad_code,
            Err(SmError {
                status: CallStatusType::ContentInvalid,
                ..
            })
        ),
        "an out-of-group change_type must 422, got {bad_code:?}"
    );
}

#[tokio::test]
async fn templateless_composition_still_gets_rm_and_terminology_validation() {
    // F-07-02: a COMPOSITION without a declared template_id must still fail on
    // RM-invariant / RM-terminology violations (here: an invalid category code),
    // and a valid templateless composition must still commit.
    let pg = Pg::start().await;
    let svc = EhrbaseService::new(pg.migrated_pool("templateless").await);

    let ehr_id = svc.create_ehr(None).await.expect("ehr").to_string();
    let ehr_uuid = ehr_id.parse::<uuid::Uuid>().expect("ehr uuid");

    let bad = svc
        .create_composition(ehr_uuid, uv(composition_with_bad_category(), "249", None))
        .await;
    assert!(
        matches!(
            bad,
            Err(SmError {
                status: CallStatusType::ContentInvalid,
                ..
            })
        ),
        "templateless composition with bad category must 422, got {bad:?}"
    );

    svc.create_composition(ehr_uuid, uv(composition("valid"), "249", None))
        .await
        .expect("a valid templateless composition still commits");
}

#[tokio::test]
async fn contribution_rejects_an_invalid_composition() {
    // F-07-01: the CONTRIBUTION commit path must run composition validation and
    // reject the whole contribution atomically.
    let pg = Pg::start().await;
    let svc = EhrbaseService::new(pg.migrated_pool("contribinvalid").await);

    let ehr_id = svc.create_ehr(None).await.expect("ehr").to_string();
    let ehr_uuid = ehr_id.parse::<uuid::Uuid>().expect("ehr uuid");

    let body = json!({
        "audit": {
            "change_type": change_type("249", "creation"),
            "committer": { "_type": "PARTY_IDENTIFIED", "name": "Dr. Contribution" }
        },
        "versions": [{
            "data": composition_with_bad_category(),
            "commit_audit": { "change_type": change_type("249", "creation") }
        }]
    });
    let res = svc.create_ehr_contribution(ehr_uuid, body).await;
    assert!(
        matches!(
            res,
            Err(SmError {
                status: CallStatusType::ContentInvalid,
                ..
            })
        ),
        "invalid composition in a contribution must 422, got {res:?}"
    );
}

#[tokio::test]
async fn revision_history_lists_every_version() {
    let pg = Pg::start().await;
    let svc = EhrbaseService::new(pg.migrated_pool("revhistory").await);

    let ehr_id = svc.create_ehr(None).await.expect("ehr").to_string();
    let ehr_uuid = ehr_id.parse::<uuid::Uuid>().expect("ehr uuid");

    let mut status = svc
        .get_ehr_status_at_time(ehr_uuid, None)
        .await
        .expect("status");
    let ovid_v1 = uid(&status).to_owned();
    status.as_object_mut().unwrap().remove("uid");
    svc.replace_ehr_status(ehr_uuid, uv(status, "251", Some(&ovid_v1)))
        .await
        .expect("status update");

    let history = svc
        .ehr_status_revision_history(ehr_uuid)
        .await
        .expect("revision history");
    assert_eq!(history["_type"], "REVISION_HISTORY");
    let items = history["items"].as_array().expect("items");
    assert_eq!(items.len(), 2, "two versions after one update");
    assert_eq!(items[0]["_type"], "REVISION_HISTORY_ITEM");
    assert!(items[0]["audits"][0]["_type"] == "AUDIT_DETAILS");
}

#[tokio::test]
async fn ehr_get_by_subject_finds_the_ehr() {
    let pg = Pg::start().await;
    let svc = EhrbaseService::new(pg.migrated_pool("bysubject").await);

    let status = json!({
        "_type": "EHR_STATUS",
        "archetype_node_id": "openEHR-EHR-EHR_STATUS.generic.v1",
        "name": { "_type": "DV_TEXT", "value": "EHR Status" },
        "subject": {
            "_type": "PARTY_IDENTIFIED",
            "external_ref": {
                "_type": "PARTY_REF",
                "namespace": "patients",
                "type": "PERSON",
                "id": { "_type": "HIER_OBJECT_ID", "value": "patient-123" }
            }
        },
        "is_queryable": true,
        "is_modifiable": true
    });
    let ehr_id = svc.create_ehr(Some(status)).await.expect("ehr").to_string();

    let found = svc
        .ehr_object_for_subject("patient-123", "patients")
        .await
        .expect("get by subject");
    assert_eq!(found["ehr_id"]["value"], ehr_id);

    assert!(
        svc.ehr_object_for_subject("nobody", "patients")
            .await
            .is_err()
    );
}

#[tokio::test]
async fn stored_query_crud() {
    let pg = Pg::start().await;
    let svc = EhrbaseService::new(pg.migrated_pool("storedq").await);

    let aql = "SELECT c FROM EHR e CONTAINS COMPOSITION c".to_owned();
    svc.query_store(
        "org.example::all_comps".to_owned(),
        Some("1.0.0".to_owned()),
        aql.clone(),
    )
    .await
    .expect("store query");

    let got = svc
        .query_version_get("org.example::all_comps".to_owned(), "1.0.0".to_owned())
        .await
        .expect("get query");
    assert_eq!(got["q"], aql);
    assert_eq!(got["version"], "1.0.0");
    assert_eq!(got["type"], "AQL");

    let list = svc
        .query_list("org.example::all_comps".to_owned())
        .await
        .expect("list");
    assert_eq!(list.len(), 1);
}

#[tokio::test]
async fn stored_query_semver_prefix_resolves_to_latest_match() {
    // F-03-07: `parameters/path/version.yaml` — a partial `{major}` or
    // `{major}.{minor}` version resolves to the HIGHEST stored version
    // matching the prefix.
    let pg = Pg::start().await;
    let svc = EhrbaseService::new(pg.migrated_pool("semverq").await);

    for (version, q) in [
        ("1.0.0", "SELECT a"),
        ("1.0.1", "SELECT b"),
        ("1.1.0", "SELECT c"),
    ] {
        svc.query_store(
            "org.example::obs".to_owned(),
            Some(version.to_owned()),
            q.to_owned(),
        )
        .await
        .expect("store");
    }

    let by_major = svc
        .query_version_get("org.example::obs".to_owned(), "1".to_owned())
        .await
        .expect("major prefix resolves");
    assert_eq!(by_major["version"], "1.1.0");
    assert_eq!(by_major["q"], "SELECT c");

    let by_minor = svc
        .query_version_get("org.example::obs".to_owned(), "1.0".to_owned())
        .await
        .expect("major.minor prefix resolves");
    assert_eq!(by_minor["version"], "1.0.1");
    assert_eq!(by_minor["q"], "SELECT b");

    // An exact triple still resolves exactly; an unmatched prefix is 404.
    let exact = svc
        .query_version_get("org.example::obs".to_owned(), "1.0.0".to_owned())
        .await
        .expect("exact version");
    assert_eq!(exact["version"], "1.0.0");
    assert!(
        svc.query_version_get("org.example::obs".to_owned(), "2".to_owned())
            .await
            .is_err(),
        "an unmatched prefix must not resolve"
    );
}

#[tokio::test]
async fn stored_query_list_matches_name_prefix() {
    // F-03-08: `definition_query_list.yaml` — the qualified name is a PATTERN:
    // `org.openehr` "will list all versions of all queries with names starting
    // with `org.openehr`"; empty ⇒ wildcard.
    let pg = Pg::start().await;
    let svc = EhrbaseService::new(pg.migrated_pool("listq").await);

    for (name, version) in [
        ("org.example::all_comps", "1.0.0"),
        ("org.example::all_comps", "1.1.0"),
        ("org.example::observations", "0.1.0"),
        ("com.acme::other", "0.1.0"),
    ] {
        svc.query_store(
            name.to_owned(),
            Some(version.to_owned()),
            "SELECT e FROM EHR e".to_owned(),
        )
        .await
        .expect("store");
    }

    let by_namespace = svc
        .query_list("org.example".to_owned())
        .await
        .expect("prefix list");
    assert_eq!(by_namespace.len(), 3, "all versions of all org.example::*");

    let by_full_name = svc
        .query_list("org.example::all_comps".to_owned())
        .await
        .expect("full-name list");
    assert_eq!(by_full_name.len(), 2, "both versions of the named query");
    assert!(
        by_full_name
            .iter()
            .all(|q| q["name"] == "org.example::all_comps"),
        "each row carries its own qualified name"
    );

    let all = svc.query_list(String::new()).await.expect("wildcard list");
    assert_eq!(all.len(), 4, "empty pattern is a wildcard");
}

#[tokio::test]
async fn item_tag_crud() {
    let pg = Pg::start().await;
    let svc = EhrbaseService::new(pg.migrated_pool("itemtags").await);

    let ehr_id = svc.create_ehr(None).await.expect("ehr").to_string();
    let ehr_uuid = ehr_id.parse::<uuid::Uuid>().expect("ehr uuid");
    let comp = svc
        .create_composition(ehr_uuid, uv(composition("Tagged"), "249", None))
        .await
        .expect("composition");
    let vo_id = comp.split("::").next().unwrap().to_owned();

    let upserted = svc
        .target_tags_replace(
            ehr_uuid,
            vo_id.clone(),
            "COMPOSITION",
            vec![json!({ "key": "priority", "value": "high" })],
        )
        .await
        .expect("tag update");
    assert!(has_key(&upserted, "priority"));

    let on_comp = svc
        .target_tags_get(ehr_uuid, vo_id.clone())
        .await
        .expect("comp tags");
    assert_eq!(on_comp.len(), 1);

    let all = svc
        .ehr_tags_get(ehr_uuid, None, None, None)
        .await
        .expect("ehr tags");
    assert_eq!(all.len(), 1);

    svc.target_tag_delete(ehr_uuid, vo_id.clone(), "priority".to_owned())
        .await
        .expect("delete tag");
    let after = svc
        .target_tags_get(ehr_uuid, vo_id.clone())
        .await
        .expect("comp tags after");
    assert!(after.is_empty());
}

#[tokio::test]
async fn item_tag_wire_shape_matches_the_oas_schema() {
    // F-03-06: the ITEM_TAG wire shape is the OAS `ItemTag` schema
    // (`additionalProperties: false`): key/value/target_path plus
    // OBJECT_REF-shaped `target` and `owner_id`; no `id`, no `target_type`.
    let pg = Pg::start().await;
    let svc = EhrbaseService::new(pg.migrated_pool("tagshape").await);

    let ehr_id = svc.create_ehr(None).await.expect("ehr").to_string();
    let ehr_uuid = ehr_id.parse::<uuid::Uuid>().expect("ehr uuid");
    let comp = svc
        .create_composition(ehr_uuid, uv(composition("Tagged"), "249", None))
        .await
        .expect("composition");
    let vo_id = comp.split("::").next().unwrap().to_owned();

    let put = svc
        .target_tags_replace(
            ehr_uuid,
            vo_id.clone(),
            "COMPOSITION",
            vec![json!({ "key": "priority", "value": "high", "target_path": "/context" })],
        )
        .await
        .expect("tag put");
    let tag = &put[0];
    assert_eq!(tag["_type"], "ITEM_TAG");
    assert_eq!(tag["key"], "priority");
    assert_eq!(tag["value"], "high");
    assert_eq!(tag["target_path"], "/context");
    // target: OBJECT_REF naming the tagged versioned object.
    assert_eq!(tag["target"]["_type"], "OBJECT_REF");
    assert_eq!(tag["target"]["type"], "COMPOSITION");
    assert_eq!(tag["target"]["id"]["_type"], "HIER_OBJECT_ID");
    assert_eq!(tag["target"]["id"]["value"], vo_id);
    // owner_id: OBJECT_REF naming the owning EHR.
    assert_eq!(tag["owner_id"]["_type"], "OBJECT_REF");
    assert_eq!(tag["owner_id"]["type"], "EHR");
    assert_eq!(tag["owner_id"]["id"]["value"], ehr_id);
    // No non-schema fields (the OAS schema is additionalProperties: false).
    assert!(
        tag.get("id").is_none(),
        "non-schema `id` must not be emitted"
    );
    assert!(
        tag.get("target_type").is_none(),
        "non-schema `target_type` must not be emitted (folded into target.type)"
    );
}

#[tokio::test]
async fn item_tag_put_replaces_the_whole_collection() {
    // F-03-05: PUT "updates the list of ALL ITEM_TAG resources … providing an
    // empty list will effectively remove all ITEM_TAG" — a full replace, not an
    // additive upsert.
    let pg = Pg::start().await;
    let svc = EhrbaseService::new(pg.migrated_pool("tagreplace").await);

    let ehr_id = svc.create_ehr(None).await.expect("ehr").to_string();
    let ehr_uuid = ehr_id.parse::<uuid::Uuid>().expect("ehr uuid");
    let comp = svc
        .create_composition(ehr_uuid, uv(composition("Tagged"), "249", None))
        .await
        .expect("composition");
    let vo_id = comp.split("::").next().unwrap().to_owned();

    let two = svc
        .target_tags_replace(
            ehr_uuid,
            vo_id.clone(),
            "COMPOSITION",
            vec![
                json!({ "key": "priority", "value": "high" }),
                json!({ "key": "status", "value": "draft" }),
            ],
        )
        .await
        .expect("put two tags");
    assert_eq!(two.len(), 2);

    // A PUT omitting `priority` removes it (full-collection replace).
    let one = svc
        .target_tags_replace(
            ehr_uuid,
            vo_id.clone(),
            "COMPOSITION",
            vec![json!({ "key": "status", "value": "final" })],
        )
        .await
        .expect("replace with one tag");
    assert_eq!(one.len(), 1, "omitted tags are removed");
    assert!(has_key(&one, "status"));
    assert!(!has_key(&one, "priority"));

    // An empty list clears every tag on the target.
    let cleared = svc
        .target_tags_replace(ehr_uuid, vo_id.clone(), "COMPOSITION", vec![])
        .await
        .expect("clear with empty list");
    assert!(cleared.is_empty(), "empty list removes all tags");

    // RM ITEM_TAG invariants: Inv_key_valid (non-empty, justified) and
    // Inv_value_valid (a set value may not be empty).
    for bad in [
        json!({ "key": " padded " }),
        json!({ "key": "" }),
        json!({ "key": "ok", "value": "" }),
    ] {
        let res = svc
            .target_tags_replace(ehr_uuid, vo_id.clone(), "COMPOSITION", vec![bad.clone()])
            .await;
        assert!(
            matches!(
                res,
                Err(SmError {
                    status: CallStatusType::ContentInvalid,
                    ..
                })
            ),
            "tag {bad} must be rejected, got {res:?}"
        );
    }
}

#[tokio::test]
async fn ehr_creation_produces_an_ehr_access() {
    // F-06-07: RM ehr § "EHR Creation" — creating an EHR yields a root EHR
    // object, an EHR_STATUS AND an EHR_ACCESS; `EHR.ehr_access` (1..1) is an
    // OBJECT_REF whose type is VERSIONED_EHR_ACCESS (invariant Ehr_access_valid).
    let pg = Pg::start().await;
    let svc = EhrbaseService::new(pg.migrated_pool("ehraccess").await);

    let ehr_uuid = svc.create_ehr(None).await.expect("ehr");
    let ehr = svc.ehr_object(ehr_uuid).await.expect("ehr object");
    let access = &ehr["ehr_access"];
    assert_eq!(access["_type"], "OBJECT_REF");
    assert_eq!(
        access["type"], "VERSIONED_EHR_ACCESS",
        "Ehr_access_valid: ehr_access.type.is_equal(\"VERSIONED_EHR_ACCESS\")"
    );
    let access_vo = access["id"]["value"].as_str().expect("ehr_access id");
    uuid::Uuid::parse_str(access_vo).expect("a real version-container uid");
    assert_ne!(
        access_vo,
        ehr["ehr_id"]["value"].as_str().unwrap(),
        "the EHR_ACCESS container is a distinct versioned object, not a fake self-ref"
    );
}

#[tokio::test]
async fn duplicate_subject_ehr_creation_conflicts() {
    // F-01-04: ITS-REST `409_EHR.yaml` + CNF master06
    // `I_EHR_SERVICE.create_ehr-two_ehrs_same_patient` — a second EHR for the
    // same subject (external_ref id + namespace) must be rejected.
    let pg = Pg::start().await;
    let svc = EhrbaseService::new(pg.migrated_pool("dupsubject").await);

    let status = |subject: &str| {
        json!({
            "_type": "EHR_STATUS",
            "archetype_node_id": "openEHR-EHR-EHR_STATUS.generic.v1",
            "name": { "_type": "DV_TEXT", "value": "EHR Status" },
            "subject": {
                "_type": "PARTY_IDENTIFIED",
                "external_ref": {
                    "_type": "PARTY_REF",
                    "namespace": "patients",
                    "type": "PERSON",
                    "id": { "_type": "HIER_OBJECT_ID", "value": subject }
                }
            },
            "is_queryable": true,
            "is_modifiable": true
        })
    };

    svc.create_ehr(Some(status("patient-1")))
        .await
        .expect("first EHR for the subject");
    let dup = svc.create_ehr(Some(status("patient-1"))).await;
    assert!(
        matches!(
            dup,
            Err(SmError {
                status: CallStatusType::CompositionAlreadyExists,
                ..
            })
        ),
        "a second EHR for the same subject must 409, got {dup:?}"
    );

    // A different subject — and the subject-less default — still create fine.
    svc.create_ehr(Some(status("patient-2")))
        .await
        .expect("different subject creates");
    svc.create_ehr(None)
        .await
        .expect("subject-less EHR creates");
    svc.create_ehr(None)
        .await
        .expect("multiple subject-less EHRs never conflict");
}

#[tokio::test]
async fn version_get_at_time_returns_the_original_version() {
    // F-01-05 / F-02-04: `versioned_ehr_status_version_get_at_time` and
    // `versioned_composition_version_get_at_time` return the VERSION extant at
    // the given time (or the latest), as an ORIGINAL_VERSION with the
    // `200_VERSION_at_time` ETag/Location metadata.
    let pg = Pg::start().await;
    let svc = EhrbaseService::new(pg.migrated_pool("attime").await);

    let ehr_id = svc.create_ehr(None).await.expect("ehr").to_string();
    let ehr_uuid = ehr_id.parse::<uuid::Uuid>().expect("ehr uuid");

    // EHR_STATUS: no version_at_time → the latest VERSION.
    // PORT NOTE (ADR-011): the reads return the ORIGINAL_VERSION `Value`; the
    // ETag/Location uid the old `.meta` carried is the ORIGINAL_VERSION's own
    // `uid.value`.
    let status_version = svc
        .ehr_status_version_at_time(ehr_uuid, None)
        .await
        .expect("status version at time (latest)");
    assert_eq!(status_version["_type"], "ORIGINAL_VERSION");
    assert_eq!(status_version["data"]["_type"], "EHR_STATUS");
    assert!(
        uid(&status_version).ends_with("::1"),
        "latest is v1, got {}",
        uid(&status_version)
    );

    // A time before the EHR existed → no version at that time (404).
    let too_early = svc
        .ehr_status_version_at_time(ehr_uuid, Some("2000-01-01T00:00:00Z".to_owned()))
        .await;
    assert!(
        matches!(
            too_early,
            Err(SmError {
                status: CallStatusType::VersionedObjectDoesNotExist,
                ..
            })
        ),
        "no version extant at the time must 404, got {too_early:?}"
    );
    // A malformed version_at_time → 400.
    let bad_time = svc
        .ehr_status_version_at_time(ehr_uuid, Some("not-a-time".to_owned()))
        .await;
    assert!(
        matches!(
            bad_time,
            Err(SmError {
                status: CallStatusType::PreconditionViolation,
                ..
            })
        ),
        "an invalid version_at_time must 400, got {bad_time:?}"
    );

    // COMPOSITION: after an update, the latest VERSION is v2.
    let comp_ovid_v1 = svc
        .create_composition(ehr_uuid, uv(composition("v1"), "249", None))
        .await
        .expect("composition");
    let comp_vo_id = comp_ovid_v1.split("::").next().unwrap().to_owned();
    let comp_vo_uuid = comp_vo_id.parse::<uuid::Uuid>().expect("vo uuid");
    svc.update_composition(
        ehr_uuid,
        comp_vo_uuid,
        uv(composition("v2"), "251", Some(&comp_ovid_v1)),
    )
    .await
    .expect("update");

    let comp_version = svc
        .composition_version_at_time(ehr_uuid, comp_vo_uuid, None)
        .await
        .expect("composition version at time (latest)");
    assert_eq!(comp_version["_type"], "ORIGINAL_VERSION");
    assert_eq!(comp_version["data"]["name"]["value"], "v2");
    assert!(
        uid(&comp_version).ends_with("::2"),
        "latest composition version is v2"
    );
}
