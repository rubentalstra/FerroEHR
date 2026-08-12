// SPDX-FileCopyrightText: FerroEHR contributors
// SPDX-License-Identifier: MIT

//! End-to-end exercise of the performance measurement machinery against a
//! local stub SUT: the open-loop driver seeds the scale corpus + the
//! standing ward through the API surface, drives the case's journey
//! workload, and produces a re-checkable measurement whose class verdict
//! re-derives from the embedded HDR histograms — including the
//! falsifiability direction (a faulting SUT can never earn the class).

#![expect(
    clippy::unwrap_used,
    reason = "test-support helpers (not `#[test]` fns, so the clippy.toml in-tests scoping does not reach them) are panic-idiomatic: a broken stub must abort the test loudly, Book ch11"
)]

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use cnf_runner::ixit::{Environment, Ixit};
use cnf_runner::perf::{ClassVerdict, JourneyCatalogue, PerformanceCase};
use cnf_runner::perf_run::client::{PerfClient, PerfPrincipals};
use cnf_runner::perf_run::corpus::{SeededCorpus, seed_scale_ladder, seed_ward};
use cnf_runner::perf_run::pack::{AuxPayloads, FlatPayload, JourneyPack, PackTemplate, TddPayload};
use cnf_runner::perf_run::window::{drive_case, rederive_verdict};

/// A minimal keep-alive HTTP stub realizing the journey wire shapes: OPT
/// upload 201, EHR create 201+Location, EHR/status/directory/contribution
/// reads 200, composition commit/contribution 201+ETag/Location, versioned
/// update 200+ETag, delete 204, queries/templates/tags 200. When
/// `fail_every_nth_read` is non-zero, that fraction of composition reads
/// returns 500 (the falsifiability lever).
fn spawn_stub(fail_every_nth_read: u64) -> (String, std::thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let reads = Arc::new(AtomicU64::new(0));
    let ehr_counter = Arc::new(AtomicU64::new(0));
    let uid_counter = Arc::new(AtomicU64::new(0));
    let handle = std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(stream) = stream else { break };
            let reads = Arc::clone(&reads);
            let ehr_counter = Arc::clone(&ehr_counter);
            let uid_counter = Arc::clone(&uid_counter);
            std::thread::spawn(move || {
                serve(
                    stream,
                    &reads,
                    &ehr_counter,
                    &uid_counter,
                    fail_every_nth_read,
                );
            });
        }
    });
    (format!("http://{addr}/base/v1"), handle)
}

fn serve(
    stream: TcpStream,
    reads: &AtomicU64,
    ehr_counter: &AtomicU64,
    uid_counter: &AtomicU64,
    fail_every_nth_read: u64,
) {
    let mut reader = BufReader::new(stream.try_clone().unwrap());
    let mut stream = stream;
    loop {
        let mut request_line = String::new();
        if reader.read_line(&mut request_line).unwrap_or(0) == 0 {
            return; // connection closed
        }
        let mut content_length = 0usize;
        let mut principal = String::new();
        loop {
            let mut header = String::new();
            if reader.read_line(&mut header).unwrap_or(0) == 0 {
                return;
            }
            if header == "\r\n" {
                break;
            }
            let lower = header.to_ascii_lowercase();
            if let Some(v) = lower.strip_prefix("content-length:") {
                content_length = v.trim().parse().unwrap_or(0);
            }
            if let Some(v) = lower.strip_prefix("x-cnf-stub-principal:") {
                v.trim().clone_into(&mut principal);
            }
        }
        let mut body = vec![0u8; content_length];
        if content_length > 0 && reader.read_exact(&mut body).is_err() {
            return;
        }

        let mut parts = request_line.split_whitespace();
        let method = parts.next().unwrap_or("");
        let path = parts.next().unwrap_or("");
        let (status, extra, body) = route(
            method,
            path,
            &principal,
            reads,
            ehr_counter,
            uid_counter,
            fail_every_nth_read,
        );
        let response = format!(
            "HTTP/1.1 {status}\r\n{extra}Content-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}",
            body.len(),
        );
        if stream.write_all(response.as_bytes()).is_err() {
            return;
        }
    }
}

#[expect(clippy::too_many_lines, reason = "one arm per stubbed wire shape")]
fn route(
    method: &str,
    path: &str,
    principal: &str,
    reads: &AtomicU64,
    ehr_counter: &AtomicU64,
    uid_counter: &AtomicU64,
    fail_every_nth_read: u64,
) -> (&'static str, String, String) {
    let fresh_uid = |counter: &AtomicU64| {
        let n = counter.fetch_add(1, Ordering::Relaxed);
        format!("uid-{n}::stub::1")
    };
    // The boundary principals: a credential-less caller is refused outright,
    // and a read-only caller is refused any write — the DENY branches the
    // access-control probe measures.
    match principal {
        "unauthenticated" => return ("401 Unauthorized", String::new(), "{}".to_owned()),
        "readonly" if method != "GET" => {
            return ("403 Forbidden", String::new(), "{}".to_owned());
        }
        _ => {}
    }
    match (method, path) {
        ("OPTIONS", _) => (
            "200 OK",
            "Allow: GET, OPTIONS\r\n".to_owned(),
            "{}".to_owned(),
        ),
        ("GET", p) if p.ends_with("/.well-known/smart-configuration") => {
            ("200 OK", String::new(), "{}".to_owned())
        }
        ("GET", p) if p.ends_with("/definition/template/adl2") => {
            ("200 OK", String::new(), "[]".to_owned())
        }
        ("POST", p) if p.ends_with("/demographic/person") => (
            "201 Created",
            format!("ETag: W/\"{}\"\r\n", fresh_uid(uid_counter)),
            String::new(),
        ),
        ("PUT", p) if p.contains("/demographic/person/") => (
            "200 OK",
            format!("ETag: W/\"{}\"\r\n", fresh_uid(uid_counter)),
            String::new(),
        ),
        ("GET", p) if p.contains("/demographic/person/") => {
            ("200 OK", String::new(), "{\"_type\":\"PERSON\"}".to_owned())
        }
        ("POST", p) if p.ends_with("/demographic/party_relationship") => (
            "201 Created",
            format!("ETag: W/\"{}\"\r\n", fresh_uid(uid_counter)),
            String::new(),
        ),
        ("GET", p) if p.contains("/demographic/party_relationship/") => (
            "200 OK",
            String::new(),
            "{\"_type\":\"PARTY_RELATIONSHIP\"}".to_owned(),
        ),
        ("POST", p) if p.ends_with("/definition/template/adl1.4") => {
            ("201 Created", String::new(), String::new())
        }
        ("GET", p) if p.ends_with("/definition/template/adl1.4") => {
            ("200 OK", String::new(), "[]".to_owned())
        }
        ("GET", p) if p.contains("/definition/template/adl1.4/") => {
            ("200 OK", String::new(), "{}".to_owned())
        }
        ("PUT", p) if p.contains("/definition/query/") => {
            ("200 OK", String::new(), "{}".to_owned())
        }
        ("POST", p) if p.ends_with("/ehr") => {
            let n = ehr_counter.fetch_add(1, Ordering::Relaxed);
            (
                "201 Created",
                format!("Location: http://stub/base/v1/ehr/ehr-{n}\r\n"),
                String::new(),
            )
        }
        ("GET", p) if p.contains("/ehr_status") => (
            "200 OK",
            format!("ETag: \"{}\"\r\n", fresh_uid(uid_counter)),
            "{\"_type\":\"EHR_STATUS\"}".to_owned(),
        ),
        ("PUT", p) if p.contains("/ehr_status") => (
            "200 OK",
            format!("ETag: \"{}\"\r\n", fresh_uid(uid_counter)),
            String::new(),
        ),
        ("POST", p) if p.ends_with("/directory") => (
            "201 Created",
            format!("ETag: \"{}\"\r\n", fresh_uid(uid_counter)),
            String::new(),
        ),
        ("GET", p) if p.ends_with("/directory") => (
            "200 OK",
            format!("ETag: \"{}\"\r\n", fresh_uid(uid_counter)),
            "{\"_type\":\"FOLDER\"}".to_owned(),
        ),
        ("PUT", p) if p.ends_with("/directory") => (
            "200 OK",
            format!("ETag: \"{}\"\r\n", fresh_uid(uid_counter)),
            String::new(),
        ),
        ("POST", p) if p.ends_with("/contribution") => {
            let n = uid_counter.fetch_add(1, Ordering::Relaxed);
            (
                "201 Created",
                format!("Location: http://stub/base/v1/contribution/contrib-{n}\r\n"),
                String::new(),
            )
        }
        ("GET", p) if p.contains("/contribution/") => (
            "200 OK",
            String::new(),
            "{\"_type\":\"CONTRIBUTION\"}".to_owned(),
        ),
        ("POST", p) if p.ends_with("/composition") => (
            "201 Created",
            format!("ETag: W/\"{}\"\r\n", fresh_uid(uid_counter)),
            String::new(),
        ),
        // The MESSAGE extension pair (register AMB-34): a TDD import commits
        // and names the created version; a whole-EHR export reads a list.
        ("POST", p) if p.contains("/message/tdd/") => (
            "201 Created",
            String::new(),
            format!("{{\"uid\":\"{}\"}}", fresh_uid(uid_counter)),
        ),
        ("GET", p) if p.contains("/message/export/") => (
            "200 OK",
            String::new(),
            "[{\"_type\":\"EXTRACT\"}]".to_owned(),
        ),
        ("PUT", p) if p.contains("/composition/") && p.ends_with("/tags") => {
            ("200 OK", String::new(), "[]".to_owned())
        }
        ("GET", p) if p.contains("/composition/") && p.ends_with("/tags") => {
            ("200 OK", String::new(), "[]".to_owned())
        }
        ("DELETE", p) if p.contains("/composition/") => {
            ("204 No Content", String::new(), String::new())
        }
        ("PUT", p) if p.contains("/composition/") => (
            "200 OK",
            format!("ETag: \"{}\"\r\n", fresh_uid(uid_counter)),
            String::new(),
        ),
        ("GET", p) if p.contains("/versioned_composition/") => {
            ("200 OK", String::new(), "{}".to_owned())
        }
        ("GET", p) if p.contains("/composition/") => {
            let n = reads.fetch_add(1, Ordering::Relaxed) + 1;
            if fail_every_nth_read != 0 && n.is_multiple_of(fail_every_nth_read) {
                ("500 Internal Server Error", String::new(), "{}".to_owned())
            } else {
                (
                    "200 OK",
                    String::new(),
                    "{\"_type\":\"COMPOSITION\"}".to_owned(),
                )
            }
        }
        ("GET", p) if p.contains("/ehr/") => {
            ("200 OK", String::new(), "{\"_type\":\"EHR\"}".to_owned())
        }
        ("POST", p) if p.ends_with("/query/aql") => {
            ("200 OK", String::new(), "{\"rows\":[]}".to_owned())
        }
        ("GET", p) if p.contains("/query/") => {
            ("200 OK", String::new(), "{\"rows\":[]}".to_owned())
        }
        _ => ("404 Not Found", String::new(), String::new()),
    }
}

/// A compact journey catalogue exercising every dependency shape: standing
/// ward reads + versioned updates, a fresh-EHR admission chain, an
/// order→result pipeline with an in-window dependent stage, the governance
/// surface, the demographic chain, the Simplified-Format channel, the
/// definition/platform probes, and the two access-control DENY branches —
/// i.e. every operation of the closed vocabulary that drives a wire.
fn catalogue() -> JourneyCatalogue {
    serde_saphyr::from_str(
        "chart_review:\n  description: d\n  derivation: g\n  stages:\n    - { op: composition_read, at: PT0S }\n    - { op: composition_revision_history, at: PT1S }\n    - { op: adhoc_query, at: PT2S }\n    - { op: directory_read, at: PT3S }\nadmission:\n  description: d\n  derivation: g\n  stages:\n    - { op: ehr_create, at: PT0S }\n    - { op: ehr_read, at: PT1S }\n    - { op: ehr_status_read, at: PT2S }\n    - { op: ehr_status_update, at: PT3S }\n    - { op: composition_commit, template: cnf.ckm.gp_data_set, at: PT4S }\n    - { op: directory_create, at: PT5S }\nlab_pipeline:\n  description: d\n  derivation: g\n  stages:\n    - { op: composition_commit, template: cnf.ckm.gp_data_set, at: PT0S }\n    - { op: contribution_commit, template: cnf.ckm.lab_result, at: { uniform: [PT2S, PT3S] } }\n    - { op: composition_read_current, at: PT4S }\n    - { op: contribution_read, at: PT5S }\ncorrection:\n  description: d\n  derivation: g\n  stages:\n    - { op: composition_read_current, at: PT0S }\n    - { op: composition_update, template: cnf.ckm.gp_data_set, at: PT1S }\nward_dashboard:\n  description: d\n  derivation: g\n  stages:\n    - { op: ward_query, at: PT0S }\n    - { op: stored_query_execute, at: PT1S }\n    - { op: template_list, at: PT2S }\n    - { op: template_get, at: PT3S }\n    - { op: tags_put, at: PT4S }\n    - { op: tags_read, at: PT5S }\ndemographic_admission:\n  description: d\n  derivation: g\n  stages:\n    - { op: party_create, at: PT0S }\n    - { op: party_read, at: PT1S }\n    - { op: party_relationship_create, at: PT2S }\n    - { op: party_relationship_read, at: PT3S }\n    - { op: party_update, at: PT4S }\nplatform_surface:\n  description: d\n  derivation: g\n  stages:\n    - { op: system_options, at: PT0S }\n    - { op: smart_configuration_read, at: PT1S }\n    - { op: template_example, at: PT2S }\n    - { op: template_adl2_list, at: PT3S }\n    - { op: analytics_query, at: PT4S }\n    - { op: terminology_query, at: PT5S }\nsimplified_formats_exchange:\n  description: d\n  derivation: g\n  stages:\n    - { op: composition_commit_flat, at: PT0S }\n    - { op: composition_read_flat, at: PT1S }\n    - { op: composition_version_read, at: PT2S }\naccess_control_probe:\n  description: d\n  derivation: g\n  stages:\n    - { op: unauthenticated_probe, at: PT0S }\n    - { op: readonly_write_denied, template: cnf.ckm.gp_data_set, at: PT1S }\n",
    )
    .unwrap()
}

fn journey_pack() -> JourneyPack {
    let skeleton = serde_json::json!({
        "_type": "COMPOSITION",
        "context": { "_type": "EVENT_CONTEXT",
                     "start_time": { "_type": "DV_DATE_TIME", "value": "2020-01-01T00:00:00Z" } },
        "composer": { "_type": "PARTY_IDENTIFIED", "name": "seed" }
    });
    let template = |key: &str, id: &str| PackTemplate {
        key: key.to_owned(),
        template_id: id.to_owned(),
        opt_xml: "<template/>".to_owned(),
        skeleton: skeleton.clone(),
    };
    JourneyPack {
        templates: vec![
            template("cnf.ckm.gp_data_set", "GP data set"),
            template(
                "cnf.ckm.lab_result",
                "Generic lab test result example simple",
            ),
            template("cnf.ckm.medicines_list", "Medicines list item R1"),
        ],
        aux: AuxPayloads {
            flat: Some(FlatPayload {
                template_id: "minimal_action.en.v1".to_owned(),
                opt_xml: "<template/>".to_owned(),
                body: serde_json::json!({ "ctx/language": "en" }),
            }),
            person: Some(serde_json::json!({
                "_type": "PERSON",
                "identities": [{ "_type": "PARTY_IDENTITY",
                    "details": { "_type": "ITEM_TREE", "items": [{ "_type": "ELEMENT",
                        "value": { "_type": "DV_TEXT", "value": "Person One" } }] } }]
            })),
            person_amended: Some(serde_json::json!({
                "_type": "PERSON",
                "identities": [{ "_type": "PARTY_IDENTITY",
                    "details": { "_type": "ITEM_TREE", "items": [{ "_type": "ELEMENT",
                        "value": { "_type": "DV_TEXT", "value": "Person Two" } }] } }]
            })),
            party_relationship: Some(serde_json::json!({
                "_type": "PARTY_RELATIONSHIP",
                "source": { "_type": "PARTY_REF",
                    "id": { "_type": "HIER_OBJECT_ID", "value": "placeholder" } }
            })),
            tdd: Some(TddPayload {
                opt_xml: "<template/>".to_owned(),
                document: "<Nested xmlns=\"http://schemas.oceanehr.com/templates\" \
                            template_id=\"nested.en.v1\"/>"
                    .to_owned(),
            }),
        },
    }
}

fn poc_case() -> PerformanceCase {
    serde_saphyr::from_str(
        "id: PERF-hospital_sim-class_POC\nkind: performance\ncomponent: PERFORMANCE\ndescription: d\ntest_purpose: t\nspec_refs: [\"CNF 2.0 performance schedule\"]\nclass: POC\ncorpus: cnf.scale.10k\nworkload:\n  arrival_rate: 60/s\n  warmup: PT5M\n  duration: PT1H\n  journeys: { chart_review: 78%, admission: 3%, lab_pipeline: 2%, correction: 2%, ward_dashboard: 5%, demographic_admission: 3%, platform_surface: 3%, simplified_formats_exchange: 2%, access_control_probe: 2% }\nthresholds:\n  - { metric: latency_p99, max: 1000 }\n  - { metric: error_rate, max: 0 }\n  - { metric: offered_load_sustained, min: 2 }\n",
    )
    .unwrap()
}

fn client_and_env(base_url: &str) -> (PerfPrincipals, Environment) {
    let ixit: Ixit = serde_json::from_value(serde_json::json!({
        "instances": {
            "sut": { "base_url": base_url, "auth": { "mode": "none" } },
            // The boundary principals the access-control probe addresses. The
            // stub answers 401/403 on their marker header, so the DENY
            // branches are measured exactly as a real deployment's are.
            "unauthenticated": { "base_url": base_url, "auth": { "mode": "none" },
                                  "headers": { "x-cnf-stub-principal": "unauthenticated" } },
            "readonly": { "base_url": base_url, "auth": { "mode": "none" },
                           "headers": { "x-cnf-stub-principal": "readonly" } },
            "smart_platform": { "base_url": base_url, "auth": { "mode": "none" } }
        },
        "smart": {
            "platform_instance": "smart_platform",
            "mint": { "issuer": "https://as.stub", "subject": "stub",
                       "key_file": "unused.pem", "kid": "stub", "ttl_seconds": 300 }
        },
        "environment": { "exclusive_server": true, "hardware_class": "test-stub",
                          "cores": 1, "memory_gb": 1, "storage_class": "ram",
                          "topology": "in-process stub" }
    }))
    .unwrap();
    let principals = PerfPrincipals::from_ixit(&ixit).unwrap();
    let environment = ixit.environment.clone().unwrap();
    (principals, environment)
}

fn seeded(client: &PerfClient, pack: &JourneyPack) -> SeededCorpus {
    let progress = |_message: String| {};
    let mut corpus =
        seed_scale_ladder(client, "cnf.scale.10k", "<opt/>", 6, 3, 4, &progress).unwrap();
    assert_eq!(
        corpus.ehr_ids.len(),
        6,
        "scale ladder seeds one EHR per requested slot"
    );
    assert_eq!(
        corpus.compositions.len(),
        18,
        "scale ladder seeds versions_per_ehr compositions per EHR"
    );
    seed_ward(client, &mut corpus, pack, 4, &progress).unwrap();
    assert_eq!(
        corpus.ward.len(),
        6,
        "the standing ward covers every seeded EHR"
    );
    corpus
}

#[test]
fn the_open_loop_journey_run_earns_the_class_on_a_healthy_sut() {
    let (base_url, _server) = spawn_stub(0);
    let (principals, environment) = client_and_env(&base_url);
    let progress = |_message: String| {};
    let pack = journey_pack();
    let catalogue = catalogue();
    let corpus = seeded(principals.primary(), &pack);

    let case = poc_case();
    let measurement = drive_case(
        &case,
        &principals,
        &corpus,
        &pack,
        &catalogue,
        &environment,
        1,
        8,
        &progress,
    )
    .unwrap();

    // The schedule dispatched the planned aggregate operation rate.
    assert!(
        measurement.offered_load_sustained >= 57.0,
        "offered load {} below the schedule rate",
        measurement.offered_load_sustained
    );
    // The full journey surface was measured: every catalogue operation
    // label appears, with zero errors, and every histogram re-checks.
    let labels: Vec<&str> = measurement
        .operations
        .iter()
        .map(|o| o.operation.as_str())
        .collect();
    for expected in [
        "ehr_create",
        "ehr_read",
        "ehr_status_read",
        "ehr_status_update",
        "composition_commit",
        "composition_read",
        "composition_read_current",
        "composition_revision_history",
        "composition_update",
        "directory_create",
        "directory_read",
        "contribution_commit",
        "contribution_read",
        "adhoc_query",
        "ward_query",
        "stored_query_execute",
        "template_list",
        "template_get",
        "tags_put",
        "tags_read",
        "party_create",
        "party_read",
        "party_update",
        "party_relationship_create",
        "party_relationship_read",
        "system_options",
        "smart_configuration_read",
        "template_example",
        "template_adl2_list",
        "analytics_query",
        "terminology_query",
        "composition_commit_flat",
        "composition_read_flat",
        "composition_version_read",
        "unauthenticated_probe",
        "readonly_write_denied",
    ] {
        assert!(labels.contains(&expected), "operation {expected} missing");
    }
    for op in &measurement.operations {
        assert_eq!(op.errors, 0, "{} saw errors", op.operation);
        let histogram = op.decode_histogram().unwrap();
        assert_eq!(histogram.len(), op.requests);
    }
    assert_eq!(measurement.verdict, ClassVerdict::Earned);
    assert!(measurement.violations.is_empty());
    assert_eq!(measurement.environment.hardware_class, "test-stub");

    // The verdict pipeline's re-derivation agrees with the stored verdict.
    let (rederived, violations) = rederive_verdict(&case, &measurement).unwrap();
    assert_eq!(rederived, ClassVerdict::Earned);
    assert!(violations.is_empty());
}

#[test]
fn a_faulting_sut_cannot_earn_the_class() {
    let (base_url, _server) = spawn_stub(5); // every 5th composition read is a 500
    let (principals, environment) = client_and_env(&base_url);
    let progress = |_message: String| {};
    let pack = journey_pack();
    let catalogue = catalogue();
    let corpus = seeded(principals.primary(), &pack);

    let case = poc_case();
    let measurement = drive_case(
        &case,
        &principals,
        &corpus,
        &pack,
        &catalogue,
        &environment,
        0,
        8,
        &progress,
    )
    .unwrap();

    let errors: u64 = measurement.operations.iter().map(|o| o.errors).sum();
    assert!(errors > 0, "the faulting stub produced no errors");
    assert_eq!(measurement.verdict, ClassVerdict::NotEarned);
    assert!(
        measurement
            .violations
            .iter()
            .any(|v| v.contains("error_rate")),
        "violations {:?} name no error_rate breach",
        measurement.violations
    );
}
