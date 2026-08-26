// SPDX-FileCopyrightText: FerroEHR contributors
// SPDX-License-Identifier: Apache-2.0

//! Journey expansion into the planned arrival schedule: deterministic,
//! open-loop, coordinated-omission-free.
//!
//! Journey instances arrive on a virtual timeline that starts
//! `max_offset` BEFORE the measured window (steady-state initialization:
//! the ward is already mid-shift when measurement begins); every stage of
//! every instance becomes one planned arrival instant, and only instants
//! inside the window are dispatched — mid-journey stages whose earlier
//! stages fell pre-window resolve against the standing ward state seeded
//! by [`crate::perf_run::corpus::seed_ward`]. The construction is DIRECT
//! — journey instants on a deterministic grid at the offered rate (plus a
//! small fixed floor margin absorbing edge clipping), never a
//! fill-to-target loop: the realized rate is the offered rate ± clipping
//! noise, and the sustained load is reported from actual dispatch.
//!
//! NOTE: no openEHR spec governs measured performance (CNF guide
//! master03-overview.adoc §Product Scope) — our own design/extension. The
//! diurnal curve realizes the ITU-T E.500 busy-hour convention the class
//! floors' peak factor already cites: `arrival_rate` is the BUSY-HOUR
//! peak, the day curve scales the off-peak troughs below it.

#![allow(
    clippy::disallowed_types,
    reason = "dev/verification tooling over JSON artifacts (the catalogue, results, wire \
              exchanges) — not the application (#1694); the carriers here are cfg(test)-only, so \
              #[expect] would be unfulfilled in the non-test build"
)]

use std::collections::BTreeMap;
use std::time::Duration;

use crate::perf::{ArrivalCurve, JourneyCatalogue, Percent, PerfOp, StageOffset};
use crate::perf_run::client::PerfPrincipals;
use crate::perf_run::pack::JourneyPack;

/// The deterministic schedule seed (fixed so two runners produce the same
/// instants for the same case).
const SCHEDULE_SEED: u64 = 0x636e_665f_686f_7370;

/// The offered-rate floor margin: stage-offset clipping at the window
/// edges makes the realized in-window rate a noisy realization of the
/// planned one, and the class floor is a hard `min` — +2% keeps the
/// honest realization above the floor and is itself honest (a slightly
/// STRICTER offered load, reported from actual dispatch).
const FLOOR_MARGIN: f64 = 1.02;

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
/// template pack, the arrival curve, and the principals the party's ixit
/// declares.
#[derive(Debug)]
pub struct JourneyWorkload<'a> {
    /// Every journey the schedule may draw from.
    pub catalogue: &'a JourneyCatalogue,
    /// The case's share of scheduled instances per journey.
    pub shares: &'a [(String, Percent)],
    /// The templates and auxiliary payloads the stages commit.
    pub pack: &'a JourneyPack,
    /// The arrival-time shape the instants are drawn under.
    pub curve: ArrivalCurve,
    /// The principals available for this run
    /// ([`crate::perf_run::client::PerfPrincipals`]).
    pub principals: &'a PerfPrincipals,
}

impl JourneyWorkload<'_> {
    /// The shares actually schedulable against this party's declared
    /// principals, RENORMALIZED to 100%.
    ///
    /// A journey any of whose stages addresses an ixit instance the party
    /// does not declare is not scheduled: the same law the functional lane
    /// applies to an undeclared deployment fact — it costs COVERAGE, never
    /// correctness — and renormalizing keeps the remaining mix at the
    /// offered operation rate instead of silently running below the class
    /// floor. The dropped journeys are named to the caller so the run
    /// record says what the party's declaration cost it.
    fn schedulable(&self) -> (Vec<(String, Percent)>, Vec<String>) {
        let mut kept: Vec<(String, Percent)> = Vec::new();
        let mut dropped: Vec<String> = Vec::new();
        for (name, share) in self.shares {
            let runnable = self.catalogue.get(name).is_none_or(|journey| {
                journey.stages.iter().all(|stage| {
                    PerfOp::parse(&stage.op)
                        .is_ok_and(|op| self.principals.declares(op.principal()))
                })
            });
            if runnable {
                kept.push((name.clone(), *share));
            } else {
                dropped.push(name.clone());
            }
        }
        let total: f64 = kept.iter().map(|(_, p)| p.0).sum();
        if total > 0.0 && (total - 100.0).abs() >= 0.01 {
            for (_, share) in &mut kept {
                share.0 = share.0 / total * 100.0;
            }
        }
        (kept, dropped)
    }
}

/// The built schedule + its planned offered-load facts.
#[derive(Debug)]
pub(crate) struct BuiltSchedule {
    pub arrivals: Vec<PlannedArrival>,
    /// Journeys the party's ixit declares no principal for (not scheduled;
    /// the remaining shares were renormalized).
    pub dropped_journeys: Vec<String>,
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
        // Highest-credit slot wins, ties to the LOWEST index — the scan keeps
        // that order exactly (a strict `>` never displaces an equal leader),
        // which is what makes the schedule reproducible.
        let mut best = 0;
        let mut best_credit = f64::NEG_INFINITY;
        for (i, credit) in self.credit.iter().enumerate() {
            if *credit > best_credit {
                best = i;
                best_credit = *credit;
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
            #[expect(
                clippy::as_conversions,
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "journey counts are far below the lossy range and non-negative by construction"
            )]
            let count = count as u64;
            (0..count)
                .map(|j| {
                    #[expect(
                        clippy::as_conversions,
                        clippy::cast_precision_loss,
                        reason = "counts << 2^52"
                    )]
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
#[expect(
    clippy::too_many_lines,
    reason = "one linear construction: expand → clip → extend → sort"
)]
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
    let (shares, dropped_journeys) = workload.schedulable();
    if shares.is_empty() {
        return Err(
            "no journey of this workload is runnable against the principals the ixit declares"
                .to_owned(),
        );
    }
    let expansion = workload.catalogue.expansion(&shares)?;
    let mut resolved: Vec<ResolvedJourney> = Vec::with_capacity(shares.len());
    for (name, _) in &shares {
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
            fresh_ehr || stages.iter().any(|s| s.op.needs_instance_prerequisite());
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
            "the workload addresses standing ward patients but the corpus has no seeded ward"
                .to_owned(),
        );
    }
    let max_offset_s = resolved.iter().map(|j| j.max_offset_s).max().unwrap_or(0);

    // Journey rate = the offered OPERATION rate through the catalogue's
    // mean expansion, with the fixed floor margin. The schedule is a
    // direct construction over the virtual timeline — no fill-to-target
    // loop: demanding an exact count would either bunch extra load (a
    // phase-shifted second pool doubles the rate) or truncate the window
    // tail.
    let journey_rate = rate * FLOOR_MARGIN / expansion.arrivals_per_journey;
    let span_s = warmup_s.saturating_add(duration_s);
    #[expect(
        clippy::as_conversions,
        clippy::cast_precision_loss,
        reason = "spans << 2^52"
    )]
    let virtual_span_s = span_s as f64 + max_offset_s as f64;
    let mut sequencer = ShareSequencer::new(&shares);
    #[expect(
        clippy::as_conversions,
        reason = "journey-kind count widens exactly: usize is at most 64 bits on every supported target"
    )]
    let kinds = resolved.len() as u64;
    let mut kind_seq: Vec<u64> = vec![0; resolved.len()];
    let mut arrivals: Vec<PlannedArrival> = Vec::new();
    let mut measured_count: u64 = 0;

    let pool = journey_instants(workload.curve, journey_rate, virtual_span_s);
    let mut journey_index: u64 = 0;
    for instant in pool {
        let kind = sequencer.next();
        let journey = resolved.get(kind).ok_or_else(|| {
            format!("journey sequencer chose slot {kind} outside the resolved catalogue")
        })?;
        let seq_slot = kind_seq.get_mut(kind).ok_or_else(|| {
            format!("journey sequencer chose slot {kind} outside the sequence counters")
        })?;
        let seq = *seq_slot;
        *seq_slot += 1;
        let patient = if journey.fresh_ehr {
            None
        } else {
            #[expect(
                clippy::as_conversions,
                clippy::cast_possible_truncation,
                reason = "ward indices are small"
            )]
            Some(((kind as u64 + kinds * seq) % ward_len.max(1) as u64) as usize)
        };
        // The journey's own clock starts max_offset before the window so
        // steady-state instances are mid-flight at t=0. Standing-ward
        // journeys resolve their pre-window stages from the seeded ward;
        // a journey with NO seeded fallback (a fresh EHR, a delete of its
        // own commit) must start in-window — a pre-window start skips the
        // instance.
        #[expect(
            clippy::as_conversions,
            clippy::cast_precision_loss,
            reason = "offsets << 2^52"
        )]
        let start_s = instant - max_offset_s as f64;
        if journey.needs_full_window && start_s < 0.0 {
            continue;
        }
        let mut kept_any = false;
        for (stage_index, stage) in journey.stages.iter().enumerate() {
            let reps = stage.at.arrivals();
            for rep in 0..reps {
                #[expect(
                    clippy::as_conversions,
                    reason = "stage index widens exactly: usize is at most 64 bits on every supported target"
                )]
                let offset = offset_s(stage.at, journey_index, stage_index as u64, rep);
                #[expect(
                    clippy::as_conversions,
                    clippy::cast_precision_loss,
                    reason = "offsets << 2^52"
                )]
                let at_s = start_s + offset as f64;
                #[expect(
                    clippy::as_conversions,
                    clippy::cast_precision_loss,
                    reason = "spans << 2^52"
                )]
                if at_s < 0.0 || at_s >= span_s as f64 {
                    continue;
                }
                #[expect(
                    clippy::as_conversions,
                    clippy::cast_precision_loss,
                    reason = "spans << 2^52"
                )]
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
    // Ordered map: schedule construction must be byte-deterministic, so no
    // hash-ordered iteration anywhere in it (`clippy::iter_over_hash_type`).
    let mut last_index: BTreeMap<u64, usize> = BTreeMap::new();
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
            #[expect(
                clippy::as_conversions,
                clippy::cast_precision_loss,
                reason = "counts << 2^52"
            )]
            {
                measured.len() as f64 / duration_s.max(1) as f64
            }
        } else {
            // Max rolling hour, 60 s stride (instants are sorted).
            let mut best = 0.0_f64;
            let mut lo = 0usize;
            let mut hi = 0usize;
            #[expect(
                clippy::as_conversions,
                clippy::cast_precision_loss,
                reason = "spans << 2^52"
            )]
            let end = (warmup_s + duration_s) as f64;
            #[expect(
                clippy::as_conversions,
                clippy::cast_precision_loss,
                reason = "spans << 2^52"
            )]
            let mut window_start = warmup_s as f64;
            while window_start + 3600.0 <= end + 1.0 {
                while measured.get(lo).is_some_and(|at| *at < window_start) {
                    lo += 1;
                }
                while measured
                    .get(hi)
                    .is_some_and(|at| *at < window_start + 3600.0)
                {
                    hi += 1;
                }
                #[expect(
                    clippy::as_conversions,
                    clippy::cast_precision_loss,
                    reason = "counts << 2^52"
                )]
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
        dropped_journeys,
        planned_measured: measured_count,
        planned_busy_hour,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ixit::Ixit;
    use crate::perf_run::client::PerfClient;
    use crate::perf_run::pack::{AuxPayloads, JourneyPack, PackTemplate};

    /// A principal set with every optional instance declared (no journey is
    /// dropped); `stub_principals(false)` declares the default one only.
    fn stub_principals(full: bool) -> PerfPrincipals {
        let ixit: Ixit = serde_json::from_value(serde_json::json!({
            "instances": { "sut": { "base_url": "http://stub", "auth": { "mode": "none" } } }
        }))
        .unwrap();
        let client = PerfClient::from_instance(ixit.default_instance().unwrap(), &ixit).unwrap();
        let principals = PerfPrincipals::single(client.clone());
        if full {
            principals
                .with_unauthenticated(client.clone())
                .with_readonly(client.clone())
                .with_admin(client.clone())
                .with_smart_platform(client)
        } else {
            principals
        }
    }

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
            aux: AuxPayloads::default(),
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
        let principals = stub_principals(true);
        let workload = JourneyWorkload {
            catalogue: &catalogue,
            shares: &shares,
            pack: &pack,
            curve: ArrivalCurve::Uniform,
            principals: &principals,
        };
        let a = build_schedule(&workload, 10.0, 5, 60, 100).unwrap();
        let b = build_schedule(&workload, 10.0, 5, 60, 100).unwrap();
        // The realized measured count is the offered rate ± clipping noise
        // — never below the floor, never a bunched multiple of it.
        // The toy fixture's 65 s window clips hard against its 6-minute
        // journey span, so the band is wide — the doubling regression
        // would land ~1,200, far outside it.
        assert!(
            (560..=700).contains(&a.planned_measured),
            "planned_measured {} outside the 10/s band",
            a.planned_measured
        );
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

    /// A party that declares only its `sut` instance cannot run the
    /// boundary/platform journeys: they are dropped, the remaining shares
    /// renormalize, and the offered operation rate is still met — an
    /// undeclared deployment fact costs coverage, never correctness.
    #[test]
    fn undeclared_principals_drop_their_journeys_and_renormalize() {
        let catalogue: JourneyCatalogue = serde_saphyr::from_str(
            "chart_review:\n  description: d\n  derivation: g\n  stages:\n    - { op: composition_read, at: PT0S }\n    - { op: adhoc_query, at: PT1S }\nvitals_round:\n  description: d\n  derivation: g\n  stages:\n    - { op: composition_commit, template: cnf.ckm.vital_signs, at: PT0S }\naccess_control_probe:\n  description: d\n  derivation: g\n  stages:\n    - { op: unauthenticated_probe, at: PT0S }\nplatform_probe:\n  description: d\n  derivation: g\n  stages:\n    - { op: smart_configuration_read, at: PT0S }\n",
        )
        .unwrap();
        let pack = pack();
        // Both the authored and the renormalized mix must stay inside the
        // write-share band — the reconciliation is not suspended just
        // because a party declares fewer principals.
        let shares = vec![
            ("chart_review".to_owned(), Percent(76.0)),
            ("vitals_round".to_owned(), Percent(14.0)),
            ("access_control_probe".to_owned(), Percent(5.0)),
            ("platform_probe".to_owned(), Percent(5.0)),
        ];

        let full = stub_principals(true);
        let all = JourneyWorkload {
            catalogue: &catalogue,
            shares: &shares,
            pack: &pack,
            curve: ArrivalCurve::Uniform,
            principals: &full,
        };
        let complete = build_schedule(&all, 10.0, 0, 60, 100).unwrap();
        assert!(complete.dropped_journeys.is_empty());
        assert!(
            complete
                .arrivals
                .iter()
                .any(|a| a.op == PerfOp::SmartConfigurationRead)
        );

        let bare = stub_principals(false);
        let partial = JourneyWorkload {
            catalogue: &catalogue,
            shares: &shares,
            pack: &pack,
            curve: ArrivalCurve::Uniform,
            principals: &bare,
        };
        let reduced = build_schedule(&partial, 10.0, 0, 60, 100).unwrap();
        assert_eq!(
            reduced.dropped_journeys,
            vec![
                "access_control_probe".to_owned(),
                "platform_probe".to_owned()
            ]
        );
        assert!(
            reduced
                .arrivals
                .iter()
                .all(|a| a.op.principal() == crate::perf::Principal::Primary)
        );
        // The renormalized mix still offers the planned operation rate
        // (10/s x 60 s, +2% margin, minus edge clipping).
        assert!(
            (600..=650).contains(&reduced.planned_measured),
            "renormalized schedule planned {} arrivals",
            reduced.planned_measured
        );
    }

    #[test]
    fn patient_striping_keeps_journey_kinds_disjoint() {
        let catalogue = catalogue();
        let pack = pack();
        let shares = shares();
        let principals = stub_principals(true);
        let workload = JourneyWorkload {
            catalogue: &catalogue,
            shares: &shares,
            pack: &pack,
            curve: ArrivalCurve::Uniform,
            principals: &principals,
        };
        let schedule = build_schedule(&workload, 20.0, 0, 30, 99).unwrap();
        // Patients used by vitals commits never collide with lab commits:
        // stripe residues (mod kind count) differ per kind.
        let mut residue_by_op: std::collections::HashMap<
            PerfOp,
            std::collections::BTreeSet<usize>,
        > = std::collections::HashMap::new();
        for arrival in &schedule.arrivals {
            if let Some(p) = arrival.patient {
                residue_by_op.entry(arrival.op).or_default().insert(p % 3);
            }
        }
        let vitals = &residue_by_op[&PerfOp::CompositionCommit];
        // commits come from two kinds (vitals stripe 1, lab stripe 2) —
        // never the chart-review stripe 0 shared with reads-only journeys.
        assert!(!vitals.contains(&0));
    }

    #[test]
    fn the_committed_catalogue_realizes_the_class_poc_rate() {
        // Regression for the doubled schedule the first live run exposed
        // (15,566 arrivals where ~7,950 belong): the REAL committed
        // catalogue + class shares at the class-POC parameters must
        // realize the offered 2/s (x the +2% floor margin), never a
        // bunched multiple.
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let catalogue: JourneyCatalogue = serde_saphyr::from_str(
            &std::fs::read_to_string(root.join("artifacts/vocab/journey_catalogue.yaml")).unwrap(),
        )
        .unwrap();
        let case: crate::perf::PerformanceCase = serde_saphyr::from_str(
            &std::fs::read_to_string(
                root.join("artifacts/schedule/performance/PERF-hospital_sim-class_POC.yaml"),
            )
            .unwrap(),
        )
        .unwrap();
        let mut templates = Vec::new();
        for (_, journey) in &catalogue.0 {
            for stage in &journey.stages {
                if let Some(key) = &stage.template
                    && !templates.iter().any(|t: &PackTemplate| &t.key == key)
                {
                    templates.push(PackTemplate {
                        key: key.clone(),
                        template_id: key.clone(),
                        opt_xml: String::new(),
                        skeleton: serde_json::json!({"_type": "COMPOSITION"}),
                    });
                }
            }
        }
        let pack = JourneyPack {
            templates,
            aux: AuxPayloads::default(),
        };
        let principals = stub_principals(true);
        let workload = JourneyWorkload {
            catalogue: &catalogue,
            shares: &case.workload.journeys,
            pack: &pack,
            curve: ArrivalCurve::Uniform,
            principals: &principals,
        };
        let schedule = build_schedule(&workload, 2.0, 300, 3600, 10_000).unwrap();
        assert!(
            schedule.dropped_journeys.is_empty(),
            "a fully-declaring party drops no journey, got {:?}",
            schedule.dropped_journeys
        );
        // 2/s x 3600 s = 7,200; the margin + clipping noise band tops out
        // well under any doubling.
        assert!(
            (7_200..=7_800).contains(&schedule.planned_measured),
            "planned_measured {} misses the 2/s floor band (floor 7,200)",
            schedule.planned_measured
        );
        assert!(schedule.planned_busy_hour >= 2.0);
    }

    #[test]
    fn the_diurnal_curve_peaks_at_the_offered_rate() {
        let catalogue = catalogue();
        let pack = pack();
        let shares = shares();
        let principals = stub_principals(true);
        let workload = JourneyWorkload {
            catalogue: &catalogue,
            shares: &shares,
            pack: &pack,
            curve: ArrivalCurve::Diurnal,
            principals: &principals,
        };
        // 10 h hold: the busy-hour rate approaches the offered rate, the
        // whole-window mean sits well below it.
        let schedule = build_schedule(&workload, 2.0, 0, 36_000, 50).unwrap();
        #[expect(
            clippy::as_conversions,
            clippy::cast_precision_loss,
            reason = "planned journey counts in a 10 h window are far below 2^52"
        )]
        let mean = schedule.planned_measured as f64 / 36_000.0;
        assert!(schedule.planned_busy_hour > mean * 1.5);
        assert!(schedule.planned_busy_hour <= 2.2);
    }
}
