#![allow(
    clippy::panic,
    clippy::print_stdout,
    clippy::print_stderr,
    let_underscore_drop
)] // test assertions/diagnostics/fixtures
//! End-to-end service tests for the EHR Extract **export** path (SM
//! `I_EHR_EXTRACT_SERVICE.export_ehrs` / `export_ehr_extracts`) against a real
//! `PostgreSQL` 18 (shared testkit harness).
//!
//! Spec: SM `docs/specs/openehr/SM/docs/UML/classes/i_ehr_extract_service.adoc`;
//! RM EHR Extract IM master05 (`X_VERSIONED_*`) + master09 (creation
//! semantics). The acceptance properties:
//!
//! 1. **Whole-EHR export** carries every versioned object of the EHR
//!    (`EHR_STATUS`, `EHR_ACCESS`, directory `FOLDER`) as an
//!    `OPENEHR_CONTENT_ITEM` wrapping the matching `X_VERSIONED_<kind>`, latest
//!    version only, with the version data byte-equal (canonical JSON) to what
//!    the read surface serves.
//! 2. **Spec-driven export** honours the `EXTRACT_ENTITY_MANIFEST.item_list`
//!    (only the named version container) and the `EXTRACT_VERSION_SPEC`
//!    (`include_all_versions` ⇒ every version; `include_revision_history` ⇒ the
//!    full `REVISION_HISTORY`).
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::too_many_lines)]

use serde_json::{Value, json};

use openehr_base::prelude::TerminologyCode;
use openehr_rm::ehr_extract::common::extract_spec::ExtractSpec;
use openehr_rm::prelude::PartyProxy;

use ehrbase::service::EhrbaseService;
use ehrbase::service::version_update::{UpdateAudit, UpdateVersion};

fn term(code: &str) -> TerminologyCode {
    TerminologyCode {
        terminology_id: "openehr".to_owned(),
        terminology_version: None,
        code_string: code.to_owned(),
        uri: None,
    }
}

fn uv(data: Value, change_code: &str, preceding: Option<&str>) -> UpdateVersion {
    UpdateVersion {
        preceding_version_uid: preceding.map(|p| p.parse().expect("OBJECT_VERSION_ID")),
        lifecycle_state: term("532"),
        attestations: None,
        data,
        audit: UpdateAudit {
            change_type: term(change_code),
            description: None,
            committer: openehr_its::json::from_canonical_value::<PartyProxy>(
                &json!({ "_type": "PARTY_IDENTIFIED", "name": "conformance tester" }),
            )
            .expect("committer"),
            system_id: None,
        },
        signature: None,
    }
}

/// Seed an EHR with an `EHR_STATUS` (create → update = two versions), a directory
/// `FOLDER`, and the auto-created `EHR_ACCESS`. Returns the EHR id and the
/// `EHR_STATUS` version-container uid.
async fn seed_ehr(svc: &EhrbaseService) -> (ehrbase::ids::EhrId, String) {
    let ehr_uuid = svc.create_ehr(None).await.expect("ehr");

    let mut status = svc
        .get_ehr_status_at_time(ehr_uuid, None)
        .await
        .expect("status get");
    let status_ovid = status["uid"]["value"].as_str().expect("uid").to_owned();
    let status_vo = status_ovid.split("::").next().unwrap().to_owned();
    status.as_object_mut().expect("status obj").remove("uid");

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

    status["is_modifiable"] = json!(false);
    svc.replace_ehr_status(ehr_uuid, uv(status, "251", Some(&status_ovid)))
        .await
        .expect("status update");

    (ehr_uuid, status_vo)
}

/// The content items of the (single) chapter of an extract.
fn items(extract: &Value) -> &Vec<Value> {
    extract["chapters"][0]["items"]
        .as_array()
        .expect("chapter items")
}

/// The content item whose wrapped object has the given `X_VERSIONED_*` `_type`.
fn find_by_xtype<'a>(extract: &'a Value, xtype: &str) -> Option<&'a Value> {
    items(extract)
        .iter()
        .find(|it| it["item"]["_type"] == json!(xtype))
}

#[tokio::test]
async fn export_ehrs_carries_every_versioned_object_latest_only() {
    let db = testkit::db().await.expect("testkit database");
    let svc = EhrbaseService::new(db.pool());
    let (ehr, _status_vo) = seed_ehr(&svc).await;

    let extracts = svc.extract_ehrs(ehr).await.expect("export_ehrs");
    assert_eq!(
        extracts.len(),
        1,
        "one EHR id → a one-element List<EXTRACT>"
    );
    let extract = &extracts[0];

    // EXTRACT envelope.
    assert_eq!(extract["_type"], json!("EXTRACT"));
    assert_eq!(extract["sequence_nr"], json!(1));
    assert_eq!(extract["chapters"][0]["_type"], json!("EXTRACT_CHAPTER"));
    assert!(
        extract["system_id"]["value"].is_string(),
        "EXTRACT.system_id is a HIER_OBJECT_ID"
    );

    // All three versioned objects present as primary content items.
    for xtype in [
        "X_VERSIONED_EHR_STATUS",
        "X_VERSIONED_EHR_ACCESS",
        "X_VERSIONED_FOLDER",
    ] {
        let item =
            find_by_xtype(extract, xtype).unwrap_or_else(|| panic!("extract must contain {xtype}"));
        assert_eq!(item["_type"], json!("OPENEHR_CONTENT_ITEM"));
        assert_eq!(item["is_primary"], json!(true), "{xtype} is primary");
    }

    // EHR_STATUS: two stored versions, latest only in the extract.
    let status_x = &find_by_xtype(extract, "X_VERSIONED_EHR_STATUS").unwrap()["item"];
    assert_eq!(
        status_x["total_version_count"],
        json!(2),
        "create + update = two versions stored"
    );
    assert_eq!(
        status_x["extract_version_count"],
        json!(1),
        "latest-only default includes one version"
    );
    let versions = status_x["versions"].as_array().expect("versions");
    assert_eq!(versions.len(), 1);
    assert_eq!(versions[0]["_type"], json!("ORIGINAL_VERSION"));

    // Canonical-JSON faithfulness: the exported version data is byte-equal to
    // the read surface's current EHR_STATUS — one shape since commit-time uid
    // stamping (#439).
    let current = svc.get_ehr_status_at_time(ehr, None).await.expect("read");
    assert_eq!(
        versions[0]["data"], current,
        "exported EHR_STATUS data must match the stored canonical content"
    );
    assert_eq!(
        versions[0]["data"]["is_modifiable"],
        json!(false),
        "latest EHR_STATUS reflects the update (is_modifiable = false)"
    );
}

#[tokio::test]
async fn export_ehr_extracts_honours_item_list_and_all_versions() {
    let db = testkit::db().await.expect("testkit database");
    let svc = EhrbaseService::new(db.pool());
    let (ehr, status_vo) = seed_ehr(&svc).await;

    // Request only the EHR_STATUS version container, all versions, with revision
    // history.
    let spec: ExtractSpec = openehr_its::json::from_canonical_value(&json!({
        "_type": "EXTRACT_SPEC",
        "version_spec": {
            "_type": "EXTRACT_VERSION_SPEC",
            "include_all_versions": true,
            "include_revision_history": true,
            "include_data": true
        },
        "manifest": {
            "_type": "EXTRACT_MANIFEST",
            "entities": [ {
                "_type": "EXTRACT_ENTITY_MANIFEST",
                "extract_id_key": ehr.to_string(),
                "ehr_id": ehr.to_string(),
                "other_ids": [],
                "item_list": [ {
                    "_type": "OBJECT_REF",
                    "namespace": "local",
                    "type": "VERSIONED_EHR_STATUS",
                    "id": { "_type": "HIER_OBJECT_ID", "value": status_vo }
                } ]
            } ]
        },
        "extract_type": {
            "_type": "DV_CODED_TEXT",
            "value": "openehr-ehr",
            "defining_code": {
                "_type": "CODE_PHRASE",
                "terminology_id": { "_type": "TERMINOLOGY_ID", "value": "openehr" },
                "code_string": "openehr-ehr"
            }
        },
        "include_multimedia": true,
        "priority": 0,
        "link_depth": 0,
        "criteria": []
    }))
    .expect("EXTRACT_SPEC");

    let extracts = svc
        .export_ehr_extracts(spec)
        .await
        .expect("export_ehr_extracts");
    assert_eq!(extracts.len(), 1, "one entity → one EXTRACT");
    let extract = &extracts[0];

    // Only the requested EHR_STATUS container is in the extract.
    let its = items(extract);
    assert_eq!(its.len(), 1, "item_list restricts to one content item");
    let status_x =
        &find_by_xtype(extract, "X_VERSIONED_EHR_STATUS").expect("EHR_STATUS present")["item"];

    // All versions included, plus the full revision history.
    assert_eq!(status_x["total_version_count"], json!(2));
    assert_eq!(status_x["extract_version_count"], json!(2));
    let versions = status_x["versions"].as_array().expect("versions");
    assert_eq!(versions.len(), 2);
    assert_eq!(
        versions[0]["data"]["is_modifiable"],
        json!(true),
        "v1 is the original (modifiable)"
    );
    assert_eq!(
        versions[1]["data"]["is_modifiable"],
        json!(false),
        "v2 is the update (is_modifiable = false)"
    );

    let history = &status_x["revision_history"];
    assert_eq!(history["_type"], json!("REVISION_HISTORY"));
    assert_eq!(
        history["items"].as_array().expect("history items").len(),
        2,
        "revision history has one item per version"
    );
}

#[tokio::test]
async fn export_ehrs_unknown_ehr_is_ehr_id_does_not_exist() {
    let db = testkit::db().await.expect("testkit database");
    let svc = EhrbaseService::new(db.pool());
    let err = svc
        .extract_ehrs(ehrbase::ids::EhrId::new())
        .await
        .expect_err("unknown EHR must fail");
    assert_eq!(
        err.status,
        ehrbase::service::status::CallStatusType::EhrIdDoesNotExist
    );
}

/// A1 rm-ehr-extract: `EXTRACT_SPEC.extract_type` must come from the extract
/// content type group; `include_multimedia` = false strips inline `DV_MULTIMEDIA`
/// data from exported versions (master09 §Creation Semantics); an
/// `EXTRACT_CONTENT_ITEM` that is masked yet carries an item violates
/// `Item_validity` on import (`extract_content_item.adoc`).
#[tokio::test]
async fn extract_spec_flags_are_honoured() {
    let db = testkit::db().await.expect("testkit database");
    let svc = EhrbaseService::new(db.pool());
    let (ehr, _status_vo) = seed_ehr(&svc).await;

    // Bad extract_type → precondition.
    let mut spec = json!({
        "_type": "EXTRACT_SPEC",
        "version_spec": { "_type": "EXTRACT_VERSION_SPEC",
            "include_all_versions": false, "include_revision_history": false,
            "include_data": true },
        "manifest": { "_type": "EXTRACT_MANIFEST", "entities": [ {
            "_type": "EXTRACT_ENTITY_MANIFEST",
            "extract_id_key": ehr.to_string(), "ehr_id": ehr.to_string(),
            "other_ids": [], "item_list": [] } ] },
        "extract_type": { "_type": "DV_CODED_TEXT", "value": "bogus",
            "defining_code": { "_type": "CODE_PHRASE", "code_string": "bogus",
                "terminology_id": { "_type": "TERMINOLOGY_ID", "value": "openehr" } } },
        "include_multimedia": true, "priority": 0, "link_depth": 0, "criteria": []
    });
    let err = svc
        .export_ehr_extracts(openehr_its::json::from_canonical_value(&spec).expect("spec"))
        .await
        .expect_err("an extract_type outside the group must be rejected");
    assert!(
        err.message.contains("extract content type"),
        "{}",
        err.message
    );

    // Valid type + include_multimedia = false → exported bodies carry no
    // inline DV_MULTIMEDIA data.
    spec["extract_type"]["defining_code"]["code_string"] = json!("openehr-ehr");
    spec["include_multimedia"] = json!(false);
    let extracts = svc
        .export_ehr_extracts(openehr_its::json::from_canonical_value(&spec).expect("spec"))
        .await
        .expect("export");
    let wire = serde_json::to_string(&extracts[0]).unwrap();
    assert!(
        !wire.contains("\"data\":\"") || !wire.contains("DV_MULTIMEDIA"),
        "no inline multimedia data may remain when include_multimedia = false"
    );

    // Masked-with-item violates Item_validity on import.
    let mut extract = extracts.into_iter().next().unwrap();
    extract["chapters"][0]["items"][0]["is_masked"] = json!(true);
    let err = svc
        .import_ehr(
            Some(ehrbase::ids::EhrId::new()),
            openehr_its::json::from_canonical_value(&extract).expect("extract"),
        )
        .await
        .expect_err("masked item carrying content must be rejected");
    assert!(err.message.contains("Item_validity"), "{}", err.message);
}
