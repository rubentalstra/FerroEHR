//! The measurement drivers (design §2.2, §4.2–§4.4).
//!
//! The **latency profile** is closed-loop with a single client: prepare once,
//! run a warmup that is discarded, then measure a fixed iteration count,
//! recording per-request service time. The whole thing is repeated over `runs`
//! independent runs and the inter-run coefficient of variation is reported — a
//! difference inside the noise band is *not* a result (design §4.4). Warmup is
//! applied identically to both servers, so the JVM is warmed, not handicapped
//! (design §4.2).

use tokio::time::Instant;

use crate::BenchError;
use crate::measure::{LatencyRecorder, LatencySummary, coefficient_of_variation};
use crate::target::Target;
use crate::workload::Scenario;

/// How much to run per scenario.
#[derive(Debug, Clone, Copy)]
pub struct DriverConfig {
    /// Discarded warmup iterations (per run).
    pub warmup_iters: u64,
    /// Measured iterations (per run).
    pub measure_iters: u64,
    /// Independent runs (each re-prepares state); ≥5 for a publishable result.
    pub runs: u32,
}

impl Default for DriverConfig {
    fn default() -> Self {
        Self {
            warmup_iters: 200,
            measure_iters: 2_000,
            runs: 5,
        }
    }
}

impl DriverConfig {
    /// A fast smoke configuration (proving the harness, not publishing numbers).
    #[must_use]
    pub fn smoke() -> Self {
        Self {
            warmup_iters: 20,
            measure_iters: 100,
            runs: 2,
        }
    }
}

/// One run's outcome.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RunResult {
    /// The latency distribution for this run.
    pub latency: LatencySummary,
    /// Sustained requests/second over the measurement window.
    pub throughput_rps: f64,
}

/// A scenario's result against one target: the pre-flight gate outcome, the
/// per-run results, the merged distribution, and the inter-run variance.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ScenarioResult {
    /// The scenario id.
    pub scenario: String,
    /// The resource group (EHR / COMPOSITION / QUERY / …) for the coverage overview.
    pub group: String,
    /// Human description.
    pub description: String,
    /// The target label (ehrbase-rs / ehrbase-java).
    pub target: String,
    /// Whether the pre-flight conformance gate passed (design §4.1). When false,
    /// the timings below are absent — we never time an error path.
    pub gate_ok: bool,
    /// The status the gate observed.
    pub gate_status: u16,
    /// Per-run results (empty if the gate failed).
    pub runs: Vec<RunResult>,
    /// The distribution merged across all runs (`None` if the gate failed).
    pub merged: Option<LatencySummary>,
    /// Median sustained throughput across runs (req/s).
    pub throughput_median_rps: Option<f64>,
    /// Inter-run coefficient of variation of throughput; `> 0.10` = high variance.
    pub throughput_cov: Option<f64>,
}

impl ScenarioResult {
    fn gate_failed(scenario: Scenario, target: &Target, status: u16) -> Self {
        Self {
            scenario: scenario.id().to_owned(),
            group: scenario.group().to_owned(),
            description: scenario.description().to_owned(),
            target: target.label().to_owned(),
            gate_ok: false,
            gate_status: status,
            runs: Vec::new(),
            merged: None,
            throughput_median_rps: None,
            throughput_cov: None,
        }
    }
}

/// Run a scenario's latency profile against a target (design §2.2).
///
/// # Errors
/// [`BenchError`] on a setup/transport failure (a *wrong* but successful
/// response is a gate failure, not an error — it is recorded, not raised).
pub async fn run_latency(
    target: &Target,
    scenario: Scenario,
    cfg: DriverConfig,
) -> Result<ScenarioResult, BenchError> {
    // Pre-flight conformance gate (§4.1): one operation, assert the status.
    let gate_prep = scenario.prepare(target).await?;
    let gate_status = scenario.operation(target, &gate_prep).await?;
    if !scenario.expected_status().contains(&gate_status) {
        return Ok(ScenarioResult::gate_failed(scenario, target, gate_status));
    }

    let mut runs = Vec::with_capacity(cfg.runs as usize);
    let mut merged = LatencyRecorder::new();
    let mut throughputs = Vec::with_capacity(cfg.runs as usize);

    for _ in 0..cfg.runs {
        let prep = scenario.prepare(target).await?;

        // Warmup — discarded (§4.2), applied identically to both servers.
        for _ in 0..cfg.warmup_iters {
            let _ = scenario.operation(target, &prep).await?;
        }

        // Measure.
        let mut rec = LatencyRecorder::new();
        let window = Instant::now();
        for _ in 0..cfg.measure_iters {
            let start = Instant::now();
            let _ = scenario.operation(target, &prep).await?;
            let elapsed_us = u64::try_from(start.elapsed().as_micros()).unwrap_or(u64::MAX);
            rec.record(elapsed_us);
        }
        let secs = window.elapsed().as_secs_f64();
        let rps = if secs > 0.0 {
            cfg.measure_iters as f64 / secs
        } else {
            0.0
        };

        merged
            .merge(&rec)
            .map_err(|e| BenchError::Unexpected(format!("merge histograms: {e}")))?;
        runs.push(RunResult {
            latency: rec.summary(),
            throughput_rps: rps,
        });
        throughputs.push(rps);
    }

    let throughput_median = median(&throughputs);

    Ok(ScenarioResult {
        scenario: scenario.id().to_owned(),
        group: scenario.group().to_owned(),
        description: scenario.description().to_owned(),
        target: target.label().to_owned(),
        gate_ok: true,
        gate_status,
        runs,
        merged: Some(merged.summary()),
        throughput_median_rps: Some(throughput_median),
        throughput_cov: coefficient_of_variation(&throughputs),
    })
}

fn median(values: &[f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let mut v = values.to_vec();
    v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let mid = v.len() / 2;
    if v.len().is_multiple_of(2) {
        f64::midpoint(v[mid - 1], v[mid])
    } else {
        v[mid]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn median_handles_even_and_odd() {
        assert!((median(&[1.0, 2.0, 3.0]) - 2.0).abs() < f64::EPSILON);
        assert!((median(&[1.0, 2.0, 3.0, 4.0]) - 2.5).abs() < f64::EPSILON);
        assert!((median(&[]) - 0.0).abs() < f64::EPSILON);
    }
}
