//! Journey expansion into the planned arrival schedule: deterministic,
//! open-loop, coordinated-omission-free.
//!
//! Journey instances arrive on a virtual timeline that starts
//! `max_offset` BEFORE the measured window (steady-state initialization:
//! the ward is already mid-shift when measurement begins); every stage of
//! every instance becomes one planned arrival instant, and only instants
//! inside the window are dispatched — mid-journey stages whose earlier
//! stages fell pre-window resolve against the standing ward state seeded
//! by [`crate::perf_run::corpus::seed_ward`]. The schedule extends by
//! whole journeys until the measured window holds at least
//! `rate × duration` operation arrivals, so the offered-load floor is met
//! by construction and reported honestly from actual dispatch.
//!
//! NOTE: no openEHR spec governs measured performance (CNF guide
//! master03-overview.adoc §Product Scope) — our own design/extension. The
//! diurnal curve realizes the ITU-T E.500 busy-hour convention the class
//! floors' peak factor already cites: `arrival_rate` is the BUSY-HOUR
//! peak, the day curve scales the off-peak troughs below it.

use std::time::Duration;

use crate::perf::{ArrivalCurve, JourneyCatalogue, Percent, PerfOp, StageOffset};
use crate::perf_run::pack::JourneyPack;

/// The deterministic schedule seed (fixed so two runners produce the same
/// instants for the same case).
const SCHEDULE_SEED: u64 = 0x636e_665f_686f_7370;

/// Which standing ward document a stage addresses (resolved at schedule
/// build from the journey's update stage, so a read-current stage knows
/// which document the journey is about).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WardDoc {
    /// The seeded GP-data-set encounter document (the default chart).
    Gp,
    /// The seeded medicines list (medicines reconciliation).
    MedList,
}

/// One planned operation arrival.
#[derive(Debug, Clone)]
pub(crate) struct PlannedArrival {
    /// Offset from schedule start (warmup begins at zero).
    pub at: Duration,
    pub op: PerfOp,
    /// Pack index of the stage's template (commit/update stages).
    pub template: Option<usize>,
    /// The journey instance this stage belongs to.
    pub journey: u64,
    /// The standing ward patient (None = the journey creates its own EHR).
    pub patient: Option<usize>,
    /// The ward document the stage addresses.
    pub doc: WardDoc,
    /// Whether the arrival falls in the measured (post-warmup) span.
    pub recorded: bool,
    /// Whether this is the journey instance's last in-window stage (the
    /// capture-state cleanup point).
    pub last: bool,
}

/// A workload bundle: the catalogue, the case's journey shares, the
/// template pack, and the arrival curve.
#[derive(Debug)]
pub struct JourneyWorkload<'a> {
    pub catalogue: &'a JourneyCatalogue,
    pub shares: &'a [(String, Percent)],
    pub pack: &'a JourneyPack,
    pub curve: ArrivalCurve,
}

/// The built schedule + its planned offered-load facts.
#[derive(Debug)]
pub(crate) struct BuiltSchedule {
    pub arrivals: Vec<PlannedArrival>,
    /// Measured-window arrivals planned (the dispatch-fidelity
    /// denominator).
    pub planned_measured: u64,
    /// The planned busy-hour offered load (max rolling-hour rate of the
    /// measured span) — the diurnal floor semantic; equals the mean rate
    /// under the uniform curve.
    pub planned_busy_hour: f64,
}

/// Exact-share deterministic interleaving (largest-remainder error
/// diffusion): instance `i`'s journey is the share entry with the largest
/// accumulated credit. Two runners with the same shares produce the same
/// sequence.
struct ShareSequencer {
    shares: Vec<f64>,
    credit: Vec<f64>,
}

impl ShareSequencer {
    fn new(shares: &[(String, Percent)]) -> Self {
        Self {
            shares: shares.iter().map(|(_, p)| p.0).collect(),
            credit: vec![0.0; shares.len()],
        }
    }

    fn next(&mut self) -> usize {
        for (credit, share) in self.credit.iter_mut().zip(&self.shares) {
            *credit += *share;
        }
        let mut best = 0;
        for i in 1..self.credit.len() {
            if self.credit[i] > self.credit[best] {
                best = i;
            }
        }
        if let Some(credit) = self.credit.get_mut(best) {
            *credit -= 100.0;
        }
        best
    }
}

/// FNV-1a over the schedule seed + indices — the deterministic draw for
/// uniform stage offsets.
fn fnv1a(parts: &[u64]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325 ^ SCHEDULE_SEED;
    for part in parts {
        for byte in part.to_le_bytes() {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
    hash
}

/// A stage offset realized for one (journey instance, stage, repetition).
fn offset_s(at: StageOffset, journey: u64, stage: u64, rep: u64) -> u64 {
    match at {
        StageOffset::Fixed(s) => s,
        StageOffset::Uniform { min_s, max_s } => {
            let span = max_s.saturating_sub(min_s).saturating_add(1);
            min_s + fnv1a(&[journey, stage, rep]) % span
        }
        StageOffset::Periodic { interval_s, .. } => interval_s.saturating_mul(rep),
    }
}

/// The hospital day curve: night base + morning/afternoon peaks +
/// shift-change bumps (the ITU-T E.500 busy-hour convention as a smooth
/// day shape); `tau` is the day fraction.
fn diurnal_weight(tau: f64) -> f64 {
    let gauss = |mu: f64, sigma: f64| (-((tau - mu).powi(2)) / (2.0 * sigma * sigma)).exp();
    let h = |hour: f64| hour / 24.0;
    0.15 + 1.00 * gauss(h(8.0), 0.030)
        + 0.90 * gauss(h(14.0), 0.030)
        + 0.40 * gauss(h(7.0), 0.020)
        + 0.40 * gauss(h(15.0), 0.020)
        + 0.35 * gauss(h(23.0), 0.020)
}

/// The curve's peak weight (the busy-hour normalizer: the offered
/// `arrival_rate` is the PEAK rate, off-peak scales below it).
fn diurnal_peak() -> f64 {
    let mut peak: f64 = 0.0;
    for i in 0..288 {
        peak = peak.max(diurnal_weight((f64::from(i) + 0.5) / 288.0));
    }
    peak
}

/// Journey arrival instants (seconds, relative to the virtual timeline
/// start) for `count`-ish instances at `rate_peak` journeys/s over
/// `span_s`: uniform = an even grid; diurnal = intensity integration (a
/// journey fires each time the accumulated intensity crosses one).
fn journey_instants(curve: ArrivalCurve, rate_peak: f64, span_s: f64) -> Vec<f64> {
    match curve {
        ArrivalCurve::Uniform => {
            let interval = 1.0 / rate_peak;
            let count = (span_s * rate_peak).ceil().max(1.0);
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            // journey counts are far below the lossy range
            let count = count as u64;
            (0..count)
                .map(|j| {
                    #[allow(clippy::cast_precision_loss)] // counts << 2^52
                    {
                        j as f64 * interval
                    }
                })
                .collect()
        }
        ArrivalCurve::Diurnal => {
            let peak = diurnal_peak();
            let mut instants = Vec::new();
            let mut accumulated = 1.0; // fire the first journey at t=0
            let step = 0.25; // seconds; fine enough for hour-scale curves
            let mut t = 0.0;
            while t < span_s {
                let tau = (t % 86_400.0) / 86_400.0;
                accumulated += rate_peak * diurnal_weight(tau) / peak * step;
                while accumulated >= 1.0 {
                    accumulated -= 1.0;
                    instants.push(t);
                }
                t += step;
            }
            instants
        }
    }
}

/// Build the planned arrival schedule for one measured window.
///
/// `ward_len` is the standing ward size (stage targets stripe across it —
/// each journey kind gets a disjoint patient stripe so mutating journeys
/// never interleave on one patient).
///
/// # Errors
/// A message on an unknown journey/operation/template or an empty
/// expansion.
#[allow(clippy::too_many_lines)] // one linear construction: expand → clip → extend → sort
pub(crate) fn build_schedule(
    workload: &JourneyWorkload<'_>,
    rate: f64,
    warmup_s: u64,
    duration_s: u64,
    ward_len: usize,
) -> Result<BuiltSchedule, String> {
    // Resolve every named journey once: stages with pack indices, the
    // ward-doc hint, the fresh-EHR flag, and the journey span.
    struct ResolvedStage {
        op: PerfOp,
        template: Option<usize>,
        at: StageOffset,
    }
    struct ResolvedJourney {
        stages: Vec<ResolvedStage>,
        fresh_ehr: bool,
        /// The instance must start in-window: it creates its own EHR or
        /// carries a dependent stage with no seeded-ward fallback (a
        /// delete of the instance's own commit).
        needs_full_window: bool,
        doc: WardDoc,
        max_offset_s: u64,
    }

    if !(rate.is_finite() && rate > 0.0) {
        return Err("arrival rate must be positive".to_owned());
    }
    let expansion = workload.catalogue.expansion(workload.shares)?;
    let mut resolved: Vec<ResolvedJourney> = Vec::with_capacity(workload.shares.len());
    for (name, _) in workload.shares {
        let journey = workload
            .catalogue
            .get(name)
            .ok_or_else(|| format!("workload names unknown journey {name:?}"))?;
        let mut stages = Vec::with_capacity(journey.stages.len());
        let mut fresh_ehr = false;
        let mut doc = WardDoc::Gp;
        for stage in &journey.stages {
            let op = PerfOp::parse(&stage.op)?;
            if op == PerfOp::EhrCreate {
                fresh_ehr = true;
            }
            let template = match &stage.template {
                None => None,
                Some(key) => Some(workload.pack.index_of(key).ok_or_else(|| {
                    format!("journey {name}: template {key} is not in the loaded pack")
                })?),
            };
            if op == PerfOp::CompositionUpdate
                && stage
                    .template
                    .as_deref()
                    .is_some_and(|k| k.contains("medicines_list"))
            {
                doc = WardDoc::MedList;
            }
            stages.push(ResolvedStage {
                op,
                template,
                at: stage.at,
            });
        }
        let needs_full_window =
            fresh_ehr || stages.iter().any(|s| s.op == PerfOp::CompositionDelete);
        resolved.push(ResolvedJourney {
            stages,
            fresh_ehr,
            needs_full_window,
            doc,
            max_offset_s: journey.max_offset_s(),
        });
    }
    let needs_ward = resolved.iter().any(|j| !j.fresh_ehr);
    if needs_ward && ward_len == 0 {
        return Err(
            "the workload addresses standing ward patients but the corpus has no seeded ward \
             (re-seed without --skip-seed)"
                .to_owned(),
        );
    }
    let max_offset_s = resolved.iter().map(|j| j.max_offset_s).max().unwrap_or(0);

    let journey_rate = rate / expansion.arrivals_per_journey;
    let span_s = warmup_s.saturating_add(duration_s);
    #[allow(clippy::cast_precision_loss)] // spans << 2^52
    let virtual_span_s = span_s as f64 + max_offset_s as f64;
    // Generous instant pool; the fill loop below consumes journeys in order
    // and stops at the target, extending the pool if clipping starved it.
    let mut sequencer = ShareSequencer::new(workload.shares);
    let kinds = resolved.len() as u64;
    let mut kind_seq: Vec<u64> = vec![0; resolved.len()];
    let mut arrivals: Vec<PlannedArrival> = Vec::new();
    let mut measured_count: u64 = 0;
    #[allow(
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss
    )] // rate × duration is far below the lossy range
    let target_measured: u64 = match workload.curve {
        // The flat window must hold the full rate × duration arrivals;
        // stage clipping is compensated by densify rounds below.
        ArrivalCurve::Uniform => (rate * duration_s as f64).ceil() as u64,
        // The day curve's off-peak troughs are the DESIGN: the floor
        // semantic is the busy hour, never the whole-window mean — no
        // fill-to-target.
        ArrivalCurve::Diurnal => 0,
    };

    let mut pool = journey_instants(workload.curve, journey_rate, virtual_span_s);
    let mut densify_round: u32 = 0;
    let mut next_instant = 0usize;
    let mut journey_index: u64 = 0;
    loop {
        let instant = if let Some(t) = pool.get(next_instant) {
            *t
        } else {
            // Stage-offset clipping left the window short of the
            // offered target: densify with phase-shifted instants
            // INSIDE the same span (never past it), halving the phase
            // each round. (Uniform only — diurnal has no flat target.)
            if measured_count >= target_measured {
                break;
            }
            densify_round += 1;
            if densify_round > 8 {
                return Err(
                    "schedule construction cannot reach the offered-load target (journey \
                     offsets exceed the window by too much)"
                        .to_owned(),
                );
            }
            let phase = (1.0 / journey_rate) / f64::from(1_u32 << densify_round);
            let more = journey_instants(workload.curve, journey_rate, virtual_span_s);
            pool.extend(more.iter().map(|t| t + phase));
            continue;
        };
        #[allow(clippy::cast_precision_loss)] // spans << 2^52
        let past_window = instant - max_offset_s as f64 > span_s as f64;
        if measured_count >= target_measured && past_window {
            break;
        }
        next_instant += 1;
        let kind = sequencer.next();
        let journey = &resolved[kind];
        let seq = kind_seq[kind];
        kind_seq[kind] += 1;
        let patient = if journey.fresh_ehr {
            None
        } else {
            #[allow(clippy::cast_possible_truncation)] // ward indices are small
            Some(((kind as u64 + kinds * seq) % ward_len.max(1) as u64) as usize)
        };
        // The journey's own clock starts max_offset before the window so
        // steady-state instances are mid-flight at t=0. Standing-ward
        // journeys resolve their pre-window stages from the seeded ward;
        // a journey with NO seeded fallback (a fresh EHR, a delete of its
        // own commit) must start in-window — a pre-window start skips the
        // instance.
        #[allow(clippy::cast_precision_loss)] // offsets << 2^52
        let start_s = instant - max_offset_s as f64;
        if journey.needs_full_window && start_s < 0.0 {
            continue;
        }
        let mut kept_any = false;
        for (stage_index, stage) in journey.stages.iter().enumerate() {
            let reps = stage.at.arrivals();
            for rep in 0..reps {
                let offset = offset_s(stage.at, journey_index, stage_index as u64, rep);
                #[allow(clippy::cast_precision_loss)] // offsets << 2^52
                let at_s = start_s + offset as f64;
                #[allow(clippy::cast_precision_loss)] // spans << 2^52
                if at_s < 0.0 || at_s >= span_s as f64 {
                    continue;
                }
                #[allow(clippy::cast_precision_loss)] // spans << 2^52
                let recorded = at_s >= warmup_s as f64;
                if recorded {
                    measured_count += 1;
                }
                kept_any = true;
                arrivals.push(PlannedArrival {
                    at: Duration::from_secs_f64(at_s),
                    op: stage.op,
                    template: stage.template,
                    journey: journey_index,
                    patient,
                    doc: journey.doc,
                    recorded,
                    last: false,
                });
            }
        }
        if kept_any {
            journey_index += 1;
        }
    }
    if measured_count == 0 {
        return Err("measurement window schedules zero arrivals".to_owned());
    }
    arrivals.sort_by_key(|a| a.at);
    // Mark each journey instance's final in-window stage (capture cleanup).
    let mut last_index: std::collections::HashMap<u64, usize> = std::collections::HashMap::new();
    for (i, arrival) in arrivals.iter().enumerate() {
        last_index.insert(arrival.journey, i);
    }
    for index in last_index.values() {
        if let Some(arrival) = arrivals.get_mut(*index) {
            arrival.last = true;
        }
    }

    // The planned busy-hour offered load over the measured span.
    let planned_busy_hour = {
        let measured: Vec<f64> = arrivals
            .iter()
            .filter(|a| a.recorded)
            .map(|a| a.at.as_secs_f64())
            .collect();
        if duration_s <= 3600 || workload.curve == ArrivalCurve::Uniform {
            #[allow(clippy::cast_precision_loss)] // counts << 2^52
            {
                measured.len() as f64 / duration_s.max(1) as f64
            }
        } else {
            // Max rolling hour, 60 s stride (instants are sorted).
            let mut best = 0.0_f64;
            let mut lo = 0usize;
            let mut hi = 0usize;
            #[allow(clippy::cast_precision_loss)] // spans << 2^52
            let end = (warmup_s + duration_s) as f64;
            #[allow(clippy::cast_precision_loss)] // spans << 2^52
            let mut window_start = warmup_s as f64;
            while window_start + 3600.0 <= end + 1.0 {
                while lo < measured.len() && measured[lo] < window_start {
                    lo += 1;
                }
                while hi < measured.len() && measured[hi] < window_start + 3600.0 {
                    hi += 1;
                }
                #[allow(clippy::cast_precision_loss)] // counts << 2^52
                {
                    best = best.max((hi - lo) as f64 / 3600.0);
                }
                window_start += 60.0;
            }
            best
        }
    };

    Ok(BuiltSchedule {
        arrivals,
        planned_measured: measured_count,
        planned_busy_hour,
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic)] // test assertions/fixtures
mod tests {
    use super::*;
    use crate::perf_run::pack::{JourneyPack, PackTemplate};

    fn catalogue() -> JourneyCatalogue {
        serde_saphyr::from_str(
            "chart_review:\n  description: d\n  derivation: g\n  stages:\n    - { op: composition_read, at: PT0S }\n    - { op: adhoc_query, at: PT30S }\nvitals_round:\n  description: d\n  derivation: g\n  stages:\n    - { op: composition_commit, template: cnf.ckm.vital_signs, at: PT0S }\nlab_pipeline:\n  description: d\n  derivation: g\n  stages:\n    - { op: composition_commit, template: cnf.ckm.vital_signs, at: PT0S }\n    - { op: contribution_commit, template: cnf.ckm.vital_signs, at: { uniform: [PT2M, PT5M] } }\n    - { op: composition_read_current, at: PT6M }\n",
        )
        .unwrap()
    }

    fn pack() -> JourneyPack {
        JourneyPack {
            templates: vec![PackTemplate {
                key: "cnf.ckm.vital_signs".to_owned(),
                template_id: "Vital signs".to_owned(),
                opt_xml: "<template/>".to_owned(),
                skeleton: serde_json::json!({"_type": "COMPOSITION"}),
            }],
        }
    }

    fn shares() -> Vec<(String, Percent)> {
        vec![
            ("chart_review".to_owned(), Percent(88.0)),
            ("vitals_round".to_owned(), Percent(7.0)),
            ("lab_pipeline".to_owned(), Percent(5.0)),
        ]
    }

    #[test]
    fn the_schedule_meets_the_offered_target_and_is_deterministic() {
        let catalogue = catalogue();
        let pack = pack();
        let shares = shares();
        let workload = JourneyWorkload {
            catalogue: &catalogue,
            shares: &shares,
            pack: &pack,
            curve: ArrivalCurve::Uniform,
        };
        let a = build_schedule(&workload, 10.0, 5, 60, 100).unwrap();
        let b = build_schedule(&workload, 10.0, 5, 60, 100).unwrap();
        assert!(a.planned_measured >= 600);
        assert_eq!(a.planned_measured, b.planned_measured);
        assert_eq!(a.arrivals.len(), b.arrivals.len());
        // sorted, in-window, warmup split correct
        let mut previous = Duration::ZERO;
        for arrival in &a.arrivals {
            assert!(arrival.at >= previous);
            previous = arrival.at;
            assert!(arrival.at < Duration::from_secs(65));
            assert_eq!(arrival.recorded, arrival.at >= Duration::from_secs(5));
        }
        // exactly one `last` per journey instance
        let mut lasts = std::collections::HashMap::new();
        for arrival in &a.arrivals {
            if arrival.last {
                *lasts.entry(arrival.journey).or_insert(0) += 1;
            }
        }
        assert!(lasts.values().all(|n| *n == 1));
    }

    #[test]
    fn patient_striping_keeps_journey_kinds_disjoint() {
        let catalogue = catalogue();
        let pack = pack();
        let shares = shares();
        let workload = JourneyWorkload {
            catalogue: &catalogue,
            shares: &shares,
            pack: &pack,
            curve: ArrivalCurve::Uniform,
        };
        let schedule = build_schedule(&workload, 20.0, 0, 30, 99).unwrap();
        // Patients used by vitals commits never collide with lab commits:
        // stripe residues (mod kind count) differ per kind.
        let mut residue_by_op: std::collections::HashMap<PerfOp, std::collections::BTreeSet<u64>> =
            std::collections::HashMap::new();
        for arrival in &schedule.arrivals {
            if let Some(p) = arrival.patient {
                residue_by_op
                    .entry(arrival.op)
                    .or_default()
                    .insert(p as u64 % 3);
            }
        }
        let vitals = &residue_by_op[&PerfOp::CompositionCommit];
        // commits come from two kinds (vitals stripe 1, lab stripe 2) —
        // never the chart-review stripe 0 shared with reads-only journeys.
        assert!(!vitals.contains(&0));
    }

    #[test]
    fn the_diurnal_curve_peaks_at_the_offered_rate() {
        let catalogue = catalogue();
        let pack = pack();
        let shares = shares();
        let workload = JourneyWorkload {
            catalogue: &catalogue,
            shares: &shares,
            pack: &pack,
            curve: ArrivalCurve::Diurnal,
        };
        // 10 h hold: the busy-hour rate approaches the offered rate, the
        // whole-window mean sits well below it.
        let schedule = build_schedule(&workload, 2.0, 0, 36_000, 50).unwrap();
        #[allow(clippy::cast_precision_loss)]
        let mean = schedule.planned_measured as f64 / 36_000.0;
        assert!(schedule.planned_busy_hour > mean * 1.5);
        assert!(schedule.planned_busy_hour <= 2.2);
    }
}
