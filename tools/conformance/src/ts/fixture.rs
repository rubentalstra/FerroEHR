//! A `wiremock`-backed FHIR R4 terminology-server fixture (B4,
//! `docs/design/terminology-server-integration.md` §5.1): a hermetic FHIR-tx
//! server the runner spins up with canned `$expand`/`$validate-code`/`$lookup`/
//! `$subsumes` responses and on-demand fault injection (timeout, `5xx`,
//! malformed body). No network, no container — a pure in-process
//! `127.0.0.1` mock, so it gates every CI run.
//!
//! The canned resources mirror the reference golden the CDR's own FHIR client
//! tests use (`docs/terminology-validation.md` §5, `ValueSet/surface`,
//! code `B` = Buccal), so a deployment that points its SUT at this fixture
//! (a fixed-port compose wiring via `host.docker.internal`) exercises the same
//! contract the `FhirTerminologyProvider` unit tests assert.
//!
//! What this fixture is *for*: (1) proving the FHIR-tx wire contract shape in
//! `nextest` without any SUT (the tests below), (2) serving as the reference
//! server the real-server mode (`--tx-server-url`) is contrasted against, and
//! (3) recording the exchange (received requests) into the conformance report.

use std::time::Duration;

use serde_json::{Value, json};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use crate::reporting::results::TxExchange;

/// The canned value set the fixture expands (`ValueSet/$expand`): the reference
/// `surface` value set (codes `B`/`L`/`O`).
pub const SURFACE_VS: &str = "http://hl7.org/fhir/ValueSet/surface";
/// The code system backing [`SURFACE_VS`].
pub const SURFACE_SYS: &str = "http://hl7.org/fhir/surface";
/// A member code of [`SURFACE_VS`] (`B` = Buccal).
pub const SURFACE_MEMBER: &str = "B";

/// A fault a fixture injects on **every** terminology operation, so a SUT (or a
/// direct client) driving it sees the corresponding failure mode (the
/// `FhirTerminologyProvider` maps each to `CallStatusType::Exception` → HTTP
/// `500`, `docs/terminology-validation.md` §5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fault {
    /// A response delayed well past any sane client timeout.
    Timeout,
    /// An HTTP `503 Service Unavailable`.
    ServerError,
    /// A `200` whose body is not FHIR JSON.
    Malformed,
}

impl Fault {
    /// A stable label for the report / skip reasons.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Fault::Timeout => "timeout",
            Fault::ServerError => "5xx",
            Fault::Malformed => "malformed",
        }
    }

    /// The `wiremock` response this fault serves.
    fn response(self) -> ResponseTemplate {
        match self {
            // 30s dwarfs any client request timeout (the CDR provider defaults
            // to 10s, the tests use sub-second timeouts).
            Fault::Timeout => ResponseTemplate::new(200)
                .set_delay(Duration::from_secs(30))
                .set_body_json(ok_validate_code()),
            Fault::ServerError => ResponseTemplate::new(503),
            Fault::Malformed => ResponseTemplate::new(200).set_body_string("this is not FHIR json"),
        }
    }
}

/// The four FHIR terminology operation paths this fixture serves.
const OPS: [&str; 4] = [
    "/ValueSet/$expand",
    "/ValueSet/$validate-code",
    "/CodeSystem/$lookup",
    "/CodeSystem/$subsumes",
];

/// A canned `ValueSet` with a two-member expansion (`B`/`L`, one nested `O`).
fn ok_expand() -> Value {
    json!({
        "resourceType": "ValueSet",
        "expansion": { "contains": [
            {"system": SURFACE_SYS, "code": "B", "display": "Buccal"},
            {"system": SURFACE_SYS, "code": "L", "display": "Lingual",
             "contains": [{"system": SURFACE_SYS, "code": "O", "display": "Occlusal"}]}
        ]}
    })
}

/// A canned `$validate-code` `Parameters` (`result = true`, `display = Buccal`).
fn ok_validate_code() -> Value {
    json!({
        "resourceType": "Parameters",
        "parameter": [
            {"name": "result", "valueBoolean": true},
            {"name": "display", "valueString": "Buccal"}
        ]
    })
}

/// A canned `$lookup` `Parameters` (`display = Buccal`).
fn ok_lookup() -> Value {
    json!({
        "resourceType": "Parameters",
        "parameter": [
            {"name": "name", "valueString": "surface"},
            {"name": "display", "valueString": "Buccal"}
        ]
    })
}

/// A canned `$subsumes` `Parameters` (`outcome = subsumes`).
fn ok_subsumes() -> Value {
    json!({
        "resourceType": "Parameters",
        "parameter": [{"name": "outcome", "valueCode": "subsumes"}]
    })
}

/// The canned happy-path response for a FHIR operation path.
fn ok_response(op: &str) -> ResponseTemplate {
    let body = match op {
        "/ValueSet/$expand" => ok_expand(),
        "/ValueSet/$validate-code" => ok_validate_code(),
        "/CodeSystem/$lookup" => ok_lookup(),
        "/CodeSystem/$subsumes" => ok_subsumes(),
        _ => json!({"resourceType": "OperationOutcome"}),
    };
    ResponseTemplate::new(200).set_body_json(body)
}

/// A boxed error for the fixture's self-check (a URL-parse or transport fault).
type BoxError = Box<dyn std::error::Error + Send + Sync>;

/// Build `{base}{op}?k=v&…` with percent-encoded query values (this build of
/// `reqwest` does not expose `RequestBuilder::query`, so URLs are assembled the
/// same way the CDR's FHIR client does — `app/ehrbase/src/terminology/fhir.rs`).
fn build_url(base: &str, op: &str, query: &[(&str, &str)]) -> Result<reqwest::Url, BoxError> {
    let mut url = reqwest::Url::parse(&format!("{base}{op}"))?;
    {
        let mut pairs = url.query_pairs_mut();
        for (k, v) in query {
            pairs.append_pair(k, v);
        }
    }
    Ok(url)
}

/// A `wiremock`-backed FHIR R4 terminology-server fixture.
#[derive(Debug)]
pub struct FhirTxFixture {
    server: MockServer,
}

impl FhirTxFixture {
    /// Start a fixture serving the canned happy-path responses on a random
    /// `127.0.0.1` port.
    pub async fn start_canned() -> Self {
        let server = MockServer::start().await;
        for op in OPS {
            Mock::given(method("GET"))
                .and(path(op))
                .respond_with(ok_response(op))
                .mount(&server)
                .await;
        }
        Self { server }
    }

    /// Start a fixture that injects `fault` on every terminology operation.
    pub async fn start_fault(fault: Fault) -> Self {
        let server = MockServer::start().await;
        for op in OPS {
            Mock::given(method("GET"))
                .and(path(op))
                .respond_with(fault.response())
                .mount(&server)
                .await;
        }
        Self { server }
    }

    /// The fixture's FHIR base URL (e.g. `http://127.0.0.1:53412`); the
    /// terminology operations hang directly off it (`{base}/ValueSet/$expand`).
    #[must_use]
    pub fn base_url(&self) -> String {
        self.server.uri()
    }

    /// The exchange the fixture has served so far — every received request as a
    /// [`TxExchange`], in receipt order (the record written to the report).
    pub async fn exchanges(&self) -> Vec<TxExchange> {
        self.server
            .received_requests()
            .await
            .unwrap_or_default()
            .into_iter()
            .map(|r| TxExchange {
                method: r.method.to_string(),
                path: r.url.path().to_owned(),
                query: r.url.query().map(str::to_owned),
            })
            .collect()
    }

    /// Self-verify liveness by issuing the four canned operations against the
    /// fixture, returning the resulting recorded exchange. Proves the fixture
    /// answers (so the recorded exchange in the report is concrete even when no
    /// SUT is wired to it) and is used by the runner as its fixture health check.
    ///
    /// # Errors
    /// Returns an error if a URL cannot be built, the fixture cannot be reached,
    /// or an operation does not answer `2xx`.
    pub async fn self_check(&self) -> Result<Vec<TxExchange>, BoxError> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(2))
            .build()?;
        let base = self.base_url();
        for (op, query) in [
            ("/ValueSet/$expand", &[("url", SURFACE_VS)][..]),
            (
                "/ValueSet/$validate-code",
                &[("url", SURFACE_VS), ("code", SURFACE_MEMBER)][..],
            ),
            ("/CodeSystem/$lookup", &[("code", SURFACE_MEMBER)][..]),
            (
                "/CodeSystem/$subsumes",
                &[("codeA", "L"), ("codeB", "O")][..],
            ),
        ] {
            let url = build_url(&base, op, query)?;
            client
                .get(url)
                .header("accept", "application/fhir+json")
                .send()
                .await?
                .error_for_status()?;
        }
        Ok(self.exchanges().await)
    }
}

#[cfg(test)]
#[allow(
    clippy::panic,
    clippy::print_stdout,
    clippy::print_stderr,
    let_underscore_drop
)] // test assertions/diagnostics/fixtures
mod tests {
    use super::*;

    /// A short-timeout FHIR client mirroring the CDR provider's timeouts, so the
    /// timeout fault is observable quickly.
    fn client(ms: u64) -> reqwest::Client {
        reqwest::Client::builder()
            .timeout(Duration::from_millis(ms))
            .build()
            .expect("build client")
    }

    /// GET `{base}{op}?query` against the fixture.
    async fn fetch(
        c: &reqwest::Client,
        base: &str,
        op: &str,
        query: &[(&str, &str)],
    ) -> reqwest::Response {
        c.get(build_url(base, op, query).expect("url"))
            .send()
            .await
            .expect("send")
    }

    #[tokio::test]
    async fn canned_expand_returns_the_surface_value_set() {
        let fx = FhirTxFixture::start_canned().await;
        let body: Value = fetch(
            &client(2_000),
            &fx.base_url(),
            "/ValueSet/$expand",
            &[("url", SURFACE_VS)],
        )
        .await
        .json()
        .await
        .expect("json");
        assert_eq!(body["resourceType"], "ValueSet");
        let contains = body["expansion"]["contains"].as_array().expect("contains");
        // Top-level B and L (O is nested under L).
        assert!(contains.iter().any(|c| c["code"] == "B"));
        assert!(contains.iter().any(|c| c["code"] == "L"));
    }

    #[tokio::test]
    async fn canned_validate_code_and_lookup_and_subsumes() {
        let fx = FhirTxFixture::start_canned().await;
        let c = client(2_000);
        let base = fx.base_url();

        let vc: Value = fetch(
            &c,
            &base,
            "/ValueSet/$validate-code",
            &[("url", SURFACE_VS), ("code", SURFACE_MEMBER)],
        )
        .await
        .json()
        .await
        .expect("json");
        assert!(
            vc["parameter"]
                .as_array()
                .expect("params")
                .iter()
                .any(|p| p["name"] == "result" && p["valueBoolean"] == true)
        );

        let lk: Value = fetch(
            &c,
            &base,
            "/CodeSystem/$lookup",
            &[("code", SURFACE_MEMBER)],
        )
        .await
        .json()
        .await
        .expect("json");
        assert!(
            lk["parameter"]
                .as_array()
                .expect("params")
                .iter()
                .any(|p| p["name"] == "display" && p["valueString"] == "Buccal")
        );

        let sub: Value = fetch(&c, &base, "/CodeSystem/$subsumes", &[])
            .await
            .json()
            .await
            .expect("json");
        assert!(
            sub["parameter"]
                .as_array()
                .expect("params")
                .iter()
                .any(|p| p["name"] == "outcome" && p["valueCode"] == "subsumes")
        );
    }

    #[tokio::test]
    async fn fault_server_error_is_5xx() {
        let fx = FhirTxFixture::start_fault(Fault::ServerError).await;
        let resp = fetch(
            &client(2_000),
            &fx.base_url(),
            "/ValueSet/$expand",
            &[("url", SURFACE_VS)],
        )
        .await;
        assert_eq!(resp.status().as_u16(), 503);
    }

    #[tokio::test]
    async fn fault_malformed_is_not_json() {
        let fx = FhirTxFixture::start_fault(Fault::Malformed).await;
        let resp = fetch(
            &client(2_000),
            &fx.base_url(),
            "/ValueSet/$expand",
            &[("url", SURFACE_VS)],
        )
        .await;
        assert_eq!(resp.status().as_u16(), 200);
        // The body is not FHIR JSON, so parsing as a Value of an object fails to
        // yield a resource — the CDR provider maps this to Exception (500).
        let text = resp.text().await.expect("text");
        assert!(serde_json::from_str::<Value>(&text).is_err());
    }

    #[tokio::test]
    async fn fault_timeout_exceeds_a_short_client_deadline() {
        let fx = FhirTxFixture::start_fault(Fault::Timeout).await;
        let err = client(300)
            .get(
                build_url(&fx.base_url(), "/ValueSet/$expand", &[("url", SURFACE_VS)])
                    .expect("url"),
            )
            .send()
            .await
            .expect_err("must time out");
        assert!(err.is_timeout(), "expected a timeout, got {err}");
    }

    #[tokio::test]
    async fn self_check_records_the_full_canned_exchange() {
        let fx = FhirTxFixture::start_canned().await;
        let exchanges = fx.self_check().await.expect("self-check");
        assert_eq!(exchanges.len(), 4, "one exchange per canned operation");
        assert!(exchanges.iter().all(|e| e.method == "GET"));
        assert!(exchanges.iter().any(|e| e.path == "/ValueSet/$expand"));
        assert!(exchanges.iter().any(|e| e.path == "/CodeSystem/$subsumes"));
    }
}
