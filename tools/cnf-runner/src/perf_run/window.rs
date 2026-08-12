// SPDX-FileCopyrightText: FerroEHR contributors
// SPDX-License-Identifier: MIT

//! The measured-window core: dispatch the built schedule open-loop, collect
//! per-operation HDR histograms, aggregate the re-checkable measurement
//! record.
//!
//! Shared by the class runs (conformance) and the stress ladder
//! (exploration).

use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, mpsc};
use std::time::Instant;

use hdrhistogram::Histogram;

use crate::ixit::{Environment, Ixit};
use crate::perf::{
    ClassVerdict, Measurement, OperationMeasurement, PerfOp, PerformanceCase, class_verdict,
};
use crate::perf_run::client::PerfPrincipals;
use crate::perf_run::corpus::SeededCorpus;
use crate::perf_run::execute::{CaptureStore, perform};
use crate::perf_run::pack::JourneyPack;
use crate::perf_run::schedule::{JourneyWorkload, build_schedule};

/// Latency histograms record microseconds in `1 µs ..= 10 min` at 3
/// significant figures — far past the client timeout, so a timeout can
/// never saturate the range.
const HDR_MAX_US: u64 = 600_000_000;

/// How many failed arrivals report their observed wire status / reason
/// through the progress channel — the triage evidence a bare error count
/// cannot carry.
const FAILURE_SAMPLES: u32 = 16;

/// The per-operation aggregation a run collects.
struct OpRecorder {
    histogram: Histogram<u64>,
    errors: u64,
}

/// One completed arrival: operation, latency from the PLANNED instant, and
/// whether the wire outcome matched the binding's expected kind.
struct Completion {
    op: PerfOp,
    latency_us: u64,
    ok: bool,
    recorded: bool,
}

/// One executed open-loop window's raw outcome — the shared core the class
/// runs (conformance) and the knee stress ladder (exploration) both drive.
#[derive(Debug)]
pub struct WindowOutcome {
    /// Measured arrivals over the actual measured span (arrivals/s).
    pub offered_load_sustained: f64,
    /// Per-operation records (encoded HDR histograms, re-checkable).
    pub operations: Vec<OperationMeasurement>,
    /// Whether the GENERATOR failed to hold the schedule (dispatch lagged
    /// more than 2% past the planned span) — the honest stop signal for a
    /// stress climb: beyond this point the instrument, not the SUT, is the
    /// bottleneck.
    pub generator_bound: bool,
}

/// Execute one open-loop window: the journey workload at `rate` operation
/// arrivals/s for `warmup_s + duration_s`, recording only the post-warmup
/// span.
///
/// # Errors
/// A message on schedule construction or aggregation failure (individual
/// arrival faults are error observations, not run failures).
#[expect(
    clippy::too_many_lines,
    reason = "one measured-window procedure: schedule → collect → aggregate"
)]
pub fn run_window(
    principals: &PerfPrincipals,
    corpus: &SeededCorpus,
    workload: &JourneyWorkload<'_>,
    rate: f64,
    warmup_s: u64,
    duration_s: u64,
    progress: &(dyn Fn(String) + Sync),
) -> Result<WindowOutcome, String> {
    let schedule = build_schedule(workload, rate, warmup_s, duration_s, corpus.ward.len())?;
    if !schedule.dropped_journeys.is_empty() {
        progress(format!(
            "journeys not scheduled (the ixit declares no principal for them; the remaining \
             shares were renormalized): {}",
            schedule.dropped_journeys.join(", ")
        ));
    }
    let total = schedule.arrivals.len();
    let captures = CaptureStore::new();

    let (tx, rx) = mpsc::channel::<Completion>();
    let collector = std::thread::spawn(move || {
        let mut recorders: Vec<(PerfOp, OpRecorder)> = Vec::new();
        let mut generator_faults: u64 = 0;
        for done in rx {
            if !done.recorded {
                continue;
            }
            if done.latency_us == u64::MAX {
                generator_faults = generator_faults.saturating_add(1);
                continue;
            }
            let index = if let Some(index) = recorders.iter().position(|(op, _)| *op == done.op) {
                index
            } else {
                let Ok(histogram) = Histogram::new_with_bounds(1, HDR_MAX_US, 3) else {
                    // Statically-valid bounds; treat the impossible
                    // failure as a generator fault rather than panic.
                    generator_faults = generator_faults.saturating_add(1);
                    continue;
                };
                recorders.push((
                    done.op,
                    OpRecorder {
                        histogram,
                        errors: 0,
                    },
                ));
                recorders.len() - 1
            };
            if let Some((_, recorder)) = recorders.get_mut(index) {
                let value = done.latency_us.clamp(1, HDR_MAX_US);
                let _saturated = recorder.histogram.record(value);
                if !done.ok {
                    recorder.errors = recorder.errors.saturating_add(1);
                }
            }
        }
        (recorders, generator_faults)
    });

    let start = Instant::now();
    let dispatched_measured = Arc::new(AtomicU64::new(0));
    // Failure sampling (the first FAILURE_SAMPLES failed arrivals).
    let failure_samples = Arc::new(AtomicU32::new(0));
    progress(format!(
        "open-loop schedule: {total} arrivals ({} measured) at {rate}/s aggregate \
         ({warmup_s}s warmup + {duration_s}s measured, {} journeys interleaved)",
        schedule.planned_measured,
        schedule
            .arrivals
            .last()
            .map_or(0, |a| a.journey.saturating_add(1))
    ));

    // The dispatch span (last arrival fired − start) is captured before the
    // scope waits for in-flight workers, so trailing responses never inflate
    // the sustained-load denominator.
    let dispatch_span = std::thread::scope(|scope| {
        for (i, planned_arrival) in schedule.arrivals.iter().enumerate() {
            let planned = start + planned_arrival.at;
            let now = Instant::now();
            if planned > now {
                std::thread::sleep(planned - now);
            }
            if planned_arrival.recorded {
                dispatched_measured.fetch_add(1, Ordering::Relaxed);
            }
            let tx = tx.clone();
            let principals = principals.clone();
            let captures = &captures;
            let arrival_index = u64::try_from(i).unwrap_or(u64::MAX);
            let failure_samples = Arc::clone(&failure_samples);
            scope.spawn(move || {
                let mut observed: Option<u16> = None;
                let outcome = perform(
                    &principals,
                    arrival_index,
                    planned_arrival,
                    corpus,
                    workload.pack,
                    captures,
                    &mut observed,
                );
                let latency = planned.elapsed();
                let latency_us = u64::try_from(latency.as_micros().min(u128::from(HDR_MAX_US)))
                    .unwrap_or(HDR_MAX_US);
                // An unresolvable prerequisite (the SUT has not landed the
                // earlier stage) is an honest error observation.
                let ok = match &outcome {
                    Ok(ok) => *ok,
                    Err(_) => false,
                };
                if !ok && failure_samples.fetch_add(1, Ordering::Relaxed) < FAILURE_SAMPLES {
                    let detail = match (&outcome, observed) {
                        (Err(reason), _) => reason.clone(),
                        (Ok(_), Some(status)) => format!("unexpected wire status {status}"),
                        (Ok(_), None) => "no wire observation".to_owned(),
                    };
                    progress(format!(
                        "arrival failure sample: {} journey {} at {:?}: {detail}",
                        planned_arrival.op.as_str(),
                        planned_arrival.journey,
                        planned_arrival.at,
                    ));
                }
                let _closed = tx.send(Completion {
                    op: planned_arrival.op,
                    latency_us,
                    ok,
                    recorded: planned_arrival.recorded,
                });
            });
            if i % 1000 == 999 {
                progress(format!("dispatched {}/{total} arrivals", i + 1));
            }
        }
        drop(tx);
        start.elapsed()
    });

    // A panic payload is `Box<dyn Any>`, not a `Display` error: recover the
    // message the panic carried so the run reports WHY the collector died.
    let (recorders, generator_faults) = collector.join().map_err(|payload| {
        let detail = payload
            .downcast_ref::<&str>()
            .map(|message| (*message).to_owned())
            .or_else(|| payload.downcast_ref::<String>().cloned())
            .unwrap_or_else(|| "non-string panic payload".to_owned());
        format!("collector thread panicked: {detail}")
    })?;
    if generator_faults > 0 {
        return Err(format!("{generator_faults} generator faults"));
    }

    // Offered load actually sustained: measured arrivals over the actual
    // measured span (>= the planned window when the generator lagged, which
    // honestly deflates the sustained rate). Under the DIURNAL curve the
    // floor semantic is the busy hour (ITU-T E.500): the planned busy-hour
    // rate scaled by dispatch fidelity — off-peak troughs are the design,
    // never a shortfall; a lagging generator still deflates it.
    let planned_span_s = warmup_s.saturating_add(duration_s);
    #[expect(
        clippy::as_conversions,
        clippy::cast_precision_loss,
        reason = "spans/counts << 2^52"
    )]
    let (offered_load_sustained, generator_bound) = {
        let actual_span = dispatch_span.as_secs_f64().max(planned_span_s as f64);
        let measured_span = actual_span - warmup_s as f64;
        let dispatched = dispatched_measured.load(Ordering::Relaxed) as f64;
        let fidelity = (dispatched / schedule.planned_measured.max(1) as f64)
            .min(planned_span_s as f64 / actual_span);
        let offered = match workload.curve {
            crate::perf::ArrivalCurve::Uniform => {
                if measured_span > 0.0 {
                    dispatched / measured_span
                } else {
                    0.0
                }
            }
            crate::perf::ArrivalCurve::Diurnal => schedule.planned_busy_hour * fidelity,
        };
        let lagged = dispatch_span.as_secs_f64() > planned_span_s as f64 * 1.02;
        (offered, lagged)
    };

    let mut operations: Vec<OperationMeasurement> = Vec::new();
    for (op, recorder) in &recorders {
        operations.push(OperationMeasurement::from_histogram(
            op.as_str(),
            &recorder.histogram,
            recorder.errors,
        )?);
    }
    operations.sort_by(|a, b| a.operation.cmp(&b.operation));

    Ok(WindowOutcome {
        offered_load_sustained,
        operations,
        generator_bound,
    })
}

/// Drive one performance case's open-loop hospital-simulation workload and
/// produce its re-checkable measurement record (verdict computed by
/// [`crate::perf::class_verdict`] from the encoded histograms).
///
/// `warmup_s` is the case's normative warmup; `duration_s` is the case's
/// sustained window or an officially EXTENDED one (the hours ladder — a
/// longer hold of the same offered load is a stricter demonstration).
/// The CLI never passes a window shorter than the case's; only the offline
/// test harness drives synthetic second-scale windows.
///
/// # Errors
/// A message on schedule construction or aggregation failure (individual
/// arrival faults are error observations, not run failures).
#[expect(clippy::too_many_arguments, reason = "the one case-drive seam")]
pub fn drive_case(
    case: &PerformanceCase,
    principals: &PerfPrincipals,
    corpus: &SeededCorpus,
    journey_pack: &JourneyPack,
    catalogue: &crate::perf::JourneyCatalogue,
    environment: &Environment,
    warmup_s: u64,
    duration_s: u64,
    progress: &(dyn Fn(String) + Sync),
) -> Result<Measurement, String> {
    case.check_invariants()?;
    let workload = JourneyWorkload {
        catalogue,
        shares: &case.workload.journeys,
        pack: journey_pack,
        curve: case.workload.arrival_curve,
        principals,
    };
    let window = run_window(
        principals,
        corpus,
        &workload,
        case.workload.arrival_rate.0,
        warmup_s,
        duration_s,
        progress,
    )?;
    let (verdict, violations) =
        class_verdict(case, window.offered_load_sustained, &window.operations)?;
    Ok(Measurement {
        case: case.id.clone(),
        class: case.class,
        environment: environment.clone(),
        offered_load_sustained: window.offered_load_sustained,
        warmup_s,
        duration_s,
        operations: window.operations,
        verdict,
        violations,
        // The perf handler attaches the sampled telemetry after the window
        // (the sampler brackets this call); stress windows never carry one.
        resources: None,
    })
}

/// Convenience: the ixit precondition for a measured run — every principal
/// the party declares (the `sut` instance is mandatory) and a present
/// environment block.
///
/// # Errors
/// A message naming the missing piece (the environment block is mandatory
/// for performance runs).
pub fn measured_run_context(ixit: &Ixit) -> Result<(PerfPrincipals, &Environment), String> {
    let principals = PerfPrincipals::from_ixit(ixit)?;
    let environment = ixit.environment.as_ref().ok_or_else(|| {
        "ixit has no environment block (mandatory for performance runs)".to_owned()
    })?;
    Ok((principals, environment))
}

/// Whether a measurement's verdict re-derives to the same value from its
/// own embedded histograms (the tamper check the verdict pipeline runs).
///
/// # Errors
/// As [`class_verdict`].
pub fn rederive_verdict(
    case: &PerformanceCase,
    measurement: &Measurement,
) -> Result<(ClassVerdict, Vec<String>), String> {
    class_verdict(
        case,
        measurement.offered_load_sustained,
        &measurement.operations,
    )
}
