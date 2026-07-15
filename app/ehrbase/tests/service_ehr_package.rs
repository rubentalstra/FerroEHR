//! RM EHR-package cross-version + tag-scoping duties, end-to-end against a
//! real `PostgreSQL` 18 (testcontainers) — A1 rm-ehr chapter.
//!
//! Spec: RM ehr `org.openehr.rm.ehr.versioned_composition.adoc`
//! (`Archetype_node_id_valid`, `Persistent_validity` — a versioned composition
//! cannot switch archetype or persistence category across versions) and
//! `org.openehr.rm.ehr.ehr.adoc` `EHR.tags` + master04 §Tags ("Tag `_target_`
//! values can only be within the same EHR").

#![allow(clippy::expect_used, clippy::unwrap_used, clippy::too_many_lines)]

use ehrbase::db::{self, DbConfig};
use ehrbase::service::EhrbaseService;
use ehrbase_sm::ItemTagAdapter;
use ehrbase_sm::{CallStatusType, EhrCompositionService, EhrService, SmError};
use ehrbase_sm::{UpdateAudit, UpdateVersion};
use openehr_base::prelude::TerminologyCode;
use openehr_rm::prelude::PartyProxy;
use serde_json::{Value, json};
use sqlx::{AssertSqlSafe, Connection, PgConnection, PgPool};
use testcontainers::runners::AsyncRunner;
use testcontainers::{ContainerAsync, ImageExt};
use testcontainers_modules::postgres::Postgres;
use uuid::Uuid;

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
            committer: serde_json::from_value::<PartyProxy>(
                json!({ "_type": "PARTY_IDENTIFIED", "name": "ehr-package tester" }),
            )
            .expect("committer"),
            system_id: None,
        },
        signature: None,
    }
}

/// A minimal *valid* RM COMPOSITION with a chosen archetype + category.
fn composition(archetype: &str, category_code: &str, category_label: &str) -> Value {
    json!({
        "_type": "COMPOSITION",
        "archetype_node_id": archetype,
        "archetype_details": {
            "_type": "ARCHETYPED",
            "archetype_id": { "_type": "ARCHETYPE_ID", "value": archetype },
            "rm_version": "1.2.0"
        },
        "name": { "_type": "DV_TEXT", "value": "ehr-package test" },
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
            "value": category_label,
            "defining_code": {
                "_type": "CODE_PHRASE",
                "terminology_id": { "_type": "TERMINOLOGY_ID", "value": "openehr" },
                "code_string": category_code
            }
        },
        "composer": { "_type": "PARTY_IDENTIFIED", "name": "ehr-package tester" }
    })
}

const ENCOUNTER: &str = "openEHR-EHR-COMPOSITION.encounter.v1";
const REPORT: &str = "openEHR-EHR-COMPOSITION.report.v1";

/// `VERSIONED_COMPOSITION.Archetype_node_id_valid`: an update whose root
/// `archetype_node_id` differs from the first version's is a 422; the same
/// archetype updates fine.
#[tokio::test]
async fn versioned_composition_cannot_switch_archetype() {
    let pg = Pg::start().await;
    let svc = EhrbaseService::new(pg.migrated_pool("vc_archetype").await);
    let ehr = svc.create_ehr(None).await.expect("ehr");
    let v1 = svc
        .create_composition(ehr, uv(composition(ENCOUNTER, "433", "event"), "249", None))
        .await
        .expect("v1");
    let vo: Uuid = v1.split("::").next().unwrap().parse().unwrap();

    let err = svc
        .update_composition(
            ehr,
            vo,
            uv(composition(REPORT, "433", "event"), "251", Some(&v1)),
        )
        .await
        .expect_err("switching archetype across versions must be rejected");
    match err {
        SmError {
            status: CallStatusType::ContentInvalid,
            message,
        } => assert!(
            message.contains("Archetype_node_id_valid"),
            "should cite the invariant, got: {message}"
        ),
        other => panic!("expected ContentInvalid, got {other:?}"),
    }

    // Same archetype: fine.
    svc.update_composition(
        ehr,
        vo,
        uv(composition(ENCOUNTER, "433", "event"), "251", Some(&v1)),
    )
    .await
    .expect("same-archetype update");
}

/// `VERSIONED_COMPOSITION.Persistent_validity`: flipping the category between
/// persistent (431) and event (433) across versions is a 422.
#[tokio::test]
async fn versioned_composition_cannot_flip_persistence() {
    let pg = Pg::start().await;
    let svc = EhrbaseService::new(pg.migrated_pool("vc_persistence").await);
    let ehr = svc.create_ehr(None).await.expect("ehr");
    let v1 = svc
        .create_composition(ehr, uv(composition(ENCOUNTER, "433", "event"), "249", None))
        .await
        .expect("v1");
    let vo: Uuid = v1.split("::").next().unwrap().parse().unwrap();

    let err = svc
        .update_composition(
            ehr,
            vo,
            uv(
                composition(ENCOUNTER, "431", "persistent"),
                "251",
                Some(&v1),
            ),
        )
        .await
        .expect_err("flipping is_persistent across versions must be rejected");
    match err {
        SmError {
            status: CallStatusType::ContentInvalid,
            message,
        } => assert!(
            message.contains("Persistent_validity"),
            "should cite the invariant, got: {message}"
        ),
        other => panic!("expected ContentInvalid, got {other:?}"),
    }
}

/// `EHR.tags`: "Tag `_target_` values can only be within the same EHR" — a
/// tag whose target versioned object lives in ANOTHER EHR is rejected, while
/// tagging one's own composition succeeds.
#[tokio::test]
async fn tag_targets_must_be_within_the_same_ehr() {
    let pg = Pg::start().await;
    let svc = EhrbaseService::new(pg.migrated_pool("tag_scope").await);
    let ehr_a = svc.create_ehr(None).await.expect("ehr A");
    let ehr_b = svc.create_ehr(None).await.expect("ehr B");
    let v1 = svc
        .create_composition(
            ehr_a,
            uv(composition(ENCOUNTER, "433", "event"), "249", None),
        )
        .await
        .expect("composition in A");
    let vo_a: Uuid = v1.split("::").next().unwrap().parse().unwrap();

    let tag = json!([{ "key": "clin-proj-27a" }]);
    // Tagging A's composition from A: accepted.
    svc.target_tags_replace(
        ehr_a,
        vo_a.to_string(),
        "EHR_STATUS",
        tag.as_array().unwrap().clone(),
    )
    .await
    .expect("own-EHR tag accepted");

    // Tagging A's composition from B: rejected — cross-EHR target.
    let err = svc
        .target_tags_replace(
            ehr_b,
            vo_a.to_string(),
            "EHR_STATUS",
            tag.as_array().unwrap().clone(),
        )
        .await
        .expect_err("cross-EHR tag target must be rejected");
    assert!(
        err.message.contains("same EHR"),
        "should cite the same-EHR duty, got: {}",
        err.message
    );
}
