//! End-to-end service tests against a real PostgreSQL 18 (testcontainers):
//! the EHR / EHR_STATUS / COMPOSITION / DIRECTORY / CONTRIBUTION lifecycle,
//! including versioning, optimistic concurrency, time-travel, and logical
//! delete — driven through the generated `EhrApi` trait exactly as the REST
//! layer calls it.
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

use ehrbase::db::{self, DbSettings};
use ehrbase::service::EhrbaseService;
use openehr_its::rest::generated::definition::DefinitionApi;
use openehr_its::rest::generated::ehr::EhrApi;
use openehr_its::rest::runtime::ApiError;

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

/// Build a generated `*Params` from a JSON object (missing `Option` fields → `None`).
fn params<P: serde::de::DeserializeOwned>(v: Value) -> P {
    serde_json::from_value(v).expect("params")
}

fn uid(v: &Value) -> &str {
    v["uid"]["value"].as_str().expect("uid.value")
}

fn composition(name: &str) -> Value {
    json!({
        "_type": "COMPOSITION",
        "archetype_node_id": "openEHR-EHR-COMPOSITION.encounter.v1",
        "name": { "_type": "DV_TEXT", "value": name }
    })
}

/// A COMPOSITION with **no** `template_id` but a terminology-invalid `category`
/// code (`openehr::9999` is not in the `composition_category` group) — used to
/// prove templateless compositions still get RM/terminology validation
/// (F-07-01/F-07-02).
fn composition_with_bad_category() -> Value {
    json!({
        "_type": "COMPOSITION",
        "archetype_node_id": "openEHR-EHR-COMPOSITION.encounter.v1",
        "name": { "_type": "DV_TEXT", "value": "Bad category" },
        "category": {
            "_type": "DV_CODED_TEXT",
            "value": "event",
            "defining_code": {
                "_type": "CODE_PHRASE",
                "terminology_id": { "_type": "TERMINOLOGY_ID", "value": "openehr" },
                "code_string": "9999"
            }
        }
    })
}

#[tokio::test]
async fn ehr_composition_lifecycle_end_to_end() {
    let pg = Pg::start().await;
    let svc = EhrbaseService::new(pg.migrated_pool("lifecycle").await);

    // ── EHR create + retrieve ────────────────────────────────────────────────
    let ehr = svc
        .ehr_create(params(json!({})), None)
        .await
        .expect("ehr_create");
    assert_eq!(ehr["_type"], "EHR");
    let ehr_id = ehr["ehr_id"]["value"].as_str().expect("ehr_id").to_owned();

    let fetched = svc
        .ehr_get_by_id(params(json!({ "ehr_id": ehr_id })))
        .await
        .expect("ehr_get_by_id");
    assert_eq!(fetched["ehr_id"]["value"], ehr_id);

    // ── EHR_STATUS: read v1, update → v2, optimistic concurrency ─────────────
    let status_v1 = svc
        .ehr_status_get_at_time(params(json!({ "ehr_id": ehr_id })))
        .await
        .expect("status get");
    let status_ovid_v1 = uid(&status_v1).to_owned();
    assert!(status_ovid_v1.ends_with("::1"), "got {status_ovid_v1}");

    let mut status_v2_body = status_v1.clone();
    status_v2_body["is_modifiable"] = json!(false);
    let status_v2 = svc
        .ehr_status_update(
            params(json!({ "ehr_id": ehr_id, "If-Match": status_ovid_v1 })),
            status_v2_body,
        )
        .await
        .expect("status update");
    assert!(uid(&status_v2).ends_with("::2"));
    assert_eq!(status_v2["is_modifiable"], json!(false));

    // Stale If-Match is rejected (precondition failed).
    let stale = svc
        .ehr_status_update(
            params(json!({ "ehr_id": ehr_id, "If-Match": status_ovid_v1 })),
            status_v1.clone(),
        )
        .await;
    assert!(stale.is_err(), "stale update must fail");

    // ── COMPOSITION: create, update, version reads ──────────────────────────
    let comp_v1 = svc
        .composition_create(
            params(json!({ "ehr_id": ehr_id })),
            composition("Encounter"),
        )
        .await
        .expect("composition_create");
    let comp_ovid_v1 = uid(&comp_v1).to_owned();
    let comp_vo_id = comp_ovid_v1.split("::").next().unwrap().to_owned();

    let got = svc
        .composition_get(params(
            json!({ "ehr_id": ehr_id, "uid_based_id": comp_vo_id }),
        ))
        .await
        .expect("composition_get");
    assert_eq!(got["name"]["value"], "Encounter");

    let comp_v2 = svc
        .composition_update(
            params(
                json!({ "ehr_id": ehr_id, "uid_based_id": comp_vo_id, "If-Match": comp_ovid_v1 }),
            ),
            composition("Encounter v2"),
        )
        .await
        .expect("composition_update");
    assert!(uid(&comp_v2).ends_with("::2"));

    // current is v2; the pinned OBJECT_VERSION_ID still returns v1
    let current = svc
        .composition_get(params(
            json!({ "ehr_id": ehr_id, "uid_based_id": comp_vo_id }),
        ))
        .await
        .expect("get current");
    assert_eq!(current["name"]["value"], "Encounter v2");
    let pinned = svc
        .composition_get(params(
            json!({ "ehr_id": ehr_id, "uid_based_id": comp_ovid_v1 }),
        ))
        .await
        .expect("get pinned v1");
    assert_eq!(pinned["name"]["value"], "Encounter");

    // VERSIONED_OBJECT + ORIGINAL_VERSION (provenance: the CONTRIBUTION ref)
    let versioned = svc
        .versioned_composition_get(params(
            json!({ "ehr_id": ehr_id, "versioned_object_uid": comp_vo_id }),
        ))
        .await
        .expect("versioned_composition_get");
    assert_eq!(versioned["_type"], "VERSIONED_OBJECT");
    // VERSIONED_OBJECT.time_created is mandatory (1..1) — F-06-05/F-01-08.
    assert!(
        versioned["time_created"]["value"].is_string(),
        "VERSIONED_OBJECT.time_created must be present, got {versioned}"
    );

    let comp_ovid_v2 = uid(&comp_v2).to_owned();
    let original = svc
        .versioned_composition_version_get_by_id(params(json!({
            "ehr_id": ehr_id, "versioned_object_uid": comp_vo_id, "version_uid": comp_ovid_v1
        })))
        .await
        .expect("original version");
    assert_eq!(original["_type"], "ORIGINAL_VERSION");
    // F-06-01/F-01-07: mandatory commit_audit (AUDIT_DETAILS).
    assert_eq!(original["commit_audit"]["_type"], "AUDIT_DETAILS");
    // F-06-02/F-01-06: change_type carries the numeric group code, rubric in value.
    assert_eq!(
        original["commit_audit"]["change_type"]["defining_code"]["code_string"],
        "249"
    );
    assert_eq!(original["commit_audit"]["change_type"]["value"], "creation");
    // F-06-04/F-02-07: a live version → lifecycle_state 532|complete|.
    assert_eq!(
        original["lifecycle_state"]["defining_code"]["code_string"],
        "532"
    );
    // F-06-03: version 1 has NO preceding_version_uid.
    assert!(
        original.get("preceding_version_uid").is_none(),
        "v1 must not carry preceding_version_uid"
    );
    // F-06-03: version 2 DOES, and it names version 1.
    let original_v2 = svc
        .versioned_composition_version_get_by_id(params(json!({
            "ehr_id": ehr_id, "versioned_object_uid": comp_vo_id, "version_uid": comp_ovid_v2
        })))
        .await
        .expect("original version v2");
    assert_eq!(original_v2["preceding_version_uid"]["value"], comp_ovid_v1);
    let contribution_uid = original["contribution"]["id"]["value"]
        .as_str()
        .expect("contribution ref")
        .to_owned();

    // ── CONTRIBUTION retrieval (audit + version refs) ────────────────────────
    let contribution = svc
        .contribution_get(params(
            json!({ "ehr_id": ehr_id, "contribution_uid": contribution_uid }),
        ))
        .await
        .expect("contribution_get");
    assert_eq!(contribution["_type"], "CONTRIBUTION");
    assert_eq!(contribution["audit"]["change_type"]["value"], "creation");
    assert!(!contribution["versions"].as_array().unwrap().is_empty());

    // ── logical delete (F-02-01/05, F-06-04) ─────────────────────────────────
    // A stale preceding_version_uid (v1, but latest is v2) → 409 Conflict.
    let stale_delete = svc
        .composition_delete(params(
            json!({ "ehr_id": ehr_id, "uid_based_id": comp_ovid_v1 }),
        ))
        .await;
    assert!(
        matches!(stale_delete, Err(ApiError::Conflict(_))),
        "stale preceding_version_uid must 409, got {stale_delete:?}"
    );
    // A bare HIER_OBJECT_ID (no version) → 400.
    assert!(
        matches!(
            svc.composition_delete(params(
                json!({ "ehr_id": ehr_id, "uid_based_id": comp_vo_id })
            ))
            .await,
            Err(ApiError::BadRequest(_))
        ),
        "delete requires an OBJECT_VERSION_ID"
    );
    // The correct latest version_uid deletes.
    svc.composition_delete(params(
        json!({ "ehr_id": ehr_id, "uid_based_id": comp_ovid_v2 }),
    ))
    .await
    .expect("composition_delete");
    // A deleted read is NOT an error/500 — it yields a null (→ 204) body (F-02-01).
    let after_delete = svc
        .composition_get(params(
            json!({ "ehr_id": ehr_id, "uid_based_id": comp_vo_id }),
        ))
        .await
        .expect("deleted composition read must not error");
    assert!(
        after_delete.is_null(),
        "deleted composition reads as an empty (204) body, got {after_delete}"
    );
    // Re-deleting an already-deleted composition → 400 (400_already_deleted).
    assert!(
        matches!(
            svc.composition_delete(params(
                json!({ "ehr_id": ehr_id, "uid_based_id": comp_ovid_v2 })
            ))
            .await,
            Err(ApiError::BadRequest(_))
        ),
        "re-delete must be 400 already-deleted"
    );
    // The deleted VERSION renders lifecycle_state 523|deleted| with no data (F-02-07).
    let parts: Vec<&str> = comp_ovid_v2.split("::").collect();
    let deleted_ovid = format!("{}::{}::3", parts[0], parts[1]);
    let deleted_version = svc
        .versioned_composition_version_get_by_id(params(json!({
            "ehr_id": ehr_id, "versioned_object_uid": comp_vo_id, "version_uid": deleted_ovid
        })))
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
    let dir = svc
        .directory_create(params(json!({ "ehr_id": ehr_id })), folder.clone())
        .await
        .expect("directory_create");
    let dir_ovid = uid(&dir).to_owned();
    let dir_got = svc
        .directory_get_at_time(params(json!({ "ehr_id": ehr_id })))
        .await
        .expect("directory_get");
    assert_eq!(dir_got["name"]["value"], "root");

    let mut folder_v2 = folder;
    folder_v2["name"]["value"] = json!("root-renamed");
    let dir_v2 = svc
        .directory_update(
            params(json!({ "ehr_id": ehr_id, "If-Match": dir_ovid })),
            folder_v2,
        )
        .await
        .expect("directory_update");
    assert!(uid(&dir_v2).ends_with("::2"));

    svc.directory_delete(params(
        json!({ "ehr_id": ehr_id, "If-Match": uid(&dir_v2) }),
    ))
    .await
    .expect("directory_delete");
}

#[tokio::test]
async fn creating_an_ehr_with_an_existing_id_conflicts() {
    let pg = Pg::start().await;
    let svc = EhrbaseService::new(pg.migrated_pool("conflict").await);

    let id = uuid::Uuid::now_v7().to_string();
    svc.ehr_create_with_id(params(json!({ "ehr_id": id })), None)
        .await
        .expect("first create");
    let again = svc
        .ehr_create_with_id(params(json!({ "ehr_id": id })), None)
        .await;
    assert!(again.is_err(), "duplicate EHR id must conflict");
}

#[tokio::test]
async fn unknown_ehr_is_not_found() {
    let pg = Pg::start().await;
    let svc = EhrbaseService::new(pg.migrated_pool("missing").await);
    let missing = uuid::Uuid::now_v7().to_string();
    assert!(
        svc.ehr_get_by_id(params(json!({ "ehr_id": missing })))
            .await
            .is_err()
    );
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

    let ehr = svc.ehr_create(params(json!({})), None).await.expect("ehr");
    let ehr_id = ehr["ehr_id"]["value"].as_str().unwrap().to_owned();

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
    assert_eq!(contribution["_type"], "CONTRIBUTION");
    let versions = contribution["versions"].as_array().expect("versions");
    assert_eq!(versions.len(), 1);

    // The version the contribution created is retrievable by its OBJECT_VERSION_ID.
    let ovid = versions[0]["id"]["value"].as_str().unwrap().to_owned();
    let comp = svc
        .composition_get(params(json!({ "ehr_id": ehr_id, "uid_based_id": ovid })))
        .await
        .expect("get created composition");
    assert_eq!(comp["name"]["value"], "Via contribution");
}

#[tokio::test]
async fn templateless_composition_still_gets_rm_and_terminology_validation() {
    // F-07-02: a COMPOSITION without a declared template_id must still fail on
    // RM-invariant / RM-terminology violations (here: an invalid category code),
    // and a valid templateless composition must still commit.
    let pg = Pg::start().await;
    let svc = EhrbaseService::new(pg.migrated_pool("templateless").await);

    let ehr = svc.ehr_create(params(json!({})), None).await.expect("ehr");
    let ehr_id = ehr["ehr_id"]["value"].as_str().unwrap().to_owned();

    let bad = svc
        .composition_create(
            params(json!({ "ehr_id": ehr_id })),
            composition_with_bad_category(),
        )
        .await;
    assert!(
        matches!(bad, Err(ApiError::ValidationFailed(_))),
        "templateless composition with bad category must 422, got {bad:?}"
    );

    svc.composition_create(params(json!({ "ehr_id": ehr_id })), composition("valid"))
        .await
        .expect("a valid templateless composition still commits");
}

#[tokio::test]
async fn contribution_rejects_an_invalid_composition() {
    // F-07-01: the CONTRIBUTION commit path must run composition validation and
    // reject the whole contribution atomically.
    let pg = Pg::start().await;
    let svc = EhrbaseService::new(pg.migrated_pool("contribinvalid").await);

    let ehr = svc.ehr_create(params(json!({})), None).await.expect("ehr");
    let ehr_id = ehr["ehr_id"]["value"].as_str().unwrap().to_owned();

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
    let res = svc
        .contribution_create(params(json!({ "ehr_id": ehr_id })), body)
        .await;
    assert!(
        matches!(res, Err(ApiError::ValidationFailed(_))),
        "invalid composition in a contribution must 422, got {res:?}"
    );
}

#[tokio::test]
async fn revision_history_lists_every_version() {
    let pg = Pg::start().await;
    let svc = EhrbaseService::new(pg.migrated_pool("revhistory").await);

    let ehr = svc.ehr_create(params(json!({})), None).await.expect("ehr");
    let ehr_id = ehr["ehr_id"]["value"].as_str().unwrap().to_owned();

    let status = svc
        .ehr_status_get_at_time(params(json!({ "ehr_id": ehr_id })))
        .await
        .expect("status");
    let ovid_v1 = uid(&status).to_owned();
    svc.ehr_status_update(
        params(json!({ "ehr_id": ehr_id, "If-Match": ovid_v1 })),
        status,
    )
    .await
    .expect("status update");

    let history = svc
        .versioned_ehr_status_revision_history(params(json!({ "ehr_id": ehr_id })))
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
    let created = svc
        .ehr_create(params(json!({})), Some(status))
        .await
        .expect("ehr");
    let ehr_id = created["ehr_id"]["value"].as_str().unwrap().to_owned();

    let found = svc
        .ehr_get_by_subject(params(json!({
            "subject_id": "patient-123", "subject_namespace": "patients"
        })))
        .await
        .expect("get by subject");
    assert_eq!(found["ehr_id"]["value"], ehr_id);

    assert!(
        svc.ehr_get_by_subject(params(json!({
            "subject_id": "nobody", "subject_namespace": "patients"
        })))
        .await
        .is_err()
    );
}

#[tokio::test]
async fn stored_query_crud() {
    let pg = Pg::start().await;
    let svc = EhrbaseService::new(pg.migrated_pool("storedq").await);

    let aql = "SELECT c FROM EHR e CONTAINS COMPOSITION c".to_owned();
    svc.definition_query_version_store_yaml(
        params(json!({ "qualified_query_name": "org.example::all_comps", "version": "1.0.0" })),
        aql.clone(),
    )
    .await
    .expect("store query");

    let got = svc
        .definition_query_version_get(params(json!({
            "qualified_query_name": "org.example::all_comps", "version": "1.0.0"
        })))
        .await
        .expect("get query");
    assert_eq!(got["q"], aql);
    assert_eq!(got["version"], "1.0.0");
    assert_eq!(got["type"], "AQL");

    let list = svc
        .definition_query_list(params(
            json!({ "qualified_query_name": "org.example::all_comps" }),
        ))
        .await
        .expect("list");
    assert_eq!(list.len(), 1);
}

#[tokio::test]
async fn item_tag_crud() {
    let pg = Pg::start().await;
    let svc = EhrbaseService::new(pg.migrated_pool("itemtags").await);

    let ehr = svc.ehr_create(params(json!({})), None).await.expect("ehr");
    let ehr_id = ehr["ehr_id"]["value"].as_str().unwrap().to_owned();
    let comp = svc
        .composition_create(params(json!({ "ehr_id": ehr_id })), composition("Tagged"))
        .await
        .expect("composition");
    let vo_id = uid(&comp).split("::").next().unwrap().to_owned();

    let has_key = |tags: &[std::collections::BTreeMap<String, Value>], k: &str| {
        tags.iter()
            .any(|t| t.get("key").and_then(Value::as_str) == Some(k))
    };

    let upserted = svc
        .composition_tags_update(
            params(json!({ "ehr_id": ehr_id, "uid_based_id": vo_id })),
            vec![json!({ "key": "priority", "value": "high" })],
        )
        .await
        .expect("tag update");
    assert!(has_key(&upserted, "priority"));

    let on_comp = svc
        .composition_tags_get(params(json!({ "ehr_id": ehr_id, "uid_based_id": vo_id })))
        .await
        .expect("comp tags");
    assert_eq!(on_comp.len(), 1);

    let all = svc
        .ehr_tags_get(params(json!({ "ehr_id": ehr_id })))
        .await
        .expect("ehr tags");
    assert_eq!(all.len(), 1);

    svc.composition_tags_delete(params(json!({
        "ehr_id": ehr_id, "uid_based_id": vo_id, "key": "priority"
    })))
    .await
    .expect("delete tag");
    let after = svc
        .composition_tags_get(params(json!({ "ehr_id": ehr_id, "uid_based_id": vo_id })))
        .await
        .expect("comp tags after");
    assert!(after.is_empty());
}
