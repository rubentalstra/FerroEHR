// SPDX-FileCopyrightText: FerroEHR contributors
// SPDX-License-Identifier: MIT

//! End-to-end service tests for the SM-6 Subject Proxy Service
//! (`I_SUBJECT_PROXY_SERVICE` + `I_DATA_BINDING`) against a real `PostgreSQL` 18
//! (shared testkit harness).
//!
//! Spec: SM `docs/specs/openehr/SM/docs/openehr_platform/master10-subject_proxy_service.adoc`
//! and its `UML/classes/*.adoc`.
//!
//! Coverage: register subject / variable / binding + data set; bind and pull a
//! variable through the **openEHR frame** (AQL executed via the Query service)
//! against committed data; the `has_subject`/`has_binding`/`not has_*`
//! precondition rejections; the FHIR/HL7v2 stubbed-seam typed rejection; and
//! `reset()`.

#![expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "clippy's in-test lint scoping (clippy.toml `allow-*-in-tests`) only \
              reaches `#[test]`-annotated functions, so it misses this integration \
              module's helpers and async bodies; panicking assertions and direct \
              fixture indexing are the intended shape here (the Rust Book ch11)"
)]

use serde_json::{Value, json};
use sqlx::PgPool;

use std::sync::Arc;

use ferroehr::service::FerroEhrService;
use ferroehr::service::status::CallStatusType;
use ferroehr::service::subject_proxy::config::{SpFhirSystem, SubjectProxyConfig};

use ferroehr::service::subject_proxy::binding::{
    DataFrame, EnvBinding, SystemCall, SystemCallBody,
};
use ferroehr::service::subject_proxy::data_set::SubjectDataSet;
use ferroehr::service::subject_proxy::sample::FramePayload;
use ferroehr::service::subject_proxy::value::VariableValue;
use ferroehr::service::subject_proxy::variable::SubjectVariable;
use ferroehr::service::version_update::{change_type_coded, lifecycle_state_coded};
use openehr_its::rest::generated::common::{UpdateAudit, UpdateAuditData, UpdateVersion};
use openehr_rm::prelude::PartyProxy;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

// ── committed-data helpers (mirror service_aql.rs) ───────────────────────────

fn uv<T: serde::de::DeserializeOwned>(data: &Value) -> UpdateVersion<T> {
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
                &json!({ "_type": "PARTY_IDENTIFIED", "name": "sps tester" }),
            )
            .expect("committer"),
        }),
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
    let db = testkit::db().await.expect("testkit database");
    let svc = FerroEhrService::new(db.pool());

    // Committed data: an EHR with one COMPOSITION named "vitals".
    let ehr = svc.create_ehr(None).await.expect("ehr");
    let subject = ehr.to_string();
    svc.create_composition(ehr, uv(&composition("vitals")))
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
    let db = testkit::db().await.expect("testkit database");
    let svc = FerroEhrService::new(db.pool());

    let ehr = svc.create_ehr(None).await.expect("ehr");
    let subject = ehr.to_string();
    svc.create_composition(ehr, uv(&composition("vitals")))
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
    let db = testkit::db().await.expect("testkit database");
    let svc = FerroEhrService::new(db.pool());

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
    let db = testkit::db().await.expect("testkit database");
    let svc = FerroEhrService::new(db.pool());

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

// ── FHIR frame executor (config-gated) ──────────────────────────────────────
//
// `i_data_binding.adoc` (`API_CALL`/`fhir_get`) + `hl7_fhir_sample.adoc`; a
// hermetic `wiremock` FHIR server stands in for the remote system (no network).

/// A subject-proxy service wired to a single FHIR system `name` → `base_url`.
///
/// NOTE: the deadlines are deliberately far longer than any loopback exchange
/// needs, because no test in this file asserts timeout *behaviour* and a missed
/// deadline is silent rather than loud: a retrieve that fails after dispatch is
/// an unavailable `SAMPLE` (`SM/docs/UML/classes/sample.adoc` — "Every retrieval
/// attempt will generate a new Sample object, regardless of whether data was
/// actually available or not"), which `extract_value` maps to the empty
/// `VARIABLE_VALUE`. Deadlines that must actually fire belong in a test that
/// delays the response on purpose.
fn service_with_fhir(pool: PgPool, name: &str, base_url: &str) -> FerroEhrService {
    let mut systems = std::collections::BTreeMap::new();
    systems.insert(
        name.to_owned(),
        SpFhirSystem {
            base_url: base_url.to_owned(),
            connect_timeout_ms: 30_000,
            request_timeout_ms: 30_000,
        },
    );
    let fhir = SubjectProxyConfig { systems }
        .build()
        .expect("build fhir executor")
        .expect("some executor");
    FerroEhrService::new(pool).with_subject_proxy(Arc::new(fhir))
}

fn fhir_frame(id: &str, system_id: &str, query_text: &str) -> DataFrame {
    DataFrame {
        id: id.to_owned(),
        model_type: "hl7-fhir".to_owned(),
        primary_method: Some(SystemCall::Api(SystemCallBody {
            system_id: Some(system_id.to_owned()),
            call_name: Some("fhir_get".to_owned()),
            query_text: Some(query_text.to_owned()),
            ..SystemCallBody::default()
        })),
        fallback_method: None,
    }
}

fn fhir_variable(name: &str, frame_id: &str, frame_path: &str, type_name: &str) -> SubjectVariable {
    SubjectVariable {
        namespace: None,
        name: name.to_owned(),
        type_name: type_name.to_owned(),
        currency: None,
        ask_user: None,
        is_manual: false,
        frame_id: frame_id.to_owned(),
        frame_path: frame_path.to_owned(),
        history: Vec::new(),
        last_frame: None,
    }
}

/// A configured FHIR system serves the resource; `get_variable` retrieves it and
/// extracts the declared value via the `frame_path` JSON pointer.
#[tokio::test]
async fn subject_proxy_fhir_frame_extracts_via_json_pointer() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/Patient/patient-123"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "resourceType": "Patient",
            "id": "patient-123",
            "birthDate": "1980-02-29",
            "meta": { "lastUpdated": "2020-01-01T09:00:00Z" }
        })))
        .expect(1)
        .mount(&server)
        .await;

    let db = testkit::db().await.expect("testkit database");
    let svc = service_with_fhir(db.pool(), "pas", &server.uri());

    svc.register_binding(EnvBinding {
        env_id: "prod".to_owned(),
        description: None,
        data_frames: vec![fhir_frame(
            "fhir::demographics",
            "pas",
            "Patient/$subject_id",
        )],
    })
    .await
    .expect("register_binding");
    svc.register_subject("patient-123".to_owned(), None)
        .await
        .expect("register_subject");
    svc.add_subject_variable(
        "patient-123".to_owned(),
        fhir_variable("dob", "fhir::demographics", "/birthDate", "Date"),
    )
    .await
    .expect("add_subject_variable");

    let value = svc
        .get_variable("patient-123".to_owned(), "dob".to_owned())
        .await
        .expect("get_variable");
    assert_eq!(
        value,
        VariableValue::Single {
            value: Some(json!("1980-02-29"))
        },
        "the FHIR frame retrieves the Patient and extracts birthDate via JSON pointer"
    );
}

/// A frame targeting a `system_id` that matches no configured system is a typed
/// rejection (fail-closed) — never an arbitrary outbound request.
#[tokio::test]
async fn subject_proxy_fhir_unknown_system_is_typed_rejection() {
    let server = MockServer::start().await;
    let db = testkit::db().await.expect("testkit database");
    // Only "pas" is configured; the frame targets "other".
    let svc = service_with_fhir(db.pool(), "pas", &server.uri());
    svc.register_binding(EnvBinding {
        env_id: "prod".to_owned(),
        description: None,
        data_frames: vec![fhir_frame("fhir::x", "other", "Patient/$subject_id")],
    })
    .await
    .expect("register_binding");

    let err = svc
        .get_frame("patient-123".to_owned(), "fhir::x".to_owned())
        .await
        .expect_err("unknown FHIR system");
    assert_eq!(
        err.status,
        CallStatusType::NotImplemented,
        "unknown system → typed rejection: {err:?}"
    );
}

/// A `500` from the primary FHIR retrieve is an unavailable sample, so the
/// `fallback_method` runs and its result wins (`data_frame.adoc`).
#[tokio::test]
async fn subject_proxy_fhir_500_runs_fallback() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/Patient/patient-500"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/Observation/patient-500"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "resourceType": "Observation", "id": "obs-1"
        })))
        .mount(&server)
        .await;

    let db = testkit::db().await.expect("testkit database");
    let svc = service_with_fhir(db.pool(), "pas", &server.uri());

    let frame = DataFrame {
        id: "fhir::with_fallback".to_owned(),
        model_type: "hl7-fhir".to_owned(),
        primary_method: Some(SystemCall::Api(SystemCallBody {
            system_id: Some("pas".to_owned()),
            call_name: Some("fhir_get".to_owned()),
            query_text: Some("Patient/$subject_id".to_owned()),
            ..SystemCallBody::default()
        })),
        fallback_method: Some(SystemCall::Api(SystemCallBody {
            system_id: Some("pas".to_owned()),
            call_name: Some("fhir_get".to_owned()),
            query_text: Some("Observation/$subject_id".to_owned()),
            ..SystemCallBody::default()
        })),
    };
    svc.register_binding(EnvBinding {
        env_id: "prod".to_owned(),
        description: None,
        data_frames: vec![frame],
    })
    .await
    .expect("register_binding");

    let sample = svc
        .get_frame("patient-500".to_owned(), "fhir::with_fallback".to_owned())
        .await
        .expect("get_frame");
    assert!(
        !sample.is_unavailable,
        "the fallback produced an available sample after the primary 500: {sample:?}"
    );
    match sample.result {
        Some(FramePayload::Hl7Fhir { resource }) => assert_eq!(
            resource.pointer("/resourceType").and_then(Value::as_str),
            Some("Observation"),
            "the fallback resource wins"
        ),
        other => panic!("expected an HL7_FHIR_SAMPLE fallback payload, got {other:?}"),
    }
}
