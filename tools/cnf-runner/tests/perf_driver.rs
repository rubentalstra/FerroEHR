//! End-to-end exercise of the performance measurement machinery against a
//! local stub SUT: the open-loop driver seeds the scale corpus through the
//! API surface, drives the case's mix, and produces a re-checkable
//! measurement whose class verdict re-derives from the embedded HDR
//! histograms — including the falsifiability direction (a faulting SUT can
//! never earn the class).
#![allow(clippy::unwrap_used, clippy::panic)] // test assertions/fixtures

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use cnf_runner::ixit::{Environment, Ixit};
use cnf_runner::perf::{ClassVerdict, PerformanceCase};
use cnf_runner::perf_run::{PerfClient, drive_case, rederive_verdict, seed_scale_ladder};

/// A minimal keep-alive HTTP stub realizing the four bound wire shapes:
/// OPT upload 201, EHR create 201+Location, composition commit 201+ETag,
/// composition read 200, ad-hoc query 200. When `fail_every_nth_read` is
/// non-zero, that fraction of reads returns 500 (the falsifiability lever).
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
        loop {
            let mut header = String::new();
            if reader.read_line(&mut header).unwrap_or(0) == 0 {
                return;
            }
            if header == "\r\n" {
                break;
            }
            if let Some(v) = header.to_ascii_lowercase().strip_prefix("content-length:") {
                content_length = v.trim().parse().unwrap_or(0);
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

fn route(
    method: &str,
    path: &str,
    reads: &AtomicU64,
    ehr_counter: &AtomicU64,
    uid_counter: &AtomicU64,
    fail_every_nth_read: u64,
) -> (&'static str, String, String) {
    match (method, path) {
        ("POST", p) if p.ends_with("/definition/template/adl1.4") => {
            ("201 Created", String::new(), String::new())
        }
        ("POST", p) if p.ends_with("/ehr") => {
            let n = ehr_counter.fetch_add(1, Ordering::Relaxed);
            (
                "201 Created",
                format!("Location: http://stub/base/v1/ehr/ehr-{n}\r\n"),
                String::new(),
            )
        }
        ("POST", p) if p.ends_with("/composition") => {
            let n = uid_counter.fetch_add(1, Ordering::Relaxed);
            (
                "201 Created",
                format!("ETag: W/\"uid-{n}::stub::1\"\r\n"),
                String::new(),
            )
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
        ("POST", p) if p.ends_with("/query/aql") => {
            ("200 OK", String::new(), "{\"rows\":[]}".to_owned())
        }
        _ => ("404 Not Found", String::new(), String::new()),
    }
}

fn poc_case() -> PerformanceCase {
    serde_saphyr::from_str(
        "id: PERF-mixed_load-class_POC\nkind: performance\ncomponent: PERFORMANCE\ndescription: d\ntest_purpose: t\nspec_refs: [\"CNF 2.0 performance schedule\"]\nclass: POC\ncorpus: cnf.scale.10k\nworkload:\n  arrival_rate: 20/s\n  warmup: PT5M\n  duration: PT1H\n  mix: { composition_read: 61%, adhoc_query: 30%, composition_commit: 8%, ehr_create: 1% }\nthresholds:\n  - { metric: latency_p99, operation: composition_read, max: 1000 }\n  - { metric: latency_p99, operation: composition_commit, max: 1000 }\n  - { metric: error_rate, max: 0 }\n  - { metric: offered_load_sustained, min: 2 }\n",
    )
    .unwrap()
}

fn client_and_env(base_url: &str) -> (PerfClient, Environment) {
    let ixit: Ixit = serde_json::from_value(serde_json::json!({
        "instances": { "sut": { "base_url": base_url, "auth": { "mode": "none" } } },
        "environment": { "exclusive_server": true, "hardware_class": "test-stub",
                          "cores": 1, "memory_gb": 1, "storage_class": "ram",
                          "topology": "in-process stub" }
    }))
    .unwrap();
    let client = PerfClient::from_instance(ixit.default_instance().unwrap()).unwrap();
    let environment = ixit.environment.clone().unwrap();
    (client, environment)
}

#[test]
fn the_open_loop_run_earns_the_class_on_a_healthy_sut() {
    let (base_url, _server) = spawn_stub(0);
    let (client, environment) = client_and_env(&base_url);
    let progress = |_message: String| {};

    let corpus = seed_scale_ladder(&client, "cnf.scale.10k", "<opt/>", 6, 3, 4, &progress).unwrap();
    assert_eq!(corpus.ehr_ids.len(), 6);
    assert_eq!(corpus.compositions.len(), 18);

    let case = poc_case();
    let measurement = drive_case(&case, &client, &corpus, &environment, 1, 5, &progress).unwrap();

    // The schedule dispatched every planned arrival at the planned rate.
    assert!(
        measurement.offered_load_sustained >= 19.0,
        "offered load {} below the schedule rate",
        measurement.offered_load_sustained
    );
    // Every mix operation was measured; the histograms re-check.
    assert_eq!(measurement.operations.len(), 4);
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
    let (base_url, _server) = spawn_stub(5); // every 5th read is a 500
    let (client, environment) = client_and_env(&base_url);
    let progress = |_message: String| {};

    let corpus = seed_scale_ladder(&client, "cnf.scale.10k", "<opt/>", 4, 2, 2, &progress).unwrap();
    let case = poc_case();
    let measurement = drive_case(&case, &client, &corpus, &environment, 0, 5, &progress).unwrap();

    let read = measurement
        .operations
        .iter()
        .find(|o| o.operation == "composition_read")
        .unwrap();
    assert!(read.errors > 0, "the faulting stub produced no errors");
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
