//! Per-class latency recording with coordinated-omission correction
//! (`docs/design/benchmark/01-measurement.md` §1).
//!
//! One [`hdrhistogram::Histogram`] per [`OpClass`] at µs resolution / 3
//! significant digits. Latency is measured against the operation's **planned**
//! send time, never its actual send time: a saturated SUT that delays a send
//! cannot flatter its tail. Both the planned send and the actual completion are
//! offsets from the run-window start; a sample whose planned send falls inside
//! the warmup floor is discarded. Errors are counted per class and excluded
//! from the latency distribution.

use std::collections::BTreeMap;
use std::time::Duration;

use base64::Engine;
use hdrhistogram::Histogram;
use hdrhistogram::serialization::{Serializer, V2Serializer};

use crate::OpClass;

/// Histogram significant figures (register 01 §1 — 3 sig-digits).
const SIGFIG: u8 = 3;

/// The recordable ceiling in microseconds (6 h). Samples above are saturated to
/// this bound rather than dropped, so a pathological tail is still visible.
const MAX_RECORD_US: u64 = 6 * 3_600 * 1_000_000;

/// Summary statistics for one operation class, as emitted into `results.json`
/// (`docs/design/benchmark/01-measurement.md` §6 `classes` entry). Latencies
/// are microseconds; `histogram_b64` is the base64 of the `HdrHistogram` V2
/// serialization (the raw distribution, for offline re-analysis).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ClassSummary {
    /// Measured (non-warmup, non-error) sample count.
    pub count: u64,
    /// Errors observed for this class (excluded from the distribution).
    pub errors: u64,
    /// 50th percentile latency (µs).
    pub p50_us: u64,
    /// 90th percentile latency (µs).
    pub p90_us: u64,
    /// 99th percentile latency (µs).
    pub p99_us: u64,
    /// 99.9th percentile latency (µs).
    pub p999_us: u64,
    /// Maximum recorded latency (µs).
    pub max_us: u64,
    /// Base64 of the `HdrHistogram` V2 serialization.
    pub histogram_b64: String,
}

/// The latency + error recorder. One histogram per [`OpClass`], created on
/// first use; the warmup floor is applied at [`Recorder::record`] time.
#[derive(Debug)]
pub struct Recorder {
    hists: BTreeMap<OpClass, Histogram<u64>>,
    /// A merged histogram over every measured sample (all classes), so an
    /// overall percentile is a direct read rather than a max-of-class-p99s
    /// approximation — the capacity/knee series (register 01 §3) reads its p99
    /// from here.
    overall: Histogram<u64>,
    errors: BTreeMap<OpClass, u64>,
    warmup: Duration,
}

impl Default for Recorder {
    fn default() -> Self {
        Self::new()
    }
}

/// Build a histogram over the fixed µs bounds. The bounds are compile-time
/// constants known valid (`1 <= low`, `high >= 2*low`, `sigfig <= 5`), so the
/// error arm is unreachable.
fn make_hist() -> Histogram<u64> {
    match Histogram::new_with_bounds(1, MAX_RECORD_US, SIGFIG) {
        Ok(h) => h,
        Err(_) => unreachable!("histogram bounds (1, {MAX_RECORD_US}, {SIGFIG}) are valid"),
    }
}

impl Recorder {
    /// A fresh recorder with a zero warmup floor.
    #[must_use]
    pub fn new() -> Self {
        Self {
            hists: BTreeMap::new(),
            overall: make_hist(),
            errors: BTreeMap::new(),
            warmup: Duration::ZERO,
        }
    }

    /// Set the warmup floor: samples whose planned send is `< warmup` are
    /// discarded by [`Recorder::record`].
    pub fn set_warmup(&mut self, warmup: Duration) {
        self.warmup = warmup;
    }

    /// Record one completed operation. `planned_send` and `actual_completion`
    /// are both offsets from the run-window start; the recorded latency is the
    /// coordinated-omission-corrected `actual_completion - planned_send`. A
    /// sample whose `planned_send` is inside the warmup floor is discarded.
    pub fn record(&mut self, class: OpClass, planned_send: Duration, actual_completion: Duration) {
        if planned_send < self.warmup {
            return;
        }
        let latency = actual_completion.saturating_sub(planned_send);
        // Clamp to at least 1 µs (the histogram's low bound); a sub-µs or
        // zero latency still counts as a real, fast sample.
        let micros = u64::try_from(latency.as_micros())
            .unwrap_or(u64::MAX)
            .max(1);
        self.hists
            .entry(class)
            .or_insert_with(make_hist)
            .saturating_record(micros);
        self.overall.saturating_record(micros);
    }

    /// Record an error for a class (excluded from the latency distribution).
    /// Warmup filtering for errors is the caller's responsibility — it holds
    /// the planned-send time; the counter is unconditional here.
    pub fn error(&mut self, class: OpClass) {
        *self.errors.entry(class).or_insert(0) += 1;
    }

    /// Per-class summaries, keyed by [`OpClass::key`], for every class in report
    /// order (classes with no samples appear with zeroed statistics).
    #[must_use]
    pub fn summaries(&self) -> BTreeMap<&'static str, ClassSummary> {
        let mut out = BTreeMap::new();
        for class in OpClass::ALL {
            let errors = self.errors.get(&class).copied().unwrap_or(0);
            let summary = match self.hists.get(&class) {
                Some(hist) => ClassSummary {
                    count: hist.len(),
                    errors,
                    p50_us: hist.value_at_quantile(0.50),
                    p90_us: hist.value_at_quantile(0.90),
                    p99_us: hist.value_at_quantile(0.99),
                    p999_us: hist.value_at_quantile(0.999),
                    max_us: hist.max(),
                    histogram_b64: serialize_hist(hist),
                },
                None => ClassSummary {
                    count: 0,
                    errors,
                    p50_us: 0,
                    p90_us: 0,
                    p99_us: 0,
                    p999_us: 0,
                    max_us: 0,
                    histogram_b64: String::new(),
                },
            };
            out.insert(class.key(), summary);
        }
        out
    }

    /// Total measured (non-warmup, non-error) samples across all classes.
    #[must_use]
    pub fn total_measured(&self) -> u64 {
        self.hists.values().map(hdrhistogram::Histogram::len).sum()
    }

    /// Total errors across all classes.
    #[must_use]
    pub fn total_errors(&self) -> u64 {
        self.errors.values().sum()
    }

    /// The 99th-percentile latency (µs) across **all** measured classes, read
    /// from the merged overall histogram — the capacity/knee series' SLO probe
    /// (register 01 §3). Zero when nothing measured.
    #[must_use]
    pub fn overall_p99_us(&self) -> u64 {
        self.overall.value_at_quantile(0.99)
    }
}

/// Serialize a histogram to base64 of its `HdrHistogram` V2 form. On the
/// (writer-only) serialization failure the entry degrades to an empty string
/// rather than aborting the run.
fn serialize_hist(hist: &Histogram<u64>) -> String {
    let mut buf = Vec::new();
    if V2Serializer::new().serialize(hist, &mut buf).is_ok() {
        base64::engine::general_purpose::STANDARD.encode(&buf)
    } else {
        String::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hdrhistogram::serialization::Deserializer;

    fn ms(n: u64) -> Duration {
        Duration::from_millis(n)
    }

    #[test]
    fn co_correction_measures_against_planned_send() {
        let mut r = Recorder::new();
        // Planned at 1000 ms, actually completed at 1200 ms → 200 ms latency,
        // regardless of when the send truly went out.
        r.record(OpClass::EhrRead, ms(1000), ms(1200));
        let s = &r.summaries()["ehr-read"];
        assert_eq!(s.count, 1);
        // 200 ms = 200_000 µs (within 3-sig-fig tolerance).
        assert!(
            (199_000..=201_000).contains(&s.p50_us),
            "p50 {} not ~200ms",
            s.p50_us
        );
    }

    #[test]
    fn warmup_samples_are_discarded() {
        let mut r = Recorder::new();
        r.set_warmup(ms(500));
        r.record(OpClass::EhrRead, ms(100), ms(150)); // planned in warmup → dropped
        r.record(OpClass::EhrRead, ms(600), ms(650)); // measured
        assert_eq!(r.total_measured(), 1);
        assert_eq!(r.summaries()["ehr-read"].count, 1);
    }

    #[test]
    fn percentile_buckets_are_correct() {
        let mut r = Recorder::new();
        // 100 samples: 99 at 10 ms, 1 at 1000 ms → p50≈10ms, max≈1000ms.
        for _ in 0..99 {
            r.record(OpClass::CompCreateSmall, ms(0), ms(10));
        }
        r.record(OpClass::CompCreateSmall, ms(0), ms(1000));
        let s = &r.summaries()["comp-create-small"];
        assert_eq!(s.count, 100);
        assert!((9_000..=11_000).contains(&s.p50_us), "p50 {}", s.p50_us);
        assert!(s.max_us >= 990_000, "max {}", s.max_us);
    }

    #[test]
    fn errors_counted_and_excluded() {
        let mut r = Recorder::new();
        r.record(OpClass::AqlWard, ms(0), ms(5));
        r.error(OpClass::AqlWard);
        r.error(OpClass::AqlWard);
        let s = &r.summaries()["aql-ward"];
        assert_eq!(s.count, 1);
        assert_eq!(s.errors, 2);
        assert_eq!(r.total_errors(), 2);
    }

    #[test]
    fn histogram_b64_round_trips() {
        let mut r = Recorder::new();
        for i in 1..=50u64 {
            r.record(OpClass::CompReadLatest, ms(0), ms(i));
        }
        let s = &r.summaries()["comp-read-latest"];
        assert!(!s.histogram_b64.is_empty());
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(&s.histogram_b64)
            .expect("valid base64");
        let restored: Histogram<u64> = Deserializer::new()
            .deserialize(&mut std::io::Cursor::new(bytes))
            .expect("valid V2 histogram");
        assert_eq!(restored.len(), 50);
    }

    #[test]
    fn overall_p99_merges_every_class() {
        let mut r = Recorder::new();
        // 80 fast samples across two classes at 10 ms + 20 slow at 2 s (100
        // total): each fast class's own p99 is 10 ms, but the *merged* p99 (the
        // 99th of 100) lands on the 2 s tail — the merge is what the knee reads.
        for _ in 0..40 {
            r.record(OpClass::CompReadLatest, ms(0), ms(10));
        }
        for _ in 0..40 {
            r.record(OpClass::AqlWard, ms(0), ms(10));
        }
        for _ in 0..20 {
            r.record(OpClass::CompCreateLarge, ms(0), ms(2000));
        }
        // Each fast class alone stays at 10 ms.
        assert!(r.summaries()["comp-read-latest"].p99_us <= 11_000);
        let p99 = r.overall_p99_us();
        assert!(
            p99 >= 1_900_000,
            "overall p99 {p99} µs should reflect the merged 2 s tail"
        );
    }

    #[test]
    fn overall_p99_is_zero_when_empty() {
        let r = Recorder::new();
        assert_eq!(r.overall_p99_us(), 0);
    }

    #[test]
    fn empty_classes_summarize_to_zero() {
        let r = Recorder::new();
        let s = r.summaries();
        assert_eq!(s.len(), OpClass::ALL.len());
        assert_eq!(s["opt-upload"].count, 0);
        assert!(s["opt-upload"].histogram_b64.is_empty());
    }
}
