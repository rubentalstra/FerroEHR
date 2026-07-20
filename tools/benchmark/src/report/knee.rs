//! The knee/saturation series artefacts: the maximum-sustained-throughput
//! probe. The `hour` rate shape is driven at an ascending load-factor ladder,
//! each step on a short fixed window (open-loop, ramping, fixed duration per
//! step); the ladder stops at the first step past the SLO (p99 > 1 s) or the
//! error-rate flag (> 0.1%), and the last sustainable step is the knee.
//!
//! `knee.json` is the machine record; `KNEE.md` + `charts/knee.svg` are
//! generated from it, never hand-typed. Measured only — the honesty limitations
//! line names the single-run, same-host lower-bound caveat.

use std::path::Path;

use super::chart;
use super::json::SutBlock;
use crate::BenchError;

/// The p99 SLO ceiling: 1 s.
pub const P99_SLO_US: u64 = 1_000_000;
/// The error-rate ceiling: 0.1%.
pub const ERROR_RATE_SLO: f64 = 0.001;

/// Whether the ladder must stop after a step: p99 past the SLO **or** the error
/// rate past the flag. The boundary values
/// (exactly 1 s / exactly 0.1%) are still sustainable — the stop is on a strict
/// breach.
#[must_use]
pub fn ladder_should_stop(p99_us: u64, error_rate: f64) -> bool {
    p99_us > P99_SLO_US || error_rate > ERROR_RATE_SLO
}

/// One capacity step: the offered load factor and the measured response over the
/// step window.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct KneeStep {
    /// The arrival-rate load factor `L` offered at this step.
    pub load_factor: f64,
    /// Sustained requests/second measured over the step window.
    pub rps: f64,
    /// Error rate over `requests + errors` at this step.
    pub error_rate: f64,
    /// The overall p99 latency (µs) across every class at this step.
    pub p99_us: u64,
    /// Measured (post-warmup) requests at this step.
    pub requests: u64,
    /// Completed clinical events (business transactions) per minute at this
    /// step — the TPC-style throughput sustained at this load factor (checklist
    /// item 25b). `#[serde(default)]` for pre-25b `knee.json` (renders as
    /// unavailable). The ladder's stop condition stays req/latency-based; this
    /// is a reported-alongside figure only.
    #[serde(default)]
    pub events_per_min: f64,
    /// Worst dispatcher lag behind the planned schedule (ms). Above
    /// [`GENERATOR_BOUND_LAG_MS`] the step is generator-bound: the load
    /// generator could not keep its own schedule, so the step bounds the
    /// *instrument*, not the SUT.
    #[serde(default)]
    pub max_dispatch_lag_ms: u64,
}

/// Dispatch lag above which a step is flagged generator-bound (1 s — the same
/// magnitude as the SLO; a generator a full second late dominates every
/// CO-corrected sample).
pub const GENERATOR_BOUND_LAG_MS: u64 = 1_000;

impl KneeStep {
    /// Whether the load generator, not the SUT, bounded this step.
    #[must_use]
    pub fn generator_bound(&self) -> bool {
        self.max_dispatch_lag_ms > GENERATOR_BOUND_LAG_MS
    }
}

/// The knee/saturation machine record (`knee.json`).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct KneeResults {
    /// The SUT identity (mirrors `results.json`'s `sut` block).
    pub sut: SutBlock,
    /// The pre-seeded scale rung the ladder ran at.
    pub scale: String,
    /// Every executed ladder step, in ascending load-factor order (the step that
    /// breached the SLO, if any, is the last entry).
    pub steps: Vec<KneeStep>,
    /// The last sustainable step (highest `L` with p99 ≤ SLO and error ≤ flag),
    /// or `None` when even the first step saturated.
    pub knee: Option<KneeStep>,
    /// The SUT stopped answering HTTP after the breaching step — it died
    /// under load (e.g. OOM-killed) rather than merely saturating. A
    /// first-class finding, surfaced loudly in `KNEE.md`.
    #[serde(default)]
    pub sut_died: bool,
}

impl KneeResults {
    /// Pretty-print the machine record.
    ///
    /// # Errors
    /// [`serde_json::Error`] if serialization fails.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }
}

/// Write the knee artefact set into `out_dir`: `knee.json`, `KNEE.md`, and
/// `charts/knee.svg`.
///
/// # Errors
/// [`BenchError::Io`] on a filesystem failure, [`BenchError::Json`] on a
/// serialization failure.
pub fn write_all(results: &KneeResults, out_dir: &Path) -> Result<(), BenchError> {
    std::fs::create_dir_all(out_dir)?;
    std::fs::write(out_dir.join("knee.json"), results.to_json()?)?;
    std::fs::write(out_dir.join("KNEE.md"), render_markdown(results))?;

    let chart_dir = out_dir.join("charts");
    std::fs::create_dir_all(&chart_dir)?;
    let svg = chart::knee_chart(&chart_points(results));
    if !svg.is_empty() {
        std::fs::write(chart_dir.join("knee.svg"), svg)?;
    }
    Ok(())
}

/// Read a `knee.json` back into [`KneeResults`] (the `compare --knee-from` input).
///
/// # Errors
/// [`BenchError::Io`] if the file cannot be read, [`BenchError::Json`] if it
/// cannot be parsed.
pub fn from_file(path: &Path) -> Result<KneeResults, BenchError> {
    let text = std::fs::read_to_string(path)?;
    Ok(serde_json::from_str(&text)?)
}

/// The chart points `(rps, p99_us, load_factor)` in ladder order.
fn chart_points(results: &KneeResults) -> Vec<(f64, u64, f64)> {
    results
        .steps
        .iter()
        .map(|s| (s.rps, s.p99_us, s.load_factor))
        .collect()
}

/// Render `KNEE.md` for one knee run.
#[must_use]
pub fn render_markdown(r: &KneeResults) -> String {
    let mut m = String::new();
    m.push_str(&format!(
        "# Maximum sustained throughput (knee) — {}\n\n",
        r.sut.product_label
    ));
    m.push_str(&format!(
        "> Generated from `knee.json` (never hand-typed). Scale **{}**. The `hour` \
         rate shape is driven at an ascending load-factor ladder on short fixed \
         windows; the ladder stops at the first step past the SLO (p99 > 1 s) or \
         the 0.1% error-rate flag. Latencies are coordinated-omission-corrected \
         against planned send times.\n\n",
        r.scale
    ));

    match &r.knee {
        Some(step) => m.push_str(&format!(
            "**Knee: L = {} → {} at p99 {} µs** (the last sustainable step; \
             SLO p99 ≤ 1 s, error ≤ 0.1%){}.\n\n",
            step.load_factor,
            super::fmt_rate(step.rps),
            step.p99_us,
            // The business-transaction throughput sustained at the knee, when
            // the data is available (a 25b-era run); the SLO stays req/latency.
            if step.events_per_min > 0.0 {
                format!(
                    " — sustaining {:.1} clinical events/min",
                    step.events_per_min
                )
            } else {
                String::new()
            }
        )),
        None => m.push_str(
            "**No sustainable step:** even the first ladder step breached the SLO. \
             The knee is below the smallest offered load factor.\n\n",
        ),
    }

    if r.sut_died {
        m.push_str(
            "> [!WARNING]\n> **The SUT died under load** — after the breaching step it no \
             longer answered HTTP at all (a crash, e.g. OOM-killed; not mere saturation). \
             The knee above is where it *stopped surviving*, not where it merely slowed. \
             This is a finding about the SUT's overload behaviour.\n\n",
        );
    }
    m.push_str("## Ladder\n\n");
    m.push_str(
        "| L | req/s | error rate | p99 (µs) | requests | dispatch lag (ms) | verdict |\n|--:|--:|--:|--:|--:|--:|---|\n",
    );
    for step in &r.steps {
        let sustainable = !ladder_should_stop(step.p99_us, step.error_rate);
        let verdict = if step.generator_bound() {
            // The generator could not keep its own schedule — this step bounds
            // the instrument, never the SUT.
            "GENERATOR-BOUND (not a SUT limit)"
        } else if sustainable {
            "sustained"
        } else {
            "SLO breached"
        };
        m.push_str(&format!(
            "| {} | {:.1} | {:.3}% | {} | {} | {} | {} |\n",
            step.load_factor,
            step.rps,
            step.error_rate * 100.0,
            step.p99_us,
            step.requests,
            step.max_dispatch_lag_ms,
            verdict
        ));
    }
    m.push('\n');

    if !chart_points(r).is_empty() {
        m.push_str("![Knee — sustained req/s vs p99 latency](charts/knee.svg)\n\n");
    }

    m.push_str("## Limitations\n\n");
    m.push_str(
        "- **Single run per step** (no inter-run variance): the ≥5-run protocol \
         (benchmarking.md §4.4) is the publication step; these numbers are \
         indicative, not certified.\n",
    );
    m.push_str(
        "- **Same-host load generator:** the generator competes for CPU with the \
         SUT at high load, so the measured knee is a **lower bound** on the SUT's \
         real capacity — an isolated load generator would push it higher.\n",
    );
    m.push_str(
        "- Provisioning is re-applied idempotently at each step; scale seeding runs \
         once before the ladder.\n\n",
    );
    m
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
    use std::collections::BTreeMap;

    fn sut() -> SutBlock {
        SutBlock {
            name: "ehrbase-rs".to_owned(),
            kind: "ours".to_owned(),
            base_url: "http://x/v1".to_owned(),
            product_label: "ehrbase-rs 3.0.0".to_owned(),
            image_digests: BTreeMap::new(),
            versions: BTreeMap::new(),
        }
    }

    fn step(l: f64, rps: f64, err: f64, p99: u64, reqs: u64) -> KneeStep {
        KneeStep {
            load_factor: l,
            rps,
            error_rate: err,
            p99_us: p99,
            requests: reqs,
            events_per_min: 0.0,
            max_dispatch_lag_ms: 0,
        }
    }

    fn results() -> KneeResults {
        let s1 = step(1.0, 50.0, 0.0, 30_000, 3000);
        let s2 = step(2.0, 95.0, 0.0005, 80_000, 5700);
        let breach = step(4.0, 120.0, 0.0, 1_500_000, 7200);
        KneeResults {
            sut: sut(),
            scale: "10k".to_owned(),
            steps: vec![s1, s2.clone(), breach],
            knee: Some(s2),
            sut_died: false,
        }
    }

    #[test]
    fn stop_condition_honours_both_slos_and_boundaries() {
        assert!(!ladder_should_stop(500_000, 0.0));
        // p99 strictly past 1 s.
        assert!(ladder_should_stop(1_500_000, 0.0));
        assert!(
            !ladder_should_stop(P99_SLO_US, 0.0),
            "exactly 1 s is sustainable"
        );
        // error rate strictly past 0.1%.
        assert!(ladder_should_stop(5_000, 0.002));
        assert!(
            !ladder_should_stop(5_000, ERROR_RATE_SLO),
            "exactly 0.1% is sustainable"
        );
    }

    #[test]
    fn knee_json_round_trips() {
        let r = results();
        let json = r.to_json().expect("serialize");
        let back: KneeResults = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.sut.name, "ehrbase-rs");
        assert_eq!(back.scale, "10k");
        assert_eq!(back.steps.len(), 3);
        assert_eq!(back.knee, Some(step(2.0, 95.0, 0.0005, 80_000, 5700)));
        assert_eq!(back.steps[2].p99_us, 1_500_000);
    }

    #[test]
    fn markdown_carries_the_ladder_knee_and_limitations() {
        let md = render_markdown(&results());
        assert!(md.contains("Knee: L = 2"));
        assert!(md.contains("## Ladder"));
        assert!(md.contains("SLO breached"));
        assert!(md.contains("sustained"));
        assert!(md.contains("lower bound"));
        assert!(md.contains("charts/knee.svg"));
    }

    #[test]
    fn knee_headline_shows_events_per_min_when_available() {
        let mut r = results();
        let mut knee = r.knee.take().expect("knee present");
        knee.events_per_min = 42.5;
        r.knee = Some(knee);
        let md = render_markdown(&r);
        assert!(
            md.contains("sustaining 42.5 clinical events/min"),
            "the sustained line shows the business-transaction throughput\n{md}"
        );
        // Absent (pre-25b) data stays silent — the base fixture has 0.0.
        assert!(!render_markdown(&results()).contains("clinical events/min"));
    }

    #[test]
    fn markdown_states_no_sustainable_step() {
        let mut r = results();
        r.knee = None;
        r.steps = vec![step(1.0, 10.0, 0.0, 2_000_000, 100)];
        let md = render_markdown(&r);
        assert!(md.contains("No sustainable step"));
    }

    #[test]
    fn write_all_emits_the_artefact_set() {
        let dir = std::env::temp_dir().join(format!("bench-knee-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        write_all(&results(), &dir).expect("write");
        assert!(dir.join("knee.json").exists());
        assert!(dir.join("KNEE.md").exists());
        assert!(dir.join("charts/knee.svg").exists());
        let back = from_file(&dir.join("knee.json")).expect("read back");
        assert_eq!(back.steps.len(), 3);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
