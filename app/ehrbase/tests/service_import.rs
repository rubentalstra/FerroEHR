//! End-to-end service tests for the EHR Extract **import** path (SM
//! `I_EHR_EXTRACT_SERVICE.import_ehr` / `import_ehr_extract`) against a real
//! `PostgreSQL` 18 (testcontainers).
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
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::too_many_lines)]

use serde_json::{Value, json};
use sqlx::{AssertSqlSafe, Connection, PgConnection, PgPool};
use testcontainers::runners::AsyncRunner;
use testcontainers::{ContainerAsync, ImageExt};
use testcontainers_modules::postgres::Postgres;
use uuid::Uuid;

use openehr_base::prelude::TerminologyCode;
use openehr_rm::ehr_extract::common::extract::Extract;
use openehr_rm::ehr_extract::common::extract_spec::ExtractSpec;
use openehr_rm::prelude::PartyProxy;

use ehrbase::db::{self, DbSettings};
use ehrbase::service::EhrbaseService;
use ehrbase_sm::{
    CallStatusType, EhrDirectoryService, EhrExtractService, EhrService, EhrStatusService,
};
use ehrbase_sm::{UpdateAudit, UpdateVersion};

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

    /// A fresh, migrated database in this container — used to give the import
    /// side its own empty target repository (`import_ehr` imports into an empty
    /// target, and a `VERSIONED_OBJECT` uid is globally unique in a repository).
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
        let settings = DbSettings::new(format!(
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
                json!({ "_type": "PARTY_IDENTIFIED", "name": "conformance tester" }),
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
async fn seed_ehr(svc: &EhrbaseService) -> Uuid {
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
async fn export_one(svc: &EhrbaseService, ehr: Uuid) -> Extract {
    let mut extracts = svc.export_ehrs(ehr).await.expect("export_ehrs");
    assert_eq!(extracts.len(), 1, "one EHR id → one EXTRACT");
    serde_json::from_value(extracts.remove(0))
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

#[tokio::test]
async fn import_ehr_clone_into_fresh_target_reuses_source_id() {
    let pg = Pg::start().await;
    let source = EhrbaseService::new(pg.migrated_pool("import_clone_src").await);
    let target = EhrbaseService::new(pg.migrated_pool("import_clone_tgt").await);

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
    let re_export = target.export_ehrs(ehr).await.expect("re-export clone");
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
}

#[tokio::test]
async fn import_ehr_into_fixed_fresh_id() {
    let pg = Pg::start().await;
    let source = EhrbaseService::new(pg.migrated_pool("import_fixed_src").await);
    let target = EhrbaseService::new(pg.migrated_pool("import_fixed_tgt").await);

    let ehr = seed_ehr(&source).await;
    let source_status = source
        .get_ehr_status_at_time(ehr, None)
        .await
        .expect("source status");

    // A caller-provided fixed id (the SM's "same patient in other EHR services"
    // case): the clone lands under `fixed`, not the source id.
    let fixed = Uuid::now_v7();
    let extract = export_one(&source, ehr).await;
    target
        .import_ehr(Some(fixed), extract)
        .await
        .expect("import_ehr with a fixed id");

    // The source id does not exist in the target; the fixed id does.
    assert_eq!(
        target
            .export_ehrs(ehr)
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
    let pg = Pg::start().await;
    let svc = EhrbaseService::new(pg.migrated_pool("import_dup").await);
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
    let pg = Pg::start().await;
    let source = EhrbaseService::new(pg.migrated_pool("import_extract_src").await);
    let target = EhrbaseService::new(pg.migrated_pool("import_extract_tgt").await);

    let src_ehr = seed_ehr(&source).await;

    // A FOLDER-only extract (item_list restricts to the directory container),
    // so importing it into an existing EHR that has no directory is a clean
    // Case-2 create (no EHR_STATUS/EHR_ACCESS singleton clash).
    let whole = source.export_ehrs(src_ehr).await.expect("whole export");
    let folder_vo = find_by_xtype(&whole[0], "X_VERSIONED_FOLDER").expect("folder in export")
        ["item"]["uid"]["value"]
        .as_str()
        .expect("folder vo uid")
        .to_owned();
    let source_folder_data =
        find_by_xtype(&whole[0], "X_VERSIONED_FOLDER").unwrap()["item"]["versions"][0]["data"]
            .clone();

    let spec: ExtractSpec = serde_json::from_value(json!({
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
        serde_json::from_value(folder_extracts.remove(0)).expect("folder EXTRACT");

    // A fresh target EHR (its own EHR_STATUS/EHR_ACCESS, no directory yet).
    let tgt_ehr = target.create_ehr(None).await.expect("target ehr");
    target
        .import_ehr_extract(tgt_ehr, folder_extract.clone())
        .await
        .expect("import the FOLDER into the existing EHR");

    // The target now carries the imported directory FOLDER with the source's data.
    let re_export = target.export_ehrs(tgt_ehr).await.expect("re-export target");
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
