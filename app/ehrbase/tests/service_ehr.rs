#![allow(
    clippy::panic,
    clippy::print_stdout,
    clippy::print_stderr,
    let_underscore_drop
)] // test assertions/diagnostics/fixtures
//! End-to-end service tests against a real PostgreSQL 18 (shared testkit harness):
//! the EHR / EHR_STATUS / COMPOSITION / DIRECTORY / CONTRIBUTION lifecycle,
//! including versioning, optimistic concurrency, time-travel, and logical
//! delete — driven through the `EhrService` envelope seam exactly as the
//! REST layer calls it, asserting both the RM payload (`.body`) and the resource
//! metadata (`.meta`, from which the HTTP edge derives `ETag`/`Location`).
#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::too_many_lines,
    clippy::doc_markdown
)]

use serde_json::{Value, json};

use openehr_base::prelude::{ObjectVersionId, TerminologyCode};
use openehr_rm::prelude::PartyProxy;

use ehrbase::service::EhrbaseService;
use ehrbase::service::error::ServiceError;
use ehrbase::service::status::{CallStatusType, SmError};
use ehrbase::versioning::change::Committed;

use ehrbase::service::version_update::{UpdateAudit, UpdateVersion};

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
            system_id: None,
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
///. Every other mandatory attribute is valid, so the category
/// code is the only defect.
fn composition_with_bad_category() -> Value {
    let mut c = composition("Bad category");
    c["category"]["defining_code"]["code_string"] = json!("9999");
    c
}

/// A minimal *valid* root FOLDER (a folder-hierarchy root). `validate_folder`
/// requires `name` and forbids inline content-by-value, which a bare root
/// satisfies (RM ehr, DIRECTORY package).
fn folder(name: &str) -> Value {
    json!({
        "_type": "FOLDER",
        "archetype_node_id": "openEHR-EHR-FOLDER.generic.v1",
        "name": { "_type": "DV_TEXT", "value": name }
    })
}

#[tokio::test]
async fn ehr_composition_lifecycle_end_to_end() {
    let db = testkit::db().await.expect("testkit database");
    let svc = EhrbaseService::new(db.pool());

    // ── EHR create + retrieve ────────────────────────────────────────────────
    // NOTE: the SM `create_ehr` returns the new UUID; the RM `EHR`
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
    // NOTE: `replace_ehr_status` returns the new version_uid (the
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

    // Reactivate before touching contents: with the B2 write guard, content
    // writes on an EHR whose EHR_STATUS.is_modifiable = false are refused
    // (RM ehr master04 §"EHR Active Status") — the deactivated-state
    // assertions above stay; the composition stages below need a modifiable
    // EHR again (a third status version).
    let mut status_v3_body = status_v2.clone();
    status_v3_body.as_object_mut().unwrap().remove("uid");
    status_v3_body["is_modifiable"] = json!(true);
    let status_v3_uid = svc
        .replace_ehr_status(ehr_uuid, uv(status_v3_body, "251", Some(&status_v2_uid)))
        .await
        .expect("status reactivate");
    assert!(status_v3_uid.ends_with("::3"));

    // A specific EHR_STATUS version reads as the BARE EHR_STATUS, not
    // an ORIGINAL_VERSION wrapper.
    let sp: Vec<&str> = status_ovid_v1.split("::").collect();
    let status_by_v = svc
        .get_ehr_status_at_version(ehr_uuid, sp[0].parse().expect("status vo uuid"), sp[2])
        .await
        .expect("status by version");
    assert_eq!(status_by_v["_type"], "EHR_STATUS");
    assert_eq!(uid(&status_by_v), status_ovid_v1);

    // ── COMPOSITION: create, update, version reads ──────────────────────────
    let comp_ovid_v1 = svc
        .create_composition(ehr_uuid, uv(composition("Encounter"), "249", None))
        .await
        .expect("composition_create")
        .version_uid();
    let comp_vo_id = comp_ovid_v1.split("::").next().unwrap().to_owned();
    let comp_vo_uuid = comp_vo_id.parse::<ehrbase::ids::VoId>().expect("vo uuid");

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
        .expect("composition_update")
        .version_uid();
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
    // RM ehr master04: the concrete binding VERSIONED_COMPOSITION, never the
    // generic VERSIONED_OBJECT, appears on the wire.
    assert_eq!(versioned["_type"], "VERSIONED_COMPOSITION");
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
    let comp_ovid_v1_id: ObjectVersionId = comp_ovid_v1.parse().expect("ovid");
    let stale_delete = svc.delete_composition(ehr_uuid, &comp_ovid_v1_id).await;
    assert!(
        matches!(stale_delete, Err(ServiceError::Conflict(_))),
        "stale preceding_version_uid must 409, got {stale_delete:?}"
    );
    // NOTE: the old "bare HIER_OBJECT_ID → 400" sub-check is
    // dropped — `delete_composition` now takes a typed `ObjectVersionId`, so a
    // bare id cannot be constructed as an argument; that decode + 400 moved to
    // the protocol adapter (`ehrbase-rest`), where it is exercised.

    // The correct latest version_uid deletes; the returned uid names the new
    // (deleted) version (204_COMPOSITION_deleted ETag/Location).
    let comp_ovid_v2_id: ObjectVersionId = comp_ovid_v2.parse().expect("ovid");
    let deleted = svc
        .delete_composition(ehr_uuid, &comp_ovid_v2_id)
        .await
        .expect("composition_delete")
        .version_uid();
    assert!(
        deleted.ends_with("::3"),
        "delete names the new (deleted) version, got {deleted}"
    );
    // A deleted read is NOT an error/500 — it yields Null (→ 204).
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
            svc.delete_composition(ehr_uuid, &comp_ovid_v2_id).await,
            Err(ServiceError::BadRequest(_))
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
    let folder = json!({ "_type": "FOLDER", "archetype_node_id": "openEHR-EHR-FOLDER.generic.v1", "name": { "_type": "DV_TEXT", "value": "root" } });
    let dir_ovid = svc
        .create_directory(ehr_uuid, uv(folder.clone(), "249", None))
        .await
        .expect("directory_create")
        .uid;
    let dir_got = svc
        .get_directory_at_time(ehr_uuid, None, None)
        .await
        .expect("directory_get")
        .body;
    assert_eq!(dir_got["name"]["value"], "root");

    let mut folder_v2 = folder;
    folder_v2["name"]["value"] = json!("root-renamed");
    let dir_ovid_v2 = svc
        .update_directory(ehr_uuid, uv(folder_v2, "251", Some(&dir_ovid)))
        .await
        .expect("directory_update")
        .uid;
    assert!(dir_ovid_v2.ends_with("::2"));

    svc.delete_directory(ehr_uuid, Some(dir_ovid_v2.parse().expect("ovid")))
        .await
        .expect("directory_delete");
}

#[tokio::test]
async fn is_modifiable_false_blocks_content_writes_but_not_ehr_status() {
    // RM ehr `ehr/master04-ehr_package.adoc` §"EHR Active Status":
    // `EHR_STATUS.is_modifiable = False` forbids writes to EHR *contents*
    // (Compositions, Folders); "the EHR_STATUS object itself is always
    // modifiable" (§"EHR Creation"), which is how the EHR is reactivated.
    // Wire code for a blocked content write is underdetermined by ITS-REST →
    // we return 409 Conflict (SM `CompositionAlreadyExists`).
    let db = testkit::db().await.expect("testkit database");
    let svc = EhrbaseService::new(db.pool());

    let ehr_uuid = svc.create_ehr(None).await.expect("ehr");

    // Seed a COMPOSITION and a directory while the EHR is active (default
    // is_modifiable = true), so update/delete have a target once deactivated.
    let comp_ovid = svc
        .create_composition(ehr_uuid, uv(composition("Active"), "249", None))
        .await
        .expect("composition while active")
        .version_uid();
    let comp_vo = comp_ovid
        .split("::")
        .next()
        .unwrap()
        .parse::<ehrbase::ids::VoId>()
        .expect("vo uuid");
    let folder = json!({ "_type": "FOLDER", "archetype_node_id": "openEHR-EHR-FOLDER.generic.v1", "name": { "_type": "DV_TEXT", "value": "root" } });
    let dir_ovid = svc
        .create_directory(ehr_uuid, uv(folder.clone(), "249", None))
        .await
        .expect("directory while active")
        .uid;

    // Deactivate: set is_modifiable = false. This EHR_STATUS write MUST succeed
    // (the EHR_STATUS object is always modifiable).
    let status_v1 = svc
        .get_ehr_status_at_time(ehr_uuid, None)
        .await
        .expect("status v1");
    let status_ovid_v1 = uid(&status_v1).to_owned();
    let mut deactivate = status_v1.clone();
    deactivate.as_object_mut().unwrap().remove("uid");
    deactivate["is_modifiable"] = json!(false);
    let status_v2_ovid = svc
        .replace_ehr_status(ehr_uuid, uv(deactivate, "251", Some(&status_ovid_v1)))
        .await
        .expect("deactivating EHR_STATUS is allowed");

    // A blocked EHR-content write is 409 Conflict (SM `CompositionAlreadyExists`
    // → `ServiceError::Conflict` on the composition write path; the directory
    // write path still surfaces the raw `SmError`).
    let comp_blocked =
        |r: &Result<Committed, ServiceError>| matches!(r, Err(ServiceError::Conflict(_)));
    let dir_blocked = |r: &Result<ehrbase::service::response::ResourceMeta, SmError>| {
        matches!(
            r,
            Err(SmError {
                status: CallStatusType::CompositionAlreadyExists,
                ..
            })
        )
    };

    // Every EHR-content write is now refused (409).
    let create = svc
        .create_composition(ehr_uuid, uv(composition("Blocked"), "249", None))
        .await;
    assert!(
        comp_blocked(&create),
        "create must 409 when inactive: {create:?}"
    );

    let update = svc
        .update_composition(
            ehr_uuid,
            comp_vo,
            uv(composition("Blocked update"), "251", Some(&comp_ovid)),
        )
        .await;
    assert!(
        comp_blocked(&update),
        "update must 409 when inactive: {update:?}"
    );

    let comp_ovid_id: ObjectVersionId = comp_ovid.parse().expect("ovid");
    let delete = svc.delete_composition(ehr_uuid, &comp_ovid_id).await;
    assert!(
        comp_blocked(&delete),
        "delete must 409 when inactive: {delete:?}"
    );

    let dir_update = svc
        .update_directory(ehr_uuid, uv(folder, "251", Some(&dir_ovid)))
        .await;
    assert!(
        dir_blocked(&dir_update),
        "directory update must 409 when inactive: {dir_update:?}"
    );

    // A CONTRIBUTION that writes content is refused too.
    let content_contrib = svc
        .create_ehr_contribution(
            ehr_uuid,
            json!({
                "versions": [{
                    "data": composition("Via contribution"),
                    "commit_audit": { "change_type": change_type("249", "creation") }
                }]
            }),
        )
        .await;
    assert!(
        matches!(
            content_contrib,
            Err(SmError {
                status: CallStatusType::CompositionAlreadyExists,
                ..
            })
        ),
        "content contribution must 409 when inactive: {content_contrib:?}"
    );

    // But an EHR_STATUS-only CONTRIBUTION (here, flipping it back to modifiable)
    // is still allowed — the EHR_STATUS object is always modifiable.
    let status_v2 = svc
        .get_ehr_status_at_time(ehr_uuid, None)
        .await
        .expect("status v2");
    assert_eq!(status_v2["is_modifiable"], json!(false));
    let mut reactivate = status_v2.clone();
    reactivate.as_object_mut().unwrap().remove("uid");
    reactivate["is_modifiable"] = json!(true);
    svc.replace_ehr_status(ehr_uuid, uv(reactivate, "251", Some(&status_v2_ovid)))
        .await
        .expect("reactivating EHR_STATUS is allowed while inactive");

    // Reactivated → content writes work again.
    svc.create_composition(ehr_uuid, uv(composition("Reactivated"), "249", None))
        .await
        .expect("content writes resume once reactivated");
}

#[tokio::test]
async fn creating_an_ehr_with_an_existing_id_conflicts() {
    let db = testkit::db().await.expect("testkit database");
    let svc = EhrbaseService::new(db.pool());

    let id = ehrbase::ids::EhrId::new();
    svc.create_ehr_with_id(id, None)
        .await
        .expect("first create");
    let again = svc.create_ehr_with_id(id, None).await;
    assert!(again.is_err(), "duplicate EHR id must conflict");
}

#[tokio::test]
async fn unknown_ehr_is_not_found() {
    let db = testkit::db().await.expect("testkit database");
    let svc = EhrbaseService::new(db.pool());
    let missing = ehrbase::ids::EhrId::new();
    assert!(svc.ehr_object(missing).await.is_err());
}

#[tokio::test]
async fn ehr_status_subject_type_is_enforced_end_to_end() {
    // RM ehr master04 §EHR Status: EHR_STATUS.subject is a (monomorphic)
    // PARTY_SELF. Through the real service seam (the same path the REST layer
    // calls), a foreign concrete _type (PARTY_IDENTIFIED) in that slot is a
    // 422 (SM ContentInvalid) naming the mismatch — on both EHR create and
    // EHR_STATUS PUT — while an anonymous empty PARTY_SELF is accepted.
    // Regression for the upstream diff findings B1/B2 (the X1 upstream
    // triage ledger; in git history at docs/conformance/upstream-ehrbase/TRIAGE.md).
    let db = testkit::db().await.expect("testkit database");
    let svc = EhrbaseService::new(db.pool());

    let wrong_subject = json!({
        "_type": "EHR_STATUS",
        "archetype_node_id": "openEHR-EHR-EHR_STATUS.generic.v1",
        "name": { "_type": "DV_TEXT", "value": "EHR Status" },
        "subject": {
            "_type": "PARTY_IDENTIFIED",
            "external_ref": {
                "_type": "PARTY_REF",
                "namespace": "patients",
                "type": "PERSON",
                "id": { "_type": "HIER_OBJECT_ID", "value": "patient-xyz" }
            }
        },
        "is_queryable": true,
        "is_modifiable": true
    });

    // (1) EHR create with a PARTY_IDENTIFIED subject → 422 naming the mismatch.
    let created = svc.create_ehr(Some(wrong_subject.clone())).await;
    match created {
        Err(SmError {
            status: CallStatusType::ContentInvalid,
            message,
        }) => assert!(
            message.contains("PARTY_SELF") && message.contains("PARTY_IDENTIFIED"),
            "rejection should name the type mismatch, got: {message}"
        ),
        other => panic!("PARTY_IDENTIFIED subject on create must be 422, got {other:?}"),
    }

    // (2) EHR_STATUS PUT with a PARTY_IDENTIFIED subject → 422 naming the mismatch.
    let ehr_uuid = svc.create_ehr(None).await.expect("ehr");
    let status_v1 = svc
        .get_ehr_status_at_time(ehr_uuid, None)
        .await
        .expect("status v1");
    let ovid_v1 = uid(&status_v1).to_owned();
    let mut put_body = wrong_subject;
    put_body.as_object_mut().unwrap().remove("uid");
    let put = svc
        .replace_ehr_status(ehr_uuid, uv(put_body, "251", Some(&ovid_v1)))
        .await;
    match put {
        Err(SmError {
            status: CallStatusType::ContentInvalid,
            message,
        }) => assert!(
            message.contains("PARTY_SELF") && message.contains("PARTY_IDENTIFIED"),
            "PUT rejection should name the type mismatch, got: {message}"
        ),
        other => panic!("PARTY_IDENTIFIED subject on PUT must be 422, got {other:?}"),
    }

    // (3) An anonymous empty PARTY_SELF subject is accepted on create
    // ("PARTY_SELF … enabling it to be made completely anonymous", master04).
    let anonymous = json!({
        "_type": "EHR_STATUS",
        "archetype_node_id": "openEHR-EHR-EHR_STATUS.generic.v1",
        "name": { "_type": "DV_TEXT", "value": "EHR Status" },
        "subject": {},
        "is_queryable": true,
        "is_modifiable": true
    });
    svc.create_ehr(Some(anonymous))
        .await
        .expect("an anonymous PARTY_SELF EHR_STATUS is accepted");
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
    let db = testkit::db().await.expect("testkit database");
    let svc = EhrbaseService::new(db.pool());

    let ehr_id = svc.create_ehr(None).await.expect("ehr").to_string();
    let ehr_uuid = ehrbase::ids::EhrId(ehr_id.parse::<uuid::Uuid>().expect("ehr uuid"));

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
    // spec-invalid combinations are rejected: creation on an existing object
    // as 400 (ITS-REST 400_CONTRIBUTION — modification-type mismatch), an
    // out-of-group code as 422 (content validation).
    let db = testkit::db().await.expect("testkit database");
    let svc = EhrbaseService::new(db.pool());

    let ehr_id = svc.create_ehr(None).await.expect("ehr").to_string();
    let ehr_uuid = ehrbase::ids::EhrId(ehr_id.parse::<uuid::Uuid>().expect("ehr uuid"));

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
                status: CallStatusType::PreconditionViolation,
                ..
            })
        ),
        "creation on an existing object is a change-control mismatch — the \
         ITS-REST 400_CONTRIBUTION scope (modification type does not match), \
         not content validation; got {bad_creation:?}"
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
    // a COMPOSITION without a declared template_id must still fail on
    // RM-invariant / RM-terminology violations (here: an invalid category code),
    // and a valid templateless composition must still commit.
    let db = testkit::db().await.expect("testkit database");
    let svc = EhrbaseService::new(db.pool());

    let ehr_id = svc.create_ehr(None).await.expect("ehr").to_string();
    let ehr_uuid = ehrbase::ids::EhrId(ehr_id.parse::<uuid::Uuid>().expect("ehr uuid"));

    let bad = svc
        .create_composition(ehr_uuid, uv(composition_with_bad_category(), "249", None))
        .await;
    assert!(
        matches!(bad, Err(ServiceError::ValidationFailed(_))),
        "templateless composition with bad category must 422, got {bad:?}"
    );

    svc.create_composition(ehr_uuid, uv(composition("valid"), "249", None))
        .await
        .expect("a valid templateless composition still commits");
}

#[tokio::test]
async fn contribution_rejects_an_invalid_composition() {
    // the CONTRIBUTION commit path must run composition validation and
    // reject the whole contribution atomically.
    let db = testkit::db().await.expect("testkit database");
    let svc = EhrbaseService::new(db.pool());

    let ehr_id = svc.create_ehr(None).await.expect("ehr").to_string();
    let ehr_uuid = ehrbase::ids::EhrId(ehr_id.parse::<uuid::Uuid>().expect("ehr uuid"));

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
    let db = testkit::db().await.expect("testkit database");
    let svc = EhrbaseService::new(db.pool());

    let ehr_id = svc.create_ehr(None).await.expect("ehr").to_string();
    let ehr_uuid = ehrbase::ids::EhrId(ehr_id.parse::<uuid::Uuid>().expect("ehr uuid"));

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
    let db = testkit::db().await.expect("testkit database");
    let svc = EhrbaseService::new(db.pool());

    let status = json!({
        "_type": "EHR_STATUS",
        "archetype_node_id": "openEHR-EHR-EHR_STATUS.generic.v1",
        "name": { "_type": "DV_TEXT", "value": "EHR Status" },
        // EHR_STATUS.subject is PARTY_SELF (RM ehr master04); the subject is
        // identified via its external_ref, not a PARTY_IDENTIFIED type.
        "subject": {
            "_type": "PARTY_SELF",
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
    let db = testkit::db().await.expect("testkit database");
    let svc = EhrbaseService::new(db.pool());

    let aql = "SELECT c FROM EHR e CONTAINS COMPOSITION c".to_owned();
    svc.query_store(
        "org.example::all_comps".to_owned(),
        Some("1.0.0".to_owned()),
        "AQL".to_owned(),
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
    // `parameters/path/version.yaml` — a partial `{major}` or
    // `{major}.{minor}` version resolves to the HIGHEST stored version
    // matching the prefix.
    let db = testkit::db().await.expect("testkit database");
    let svc = EhrbaseService::new(db.pool());

    // Store-time AQL validation is now enforced, so the per-version bodies must
    // be well-formed AQL (the letters `a`/`b`/`c` keep them distinguishable).
    for (version, q) in [
        ("1.0.0", "SELECT a FROM EHR a"),
        ("1.0.1", "SELECT b FROM EHR b"),
        ("1.1.0", "SELECT c FROM EHR c"),
    ] {
        svc.query_store(
            "org.example::obs".to_owned(),
            Some(version.to_owned()),
            "AQL".to_owned(),
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
    assert_eq!(by_major["q"], "SELECT c FROM EHR c");

    let by_minor = svc
        .query_version_get("org.example::obs".to_owned(), "1.0".to_owned())
        .await
        .expect("major.minor prefix resolves");
    assert_eq!(by_minor["version"], "1.0.1");
    assert_eq!(by_minor["q"], "SELECT b FROM EHR b");

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
    // `definition_query_list.yaml` — the qualified name is a PATTERN:
    // `org.openehr` "will list all versions of all queries with names starting
    // with `org.openehr`"; empty ⇒ wildcard.
    let db = testkit::db().await.expect("testkit database");
    let svc = EhrbaseService::new(db.pool());

    for (name, version) in [
        ("org.example::all_comps", "1.0.0"),
        ("org.example::all_comps", "1.1.0"),
        ("org.example::observations", "0.1.0"),
        ("com.acme::other", "0.1.0"),
    ] {
        svc.query_store(
            name.to_owned(),
            Some(version.to_owned()),
            "AQL".to_owned(),
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
    let db = testkit::db().await.expect("testkit database");
    let svc = EhrbaseService::new(db.pool());

    let ehr_id = svc.create_ehr(None).await.expect("ehr").to_string();
    let ehr_uuid = ehrbase::ids::EhrId(ehr_id.parse::<uuid::Uuid>().expect("ehr uuid"));
    let comp = svc
        .create_composition(ehr_uuid, uv(composition("Tagged"), "249", None))
        .await
        .expect("composition")
        .version_uid();
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
    // the ITEM_TAG wire shape is the OAS `ItemTag` schema
    // (`additionalProperties: false`): key/value/target_path plus
    // OBJECT_REF-shaped `target` and `owner_id`; no `id`, no `target_type`.
    let db = testkit::db().await.expect("testkit database");
    let svc = EhrbaseService::new(db.pool());

    let ehr_id = svc.create_ehr(None).await.expect("ehr").to_string();
    let ehr_uuid = ehrbase::ids::EhrId(ehr_id.parse::<uuid::Uuid>().expect("ehr uuid"));
    let comp = svc
        .create_composition(ehr_uuid, uv(composition("Tagged"), "249", None))
        .await
        .expect("composition")
        .version_uid();
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
    // PUT "updates the list of ALL ITEM_TAG resources … providing an
    // empty list will effectively remove all ITEM_TAG" — a full replace, not an
    // additive upsert.
    let db = testkit::db().await.expect("testkit database");
    let svc = EhrbaseService::new(db.pool());

    let ehr_id = svc.create_ehr(None).await.expect("ehr").to_string();
    let ehr_uuid = ehrbase::ids::EhrId(ehr_id.parse::<uuid::Uuid>().expect("ehr uuid"));
    let comp = svc
        .create_composition(ehr_uuid, uv(composition("Tagged"), "249", None))
        .await
        .expect("composition")
        .version_uid();
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
    // RM ehr § "EHR Creation" — creating an EHR yields a root EHR
    // object, an EHR_STATUS AND an EHR_ACCESS; `EHR.ehr_access` (1..1) is an
    // OBJECT_REF whose type is VERSIONED_EHR_ACCESS (invariant Ehr_access_valid).
    let db = testkit::db().await.expect("testkit database");
    let svc = EhrbaseService::new(db.pool());

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
    // ITS-REST `409_EHR.yaml` + CNF master06
    // `I_EHR_SERVICE.create_ehr-two_ehrs_same_patient` — a second EHR for the
    // same subject (external_ref id + namespace) must be rejected.
    let db = testkit::db().await.expect("testkit database");
    let svc = EhrbaseService::new(db.pool());

    let status = |subject: &str| {
        json!({
            "_type": "EHR_STATUS",
            "archetype_node_id": "openEHR-EHR-EHR_STATUS.generic.v1",
            "name": { "_type": "DV_TEXT", "value": "EHR Status" },
            // EHR_STATUS.subject is PARTY_SELF (RM ehr master04); identified by
            // its external_ref.
            "subject": {
                "_type": "PARTY_SELF",
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
    let db = testkit::db().await.expect("testkit database");
    let svc = EhrbaseService::new(db.pool());

    let ehr_id = svc.create_ehr(None).await.expect("ehr").to_string();
    let ehr_uuid = ehrbase::ids::EhrId(ehr_id.parse::<uuid::Uuid>().expect("ehr uuid"));

    // EHR_STATUS: no version_at_time → the latest VERSION.
    // NOTE: the reads return the ORIGINAL_VERSION `Value`; the
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
        .expect("composition")
        .version_uid();
    let comp_vo_id = comp_ovid_v1.split("::").next().unwrap().to_owned();
    let comp_vo_uuid = comp_vo_id.parse::<ehrbase::ids::VoId>().expect("vo uuid");
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

/// T1 — `EHR.folders` indexes MULTIPLE folder hierarchies (RM ehr master04
/// §Folders: "at any time, an entirely new Folder hierarchy may be added, which
/// will be referenced by a new member of the `EHR._folders_` attribute"). A
/// CONTRIBUTION may commit a *second* FOLDER hierarchy; it joins `EHR.folders`
/// as a new member. `EHR.directory` is `folders.item(1)` (RM ehr §EHR Class
/// `Directory_in_folders`) — the lowest-rank LIVE hierarchy. The `/directory`
/// endpoint binds only that slot; extra hierarchies come via CONTRIBUTION only.
#[tokio::test]
async fn ehr_folders_indexes_multiple_hierarchies_in_rank_order() {
    let db = testkit::db().await.expect("testkit database");
    let svc = EhrbaseService::new(db.pool());

    let ehr_uuid = svc.create_ehr(None).await.expect("ehr");

    // Before any folder: `EHR.folders`/`EHR.directory` (both 0..1) are absent.
    let ehr0 = svc.ehr_object(ehr_uuid).await.expect("ehr0");
    assert!(
        ehr0.get("folders").is_none(),
        "no folders before any hierarchy, got {ehr0}"
    );
    assert!(
        ehr0.get("directory").is_none(),
        "no directory before any hierarchy, got {ehr0}"
    );

    // Hierarchy 1 (rank 1) via the directory endpoint.
    let dir1_ovid = svc
        .create_directory(ehr_uuid, uv(folder("primary"), "249", None))
        .await
        .expect("directory_create (hierarchy 1)")
        .uid;
    let vo1 = dir1_ovid.split("::").next().unwrap().to_owned();

    // Hierarchy 2 (rank 2) via a CONTRIBUTION — a *second* FOLDER creation is now
    // allowed and appends a new member of `EHR.folders` (RM ehr master04 §Folders).
    let mut secondary = folder("secondary");
    secondary["archetype_node_id"] = json!("openEHR-EHR-FOLDER.episodes.v1");
    let contrib2 = svc
        .create_ehr_contribution(
            ehr_uuid,
            json!({
                "versions": [{
                    "data": secondary,
                    "commit_audit": { "change_type": change_type("249", "creation") }
                }]
            }),
        )
        .await
        .expect("second folder hierarchy via contribution");
    let f2_ovid_v1 = contrib2.body["versions"][0]["id"]["value"]
        .as_str()
        .unwrap()
        .to_owned();
    let vo2 = f2_ovid_v1.split("::").next().unwrap().to_owned();
    assert!(
        f2_ovid_v1.ends_with("::1"),
        "the second hierarchy is a fresh versioned object at v1, got {f2_ovid_v1}"
    );
    assert_ne!(
        vo1, vo2,
        "the two hierarchies are distinct versioned objects"
    );

    // EHR read: `folders` lists BOTH refs in rank order; each is an OBJECT_REF to
    // a VERSIONED_FOLDER, namespace local (RM ehr §EHR Class `Folders_valid`).
    let ehr = svc.ehr_object(ehr_uuid).await.expect("ehr with folders");
    let folders = ehr["folders"].as_array().expect("folders array");
    assert_eq!(folders.len(), 2, "both hierarchies indexed, got {ehr}");
    for (member, vo) in folders.iter().zip([&vo1, &vo2]) {
        assert_eq!(member["_type"], "OBJECT_REF");
        assert_eq!(member["namespace"], "local");
        assert_eq!(member["type"], "VERSIONED_FOLDER");
        assert_eq!(member["id"]["_type"], "HIER_OBJECT_ID");
        assert_eq!(member["id"]["value"], *vo);
    }
    // `Directory_in_folders`: `folders /= Void implies folders.item(1) = directory`.
    assert_eq!(ehr["directory"], folders[0]);

    // The two hierarchies version independently: bump hierarchy 2 → v2 via a
    // CONTRIBUTION; hierarchy 1 (the directory slot) stays at v1.
    let mut secondary_v2 = folder("secondary v2");
    secondary_v2["archetype_node_id"] = json!("openEHR-EHR-FOLDER.episodes.v1");
    let contrib2b = svc
        .create_ehr_contribution(
            ehr_uuid,
            json!({
                "versions": [{
                    "data": secondary_v2,
                    "preceding_version_uid": f2_ovid_v1,
                    "commit_audit": { "change_type": change_type("251", "modification") }
                }]
            }),
        )
        .await
        .expect("update second hierarchy");
    assert!(
        contrib2b.body["versions"][0]["id"]["value"]
            .as_str()
            .unwrap()
            .ends_with("::2"),
        "hierarchy 2 is now v2"
    );
    let dir = svc
        .get_directory_at_time(ehr_uuid, None, None)
        .await
        .expect("directory")
        .body;
    assert_eq!(dir["name"]["value"], "primary");
    assert!(
        uid(&dir).ends_with("::1"),
        "the directory (hierarchy 1) is still v1, got {}",
        uid(&dir)
    );

    // Delete the directory (rank 1) → the directory slot resolves to the next
    // LIVE hierarchy (rank 2); `/directory` now serves hierarchy 2.
    svc.delete_directory(ehr_uuid, Some(dir1_ovid.parse().expect("ovid")))
        .await
        .expect("delete directory (hierarchy 1)");
    let dir_after = svc
        .get_directory_at_time(ehr_uuid, None, None)
        .await
        .expect("directory after delete")
        .body;
    assert_eq!(
        dir_after["name"]["value"], "secondary v2",
        "the directory falls through to hierarchy 2"
    );
    // EHR read: `folders` now lists only the live hierarchy 2; directory == it.
    let ehr_after = svc.ehr_object(ehr_uuid).await.expect("ehr after delete");
    let folders_after = ehr_after["folders"].as_array().expect("folders after");
    assert_eq!(
        folders_after.len(),
        1,
        "the deleted hierarchy drops out of folders, got {ehr_after}"
    );
    assert_eq!(folders_after[0]["id"]["value"], vo2);
    assert_eq!(ehr_after["directory"], folders_after[0]);
}

/// T1 — logically deleting a SECONDARY folder hierarchy removes it from
/// `EHR.folders` (only LIVE hierarchies are members — RM ehr master04 §Folders)
/// while leaving the directory (rank 1) intact.
#[tokio::test]
async fn logical_delete_of_a_secondary_hierarchy_drops_it_from_folders() {
    let db = testkit::db().await.expect("testkit database");
    let svc = EhrbaseService::new(db.pool());
    let ehr_uuid = svc.create_ehr(None).await.expect("ehr");

    // Two hierarchies, both via CONTRIBUTION.
    let f1 = svc
        .create_ehr_contribution(
            ehr_uuid,
            json!({
                "versions": [{
                    "data": folder("primary"),
                    "commit_audit": { "change_type": change_type("249", "creation") }
                }]
            }),
        )
        .await
        .expect("hierarchy 1");
    let vo1 = f1.body["versions"][0]["id"]["value"]
        .as_str()
        .unwrap()
        .split("::")
        .next()
        .unwrap()
        .to_owned();
    let f2 = svc
        .create_ehr_contribution(
            ehr_uuid,
            json!({
                "versions": [{
                    "data": folder("secondary"),
                    "commit_audit": { "change_type": change_type("249", "creation") }
                }]
            }),
        )
        .await
        .expect("hierarchy 2");
    let f2_ovid = f2.body["versions"][0]["id"]["value"]
        .as_str()
        .unwrap()
        .to_owned();

    let ehr = svc
        .ehr_object(ehr_uuid)
        .await
        .expect("ehr with two folders");
    assert_eq!(ehr["folders"].as_array().unwrap().len(), 2);

    // Logically delete the SECONDARY (rank 2) via CONTRIBUTION (523|deleted|; a
    // delete member carries no data — RM common master06 §"Logical Deletion").
    svc.create_ehr_contribution(
        ehr_uuid,
        json!({
            "versions": [{
                "preceding_version_uid": f2_ovid,
                "commit_audit": { "change_type": change_type("523", "deleted") }
            }]
        }),
    )
    .await
    .expect("delete secondary hierarchy");

    // `folders` now lists only the live hierarchy 1; directory == it (rank 1).
    let ehr2 = svc.ehr_object(ehr_uuid).await.expect("ehr after delete");
    let folders = ehr2["folders"].as_array().expect("folders");
    assert_eq!(
        folders.len(),
        1,
        "the deleted secondary drops out of folders, got {ehr2}"
    );
    assert_eq!(folders[0]["id"]["value"], vo1);
    assert_eq!(ehr2["directory"]["id"]["value"], vo1);
}

/// T1 — single-hierarchy behaviour is unchanged: `POST /directory` manages the
/// single directory slot (`folders[1]`, RM ehr §EHR Class `Directory_in_folders`)
/// and 409s on a second create. Additional hierarchies are added via CONTRIBUTION
/// only (ITS-REST/SM bind only the directory).
#[tokio::test]
async fn directory_endpoint_rejects_a_second_directory_create() {
    let db = testkit::db().await.expect("testkit database");
    let svc = EhrbaseService::new(db.pool());
    let ehr_uuid = svc.create_ehr(None).await.expect("ehr");

    svc.create_directory(ehr_uuid, uv(folder("root"), "249", None))
        .await
        .expect("first directory");

    let second = svc
        .create_directory(ehr_uuid, uv(folder("root"), "249", None))
        .await;
    assert!(
        matches!(
            second,
            Err(SmError {
                status: CallStatusType::CompositionAlreadyExists,
                ..
            })
        ),
        "a second directory create must 409, got {second:?}"
    );
}

#[tokio::test]
async fn ehr_uri_resolves_local_structures_and_item_paths() {
    // `ehr:` URI local resolution (our own extension — no openEHR spec obliges
    // a server to resolve a DV_EHR_URI; BASE architecture_overview master11
    // §"EHR URIs"): a top-level structure resolves by versioned-object uid
    // (latest trunk assumed) or exact OBJECT_VERSION_ID, and an item path
    // selects interior nodes (master11 §"Item URIs").
    let db = testkit::db().await.expect("testkit database");
    let svc = EhrbaseService::new(db.pool());

    let ehr_uuid = svc.create_ehr(None).await.expect("ehr");
    let comp_ovid = svc
        .create_composition(ehr_uuid, uv(composition("Uri target"), "249", None))
        .await
        .expect("composition_create")
        .version_uid();
    let comp_vo = comp_ovid.split("::").next().unwrap();

    // Latest-trunk form: ehr:/{ehr_id}/compositions/{vo_id}.
    let uri: openehr_rm::paths::EhrUri = format!("ehr:/{ehr_uuid}/compositions/{comp_vo}")
        .parse()
        .expect("latest-form uri");
    let comp = svc.resolve_ehr_uri(&uri).await.expect("resolve latest");
    assert_eq!(comp["_type"], "COMPOSITION");
    assert_eq!(uid(&comp), comp_ovid);

    // Exact-version form: ehr:/{ehr_id}/compositions/{OBJECT_VERSION_ID}.
    let uri: openehr_rm::paths::EhrUri = format!("ehr:/{ehr_uuid}/compositions/{comp_ovid}")
        .parse()
        .expect("exact-version uri");
    let same = svc.resolve_ehr_uri(&uri).await.expect("resolve exact");
    assert_eq!(uid(&same), comp_ovid);

    // Item path into the structure: /name/value is a unique leaf.
    let uri: openehr_rm::paths::EhrUri =
        format!("ehr:/{ehr_uuid}/compositions/{comp_vo}/name/value")
            .parse()
            .expect("item-path uri");
    let name = svc.resolve_ehr_uri(&uri).await.expect("resolve item path");
    assert_eq!(name, json!("Uri target"));

    // The attribute-only `directory` locator resolves EHR.directory once a
    // hierarchy exists (= folders.item(1), RM ehr §EHR Class
    // Directory_in_folders).
    svc.create_directory(ehr_uuid, uv(folder("root"), "249", None))
        .await
        .expect("directory create");
    let uri: openehr_rm::paths::EhrUri = format!("ehr:/{ehr_uuid}/directory")
        .parse()
        .expect("directory uri");
    let dir = svc.resolve_ehr_uri(&uri).await.expect("resolve directory");
    assert_eq!(dir["_type"], "FOLDER");

    // A foreign system id is not locally resolvable (master11 §"EHR URIs":
    // cross-system name resolution is unspecified) → NotFound.
    let uri: openehr_rm::paths::EhrUri =
        format!("ehr://foreign.example.org/{ehr_uuid}/compositions/{comp_vo}")
            .parse()
            .expect("foreign uri");
    let err = svc.resolve_ehr_uri(&uri).await.expect_err("foreign system");
    assert!(err.to_string().contains("foreign system"), "got {err}");
}

/// The discrete `I_EHR_STATUS` mutators (`i_ehr_status.adoc`
/// §set/clear_ehr_queryable, §set/clear_ehr_modifiable, §update_other_details):
/// each commits a new implicit-CONTRIBUTION EHR_STATUS version and its
/// post-condition is observable via `get_ehr_status`. Critically,
/// `clear_ehr_modifiable` must stay committable on the EHR it disables and
/// `set_ehr_modifiable` must undo it (EHR_STATUS "is always modifiable",
/// ehr/master04 §"EHR Active Status").
#[tokio::test]
async fn ehr_status_discrete_mutators() {
    let db = testkit::db().await.expect("testkit database");
    let svc = EhrbaseService::new(db.pool());

    let ehr_uuid = svc.create_ehr(None).await.expect("ehr_create");

    // Defaults: both flags true (default_ehr_status).
    let v1 = svc
        .get_ehr_status_at_time(ehr_uuid, None)
        .await
        .expect("status v1");
    assert_eq!(v1["is_queryable"], json!(true));
    assert_eq!(v1["is_modifiable"], json!(true));
    assert!(uid(&v1).ends_with("::1"));

    // clear_ehr_queryable → false; a new trunk version is committed.
    let q_off = svc
        .clear_ehr_queryable(ehr_uuid)
        .await
        .expect("clear_ehr_queryable");
    assert!(q_off.ends_with("::2"), "got {q_off}");
    let s = svc
        .get_ehr_status_at_time(ehr_uuid, None)
        .await
        .expect("status");
    assert_eq!(s["is_queryable"], json!(false));
    assert_eq!(s["is_modifiable"], json!(true), "unrelated flag untouched");

    // set_ehr_queryable → true again.
    let q_on = svc
        .set_ehr_queryable(ehr_uuid)
        .await
        .expect("set_ehr_queryable");
    assert!(q_on.ends_with("::3"), "got {q_on}");
    assert_eq!(
        svc.get_ehr_status_at_time(ehr_uuid, None)
            .await
            .expect("status")["is_queryable"],
        json!(true)
    );

    // clear_ehr_modifiable deactivates the EHR's contents — the STATUS write
    // itself must still succeed (EHR_STATUS is always modifiable).
    let m_off = svc
        .clear_ehr_modifiable(ehr_uuid)
        .await
        .expect("clear_ehr_modifiable must be committable on the EHR it disables");
    assert!(m_off.ends_with("::4"), "got {m_off}");
    assert_eq!(
        svc.get_ehr_status_at_time(ehr_uuid, None)
            .await
            .expect("status")["is_modifiable"],
        json!(false)
    );

    // With the EHR deactivated, a CONTENT write is refused (write guard) …
    let blocked = svc
        .create_composition(ehr_uuid, uv(composition("blocked"), "249", None))
        .await;
    assert!(
        blocked.is_err(),
        "content write on a non-modifiable EHR must be refused, got {blocked:?}"
    );

    // … yet set_ehr_modifiable reactivates it (the STATUS mutator is not gated).
    let m_on = svc
        .set_ehr_modifiable(ehr_uuid)
        .await
        .expect("set_ehr_modifiable reactivates");
    assert!(m_on.ends_with("::5"), "got {m_on}");
    assert_eq!(
        svc.get_ehr_status_at_time(ehr_uuid, None)
            .await
            .expect("status")["is_modifiable"],
        json!(true)
    );
    // Reactivated: content writes now succeed.
    svc.create_composition(ehr_uuid, uv(composition("allowed"), "249", None))
        .await
        .expect("content write after reactivation");

    // update_other_details replaces the ITEM_TREE.
    let details = json!({
        "_type": "ITEM_TREE",
        "archetype_node_id": "at0001",
        "name": { "_type": "DV_TEXT", "value": "other_details" },
        "items": []
    });
    let od = svc
        .update_other_details(ehr_uuid, details.clone())
        .await
        .expect("update_other_details");
    assert!(od.ends_with("::6"), "got {od}");
    let after = svc
        .get_ehr_status_at_time(ehr_uuid, None)
        .await
        .expect("status");
    assert_eq!(after["other_details"]["_type"], "ITEM_TREE");
    assert_eq!(after["other_details"]["archetype_node_id"], "at0001");
}

/// `I_EHR_DIRECTORY` §has_directory_version + §get_versioned_directory: the
/// existence check is true for real versions (incl. across updates) and false
/// for an unknown version id; the VERSIONED_OBJECT view carries the mandatory
/// `time_created` (VERSIONED_OBJECT.time_created, RM common change_control).
#[tokio::test]
async fn directory_versioned_and_has_version() {
    let db = testkit::db().await.expect("testkit database");
    let svc = EhrbaseService::new(db.pool());

    let ehr_uuid = svc.create_ehr(None).await.expect("ehr_create");

    let dir_v1 = svc
        .create_directory(ehr_uuid, uv(folder("root"), "249", None))
        .await
        .expect("directory_create")
        .uid;
    assert!(dir_v1.ends_with("::1"));

    // has_directory_version: true for the real v1 OBJECT_VERSION_ID.
    assert!(
        svc.has_directory_version(ehr_uuid, dir_v1.parse().expect("ovid"))
            .await
            .expect("has_directory_version v1"),
        "the committed directory version must exist"
    );

    // False for a version that does not exist (a v2 that was never committed).
    let sp: Vec<&str> = dir_v1.split("::").collect();
    let phantom = format!("{}::{}::2", sp[0], sp[1]);
    assert!(
        !svc.has_directory_version(ehr_uuid, phantom.parse().expect("ovid"))
            .await
            .expect("has_directory_version phantom"),
        "an uncommitted version id must not exist"
    );

    // False for an unrelated versioned-object id.
    let alien = format!("{}::{}::1", uuid::Uuid::new_v4(), sp[1]);
    assert!(
        !svc.has_directory_version(ehr_uuid, alien.parse().expect("ovid"))
            .await
            .expect("has_directory_version alien"),
        "a foreign vo id is not this EHR's directory"
    );

    // Update → v2; both versions now exist.
    let mut folder_v2 = folder("root");
    folder_v2["name"]["value"] = json!("root-renamed");
    let dir_v2 = svc
        .update_directory(ehr_uuid, uv(folder_v2, "251", Some(&dir_v1)))
        .await
        .expect("directory_update")
        .uid;
    assert!(dir_v2.ends_with("::2"));
    assert!(
        svc.has_directory_version(ehr_uuid, dir_v2.parse().expect("ovid"))
            .await
            .expect("has v2")
    );

    // get_versioned_directory: the VERSIONED_OBJECT view.
    let versioned = svc
        .get_versioned_directory(ehr_uuid)
        .await
        .expect("get_versioned_directory");
    // RM ehr master04: the concrete binding VERSIONED_FOLDER.
    assert_eq!(versioned["_type"], "VERSIONED_FOLDER");
    assert_eq!(versioned["uid"]["value"], sp[0]);
    assert_eq!(versioned["owner_id"]["id"]["value"], ehr_uuid.to_string());
    assert!(
        versioned["time_created"]["value"].is_string(),
        "VERSIONED_OBJECT.time_created must be present, got {versioned}"
    );
}

/// The P20-item-33 write-path fixes must not change the wire: a write response
/// built from the CONTRIBUTION commit results (never a post-commit re-read) has
/// to be identical to what a fresh read yields.
///
/// - Fix E: the EHR create representation is assembled from the commit `Committed`
///   rows (`ehr_created_object`, from the create-time stash) and MUST be
///   byte-identical to a fresh `ehr_summary` read (`ehr_object`) for a new EHR.
/// - Fix D: the DIRECTORY create/update response `OBJECT_VERSION_ID`
///   (`committed_response`) MUST equal the `uid` a fresh read injects (RM common
///   master06 §Committal: the written version identity).
/// - Item 34: the EHR_STATUS update response `OBJECT_VERSION_ID`
///   (`committed_response`, replacing the discarded post-commit reassembly) MUST
///   equal a fresh read's `uid`, and the mutation MUST persist (the folded
///   subject/is_queryable sync rides the write's UPDATE).
#[tokio::test]
async fn write_responses_match_a_fresh_read() {
    let db = testkit::db().await.expect("testkit database");
    let svc = EhrbaseService::new(db.pool());

    // Fix E — built-from-commit EHR body == fresh ehr_summary read.
    let ehr_uuid = svc.create_ehr(None).await.expect("ehr_create");
    let built = svc
        .ehr_created_object(ehr_uuid)
        .await
        .expect("ehr_created_object (built from commit, stash)");
    let fresh = svc
        .ehr_object(ehr_uuid)
        .await
        .expect("ehr_object (fresh read)");
    assert_eq!(
        built, fresh,
        "the EHR body built from the commit results must equal a fresh ehr_summary read"
    );

    // Fix D — directory create response uid == fresh read uid.
    let dir_v1 = svc
        .create_directory(ehr_uuid, uv(folder("root"), "249", None))
        .await
        .expect("directory_create")
        .uid;
    let got_v1 = svc
        .get_directory_at_time(ehr_uuid, None, None)
        .await
        .expect("dir get v1")
        .body;
    assert_eq!(
        got_v1["uid"]["value"], dir_v1,
        "the create ETag (committed_response) must equal the stored version uid"
    );

    // Fix D — directory update response uid == fresh read uid.
    let mut folder_v2 = folder("root");
    folder_v2["name"]["value"] = json!("root2");
    let dir_v2 = svc
        .update_directory(ehr_uuid, uv(folder_v2, "251", Some(&dir_v1)))
        .await
        .expect("directory_update")
        .uid;
    let got_v2 = svc
        .get_directory_at_time(ehr_uuid, None, None)
        .await
        .expect("dir get v2")
        .body;
    assert_eq!(
        got_v2["uid"]["value"], dir_v2,
        "the update ETag (committed_response) must equal the stored version uid"
    );
    assert!(dir_v2.ends_with("::2"), "update yields trunk version 2");

    // Item 34 — EHR_STATUS update response uid (committed_response, no
    // post-commit reassembly) == fresh read uid, and the `is_queryable` mutation
    // persists (the folded subject/is_queryable sync rode the write's UPDATE).
    let status_v1 = svc
        .get_ehr_status(ehr_uuid)
        .await
        .expect("get_ehr_status v1");
    let status_uid_v1 = status_v1["uid"]["value"]
        .as_str()
        .expect("status uid v1")
        .to_owned();
    let mut status_body = status_v1.clone();
    status_body
        .as_object_mut()
        .expect("status object")
        .remove("uid");
    status_body["is_queryable"] = json!(false);
    let status_uid_v2 = svc
        .replace_ehr_status(ehr_uuid, uv(status_body, "251", Some(&status_uid_v1)))
        .await
        .expect("replace_ehr_status");
    assert!(
        status_uid_v2.ends_with("::2"),
        "status update yields trunk version 2"
    );
    let fresh_status = svc
        .get_ehr_status(ehr_uuid)
        .await
        .expect("get_ehr_status v2");
    assert_eq!(
        fresh_status["uid"]["value"], status_uid_v2,
        "the status update ETag (committed_response) must equal the stored version uid"
    );
    assert_eq!(
        fresh_status["is_queryable"],
        json!(false),
        "the EHR_STATUS mutation persisted through the folded write"
    );
}
