#![allow(
    clippy::panic,
    clippy::print_stdout,
    clippy::print_stderr,
    let_underscore_drop
)] // test assertions/diagnostics/fixtures
//! End-to-end DEMOGRAPHIC service tests against a real PostgreSQL 18
//! (shared testkit harness): the party CRUD + versioning + VERSIONED_PARTY +
//! contribution + tags lifecycle, driven through the `DemographicService`
//! envelope seam exactly as the REST layer calls it. Verifies the 0003 party
//! migration applies cleanly (the harness runs migrations) and that parties
//! version with no EHR scope.
#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::too_many_lines,
    clippy::doc_markdown
)]

use serde_json::{Value, json};
use sqlx::PgPool;

use ehrbase::service::EhrbaseService;
use ehrbase::service::demographic::types::PartyKind;
use ehrbase::service::status::{CallStatusType, SmError};
use ehrbase::service::version_update::UpdateVersion;

/// The `uid.value` (`OBJECT_VERSION_ID`) of a versioned-object body.
fn ovid(v: &Value) -> &str {
    v["uid"]["value"].as_str().expect("uid.value")
}

/// The bare versioned-object UUID from an `OBJECT_VERSION_ID`.
fn vo_uuid(v: &Value) -> String {
    ovid(v).split("::").next().expect("vo uuid").to_owned()
}

/// The DB's current instant (`SELECT now()`) as a `jiff::Timestamp`. Time-travel
/// probes MUST anchor their reference instant on this server clock — the same
/// clock that stamps `vo_version.sys_period` — never on the test-process wall
/// clock: a client/DB clock skew under parallel test load would race
/// the at-time read against the version validity intervals.
async fn db_now(pool: &PgPool) -> jiff::Timestamp {
    sqlx::query_scalar::<_, jiff_sqlx::Timestamp>("SELECT now()")
        .fetch_one(pool)
        .await
        .expect("db now()")
        .to_jiff()
}

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
                "items": [{
                    "_type": "ELEMENT",
                    "archetype_node_id": "at0003",
                    "name": { "_type": "DV_TEXT", "value": "family" },
                    "value": { "_type": "DV_TEXT", "value": name }
                }]
            }
        }]
    })
}

fn organisation(name: &str) -> Value {
    json!({
        "_type": "ORGANISATION",
        "archetype_node_id": "openEHR-DEMOGRAPHIC-ORGANISATION.organisation.v1",
        "name": { "_type": "DV_TEXT", "value": name },
        "identities": [{
            "_type": "PARTY_IDENTITY",
            "archetype_node_id": "at0001",
            "name": { "_type": "DV_TEXT", "value": "legal name" },
            "details": {
                "_type": "ITEM_TREE",
                "archetype_node_id": "at0002",
                "name": { "_type": "DV_TEXT", "value": "structure" },
                "items": [{
                    "_type": "ELEMENT",
                    "archetype_node_id": "at0003",
                    "name": { "_type": "DV_TEXT", "value": "org name" },
                    "value": { "_type": "DV_TEXT", "value": name }
                }]
            }
        }]
    })
}

fn role(name: &str) -> Value {
    json!({
        "_type": "ROLE",
        "archetype_node_id": "openEHR-DEMOGRAPHIC-ROLE.role.v1",
        "name": { "_type": "DV_TEXT", "value": name },
        "identities": [{
            "_type": "PARTY_IDENTITY",
            "archetype_node_id": "at0001",
            "name": { "_type": "DV_TEXT", "value": name },
            "details": {
                "_type": "ITEM_TREE",
                "archetype_node_id": "at0002",
                "name": { "_type": "DV_TEXT", "value": "structure" },
                "items": []
            }
        }],
        "performer": {
            "_type": "PARTY_REF",
            "namespace": "demographic",
            "type": "PERSON",
            "id": { "_type": "HIER_OBJECT_ID", "value": "cccccccc-cccc-4ccc-8ccc-cccccccccccc" }
        }
        // No `capabilities`: a PRESENT list must be non-empty
        // (ROLE.Capabilities_valid, role.adoc) — absence is the valid way to
        // carry "no capabilities".
    })
}

/// The SM `UPDATE_VERSION` commit envelope for a bare-RM party write, built
/// from the wire shape (`commit_audit`, terminology-coded lifecycle/change).
fn uv(data: &Value, preceding: Option<&str>) -> UpdateVersion {
    let mut v = json!({
        "lifecycle_state": { "terminology_id": "openehr", "code_string": "532" },
        "data": data,
        "commit_audit": {
            "change_type": { "terminology_id": "openehr", "code_string": "249" },
            "committer": { "_type": "PARTY_IDENTIFIED", "name": "sm tester" }
        }
    });
    if let Some(p) = preceding {
        v["preceding_version_uid"] = json!({ "value": p });
    }
    serde_json::from_value(v).expect("UpdateVersion")
}

/// The literal SM `I_DEMOGRAPHIC_SERVICE` + `I_PARTY` calls (typed Uuid/version
/// arguments), not the wire seam: `create_party (UV_PARTY): UUID` →
/// `has_party`/`get_party`/`get_party_at_version` → `update_party` →
/// `delete_party`, checking the `Post_party_deleted: not has_party`
/// post-condition (`i_party.adoc`).
#[tokio::test]
async fn party_sm_calls_round_trip() {
    let db = testkit::db().await.expect("testkit database");
    let svc = EhrbaseService::new(db.pool());

    // create_party(UV_PARTY) → the new VERSIONED_OBJECT's id.
    let vo_id = svc
        .create_party(uv(&person("Jane"), None))
        .await
        .expect("create_party");

    // has_party true; get_party returns the current PERSON.
    assert!(svc.has_party(vo_id).await.expect("has_party"), "live party");
    let got = svc.get_party(vo_id).await.expect("get_party");
    assert_eq!(got["_type"], "PERSON");
    assert_eq!(got["name"]["value"], "Jane");

    // The current version id — has_party_version_id + get_party_at_version.
    let v1 = got["uid"]["value"].as_str().expect("uid.value").to_owned();
    assert!(v1.ends_with("::1"), "first version, got {v1}");
    assert!(
        svc.has_party_version_id(v1.clone())
            .await
            .expect("has_party_version_id")
    );
    let by_ver = svc
        .get_party_at_version(v1.clone())
        .await
        .expect("get_party_at_version");
    assert_eq!(by_ver["_type"], "ORIGINAL_VERSION");
    // An unknown version id → false / object_version_does_not_exist.
    let bogus = format!("{vo_id}::ehrbase-rs.local::9");
    assert!(!svc.has_party_version_id(bogus.clone()).await.expect("has"));
    let miss = svc.get_party_at_version(bogus).await;
    assert!(
        matches!(
            miss,
            Err(SmError {
                status: CallStatusType::ObjectVersionDoesNotExist,
                ..
            })
        ),
        "unknown version → object_version_does_not_exist, got {miss:?}"
    );

    // update_party(UV_PARTY with preceding_version_uid) → the new version uid.
    let v2 = svc
        .update_party(vo_id, uv(&person("Jane Roe"), Some(&v1)))
        .await
        .expect("update_party");
    assert!(v2.ends_with("::2"), "second version, got {v2}");
    let got2 = svc.get_party(vo_id).await.expect("get after update");
    assert_eq!(got2["name"]["value"], "Jane Roe");

    // delete_party → post-condition `not has_party`.
    let del = svc.delete_party(vo_id).await.expect("delete_party");
    assert!(!del.is_empty(), "delete returns the deleted version uid");
    assert!(
        !svc.has_party(vo_id).await.expect("has_party after delete"),
        "Post_party_deleted: not has_party"
    );
    // get_party on a deleted (no live current) party → versioned_object_does_not_exist.
    let after = svc.get_party(vo_id).await;
    assert!(
        matches!(
            after,
            Err(SmError {
                status: CallStatusType::VersionedObjectDoesNotExist,
                ..
            })
        ),
        "get_party after delete → 404, got {after:?}"
    );

    // A never-seen id: has_party false, get_party 404.
    let never = ehrbase::ids::VoId::new();
    assert!(!svc.has_party(never).await.expect("has_party unknown"));
    assert!(svc.get_party(never).await.is_err(), "unknown party → error");
}

#[tokio::test]
async fn person_lifecycle_end_to_end() {
    let db = testkit::db().await.expect("testkit database");
    let pool = db.pool();
    let svc = EhrbaseService::new(pool.clone());

    // create → v1
    let created = svc
        .party_create(PartyKind::Person, person("Jane"), None)
        .await
        .expect("create person");
    assert_eq!(created.body["_type"], "PERSON");
    let ovid_v1 = ovid(&created.body).to_owned();
    let vo = vo_uuid(&created.body);
    assert!(ovid_v1.ends_with("::1"), "first version, got {ovid_v1}");
    assert_eq!(
        created.meta.as_ref().expect("meta").ehr_id,
        "",
        "a party has no EHR scope"
    );

    // get current (bare HIER_OBJECT_ID)
    let got = svc
        .party_get(PartyKind::Person, vo.clone(), None)
        .await
        .expect("get person");
    assert_eq!(got.body["name"]["value"], "Jane");

    // get by OBJECT_VERSION_ID (that specific version)
    let by_ovid = svc
        .party_get(PartyKind::Person, ovid_v1.clone(), None)
        .await
        .expect("get by ovid");
    assert_eq!(ovid(&by_ovid.body), ovid_v1);

    // time-travel: capture a time inside v1 FROM THE DB CLOCK, then update to
    // v2. The reference instant MUST come from the server clock (the clock that
    // stamps `vo_version.sys_period`), never the test-process wall clock — a
    // client/DB clock skew under parallel-load testcontainers races the at-time
    // read against the version validity intervals (a real flake this fixes).
    let t_v1 = db_now(&pool).await;
    // A short gap so v2's commit transaction timestamp is strictly greater than
    // t_v1 even at microsecond resolution (both are now on the DB clock).
    tokio::time::sleep(std::time::Duration::from_millis(15)).await;

    // update (If-Match = current OVID) → v2
    let updated = svc
        .party_update(
            PartyKind::Person,
            vo.clone(),
            ovid_v1.clone(),
            person("Jane Roe"),
            None,
        )
        .await
        .expect("update person");
    let ovid_v2 = ovid(&updated.body).to_owned();
    assert!(ovid_v2.ends_with("::2"), "second version, got {ovid_v2}");

    // at-time read returns v1
    let at_v1 = svc
        .party_get(PartyKind::Person, vo.clone(), Some(t_v1.to_string()))
        .await
        .expect("at-time");
    assert_eq!(at_v1.body["name"]["value"], "Jane");

    // stale If-Match → precondition failed (412)
    let stale = svc
        .party_update(
            PartyKind::Person,
            vo.clone(),
            ovid_v1.clone(),
            person("stale"),
            None,
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
        "stale update, got {stale:?}"
    );

    // wrong-kind route → NotFound
    let wrong = svc.party_get(PartyKind::Role, vo.clone(), None).await;
    assert!(
        matches!(
            wrong,
            Err(SmError {
                status: CallStatusType::VersionedObjectDoesNotExist,
                ..
            })
        ),
        "person under role route is 404, got {wrong:?}"
    );

    // VERSIONED_PARTY + revision history + version-by-id
    let vp = svc
        .versioned_party_get(vo.clone())
        .await
        .expect("versioned_party");
    assert_eq!(vp.body["_type"], "VERSIONED_PARTY");
    let rh = svc
        .versioned_party_revision_history(vo.clone())
        .await
        .expect("revision_history");
    assert_eq!(rh.body["items"].as_array().expect("items").len(), 2);
    let ov = svc
        .versioned_party_version_get_by_id(vo.clone(), ovid_v1.clone())
        .await
        .expect("version by id");
    assert_eq!(ov.body["_type"], "ORIGINAL_VERSION");

    // delete (OBJECT_VERSION_ID on the path, no If-Match) → deleted;
    // subsequent get → 204 (Null)
    let deleted = svc
        .party_delete(PartyKind::Person, ovid_v2.clone(), None, None)
        .await
        .expect("delete");
    assert!(deleted.is_empty());
    let after = svc
        .party_get(PartyKind::Person, vo.clone(), None)
        .await
        .expect("get after delete");
    assert!(after.is_empty(), "deleted current read is 204 (Null body)");
}

/// Party write-path fix: the create/update representation is now
/// built **from the commit result** (never a post-commit re-read). It must be
/// byte-identical to a fresh read — the served body is
/// `inject_uid(reassemble(decompose(body)))` and the node codec round-trips
/// losslessly (RM common master06 §Committal: the written version identity +
/// content). Mirrors the EHR/DIRECTORY `write_responses_match_a_fresh_read`
/// gate; covers PERSON and the ORGANISATION sibling (same path, keyed by
/// `PartyKind`).
#[tokio::test]
async fn party_write_responses_match_a_fresh_read() {
    let db = testkit::db().await.expect("testkit database");
    let svc = EhrbaseService::new(db.pool());

    // create → the built-from-commit body equals a fresh read.
    let created = svc
        .party_create(PartyKind::Person, person("Jane"), None)
        .await
        .expect("create person");
    let ovid_v1 = ovid(&created.body).to_owned();
    let vo = vo_uuid(&created.body);
    let fresh = svc
        .party_get(PartyKind::Person, vo.clone(), None)
        .await
        .expect("get person");
    assert_eq!(
        created.body, fresh.body,
        "create body built from the commit must equal a fresh read"
    );
    assert_eq!(
        created.meta.as_ref().expect("create meta").uid,
        fresh.meta.as_ref().expect("read meta").uid,
        "create uid == fresh read uid"
    );

    // update → the built-from-commit body equals a fresh read.
    let updated = svc
        .party_update(
            PartyKind::Person,
            vo.clone(),
            ovid_v1,
            person("Jane Roe"),
            None,
        )
        .await
        .expect("update person");
    let fresh_v2 = svc
        .party_get(PartyKind::Person, vo.clone(), None)
        .await
        .expect("get person v2");
    assert_eq!(
        updated.body, fresh_v2.body,
        "update body built from the commit must equal a fresh read"
    );
    assert_eq!(
        updated.meta.as_ref().expect("update meta").uid,
        fresh_v2.meta.as_ref().expect("read meta").uid,
    );

    // ORGANISATION sibling — the same create path keyed by PartyKind.
    let created_org = svc
        .party_create(PartyKind::Organisation, organisation("Acme"), None)
        .await
        .expect("create organisation");
    let vo_org = vo_uuid(&created_org.body);
    let fresh_org = svc
        .party_get(PartyKind::Organisation, vo_org, None)
        .await
        .expect("get organisation");
    assert_eq!(
        created_org.body, fresh_org.body,
        "organisation create body built from the commit must equal a fresh read"
    );
}

/// The versioned-party delete shape the DEM ECC cases drive (ECC-DEM-005/006 …):
/// the path is the **bare** versioned-object uid and the preceding version is
/// carried by `If-Match` (the `OBJECT_VERSION_ID`). Delete succeeds (`204`) and
/// the current read afterwards is a deleted/`204` (Null body).
#[tokio::test]
async fn person_delete_by_versioned_uid_with_if_match() {
    let db = testkit::db().await.expect("testkit database");
    let svc = EhrbaseService::new(db.pool());

    let created = svc
        .party_create(PartyKind::Person, person("Jane"), None)
        .await
        .expect("create person");
    let ovid = ovid(&created.body).to_owned();
    let vo = vo_uuid(&created.body);

    // Bare versioned-object uid on the path, `If-Match` = the current OVID.
    let deleted = svc
        .party_delete(PartyKind::Person, vo.clone(), Some(ovid), None)
        .await
        .expect("delete by versioned uid + If-Match");
    assert!(deleted.is_empty());

    // The deleted current version reads back as 204 (Null body).
    let after = svc
        .party_get(PartyKind::Person, vo, None)
        .await
        .expect("get after delete");
    assert!(after.is_empty(), "deleted current read is 204 (Null body)");
}

#[tokio::test]
async fn role_create_and_get() {
    let db = testkit::db().await.expect("testkit database");
    let svc = EhrbaseService::new(db.pool());

    let created = svc
        .party_create(PartyKind::Role, role("Clinician"), None)
        .await
        .expect("create role");
    assert_eq!(created.body["_type"], "ROLE");
    let vo = vo_uuid(&created.body);
    let got = svc
        .party_get(PartyKind::Role, vo, None)
        .await
        .expect("get role");
    assert_eq!(got.body["name"]["value"], "Clinician");
    assert_eq!(got.body["performer"]["type"], "PERSON");
}

#[tokio::test]
async fn demographic_contribution_multi_version() {
    let db = testkit::db().await.expect("testkit database");
    let svc = EhrbaseService::new(db.pool());

    let body = json!({
        "_type": "CONTRIBUTION",
        "versions": [
            {
                "_type": "ORIGINAL_VERSION",
                "commit_audit": {
                    "change_type": {
                        "_type": "DV_CODED_TEXT",
                        "value": "creation",
                        "defining_code": {
                            "_type": "CODE_PHRASE",
                            "terminology_id": { "_type": "TERMINOLOGY_ID", "value": "openehr" },
                            "code_string": "249"
                        }
                    }
                },
                "data": person("Alice")
            },
            {
                "_type": "ORIGINAL_VERSION",
                "commit_audit": {
                    "change_type": {
                        "_type": "DV_CODED_TEXT",
                        "value": "creation",
                        "defining_code": {
                            "_type": "CODE_PHRASE",
                            "terminology_id": { "_type": "TERMINOLOGY_ID", "value": "openehr" },
                            "code_string": "249"
                        }
                    }
                },
                "data": role("Nurse")
            }
        ],
        "audit": {
            "committer": { "_type": "PARTY_IDENTIFIED", "name": "tester" }
        }
    });

    let created = svc
        .demographic_contribution_create(body)
        .await
        .expect("create contribution");
    assert_eq!(created.body["_type"], "CONTRIBUTION");
    let uid = created.body["uid"]["value"]
        .as_str()
        .expect("uid")
        .to_owned();
    assert_eq!(
        created.body["versions"].as_array().expect("versions").len(),
        2
    );

    let got = svc
        .demographic_contribution_get(uid)
        .await
        .expect("get contribution");
    assert_eq!(got.body["versions"].as_array().expect("versions").len(), 2);
}

#[tokio::test]
async fn party_tags_crud() {
    let db = testkit::db().await.expect("testkit database");
    let svc = EhrbaseService::new(db.pool());

    let created = svc
        .party_create(PartyKind::Person, person("Tagged"), None)
        .await
        .expect("create person");
    let vo = vo_uuid(&created.body);

    // PUT a tag collection
    let tags = svc
        .party_tags_update(
            PartyKind::Person,
            vo.clone(),
            vec![json!({ "key": "priority", "value": "high" })],
        )
        .await
        .expect("put tags");
    let arr = tags.body.as_array().expect("tags array");
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["key"], "priority");

    // GET tags on the party
    let got = svc
        .party_tags_get(PartyKind::Person, vo.clone())
        .await
        .expect("get tags");
    assert_eq!(got.body.as_array().expect("arr").len(), 1);

    // demographic tags (ehr-less scope) sees it
    let all = svc
        .demographic_tags_get(None, None, None)
        .await
        .expect("all demographic tags");
    assert_eq!(all.body.as_array().expect("arr").len(), 1);

    // DELETE the tag
    svc.party_tags_delete(PartyKind::Person, vo.clone(), "priority".to_owned())
        .await
        .expect("delete tag");
    let empty = svc
        .party_tags_get(PartyKind::Person, vo)
        .await
        .expect("get tags after delete");
    assert!(empty.body.as_array().expect("arr").is_empty());
}
