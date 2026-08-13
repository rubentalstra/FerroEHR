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
