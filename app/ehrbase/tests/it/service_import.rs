//! End-to-end service tests for the EHR Extract **import** path (SM
//! `I_EHR_EXTRACT_SERVICE.import_ehr` / `import_ehr_extract`) against a real
//! `PostgreSQL` 18 (shared testkit harness).
//!
//! Spec: SM `docs/specs/openehr/SM/docs/UML/classes/i_ehr_extract_service.adoc`;
//! RM EHR Extract IM master05 (`X_VERSIONED_*`) + RM common
//! `docs/specs/openehr/RM/docs/common/master06-change_control_package.adoc`
//! §Copying (`IMPORTED_VERSION` semantics, Cases 1/2/3) + §Committal. The
//! acceptance properties:
//!
//! 1. **Whole-EHR clone** (`import_ehr`) replays a whole-EHR export into an
//!    empty target, reusing the source EHR id when none is given (master06
//!    §Copying Case 1: "the newly created EHR should re-use the EHR identifier
//!    from the source system"), or a caller-provided fixed id (the SM's
//!    "same patient in other EHR services" case). Each imported version keeps
//!    the wrapped `ORIGINAL_VERSION`'s identity (`OBJECT_VERSION_ID`) and data
//!    verbatim — so reading the clone's `EHR_STATUS` returns byte-identical
//!    canonical JSON to the source's.
//! 2. **A duplicate target id** is `ehr_create_fail_duplicate_id` (`import_ehr`
//!    imports into an *empty* target).
//! 3. **Extract import into an existing EHR** (`import_ehr_extract`) lands a new
//!    versioned object (master06 §Copying Case 2: first receipt of an item
//!    clones its `VERSIONED_OBJECT` with the received `uid.object_id()`);
//!    re-importing the same trunk version is a conflict.
//! 4. **The clone is a complete, first-class local EHR**: `EHR.ehr_access` is
//!    1..1 (RM ehr `ehr.adoc` invariant `Ehr_access_valid`; master04 §EHR
//!    Creation — "a root EHR object, an EHR Status object, and an EHR Access
//!    object … created and committed in a Contribution"), so an extract that
//!    carries no `EHR_ACCESS` is completed with the local default; and the
//!    imported `EHR_STATUS.subject` is promoted, so the clone is found by the
//!    subject lookup (SM `I_EHR_SERVICE.get_ehrs_for_subject`,
//!    `operations/ehr_get_by_subject.yaml`) and bound by the
//!    one-EHR-per-subject rule (RM ehr master04 §EHR Status) like any other EHR.

#![expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "clippy's in-test lint scoping (clippy.toml `allow-*-in-tests`) only \
              reaches `#[test]`-annotated functions, so it misses this integration \
              module's helpers and async bodies; panicking assertions and direct \
              fixture indexing are the intended shape here (the Rust Book ch11)"
)]

use serde_json::{Value, json};

use openehr_base::prelude::TerminologyCode;
use openehr_rm::ehr_extract::common::extract::Extract;
use openehr_rm::ehr_extract::common::extract_spec::ExtractSpec;
use openehr_rm::prelude::PartyProxy;

use ehrbase::service::EhrbaseService;
use ehrbase::service::ehr_index::types::SubjectRef;
use ehrbase::service::status::CallStatusType;

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
/// `FOLDER`, and the auto-created `EHR_ACCESS` — the same shape the export tests
/// seed. Returns the EHR id.
async fn seed_ehr(svc: &EhrbaseService) -> ehrbase::ids::EhrId {
    let ehr_uuid = svc.create_ehr(None).await.expect("ehr");

    let mut status = svc
        .get_ehr_status_at_time(ehr_uuid, None)
        .await
        .expect("status get");
    let status_ovid = status["uid"]["value"].as_str().expect("uid").to_owned();
    status.as_object_mut().expect("status obj").remove("uid");

    svc.create_directory(
        ehr_uuid,
        uv(
            json!({
                "_type": "FOLDER",
                "archetype_node_id": "openEHR-EHR-FOLDER.generic.v1",
                "name": { "_type": "DV_TEXT", "value": "root" }
            }),
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

    ehr_uuid
}

/// The single-`EXTRACT` result of an `export_ehrs`, as a typed [`Extract`]
/// (the wire shape the import calls receive).
async fn export_one(svc: &EhrbaseService, ehr: ehrbase::ids::EhrId) -> Extract {
    let mut extracts = svc.extract_ehrs(ehr).await.expect("export_ehrs");
    assert_eq!(extracts.len(), 1, "one EHR id → one EXTRACT");
    openehr_its::json::from_canonical_value(&extracts.remove(0))
        .expect("EXTRACT deserializes into the typed RM model")
}

/// The content item of an extract whose wrapped object has the given
/// `X_VERSIONED_*` `_type`.
fn find_by_xtype<'a>(extract: &'a Value, xtype: &str) -> Option<&'a Value> {
    extract["chapters"][0]["items"]
        .as_array()
        .expect("chapter items")
        .iter()
        .find(|it| it["item"]["_type"] == json!(xtype))
}

/// How many content items of an extract wrap the given `X_VERSIONED_*` `_type`.
fn count_by_xtype(extract: &Value, xtype: &str) -> usize {
    extract["chapters"][0]["items"]
        .as_array()
        .expect("chapter items")
        .iter()
        .filter(|it| it["item"]["_type"] == json!(xtype))
        .count()
}

/// The same extract with every `X_VERSIONED_EHR_ACCESS` content item removed —
/// the export of a source that holds no `EHR_ACCESS`, or one filtered out en
/// route.
fn without_ehr_access(extract: &Value) -> Extract {
    let mut raw = extract.clone();
    let items: Vec<Value> = raw["chapters"][0]["items"]
        .as_array()
        .expect("chapter items")
        .iter()
        .filter(|it| it["item"]["_type"] != json!("X_VERSIONED_EHR_ACCESS"))
        .cloned()
        .collect();
    raw["chapters"][0]["items"] = Value::Array(items);
    openehr_its::json::from_canonical_value(&raw).expect("EXTRACT without EHR_ACCESS")
}

/// [`status_for_subject`] with an explicit namespace (the extract rewrite
/// normalizes carried refs to `"local"` — EHR Extract master09 §Creation
/// Semantics — so conflict fixtures must name it).
fn status_for_subject_in(subject_id: &str, namespace: &str) -> Value {
    let mut status = status_for_subject(subject_id);
    status["subject"]["external_ref"]["namespace"] = json!(namespace);
    status
}

/// An `EHR_STATUS` whose `PARTY_SELF` subject carries an `external_ref` —
/// RM ehr master04 §EHR Status (the subject 0..1 identifies the EHR).
fn status_for_subject(subject_id: &str) -> Value {
    json!({
        "_type": "EHR_STATUS",
        "archetype_node_id": "openEHR-EHR-EHR_STATUS.generic.v1",
        "archetype_details": {
            "_type": "ARCHETYPED",
            "archetype_id": { "_type": "ARCHETYPE_ID",
                              "value": "openEHR-EHR-EHR_STATUS.generic.v1" },
            "rm_version": "1.2.0"
        },
        "name": { "_type": "DV_TEXT", "value": "EHR Status" },
        "subject": {
            "_type": "PARTY_SELF",
            "external_ref": {
                "_type": "PARTY_REF",
                "namespace": "patients",
                "type": "PERSON",
                "id": { "_type": "HIER_OBJECT_ID", "value": subject_id }
            }
        },
        "is_queryable": true,
        "is_modifiable": true
    })
}

#[tokio::test]
async fn import_ehr_clone_into_fresh_target_reuses_source_id() {
    let source_db = testkit::db().await.expect("testkit database");
    let source = EhrbaseService::new(source_db.pool());
    let target_db = testkit::db().await.expect("testkit database");
    let target = EhrbaseService::new(target_db.pool());

    let ehr = seed_ehr(&source).await;
    let source_status = source
        .get_ehr_status_at_time(ehr, None)
        .await
        .expect("source status");

    // No fixed id → the clone reuses the source EHR id (master06 §Copying Case 1).
    let extract = export_one(&source, ehr).await;
    target.import_ehr(None, extract).await.expect("import_ehr");

    // The clone exists under the *same* EHR id in the (previously empty) target.
    let target_status = target
        .get_ehr_status_at_time(ehr, None)
        .await
        .expect("cloned status readable under the reused ehr_id");

    // The wrapped ORIGINAL_VERSION was preserved verbatim: identical uid
    // (OBJECT_VERSION_ID) and data ("the ORIGINAL_VERSION instance is never
    // modified", master06 §Copying).
    assert_eq!(
        target_status, source_status,
        "cloned EHR_STATUS must be byte-identical to the source (uid + data preserved)"
    );
    assert_eq!(
        target_status["is_modifiable"],
        json!(false),
        "the latest (v2) EHR_STATUS was imported, is_modifiable = false"
    );

    // The clone also carries the source's EHR_ACCESS and directory FOLDER.
    let re_export = target.extract_ehrs(ehr).await.expect("re-export clone");
    for xtype in [
        "X_VERSIONED_EHR_STATUS",
        "X_VERSIONED_EHR_ACCESS",
        "X_VERSIONED_FOLDER",
    ] {
        assert!(
            find_by_xtype(&re_export[0], xtype).is_some(),
            "clone must carry {xtype}"
        );
    }
    // Exactly one: an extract that already carries an EHR_ACCESS is never given
    // a second, locally bootstrapped one (EHR.ehr_access is 1..1 — RM ehr
    // `ehr.adoc` invariant `Ehr_access_valid`).
    assert_eq!(
        count_by_xtype(&re_export[0], "X_VERSIONED_EHR_ACCESS"),
        1,
        "the carried EHR_ACCESS is the only one"
    );
}

#[tokio::test]
async fn import_ehr_without_ehr_access_bootstraps_the_mandatory_default() {
    // RM ehr `ehr.adoc` invariant `Ehr_access_valid` makes `EHR.ehr_access`
    // 1..1, and master04 §EHR Creation requires that creating an EHR yields "a
    // root EHR object, an EHR Status object, and an EHR Access object …
    // created and committed in a Contribution". An extract that carries no
    // EHR_ACCESS must therefore be completed locally — otherwise the clone
    // violates the invariant permanently and its served EHR body omits the
    // mandatory reference.
    let source_db = testkit::db().await.expect("testkit database");
    let source = EhrbaseService::new(source_db.pool());
    let target_db = testkit::db().await.expect("testkit database");
    let target = EhrbaseService::new(target_db.pool());

    let ehr = seed_ehr(&source).await;
    let exported = source.extract_ehrs(ehr).await.expect("export");
    assert_eq!(
        count_by_xtype(&exported[0], "X_VERSIONED_EHR_ACCESS"),
        1,
        "fixture: the source export carries an EHR_ACCESS to strip"
    );
    target
        .import_ehr(None, without_ehr_access(&exported[0]))
        .await
        .expect("import an extract with no EHR_ACCESS");

    // The served EHR body carries the mandatory ehr_access reference.
    let body = target.ehr_object(ehr).await.expect("EHR body");
    assert_eq!(body["ehr_access"]["_type"], json!("OBJECT_REF"));
    assert_eq!(body["ehr_access"]["type"], json!("VERSIONED_EHR_ACCESS"));
    assert!(
        body["ehr_access"]["id"]["value"]
            .as_str()
            .is_some_and(|v| !v.is_empty()),
        "the ehr_access ref names the VERSIONED_EHR_ACCESS, got {body:#}"
    );

    // And the container really exists — one locally created first version.
    let re_export = target.extract_ehrs(ehr).await.expect("re-export clone");
    assert_eq!(
        count_by_xtype(&re_export[0], "X_VERSIONED_EHR_ACCESS"),
        1,
        "exactly one bootstrapped EHR_ACCESS"
    );
    let access =
        find_by_xtype(&re_export[0], "X_VERSIONED_EHR_ACCESS").expect("bootstrapped EHR_ACCESS");
    let versions = access["item"]["versions"]
        .as_array()
        .expect("EHR_ACCESS versions");
    assert_eq!(versions.len(), 1, "the bootstrap commits one version");
    let version_uid = versions[0]["uid"]["value"]
        .as_str()
        .expect("version uid.value");
    assert!(
        version_uid.ends_with("::1"),
        "the bootstrapped EHR_ACCESS is a first version, got {version_uid}"
    );
    assert_eq!(versions[0]["data"]["_type"], json!("EHR_ACCESS"));
    // The default is an archetype root (RM ehr `ehr_access.adoc`
    // `Is_archetype_root`; RM common `locatable.adoc` `Archetyped_valid`).
    assert!(
        versions[0]["data"]["archetype_details"].is_object(),
        "the default EHR_ACCESS carries its ARCHETYPED block, got {:#}",
        versions[0]["data"]
    );
}

#[tokio::test]
async fn import_ehr_promotes_the_subject_for_lookup_and_uniqueness() {
    // An imported EHR is a full local EHR: its EHR_STATUS.subject is promoted,
    // so the clone is found by the subject lookup (SM
    // `I_EHR_SERVICE.get_ehrs_for_subject`;
    // `operations/ehr_get_by_subject.yaml`) and holds the subject against a
    // later create (one EHR per subject — RM ehr master04 §EHR Status).
    let source_db = testkit::db().await.expect("testkit database");
    let source = EhrbaseService::new(source_db.pool());
    let target_db = testkit::db().await.expect("testkit database");
    let target = EhrbaseService::new(target_db.pool());

    let ehr = source
        .create_ehr(Some(status_for_subject("patient-import-1")))
        .await
        .expect("source EHR for the subject");
    let extract = export_one(&source, ehr).await;
    target.import_ehr(None, extract).await.expect("import_ehr");

    // The extract normalizes every OBJECT_REF-family namespace to "local"
    // (EHR Extract master09 §Creation Semantics: "rewriting its
    // OBJECT_REFs so that namespace = \"local\""), so the imported
    // subject is patient-import-1@local — the promoted columns follow
    // the STORED status, exactly as the fix requires.
    let subject = SubjectRef::person("patient-import-1", "local");
    assert!(
        target
            .has_ehr_for_subject(subject.clone())
            .await
            .expect("has_ehr_for_subject"),
        "the imported clone must be visible to the subject lookup"
    );
    let found = target
        .get_ehrs_for_subject(subject)
        .await
        .expect("get_ehrs_for_subject");
    assert_eq!(found.len(), 1, "one EHR per subject");
    assert_eq!(found[0].ehr_id, ehr.to_string());

    // The subject is now taken in the target — under the "local" namespace
    // the extract normalized it to — so a later create naming that same
    // (id, namespace) pair is a 409.
    let dup = target
        .create_ehr(Some(status_for_subject_in("patient-import-1", "local")))
        .await;
    assert!(
        matches!(
            dup,
            Err(ehrbase::service::status::SmError {
                status: CallStatusType::CompositionAlreadyExists,
                ..
            })
        ),
        "an EHR for a subject an imported clone already holds must 409, got {dup:?}"
    );
}

#[tokio::test]
async fn import_ehr_conflicting_subject_is_rejected_and_rolled_back() {
    // Importing a clone of a subject this repository already holds under
    // another EHR is a conflict, not a silent duplicate (one EHR per subject —
    // RM ehr master04 §EHR Status); the whole import transaction rolls back.
    let source_db = testkit::db().await.expect("testkit database");
    let source = EhrbaseService::new(source_db.pool());
    let target_db = testkit::db().await.expect("testkit database");
    let target = EhrbaseService::new(target_db.pool());

    // The import will carry the subject under namespace "local" (the
    // extract's master09 §Creation Semantics OBJECT_REF rewrite), so the
    // pre-existing holder must own it under that namespace to collide.
    let owner = target
        .create_ehr(Some(status_for_subject_in("patient-import-2", "local")))
        .await
        .expect("the target already holds the subject");
    let ehr = source
        .create_ehr(Some(status_for_subject("patient-import-2")))
        .await
        .expect("source EHR for the same subject");
    let extract = export_one(&source, ehr).await;

    let err = target
        .import_ehr(None, extract)
        .await
        .expect_err("importing a subject the target already holds must be rejected");
    assert_eq!(err.status, CallStatusType::CompositionAlreadyExists);
    assert!(
        err.message.contains("patient-import-2") && err.message.contains(&owner.to_string()),
        "the error must name the subject and the EHR that holds it, got: {}",
        err.message
    );

    // Rolled back: the clone left nothing behind, and the owner is untouched.
    assert!(
        !target.has_ehr(ehr).await.expect("has_ehr"),
        "a rejected import must not leave a partial EHR"
    );
    let found = target
        .get_ehrs_for_subject(SubjectRef::person("patient-import-2", "local"))
        .await
        .expect("get_ehrs_for_subject");
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].ehr_id, owner.to_string());
}

#[tokio::test]
async fn import_ehr_into_fixed_fresh_id() {
    let source_db = testkit::db().await.expect("testkit database");
    let source = EhrbaseService::new(source_db.pool());
    let target_db = testkit::db().await.expect("testkit database");
    let target = EhrbaseService::new(target_db.pool());

    let ehr = seed_ehr(&source).await;
    let source_status = source
        .get_ehr_status_at_time(ehr, None)
        .await
        .expect("source status");

    // A caller-provided fixed id (the SM's "same patient in other EHR services"
    // case): the clone lands under `fixed`, not the source id.
    let fixed = ehrbase::ids::EhrId::new();
    let extract = export_one(&source, ehr).await;
    target
        .import_ehr(Some(fixed), extract)
        .await
        .expect("import_ehr with a fixed id");

    // The source id does not exist in the target; the fixed id does.
    assert_eq!(
        target
            .extract_ehrs(ehr)
            .await
            .expect_err("source id absent in target")
            .status,
        CallStatusType::EhrIdDoesNotExist
    );
    let mut target_status = target
        .get_ehr_status_at_time(fixed, None)
        .await
        .expect("status under the fixed id");

    // The versioned-object identity (uid) is preserved regardless of the target
    // EHR id (the OBJECT_VERSION_ID's object_id is the EHR_STATUS container, not
    // the EHR): data + uid match the source exactly.
    assert_eq!(target_status, source_status);
    target_status.as_object_mut().unwrap().remove("uid");
    assert_eq!(target_status["is_modifiable"], json!(false));
}

#[tokio::test]
async fn import_ehr_duplicate_target_is_rejected() {
    let db = testkit::db().await.expect("testkit database");
    let svc = EhrbaseService::new(db.pool());
    let ehr = seed_ehr(&svc).await;

    // import_ehr imports into an *empty* target; a fixed id that already exists
    // is `ehr_create_fail_duplicate_id`.
    let extract = export_one(&svc, ehr).await;
    let err = svc
        .import_ehr(Some(ehr), extract)
        .await
        .expect_err("duplicate target EHR id must be rejected");
    assert_eq!(err.status, CallStatusType::EhrCreateFailDuplicateId);
}

#[tokio::test]
async fn import_ehr_extract_adds_a_versioned_object_and_rejects_re_import() {
    let source_db = testkit::db().await.expect("testkit database");
    let source = EhrbaseService::new(source_db.pool());
    let target_db = testkit::db().await.expect("testkit database");
    let target = EhrbaseService::new(target_db.pool());

    let src_ehr = seed_ehr(&source).await;

    // A FOLDER-only extract (item_list restricts to the directory container),
    // so importing it into an existing EHR that has no directory is a clean
    // Case-2 create (no EHR_STATUS/EHR_ACCESS singleton clash).
    let whole = source.extract_ehrs(src_ehr).await.expect("whole export");
    let folder_vo = find_by_xtype(&whole[0], "X_VERSIONED_FOLDER").expect("folder in export")
        ["item"]["uid"]["value"]
        .as_str()
        .expect("folder vo uid")
        .to_owned();
    let source_folder_data =
        find_by_xtype(&whole[0], "X_VERSIONED_FOLDER").unwrap()["item"]["versions"][0]["data"]
            .clone();

    let spec: ExtractSpec = openehr_its::json::from_canonical_value(&json!({
        "_type": "EXTRACT_SPEC",
        "manifest": {
            "_type": "EXTRACT_MANIFEST",
            "entities": [ {
                "_type": "EXTRACT_ENTITY_MANIFEST",
                "extract_id_key": src_ehr.to_string(),
                "ehr_id": src_ehr.to_string(),
                "other_ids": [],
                "item_list": [ {
                    "_type": "OBJECT_REF",
                    "namespace": "local",
                    "type": "VERSIONED_FOLDER",
                    "id": { "_type": "HIER_OBJECT_ID", "value": folder_vo }
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

    let mut folder_extracts = source
        .export_ehr_extracts(spec)
        .await
        .expect("folder-only export");
    let folder_extract: Extract =
        openehr_its::json::from_canonical_value(&folder_extracts.remove(0))
            .expect("folder EXTRACT");

    // A fresh target EHR (its own EHR_STATUS/EHR_ACCESS, no directory yet).
    let tgt_ehr = target.create_ehr(None).await.expect("target ehr");
    target
        .import_ehr_extract(tgt_ehr, folder_extract.clone())
        .await
        .expect("import the FOLDER into the existing EHR");

    // The target now carries the imported directory FOLDER with the source's data.
    let re_export = target
        .extract_ehrs(tgt_ehr)
        .await
        .expect("re-export target");
    let imported_folder =
        find_by_xtype(&re_export[0], "X_VERSIONED_FOLDER").expect("folder now present in target");
    assert_eq!(
        imported_folder["item"]["versions"][0]["data"], source_folder_data,
        "imported FOLDER data must match the source"
    );

    // Re-importing the same trunk version of the same container is a conflict
    // (trunk-only; the container already has that version).
    let err = target
        .import_ehr_extract(tgt_ehr, folder_extract)
        .await
        .expect_err("re-import of the same version must be rejected");
    assert_eq!(err.status, CallStatusType::CompositionAlreadyExists);
}
