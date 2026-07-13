//! Deterministic scale-ladder seeding (register 00 §5): populate a SUT with a
//! fixed, reproducible dataset so read/AQL performance is measured against real
//! stored volume. The same seed produces the same data on every SUT (seeded
//! **through the API**, no DB backdoor), so a cross-SUT scale comparison is
//! apples-to-apples.
//!
//! The corpus templates are provisioned once, then compositions are committed
//! across ~`N/50` EHRs (one canonical composition per template, rotated, with
//! only the per-iteration timestamp varied — simple, honest, deterministic).

use serde_json::Value;

use conformance::harness::{AuthSlot, HttpRequest, Method, Transport};
use conformance::transport::SutClient;

use crate::drive::{template_composition, template_opt_xml};
use crate::{BenchError, Scale, TemplateKind};

/// Distribute the rung's compositions across this many per EHR (register 00 §5:
/// "distribute over ~ N/50 EHRs").
const COMPS_PER_EHR: u64 = 50;

/// The three corpus templates the workload + seeder use.
const TEMPLATES: [TemplateKind; 3] = [
    TemplateKind::Vitals,
    TemplateKind::Nested,
    TemplateKind::Persistent,
];

/// Seed the SUT to a scale rung: provision the templates, then commit its
/// composition count across `count / 50` EHRs. Returns the number committed.
/// `Scale::Empty` is a no-op (returns 0).
///
/// # Errors
/// [`BenchError`] on a transport failure or an unexpected server response.
pub async fn seed_scale(client: &SutClient, scale: Scale, seed: u64) -> Result<u64, BenchError> {
    let target = scale.compositions();
    if target == 0 {
        return Ok(0);
    }

    for kind in TEMPLATES {
        provision(client, kind).await?;
    }

    // One canonical composition per template, rendered once and reused with only
    // the timestamp varied per iteration (register 00 §4: variation touches
    // values, never structure). Any fixture-carried `uid` is stripped: version
    // identities are server-assigned on commit, and re-committing a fixed
    // OBJECT identity is a `409` (ITS-REST: a resource with the same
    // identifier already exists) — the corpus `composition_evaluation_test`
    // fixture ships one.
    let bases: Vec<Value> = TEMPLATES
        .iter()
        .map(|k| {
            template_composition(*k).map(|mut v| {
                if let Some(obj) = v.as_object_mut() {
                    obj.remove("uid");
                }
                v
            })
        })
        .collect::<Result<_, _>>()?;

    let ehr_count = (target / COMPS_PER_EHR).max(1);
    let per_ehr = target.div_ceil(ehr_count);

    // Per EHR: exactly ONE persistent composition (a persistent COMPOSITION is
    // a per-EHR singleton updated over time — RM ehr §COMPOSITION category
    // `persistent`; the SUT rightly answers 409 on a second create), then
    // event compositions (Vitals/Nested alternating) for the bulk. One care
    // plan + a stream of events per patient is also the realistic shape.
    let (persistent, events): (Vec<&Value>, Vec<&Value>) = {
        let mut p = Vec::new();
        let mut e = Vec::new();
        for (kind, base) in TEMPLATES.iter().zip(bases.iter()) {
            if matches!(kind, TemplateKind::Persistent) {
                p.push(base);
            } else {
                e.push(base);
            }
        }
        (p, e)
    };

    let mut committed: u64 = 0;
    'ehrs: for _ in 0..ehr_count {
        let ehr = create_ehr(client).await?;
        for slot in 0..per_ehr {
            if committed >= target {
                break 'ehrs;
            }
            let body = if slot == 0 && !persistent.is_empty() {
                vary_timestamp(persistent[0], seed, committed)
            } else {
                let idx = usize::try_from(committed % events.len() as u64).unwrap_or(0);
                vary_timestamp(events[idx], seed, committed)
            };
            commit_composition(client, &ehr, &body).await?;
            committed += 1;
            if committed.is_multiple_of(1000) {
                eprintln!("bench seed: {committed}/{target} compositions");
            }
        }
    }
    Ok(committed)
}

/// Provision one template's OPT, tolerating an already-present template.
async fn provision(client: &SutClient, kind: TemplateKind) -> Result<(), BenchError> {
    let xml = template_opt_xml(kind)?;
    let req = HttpRequest::new(Method::Post, "/definition/template/adl1.4")
        .with_auth(AuthSlot::Regular)
        .text_body(xml, "application/xml");
    let resp = client.send(req).await?;
    if matches!(resp.status, 200 | 201 | 204 | 409) {
        Ok(())
    } else {
        Err(BenchError::Unexpected(format!(
            "seed: upload template got {}",
            resp.status
        )))
    }
}

/// Create an EHR and return its `ehr_id`.
async fn create_ehr(client: &SutClient) -> Result<String, BenchError> {
    let req = HttpRequest::new(Method::Post, "/ehr")
        .with_auth(AuthSlot::Regular)
        .header("prefer", "return=representation");
    let resp = client.send(req).await?;
    if resp.status != 201 {
        return Err(BenchError::Unexpected(format!(
            "seed: create EHR got {}",
            resp.status
        )));
    }
    resp.json()
        .map_err(|e| BenchError::Unexpected(format!("seed: EHR body {e}")))?
        .pointer("/ehr_id/value")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
        .ok_or_else(|| BenchError::Unexpected("seed: no ehr_id in body".to_owned()))
}

/// Commit one composition (minimal return — throughput seeding, not measurement).
async fn commit_composition(
    client: &SutClient,
    ehr_id: &str,
    body: &Value,
) -> Result<(), BenchError> {
    let req = HttpRequest {
        method: Method::Post,
        path: format!("/ehr/{ehr_id}/composition"),
        headers: vec![
            ("content-type".to_owned(), "application/json".to_owned()),
            ("prefer".to_owned(), "return=minimal".to_owned()),
        ],
        body: Some(serde_json::to_vec(body).unwrap_or_default()),
        auth: AuthSlot::Regular,
    };
    let resp = client.send(req).await?;
    if resp.status == 201 {
        Ok(())
    } else {
        Err(BenchError::Unexpected(format!(
            "seed: commit composition got {}",
            resp.status
        )))
    }
}

/// Clone a base composition and set its `context.start_time` to a deterministic
/// timestamp advancing one minute per iteration (offset by the seed), so the
/// seeded corpus is ordered and reproducible. Compositions without a
/// `context/start_time` leaf (persistent category) are left unchanged.
fn vary_timestamp(base: &Value, seed: u64, iter: u64) -> Value {
    let mut comp = base.clone();
    if let Some(slot) = comp.pointer_mut("/context/start_time/value") {
        *slot = Value::String(iso_at(seed, iter));
    }
    comp
}

/// A deterministic RFC-3339 timestamp: `2020-01-01T00:00:00Z` + (seed + iter·60)
/// seconds.
fn iso_at(seed: u64, iter: u64) -> String {
    // 2020-01-01T00:00:00Z.
    const BASE: i64 = 1_577_836_800;
    // Fold the seed into a bounded day-scale offset so the synthetic clock stays
    // in a sane range (and the casts are always in i64 range).
    let seed = i64::try_from(seed % 86_400).unwrap_or(0);
    let iter = i64::try_from(iter % 1_000_000_000).unwrap_or(0);
    let offset = seed.wrapping_add(iter.wrapping_mul(60));
    jiff::Timestamp::from_second(BASE.wrapping_add(offset))
        .map_or_else(|_| "2020-01-01T00:00:00Z".to_owned(), |t| t.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timestamp_is_deterministic_and_advances() {
        let a = iso_at(0, 0);
        let b = iso_at(0, 0);
        assert_eq!(a, b, "same (seed, iter) must be identical");
        assert_ne!(iso_at(0, 0), iso_at(0, 1), "iterations advance the clock");
        assert!(a.starts_with("2020-01-01T00:00:00"), "{a}");
    }

    #[test]
    fn vary_only_touches_start_time_when_present() {
        let base = serde_json::json!({
            "_type": "COMPOSITION",
            "context": { "start_time": { "value": "1999-01-01T00:00:00Z" } }
        });
        let varied = vary_timestamp(&base, 5, 3);
        assert_ne!(
            varied.pointer("/context/start_time/value"),
            base.pointer("/context/start_time/value")
        );

        // A persistent composition without a context is returned unchanged.
        let persistent = serde_json::json!({ "_type": "COMPOSITION" });
        assert_eq!(vary_timestamp(&persistent, 5, 3), persistent);
    }
}
