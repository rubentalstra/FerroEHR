//! Version-tree branching + merge provenance, end-to-end against a real
//! `PostgreSQL` 18 (testcontainers).
//!
//! Spec: RM common `master06-change_control_package.adoc` §Version tree /
//! §Distributed versioning — "To support branching, a further pair of numbers
//! is added … branching version identifiers [are required] when local
//! modifications are made to versions copied from elsewhere" — and §Version
//! Merging (`ORIGINAL_VERSION.other_input_version_uids`). BASE
//! `VERSION_TREE_ID`: `trunk_version [ '.' branch_number '.' branch_version ]`.
//!
//! Covers (A1 rm-common-change-control R7/R19/R50):
//! 1. modifying an imported version created by ANOTHER system forks a branch
//!    (`t.1.1`, local `creating_system_id`) while the imported trunk version
//!    stays the container current;
//! 2. continuing one's own branch tip advances it (`t.1.2`) and supersedes
//!    the previous tip; every branch version stays addressable by its full
//!    `OBJECT_VERSION_ID`, with the true `preceding_version_uid` served;
//! 3. `other_input_version_uids` round-trips the CONTRIBUTION wire and is
//!    served on the `ORIGINAL_VERSION`;
//! 4. an exported version tree containing branches re-imports whole
//!    (branch import is first-class).

#![allow(clippy::expect_used, clippy::unwrap_used, clippy::too_many_lines)]

use ehrbase::db::{self, DbConfig};
use ehrbase::service::EhrbaseService;
use ehrbase::service::version_update::{UpdateAudit, UpdateVersion};
use openehr_base::prelude::TerminologyCode;
use openehr_rm::prelude::PartyProxy;
use serde_json::{Value, json};
use sqlx::{AssertSqlSafe, Connection, PgConnection, PgPool};
use testcontainers::runners::AsyncRunner;
use testcontainers::{ContainerAsync, ImageExt};
use testcontainers_modules::postgres::Postgres;
use uuid::Uuid;

/// The service's own system id (`DEFAULT_SYSTEM_ID`) — the local
/// `creating_system_id` every non-import write records.
const LOCAL: &str = "ehrbase-rs.local";
/// The pretend foreign system a copied version tree originates from.
const FOREIGN: &str = "sysA.example.org";

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
                json!({ "_type": "PARTY_IDENTIFIED", "name": "branching tester" }),
            )
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
        "composer": { "_type": "PARTY_IDENTIFIED", "name": "branching tester" }
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

/// A CONTRIBUTION body modifying one composition (`251|modification|`).
fn modify_contribution(data: &Value, preceding: &str) -> Value {
    json!({
        "versions": [{
            "data": data,
            "preceding_version_uid": { "value": preceding },
            "commit_audit": {
                "change_type": change_type("251", "modification"),
                "committer": committer("branching tester")
            }
        }],
        "audit": {
            "change_type": change_type("251", "modification"),
            "committer": committer("branching tester")
        }
    })
}

/// The `OBJECT_VERSION_ID` of the first version listed in a served CONTRIBUTION.
fn first_version_uid(contribution: &Value) -> String {
    contribution["versions"][0]["id"]["value"]
        .as_str()
        .expect("versions[0].id.value")
        .to_owned()
}

/// A whole-EHR, ALL-versions `EXTRACT_SPEC` (RM `ehr_extract` master04
/// `EXTRACT_VERSION_SPEC.include_all_versions`) — the full-tree copy the
/// latest-only `export_ehrs` deliberately is not (latest = the latest TRUNK
/// version, master06).
fn all_versions_spec(ehr: Uuid) -> Value {
    json!({
        "_type": "EXTRACT_SPEC",
        "version_spec": {
            "_type": "EXTRACT_VERSION_SPEC",
            "include_all_versions": true,
            "include_revision_history": false,
            "include_data": true
        },
        "manifest": {
            "_type": "EXTRACT_MANIFEST",
            "entities": [ {
                "_type": "EXTRACT_ENTITY_MANIFEST",
                "extract_id_key": ehr.to_string(),
                "ehr_id": ehr.to_string(),
                "other_ids": [],
                "item_list": []
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
    })
}

/// Seed an EHR holding one COMPOSITION at trunk v2, then export it and rewrite
/// every version identity to the FOREIGN system — producing the extract a copy
/// from another openEHR system would carry (RM common master06 §Copying: the
/// `creating_system_id` identifies the system of original creation).
async fn foreign_extract(svc: &EhrbaseService) -> (Value, String) {
    let source = svc.create_ehr(None).await.expect("create source ehr");
    let v1 = svc
        .create_composition(source, uv(composition("v1"), "249", None))
        .await
        .expect("composition v1");
    let vo = v1.split("::").next().unwrap().to_owned();
    svc.update_composition(
        source,
        vo.parse().unwrap(),
        uv(composition("v2"), "251", Some(&v1)),
    )
    .await
    .expect("composition v2");

    let mut extracts = svc.extract_ehrs(source).await.expect("export");
    let extract = extracts.remove(0);
    let rewritten: Value = serde_json::from_str(
        &serde_json::to_string(&extract)
            .unwrap()
            .replace(&format!("::{LOCAL}::"), &format!("::{FOREIGN}::")),
    )
    .unwrap();
    (rewritten, vo)
}

/// Import the foreign extract into a fresh EHR id on `svc`, returning
/// (target ehr id, composition vo id).
async fn import_foreign(svc: &EhrbaseService, extract: Value, vo: &str) -> (Uuid, Uuid) {
    let target = Uuid::now_v7();
    svc.import_ehr(
        Some(target),
        serde_json::from_value(extract).expect("EXTRACT deserializes"),
    )
    .await
    .expect("import_ehr");
    (target, vo.parse().unwrap())
}

#[tokio::test]
async fn modifying_an_imported_foreign_version_forks_a_branch() {
    let pg = Pg::start().await;
    // The source and the importing target are separate repositories (a copied
    // versioned object keeps its vo_id, so importing into the SAME repository
    // that already owns it is — correctly — a conflict).
    let source_svc = EhrbaseService::new(pg.migrated_pool("branch_fork_src").await);
    let svc = EhrbaseService::new(pg.migrated_pool("branch_fork").await);
    let (extract, vo) = foreign_extract(&source_svc).await;
    let (target, vo_id) = import_foreign(&svc, extract, &vo).await;

    // (1) Modify the imported trunk tip (created by FOREIGN) → the commit must
    // fork branch 2.1.1 with the LOCAL creating_system_id (master06: "branching
    // version identifiers [are required] when local modifications are made to
    // versions copied from elsewhere").
    let foreign_tip = format!("{vo_id}::{FOREIGN}::2");
    let contribution = svc
        .create_ehr_contribution(
            target,
            modify_contribution(&composition("local mod"), &foreign_tip),
        )
        .await
        .expect("branch-forking modification");
    let branch_uid = first_version_uid(&contribution.body);
    assert_eq!(
        branch_uid,
        format!("{vo_id}::{LOCAL}::2.1.1"),
        "a local modification of a foreign version forks branch t.1.1"
    );

    // (2) The container current (latest trunk, master06 latest_trunk_version)
    // is STILL the imported foreign trunk v2 — the branch coexists.
    let current = svc
        .get_composition_latest(target, vo_id)
        .await
        .expect("current composition");
    assert_eq!(
        current["uid"]["value"].as_str().unwrap(),
        foreign_tip,
        "the trunk tip stays current; a fork does not supersede it"
    );

    // (3) The branch version is addressable by its full OBJECT_VERSION_ID and
    // serves the TRUE preceding_version_uid (the foreign trunk version).
    let ov = svc
        .composition_original_version(target, branch_uid.parse().unwrap())
        .await
        .expect("branch ORIGINAL_VERSION");
    assert_eq!(ov["uid"]["value"], json!(branch_uid));
    assert_eq!(
        ov["preceding_version_uid"]["value"],
        json!(foreign_tip),
        "preceding_version_uid is stored, not synthesized — it names the \
         foreign version the branch forked from"
    );

    // (4) Continue OUR branch tip → 2.1.2, superseding 2.1.1; both remain
    // readable (versions are indelible).
    let contribution2 = svc
        .create_ehr_contribution(
            target,
            modify_contribution(&composition("local mod 2"), &branch_uid),
        )
        .await
        .expect("branch continuation");
    let branch_uid2 = first_version_uid(&contribution2.body);
    assert_eq!(branch_uid2, format!("{vo_id}::{LOCAL}::2.1.2"));
    let ov2 = svc
        .composition_original_version(target, branch_uid2.parse().unwrap())
        .await
        .expect("branch tip ORIGINAL_VERSION");
    assert_eq!(ov2["preceding_version_uid"]["value"], json!(branch_uid));
    svc.composition_original_version(target, branch_uid.parse().unwrap())
        .await
        .expect("superseded branch version stays readable");

    // (5) A second fork from the same foreign trunk version numbers the NEXT
    // branch (2.2.1) — branch numbers count per fork point (master06
    // §Version tree).
    let contribution3 = svc
        .create_ehr_contribution(
            target,
            modify_contribution(&composition("second fork"), &foreign_tip),
        )
        .await
        .expect("second fork");
    assert_eq!(
        first_version_uid(&contribution3.body),
        format!("{vo_id}::{LOCAL}::2.2.1")
    );
}

#[tokio::test]
async fn merge_provenance_round_trips_the_wire() {
    let pg = Pg::start().await;
    let svc = EhrbaseService::new(pg.migrated_pool("merge_prov").await);
    let ehr = svc.create_ehr(None).await.expect("ehr");
    let v1 = svc
        .create_composition(ehr, uv(composition("v1"), "249", None))
        .await
        .expect("v1");
    let vo: Uuid = v1.split("::").next().unwrap().parse().unwrap();

    // A modification carrying other_input_version_uids (master06 §Version
    // Merging — the merged-in inputs) is stored and served.
    let merged_in = format!("{}::{FOREIGN}::3", Uuid::now_v7());
    let mut body = modify_contribution(&composition("merged"), &v1);
    body["versions"][0]["other_input_version_uids"] = json!([{ "value": merged_in }]);
    let contribution = svc
        .create_ehr_contribution(ehr, body)
        .await
        .expect("merge commit");
    let v2 = first_version_uid(&contribution.body);
    let ov = svc
        .composition_original_version(ehr, v2.parse().unwrap())
        .await
        .expect("merged ORIGINAL_VERSION");
    assert_eq!(
        ov["other_input_version_uids"][0]["value"],
        json!(merged_in),
        "other_input_version_uids round-trips (Is_merged_validity: is_merged \
         is its derived boolean)"
    );
    // A plain version carries none.
    let ov1 = svc
        .composition_original_version(ehr, v1.parse().unwrap())
        .await
        .expect("v1");
    assert!(ov1.get("other_input_version_uids").is_none());
    let _ = vo;
}

#[tokio::test]
async fn a_version_tree_with_branches_reexports_and_reimports_whole() {
    let pg = Pg::start().await;
    let source_svc = EhrbaseService::new(pg.migrated_pool("branch_reimport_src").await);
    let svc = EhrbaseService::new(pg.migrated_pool("branch_reimport").await);
    let (extract, vo) = foreign_extract(&source_svc).await;
    let (target, vo_id) = import_foreign(&svc, extract, &vo).await;

    // Grow a local branch on the imported tree.
    let foreign_tip = format!("{vo_id}::{FOREIGN}::2");
    svc.create_ehr_contribution(
        target,
        modify_contribution(&composition("local branch"), &foreign_tip),
    )
    .await
    .expect("fork");

    // Export the whole tree (trunk from FOREIGN + branch from LOCAL, ALL
    // versions) and re-import into a THIRD repository: the mixed-system,
    // branched version tree is preserved verbatim (master06 §Copying — branch
    // import is first-class).
    let mut extracts = svc
        .export_ehr_extracts(
            serde_json::from_value(all_versions_spec(target)).expect("EXTRACT_SPEC"),
        )
        .await
        .expect("re-export (all versions)");
    let wire = serde_json::to_string(&extracts[0]).unwrap();
    assert!(
        wire.contains(&format!("::{LOCAL}::2.1.1")),
        "the exported version tree must include the branch version"
    );
    let third_svc = EhrbaseService::new(pg.migrated_pool("branch_reimport_third").await);
    let third = Uuid::now_v7();
    third_svc
        .import_ehr(
            Some(third),
            serde_json::from_value(extracts.remove(0)).expect("EXTRACT"),
        )
        .await
        .expect("re-import of a branched, multi-system version tree");

    let branch_uid = format!("{vo_id}::{LOCAL}::2.1.1");
    let ov = third_svc
        .composition_original_version(third, branch_uid.parse().unwrap())
        .await
        .expect("re-imported branch version");
    assert_eq!(ov["uid"]["value"], json!(branch_uid));
    let trunk = third_svc
        .get_composition_latest(third, vo_id)
        .await
        .expect("re-imported trunk current");
    assert_eq!(trunk["uid"]["value"].as_str().unwrap(), foreign_tip);
}
