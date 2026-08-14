// SPDX-FileCopyrightText: FerroEHR contributors
// SPDX-License-Identifier: MIT

//! End-to-end tests of the `spec_profile` compatibility stamp and its
//! read-time gate, against a real PostgreSQL 18 (shared testkit harness).
//!
//! No openEHR spec governs runtime specification-generation selection — our own
//! design/extension. The development-only construct these tests commit is
//! `GENERIC_ENTRY.data` holding a `CLUSTER`: RM 1.1.0 types that attribute
//! `ITEM_TREE`, and SPECRM-18 retyped it to the abstract `ITEM`
//! (= `CLUSTER` | `ELEMENT`) after that release —
//! `RM/docs/integration/master00-amendment_record.adoc`, issue 1.0, listed
//! above the `RM Release 1.1.0` marker. The two types are disjoint, so a
//! `CLUSTER` there is exactly a body the development generation reads and the
//! released one cannot.

#![expect(
    clippy::expect_used,
    reason = "clippy's in-test lint scoping (clippy.toml `allow-*-in-tests`) only \
              reaches `#[test]`-annotated functions, so it misses this integration \
              module's helpers and async bodies; panicking assertions are the \
              intended shape here (the Rust Book ch11)"
)]

use serde_json::{Value, json};

use ferroehr::config::profile::SpecProfile;
use ferroehr::ids::{EhrId, VoId};
use ferroehr::service::FerroEhrService;
use ferroehr::service::status::{CallStatusType, SmError};
use ferroehr::service::version_update::{change_type_coded, lifecycle_state_coded};
use openehr_its::rest::generated::common::{UpdateAudit, UpdateAuditData, UpdateVersion};
use openehr_rm::prelude::{Composition, PartyProxy};

/// A minimal valid RM COMPOSITION carrying the given `content` list.
fn composition(name: &str, content: &Value) -> Value {
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
        "composer": { "_type": "PARTY_IDENTIFIED", "name": "conformance tester" },
        "content": content
    })
}

/// A COMPOSITION whose content both generation sets express.
fn stable_clean_composition() -> Value {
    composition(
        "released surface",
        &json!([ {
            "_type": "SECTION",
            "name": { "_type": "DV_TEXT", "value": "section" },
            "archetype_node_id": "openEHR-EHR-SECTION.adhoc.v1"
        } ]),
    )
}

/// A COMPOSITION whose content only the DEVELOPMENT generation set expresses
/// (see the module docs: `GENERIC_ENTRY.data` as a `CLUSTER`).
fn development_only_composition() -> Value {
    composition(
        "development surface",
        &json!([ {
            "_type": "GENERIC_ENTRY",
            "name": { "_type": "DV_TEXT", "value": "entry" },
            "archetype_node_id": "openEHR-EHR-GENERIC_ENTRY.msg.v1",
            "data": {
                "_type": "CLUSTER",
                "name": { "_type": "DV_TEXT", "value": "data" },
                "archetype_node_id": "at0000",
                "items": [ {
                    "_type": "ELEMENT",
                    "name": { "_type": "DV_TEXT", "value": "leaf" },
                    "archetype_node_id": "at0001",
                    "value": { "_type": "DV_TEXT", "value": "x" }
                } ]
            }
        } ]),
    )
}

/// The SM `UPDATE_VERSION` commit envelope for a bare-RM COMPOSITION create.
fn create_version(data: &Value) -> UpdateVersion<Composition> {
    UpdateVersion {
        preceding_version_uid: None,
        lifecycle_state: lifecycle_state_coded("532"),
        attestations: None,
        data: openehr_its::json::from_canonical_value(data)
            .expect("the fixture commit body decodes as its RM type"),
        commit_audit: UpdateAudit::UpdateAudit(UpdateAuditData {
            _type: None,
            system_id: None,
            change_type: change_type_coded("249"),
            description: None,
            committer: openehr_its::json::from_canonical_value::<PartyProxy>(
                &json!({ "_type": "PARTY_IDENTIFIED", "name": "conformance tester" }),
            )
            .expect("committer"),
        }),
        signature: None,
    }
}

/// The stored `vo_version.stable_compatible` of an object's only version.
async fn stamp(pool: &sqlx::PgPool, vo_id: VoId) -> Option<bool> {
    sqlx::query_scalar::<_, Option<bool>>(
        "SELECT stable_compatible FROM vo_version WHERE vo_id = $1",
    )
    .bind(vo_id)
    .fetch_one(pool)
    .await
    .expect("the stamp column is readable")
}

/// A `stable`-profile service over the same database.
fn stable_service(pool: sqlx::PgPool) -> FerroEhrService {
    FerroEhrService::new(pool).with_spec_profile(SpecProfile::Stable)
}

/// Commit one COMPOSITION into a fresh EHR under the DEVELOPMENT profile.
async fn commit(svc: &FerroEhrService, body: &Value) -> (EhrId, VoId) {
    let ehr_id = svc.create_ehr(None).await.expect("ehr_create");
    let committed = svc
        .create_composition(ehr_id, create_version(body))
        .await
        .expect("composition create");
    (ehr_id, committed.vo_id)
}

/// A body the released generations express stamps `true` at commit and is
/// served under BOTH profiles.
#[tokio::test]
async fn a_released_surface_composition_stamps_true_and_reads_under_both_profiles() {
    let db = testkit::db().await.expect("testkit database");
    let svc = FerroEhrService::new(db.pool());
    let (ehr_id, vo_id) = commit(&svc, &stable_clean_composition()).await;

    assert_eq!(
        stamp(&db.pool(), vo_id).await,
        Some(true),
        "a released-surface body is stamped stable-compatible at commit"
    );

    let under_development = svc
        .get_composition_latest(ehr_id, vo_id)
        .await
        .expect("served under the development profile");
    assert_eq!(under_development["_type"], "COMPOSITION");

    let under_stable = stable_service(db.pool())
        .get_composition_latest(ehr_id, vo_id)
        .await
        .expect("served under the stable profile");
    assert_eq!(under_stable, under_development);
}

/// A body only the development generations express stamps `false` at commit,
/// is served under `development`, and is a `409`-class conflict under
/// `stable` — naming the profile, the version and the remedy.
#[tokio::test]
async fn a_development_only_composition_stamps_false_and_is_refused_under_stable() {
    let db = testkit::db().await.expect("testkit database");
    let svc = FerroEhrService::new(db.pool());
    let (ehr_id, vo_id) = commit(&svc, &development_only_composition()).await;

    assert_eq!(
        stamp(&db.pool(), vo_id).await,
        Some(false),
        "a development-only body is stamped NOT stable-compatible at commit"
    );

    let served = svc
        .get_composition_latest(ehr_id, vo_id)
        .await
        .expect("served under the development profile that accepted it");
    assert_eq!(served["content"][0]["data"]["_type"], "CLUSTER");

    let refused = stable_service(db.pool())
        .get_composition_latest(ehr_id, vo_id)
        .await;
    let Err(SmError {
        status: CallStatusType::Conflict,
        message,
        ..
    }) = refused
    else {
        panic!("the stable profile must refuse with a conflict, got {refused:?}");
    };
    assert!(message.contains("stable"), "{message}");
    assert!(message.contains("development"), "{message}");
    assert!(
        message.contains(&vo_id.to_string()),
        "the refusal names the version: {message}"
    );
}

/// An UNSTAMPED (`NULL`) row — one committed before the column existed, or
/// written by a verbatim-replay path — is assessed on the fly at read, in both
/// directions, and the read never writes the answer back.
#[tokio::test]
async fn an_unstamped_row_is_assessed_on_the_fly() {
    let db = testkit::db().await.expect("testkit database");
    let svc = FerroEhrService::new(db.pool());
    let (clean_ehr, clean_vo) = commit(&svc, &stable_clean_composition()).await;
    let (dirty_ehr, dirty_vo) = commit(&svc, &development_only_composition()).await;

    sqlx::query("UPDATE vo_version SET stable_compatible = NULL WHERE vo_id = ANY($1)")
        .bind(vec![clean_vo, dirty_vo])
        .execute(&db.pool())
        .await
        .expect("unstamp the two rows");

    let stable = stable_service(db.pool());
    stable
        .get_composition_latest(clean_ehr, clean_vo)
        .await
        .expect("an unstamped released-surface body is assessed and served");
    let refused = stable.get_composition_latest(dirty_ehr, dirty_vo).await;
    assert!(
        matches!(
            refused,
            Err(SmError {
                status: CallStatusType::Conflict,
                ..
            })
        ),
        "an unstamped development-only body is assessed and refused, got {refused:?}"
    );

    assert_eq!(
        stamp(&db.pool(), dirty_vo).await,
        None,
        "a read never writes the assessment back — reads stay pure"
    );
}

/// The stamp travels with the rows through the cold archival tier: archiving
/// and restoring an EHR must not change a version's profile compatibility.
#[tokio::test]
async fn archive_and_restore_preserve_the_stamp() {
    let db = testkit::db().await.expect("testkit database");
    let svc = FerroEhrService::new(db.pool());
    let (ehr_id, vo_id) = commit(&svc, &development_only_composition()).await;
    assert_eq!(stamp(&db.pool(), vo_id).await, Some(false));

    svc.archive_ehrs(vec![ehr_id.to_string()])
        .await
        .expect("archive the EHR");
    let cold: Option<bool> = sqlx::query_scalar::<_, Option<bool>>(
        "SELECT stable_compatible FROM cold.vo_version WHERE vo_id = $1",
    )
    .bind(vo_id)
    .fetch_one(&db.pool())
    .await
    .expect("the archived row carries the column");
    assert_eq!(cold, Some(false), "the cold mirror carries the stamp");

    // An archived object stays retrievable, and the gate still applies to it.
    let refused = stable_service(db.pool())
        .get_composition_latest(ehr_id, vo_id)
        .await;
    assert!(
        matches!(
            refused,
            Err(SmError {
                status: CallStatusType::Conflict,
                ..
            })
        ),
        "the cold-tier read is gated exactly like the primary one, got {refused:?}"
    );

    svc.restore_archived_ehrs(vec![ehr_id.to_string()])
        .await
        .expect("restore the EHR");
    assert_eq!(
        stamp(&db.pool(), vo_id).await,
        Some(false),
        "the restored row carries the stamp it was archived with"
    );
    svc.get_composition_latest(ehr_id, vo_id)
        .await
        .expect("the restored object reads under the development profile");
}

// ── The FHIR read façade goes through the same gate ──────────────────────────

/// The OPT the FHIR mapping binds to, and its ids.
const OPT_REL: &str = "tests/resources/service/knowledge/opt/minimal_evaluation.opt";
const TEMPLATE_ID: &str = "minimal_evaluation.en.v1";
const ROOT_ARCHETYPE: &str = "openEHR-EHR-COMPOSITION.minimal.v1";
/// The FHIR subject external id both compositions hang off.
const SUBJECT: &str = "p-42";

/// Read a test resource anchored at the crate manifest directory.
fn fixture(rel: &str) -> String {
    let path = format!("{}/{rel}", env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(&path).expect("read the OPT fixture")
}

/// The FHIR mapping bound to the minimal-evaluation template.
fn mapping_body() -> Value {
    json!({
        "name": "spec-profile-observation",
        "definition": {
            "resource_type": "Observation",
            "template_id": TEMPLATE_ID,
            "subject": {
                "reference_path": "subject.reference",
                "namespace": "fhir",
                "strip_prefix": "Patient/"
            },
            "context": {
                "ctx/language": "en", "ctx/territory": "US",
                "ctx/composer_name": "fhir-connector", "ctx/time": "2026-02-03T04:05:06Z"
            },
            "entries": [
                { "openehr_path": "minimal/minimal:0/quantity",
                  "fhir_path": "valueQuantity.value",
                  "transform": { "kind": "quantity", "unit_path": "valueQuantity.unit" } }
            ]
        }
    })
}

/// The inbound FHIR resource whose ingest produces the CLEAN, template-bound
/// COMPOSITION the façade also reverse-maps.
fn observation() -> Value {
    json!({
        "resourceType": "Observation",
        "id": "spec-profile-obs-1",
        "status": "final",
        "subject": { "reference": format!("Patient/{SUBJECT}") },
        "valueQuantity": { "value": 118, "unit": "kg" }
    })
}

/// The development-only COMPOSITION planted under the mapped template's root
/// archetype, so the façade's template-bound query reaches it.
fn development_only_under_template() -> Value {
    let mut body = development_only_composition();
    let root = body.as_object_mut().expect("the fixture is a JSON object");
    root.insert("archetype_node_id".to_owned(), json!(ROOT_ARCHETYPE));
    root.insert(
        "archetype_details".to_owned(),
        json!({
            "_type": "ARCHETYPED",
            "archetype_id": { "_type": "ARCHETYPE_ID", "value": ROOT_ARCHETYPE },
            "rm_version": "1.2.0"
        }),
    );
    body
}

/// Stamp `template_id` onto a stored COMPOSITION's root node fragment.
///
/// The façade's AQL matches on `archetype_details/template_id/value`, and a
/// COMPOSITION declaring a template is validated against it at commit — so a
/// body carrying template-foreign content (the `GENERIC_ENTRY` this module's
/// docs describe) can only reach the store the way this fixture puts it there.
/// `archetype_details` is not a structure node, so it lives in the root row's
/// canonical fragment (`num = 0`).
async fn stamp_template_on_stored_root(pool: &sqlx::PgPool, vo_id: VoId) {
    let affected = sqlx::query(
        "UPDATE node SET data = jsonb_set(data, '{archetype_details,template_id}', $2, true) \
         WHERE vo_id = $1 AND num = 0",
    )
    .bind(vo_id)
    .bind(json!({ "_type": "TEMPLATE_ID", "value": TEMPLATE_ID }))
    .execute(pool)
    .await
    .expect("stamp the template id on the stored root node")
    .rows_affected();
    assert_eq!(affected, 1, "exactly one root node row is stamped");
}

/// The FHIR read façade loads full stored bodies, so it takes the same
/// `spec_profile` gate every other served read takes: under `stable` a
/// development-only body is the `409`-class conflict, and no resource is
/// mapped from it.
#[tokio::test]
async fn the_fhir_read_facade_is_gated_by_the_spec_profile() {
    let db = testkit::db().await.expect("testkit database");
    let svc = FerroEhrService::new(db.pool());
    svc.template_adl14_upload(fixture(OPT_REL))
        .await
        .expect("ingest the OPT");
    svc.fhir_mapping_create(mapping_body())
        .await
        .expect("create the FHIR mapping");

    // One EHR carrying two COMPOSITIONs of the mapped template: the clean one
    // an inbound FHIR ingest commits, and a development-only one.
    let ehr_id = svc
        .create_ehr_for_subject(
            ferroehr::service::ehr_index::types::SubjectRef::person(SUBJECT, "fhir"),
            None,
        )
        .await
        .expect("create the subject's EHR");
    svc.fhir_ingest("Observation".to_owned(), None, observation())
        .await
        .expect("the inbound ingest commits the clean composition");
    let planted = svc
        .create_composition(ehr_id, create_version(&development_only_under_template()))
        .await
        .expect("the development profile commits the development-only body");
    assert_eq!(
        stamp(&db.pool(), planted.vo_id).await,
        Some(false),
        "the planted body is stamped NOT stable-compatible at commit"
    );
    stamp_template_on_stored_root(&db.pool(), planted.vo_id).await;

    // Under the profile that accepted it, the façade serves the Bundle.
    let bundle = svc
        .fhir_search("Observation".to_owned(), ehr_id.to_string(), None)
        .await
        .expect("the development profile serves the façade");
    assert_eq!(
        bundle["total"].as_u64(),
        Some(2),
        "both stored compositions of the mapped template are in the Bundle: {bundle}"
    );
    assert!(
        bundle["entry"].as_array().is_some_and(|entries| entries
            .iter()
            .any(|e| e["resource"]["valueQuantity"]["value"].as_f64() == Some(118.0))),
        "the clean composition still reverse-maps its value: {bundle}"
    );

    // Under `stable` the same façade refuses rather than mapping a body the
    // released generations do not define.
    let refused = stable_service(db.pool())
        .fhir_search("Observation".to_owned(), ehr_id.to_string(), None)
        .await;
    let Err(SmError {
        status: CallStatusType::Conflict,
        message,
        ..
    }) = refused
    else {
        panic!("the stable profile must refuse the FHIR read, got {refused:?}");
    };
    assert!(message.contains("stable"), "{message}");
    assert!(
        message.contains(&planted.vo_id.to_string()),
        "the refusal names the offending version: {message}"
    );
}
