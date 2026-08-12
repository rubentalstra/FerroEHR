// SPDX-FileCopyrightText: FerroEHR contributors
// SPDX-License-Identifier: MIT

//! The AQL optimization probe — the seeded-corpus troubleshooting loop.
//!
//! As an instrument it fires the measurement machinery's own AQL vocabulary
//! against a live SUT N times each, records wire-latency percentiles, and
//! attributes the DB-side cost per probe via `pg_stat_statements` through
//! the container runtime (the ixit `containers` capability). A seeded
//! database is the only place real bottlenecks show — an empty database
//! hides every planner failure — so the probe always runs against the
//! scale corpus + standing ward the class runs use.
//!
//! Exploration evidence for the optimization ladder (rungs cite probe
//! reports as their before/after) — never a conformance record, and
//! results.json is never touched.
//!
//! NOTE: no openEHR spec governs measured performance — our own
//! design/extension (the CNF guide excludes non-functional conformance;
//! see [`crate::perf`]).

#![expect(
    clippy::disallowed_types,
    reason = "dev/verification tooling over JSON artifacts (the catalogue, results, wire \
              exchanges) — not the application (#1694)"
)]

use std::process::Command;
use std::time::Instant;

use reqwest::StatusCode;
use serde::{Deserialize, Serialize};

use crate::ixit::{Containers, Environment};
use crate::perf_run::client::PerfClient;
use crate::perf_run::corpus::{ADHOC_AQL, STORED_QUERY_NAME, SeededCorpus, WARD_AQL};
use crate::perf_run::resources::settle_maintenance;

/// The probe shape (flag-tunable).
#[derive(Debug, Clone)]
pub struct ProbeOptions {
    /// Requests fired per probe (wire percentiles derive from these).
    pub requests: u32,
}

impl Default for ProbeOptions {
    fn default() -> Self {
        Self { requests: 20 }
    }
}

/// Wire-latency percentiles of one probe (milliseconds, client-observed).
#[expect(
    clippy::struct_field_names,
    reason = "the `_ms` suffix is the UNIT, and it is also the published artifact's \
              JSON key — dropping it would make a bare number ambiguous in the record"
)]
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WireStats {
    /// Fastest observed request, milliseconds.
    pub min_ms: f64,
    /// Median observed request, milliseconds.
    pub p50_ms: f64,
    /// 95th-percentile observed request, milliseconds.
    pub p95_ms: f64,
    /// Slowest observed request, milliseconds.
    pub max_ms: f64,
}

/// One DB statement's attributed cost over a probe's request burst
/// (`pg_stat_statements`, normalized SQL text).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StatementCost {
    /// The normalized statement text as `pg_stat_statements` reports it.
    pub sql: String,
    /// Executions attributed to the probe's request burst.
    pub calls: u64,
    /// Mean execution time per call, milliseconds.
    pub mean_ms: f64,
    /// Total execution time across the calls, milliseconds.
    pub total_ms: f64,
    /// Planning share (`pg_stat_statements.track_planning`; `PostgreSQL` docs
    /// section `pg_stat_statements`): mean/total planner time, separated
    /// from execution so a re-plan-heavy statement (unnamed statements re-plan
    /// every execution) is attributable. Zero when the probe could not enable
    /// `track_planning`.
    #[serde(default)]
    pub mean_plan_ms: f64,
    /// Total planner time across the calls, milliseconds.
    #[serde(default)]
    pub total_plan_ms: f64,
    /// Shared-buffer hits: blocks served without touching storage.
    pub shared_blks_hit: u64,
    /// Shared-buffer misses: blocks read from storage.
    pub shared_blks_read: u64,
}

/// One executed probe: the AQL fired, its wire percentiles, and the
/// attributed DB statements (empty when attribution is unavailable).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProbeResult {
    /// The probe's name in the report.
    pub name: String,
    /// The AQL text the probe executed.
    pub aql: String,
    /// Requests that did not return 200 (a failing probe is a finding,
    /// never an instrument error).
    pub failures: u32,
    /// Client-observed latency percentiles over the burst.
    pub wire_ms: WireStats,
    /// The DB statements attributed to the burst, costliest first.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub statements: Vec<StatementCost>,
}

/// The probe report — committed-able exploration evidence, environment-
/// bound like every other measurement artifact.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AqlProbeReport {
    /// The corpus the probes ran against (a class corpus key).
    pub corpus: String,
    /// The deployment the probes were measured in.
    pub environment: Environment,
    /// How many requests each probe fired.
    pub requests_per_probe: u32,
    /// Whether the maintenance debt was settled before probing
    /// (`vacuumdb --analyze` through the DB container).
    pub maintenance_settled: bool,
    /// `pg_stat_statements` when DB-side attribution ran, else the honest
    /// reason it could not.
    pub attribution: String,
    /// Every executed probe, in execution order.
    pub probes: Vec<ProbeResult>,
    /// The human summary, incl. the explicit exploration disclaimer.
    pub remark: String,
}

/// One wire probe: name, the AQL it exercises, method, path, JSON body.
type ProbeSpec = (
    &'static str,
    String,
    reqwest::Method,
    String,
    Option<Vec<u8>>,
);

/// The percentile of a sorted sample set (nearest-rank; `0` when empty).
fn percentile(sorted: &[f64], q: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    #[expect(
        clippy::as_conversions,
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "sample counts are tiny; the nearest-rank index is non-negative by construction"
    )]
    let index = ((q * sorted.len() as f64).ceil() as usize).saturating_sub(1);
    sorted
        .get(index.min(sorted.len() - 1))
        .copied()
        .unwrap_or(0.0)
}

/// Run one SQL statement on the `postgres` maintenance database inside the
/// DB container, returning stdout (the probe's attribution channel —
/// instrument telemetry, never a clinical-data path).
fn db_sql(db_container: &str, sql: &str) -> Result<String, String> {
    let output = Command::new("docker")
        .args([
            "exec",
            db_container,
            "psql",
            "-U",
            "postgres",
            "-d",
            "postgres",
            "-Atc",
            sql,
        ])
        .output()
        .map_err(|e| format!("docker exec: {e}"))?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    } else {
        Err(format!(
            "psql failed (exit {:?}): {}",
            output.status.code(),
            String::from_utf8_lossy(&output.stderr).trim()
        ))
    }
}

/// Read the top statements since the last reset, ordered by total time.
fn read_statements(db_container: &str) -> Result<Vec<StatementCost>, String> {
    let json = db_sql(
        db_container,
        "SELECT COALESCE(json_agg(t),'[]'::json) FROM (SELECT calls, mean_exec_time, \
         total_exec_time, mean_plan_time, total_plan_time, shared_blks_hit, \
         shared_blks_read, query FROM pg_stat_statements \
         WHERE query NOT ILIKE '%pg_stat_statements%' AND query NOT ILIKE 'VACUUM%' \
         ORDER BY total_exec_time + total_plan_time DESC LIMIT 8) t;",
    )?;
    let rows: Vec<serde_json::Value> =
        serde_json::from_str(json.trim()).map_err(|e| format!("statement JSON: {e}"))?;
    Ok(rows
        .iter()
        .filter_map(|row| {
            Some(StatementCost {
                sql: row.get("query")?.as_str()?.to_owned(),
                calls: row.get("calls")?.as_u64()?,
                mean_ms: row.get("mean_exec_time")?.as_f64()?,
                total_ms: row.get("total_exec_time")?.as_f64()?,
                // Planning columns are zero (never NULL) while track_planning
                // is off (PostgreSQL docs §pg_stat_statements).
                mean_plan_ms: row
                    .get("mean_plan_time")
                    .and_then(serde_json::Value::as_f64)
                    .unwrap_or(0.0),
                total_plan_ms: row
                    .get("total_plan_time")
                    .and_then(serde_json::Value::as_f64)
                    .unwrap_or(0.0),
                shared_blks_hit: row
                    .get("shared_blks_hit")
                    .and_then(serde_json::Value::as_u64)
                    .unwrap_or(0),
                shared_blks_read: row
                    .get("shared_blks_read")
                    .and_then(serde_json::Value::as_u64)
                    .unwrap_or(0),
            })
        })
        .collect())
}

/// Execute the probe set against a live, seeded SUT.
///
/// # Errors
/// A message on a probe-construction failure (an empty corpus); a failing
/// REQUEST is a recorded finding, and absent attribution/settling degrade
/// to honest report fields — never errors.
#[expect(clippy::too_many_lines, reason = "one linear probe procedure")]
pub fn run_probe(
    client: &PerfClient,
    corpus: &SeededCorpus,
    environment: &Environment,
    containers: Option<&Containers>,
    options: &ProbeOptions,
    progress: &(dyn Fn(String) + Sync),
) -> Result<AqlProbeReport, String> {
    let ehr_id = corpus
        .ehr_ids
        .first()
        .ok_or_else(|| "corpus index has no EHRs".to_owned())?
        .clone();
    let requests = options.requests.max(1);

    // Settle first — probing an unsettled database measures the maintenance
    // debt, not the schema (a stale-statistics plan cost a measured ~9x).
    let maintenance_settled = if let Some(c) = containers {
        match settle_maintenance(&c.db) {
            Ok(()) => {
                progress("maintenance settled (vacuumdb --analyze)".to_owned());
                true
            }
            Err(e) => {
                progress(format!("maintenance not settled: {e}"));
                false
            }
        }
    } else {
        progress("maintenance not settled (no ixit `containers` block)".to_owned());
        false
    };

    // Attribution capability: pg_stat_statements through the DB container.
    // `track_planning` separates planner time from executor time per
    // statement (PostgreSQL docs §pg_stat_statements) — the planning-share
    // attribution the optimization ladder decides rung admissions on. The GUC
    // is settable at runtime (ALTER SYSTEM + reload; no restart needed for a
    // pg_stat_statements tracking knob).
    let attribution_db = containers.and_then(|c| {
        match db_sql(&c.db, "CREATE EXTENSION IF NOT EXISTS pg_stat_statements;") {
            Ok(_) => {
                // Two separate calls: `ALTER SYSTEM` refuses to run inside a
                // transaction block, and a multi-statement simple query is one
                // implicit transaction (PostgreSQL docs §ALTER SYSTEM).
                let armed = db_sql(
                    &c.db,
                    "ALTER SYSTEM SET pg_stat_statements.track_planning = on;",
                )
                .and_then(|_| db_sql(&c.db, "SELECT pg_reload_conf();"));
                if let Err(e) = armed {
                    progress(format!(
                        "track_planning unavailable (plan share reads 0): {e}"
                    ));
                }
                Some(c.db.clone())
            }
            Err(e) => {
                progress(format!("statement attribution unavailable: {e}"));
                None
            }
        }
    });
    let attribution = attribution_db.as_ref().map_or_else(
        || "unavailable (no containers block or pg_stat_statements not loadable)".to_owned(),
        |_| "pg_stat_statements".to_owned(),
    );

    // The closed probe set: the measurement machinery's own AQL vocabulary.
    let adhoc_body =
        serde_json::json!({ "q": ADHOC_AQL, "query_parameters": { "ehr_id": ehr_id } });
    let ward_body = serde_json::json!({ "q": WARD_AQL });
    let stored_path = format!("/query/{STORED_QUERY_NAME}?ehr_id={ehr_id}");
    let probe_set: Vec<ProbeSpec> = vec![
        (
            "ward_worklist",
            WARD_AQL.to_owned(),
            reqwest::Method::POST,
            "/query/aql".to_owned(),
            Some(serde_json::to_vec(&ward_body).map_err(|e| e.to_string())?),
        ),
        (
            "adhoc_trend",
            ADHOC_AQL.to_owned(),
            reqwest::Method::POST,
            "/query/aql".to_owned(),
            Some(serde_json::to_vec(&adhoc_body).map_err(|e| e.to_string())?),
        ),
        (
            "stored_dashboard",
            format!("GET /query/{STORED_QUERY_NAME} (registered: {ADHOC_AQL})"),
            reqwest::Method::GET,
            stored_path,
            None,
        ),
    ];

    let mut probes = Vec::new();
    for (name, aql, method, path, body) in probe_set {
        if let Some(db) = &attribution_db {
            let _reset = db_sql(db, "SELECT pg_stat_statements_reset();");
        }
        let mut samples_ms: Vec<f64> = Vec::new();
        let mut failures: u32 = 0;
        for _ in 0..requests {
            let started = Instant::now();
            let reply = client.request(
                method.clone(),
                &path,
                body.as_ref()
                    .map(|bytes| ("application/json", bytes.clone())),
                false,
                None,
            );
            let elapsed_ms = started.elapsed().as_secs_f64() * 1_000.0;
            match reply {
                Ok(reply) if reply.status == StatusCode::OK => samples_ms.push(elapsed_ms),
                Ok(_) | Err(_) => {
                    failures = failures.saturating_add(1);
                    samples_ms.push(elapsed_ms);
                }
            }
        }
        samples_ms.sort_by(f64::total_cmp);
        let wire_ms = WireStats {
            min_ms: samples_ms.first().copied().unwrap_or(0.0),
            p50_ms: percentile(&samples_ms, 0.50),
            p95_ms: percentile(&samples_ms, 0.95),
            max_ms: samples_ms.last().copied().unwrap_or(0.0),
        };
        let statements = attribution_db
            .as_ref()
            .and_then(|db| match read_statements(db) {
                Ok(rows) => Some(rows),
                Err(e) => {
                    progress(format!("probe {name}: attribution read failed: {e}"));
                    None
                }
            })
            .unwrap_or_default();
        progress(format!(
            "probe {name}: p50 {:.1} ms · p95 {:.1} ms · max {:.1} ms ({requests} requests, {failures} failures{})",
            wire_ms.p50_ms,
            wire_ms.p95_ms,
            wire_ms.max_ms,
            statements.first().map_or(String::new(), |top| {
                format!(
                    "; top statement {:.1} ms mean × {} calls",
                    top.mean_ms, top.calls
                )
            }),
        ));
        probes.push(ProbeResult {
            name: name.to_owned(),
            aql,
            failures,
            wire_ms,
            statements,
        });
    }

    let remark = format!(
        "AQL probe over the seeded {} corpus — exploration evidence for the optimization \
         loop; never a conformance record (results.json untouched). Wire percentiles from \
         {requests} requests per probe; DB attribution: {attribution}.",
        corpus.corpus,
    );
    progress(remark.clone());
    Ok(AqlProbeReport {
        corpus: corpus.corpus.clone(),
        environment: environment.clone(),
        requests_per_probe: requests,
        maintenance_settled,
        attribution,
        probes,
        remark,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percentiles_are_nearest_rank() {
        let sorted: Vec<f64> = (1..=100).map(f64::from).collect();
        assert!((percentile(&sorted, 0.50) - 50.0).abs() < f64::EPSILON);
        assert!((percentile(&sorted, 0.95) - 95.0).abs() < f64::EPSILON);
        assert!((percentile(&[42.0], 0.95) - 42.0).abs() < f64::EPSILON);
        assert!(percentile(&[], 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn the_report_round_trips_and_degrades_honestly() {
        let report = AqlProbeReport {
            corpus: "cnf.scale.10k".to_owned(),
            environment: serde_json::from_value(serde_json::json!({
                "hardware_class": "test", "cores": 1, "memory_gb": 1,
                "storage_class": "ram", "topology": "stub"
            }))
            .unwrap(),
            requests_per_probe: 20,
            maintenance_settled: false,
            attribution: "unavailable (no containers block or pg_stat_statements not loadable)"
                .to_owned(),
            probes: vec![ProbeResult {
                name: "ward_worklist".to_owned(),
                aql: "SELECT ...".to_owned(),
                failures: 0,
                wire_ms: WireStats {
                    min_ms: 1.0,
                    p50_ms: 2.0,
                    p95_ms: 3.0,
                    max_ms: 4.0,
                },
                statements: Vec::new(),
            }],
            remark: "r".to_owned(),
        };
        let value = serde_json::to_value(&report).unwrap();
        // Absent attribution serializes as an empty-free record (statements
        // omitted), never fabricated rows.
        assert!(value["probes"][0].get("statements").is_none());
        let parsed: AqlProbeReport = serde_json::from_value(value).unwrap();
        assert_eq!(parsed.probes.len(), 1);
        assert!(!parsed.maintenance_settled);
    }
}
