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

    let original = svc
        .versioned_composition_version_get_by_id(params(json!({
            "ehr_id": ehr_id, "versioned_object_uid": comp_vo_id, "version_uid": comp_ovid_v1
        })))
        .await
        .expect("original version");
    assert_eq!(original["_type"], "ORIGINAL_VERSION");
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

    // ── logical delete → gone ────────────────────────────────────────────────
    svc.composition_delete(params(
        json!({ "ehr_id": ehr_id, "uid_based_id": comp_vo_id }),
    ))
    .await
    .expect("composition_delete");
    assert!(
        svc.composition_get(params(
            json!({ "ehr_id": ehr_id, "uid_based_id": comp_vo_id })
        ))
        .await
        .is_err(),
        "deleted composition must not be retrievable"
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
