//! Service-layer metric emission against a real `PostgreSQL` 18 (shared testkit
//! harness): the `compositions_committed_total` counter must move once per
//! COMPOSITION version that a commit route actually committed — the direct
//! create/update/delete routes and the CONTRIBUTION commit — and must NOT move
//! for a write whose transaction rolled back.
//!
//! The `change_type` label is the numeric openEHR `audit_change_type` group code
//! the version's audit records (`249|creation|` / `251|modification|` /
//! `523|deleted|`; RM common `master06-change_control_package.adoc`
//! §Contributions). No openEHR spec governs telemetry — the counter is our own
//! operational surface.

#![expect(
    clippy::expect_used,
    reason = "clippy's in-test lint scoping (clippy.toml `allow-*-in-tests`) only \
              reaches `#[test]`-annotated functions, so it misses this integration \
              module's helpers and async bodies; panicking assertions and direct \
              fixture indexing are the intended shape here (the Rust Book ch11)"
)]

use std::sync::OnceLock;

use ehrbase::service::EhrbaseService;
use ehrbase::service::version_update::{UpdateAudit, UpdateVersion};
use ehrbase::telemetry::build_info::BuildInfo;
use ehrbase::telemetry::prometheus::install;
use metrics_exporter_prometheus::PrometheusHandle;
use openehr_base::prelude::{ObjectVersionId, TerminologyCode};
use openehr_rm::prelude::PartyProxy;
use serde_json::{Value, json};

/// The process-wide recorder, installed through the real
/// [`install`] entry point (so the catalog's `describe_*` registrations run too)
/// and shared because only one global `metrics` recorder can exist per process.
fn recorder() -> &'static PrometheusHandle {
    static HANDLE: OnceLock<PrometheusHandle> = OnceLock::new();
    HANDLE.get_or_init(|| install(&BuildInfo::current()).expect("install prometheus recorder"))
}

/// The rendered value of `compositions_committed_total{change_type="<code>"}`,
/// or `0` when the series has not been emitted yet.
fn committed(handle: &PrometheusHandle, change_type: &str) -> u64 {
    let needle = format!("change_type=\"{change_type}\"");
    handle
        .render()
        .lines()
        .filter(|l| l.starts_with("compositions_committed_total{") && l.contains(&needle))
        .find_map(|l| l.rsplit(' ').next().and_then(|v| v.parse::<u64>().ok()))
        .unwrap_or(0)
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

fn committer(name: &str) -> Value {
    json!({ "_type": "PARTY_IDENTIFIED", "name": name })
}

/// The wire `change_type` `DV_CODED_TEXT` of a CONTRIBUTION version item.
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

/// The SM `UPDATE_VERSION` commit envelope for a bare-RM composition write.
fn uv(data: Value, change_code: &str, preceding: Option<&str>) -> UpdateVersion {
    UpdateVersion {
        preceding_version_uid: preceding.map(|p| p.parse().expect("OBJECT_VERSION_ID")),
        lifecycle_state: term("532"),
        attestations: None,
        data,
        audit: UpdateAudit {
            change_type: term(change_code),
            description: None,
            committer: openehr_its::json::from_canonical_value::<PartyProxy>(&committer(
                "metrics tester",
            ))
            .expect("committer"),
            system_id: None,
        },
        signature: None,
    }
}

/// A minimal *valid* RM COMPOSITION.
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
        "composer": { "_type": "PARTY_IDENTIFIED", "name": "metrics tester" }
    })
}

fn vo_of(ovid: &str) -> &str {
    ovid.split("::").next().expect("object id part")
}

/// One test drives every phase in a single process, so the counter deltas are
/// never entangled with another test's commits.
#[tokio::test]
async fn compositions_committed_total_counts_every_committed_composition_version() {
    let handle = recorder();
    let db = testkit::db().await.expect("testkit database");
    let svc = EhrbaseService::new(db.pool());
    let ehr_id = svc.create_ehr(None).await.expect("create_ehr");

    let (creations, modifications, deletions) = (
        committed(handle, "249"),
        committed(handle, "251"),
        committed(handle, "523"),
    );

    // (1) The direct create route: one 249|creation| version.
    let v1 = svc
        .create_composition(ehr_id, uv(composition("v1"), "249", None))
        .await
        .expect("create_composition")
        .version_uid();
    assert_eq!(
        committed(handle, "249"),
        creations + 1,
        "create_composition must count one 249|creation| commit"
    );

    // (2) The direct update route: one 251|modification| version.
    let v2 = svc
        .update_composition(
            ehr_id,
            vo_of(&v1).parse().expect("vo uuid"),
            uv(composition("v2"), "251", Some(&v1)),
        )
        .await
        .expect("update_composition")
        .version_uid();
    assert_eq!(
        committed(handle, "251"),
        modifications + 1,
        "update_composition must count one 251|modification| commit"
    );

    // (3) A rolled-back write counts NOTHING: the stale `preceding_version_uid`
    // fails the version placement inside the commit transaction, so the
    // transaction never commits.
    let stale: ObjectVersionId = v1.parse().expect("OBJECT_VERSION_ID");
    let conflict = svc
        .update_composition(
            ehr_id,
            vo_of(&v1).parse().expect("vo uuid"),
            uv(composition("stale"), "251", Some(&v1)),
        )
        .await;
    assert!(
        conflict.is_err(),
        "an update against a superseded version must fail: {conflict:?}"
    );
    assert_eq!(
        committed(handle, "251"),
        modifications + 1,
        "a rolled-back update must not move the counter"
    );

    // (4) The direct delete route: one 523|deleted| version.
    let latest: ObjectVersionId = v2.parse().expect("OBJECT_VERSION_ID");
    svc.delete_composition(ehr_id, &latest, None)
        .await
        .expect("delete_composition");
    assert_eq!(
        committed(handle, "523"),
        deletions + 1,
        "delete_composition must count one 523|deleted| commit"
    );
    // The stale identity was only ever used for the negative case above.
    assert_ne!(stale.value, latest.value);

    // (5) The CONTRIBUTION route: its COMPOSITION versions are counted too, and
    // its non-COMPOSITION versions (EHR_STATUS here) are not.
    let contribution = json!({
        "_type": "CONTRIBUTION",
        "versions": [{
            "_type": "ORIGINAL_VERSION",
            "commit_audit": {
                "change_type": change_type("249", "creation"),
                "committer": committer("metrics tester")
            },
            "lifecycle_state": change_type("532", "complete"),
            "data": composition("contributed")
        }],
        "audit": { "committer": committer("metrics tester") }
    });
    svc.create_ehr_contribution(ehr_id, contribution)
        .await
        .expect("create_ehr_contribution");
    assert_eq!(
        committed(handle, "249"),
        creations + 2,
        "a CONTRIBUTION-committed COMPOSITION must count one 249|creation| commit"
    );

    // The rendered exposition carries the counter with its catalog description,
    // so the operational dashboards read a real series (not a dead metric).
    let rendered = handle.render();
    assert!(
        rendered.contains("# TYPE compositions_committed_total counter"),
        "the counter must render with its TYPE: {rendered}"
    );
    assert!(
        rendered.contains("# HELP compositions_committed_total"),
        "the counter must render with its catalog HELP text: {rendered}"
    );
}
