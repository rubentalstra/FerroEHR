// SPDX-FileCopyrightText: FerroEHR contributors
// SPDX-License-Identifier: Apache-2.0

//! The step-load STRESS instrument.
//!
//! Short, intense load steps at
//! geometrically climbing offered rates locate the **maximum sustainable
//! throughput** (the industry headline for this procedure; TPC benchmarks
//! call their equivalent "maximum qualified throughput" — the highest rate
//! held inside a response-time constraint). On the latency-throughput curve
//! this is the knee: the point past which latency departs its plateau —
//! WHERE THE SYSTEM BREAKS is the instrument's one question.
//!
//! This is EXPLORATION, deliberately distinct from conformance: it earns
//! nothing, lives in its own artifact, never touches results.json, and its
//! vocabulary is deliberately CLASS-FREE — the volumetric class ladder
//! belongs to the measured class runs alone, and no class token appears in
//! the stress CLI, report, remark, or rendered chart (a knee measures
//! nothing about a class). No openEHR spec governs stress testing — our
//! own design/extension; the step envelope (p99 budget + a small error
//! tolerance) is standard load-testing methodology.
//!
//! Machinery-wise the instrument reuses ONLY runner-owned parts: the
//! open-loop arrival scheduler ([`crate::perf_run::window::run_window`],
//! coordinated-omission-free), the schedule's journey workload and corpus
//! seeding, and the re-checkable HDR-V2 records embedded per step.

use serde::{Deserialize, Serialize};

use crate::ixit::Environment;
use crate::perf::OperationMeasurement;
use crate::perf_run::client::PerfPrincipals;
use crate::perf_run::corpus::SeededCorpus;
use crate::perf_run::schedule::JourneyWorkload;
use crate::perf_run::window::run_window;

/// The stress envelope + ladder shape (all defaults flag-tunable).
#[derive(Debug, Clone)]
pub struct StressOptions {
    /// The first load step's offered rate (arrivals/s).
    pub start_rate: f64,
    /// The climb cap; reaching it without a breach reports `ladder_capped`.
    pub max_rate: f64,
    /// Warmup before each step's recorded hold (seconds).
    pub step_warmup_s: u64,
    /// Each step's recorded hold (seconds) — short and intense by design.
    pub step_hold_s: u64,
    /// Post-breach bisection refinements between last-good and breached.
    pub bisections: u32,
    /// The p99 budget every operation must hold per step (milliseconds).
    pub p99_budget_ms: f64,
    /// The error tolerance per step (fraction of requests) — standard
    /// load-testing practice (no zero-error conformance demand here).
    pub error_budget: f64,
}

impl Default for StressOptions {
    fn default() -> Self {
        Self {
            start_rate: 2.0,
            max_rate: 4096.0,
            step_warmup_s: 30,
            step_hold_s: 120,
            bisections: 3,
            p99_budget_ms: 1_000.0,
            error_budget: 0.001,
        }
    }
}

/// One executed step of the load ladder.
#[derive(Debug, Serialize, Deserialize)]
pub struct LoadStep {
    /// The scheduled offered rate (arrivals/s).
    pub rate: f64,
    /// The rate the generator actually sustained (arrivals/s).
    pub offered_load_sustained: f64,
    /// Per-operation records — encoded HDR V2 histograms, re-checkable.
    pub operations: Vec<OperationMeasurement>,
    /// Whether the step held the stress envelope.
    pub stable: bool,
    /// The named envelope breaches behind an unstable step.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub breaches: Vec<String>,
    /// The generator, not the SUT, was the bottleneck at this rate.
    pub generator_bound: bool,
    /// The step's resource telemetry (the shared measured-run sampler over
    /// this step's own warmup+hold window; no disk anchors — exploration
    /// stays light). Joins resource burn to offered rate: a breached rung
    /// shows WHERE it saturated. Optional by the ixit `containers`
    /// capability; absence never fails a step.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resources: Option<crate::perf::ResourcesRecord>,
}

/// The stress report — the committed exploration artifact. Never merged into
/// results.json; earns nothing.
#[derive(Debug, Serialize, Deserialize)]
pub struct StressReport {
    /// The corpus the stress ran on (a scale corpus key, e.g. `cnf.scale.10k`).
    pub corpus: String,
    /// The ixit environment the stress ran in.
    pub environment: Environment,
    /// Warmup per step (seconds).
    pub step_warmup_s: u64,
    /// Recorded hold per step (seconds).
    pub step_hold_s: u64,
    /// The p99 budget applied per step (milliseconds).
    pub p99_budget_ms: f64,
    /// The error tolerance applied per step (fraction).
    pub error_budget: f64,
    /// Every executed step, in execution order (ladder then bisection).
    pub steps: Vec<LoadStep>,
    /// The maximum sustainable throughput: the highest offered rate a stable
    /// step held inside the envelope (arrivals/s); 0 when even the first
    /// step breached.
    pub max_sustainable_throughput_per_s: f64,
    /// The climb reached `max_rate` without a breach.
    pub ladder_capped: bool,
    /// The generator topped out before the SUT did.
    pub generator_bound: bool,
    /// The human summary, incl. the explicit exploration disclaimer.
    pub remark: String,
}

/// Evaluate one step's records against the stress envelope.
fn step_breaches(
    operations: &[OperationMeasurement],
    options: &StressOptions,
) -> Result<Vec<String>, String> {
    let mut breaches = Vec::new();
    let (mut requests, mut errors) = (0_u64, 0_u64);
    for op in operations {
        requests = requests.saturating_add(op.requests);
        errors = errors.saturating_add(op.errors);
        // Re-derive from the decoded histogram — the same discipline as the
        // measured-run verdicts (the summary fields are never load-bearing).
        let histogram = op.decode_histogram()?;
        #[expect(
            clippy::as_conversions,
            clippy::cast_precision_loss,
            reason = "latencies << 2^52 µs"
        )]
        let p99_ms = histogram.value_at_quantile(0.99) as f64 / 1_000.0;
        if p99_ms > options.p99_budget_ms {
            breaches.push(format!(
                "{} p99 {p99_ms:.1}ms > budget {:.0}ms",
                op.operation, options.p99_budget_ms
            ));
        }
    }
    #[expect(
        clippy::as_conversions,
        clippy::cast_precision_loss,
        reason = "request counts << 2^52"
    )]
    let error_rate = if requests == 0 {
        1.0
    } else {
        errors as f64 / requests as f64
    };
    if error_rate > options.error_budget {
        breaches.push(format!(
            "error rate {error_rate:.4} > tolerance {:.4}",
            options.error_budget
        ));
    }
    Ok(breaches)
}

/// Run the geometric step-load ladder and locate the maximum sustainable
/// throughput.
///
/// # Errors
/// A message on schedule construction or aggregation failure (an unstable
/// step is a finding, never an error).
#[expect(
    clippy::too_many_lines,
    reason = "one linear procedure: climb → bisect → report"
)]
pub fn run_stress(
    principals: &PerfPrincipals,
    corpus: &SeededCorpus,
    workload: &JourneyWorkload<'_>,
    environment: &Environment,
    containers: Option<&crate::ixit::Containers>,
    options: &StressOptions,
    progress: &(dyn Fn(String) + Sync),
) -> Result<StressReport, String> {
    let mut steps: Vec<LoadStep> = Vec::new();
    let mut last_good: f64 = 0.0;
    let mut first_bad: Option<f64> = None;
    let mut generator_bound = false;
    let mut ladder_capped = false;
    if containers.is_none() {
        progress("resources: not sampled (ixit declares no `containers` block)".to_owned());
    }

    let run_step = |rate: f64,
                    steps: &mut Vec<LoadStep>,
                    generator_bound: &mut bool|
     -> Result<bool, String> {
        // Settle the maintenance debt the previous rungs' writes built up
        // BEFORE the rung, so autovacuum/analyze never fires inside a hold
        // and every rung starts from the same settled state.
        if let Some(c) = containers {
            if let Err(e) = crate::perf_run::resources::settle_maintenance(&c.db) {
                progress(format!("maintenance not settled: {e}"));
            }
        } else {
            progress("maintenance not settled (no ixit `containers` block)".to_owned());
        }
        progress(format!(
            "load step at {rate}/s ({}s hold)",
            options.step_hold_s
        ));
        // The sampler brackets this step's own warmup+hold window (phase
        // stamps derive from the step bounds).
        let sampler = containers.map(|c| {
            crate::perf_run::resources::ResourceSampler::start(
                c,
                options.step_warmup_s,
                options.step_hold_s,
            )
        });
        let window = run_window(
            principals,
            corpus,
            workload,
            rate,
            options.step_warmup_s,
            options.step_hold_s,
            progress,
        );
        let resources = sampler.and_then(|sampler| {
            let (series, notes) = sampler.stop();
            for note in notes {
                progress(note);
            }
            series
                .iter()
                .any(|s| !s.samples.is_empty())
                .then_some(crate::perf::ResourcesRecord {
                    sample_interval_s: crate::perf_run::resources::SAMPLE_INTERVAL.as_secs(),
                    containers: series,
                    disk: None,
                })
        });
        let window = window?;
        let breaches = step_breaches(&window.operations, options)?;
        let stable = breaches.is_empty() && !window.generator_bound;
        if window.generator_bound {
            *generator_bound = true;
            progress(format!(
                "generator bound at {rate}/s — the instrument, not the SUT, is the bottleneck"
            ));
        }
        // The verdict, live: what the instrument DECIDED about this rung —
        // never leave the console reading as a pass while the SUT sheds.
        let resource_note = resources.as_ref().map_or(String::new(), |r| {
            let peaks: Vec<String> = r
                .containers
                .iter()
                .map(|c| format!("{} peak {:.0}% cpu", c.role.label(), c.cpu_peak()))
                .collect();
            format!(", {}", peaks.join(", "))
        });
        progress(if stable {
            format!(
                "step {rate}/s: stable (sustained {:.1}/s{resource_note})",
                window.offered_load_sustained
            )
        } else {
            format!(
                "step {rate}/s: BREACHED (sustained {:.1}/s{resource_note}) — {}",
                window.offered_load_sustained,
                breaches.join("; ")
            )
        });
        steps.push(LoadStep {
            rate,
            offered_load_sustained: window.offered_load_sustained,
            operations: window.operations,
            stable,
            breaches,
            generator_bound: window.generator_bound,
            resources,
        });
        Ok(stable)
    };

    // The geometric climb.
    let mut rate = options.start_rate.max(0.5);
    loop {
        if rate > options.max_rate {
            ladder_capped = true;
            break;
        }
        let stable = run_step(rate, &mut steps, &mut generator_bound)?;
        if stable {
            last_good = last_good.max(rate);
            rate *= 2.0;
        } else {
            if !generator_bound {
                first_bad = Some(rate);
            }
            break;
        }
    }

    // Bisection refinement between the last stable and the breached rate.
    if let Some(mut bad) = first_bad {
        let mut good = last_good;
        for _ in 0..options.bisections {
            let mid = f64::midpoint(good, bad);
            if !(mid.is_finite() && mid > good && mid < bad) {
                break;
            }
            progress(format!(
                "bisecting between {good}/s (stable) and {bad}/s (breached)"
            ));
            if run_step(mid, &mut steps, &mut generator_bound)? {
                good = mid;
            } else {
                bad = mid;
            }
        }
        last_good = good;
    }

    // The rung recap — one line per executed step, so an operator reading
    // only the tail still sees the whole ladder's verdicts.
    for step in &steps {
        progress(format!(
            "recap: {:>7}/s {} {}",
            step.rate,
            if step.stable { "stable  " } else { "BREACHED" },
            step.breaches.first().map_or("", String::as_str),
        ));
    }

    let remark = format!(
        "Maximum sustainable throughput ≈ {last_good:.1} arrivals/s on the {} corpus \
         ({}s steps){}{}. Exploration only — never a conformance record; the first \
         breached rung past this rate is where the system leaves the envelope.",
        corpus.corpus,
        options.step_hold_s,
        if ladder_capped {
            " — ladder capped without a breach"
        } else {
            ""
        },
        if generator_bound {
            " — generator bound (the instrument topped out first)"
        } else {
            ""
        },
    );
    progress(remark.clone());

    Ok(StressReport {
        corpus: corpus.corpus.clone(),
        environment: environment.clone(),
        step_warmup_s: options.step_warmup_s,
        step_hold_s: options.step_hold_s,
        p99_budget_ms: options.p99_budget_ms,
        error_budget: options.error_budget,
        steps,
        max_sustainable_throughput_per_s: last_good,
        ladder_capped,
        generator_bound,
        remark,
    })
}

#[cfg(test)]
mod tests {
    use hdrhistogram::Histogram;

    use super::*;

    fn record(p99_us: u64, errors: u64) -> OperationMeasurement {
        // 95 fast samples + 5 at the target value, so the p99 rank lands on
        // the target (a single 1-in-100 outlier sits below the 0.99 rank).
        let mut h = Histogram::<u64>::new(3).unwrap();
        for _ in 0..95 {
            h.record(1_000).unwrap();
        }
        for _ in 0..5 {
            h.record(p99_us).unwrap();
        }
        OperationMeasurement::from_histogram("composition_read", &h, errors).unwrap()
    }

    #[test]
    fn the_envelope_breaches_on_p99_and_errors() {
        let options = StressOptions::default();
        assert!(
            step_breaches(&[record(20_000, 0)], &options)
                .unwrap()
                .is_empty()
        );
        let slow = step_breaches(&[record(3_000_000, 0)], &options).unwrap();
        assert!(slow.iter().any(|b| b.contains("p99")), "{slow:?}");
        let flaky = step_breaches(&[record(20_000, 5)], &options).unwrap();
        assert!(flaky.iter().any(|b| b.contains("error rate")), "{flaky:?}");
    }
}
