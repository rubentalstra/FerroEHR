//! The open-loop arrival schedule: a deterministic, seeded expansion of the
//! ward + event catalogue into a time-ordered list of [`PlannedOp`]s with
//! pre-rendered payloads (our own workload-model design; no openEHR spec
//! governs it).
//!
//! Discipline enforced here:
//! - **Determinism.** A single seeded [`StdRng`] is drawn in a fixed order
//!   (patients by index, events in [`ClinicalEvent::MEASURED`] order, occurrences
//!   in sequence), so the same `WorkloadSpec` yields a byte-identical schedule.
//! - **Ordering invariant.** Every patient's `CreateEhr` is its earliest op, and
//!   every read/update/query of composition or directory state is scheduled
//!   strictly after the op that creates that state — standing patients are
//!   bootstrapped in the warmup window, new admits at their (measured) admission
//!   time. See the [`event`](crate::model::event) module NOTE.
//! - **Arrival shape.** `hour`/`smoke` spread occurrences uniformly over the
//!   active window; `day` samples from the diurnal weight curve (peaks
//!   ~08:00/14:00, bumps 07:00/15:00/23:00, night trough).

use std::time::Duration;

use rand::RngExt;
use rand::SeedableRng;
use rand::rngs::StdRng;
use serde_json::Value;

use crate::model::WorkloadSpec;
use crate::model::event::{self, ClinicalEvent, EventInstance, Step};
use crate::model::ward::{Patient, Role, Ward};
use crate::render::{self, VaryParams};
use crate::{Action, BenchError, PlannedOp, Profile, TemplateKind};

/// Spacing between the ops of a single event (a clinical event's requests
/// arrive near-simultaneously, but ordered).
const STEP_SPACING: Duration = Duration::from_millis(1);

/// Render a composition via the constraint-aware path (templates are prepared
/// once in [`Builder::new`], where `preflight` has already proven every
/// committed template renderable, so a `Null` here would only follow a genuine
/// render fault).
fn render_composition(template: TemplateKind, params: &VaryParams) -> Value {
    render::composition(template, params).unwrap_or(Value::Null)
}

/// Patient-scoped AQL (E5): CONTAINS OBSERVATION + `ehr_id` filter + ORDER BY +
/// LIMIT — the `{{ehr_id}}` placeholder is substituted by the driver. Valid
/// AQL 1.1 (orderByClause precedes limitClause, `AqlParser.g4`); the form is the
/// ECC query suite's EHR-scoped CONTAINS pattern. The ORDER BY path is also
/// projected: AQL 1.1 does not require that, but upstream `EHRbase` rejects an
/// ORDER BY path absent from SELECT ("Not implemented", observed live against
/// 2.34.0) — projecting it keeps the byte-identical request executable on both
/// engines, and a chart review listing the event time is the realistic query
/// anyway.
pub const PATIENT_AQL: &str = "SELECT c/uid/value, c/name/value, \
     c/context/start_time/value FROM EHR e \
     CONTAINS COMPOSITION c CONTAINS OBSERVATION o \
     WHERE e/ehr_id/value = '{{ehr_id}}' \
     ORDER BY c/context/start_time/value DESC LIMIT 20";

/// Ward-population AQL (E8): no `ehr_id` filter, ORDER BY + LIMIT — the ECC
/// `AqlAdvanced` form, accepted by both engines.
pub const WARD_AQL: &str = "SELECT e/ehr_id/value FROM EHR e ORDER BY e/ehr_id/value ASC LIMIT 50";

/// Diurnal-curve resolution (5-minute buckets over the compressed day).
const DIURNAL_BUCKETS: usize = 288;

/// Build the full arrival schedule for a spec + ward, sorted by planned time.
///
/// # Errors
/// [`BenchError`] if a payload skeleton cannot be read or parsed.
pub fn build_ops(
    spec: &WorkloadSpec,
    ward: &Ward,
    window: Duration,
    warmup: Duration,
) -> Result<Vec<PlannedOp>, BenchError> {
    let mut builder = Builder::new(spec, window, warmup)?;
    for patient in &ward.patients {
        builder.schedule_patient(patient);
    }
    let mut ops = builder.ops;
    ops.sort_by_key(|op| op.at);
    Ok(ops)
}

struct Builder {
    profile: Profile,
    load_factor: f64,
    window_s: f64,
    warmup_s: f64,
    rng: StdRng,
    seed: u64,
    diurnal_cdf: Vec<f64>,
    ops: Vec<PlannedOp>,
    /// Monotonic id assigned to each emitted clinical-event occurrence, so the
    /// driver's event ledger can key completion per business transaction
    /// (checklist item 25b). Deterministic: patients/events/occurrences are
    /// iterated in a fixed order.
    next_event_id: u64,
}

impl Builder {
    fn new(spec: &WorkloadSpec, window: Duration, warmup: Duration) -> Result<Self, BenchError> {
        // Prepare every template once (WebTemplate build + faithfulness gate) so
        // a template that cannot render a committed payload surfaces as a build
        // error, never a silently-null payload in the hot loop. Renders
        // themselves are a cached lookup + `to_flat`/jitter/`from_flat`.
        render::preflight()?;
        // Validate the auxiliary fixtures once so a missing/corrupt file surfaces
        // as a build error rather than a silently-null payload in the hot loop.
        conformance::testdata::fixtures::read_from("ehr-status.valid", render::EHR_STATUS_FIXTURE)
            .map_err(|e| BenchError::Fixture(e.to_string()))?;
        conformance::testdata::fixtures::read_from(
            "contribution.valid",
            render::CONTRIBUTION_ENVELOPE,
        )
        .map_err(|e| BenchError::Fixture(e.to_string()))?;
        Ok(Self {
            profile: spec.profile,
            load_factor: spec.load_factor,
            window_s: window.as_secs_f64().max(1.0),
            warmup_s: warmup.as_secs_f64(),
            rng: StdRng::seed_from_u64(spec.seed),
            seed: spec.seed,
            diurnal_cdf: diurnal_cdf(),
            ops: Vec::new(),
            next_event_id: 0,
        })
    }

    fn schedule_patient(&mut self, patient: &Patient) {
        match patient.role {
            Role::Standing => {
                // Bootstrap in warmup; measured events over the whole window.
                let boot = self.warmup_pick();
                self.emit_event(patient, ClinicalEvent::Admission, boot);
                self.emit_measured(patient, self.warmup_s, self.window_s, 1.0);
            }
            Role::Discharged => {
                let boot = self.warmup_pick();
                self.emit_event(patient, ClinicalEvent::Admission, boot);
                // Discharged in the last ~40% of the measured window.
                let span = self.window_s - self.warmup_s;
                let discharge = self.warmup_s + span * self.rng.random_range(0.6..0.95);
                let active_fraction = (discharge - self.warmup_s) / span.max(1.0);
                self.emit_measured(patient, self.warmup_s, discharge, active_fraction);
                self.emit_event(patient, ClinicalEvent::Discharge, secs(discharge));
            }
            Role::NewAdmit => {
                // Admitted in the first half of the measured window (measured).
                let span = self.window_s - self.warmup_s;
                let admit = self.warmup_s + span * self.rng.random_range(0.0..0.5);
                self.emit_event(patient, ClinicalEvent::Admission, secs(admit));
                // Dependent events strictly after the admission sequence ends.
                let admit_end = admit + admission_span_s();
                let active_fraction = (self.window_s - admit_end) / span.max(1.0);
                self.emit_measured(patient, admit_end, self.window_s, active_fraction.max(0.0));
            }
        }
    }

    /// Emit the per-patient measured events (E2–E8) over `[lo, hi)` seconds,
    /// counts scaled by `active_fraction`.
    fn emit_measured(&mut self, patient: &Patient, lo: f64, hi: f64, active_fraction: f64) {
        for event in ClinicalEvent::MEASURED {
            let count = self.measured_count(event, active_fraction);
            for _ in 0..count {
                let at = self.sample_time(lo, hi);
                self.emit_event(patient, event, at);
            }
        }
    }

    /// Expand one event occurrence into ops at `base` + step spacing, including
    /// the probabilistic follow-ups and contribution batch sizing.
    fn emit_event(&mut self, patient: &Patient, event: ClinicalEvent, base: Duration) {
        let mut steps = event.steps();
        match event {
            ClinicalEvent::MedicationRound if self.rng.random_bool(event::MED_CORRECTION_PROB) => {
                // A missed-dose correction re-versions the medication composition
                // the round just created (the target exists for the driver).
                steps.push(Step::UpdateComposition {
                    template: TemplateKind::CkmMedicationOrder,
                });
            }
            ClinicalEvent::CarePlan if self.rng.random_bool(event::DIR_UPDATE_PROB) => {
                steps.push(Step::UpdateDirectory);
            }
            _ => {}
        }
        // One business-transaction occurrence: a schedule-unique id, the step
        // count (completion denominator), and the LAST step's planned time (the
        // warmup-boundary discriminator for the whole transaction).
        let id = self.next_event_id;
        self.next_event_id += 1;
        let step_count = u32::try_from(steps.len()).unwrap_or(u32::MAX);
        let boundary_at = base + STEP_SPACING * step_count.saturating_sub(1);
        let instance = EventInstance {
            class: event,
            id,
            steps: step_count,
            boundary_at,
        };
        for (i, step) in steps.into_iter().enumerate() {
            let at = base + STEP_SPACING * u32::try_from(i).unwrap_or(u32::MAX);
            let action = self.render_action(patient, step, at);
            self.ops.push(PlannedOp {
                at,
                class: step.op_class(),
                patient: patient.index,
                action,
                event: instance,
            });
        }
    }

    /// Build the concrete [`Action`] (with rendered payload) for a step.
    fn render_action(&mut self, patient: &Patient, step: Step, at: Duration) -> Action {
        let params = self.params(patient, at);
        match step {
            Step::CreateEhr => {
                let status = render::ehr_status(&patient.subject_id, self.seed)
                    .unwrap_or_else(|_| Value::Null);
                Action::CreateEhr { status }
            }
            Step::ReadEhr => Action::ReadEhr,
            Step::CreateComposition { template, .. } => Action::CreateComposition {
                template,
                payload: render_composition(template, &params),
            },
            Step::UpdateComposition { template } => Action::UpdateComposition {
                template,
                payload: render_composition(template, &params),
            },
            Step::ReadLatest => Action::ReadLatestComposition,
            Step::ReadVersion => Action::ReadCompositionVersion,
            Step::Contribution { template } => {
                let n = self.rng.random_range(1..=3);
                let payload =
                    render::contribution(template, &params, n).unwrap_or_else(|_| Value::Null);
                Action::CommitContribution { template, payload }
            }
            Step::AqlPatient => Action::AqlPatient {
                query: PATIENT_AQL.to_owned(),
            },
            Step::AqlWard => Action::AqlWard {
                query: WARD_AQL.to_owned(),
            },
            Step::ReadDirectory => Action::ReadDirectory,
            Step::UpdateDirectory => Action::UpdateDirectory {
                payload: render::folder(&params),
            },
            Step::ReadHistory => Action::ReadRevisionHistory,
            Step::UpdateStatus => {
                let payload = render::ehr_status(&patient.subject_id, self.seed)
                    .unwrap_or_else(|_| Value::Null);
                Action::UpdateStatus { payload }
            }
            Step::UploadOpt { template } => Action::UploadOpt { template },
            Step::ListTemplates => Action::ListTemplates,
        }
    }

    fn params(&self, patient: &Patient, at: Duration) -> VaryParams {
        VaryParams {
            subject_id: patient.subject_id.clone(),
            composer: patient.composer.clone(),
            event_time: sim_time(at, self.window_s),
            seed: self.seed,
        }
    }

    /// Draw the occurrence count for an event over the patient's active window.
    fn measured_count(&mut self, event: ClinicalEvent, active_fraction: f64) -> u32 {
        match self.profile {
            Profile::Smoke => event.smoke_count(),
            Profile::Hour | Profile::Day => {
                let expected =
                    event.rate_per_patient_day() * self.load_factor * active_fraction.max(0.0);
                self.draw_count(expected)
            }
        }
    }

    /// A count with the fractional expectation preserved via a Bernoulli draw.
    /// `base` is a non-negative, small floored expectation, so the cast to u32
    /// neither truncates meaningfully nor loses a sign.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    fn draw_count(&mut self, expected: f64) -> u32 {
        let base = expected.floor();
        let frac = (expected - base).clamp(0.0, 1.0);
        let mut n = base.max(0.0) as u32;
        if self.rng.random_bool(frac) {
            n += 1;
        }
        n
    }

    /// A warmup-window base time for a bootstrap sequence (leaves headroom for
    /// the step spacing so all bootstrap ops stay inside the warmup floor).
    fn warmup_pick(&mut self) -> Duration {
        if self.warmup_s <= 0.0 {
            return Duration::ZERO;
        }
        secs(self.rng.random_range(0.0..self.warmup_s * 0.3))
    }

    /// Sample a planned time in `[lo, hi)` seconds per the profile arrival shape.
    fn sample_time(&mut self, lo: f64, hi: f64) -> Duration {
        if hi <= lo {
            return secs(lo);
        }
        match self.profile {
            Profile::Day => {
                for _ in 0..8 {
                    let tau = self.sample_diurnal();
                    let candidate = self.warmup_s + tau * (self.window_s - self.warmup_s);
                    if candidate >= lo && candidate < hi {
                        return secs(candidate);
                    }
                }
                secs(lo + self.rng.random_range(0.0..1.0) * (hi - lo))
            }
            Profile::Hour | Profile::Smoke => {
                secs(lo + self.rng.random_range(0.0..1.0) * (hi - lo))
            }
        }
    }

    /// Inverse-transform sample the diurnal curve → a day fraction in `[0, 1)`.
    fn sample_diurnal(&mut self) -> f64 {
        let u: f64 = self.rng.random_range(0.0..1.0);
        // The CDF is monotone; a linear scan is fine at 288 buckets.
        let mut i = 0;
        while i + 1 < self.diurnal_cdf.len() && self.diurnal_cdf[i + 1] < u {
            i += 1;
        }
        (i as f64 + 0.5) / DIURNAL_BUCKETS as f64
    }
}

/// The total span (seconds) of the admission sequence's step spacing.
fn admission_span_s() -> f64 {
    let steps = u32::try_from(event::admission_steps().len()).unwrap_or(u32::MAX);
    (STEP_SPACING * steps).as_secs_f64() + 0.001
}

fn secs(s: f64) -> Duration {
    Duration::from_secs_f64(s.max(0.0))
}

/// Format a within-window offset as an RFC 3339 time along the compressed day
/// (fixed base date 2024-06-01; deterministic integer math, no timezone drift).
/// `tau` is clamped to `[0, 1)`, so the product is a bounded, non-negative
/// millisecond-of-day well within u64.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn sim_time(at: Duration, window_s: f64) -> String {
    let tau = (at.as_secs_f64() / window_s.max(1.0)).clamp(0.0, 0.999_999);
    let day_ms = (tau * 86_400_000.0) as u64;
    let ms = day_ms % 1000;
    let total_s = day_ms / 1000;
    let hh = total_s / 3600;
    let mm = (total_s % 3600) / 60;
    let ss = total_s % 60;
    format!("2024-06-01T{hh:02}:{mm:02}:{ss:02}.{ms:03}Z")
}

/// The normalized cumulative distribution of the diurnal weight curve.
/// `cdf[i]` is the cumulative weight up to bucket `i`.
fn diurnal_cdf() -> Vec<f64> {
    let mut weights = Vec::with_capacity(DIURNAL_BUCKETS);
    let mut total = 0.0;
    for i in 0..DIURNAL_BUCKETS {
        let tau = (i as f64 + 0.5) / DIURNAL_BUCKETS as f64;
        let w = diurnal_weight(tau);
        weights.push(w);
        total += w;
    }
    let mut cdf = Vec::with_capacity(DIURNAL_BUCKETS + 1);
    let mut acc = 0.0;
    cdf.push(0.0);
    for w in weights {
        acc += w / total;
        cdf.push(acc);
    }
    cdf
}

/// The diurnal activity weight at day fraction `tau`: a low
/// night base with morning (08:00) and afternoon (14:00) peaks and shift-change
/// bumps at 07:00/15:00/23:00.
fn diurnal_weight(tau: f64) -> f64 {
    let gauss = |mu: f64, sigma: f64| (-((tau - mu).powi(2)) / (2.0 * sigma * sigma)).exp();
    let h = |hour: f64| hour / 24.0;
    0.15 + 1.00 * gauss(h(8.0), 0.030)
        + 0.90 * gauss(h(14.0), 0.030)
        + 0.40 * gauss(h(7.0), 0.020)
        + 0.40 * gauss(h(15.0), 0.020)
        + 0.35 * gauss(h(23.0), 0.020)
}

#[cfg(test)]
#[allow(
    clippy::panic,
    clippy::print_stdout,
    clippy::print_stderr,
    let_underscore_drop
)] // test assertions/diagnostics/fixtures
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::OpClass;

    fn spec(profile: Profile, ward: usize, seed: u64) -> WorkloadSpec {
        WorkloadSpec {
            profile,
            ward_size: ward,
            load_factor: 1.0,
            seed,
        }
    }

    fn build(spec: &WorkloadSpec) -> Vec<PlannedOp> {
        let ward = Ward::new(spec);
        build_ops(spec, &ward, Duration::from_hours(1), Duration::from_mins(5))
            .expect("schedule builds")
    }

    #[test]
    fn schedule_is_sorted_and_nonempty() {
        let ops = build(&spec(Profile::Hour, 20, 1));
        assert!(!ops.is_empty());
        assert!(
            ops.windows(2).all(|w| w[0].at <= w[1].at),
            "ops sorted by at"
        );
    }

    #[test]
    fn schedule_is_deterministic() {
        let s = spec(Profile::Hour, 24, 42);
        let a = build(&s);
        let b = build(&s);
        assert_eq!(a.len(), b.len());
        for (x, y) in a.iter().zip(b.iter()) {
            assert_eq!(x.at, y.at);
            assert_eq!(x.class, y.class);
            assert_eq!(x.patient, y.patient);
            // Payloads are rendered deterministically too.
            assert_eq!(format!("{:?}", x.action), format!("{:?}", y.action));
        }
    }

    #[test]
    fn different_seed_changes_schedule() {
        let a = build(&spec(Profile::Hour, 24, 1));
        let b = build(&spec(Profile::Hour, 24, 2));
        // Overwhelmingly likely to differ in timing.
        let a_times: Vec<_> = a.iter().map(|o| o.at).collect();
        let b_times: Vec<_> = b.iter().map(|o| o.at).collect();
        assert_ne!(a_times, b_times);
    }

    #[test]
    fn per_patient_ordering_invariant_holds() {
        let ops = build(&spec(Profile::Hour, 24, 7));
        let mut by_patient: BTreeMap<usize, Vec<&PlannedOp>> = BTreeMap::new();
        for op in &ops {
            by_patient.entry(op.patient).or_default().push(op);
        }
        for (patient, mut list) in by_patient {
            list.sort_by_key(|o| o.at);
            // CreateEhr is the earliest op for the patient.
            let first_create_ehr = list
                .iter()
                .position(|o| o.class == OpClass::EhrCreate)
                .expect("patient has a CreateEhr");
            assert_eq!(first_create_ehr, 0, "patient {patient} CreateEhr not first");

            // First composition create precedes every dependent op.
            let first_comp_at = list
                .iter()
                .find(|o| matches!(o.class, OpClass::CompCreateSmall | OpClass::CompCreateLarge))
                .map(|o| o.at)
                .expect("patient has a composition create");
            for o in &list {
                if matches!(
                    o.class,
                    OpClass::CompReadLatest
                        | OpClass::CompReadVersion
                        | OpClass::CompUpdate
                        | OpClass::HistoryRead
                        | OpClass::AqlPatient
                ) {
                    assert!(
                        o.at > first_comp_at,
                        "patient {patient} dependent {:?} at {:?} not after first create {:?}",
                        o.class,
                        o.at,
                        first_comp_at
                    );
                }
            }
        }
    }

    #[test]
    fn read_write_budget_is_about_seventy_thirty() {
        // Clinical write budget counts a CONTRIBUTION as its committed
        // compositions (capacity-planning lineage): a batch
        // commit of N compositions is N clinical writes.
        let ops = build(&spec(Profile::Hour, 64, 3));
        let mut reads = 0u64;
        let mut writes = 0u64;
        for op in &ops {
            if op.at < Duration::from_mins(5) {
                continue; // warmup discarded
            }
            if op.class.is_read() {
                reads += 1;
            } else if let Action::CommitContribution { payload, .. } = &op.action {
                let n = payload
                    .get("versions")
                    .and_then(|v| v.as_array())
                    .map_or(1, Vec::len);
                writes += n as u64;
            } else if op.class == OpClass::OptUpload {
                // provisioning is outside the measured mix
            } else {
                writes += 1;
            }
        }
        let frac = reads as f64 / (reads + writes) as f64;
        assert!(
            (0.65..=0.75).contains(&frac),
            "read fraction {frac:.3} outside 70±5% (reads={reads}, writes={writes})"
        );
    }

    #[test]
    fn ops_carry_consistent_event_instances() {
        let ops = build(&spec(Profile::Hour, 24, 13));
        // Group every op by its event-instance id.
        let mut by_instance: BTreeMap<u64, Vec<&PlannedOp>> = BTreeMap::new();
        for op in &ops {
            by_instance.entry(op.event.id).or_default().push(op);
        }
        for (id, steps) in &by_instance {
            let first = steps[0].event;
            // Every step of an occurrence shares the same tag.
            for op in steps {
                assert_eq!(op.event.id, *id);
                assert_eq!(op.event.class, first.class);
                assert_eq!(op.event.steps, first.steps);
                assert_eq!(op.event.boundary_at, first.boundary_at);
            }
            // The declared step count matches the ops actually emitted, and the
            // boundary is the last step's planned time (max `at`).
            let count = u32::try_from(steps.len()).unwrap_or(u32::MAX);
            assert_eq!(first.steps, count, "instance {id} step-count mismatch");
            let last_at = steps.iter().map(|o| o.at).max().expect("non-empty");
            assert_eq!(first.boundary_at, last_at, "instance {id} boundary");
        }
    }

    #[test]
    fn event_instance_ids_are_unique_per_occurrence() {
        // A deterministic build assigns a distinct id to each emitted occurrence;
        // ids never collide across patients/events.
        let ops = build(&spec(Profile::Hour, 32, 21));
        let mut seen_pairs = std::collections::HashSet::new();
        for op in &ops {
            // (id, at) within an instance repeats per step; (id) maps to exactly
            // one class — so a class change under one id would be a bug.
            seen_pairs.insert((op.event.id, op.event.class));
        }
        let distinct_ids: std::collections::HashSet<u64> = ops.iter().map(|o| o.event.id).collect();
        assert_eq!(
            seen_pairs.len(),
            distinct_ids.len(),
            "each event id maps to exactly one class"
        );
    }

    #[test]
    fn smoke_profile_is_small() {
        let s = spec(Profile::Smoke, 4, 1);
        let ward = Ward::new(&s);
        let ops = build_ops(&s, &ward, Duration::from_mins(2), Duration::from_secs(15))
            .expect("smoke builds");
        // A handful of ops per patient — far smaller than the hour profile.
        assert!(!ops.is_empty());
        assert!(ops.len() < 200, "smoke should be small, got {}", ops.len());
    }

    #[test]
    fn day_profile_builds_and_is_sorted() {
        let ops = build(&spec(Profile::Day, 20, 9));
        assert!(!ops.is_empty());
        assert!(ops.windows(2).all(|w| w[0].at <= w[1].at));
    }
}
