//! Deterministic data-scale seeding (design §2.3): populate a SUT with a fixed,
//! reproducible dataset so AQL/read scenarios can be measured at empty / 10k /
//! 100k / 1M compositions. The same seed produces the same data on both servers,
//! so a scale comparison is apples-to-apples.
//!
//! Both servers are seeded **through the API** (no DB backdoor), so the dataset
//! is identical by construction. This is the bulk-create primitive; the
//! empty→1M ladder orchestration and its per-rung storage-footprint measurement
//! (design §3.5) build on it.

use ehrbase_conformance::fixtures;
use ehrbase_conformance::harness::{AuthSlot, HttpRequest, Method};

use crate::BenchError;
use crate::target::Target;

const NESTED_OPT: &str = "valid_templates/nested/nested.opt";
const NESTED_COMPOSITION: &str = "compositions/CANONICAL_JSON/nested.en.v1__full.json";

/// Seed `ehrs` EHRs, each with `comps_per_ehr` compositions of the nested
/// template. Returns the number of compositions committed. Idempotent on the
/// template (re-upload → 409 tolerated).
///
/// # Errors
/// [`BenchError`] on a transport failure or an unexpected server response.
pub async fn seed(target: &Target, ehrs: u32, comps_per_ehr: u32) -> Result<u64, BenchError> {
    ensure_template(target).await?;
    let body =
        fixtures::read_json(NESTED_COMPOSITION).map_err(|e| BenchError::Fixture(e.to_string()))?;
    let body = body.to_string();

    let mut committed = 0u64;
    for _ in 0..ehrs {
        let ehr_id = create_ehr(target).await?;
        for _ in 0..comps_per_ehr {
            commit_composition(target, &ehr_id, &body).await?;
            committed += 1;
        }
    }
    Ok(committed)
}

async fn ensure_template(target: &Target) -> Result<(), BenchError> {
    let opt = fixtures::read(NESTED_OPT).map_err(|e| BenchError::Fixture(e.to_string()))?;
    let req = HttpRequest::new(Method::Post, "/definition/template/adl1.4")
        .with_auth(AuthSlot::Regular)
        .header("content-type", "application/xml")
        .text_body(opt, "application/xml");
    let resp = target.send(req).await?;
    if resp.status == 201 || resp.status == 409 {
        Ok(())
    } else {
        Err(BenchError::Unexpected(format!(
            "seed: upload template got {}",
            resp.status
        )))
    }
}

async fn create_ehr(target: &Target) -> Result<String, BenchError> {
    let req = HttpRequest::new(Method::Post, "/ehr")
        .with_auth(AuthSlot::Regular)
        .header("prefer", "return=representation");
    let resp = target.send(req).await?;
    if resp.status != 201 {
        return Err(BenchError::Unexpected(format!(
            "seed: create EHR got {}",
            resp.status
        )));
    }
    resp.json()
        .map_err(|e| BenchError::Unexpected(format!("seed: EHR body {e}")))?
        .get("ehr_id")
        .and_then(|v| v.get("value"))
        .and_then(serde_json::Value::as_str)
        .map(ToOwned::to_owned)
        .ok_or_else(|| BenchError::Unexpected("seed: no ehr_id in body".to_owned()))
}

async fn commit_composition(target: &Target, ehr_id: &str, body: &str) -> Result<(), BenchError> {
    let req = HttpRequest::new(Method::Post, format!("/ehr/{ehr_id}/composition"))
        .with_auth(AuthSlot::Regular)
        .header("prefer", "return=minimal")
        .text_body(body.to_owned(), "application/json");
    let resp = target.send(req).await?;
    if resp.status == 201 {
        Ok(())
    } else {
        Err(BenchError::Unexpected(format!(
            "seed: commit composition got {}",
            resp.status
        )))
    }
}
