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

use std::collections::{BTreeMap, HashMap};
use std::time::Duration;

use base64::Engine;
use hdrhistogram::Histogram;
use hdrhistogram::serialization::{Serializer, V2Serializer};

use crate::OpClass;
use crate::model::event::{ClinicalEvent, EventInstance};

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
    /// Generator-side schedule-dependency misses (never attributed to the
    /// SUT; reported beside, not inside, the error rate).
    #[serde(default)]
    pub dep_misses: u64,
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
    dep_misses: BTreeMap<OpClass, u64>,
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
            dep_misses: BTreeMap::new(),
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
    /// Count a SERVER-attributed error (unexpected status / transport /
    /// malformed success body). Warmup-gated exactly like [`Recorder::record`],
    /// so successes and errors face the same discard window.
    pub fn error(&mut self, class: OpClass, planned_send: Duration) {
        if planned_send < self.warmup {
            return;
        }
        *self.errors.entry(class).or_insert(0) += 1;
    }

    /// Count a GENERATOR-side schedule-dependency miss (a prerequisite id
    /// never arrived) — reported beside, never inside, the server error rate.
    /// Warmup-gated like [`Recorder::record`].
    pub fn dep_miss(&mut self, class: OpClass, planned_send: Duration) {
        if planned_send < self.warmup {
            return;
        }
        *self.dep_misses.entry(class).or_insert(0) += 1;
    }

    /// Per-class summaries, keyed by [`OpClass::key`], for every class in report
    /// order (classes with no samples appear with zeroed statistics).
    #[must_use]
    pub fn summaries(&self) -> BTreeMap<&'static str, ClassSummary> {
        let mut out = BTreeMap::new();
        for class in OpClass::ALL {
            let errors = self.errors.get(&class).copied().unwrap_or(0);
            let dep_misses = self.dep_misses.get(&class).copied().unwrap_or(0);
            let summary = match self.hists.get(&class) {
                Some(hist) => ClassSummary {
                    count: hist.len(),
                    errors,
                    dep_misses,
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
                    dep_misses,
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

/// Per event-class business-transaction tally (checklist item 25b): a clinical
/// event is *attempted* if its last step landed in the measurement window, and
/// *completed* only if every one of its steps succeeded.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct EventSummary {
    /// Occurrences whose last step landed in the measurement window.
    pub attempted: u64,
    /// Occurrences where every step succeeded.
    pub completed: u64,
}

/// The per-occurrence progress of one clinical-event business transaction.
#[derive(Debug)]
struct InstanceState {
    class: ClinicalEvent,
    boundary_at: Duration,
    steps: u32,
    ok: u32,
}

/// Accumulates clinical-event (business-transaction) completion across the
/// open-loop dispatch (checklist item 25b). The steps of one occurrence run in
/// separate tasks and arrive as independent samples, so completion is tracked
/// per occurrence keyed by its schedule-unique [`EventInstance::id`]: the
/// occurrence is *completed* only when every one of its
/// [`EventInstance::steps`] succeeded (a step that errored or was never
/// dispatched — an excluded template — leaves `ok < steps`). The warmup floor
/// is applied per occurrence by its LAST step's planned send
/// ([`EventInstance::boundary_at`]), symmetric with the per-request warmup
/// discard in [`Recorder::record`].
#[derive(Debug, Default)]
pub struct EventLedger {
    instances: HashMap<u64, InstanceState>,
    warmup: Duration,
}

impl EventLedger {
    /// A fresh ledger with a zero warmup floor.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the warmup floor: an occurrence whose last step's planned send is
    /// `< warmup` is discarded whole by [`EventLedger::summaries`].
    pub fn set_warmup(&mut self, warmup: Duration) {
        self.warmup = warmup;
    }

    /// Fold one step's outcome into its occurrence. `ok` is whether the step was
    /// a measured success (expected status); an errored or missing step keeps
    /// the occurrence short of completion.
    pub fn observe(&mut self, event: EventInstance, ok: bool) {
        let st = self.instances.entry(event.id).or_insert(InstanceState {
            class: event.class,
            boundary_at: event.boundary_at,
            steps: event.steps,
            ok: 0,
        });
        if ok {
            st.ok += 1;
        }
    }

    /// Per event-class attempted/completed tallies, applying the warmup rule.
    /// An occurrence whose last step is inside the warmup floor is discarded;
    /// otherwise it is attempted, and completed iff every step succeeded.
    /// Keyed in catalogue order (the derived [`ClinicalEvent`] `Ord`).
    #[must_use]
    pub fn summaries(&self) -> BTreeMap<ClinicalEvent, EventSummary> {
        let mut out: BTreeMap<ClinicalEvent, EventSummary> = BTreeMap::new();
        for st in self.instances.values() {
            if st.boundary_at < self.warmup {
                continue;
            }
            let entry = out.entry(st.class).or_default();
            entry.attempted += 1;
            if st.ok == st.steps {
                entry.completed += 1;
            }
        }
        out
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
        r.error(OpClass::AqlWard, ms(0));
        r.error(OpClass::AqlWard, ms(0));
        let s = &r.summaries()["aql-ward"];
        assert_eq!(s.count, 1);
        assert_eq!(s.errors, 2);
        assert_eq!(r.total_errors(), 2);
    }

    #[test]
    fn errors_and_misses_face_the_same_warmup_window_and_split() {
        let mut r = Recorder::new();
        r.set_warmup(ms(10));
        // Inside the warmup window: discarded on BOTH sides, symmetrically.
        r.error(OpClass::AqlWard, ms(1));
        r.dep_miss(OpClass::AqlWard, ms(1));
        // Measured window: one server error, one generator miss — separate.
        r.error(OpClass::AqlWard, ms(20));
        r.dep_miss(OpClass::AqlWard, ms(20));
        let s = &r.summaries()["aql-ward"];
        assert_eq!(s.errors, 1, "server errors exclude the warmup window");
        assert_eq!(s.dep_misses, 1, "misses split from errors, same window");
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

    // ── EventLedger (business-transaction completion, checklist item 25b) ──────

    fn instance(id: u64, class: ClinicalEvent, steps: u32, boundary: Duration) -> EventInstance {
        EventInstance {
            class,
            id,
            steps,
            boundary_at: boundary,
        }
    }

    #[test]
    fn event_completes_only_when_every_step_succeeds() {
        let mut l = EventLedger::new();
        let ev = instance(1, ClinicalEvent::Admission, 3, ms(500));
        // Three steps of one occurrence, all succeed → the occurrence completes.
        l.observe(ev, true);
        l.observe(ev, true);
        l.observe(ev, true);
        let s = l.summaries();
        let a = s[&ClinicalEvent::Admission];
        assert_eq!(a.attempted, 1);
        assert_eq!(a.completed, 1);
    }

    #[test]
    fn any_step_failure_leaves_the_event_incomplete() {
        let mut l = EventLedger::new();
        let ev = instance(1, ClinicalEvent::ChartReview, 3, ms(500));
        l.observe(ev, true);
        l.observe(ev, false); // one failed step
        l.observe(ev, true);
        let s = l.summaries();
        let c = s[&ClinicalEvent::ChartReview];
        assert_eq!(c.attempted, 1, "still attempted");
        assert_eq!(c.completed, 0, "a failed step blocks completion");
    }

    #[test]
    fn a_missing_step_blocks_completion() {
        // An excluded template drops a step at dispatch (never observed), so the
        // occurrence can never reach ok == steps.
        let mut l = EventLedger::new();
        let ev = instance(1, ClinicalEvent::Discharge, 2, ms(500));
        l.observe(ev, true); // only one of the two steps ever arrives
        let s = l.summaries();
        let d = s[&ClinicalEvent::Discharge];
        assert_eq!(d.attempted, 1);
        assert_eq!(d.completed, 0);
    }

    #[test]
    fn warmup_discards_an_event_by_its_last_step() {
        let mut l = EventLedger::new();
        l.set_warmup(ms(500));
        // Occurrence 1: last step at 400 ms (< warmup) → discarded whole, even
        // though its steps succeeded.
        let boot = instance(1, ClinicalEvent::Admission, 2, ms(400));
        l.observe(boot, true);
        l.observe(boot, true);
        // Occurrence 2: last step at 600 ms (≥ warmup) → measured + completed.
        let measured = instance(2, ClinicalEvent::Admission, 2, ms(600));
        l.observe(measured, true);
        l.observe(measured, true);
        let s = l.summaries();
        let a = s[&ClinicalEvent::Admission];
        assert_eq!(
            a.attempted, 1,
            "only the post-warmup occurrence is attempted"
        );
        assert_eq!(a.completed, 1);
    }

    #[test]
    fn ledger_tallies_are_per_class() {
        let mut l = EventLedger::new();
        let vitals = instance(1, ClinicalEvent::ShiftVitals, 1, ms(100));
        let meds = instance(2, ClinicalEvent::MedicationRound, 1, ms(100));
        l.observe(vitals, true);
        l.observe(meds, false);
        let s = l.summaries();
        assert_eq!(s[&ClinicalEvent::ShiftVitals].completed, 1);
        assert_eq!(s[&ClinicalEvent::MedicationRound].attempted, 1);
        assert_eq!(s[&ClinicalEvent::MedicationRound].completed, 0);
    }
}
