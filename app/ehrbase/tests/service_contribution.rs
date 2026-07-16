//! End-to-end tests for ATTESTATION support in the CONTRIBUTION path
//! (RM common `master06-change_control_package.adoc` §Change Control /
//! §Attestation; ITS-REST `UpdateVersion.yaml` + `UpdateAttestation.yaml`)
//! against a real `PostgreSQL` 18 (testcontainers).
//!
//! Covers: attestations committed with a NEW version
//! (`UPDATE_VERSION.attestations`, "Signing content at committal"); a later
//! `666|attestation|`-only contribution attaching an `ATTESTATION` to an
//! existing `ORIGINAL_VERSION` (no new version); their exposure on the served
//! `ORIGINAL_VERSION` and in `REVISION_HISTORY`; and `CONTRIBUTION.versions` +
//! the aggregate change-type semantics. Plus the error surface (400/422/404).

#![allow(clippy::expect_used, clippy::unwrap_used, clippy::too_many_lines)]

use ehrbase::db::{self, DbConfig};
use ehrbase::service::EhrbaseService;
use ehrbase::service::status::{CallStatusType, SmError};

use ehrbase::service::list::Page;
use ehrbase::service::version_update::{UpdateAudit, UpdateVersion};
use openehr_base::prelude::TerminologyCode;
use openehr_rm::prelude::PartyProxy;
use serde_json::{Value, json};
use sqlx::{AssertSqlSafe, Connection, PgConnection, PgPool};
use testcontainers::runners::AsyncRunner;
use testcontainers::{ContainerAsync, ImageExt};
use testcontainers_modules::postgres::Postgres;

struct Pg {
    _container: ContainerAsync<Postgres>,
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
            _container: container,
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
        let settings = DbConfig::new(format!(
            "postgres://postgres:postgres@{}:{}/{name}",
            self.host, self.port
        ));
        let pool = db::connect(&settings).await.expect("pool");
        db::run_migrations(&pool).await.expect("migrate");
        pool
    }
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
            committer: serde_json::from_value::<PartyProxy>(
                json!({ "_type": "PARTY_IDENTIFIED", "name": "conformance tester" }),
            )
            .expect("committer"),
            system_id: None,
        },
        signature: None,
    }
}

/// A minimal *valid* RM COMPOSITION (mirrors the signing-test fixture).
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

fn committer(name: &str) -> Value {
    json!({ "_type": "PARTY_IDENTIFIED", "name": name })
}

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

/// A wire `UPDATE_ATTESTATION` partial (the server completes it).
fn attestation(reason: &str, is_pending: bool) -> Value {
    json!({
        "_type": "UPDATE_ATTESTATION",
        "change_type": change_type("666", "attestation"),
        "committer": committer("attesting clinician"),
        "reason": { "_type": "DV_TEXT", "value": reason },
        "is_pending": is_pending,
        "proof": "proof-bytes"
    })
}

async fn create_ehr(svc: &EhrbaseService) -> String {
    svc.create_ehr(None).await.expect("create_ehr").to_string()
}

/// Read the served `ORIGINAL_VERSION` of a composition version.
async fn read_version(svc: &EhrbaseService, ehr_id: &str, _vo: &str, ovid: &str) -> Value {
    svc.composition_original_version(
        ehr_id.parse().expect("ehr uuid"),
        ovid.parse().expect("ovid"),
    )
    .await
    .expect("versioned composition version")
}

/// The `OBJECT_VERSION_ID` of the first version listed in a `CONTRIBUTION`.
fn first_version_uid(contribution: &Value) -> String {
    contribution["versions"][0]["id"]["value"]
        .as_str()
        .expect("versions[0].id.value")
        .to_owned()
}

fn vo_of(ovid: &str) -> String {
    ovid.split("::").next().unwrap().to_owned()
}

#[tokio::test]
async fn accompanying_attestation_then_standalone_666_attestation() {
    let pg = Pg::start().await;
    let svc = EhrbaseService::new(pg.migrated_pool("attestation_flow").await);
    let ehr_id = create_ehr(&svc).await;

    // (1) A CONTRIBUTION that creates a COMPOSITION carrying an attestation
    // committed with the version (UPDATE_VERSION.attestations).
    let contribution = json!({
        "_type": "CONTRIBUTION",
        "versions": [{
            "_type": "ORIGINAL_VERSION",
            "commit_audit": {
                "change_type": change_type("249", "creation"),
                "committer": committer("author")
            },
            "lifecycle_state": change_type("532", "complete"),
            "data": composition("v1"),
            "attestations": [ attestation("witnessed", false) ]
        }],
        "audit": { "committer": committer("author") }
    });
    let created = svc
        .create_ehr_contribution(ehr_id.parse().expect("ehr uuid"), contribution)
        .await
        .expect("contribution_create with accompanying attestation → 201");
    let ovid_v1 = first_version_uid(&created.body);
    let vo_id = vo_of(&ovid_v1);

    // Reading the ORIGINAL_VERSION exposes the completed ATTESTATION.
    let ov = read_version(&svc, &ehr_id, &vo_id, &ovid_v1).await;
    let atts = ov["attestations"].as_array().expect("attestations array");
    assert_eq!(atts.len(), 1, "one attestation after step 1");
    let att = &atts[0];
    assert_eq!(att["_type"], "ATTESTATION");
    assert!(att["system_id"].is_string(), "server-completed system_id");
    assert_eq!(att["time_committed"]["_type"], "DV_DATE_TIME");
    assert_eq!(att["reason"]["value"], "witnessed");
    assert_eq!(att["is_pending"], json!(false));
    // The inherited change_type is the 666|attestation| code.
    assert_eq!(att["change_type"]["defining_code"]["code_string"], "666");

    // (2) A later 666-only CONTRIBUTION attesting that same version.
    let attest_contribution = json!({
        "versions": [{
            "preceding_version_uid": { "value": ovid_v1.clone() },
            "commit_audit": {
                "change_type": change_type("666", "attestation"),
                "committer": committer("senior reviewer"),
                "reason": { "_type": "DV_TEXT", "value": "authorised" },
                "is_pending": false
            }
        }]
    });
    let attest_created = svc
        .create_ehr_contribution(ehr_id.parse().expect("ehr uuid"), attest_contribution)
        .await
        .expect("666-only contribution → 201");

    // The 666 contribution's aggregate change_type is 666, and its versions
    // list the attested (existing) version.
    assert_eq!(
        attest_created.body["audit"]["change_type"]["defining_code"]["code_string"],
        "666"
    );
    let attest_versions = attest_created.body["versions"]
        .as_array()
        .expect("versions");
    assert_eq!(attest_versions.len(), 1, "attested version is listed");
    assert_eq!(
        attest_versions[0]["id"]["value"].as_str(),
        Some(ovid_v1.as_str())
    );

    // The ORIGINAL_VERSION now lists both attestations.
    let ov = read_version(&svc, &ehr_id, &vo_id, &ovid_v1).await;
    assert_eq!(
        ov["attestations"].as_array().map(Vec::len),
        Some(2),
        "two attestations after the standalone 666"
    );

    // REVISION_HISTORY: the single version's audits = commit audit + both
    // attestations (revision_history_item.adoc "there may also be further
    // attestations").
    let rh = svc
        .composition_revision_history(
            ehr_id.parse().expect("ehr uuid"),
            vo_id.parse().expect("vo uuid"),
        )
        .await
        .expect("revision history");
    let items = rh["items"].as_array().expect("items");
    assert_eq!(items.len(), 1, "one version");
    let audits = items[0]["audits"].as_array().expect("audits");
    assert_eq!(audits.len(), 3, "commit audit + 2 attestations");
    assert_eq!(audits[0]["_type"], "AUDIT_DETAILS");
    assert_eq!(audits[1]["_type"], "ATTESTATION");
    assert_eq!(audits[2]["_type"], "ATTESTATION");
}

#[tokio::test]
async fn attestation_error_cases() {
    let pg = Pg::start().await;
    let svc = EhrbaseService::new(pg.migrated_pool("attestation_errors").await);
    let ehr_id = create_ehr(&svc).await;

    // A real composition to attest.
    let v1 = svc
        .create_composition(
            ehr_id.parse().expect("ehr uuid"),
            uv(composition("v1"), "249", None),
        )
        .await
        .expect("composition_create");
    let ovid_v1 = v1;

    let attempt = |body: Value| {
        let svc = svc.clone();
        let ehr = ehr_id.clone();
        async move {
            svc.create_ehr_contribution(ehr.parse().expect("ehr uuid"), body)
                .await
        }
    };

    // 666 without preceding_version_uid → 400 (cannot name its target).
    let err = attempt(json!({
        "versions": [{
            "commit_audit": {
                "change_type": change_type("666", "attestation"),
                "committer": committer("x"),
                "reason": { "_type": "DV_TEXT", "value": "authorised" },
                "is_pending": false
            }
        }]
    }))
    .await
    .expect_err("666 without preceding → error");
    assert!(
        matches!(
            err,
            SmError {
                status: CallStatusType::PreconditionViolation,
                ..
            }
        ),
        "got {err:?}"
    );

    // 666 carrying data → 422 (attestation adds no content).
    let err = attempt(json!({
        "versions": [{
            "preceding_version_uid": { "value": ovid_v1.clone() },
            "commit_audit": {
                "change_type": change_type("666", "attestation"),
                "committer": committer("x"),
                "reason": { "_type": "DV_TEXT", "value": "authorised" },
                "is_pending": false
            },
            "data": composition("nope")
        }]
    }))
    .await
    .expect_err("666 with data → error");
    assert!(
        matches!(
            err,
            SmError {
                status: CallStatusType::ContentInvalid,
                ..
            }
        ),
        "got {err:?}"
    );

    // Attestation missing reason → 422 (ATTESTATION.reason 1..1).
    let err = attempt(json!({
        "versions": [{
            "preceding_version_uid": { "value": ovid_v1.clone() },
            "commit_audit": {
                "change_type": change_type("666", "attestation"),
                "committer": committer("x"),
                "is_pending": false
            }
        }]
    }))
    .await
    .expect_err("missing reason → error");
    assert!(
        matches!(
            err,
            SmError {
                status: CallStatusType::ContentInvalid,
                ..
            }
        ),
        "got {err:?}"
    );

    // Attestation missing is_pending → 422 (ATTESTATION.is_pending 1..1).
    let err = attempt(json!({
        "versions": [{
            "preceding_version_uid": { "value": ovid_v1.clone() },
            "commit_audit": {
                "change_type": change_type("666", "attestation"),
                "committer": committer("x"),
                "reason": { "_type": "DV_TEXT", "value": "authorised" }
            }
        }]
    }))
    .await
    .expect_err("missing is_pending → error");
    assert!(
        matches!(
            err,
            SmError {
                status: CallStatusType::ContentInvalid,
                ..
            }
        ),
        "got {err:?}"
    );

    // Attestation of a non-existent version → precondition violation, not an
    // existence error: SM I_EHR_CONTRIBUTION.commit_contribution declares only
    // `ehr_id_does_not_exist` (SM `i_ehr_contribution.adoc`) — a missing
    // body-referenced target is invalid committed content (ITS-REST
    // `400_CONTRIBUTION`: the modification does not match a stored object).
    let ghost = "00000000-0000-7000-8000-000000000000::ehrbase-rs.local::1";
    let err = attempt(json!({
        "versions": [{
            "preceding_version_uid": { "value": ghost },
            "commit_audit": {
                "change_type": change_type("666", "attestation"),
                "committer": committer("x"),
                "reason": { "_type": "DV_TEXT", "value": "authorised" },
                "is_pending": false
            }
        }]
    }))
    .await
    .expect_err("non-existent version → error");
    assert!(
        matches!(
            err,
            SmError {
                status: CallStatusType::PreconditionViolation,
                ..
            }
        ),
        "got {err:?}"
    );
}

/// A valid `EHR_STATUS` for an update (a fresh version, distinct content).
fn ehr_status(queryable: bool) -> Value {
    json!({
        "_type": "EHR_STATUS",
        "archetype_node_id": "openEHR-EHR-EHR_STATUS.generic.v1",
        "name": { "_type": "DV_TEXT", "value": "EHR Status" },
        "subject": { "_type": "PARTY_SELF" },
        "is_queryable": queryable,
        "is_modifiable": true
    })
}

/// SM `I_EHR_CONTRIBUTION.list_contributions` / `contribution_count` +
/// `I_EHR_SERVICE.get_ehr` → `EHR_SUMMARY`, against a real PG 18. Native-API
/// calls only (no ITS-REST route); see `i_ehr_contribution.adoc` +
/// `ehr_summary.adoc`.
#[tokio::test]
async fn contribution_listing_count_and_ehr_summary() {
    let pg = Pg::start().await;
    let svc = EhrbaseService::new(pg.migrated_pool("contribution_listing").await);

    // Unknown EHR → NotFound (SM ehr_does_not_exist) for every native call.
    let ghost = "00000000-0000-7000-8000-0000000000ff";
    assert!(matches!(
        svc.list_contributions(ghost.parse().expect("uuid"), None, Page::all())
            .await,
        Err(SmError {
            status: CallStatusType::VersionedObjectDoesNotExist,
            ..
        })
    ));
    assert!(matches!(
        svc.contribution_count(ghost.parse().expect("uuid"), None)
            .await,
        Err(SmError {
            status: CallStatusType::VersionedObjectDoesNotExist,
            ..
        })
    ));
    assert!(matches!(
        svc.get_ehr(ghost.parse().expect("uuid")).await,
        Err(SmError {
            status: CallStatusType::VersionedObjectDoesNotExist,
            ..
        })
    ));

    // Seed: (1) EHR creation, (2) an EHR_STATUS update, (3) a composition — three
    // CONTRIBUTIONs, one of them a COMPOSITION.
    let ehr_id = create_ehr(&svc).await; // contribution #1

    let ehr_uuid = ehr_id.parse::<uuid::Uuid>().expect("ehr uuid");
    let status_uid = svc
        .get_ehr_status_at_time(ehr_uuid, None)
        .await
        .expect("get current EHR_STATUS")["uid"]["value"]
        .as_str()
        .expect("status uid")
        .to_owned();
    svc.replace_ehr_status(ehr_uuid, uv(ehr_status(false), "251", Some(&status_uid)))
        .await
        .expect("EHR_STATUS update"); // contribution #2

    svc.create_composition(ehr_uuid, uv(composition("obs"), "249", None))
        .await
        .expect("composition_create"); // contribution #3

    // contribution_count matches the seeded three.
    let count = svc
        .contribution_count(ehr_uuid, None)
        .await
        .expect("contribution_count");
    assert_eq!(count, 3, "EHR creation + status update + composition");

    // contribution_list returns all three, oldest-first, distinct.
    let all = svc
        .list_contributions(ehr_uuid, None, Page::all())
        .await
        .expect("contribution_list");
    assert_eq!(all.len(), 3, "three contribution ids");
    let distinct: std::collections::BTreeSet<_> = all.iter().collect();
    assert_eq!(distinct.len(), 3, "ids are distinct");

    // Paging: offset 1, fetch 1 → exactly the second id of the full list.
    let page = svc
        .list_contributions(
            ehr_uuid,
            None,
            Page {
                item_offset: Some(1),
                items_to_fetch: Some(1),
            },
        )
        .await
        .expect("paged contribution_list");
    assert_eq!(
        page,
        vec![all[1].clone()],
        "offset 1 / fetch 1 slices the list"
    );

    // time_range: an upper bound before every commit excludes all.
    let empty = svc
        .list_contributions(
            ehr_uuid,
            Some((None, Some("2000-01-01T00:00:00Z".to_owned()))),
            Page::all(),
        )
        .await
        .expect("bounded contribution_list");
    assert!(
        empty.is_empty(),
        "upper bound in the past → no contributions"
    );
    assert_eq!(
        svc.contribution_count(
            ehr_uuid,
            Some((None, Some("2000-01-01T00:00:00Z".to_owned())))
        )
        .await
        .expect("bounded count"),
        0
    );
    // A malformed bound → 400 BadRequest.
    assert!(matches!(
        svc.contribution_count(ehr_uuid, Some((Some("not-a-time".to_owned()), None)))
            .await,
        Err(SmError {
            status: CallStatusType::PreconditionViolation,
            ..
        })
    ));

    // EHR_SUMMARY: mandatory fields + the counts.
    let summary = svc.get_ehr(ehr_uuid).await.expect("get_ehr_summary");
    assert_eq!(summary.ehr_id, ehr_id);
    assert!(!summary.system_id.is_empty(), "system_id (EHR.system_id)");
    assert_eq!(summary.ehr_status["_type"], "EHR_STATUS", "ehr_status copy");
    assert!(
        summary.time_created.parse::<jiff::Timestamp>().is_ok(),
        "time_created is ISO 8601"
    );
    assert_eq!(summary.contribution_count, 3);
    assert_eq!(
        summary.composition_count, 1,
        "one versioned COMPOSITION (versioned objects, not versions)"
    );
}

/// A `TerminologyCode`-shaped `UPDATE_VERSION.lifecycle_state` (the wire shape
/// per ITS-REST `UpdateVersion.yaml`): `{terminology_id, code_string}`.
fn lifecycle(code: &str) -> Value {
    json!({ "terminology_id": "openehr", "code_string": code })
}

/// Read the `lifecycle_state.defining_code.code_string` of a served
/// `ORIGINAL_VERSION`.
async fn lifecycle_code_of(svc: &EhrbaseService, ehr_id: &str, vo: &str, ovid: &str) -> String {
    let ov = read_version(svc, ehr_id, vo, ovid).await;
    ov["lifecycle_state"]["defining_code"]["code_string"]
        .as_str()
        .expect("lifecycle_state code_string")
        .to_owned()
}

#[tokio::test]
async fn contribution_honors_the_five_lifecycle_states() {
    // M1 (RM common master06 §"Version Lifecycle"): the client-supplied
    // lifecycle_state on create/modify is stored + served faithfully for every
    // normative code; only the delete path is forced to 523. 553/800/801 are
    // NOT deletions — the version is readable with its data.
    let pg = Pg::start().await;
    let svc = EhrbaseService::new(pg.migrated_pool("lifecycle_states").await);
    let ehr_id = create_ehr(&svc).await;

    // (1) Create v1 as incomplete (553).
    let created = svc
        .create_ehr_contribution(
            ehr_id.parse().expect("ehr uuid"),
            json!({
                "versions": [{
                    "commit_audit": {
                        "change_type": change_type("249", "creation"),
                        "committer": committer("author")
                    },
                    "lifecycle_state": lifecycle("553"),
                    "data": composition("incomplete v1")
                }],
                "audit": { "committer": committer("author") }
            }),
        )
        .await
        .expect("create incomplete (553) → 201");
    let ovid_v1 = first_version_uid(&created.body);
    let vo_id = vo_of(&ovid_v1);

    let ov1 = read_version(&svc, &ehr_id, &vo_id, &ovid_v1).await;
    assert_eq!(
        ov1["lifecycle_state"]["defining_code"]["code_string"],
        "553"
    );
    assert_eq!(ov1["lifecycle_state"]["value"], "incomplete");
    // A 553 version is not a deletion: its data is served.
    assert_eq!(ov1["data"]["_type"], "COMPOSITION");
    // The latest read returns 200 with data (not the deleted 204 path).
    let latest = svc
        .get_composition_latest(
            ehr_id.parse().expect("ehr uuid"),
            vo_id.parse().expect("vo uuid"),
        )
        .await
        .expect("composition_get incomplete");
    assert_eq!(latest["name"]["value"], "incomplete v1");

    // (2) The master06 §Version Lifecycle state machine only permits listed
    // transitions: `inactive` is entered by `deactivate` FROM `complete`, so an
    // incomplete → inactive modify is rejected 422 naming the state machine.
    let mut current = ovid_v1;
    let illegal = svc
        .create_ehr_contribution(
            ehr_id.parse().expect("ehr uuid"),
            json!({
                "versions": [{
                    "commit_audit": {
                        "change_type": change_type("251", "modification"),
                        "committer": committer("author")
                    },
                    "preceding_version_uid": { "value": current },
                    "lifecycle_state": lifecycle("800"),
                    "data": composition("edited")
                }],
                "audit": { "committer": committer("author") }
            }),
        )
        .await
        .expect_err("incomplete → inactive is not a listed transition");
    assert!(
        matches!(
            illegal,
            SmError {
                status: CallStatusType::ContentInvalid,
                ..
            }
        ) && illegal.message.contains("state machine"),
        "expected the 422 naming the state machine, got {illegal:?}"
    );

    // Walk the LEGAL transitions (master06 §Version Lifecycle + §Abandoned and
    // Inactive States): 553 →complete→ 532 →deactivate→ 800 →retrieve→ 553
    // →abandon→ 801 — each a new version carrying the client lifecycle_state.
    for (n, code) in [(2, "532"), (3, "800"), (4, "553"), (5, "801")] {
        let modified = svc
            .create_ehr_contribution(
                ehr_id.parse().expect("ehr uuid"),
                json!({
                    "versions": [{
                        "commit_audit": {
                            "change_type": change_type("251", "modification"),
                            "committer": committer("author")
                        },
                        "preceding_version_uid": { "value": current },
                        "lifecycle_state": lifecycle(code),
                        "data": composition("edited")
                    }],
                    "audit": { "committer": committer("author") }
                }),
            )
            .await
            .unwrap_or_else(|e| panic!("modify to {code} → 201, got {e:?}"));
        current = first_version_uid(&modified.body);
        assert!(current.ends_with(&format!("::{n}")), "version {n}");
        assert_eq!(
            lifecycle_code_of(&svc, &ehr_id, &vo_id, &current).await,
            code,
            "served lifecycle_state must be {code}"
        );
    }

    // (3) An out-of-group lifecycle code is a 422 naming the group.
    let err = svc
        .create_ehr_contribution(
            ehr_id.parse().expect("ehr uuid"),
            json!({
                "versions": [{
                    "commit_audit": {
                        "change_type": change_type("251", "modification"),
                        "committer": committer("author")
                    },
                    "preceding_version_uid": { "value": current },
                    "lifecycle_state": lifecycle("999"),
                    "data": composition("bad state")
                }],
                "audit": { "committer": committer("author") }
            }),
        )
        .await
        .expect_err("invalid lifecycle_state → error");
    assert!(
        matches!(
            err,
            SmError {
                status: CallStatusType::ContentInvalid,
                ..
            }
        ) && err.message.contains("version_lifecycle_state"),
        "expected content_invalid naming version_lifecycle_state, got {err:?}"
    );

    // (4) The delete path is still forced to 523, regardless of any lifecycle
    // hint, and the latest read is then the deleted (204/null) path.
    svc.create_ehr_contribution(
        ehr_id.parse().expect("ehr uuid"),
        json!({
            "versions": [{
                "commit_audit": {
                    "change_type": change_type("523", "deleted"),
                    "committer": committer("author")
                },
                "preceding_version_uid": { "value": current }
            }],
            "audit": { "committer": committer("author") }
        }),
    )
    .await
    .expect("delete (523) → 201");
    // The now-current version is a deletion: lifecycle 523.
    let del = svc
        .get_composition_latest(
            ehr_id.parse().expect("ehr uuid"),
            vo_id.parse().expect("vo uuid"),
        )
        .await
        .expect("composition_get deleted");
    assert!(
        del.is_null(),
        "a deleted composition reads as an empty body (204), got {del:?}"
    );
}

#[tokio::test]
async fn version_commit_audit_defaults_from_the_contribution_audit() {
    // m4 (RM common master06 §"Committal"): the CONTRIBUTION audit's
    // system_id/committer "should be copied into the corresponding attributes
    // of the commit_audit of each VERSION" when the version item omits them; a
    // version item that supplies its own values keeps them verbatim.
    let pg = Pg::start().await;
    let svc = EhrbaseService::new(pg.migrated_pool("audit_copy").await);
    let ehr_id = create_ehr(&svc).await;

    let created = svc
        .create_ehr_contribution(
            ehr_id.parse().expect("ehr uuid"),
            json!({
                "versions": [
                    // (A) commit_audit omits committer + system_id → inherit them.
                    {
                        "commit_audit": { "change_type": change_type("249", "creation") },
                        "data": composition("inherits contribution audit")
                    },
                    // (B) commit_audit supplies distinct committer + system_id → keep.
                    {
                        "commit_audit": {
                            "change_type": change_type("249", "creation"),
                            "committer": committer("version-B author"),
                            "system_id": "version-b.system"
                        },
                        "data": composition("keeps its own audit")
                    }
                ],
                "audit": {
                    "committer": committer("contribution committer"),
                    "system_id": "contribution.system"
                }
            }),
        )
        .await
        .expect("two-creation contribution → 201");

    // Collect each created version's commit_audit (system_id + committer name).
    let mut seen: Vec<(String, String)> = Vec::new();
    for v in created.body["versions"].as_array().expect("versions") {
        let ovid = v["id"]["value"].as_str().expect("ovid").to_owned();
        let vo = vo_of(&ovid);
        let ov = read_version(&svc, &ehr_id, &vo, &ovid).await;
        let sys = ov["commit_audit"]["system_id"]
            .as_str()
            .expect("system_id")
            .to_owned();
        let who = ov["commit_audit"]["committer"]["name"]
            .as_str()
            .expect("committer name")
            .to_owned();
        seen.push((sys, who));
    }
    seen.sort();

    assert!(
        seen.contains(&(
            "contribution.system".to_owned(),
            "contribution committer".to_owned()
        )),
        "version A must inherit the contribution audit, got {seen:?}"
    );
    assert!(
        seen.contains(&("version-b.system".to_owned(), "version-B author".to_owned())),
        "version B must keep its own committer/system_id, got {seen:?}"
    );
}

/// `get_contribution_resolved` (ITS-REST `Prefer: resolve_refs`): the
/// CONTRIBUTION's `versions` carry the full `ORIGINAL_VERSION` objects instead
/// of `OBJECT_REF`s; the unresolved form is unchanged.
#[tokio::test]
async fn contribution_resolve_refs() {
    let pg = Pg::start().await;
    let svc = EhrbaseService::new(pg.migrated_pool("contribution_resolve").await);

    let ehr_id = create_ehr(&svc).await;
    let ehr_uuid = ehr_id.parse::<uuid::Uuid>().expect("ehr uuid");
    let comp_uid = svc
        .create_composition(ehr_uuid, uv(composition("obs"), "249", None))
        .await
        .expect("composition_create");

    let all = svc
        .list_contributions(ehr_uuid, None, Page::all())
        .await
        .expect("list");
    let cid = all
        .last()
        .expect("a contribution")
        .parse::<uuid::Uuid>()
        .expect("contribution uuid");

    let plain = svc.get_contribution(ehr_uuid, cid).await.expect("plain");
    assert_eq!(
        plain["versions"][0]["_type"], "OBJECT_REF",
        "unresolved versions are OBJECT_REFs: {plain}"
    );

    let resolved = svc
        .get_contribution_resolved(ehr_uuid, cid)
        .await
        .expect("resolved");
    let v = &resolved["versions"][0];
    assert_eq!(
        v["_type"], "ORIGINAL_VERSION",
        "resolve_refs returns the full VERSION: {resolved}"
    );
    assert_eq!(
        v["uid"]["value"].as_str().expect("version uid"),
        comp_uid,
        "the resolved version is the committed composition version"
    );
    assert!(
        v["data"]["_type"] == "COMPOSITION",
        "resolved version carries its data"
    );
}

/// A client-supplied CONTRIBUTION uid is honoured when unused, rejected as a
/// conflict when already in use, and rejected as unprocessable when malformed
/// (ITS-REST `contribution_create`; RM common master06 §CONTRIBUTION `uid`).
#[tokio::test]
async fn contribution_supplied_uid() {
    let pg = Pg::start().await;
    let svc = EhrbaseService::new(pg.migrated_pool("contribution_uid").await);

    let ehr_id = create_ehr(&svc).await;
    let ehr_uuid = ehr_id.parse::<uuid::Uuid>().expect("ehr uuid");

    let wanted = uuid::Uuid::now_v7();
    let mut body = serde_json::json!({
        "uid": { "_type": "HIER_OBJECT_ID", "value": wanted.to_string() },
        "versions": [{
            "data": composition("obs"),
            "lifecycle_state": { "code_string": "532", "terminology_id": { "value": "openehr" } },
            "commit_audit": { "change_type": { "value": "creation",
                "defining_code": { "code_string": "249", "terminology_id": { "value": "openehr" } } } }
        }],
        "audit": { "committer": { "_type": "PARTY_IDENTIFIED", "name": "T" } }
    });
    let resp = svc
        .create_ehr_contribution(ehr_uuid, body.clone())
        .await
        .expect("supplied uid accepted");
    assert_eq!(
        resp.body["uid"]["value"].as_str(),
        Some(wanted.to_string().as_str()),
        "the supplied uid is the stored uid"
    );

    // Re-using the uid conflicts.
    body["versions"][0]["data"] = composition("obs2");
    let dup = svc
        .create_ehr_contribution(ehr_uuid, body)
        .await
        .expect_err("duplicate uid rejected");
    assert!(dup.message.contains("already in use"), "got {dup:?}");
}

/// The combined EHR-existence + content-writability create gate
/// (`ensure_ehr_content_writable`) preserves the pre-fold error surface after
/// the two separate pool reads were collapsed into one `ehr_writability` round
/// trip: an unknown EHR still maps to `VersionedObjectDoesNotExist` (404, never
/// a DB error or a conflict), and a deactivated EHR (`EHR_STATUS.is_modifiable =
/// false`) still maps to a conflict (409) — RM ehr master04 §EHR Creation /
/// §EHR Active Status.
#[tokio::test]
async fn create_composition_gate_error_surface_survives_the_writability_fold() {
    let pg = Pg::start().await;
    let svc = EhrbaseService::new(pg.migrated_pool("writability_fold").await);

    // (1) Unknown EHR → 404 VersionedObjectDoesNotExist (the existence signal of
    // the folded query), never a conflict and never a driver error.
    let ghost = "00000000-0000-7000-8000-0000000000fe"
        .parse::<uuid::Uuid>()
        .expect("uuid");
    let missing = svc
        .create_composition(ghost, uv(composition("obs"), "249", None))
        .await
        .expect_err("unknown EHR rejected");
    assert_eq!(
        missing.status,
        CallStatusType::VersionedObjectDoesNotExist,
        "unknown ehr_id → 404, got {missing:?}"
    );

    // A live, modifiable EHR accepts a composition (the fold does not falsely
    // block — is_modifiable = None/true → writable).
    let ehr_id = create_ehr(&svc).await;
    let ehr_uuid = ehr_id.parse::<uuid::Uuid>().expect("ehr uuid");
    svc.create_composition(ehr_uuid, uv(composition("obs"), "249", None))
        .await
        .expect("modifiable EHR accepts a composition");

    // (2) Deactivate the EHR (EHR_STATUS.is_modifiable = false) and retry: the
    // content write is refused with a conflict (the modifiability signal of the
    // folded query, checked after existence).
    let status_uid = svc
        .get_ehr_status_at_time(ehr_uuid, None)
        .await
        .expect("current EHR_STATUS")["uid"]["value"]
        .as_str()
        .expect("status uid")
        .to_owned();
    let deactivated = json!({
        "_type": "EHR_STATUS",
        "archetype_node_id": "openEHR-EHR-EHR_STATUS.generic.v1",
        "name": { "_type": "DV_TEXT", "value": "EHR Status" },
        "subject": { "_type": "PARTY_SELF" },
        "is_queryable": true,
        "is_modifiable": false
    });
    svc.replace_ehr_status(ehr_uuid, uv(deactivated, "251", Some(&status_uid)))
        .await
        .expect("EHR_STATUS deactivation");

    let blocked = svc
        .create_composition(ehr_uuid, uv(composition("obs2"), "249", None))
        .await
        .expect_err("non-modifiable EHR blocks content writes");
    assert_eq!(
        blocked.status,
        CallStatusType::CompositionAlreadyExists,
        "is_modifiable = false → 409 conflict, got {blocked:?}"
    );
    assert!(
        blocked.message.contains("not modifiable"),
        "got {blocked:?}"
    );
}

/// The temporal non-overlap invariant survives the removal of the `GiST`
/// EXCLUDE constraints (RM common master06 §Version tree: one valid version
/// per lineage at any instant; the enforcement is now by construction —
/// close-then-insert at one `now()` per write, one open row per lineage via
/// the partial unique indexes). A burst of sequential updates must leave
/// exactly one open trunk row and ZERO overlapping validity pairs — asserted
/// with the same lineage-pair query the admin archive load audits with.
#[tokio::test]
async fn version_validity_never_overlaps_without_the_exclusion_constraints() {
    let pg = Pg::start().await;
    let pool = pg.migrated_pool("no_overlap_invariant").await;
    let svc = EhrbaseService::new(pool.clone());

    let ehr_id = create_ehr(&svc).await;
    let ehr_uuid = ehr_id.parse::<uuid::Uuid>().expect("ehr uuid");
    let created = svc
        .create_composition(ehr_uuid, uv(composition("obs"), "249", None))
        .await
        .expect("create");
    let mut preceding = created;
    for i in 0..8 {
        preceding = svc
            .update_composition(
                ehr_uuid,
                preceding
                    .split("::")
                    .next()
                    .expect("vo id part")
                    .parse()
                    .expect("vo uuid"),
                uv(composition(&format!("obs-v{i}")), "251", Some(&preceding)),
            )
            .await
            .expect("update");
    }

    let open_trunk: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM vo_version \
         WHERE ehr_id = $1 AND kind = 'COMPOSITION' \
           AND branch_number = 0 AND upper_inf(sys_period)",
    )
    .bind(ehr_uuid)
    .fetch_one(&pool)
    .await
    .expect("open-row count");
    assert_eq!(open_trunk, 1, "exactly one open trunk row per composition");

    let overlap: bool = sqlx::query_scalar(
        "SELECT EXISTS ( \
             SELECT 1 FROM vo_version a \
             JOIN vo_version b ON a.vo_id = b.vo_id \
                 AND a.branch_number = b.branch_number \
                 AND (a.branch_number = 0 \
                      OR (a.creating_system_id = b.creating_system_id \
                          AND a.trunk_version = b.trunk_version)) \
                 AND a.sys_version < b.sys_version \
                 AND a.sys_period && b.sys_period \
             WHERE a.ehr_id = $1)",
    )
    .bind(ehr_uuid)
    .fetch_one(&pool)
    .await
    .expect("overlap audit");
    assert!(!overlap, "no lineage carries overlapping validity periods");
}
