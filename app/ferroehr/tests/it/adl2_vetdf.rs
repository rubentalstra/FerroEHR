// SPDX-FileCopyrightText: FerroEHR contributors
// SPDX-License-Identifier: MIT

//! ADL2 VETDF validation over the terminology-service resolver seam, end to
//! end (AM ADL2 `master03-archetype_package.adoc` §Validity Rules).
//!
//! An uploaded archetype whose external SNOMED CT term binding the (mocked)
//! FHIR terminology server does not know (`CodeSystem/$lookup` → `404`) is
//! rejected `422` with the VETDF rule code; one the server knows (`200`) is
//! accepted. The terminology backend is a hermetic `wiremock` FHIR R4B server —
//! no live network. Real `PostgreSQL` 18 via the shared testkit harness.

#![expect(
    clippy::expect_used,
    reason = "clippy's in-test lint scoping (clippy.toml `allow-*-in-tests`) only \
              reaches `#[test]`-annotated functions, so it misses this integration \
              module's helpers and async bodies; panicking assertions and direct \
              fixture indexing are the intended shape here (the Rust Book ch11)"
)]

use std::sync::Arc;

use ferroehr::service::FerroEhrService;
use ferroehr::service::status::CallStatusType;
use ferroehr::service::terminology::config::{FhirOperation, FhirProviderConfig, ProviderKind};
use ferroehr::service::terminology::fhir::FhirTerminologyProvider;
use serde_json::json;
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

const HRID: &str = "openEHR-EHR-OBSERVATION.vetdf.v1.0.0";
const SNOMED_ID: &str = "SNOMED-CT";
const SNOMED_TARGET: &str = "http://snomedct.info/id/271649006";

/// A minimal spec-valid ADL2 archetype (the same shape the passing
/// `service_definition` upload test uses) whose root node `id1` carries an
/// external SNOMED CT term binding — the VETDF subject.
fn archetype_with_external_binding() -> String {
    "\
archetype (adl_version=2.0.6; rm_release=1.1.0)
    openEHR-EHR-OBSERVATION.vetdf.v1.0.0

language
    original_language = <[ISO_639-1::en]>

description
    lifecycle_state = <\"published\">
    details = <
        [\"en\"] = <
            language = <[ISO_639-1::en]>
        >
    >

definition
    OBSERVATION[id1] matches { *}

terminology
    term_definitions = <
        [\"en\"] = <
            [\"id1\"] = <text = <\"Root\"> description = <\"Root.\">>
        >
    >
    term_bindings = <
        [\"SNOMED-CT\"] = <
            [\"id1\"] = <http://snomedct.info/id/271649006>
        >
    >
"
    .to_owned()
}

/// Build a provider pointing at `base` with the response cache off (each test
/// asserts a fresh remote answer).
fn provider(base: &str) -> FhirTerminologyProvider {
    let cfg = FhirProviderConfig {
        kind: ProviderKind::Fhir,
        url: base.to_owned(),
        // has_term uses `CodeSystem/$lookup` regardless of the membership op.
        operation: FhirOperation::ValidateCode,
        connect_timeout_ms: 500,
        request_timeout_ms: 800,
        oauth2_client: None,
        client_cert_path: None,
        client_key_path: None,
        ca_bundle_path: None,
        cache_ttl_secs: 0,
        cache_capacity: 0,
    };
    FhirTerminologyProvider::new("test", &cfg).expect("build provider")
}

#[tokio::test]
async fn unknown_external_binding_is_rejected_with_vetdf() {
    let server = MockServer::start().await;
    // The terminology server does not know the bound SNOMED CT code → 404.
    Mock::given(method("GET"))
        .and(path("/CodeSystem/$lookup"))
        .and(query_param("system", SNOMED_ID))
        .and(query_param("code", SNOMED_TARGET))
        .respond_with(ResponseTemplate::new(404))
        .mount(&server)
        .await;

    let db = testkit::db().await.expect("testkit database");
    let svc = FerroEhrService::new(db.pool())
        .with_external_terminology(Arc::new(provider(&server.uri())));

    let err = svc
        .upload_artefact(archetype_with_external_binding())
        .await
        .expect_err("an unknown external term binding must be rejected");
    assert_eq!(
        err.status,
        CallStatusType::ContentInvalid,
        "VETDF is a content-invalid (422) failure"
    );
    assert!(
        err.message.contains("VETDF"),
        "the rejection carries the VETDF rule code: {}",
        err.message
    );
    assert!(
        !svc.has_artefact(HRID.to_owned()).await.unwrap(),
        "a rejected artefact is not stored"
    );
}

#[tokio::test]
async fn known_external_binding_is_accepted() {
    let server = MockServer::start().await;
    // The terminology server knows the bound code → 200 (a $lookup resolves).
    Mock::given(method("GET"))
        .and(path("/CodeSystem/$lookup"))
        .and(query_param("system", SNOMED_ID))
        .and(query_param("code", SNOMED_TARGET))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "resourceType": "Parameters",
            "parameter": [{"name": "display", "valueString": "Entire lung"}]
        })))
        .mount(&server)
        .await;

    let db = testkit::db().await.expect("testkit database");
    let svc = FerroEhrService::new(db.pool())
        .with_external_terminology(Arc::new(provider(&server.uri())));

    svc.upload_artefact(archetype_with_external_binding())
        .await
        .expect("a known external term binding is accepted");
    assert!(
        svc.has_artefact(HRID.to_owned()).await.unwrap(),
        "the accepted artefact is stored"
    );
}
