//! SM-2 end-to-end tests for the Definitions native API against a real
//! `PostgreSQL` 18 (testcontainers): ADL 1.4 source archetypes
//! (`I_DEFINITION_ADL14`), OPTs (delegated to `template_store`), and registered
//! queries (`I_DEFINITION_QUERY`). Driven through the SM service traits exactly
//! as a protocol adapter would call them; the SM pre/post-conditions are the
//! assertions.
//!
//! Requires Docker. Each test owns its container (`Drop` removes it).
#![allow(clippy::expect_used, clippy::unwrap_used)]

use sqlx::{AssertSqlSafe, Connection, PgConnection, PgPool};
use testcontainers::runners::AsyncRunner;
use testcontainers::{ContainerAsync, ImageExt};
use testcontainers_modules::postgres::Postgres;

use ehrbase::db::{self, DbConfig};
use ehrbase::service::EhrbaseService;
use ehrbase_sm::extensions::adapters::TemplateListFilter;
use ehrbase_sm::{
    CallStatusType, DefinitionAdapter, DefinitionAdl2Service, DefinitionAdl14Service,
    DefinitionQueryService, Page, SmError,
};

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

fn fixture(rel: &str) -> String {
    let path = format!("{}/{rel}", env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {path}: {e}"))
}

const ARCHETYPE_REL: &str =
    "tests/resources/service/knowledge/archetypes/openEHR-EHR-COMPOSITION.prescription.v1.adl";
const ARCHETYPE_ID: &str = "openEHR-EHR-COMPOSITION.prescription.v1";

const OPT_REL: &str = "tests/resources/service/knowledge/IDCR Allergies List.v0.opt";
const OPT_TEMPLATE_ID: &str = "IDCR Allergies List.v0";

// ── ADL 1.4 archetypes (I_DEFINITION_ADL14) ──────────────────────────────────

#[tokio::test]
async fn archetype_upload_get_list_match_replace_delete() {
    let pg = Pg::start().await;
    let svc = EhrbaseService::new(pg.migrated_pool("def_arch").await);
    let adl = fixture(ARCHETYPE_REL);

    // Precondition: not present yet.
    assert!(!svc.has_archetype(ARCHETYPE_ID.to_owned()).await.unwrap());
    assert_eq!(
        DefinitionAdl14Service::archetypes_count(&svc)
            .await
            .unwrap(),
        0
    );

    // valid_archetype on good vs bad source.
    assert!(svc.valid_archetype(adl.clone()).await.unwrap());
    assert!(
        !svc.valid_archetype("not an archetype".to_owned())
            .await
            .unwrap()
    );

    // upload → Post_has_archetype.
    svc.upload_archetype(adl.clone()).await.expect("upload");
    assert!(svc.has_archetype(ARCHETYPE_ID.to_owned()).await.unwrap());
    assert_eq!(
        DefinitionAdl14Service::archetypes_count(&svc)
            .await
            .unwrap(),
        1
    );

    // get returns the source verbatim.
    let got = svc
        .get_archetype(ARCHETYPE_ID.to_owned())
        .await
        .expect("get");
    assert_eq!(got, adl, "stored ADL source is byte-identical");

    // list + list_matching (regex).
    let list = DefinitionAdl14Service::list_archetypes(&svc, Page::all())
        .await
        .unwrap();
    assert_eq!(list, vec![ARCHETYPE_ID.to_owned()]);
    let matched = svc
        .list_matching_archetypes("COMPOSITION\\.prescription".to_owned(), Page::all())
        .await
        .unwrap();
    assert_eq!(matched, vec![ARCHETYPE_ID.to_owned()]);
    let none = svc
        .list_matching_archetypes("OBSERVATION".to_owned(), Page::all())
        .await
        .unwrap();
    assert!(none.is_empty());

    // upload_archetype replaces if the id already exists (spec: "replace it").
    let replacement = format!("{adl}\n-- replaced\n");
    svc.upload_archetype(replacement.clone())
        .await
        .expect("replace");
    assert_eq!(
        DefinitionAdl14Service::archetypes_count(&svc)
            .await
            .unwrap(),
        1,
        "replace, not insert"
    );
    assert_eq!(
        svc.get_archetype(ARCHETYPE_ID.to_owned()).await.unwrap(),
        replacement,
        "replacement overwrote the source"
    );

    // delete → Post_archetype_removed.
    svc.delete_archetype(ARCHETYPE_ID.to_owned())
        .await
        .expect("delete");
    assert!(!svc.has_archetype(ARCHETYPE_ID.to_owned()).await.unwrap());
    assert_eq!(
        DefinitionAdl14Service::archetypes_count(&svc)
            .await
            .unwrap(),
        0
    );
}

#[tokio::test]
async fn archetype_errors() {
    let pg = Pg::start().await;
    let svc = EhrbaseService::new(pg.migrated_pool("def_arch_err").await);

    // Invalid ADL → 422 invalid_archetype.
    // PORT NOTE: the SM-specific Definition statuses (invalid_archetype,
    // artefact_does_not_exist, invalid_id_pattern, …) are flattened through the
    // service's `ServiceError::sm()` → `From<ServiceError> for SmError` round-trip
    // to the generic `content_invalid` (422) / `versioned_object_does_not_exist`
    // (404) / `precondition_violation` (400) — the wire HTTP codes are unchanged.
    let bad = svc
        .upload_archetype("this is not an archetype".to_owned())
        .await
        .expect_err("invalid archetype rejected");
    assert!(
        matches!(
            bad,
            SmError {
                status: CallStatusType::ContentInvalid,
                ..
            }
        ),
        "got {bad:?}"
    );

    // get / delete of an absent archetype → 404.
    let missing = svc
        .get_archetype("openEHR-EHR-OBSERVATION.absent.v1".to_owned())
        .await
        .expect_err("absent archetype");
    assert!(
        matches!(
            missing,
            SmError {
                status: CallStatusType::VersionedObjectDoesNotExist,
                ..
            }
        ),
        "got {missing:?}"
    );
    let del_missing = svc
        .delete_archetype("openEHR-EHR-OBSERVATION.absent.v1".to_owned())
        .await
        .expect_err("delete absent");
    assert!(
        matches!(
            del_missing,
            SmError {
                status: CallStatusType::VersionedObjectDoesNotExist,
                ..
            }
        ),
        "got {del_missing:?}"
    );

    // Uncompilable regex → 400 invalid_id_pattern (unbalanced group).
    let bad_re = svc
        .list_matching_archetypes("(".to_owned(), Page::all())
        .await
        .expect_err("invalid pattern");
    assert!(
        matches!(
            bad_re,
            SmError {
                status: CallStatusType::PreconditionViolation,
                ..
            }
        ),
        "got {bad_re:?}"
    );
}

// ── ADL 1.4 OPTs (I_DEFINITION_ADL14; on template_store, UUID-keyed) ──────────

#[tokio::test]
async fn opt_upload_has_get_list_match_delete() {
    let pg = Pg::start().await;
    let svc = EhrbaseService::new(pg.migrated_pool("def_opt").await);
    let xml = fixture(OPT_REL);

    // valid_opt on good vs bad XML.
    assert!(svc.valid_opt(xml.clone()).await.unwrap());
    assert!(!svc.valid_opt("<not-a-template/>".to_owned()).await.unwrap());
    assert_eq!(DefinitionAdl14Service::opts_count(&svc).await.unwrap(), 0);

    // upload_opt (Pre_valid).
    svc.upload_opt(xml.clone()).await.expect("upload opt");
    assert_eq!(DefinitionAdl14Service::opts_count(&svc).await.unwrap(), 1);

    // list_opts yields the OPT's UUID; use it for has/get/delete (UUID-keyed).
    let opts = DefinitionAdl14Service::list_opts(&svc, Page::all())
        .await
        .unwrap();
    assert_eq!(opts.len(), 1);
    let uuid = opts[0].clone();
    assert!(
        uuid::Uuid::parse_str(&uuid).is_ok(),
        "list_opts returns a UUID"
    );
    assert!(svc.has_opt(uuid.clone()).await.unwrap());
    let got = svc.get_opt(uuid.clone()).await.expect("get opt");
    assert_eq!(got, xml, "stored OPT XML is byte-identical");

    // list_matching_opts matches on the template_id (spec return-type defect —
    // we return template ids, per the PORT NOTE).
    let matched = svc
        .list_matching_opts("IDCR.*Allergies".to_owned(), Page::all())
        .await
        .unwrap();
    assert_eq!(matched, vec![OPT_TEMPLATE_ID.to_owned()]);

    // Re-uploading the same template_id conflicts (wire's CNF-tested rule wins).
    let conflict = svc
        .upload_opt(xml.clone())
        .await
        .expect_err("duplicate opt");
    assert!(
        matches!(
            conflict,
            SmError {
                status: CallStatusType::CompositionAlreadyExists,
                ..
            }
        ),
        "got {conflict:?}"
    );

    // delete_opt (Pre_has_opt / Post_opt_removed).
    svc.delete_opt(uuid.clone()).await.expect("delete opt");
    assert!(!svc.has_opt(uuid.clone()).await.unwrap());
    assert_eq!(DefinitionAdl14Service::opts_count(&svc).await.unwrap(), 0);
}

/// The ITS-REST `definition_template_adl1.4_list` filter + pagination params
/// (`operations/definition_template_adl1.4_list.yaml`;
/// `parameters/query/filter_template_id.yaml` — "supports wildcards `*`";
/// `master02-overview.adoc` §List Handling). Two OPTs are uploaded through the
/// wire-shaped `DefinitionAdapter`; the list must honour the `template_id` glob
/// and the `offset`/`fetch` window (the G-1 gap: previously every param was
/// dropped and the full set returned).
#[tokio::test]
async fn template_adl14_list_filters_and_paginates() {
    let pg = Pg::start().await;
    let svc = EhrbaseService::new(pg.migrated_pool("def_tpl_list").await);

    // Two templates: "IDCR Allergies List.v0" and "IDCR Problem List.v1".
    svc.template_adl14_upload(fixture(OPT_REL))
        .await
        .expect("upload allergies");
    svc.template_adl14_upload(fixture(
        "tests/resources/service/knowledge/IDCR Problem List.v1.opt",
    ))
    .await
    .expect("upload problem");

    let template_ids = |list: &[serde_json::Value]| -> Vec<String> {
        list.iter()
            .map(|t| t["template_id"].as_str().unwrap_or_default().to_owned())
            .collect()
    };

    // Empty filter + Page::all → the full set (both templates).
    let all = svc
        .template_adl14_list(TemplateListFilter::default(), Page::all())
        .await
        .expect("list all");
    assert_eq!(all.len(), 2, "both templates without a filter");

    // template_id glob "IDCR Allergies*" → only the allergies template.
    let filtered = svc
        .template_adl14_list(
            TemplateListFilter {
                template_id: Some("IDCR Allergies*".to_owned()),
                ..TemplateListFilter::default()
            },
            Page::all(),
        )
        .await
        .expect("list filtered");
    assert_eq!(
        template_ids(&filtered),
        vec![OPT_TEMPLATE_ID.to_owned()],
        "glob matches only the allergies template"
    );

    // A glob matching neither → empty.
    let none = svc
        .template_adl14_list(
            TemplateListFilter {
                template_id: Some("does-not-exist*".to_owned()),
                ..TemplateListFilter::default()
            },
            Page::all(),
        )
        .await
        .expect("list empty");
    assert!(none.is_empty(), "no template matches the glob");

    // offset=1 fetch=1 over the sorted set (Allergies < Problem) → the second.
    let paged = svc
        .template_adl14_list(
            TemplateListFilter::default(),
            Page {
                item_offset: Some(1),
                items_to_fetch: Some(1),
            },
        )
        .await
        .expect("list paged");
    assert_eq!(
        template_ids(&paged),
        vec!["IDCR Problem List.v1".to_owned()],
        "offset=1 fetch=1 yields the second sorted template"
    );
}

/// The wire `query_type` formalism: a non-AQL formalism is an honest
/// *unsupported-formalism* reject (a distinct `precondition_violation`/`400`,
/// per `operations/definition_query_store.yaml`'s `200/400` set +
/// `parameters/query/query_type.yaml`), not a blanket "invalid AQL"; an AQL
/// formalism stores.
#[tokio::test]
async fn query_store_rejects_non_aql_formalism() {
    let pg = Pg::start().await;
    let svc = EhrbaseService::new(pg.migrated_pool("def_qtype").await);

    let aql = "SELECT c FROM EHR e CONTAINS COMPOSITION c".to_owned();

    // A non-AQL formalism → unsupported-formalism reject.
    let err = svc
        .query_store(
            "org.example::q1".to_owned(),
            Some("1.0.0".to_owned()),
            "SQL".to_owned(),
            aql.clone(),
        )
        .await
        .expect_err("non-AQL formalism must be rejected");
    assert!(
        matches!(
            err,
            SmError {
                status: CallStatusType::PreconditionViolation,
                ..
            }
        ),
        "non-AQL is precondition_violation, got {err:?}"
    );

    // The AQL formalism (case-insensitive) stores.
    svc.query_store(
        "org.example::q1".to_owned(),
        Some("1.0.0".to_owned()),
        "aql".to_owned(),
        aql,
    )
    .await
    .expect("AQL formalism stores");
}

#[tokio::test]
async fn opt_errors() {
    let pg = Pg::start().await;
    let svc = EhrbaseService::new(pg.migrated_pool("def_opt_err").await);

    // Invalid OPT → 422 invalid_template.
    let bad = svc
        .upload_opt("<not-a-template/>".to_owned())
        .await
        .expect_err("invalid opt");
    // PORT NOTE: the OPT ingestion path returns `ServiceError::Unprocessable`,
    // flattened at the SM boundary to `content_invalid`.
    assert!(
        matches!(
            bad,
            SmError {
                status: CallStatusType::ContentInvalid,
                ..
            }
        ),
        "got {bad:?}"
    );

    // Unparseable UUID → 400.
    let bad_uuid = svc
        .has_opt("not-a-uuid".to_owned())
        .await
        .expect_err("bad uuid");
    assert!(
        matches!(
            bad_uuid,
            SmError {
                status: CallStatusType::PreconditionViolation,
                ..
            }
        ),
        "got {bad_uuid:?}"
    );

    // Absent (well-formed) UUID → get/delete 404.
    let absent = uuid::Uuid::now_v7().to_string();
    let get_missing = svc.get_opt(absent.clone()).await.expect_err("absent opt");
    assert!(
        matches!(
            get_missing,
            SmError {
                status: CallStatusType::VersionedObjectDoesNotExist,
                ..
            }
        ),
        "got {get_missing:?}"
    );
    let del_missing = svc.delete_opt(absent).await.expect_err("delete absent opt");
    assert!(
        matches!(
            del_missing,
            SmError {
                status: CallStatusType::VersionedObjectDoesNotExist,
                ..
            }
        ),
        "got {del_missing:?}"
    );
}

// ── ADL2 artefacts (I_DEFINITION_ADL2; on adl2_artefact, HRID-keyed) ──────────

/// Build a minimal spec-valid ADL2 source: header, HRID, optional
/// `specialize`, `language`, `definition` (root node `id1`, or `id1.1` when
/// specialised — AOM2 master08 VARCN), `terminology` (ADL2 master02
/// §Structure — the registration validator enforces STCNT + the
/// terminology-side AOM2 rules).
fn adl2_source(keyword: &str, hrid: &str, specialize: Option<&str>) -> String {
    let rm_type = hrid
        .split('.')
        .next()
        .and_then(|q| q.rsplit_once('-').map(|(_, e)| e))
        .expect("HRID carries an RM entity");
    let root = if specialize.is_some() { "id1.1" } else { "id1" };
    let spec = specialize.map_or(String::new(), |p| format!("\nspecialize\n    {p}\n"));
    format!(
        "{keyword} (adl_version=2.0.6; rm_release=1.1.0)\n    {hrid}\n{spec}\n\
         language\n    original_language = <[ISO_639-1::en]>\n\n\
         definition\n    {rm_type}[{root}] matches {{ *}}\n\n\
         terminology\n    term_definitions = <\n        [\"en\"] = <\n            \
         [\"{root}\"] = <text = <\"Root\"> description = <\"Root.\">>\n        >\n    >\n"
    )
}

const ADL2_OPT_HRID: &str = "openEHR-EHR-COMPOSITION.t_clinical_info.v1.0.0";
const ADL2_ARCH_HRID: &str = "openEHR-EHR-OBSERVATION.bp.v1.0.0";
const ADL2_TMPL_HRID: &str = "openEHR-EHR-COMPOSITION.t_vitals.v2.0.0";

#[tokio::test]
#[allow(clippy::too_many_lines)] // one full I_DEFINITION_ADL2 lifecycle on one container
async fn adl2_upload_get_list_by_kind_match_replace_delete() {
    let pg = Pg::start().await;
    let svc = EhrbaseService::new(pg.migrated_pool("def_adl2").await);

    // Preconditions: empty, and valid_artefact on good vs bad source.
    assert_eq!(svc.artefacts_count().await.unwrap(), 0);
    let adl2_opt = adl2_source(
        "operational_template",
        ADL2_OPT_HRID,
        Some("openEHR-EHR-COMPOSITION.discharge.v1"),
    );
    let adl2_arch = adl2_source("archetype", ADL2_ARCH_HRID, None);
    let adl2_tmpl = adl2_source("template", ADL2_TMPL_HRID, None);
    assert!(svc.valid_artefact(adl2_opt.clone()).await.unwrap());
    assert!(
        !svc.valid_artefact("this is not adl2".to_owned())
            .await
            .unwrap()
    );

    // upload_artefact (OPT) → Post_has_artefact (keyed by ARCHETYPE_HRID).
    assert!(!svc.has_artefact(ADL2_OPT_HRID.to_owned()).await.unwrap());
    svc.upload_artefact(adl2_opt.clone())
        .await
        .expect("upload opt");
    assert!(svc.has_artefact(ADL2_OPT_HRID.to_owned()).await.unwrap());

    // get_artefact returns the source verbatim.
    assert_eq!(
        svc.get_artefact(ADL2_OPT_HRID.to_owned()).await.unwrap(),
        adl2_opt,
        "stored ADL2 source is byte-identical"
    );

    // Upload one artefact of each other kind.
    svc.upload_artefact(adl2_arch.clone())
        .await
        .expect("upload archetype");
    svc.upload_artefact(adl2_tmpl.clone())
        .await
        .expect("upload template");

    // Counts by concrete type. `archetypes_count`/`opts_count` (and
    // `list_archetypes`/`list_opts`) are declared on both the ADL 1.4 and ADL2
    // Definitions traits, so calls on the concrete service are qualified.
    assert_eq!(svc.artefacts_count().await.unwrap(), 3);
    assert_eq!(
        DefinitionAdl2Service::archetypes_count(&svc).await.unwrap(),
        1
    );
    assert_eq!(svc.templates_count().await.unwrap(), 1);
    assert_eq!(DefinitionAdl2Service::opts_count(&svc).await.unwrap(), 1);

    // list_artefacts = all HRIDs; per-kind lists partition them.
    let all = svc.list_artefacts(Page::all()).await.unwrap();
    assert_eq!(all.len(), 3);
    assert!(all.contains(&ADL2_OPT_HRID.to_owned()));
    assert_eq!(
        DefinitionAdl2Service::list_archetypes(&svc, Page::all())
            .await
            .unwrap(),
        vec![ADL2_ARCH_HRID.to_owned()]
    );
    assert_eq!(
        svc.list_templates(Page::all()).await.unwrap(),
        vec![ADL2_TMPL_HRID.to_owned()]
    );
    assert_eq!(
        DefinitionAdl2Service::list_opts(&svc, Page::all())
            .await
            .unwrap(),
        vec![ADL2_OPT_HRID.to_owned()]
    );

    // list_matching_artefacts (regex on the HRID).
    let obs = svc
        .list_matching_artefacts("OBSERVATION".to_owned(), Page::all())
        .await
        .unwrap();
    assert_eq!(obs, vec![ADL2_ARCH_HRID.to_owned()]);

    // upload_artefact replaces if the HRID already exists (spec: "replace it").
    let replacement = format!("{adl2_opt}-- replaced\n");
    svc.upload_artefact(replacement.clone())
        .await
        .expect("replace");
    assert_eq!(
        svc.artefacts_count().await.unwrap(),
        3,
        "replace, not insert"
    );
    assert_eq!(
        svc.get_artefact(ADL2_OPT_HRID.to_owned()).await.unwrap(),
        replacement
    );

    // delete_artefact → Post: gone.
    svc.delete_artefact(ADL2_OPT_HRID.to_owned())
        .await
        .expect("delete");
    assert!(!svc.has_artefact(ADL2_OPT_HRID.to_owned()).await.unwrap());
    assert_eq!(svc.artefacts_count().await.unwrap(), 2);
}

#[tokio::test]
async fn adl2_errors() {
    let pg = Pg::start().await;
    let svc = EhrbaseService::new(pg.migrated_pool("def_adl2_err").await);

    // Invalid ADL2 (unrecognised header) → 422 invalid_artefact.
    let bad = svc
        .upload_artefact("concept\nopenEHR-EHR-OBSERVATION.bp.v1.0.0".to_owned())
        .await
        .expect_err("invalid artefact rejected");
    assert!(
        matches!(
            bad,
            SmError {
                status: CallStatusType::ContentInvalid,
                ..
            }
        ),
        "got {bad:?}"
    );

    // Recognised header but a malformed HRID → 422.
    let bad_hrid = svc
        .upload_artefact("archetype\nnot-an-hrid".to_owned())
        .await
        .expect_err("malformed hrid rejected");
    assert!(
        matches!(
            bad_hrid,
            SmError {
                status: CallStatusType::ContentInvalid,
                ..
            }
        ),
        "got {bad_hrid:?}"
    );

    // get / delete of an absent artefact → 404.
    let missing = svc
        .get_artefact("openEHR-EHR-OBSERVATION.absent.v1.0.0".to_owned())
        .await
        .expect_err("absent artefact");
    assert!(
        matches!(
            missing,
            SmError {
                status: CallStatusType::VersionedObjectDoesNotExist,
                ..
            }
        ),
        "got {missing:?}"
    );
    let del_missing = svc
        .delete_artefact("openEHR-EHR-OBSERVATION.absent.v1.0.0".to_owned())
        .await
        .expect_err("delete absent");
    assert!(
        matches!(
            del_missing,
            SmError {
                status: CallStatusType::VersionedObjectDoesNotExist,
                ..
            }
        ),
        "got {del_missing:?}"
    );

    // Uncompilable regex → 400 invalid_id_pattern.
    let bad_re = svc
        .list_matching_artefacts("(".to_owned(), Page::all())
        .await
        .expect_err("invalid pattern");
    assert!(
        matches!(
            bad_re,
            SmError {
                status: CallStatusType::PreconditionViolation,
                ..
            }
        ),
        "got {bad_re:?}"
    );
}

// ── registered queries (I_DEFINITION_QUERY) ──────────────────────────────────

#[tokio::test]
#[allow(clippy::too_many_lines)] // one full I_DEFINITION_QUERY lifecycle on one container
async fn query_valid_store_list_match_delete() {
    let pg = Pg::start().await;
    let svc = EhrbaseService::new(pg.migrated_pool("def_query").await);
    let good = "SELECT c FROM COMPOSITION c";

    // valid_query: formalism equivalence + parse.
    assert!(
        svc.valid_query(good.to_owned(), "aql".to_owned())
            .await
            .unwrap()
    );
    assert!(
        svc.valid_query(good.to_owned(), "AQL::1".to_owned())
            .await
            .unwrap()
    );
    assert!(
        !svc.valid_query(good.to_owned(), "cql".to_owned())
            .await
            .unwrap()
    );
    assert!(
        !svc.valid_query("this is not aql".to_owned(), "AQL".to_owned())
            .await
            .unwrap()
    );
    assert_eq!(svc.queries_count().await.unwrap(), 0);

    // store_query with no name → a generated `misc::q_<uuid>` name.
    let desc = svc
        .store_query(good.to_owned(), "aql".to_owned(), None)
        .await
        .expect("store generated");
    assert!(
        desc.qualified_query_name.starts_with("misc::q_"),
        "generated name: {}",
        desc.qualified_query_name
    );
    assert_eq!(desc.formalism, "aql");
    assert_eq!(desc.source.as_deref(), Some(good));
    assert!(desc.version.is_some());
    // Post: the generated query is registered.
    assert!(
        svc.has_query(desc.qualified_query_name.clone())
            .await
            .unwrap()
    );

    // store_query with an explicit qualified name.
    let named = svc
        .store_query(
            good.to_owned(),
            "AQL".to_owned(),
            Some("ehr::over_50".to_owned()),
        )
        .await
        .expect("store named");
    assert_eq!(named.qualified_query_name, "ehr::over_50");
    assert!(svc.has_query("ehr::over_50".to_owned()).await.unwrap());
    // A bare name gets the "misc" namespace applied.
    let _ = svc
        .store_query(
            good.to_owned(),
            "aql".to_owned(),
            Some("bare_name".to_owned()),
        )
        .await
        .expect("store bare");
    assert!(svc.has_query("bare_name".to_owned()).await.unwrap());
    assert!(svc.has_query("misc::bare_name".to_owned()).await.unwrap());

    // queries_count = distinct qualified names (3 stored).
    assert_eq!(svc.queries_count().await.unwrap(), 3);

    // list_queries has all three.
    let all = svc.list_queries(Page::all()).await.unwrap();
    assert_eq!(all.len(), 3);

    // list_matching_queries: id regex on the qualified name.
    let ehr_only = svc
        .list_matching_queries("^ehr::".to_owned(), None, Page::all())
        .await
        .unwrap();
    assert_eq!(ehr_only.len(), 1);
    assert_eq!(ehr_only[0].qualified_query_name, "ehr::over_50");
    // artefact pattern scans the source: `COMPOSITION` matches all.
    let by_artefact = svc
        .list_matching_queries(".*".to_owned(), Some("COMPOSITION".to_owned()), Page::all())
        .await
        .unwrap();
    assert_eq!(by_artefact.len(), 3);
    let by_artefact_none = svc
        .list_matching_queries(".*".to_owned(), Some("OBSERVATION".to_owned()), Page::all())
        .await
        .unwrap();
    assert!(by_artefact_none.is_empty());

    // delete_query removes by name → Post_query_deleted.
    svc.delete_query("ehr::over_50".to_owned())
        .await
        .expect("delete");
    assert!(!svc.has_query("ehr::over_50".to_owned()).await.unwrap());
    assert_eq!(svc.queries_count().await.unwrap(), 2);
    // Deleting an absent query → 404.
    let del_missing = svc
        .delete_query("ehr::over_50".to_owned())
        .await
        .expect_err("delete absent");
    assert!(
        matches!(
            del_missing,
            SmError {
                status: CallStatusType::VersionedObjectDoesNotExist,
                ..
            }
        ),
        "got {del_missing:?}"
    );

    // store_query on invalid AQL → 422 invalid_query.
    let bad = svc
        .store_query("nonsense".to_owned(), "aql".to_owned(), None)
        .await
        .expect_err("invalid query rejected");
    assert!(
        matches!(
            bad,
            SmError {
                status: CallStatusType::ContentInvalid,
                ..
            }
        ),
        "got {bad:?}"
    );

    // Bad id-pattern regex → 400.
    let bad_re = svc
        .list_matching_queries("(".to_owned(), None, Page::all())
        .await
        .expect_err("bad regex");
    assert!(
        matches!(
            bad_re,
            SmError {
                status: CallStatusType::PreconditionViolation,
                ..
            }
        ),
        "got {bad_re:?}"
    );
}

#[tokio::test]
async fn query_store_set_not_implemented() {
    let pg = Pg::start().await;
    let svc = EhrbaseService::new(pg.migrated_pool("def_query_set").await);
    // store_query_set is a spec TODO → 501 (trait default, PORT NOTE).
    let err = svc
        .store_query_set(None)
        .await
        .expect_err("not implemented");
    assert!(
        matches!(
            err,
            SmError {
                status: CallStatusType::NotImplemented,
                ..
            }
        ),
        "got {err:?}"
    );
}

/// The REST adapter surface for ADL2 template upload: a duplicate HRID is a
/// conflict (`409_template_already_exists`, `definition-codegen.openapi.yaml`
/// `/definition/template/adl2` POST) and invalid source is a 400 — while the
/// SM-native `upload_artefact` keeps replace semantics (SM master04
/// `i_definition_adl2.adoc`: "replace it").
#[tokio::test]
async fn adl2_template_upload_wire_conflicts_on_duplicate() {
    let pg = Pg::start().await;
    let svc = EhrbaseService::new(pg.migrated_pool("def_adl2_wire").await);

    let tmpl = adl2_source("template", "openEHR-EHR-COMPOSITION.t_wire.v1.0.0", None);
    let hrid = svc
        .template_adl2_upload(tmpl.clone())
        .await
        .expect("first upload");
    assert_eq!(hrid, "openEHR-EHR-COMPOSITION.t_wire.v1.0.0");

    let dup = svc
        .template_adl2_upload(tmpl.clone())
        .await
        .expect_err("duplicate template id conflicts on the wire surface");
    assert!(dup.message.contains("already exists"), "got {dup:?}");

    // The SM-native upload still replaces.
    svc.upload_artefact(tmpl).await.expect("native replace");

    // Invalid source → 400-class precondition (STCNT et al.), not 422.
    let bad = svc
        .template_adl2_upload(
            "template (adl_version=2.0.6)\nopenEHR-EHR-COMPOSITION.t_bad.v1\n".to_owned(),
        )
        .await
        .expect_err("invalid source rejected");
    assert!(
        matches!(bad.status, CallStatusType::PreconditionViolation),
        "got {bad:?}"
    );
}
