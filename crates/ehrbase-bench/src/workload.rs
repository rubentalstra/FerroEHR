//! The pre-registered workload (design §2): the scenarios driven against both
//! servers. Payloads come from the vendored CNF fixture corpus so neither
//! server gets a bespoke-tuned input, and the set is **frozen** via
//! [`workload_lock`] — a hash recorded in every report so results cannot be
//! silently re-tuned after seeing them.
//!
//! Each scenario is a `prepare` (one-time setup: create the EHR, upload the
//! template) followed by a repeatable `operation` (the single measured request).
//! The representative create/read/query core (W1/W2/W4/W8) is implemented; the
//! remaining W3/W5–W13 scenarios from the design slot in behind the same two
//! methods. `expected_status` feeds the pre-flight conformance gate (design
//! §4.1) so the harness never times an error path.

use ehrbase_conformance::fixtures;
use ehrbase_conformance::harness::{AuthSlot, HttpRequest, Method};

use crate::BenchError;
use crate::target::Target;

/// The vendored fixtures the workload commits (a known template + a composition
/// that validates against it — the content suite confirmed this pair).
const NESTED_OPT: &str = "valid_templates/nested/nested.opt";
const NESTED_COMPOSITION: &str = "compositions/CANONICAL_JSON/nested.en.v1__full.json";

/// A pre-registered benchmark scenario.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scenario {
    /// W1 — create an EHR (baseline write + id generation).
    EhrCreate,
    /// W2 — create a composition (the hot write path).
    CompositionCreate,
    /// W4 — get a composition by version id (point read / reassembly).
    CompositionGet,
    /// W8 — execute a simple ad-hoc AQL query (full-scan read).
    AqlQuery,
}

impl Scenario {
    /// Every implemented scenario, in workload order.
    pub const ALL: &'static [Scenario] = &[
        Scenario::EhrCreate,
        Scenario::CompositionCreate,
        Scenario::CompositionGet,
        Scenario::AqlQuery,
    ];

    /// The workload id (W1..W13).
    #[must_use]
    pub fn id(self) -> &'static str {
        match self {
            Scenario::EhrCreate => "W1",
            Scenario::CompositionCreate => "W2",
            Scenario::CompositionGet => "W4",
            Scenario::AqlQuery => "W8",
        }
    }

    /// A one-line description.
    #[must_use]
    pub fn description(self) -> &'static str {
        match self {
            Scenario::EhrCreate => "create EHR",
            Scenario::CompositionCreate => "create composition (nested template)",
            Scenario::CompositionGet => "get composition by version id",
            Scenario::AqlQuery => "AQL: SELECT c FROM COMPOSITION c",
        }
    }

    /// The status codes a correct server returns for the measured operation —
    /// the pre-flight conformance gate (design §4.1). A response outside this
    /// set excludes the scenario from that server's timing (we never time an
    /// error path).
    #[must_use]
    pub fn expected_status(self) -> &'static [u16] {
        match self {
            Scenario::EhrCreate | Scenario::CompositionCreate => &[201],
            Scenario::CompositionGet | Scenario::AqlQuery => &[200],
        }
    }

    /// One-time setup: create the EHR and (for write/read scenarios) upload the
    /// template and seed a composition to read back.
    ///
    /// # Errors
    /// [`BenchError`] on transport failure or an unexpected setup response.
    pub async fn prepare(self, target: &Target) -> Result<Prepared, BenchError> {
        let ehr_id = create_ehr(target).await?;
        match self {
            Scenario::EhrCreate => Ok(Prepared {
                ehr_id: None,
                composition_body: None,
                composition_uid: None,
            }),
            Scenario::CompositionCreate => {
                ensure_template(target).await?;
                Ok(Prepared {
                    ehr_id: Some(ehr_id),
                    composition_body: Some(composition_body()?),
                    composition_uid: None,
                })
            }
            Scenario::CompositionGet => {
                ensure_template(target).await?;
                let uid = create_composition(target, &ehr_id, &composition_body()?).await?;
                Ok(Prepared {
                    ehr_id: Some(ehr_id),
                    composition_body: None,
                    composition_uid: Some(uid),
                })
            }
            Scenario::AqlQuery => {
                ensure_template(target).await?;
                let _ = create_composition(target, &ehr_id, &composition_body()?).await?;
                Ok(Prepared {
                    ehr_id: Some(ehr_id),
                    composition_body: None,
                    composition_uid: None,
                })
            }
        }
    }

    /// Perform one measured request; returns the HTTP status for gating.
    ///
    /// # Errors
    /// [`BenchError`] on a transport-level failure.
    pub async fn operation(self, target: &Target, prep: &Prepared) -> Result<u16, BenchError> {
        let req = match self {
            Scenario::EhrCreate => HttpRequest::new(Method::Post, "/ehr")
                .with_auth(AuthSlot::Regular)
                .header("prefer", "return=representation"),
            Scenario::CompositionCreate => {
                let ehr = prep.ehr_id.as_deref().ok_or_missing("ehr_id")?;
                let body = prep.composition_body.as_deref().ok_or_missing("body")?;
                HttpRequest::new(Method::Post, format!("/ehr/{ehr}/composition"))
                    .with_auth(AuthSlot::Regular)
                    .header("prefer", "return=representation")
                    .text_body(body.to_owned(), "application/json")
            }
            Scenario::CompositionGet => {
                let ehr = prep.ehr_id.as_deref().ok_or_missing("ehr_id")?;
                let uid = prep.composition_uid.as_deref().ok_or_missing("uid")?;
                HttpRequest::new(Method::Get, format!("/ehr/{ehr}/composition/{uid}"))
                    .with_auth(AuthSlot::Regular)
                    .header("accept", "application/json")
            }
            Scenario::AqlQuery => HttpRequest::new(Method::Post, "/query/aql")
                .with_auth(AuthSlot::Regular)
                .header("accept", "application/json")
                .text_body(
                    serde_json::json!({ "q": "SELECT c FROM COMPOSITION c LIMIT 10" }).to_string(),
                    "application/json",
                ),
        };
        let resp = target.send(req).await?;
        Ok(resp.status)
    }
}

/// The state carried from `prepare` into each `operation`.
#[derive(Debug, Clone, Default)]
pub struct Prepared {
    /// The EHR the operation targets (write/read scenarios).
    pub ehr_id: Option<String>,
    /// The composition body to POST (create scenario).
    pub composition_body: Option<String>,
    /// A committed composition's version uid (read scenario).
    pub composition_uid: Option<String>,
}

/// A stable hash over the frozen workload definition (ids + descriptions +
/// expected statuses + the fixture pair). Recorded in every report so a reader
/// can confirm the workload was not changed between runs (design §2).
#[must_use]
pub fn workload_lock() -> String {
    // A small, dependency-free FNV-1a over the canonical definition string.
    let mut def = String::new();
    for s in Scenario::ALL {
        def.push_str(s.id());
        def.push('|');
        def.push_str(s.description());
        def.push('|');
        for code in s.expected_status() {
            def.push_str(&code.to_string());
            def.push(',');
        }
        def.push('\n');
    }
    def.push_str(NESTED_OPT);
    def.push_str(NESTED_COMPOSITION);

    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for b in def.as_bytes() {
        hash ^= u64::from(*b);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{hash:016x}")
}

// ── setup helpers ────────────────────────────────────────────────────────────

async fn create_ehr(target: &Target) -> Result<String, BenchError> {
    let req = HttpRequest::new(Method::Post, "/ehr")
        .with_auth(AuthSlot::Regular)
        .header("prefer", "return=representation");
    let resp = target.send(req).await?;
    if resp.status != 201 {
        return Err(BenchError::Unexpected(format!(
            "create EHR: expected 201, got {}",
            resp.status
        )));
    }
    let value = resp
        .json()
        .map_err(|e| BenchError::Unexpected(format!("create EHR body: {e}")))?;
    value
        .get("ehr_id")
        .and_then(|v| v.get("value"))
        .and_then(serde_json::Value::as_str)
        .map(ToOwned::to_owned)
        .ok_or_else(|| BenchError::Unexpected("create EHR: no ehr_id/value in body".to_owned()))
}

async fn ensure_template(target: &Target) -> Result<(), BenchError> {
    let opt = fixtures::read(NESTED_OPT).map_err(|e| BenchError::Fixture(e.to_string()))?;
    let req = HttpRequest::new(Method::Post, "/definition/template/adl1.4")
        .with_auth(AuthSlot::Regular)
        .header("content-type", "application/xml")
        .text_body(opt, "application/xml");
    let resp = target.send(req).await?;
    // 201 = created, 409 = already present (idempotent upload across runs).
    if resp.status == 201 || resp.status == 409 {
        Ok(())
    } else {
        Err(BenchError::Unexpected(format!(
            "upload template: got {}",
            resp.status
        )))
    }
}

fn composition_body() -> Result<String, BenchError> {
    let value =
        fixtures::read_json(NESTED_COMPOSITION).map_err(|e| BenchError::Fixture(e.to_string()))?;
    Ok(value.to_string())
}

async fn create_composition(
    target: &Target,
    ehr_id: &str,
    body: &str,
) -> Result<String, BenchError> {
    let req = HttpRequest::new(Method::Post, format!("/ehr/{ehr_id}/composition"))
        .with_auth(AuthSlot::Regular)
        .header("prefer", "return=representation")
        .header("accept", "application/json")
        .text_body(body.to_owned(), "application/json");
    let resp = target.send(req).await?;
    if resp.status != 201 {
        return Err(BenchError::Unexpected(format!(
            "seed composition: got {}",
            resp.status
        )));
    }
    resp.header("etag")
        .or_else(|| resp.header("location"))
        .map(|h| {
            h.trim_matches('"')
                .rsplit('/')
                .next()
                .unwrap_or(h)
                .to_owned()
        })
        .ok_or_else(|| BenchError::Unexpected("seed composition: no ETag/Location".to_owned()))
}

/// Small helper for the `Option` → `BenchError` unwrap in `operation`.
trait OrMissing<T> {
    fn ok_or_missing(self, what: &str) -> Result<T, BenchError>;
}

impl<T> OrMissing<T> for Option<T> {
    fn ok_or_missing(self, what: &str) -> Result<T, BenchError> {
        self.ok_or_else(|| BenchError::Unexpected(format!("prepared state missing {what}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_scenarios_have_distinct_ids() {
        let ids: Vec<_> = Scenario::ALL.iter().map(|s| s.id()).collect();
        let mut sorted = ids.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(ids.len(), sorted.len(), "scenario ids must be distinct");
    }

    #[test]
    fn workload_lock_is_stable() {
        assert_eq!(workload_lock(), workload_lock());
        assert_eq!(workload_lock().len(), 16);
    }

    #[test]
    fn expected_statuses_are_success_codes() {
        for s in Scenario::ALL {
            assert!(s.expected_status().iter().all(|c| (200..300).contains(c)));
        }
    }
}
