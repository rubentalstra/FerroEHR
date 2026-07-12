//! End-to-end service tests for the SM-6 Subject Proxy Service
//! (`I_SUBJECT_PROXY_SERVICE` + `I_DATA_BINDING`) against a real `PostgreSQL` 18
//! (testcontainers).
//!
//! Spec: SM `docs/specs/openehr/SM/docs/openehr_platform/master10-subject_proxy_service.adoc`
//! and its `UML/classes/*.adoc`; design
//! `docs/design/sm-platform/08-target-architecture.md` §4.4.
//!
//! Coverage: register subject / variable / binding + data set; bind and pull a
//! variable through the **openEHR frame** (AQL executed via the Query service)
//! against committed data; the `has_subject`/`has_binding`/`not has_*`
//! precondition rejections; the FHIR/HL7v2 stubbed-seam typed rejection; and
//! `reset()`.
#![allow(clippy::expect_used, clippy::unwrap_used)]

use serde_json::{Value, json};
use sqlx::{AssertSqlSafe, Connection, PgConnection, PgPool};
use testcontainers::runners::AsyncRunner;
use testcontainers::{ContainerAsync, ImageExt};
use testcontainers_modules::postgres::Postgres;

use ehrbase::db::{self, DbSettings};
use ehrbase::service::EhrbaseService;
use ehrbase_sm::{UpdateAudit, UpdateVersion};
use ehrbase_sm::{
    CallStatusType, DataBinding, DataFrame, EhrCompositionService, EhrService, EnvBinding,
    SubjectDataSet, SubjectProxyService, SubjectVariable, SystemCall, SystemCallBody,
    VariableValue,
};
use openehr_base::prelude::TerminologyCode;
use openehr_rm::prelude::PartyProxy;

struct Pg {
    #[allow(dead_code)]
    container: ContainerAsync<Postgres>,
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
            container,
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
        let settings = DbSettings::new(format!(
            "postgres://postgres:postgres@{}:{}/{name}",
            self.host, self.port
        ));
        let pool = db::connect(&settings).await.expect("pool");
        db::run_migrations(&pool).await.expect("migrate");
        pool
    }
}

// ── committed-data helpers (mirror service_aql.rs) ───────────────────────────

fn term(code: &str) -> TerminologyCode {
    TerminologyCode {
        terminology_id: "openehr".to_owned(),
        terminology_version: None,
        code_string: code.to_owned(),
        uri: None,
    }
}

fn uv(data: Value) -> UpdateVersion {
    UpdateVersion {
        preceding_version_uid: None,
        lifecycle_state: term("532"),
        attestations: None,
        data,
        audit: UpdateAudit {
            change_type: term("249"),
            description: None,
            committer: serde_json::from_value::<PartyProxy>(
                json!({ "_type": "PARTY_IDENTIFIED", "name": "sps tester" }),
            )
            .expect("committer"),
            system_id: None,
        },
        signature: None,
    }
}

/// A minimal committed COMPOSITION named `name` (template stripped so only
/// RM-invariant + terminology validation runs).
fn composition(name: &str) -> Value {
    let path = format!(
        "{}/../../crates/openehr-its/tests/vendor/openehr_sdk/composition/canonical_json/minimal_observation.json",
        env!("CARGO_MANIFEST_DIR")
    );
    let mut c: Value = serde_json::from_str(&std::fs::read_to_string(&path).expect("read fixture"))
        .expect("parse fixture");
    if let Some(details) = c
        .get_mut("archetype_details")
        .and_then(Value::as_object_mut)
    {
        details.remove("template_id");
    }
    if let Some(obj) = c.as_object_mut() {
        obj.remove("uid");
    }
    c["name"] = json!({ "_type": "DV_TEXT", "value": name });
    c["content"][0]["archetype_details"] = json!({
        "_type": "ARCHETYPED",
        "archetype_id": { "_type": "ARCHETYPE_ID", "value": "openEHR-EHR-OBSERVATION.minimal.v1" },
        "rm_version": "1.1.0",
    });
    c
}

/// The openEHR frame binding for `env`: an AQL `QUERY_CALL` projecting the
/// composition name (scoped to the subject's EHR by the executor).
fn openehr_frame(id: &str) -> DataFrame {
    DataFrame {
        id: id.to_owned(),
        model_type: "openehr".to_owned(),
        primary_method: Some(SystemCall::Query(SystemCallBody {
            call_name: Some("aql_query".to_owned()),
            query_text: Some(
                "SELECT c/name/value AS comp_name FROM EHR e CONTAINS COMPOSITION c".to_owned(),
            ),
            ..SystemCallBody::default()
        })),
        fallback_method: None,
    }
}

fn variable(name: &str, frame_id: &str) -> SubjectVariable {
    SubjectVariable {
        namespace: None,
        name: name.to_owned(),
        type_name: "String".to_owned(),
        currency: None,
        ask_user: None,
        is_manual: false,
        frame_id: frame_id.to_owned(),
        frame_path: "comp_name".to_owned(),
        history: Vec::new(),
        last_frame: None,
    }
}

/// Happy path: register a binding + subject + variable, commit a composition,
/// then pull the variable through the openEHR frame (AQL via the Query service).
#[tokio::test]
async fn subject_proxy_pulls_variable_through_openehr_frame() {
    let pg = Pg::start().await;
    let svc = EhrbaseService::new(pg.migrated_pool("sps_openehr_frame").await);

    // Committed data: an EHR with one COMPOSITION named "vitals".
    let ehr = svc.create_ehr(None).await.expect("ehr");
    let subject = ehr.to_string();
    svc.create_composition(ehr, uv(composition("vitals")))
        .await
        .expect("commit composition");

    // Register the environment binding (openEHR frame) and the subject proxy.
    svc.register_binding(EnvBinding {
        env_id: "prod".to_owned(),
        description: Some("test env".to_owned()),
        data_frames: vec![openehr_frame("openehr::comps")],
    })
    .await
    .expect("register_binding");
    assert!(
        svc.has_binding("prod".to_owned())
            .await
            .expect("has_binding")
    );

    svc.register_subject(subject.clone(), Some("individual".to_owned()))
        .await
        .expect("register_subject");
    assert!(svc.has_subject(subject.clone()).await.expect("has_subject"));

    svc.add_subject_variable(subject.clone(), variable("latest_comp", "openehr::comps"))
        .await
        .expect("add_subject_variable");

    // get_variable_defs: "name: Type".
    let defs = svc
        .get_variable_defs(subject.clone())
        .await
        .expect("get_variable_defs");
    assert_eq!(defs, vec!["latest_comp: String".to_owned()]);

    // Pull the value through the frame: the committed composition's name.
    let value = svc
        .get_variable(subject.clone(), "latest_comp".to_owned())
        .await
        .expect("get_variable");
    assert_eq!(
        value,
        VariableValue::Single {
            value: Some(json!("vitals"))
        },
        "the openEHR frame extracts the committed composition name"
    );
}

/// A data set registered by an application resolves through `get_data_set`, and
/// `remove_application` drops it (with `has_application` gating).
#[tokio::test]
async fn subject_proxy_application_data_set_round_trip() {
    let pg = Pg::start().await;
    let svc = EhrbaseService::new(pg.migrated_pool("sps_data_set").await);

    let ehr = svc.create_ehr(None).await.expect("ehr");
    let subject = ehr.to_string();
    svc.create_composition(ehr, uv(composition("vitals")))
        .await
        .expect("commit composition");

    svc.register_binding(EnvBinding {
        env_id: "prod".to_owned(),
        description: None,
        data_frames: vec![openehr_frame("openehr::comps")],
    })
    .await
    .expect("register_binding");
    svc.register_subject(subject.clone(), None)
        .await
        .expect("register_subject");

    // Register an application data set with a data-set-local alias ("dob"-style)
    // for the canonical variable.
    let mut variables = std::collections::BTreeMap::new();
    variables.insert("comp".to_owned(), variable("latest_comp", "openehr::comps"));
    svc.register_application_data_set(SubjectDataSet {
        id: "antenatal.v1".to_owned(),
        subject_id: subject.clone(),
        creating_app_id: Some("task_planning".to_owned()),
        using_app_ids: vec![],
        variables,
    })
    .await
    .expect("register_application_data_set");

    // has_application is true for the registering app; the derived canonical
    // variable was created (register_application_data_set create-if-absent).
    assert!(
        svc.has_application("task_planning".to_owned())
            .await
            .expect("has_application")
    );
    let defs = svc
        .get_variable_defs(subject.clone())
        .await
        .expect("get_variable_defs");
    assert_eq!(defs, vec!["latest_comp: String".to_owned()]);

    // get_data_set resolves every variable to a VARIABLE_SAMPLE.
    let result = svc
        .get_data_set(subject.clone(), "antenatal.v1".to_owned())
        .await
        .expect("get_data_set");
    assert_eq!(result.name, "antenatal.v1");
    assert_eq!(result.subject_id, subject);
    assert_eq!(result.variables.len(), 1);
    let sample = &result.variables[0];
    assert!(!sample.is_unavailable);
    assert_eq!(
        sample.result,
        Some(VariableValue::Single {
            value: Some(json!("vitals"))
        })
    );

    // remove_application drops the data set across subjects.
    svc.remove_application("task_planning".to_owned())
        .await
        .expect("remove_application");
    assert!(
        !svc.has_application("task_planning".to_owned())
            .await
            .expect("has_application after remove")
    );
    let err = svc
        .get_data_set(subject.clone(), "antenatal.v1".to_owned())
        .await
        .expect_err("data set removed");
    assert_eq!(err.status, CallStatusType::PreconditionViolation);
}

/// A FHIR (non-openEHR) frame is a stubbed seam: `get_frame` rejects it
/// `NotImplemented` (the "FHIR/HL7v2 frame seams stubbed as typed rejections").
#[tokio::test]
async fn subject_proxy_fhir_frame_is_typed_rejection() {
    let pg = Pg::start().await;
    let svc = EhrbaseService::new(pg.migrated_pool("sps_fhir_stub").await);

    svc.register_binding(EnvBinding {
        env_id: "prod".to_owned(),
        description: None,
        data_frames: vec![DataFrame {
            id: "fhir::demographics".to_owned(),
            model_type: "hl7-fhir".to_owned(),
            primary_method: Some(SystemCall::Api(SystemCallBody {
                system_id: Some("ehr1.nhs.org.uk".to_owned()),
                call_name: Some("fhir_get".to_owned()),
                ..SystemCallBody::default()
            })),
            fallback_method: None,
        }],
    })
    .await
    .expect("register_binding");

    let err = svc
        .get_frame("some-subject".to_owned(), "fhir::demographics".to_owned())
        .await
        .expect_err("a FHIR frame is not implemented");
    assert_eq!(
        err.status,
        CallStatusType::NotImplemented,
        "FHIR frame → not_implemented: {err:?}"
    );

    // An unknown frame is a precondition violation, not a 500.
    let err = svc
        .get_frame("some-subject".to_owned(), "no::such_frame".to_owned())
        .await
        .expect_err("unknown frame");
    assert_eq!(err.status, CallStatusType::PreconditionViolation);
}

/// The SM pre-conditions become typed rejections: duplicate subject
/// (`not has_subject`), unknown subject (`has_subject`), duplicate binding
/// (`not has_binding`); and `reset()` returns the service to virgin state.
#[tokio::test]
async fn subject_proxy_preconditions_and_reset() {
    let pg = Pg::start().await;
    let svc = EhrbaseService::new(pg.migrated_pool("sps_preconditions").await);

    // Unknown subject: add_subject_variable / get_variable both reject.
    let err = svc
        .add_subject_variable("ghost".to_owned(), variable("v", "f"))
        .await
        .expect_err("add to unknown subject");
    assert_eq!(err.status, CallStatusType::PreconditionViolation);
    let err = svc
        .get_variable("ghost".to_owned(), "v".to_owned())
        .await
        .expect_err("get from unknown subject");
    assert_eq!(err.status, CallStatusType::PreconditionViolation);

    // register_subject twice: `not has_subject` fails the second time.
    svc.register_subject("s1".to_owned(), None)
        .await
        .expect("first register_subject");
    let err = svc
        .register_subject("s1".to_owned(), None)
        .await
        .expect_err("duplicate subject");
    assert_eq!(err.status, CallStatusType::PreconditionViolation);

    // register_binding twice for the same env: `not has_binding` fails.
    svc.register_binding(EnvBinding {
        env_id: "prod".to_owned(),
        description: None,
        data_frames: vec![],
    })
    .await
    .expect("first register_binding");
    let err = svc
        .register_binding(EnvBinding {
            env_id: "prod".to_owned(),
            description: None,
            data_frames: vec![],
        })
        .await
        .expect_err("duplicate binding");
    assert_eq!(err.status, CallStatusType::PreconditionViolation);

    // add_binding_frame requires the binding to exist.
    let err = svc
        .add_binding_frame("no_env".to_owned(), openehr_frame("f1"))
        .await
        .expect_err("frame for unknown binding");
    assert_eq!(err.status, CallStatusType::PreconditionViolation);

    // reset() → virgin state: no subject, no binding.
    svc.reset().await.expect("reset");
    assert!(!svc.has_subject("s1".to_owned()).await.expect("has_subject"));
    assert!(
        !svc.has_binding("prod".to_owned())
            .await
            .expect("has_binding")
    );
}
