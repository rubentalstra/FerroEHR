// SPDX-FileCopyrightText: FerroEHR contributors
// SPDX-License-Identifier: MIT

//! FOLDER `items` referential validation (issue #1951, register AMB-211):
//! a reference whose namespace claims THIS system must resolve to a
//! versioned object in the EHR; foreign and `unknown` namespaces pass
//! verbatim (BASE `object_ref.adoc` — distributed referencing). The check
//! binds the direct directory routes and the CONTRIBUTION path (inside the
//! commit transaction, after the set's inserts, so the verdict never
//! depends on member order).

#![expect(
    clippy::expect_used,
    reason = "clippy's in-test lint scoping (clippy.toml `allow-*-in-tests`) only \
              reaches `#[test]`-annotated functions, so it misses this integration \
              module's helpers and async bodies; panicking assertions are the \
              intended shape here (the Rust Book ch11)"
)]

use serde_json::{Value, json};

use ferroehr::service::status::{CallStatusType, SmError};
use ferroehr::service::version_update::{change_type_coded, lifecycle_state_coded};
use ferroehr::service::{DEFAULT_SYSTEM_ID, FerroEhrService};
use openehr_its::rest::generated::common::{UpdateAudit, UpdateAuditData, UpdateVersion};
use openehr_rm::prelude::PartyProxy;

/// The SM `UPDATE_VERSION` commit envelope for a bare-RM write.
fn uv<T: serde::de::DeserializeOwned>(
    data: &Value,
    change_code: &str,
    preceding: Option<&str>,
) -> UpdateVersion<T> {
    UpdateVersion {
        preceding_version_uid: preceding.map(|p| p.parse().expect("OBJECT_VERSION_ID")),
        lifecycle_state: lifecycle_state_coded("532"),
        attestations: None,
        data: openehr_its::json::from_canonical_value(data)
            .expect("the fixture commit body decodes as its RM type"),
        commit_audit: UpdateAudit::UpdateAudit(UpdateAuditData {
            _type: None,
            system_id: None,
            change_type: change_type_coded(change_code),
            description: None,
            committer: openehr_its::json::from_canonical_value::<PartyProxy>(
                &json!({ "_type": "PARTY_IDENTIFIED", "name": "conformance tester" }),
            )
            .expect("committer"),
        }),
        signature: None,
    }
}

/// A minimal valid RM COMPOSITION (`language`/`territory`/`category`/
/// `composer` are `1..1`).
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

/// A root FOLDER whose single `items` entry references `(namespace, id)`.
fn folder_with_item(namespace: &str, id_value: &str) -> Value {
    json!({
        "_type": "FOLDER",
        "archetype_node_id": "openEHR-EHR-FOLDER.generic.v1",
        "name": { "_type": "DV_TEXT", "value": "root" },
        "items": [{
            "_type": "OBJECT_REF",
            "namespace": namespace,
            "type": "VERSIONED_COMPOSITION",
            "id": { "_type": "HIER_OBJECT_ID", "value": id_value }
        }]
    })
}

fn assert_unresolvable(err: &SmError, expected_path_fragment: &str) {
    assert_eq!(
        err.status,
        CallStatusType::ContentInvalid,
        "an unresolvable this-system reference is content-invalid (→ 422), got {err:?}"
    );
    assert!(
        err.message.contains(expected_path_fragment),
        "the refusal names the dangling reference's tree path; got: {}",
        err.message
    );
}

#[tokio::test]
async fn direct_directory_routes_gate_this_system_refs() {
    let db = testkit::db().await.expect("testkit database");
    let svc = FerroEhrService::new(db.pool());
    let ehr_uuid = svc.create_ehr(None).await.expect("ehr_create");

    // A this-system (`local`) reference to a uid that exists nowhere: refused,
    // naming the item's path.
    let dangling = folder_with_item("local", "8f8f0d7e-1111-4e6e-9c3a-000000000001");
    let err = svc
        .create_directory(ehr_uuid, uv(&dangling, "249", None))
        .await
        .expect_err("a dangling local reference must be refused");
    assert_unresolvable(&err, "/items[0]");

    // The configured system id claims this system exactly like `local`.
    let dangling_sysid =
        folder_with_item(DEFAULT_SYSTEM_ID, "8f8f0d7e-1111-4e6e-9c3a-000000000002");
    let err = svc
        .create_directory(ehr_uuid, uv(&dangling_sysid, "249", None))
        .await
        .expect_err("the configured system id claims this system");
    assert_unresolvable(&err, "/items[0]");

    // Foreign and `unknown` namespaces are distributed references (BASE
    // `object_ref.adoc`) — accepted verbatim, resolvable or not.
    let foreign = folder_with_item("my.system.id", "8f8f0d7e-1111-4e6e-9c3a-000000000003");
    let meta = svc
        .create_directory(ehr_uuid, uv(&foreign, "249", None))
        .await
        .expect("a foreign-namespace reference is accepted unchecked");
    svc.delete_directory(ehr_uuid, Some(meta.uid.parse().expect("ovid")), None)
        .await
        .expect("cleanup foreign-ref directory");
    let unknown = folder_with_item("unknown", "8f8f0d7e-1111-4e6e-9c3a-000000000004");
    let meta = svc
        .create_directory(ehr_uuid, uv(&unknown, "249", None))
        .await
        .expect("BASE legalizes namespace 'unknown' — accepted unchecked");
    svc.delete_directory(ehr_uuid, Some(meta.uid.parse().expect("ovid")), None)
        .await
        .expect("cleanup unknown-ref directory");

    // A local reference that RESOLVES: commit a composition, reference its
    // versioned object — accepted, in a nested sub-folder too (the walker
    // recurses and paths name the nesting).
    let committed = svc
        .create_composition(ehr_uuid, uv(&composition("Referenced"), "249", None))
        .await
        .expect("composition_create");
    let comp_root = committed.vo_id.to_string();
    let nested = json!({
        "_type": "FOLDER",
        "archetype_node_id": "openEHR-EHR-FOLDER.generic.v1",
        "name": { "_type": "DV_TEXT", "value": "root" },
        "folders": [{
            "_type": "FOLDER",
            "archetype_node_id": "openEHR-EHR-FOLDER.generic.v1",
            "name": { "_type": "DV_TEXT", "value": "episodes" },
            "items": [{
                "_type": "OBJECT_REF",
                "namespace": "local",
                "type": "VERSIONED_COMPOSITION",
                "id": { "_type": "HIER_OBJECT_ID", "value": comp_root }
            }]
        }]
    });
    let meta = svc
        .create_directory(ehr_uuid, uv(&nested, "249", None))
        .await
        .expect("a resolvable local reference is accepted");

    // The UPDATE path gates identically, and the violation path names the
    // nested location.
    let mut bad_update = nested.clone();
    bad_update["folders"][0]["items"][0]["id"]["value"] =
        json!("8f8f0d7e-1111-4e6e-9c3a-000000000005");
    let err = svc
        .update_directory(ehr_uuid, uv(&bad_update, "251", Some(&meta.uid)))
        .await
        .expect_err("a dangling local reference is refused on update too");
    assert_unresolvable(&err, "/folders[0]/items[0]");
}

#[tokio::test]
async fn contribution_path_gates_this_system_refs() {
    let db = testkit::db().await.expect("testkit database");
    let svc = FerroEhrService::new(db.pool());
    let ehr_uuid = svc.create_ehr(None).await.expect("ehr_create");

    // Version identity is repository-allocated (a CONTRIBUTION member never
    // carries its own uid), so a folder references content committed EARLIER:
    // contribution A creates the composition, contribution B commits a folder
    // hierarchy whose local reference names A's allocated uid — accepted
    // through the CONTRIBUTION path.
    let complete = json!({
        "_type": "DV_CODED_TEXT", "value": "complete",
        "defining_code": {
            "_type": "CODE_PHRASE",
            "terminology_id": { "_type": "TERMINOLOGY_ID", "value": "openehr" },
            "code_string": "532"
        }
    });
    let audit = json!({
        "change_type": {
            "_type": "DV_CODED_TEXT", "value": "creation",
            "defining_code": {
                "_type": "CODE_PHRASE",
                "terminology_id": { "_type": "TERMINOLOGY_ID", "value": "openehr" },
                "code_string": "249"
            }
        },
        "committer": { "_type": "PARTY_IDENTIFIED", "name": "author" }
    });
    let create_composition = json!({
        "_type": "CONTRIBUTION",
        "versions": [{
            "_type": "ORIGINAL_VERSION",
            "commit_audit": audit,
            "lifecycle_state": complete.clone(),
            "data": composition("Referenced through a contribution")
        }],
        "audit": { "change_type": audit["change_type"], "committer": audit["committer"] }
    });
    let created = svc
        .create_ehr_contribution(ehr_uuid, create_composition)
        .await
        .expect("contribution A: composition create");
    let comp_ovid = created.body["versions"][0]["id"]["value"]
        .as_str()
        .expect("the created version's OBJECT_VERSION_ID")
        .to_owned();
    let comp_root = comp_ovid.split("::").next().expect("uid root").to_owned();

    let mut folder = folder_with_item("local", &comp_root);
    folder["name"]["value"] = json!("episodes hierarchy");
    let contribution = json!({
        "_type": "CONTRIBUTION",
        "versions": [{
            "_type": "ORIGINAL_VERSION",
            "commit_audit": audit,
            "lifecycle_state": complete.clone(),
            "data": folder
        }],
        "audit": { "change_type": audit["change_type"], "committer": audit["committer"] }
    });
    svc.create_ehr_contribution(ehr_uuid, contribution)
        .await
        .expect("contribution B: a folder referencing A's allocated uid commits");

    // The same shape with a reference that nothing in the set (or the EHR)
    // carries: the WHOLE set is refused — the transaction rolls back.
    let dangling = json!({
        "_type": "CONTRIBUTION",
        "versions": [{
            "_type": "ORIGINAL_VERSION",
            "commit_audit": audit,
            "lifecycle_state": complete.clone(),
            "data": {
                "_type": "FOLDER",
                "archetype_node_id": "openEHR-EHR-FOLDER.generic.v1",
                "name": { "_type": "DV_TEXT", "value": "second hierarchy" },
                "items": [{
                    "_type": "OBJECT_REF",
                    "namespace": "local",
                    "type": "VERSIONED_COMPOSITION",
                    "id": { "_type": "HIER_OBJECT_ID",
                            "value": "1e5c28d2-2222-4aaa-9bbb-000000000000" }
                }]
            }
        }],
        "audit": { "change_type": audit["change_type"], "committer": audit["committer"] }
    });
    let err = svc
        .create_ehr_contribution(ehr_uuid, dangling)
        .await
        .expect_err("a dangling this-system reference refuses the whole set");
    assert_eq!(
        err.status,
        CallStatusType::ContentInvalid,
        "contribution refusal is content-invalid, got {err:?}"
    );
    assert!(err.message.contains("/items[0]"), "got: {}", err.message);
}
