//! The register-00 workload model: a ward of patients driven through a
//! simulated clinical day, expanded into a deterministic open-loop arrival
//! schedule (`docs/design/benchmark/00-workload-model.md`).
//!
//! [`build`] is the single entry point: given a [`WorkloadSpec`] it produces a
//! [`Workload`] — the time-sorted [`PlannedOp`]s (payloads pre-rendered), the
//! warmup floor, the run window, the `workload.lock` hash, and the set of
//! templates the driver must provision before the run.

pub mod event;
pub mod lock;
pub mod schedule;
pub mod ward;

use std::time::Duration;

use crate::render;
use crate::{BenchError, PlannedOp, Profile, TemplateKind};
use ward::Ward;

/// The inputs that fully determine a workload (register 00 §1/§3).
#[derive(Debug, Clone)]
pub struct WorkloadSpec {
    /// The time-compression profile.
    pub profile: Profile,
    /// The number of patients present at day start.
    pub ward_size: usize,
    /// Arrival-rate multiplier (`hour`/`day`; ignored by `smoke`).
    pub load_factor: f64,
    /// The generator seed (recorded in the lock; same seed → same schedule).
    pub seed: u64,
}

/// A fully-planned workload ready for the driver.
#[derive(Debug, Clone)]
pub struct Workload {
    /// The arrival schedule, sorted by planned send time (warmup ops included).
    pub ops: Vec<PlannedOp>,
    /// The warmup floor: ops with `at < warmup` are discarded by the recorder.
    pub warmup: Duration,
    /// The total run window (warmup + measured).
    pub window: Duration,
    /// The `workload.lock` hash (model + params + template set + seed).
    pub lock: String,
    /// The templates (OPTs) the driver uploads before the run.
    pub provisioning: Vec<TemplateKind>,
}

/// Build the workload for a spec.
///
/// # Errors
/// [`BenchError`] if a payload skeleton fixture cannot be read or parsed.
pub fn build(spec: &WorkloadSpec) -> Result<Workload, BenchError> {
    let (window, warmup) = profile_timing(spec.profile);
    let ward = Ward::new(spec);
    let ops = schedule::build_ops(spec, &ward, window, warmup)?;

    // Both packs are provisioned every run, in a fixed order: the retained
    // ECC-corpus fixtures (proven payloads + the persistent/directory
    // structure) then the official openEHR CKM pack (the clinical-event
    // templates). Register 00 §4 / the B3 owner directive — this is a fixed
    // union, not an op scan; a SUT that rejects an upload has that template's
    // ops excluded loudly at dispatch (see `drive`).
    let mut provisioning = vec![
        TemplateKind::Vitals,
        TemplateKind::Nested,
        TemplateKind::Persistent,
    ];
    provisioning.extend(crate::pack::KINDS);

    // The extensible, ordered template-source list the lock hashes over.
    let template_sources: Vec<String> = provisioning
        .iter()
        .map(|&kind| source_descriptor(kind))
        .collect();
    let lock = lock::compute(spec, window, warmup, &template_sources);

    Ok(Workload {
        ops,
        warmup,
        window,
        lock,
        provisioning,
    })
}

/// Build a **capacity** workload: the identical clinical mix at `spec`'s load
/// factor, compressed onto a short measurement window (register 01 §3 — the
/// knee/saturation series; `docs/design/benchmarking.md` §2.2 open-loop
/// short-fixed-duration steps). Only the *clock* changes: the per-patient-day
/// rate shape (steady daily-mean rates × `spec.load_factor`) is unchanged, so
/// the whole simulated day's operation count is packed into `step_window` — a
/// shorter window (or a higher load factor) raises the offered request rate.
/// The Hour arrival shape is forced (uniform spread, rate-driven counts — not
/// the diurnal curve or the fixed `smoke` counts) regardless of `spec.profile`.
///
/// # Errors
/// [`BenchError`] if a payload skeleton fixture cannot be read or parsed.
pub fn build_capacity(
    spec: &WorkloadSpec,
    step_window: Duration,
    warmup: Duration,
) -> Result<Workload, BenchError> {
    // The Hour rate shape is the capacity mix; the clock compresses onto the
    // short step window. `schedule::build_ops` derives occurrence counts from
    // the per-patient-day rates alone (× load_factor × active fraction) and
    // spreads them over `[warmup, window)`, so a shorter window is a pure
    // time-compression of the same clinical day.
    let hour_spec = WorkloadSpec {
        profile: Profile::Hour,
        ..spec.clone()
    };
    let ward = Ward::new(&hour_spec);
    let ops = schedule::build_ops(&hour_spec, &ward, step_window, warmup)?;

    let mut provisioning = vec![
        TemplateKind::Vitals,
        TemplateKind::Nested,
        TemplateKind::Persistent,
    ];
    provisioning.extend(crate::pack::KINDS);

    // The lock hashes the actual window/warmup (it already takes them), so two
    // capacity runs at the same L/window share a lock and differing windows do
    // not — the fairness/comparability guarantee holds for the knee series too.
    let template_sources: Vec<String> = provisioning
        .iter()
        .map(|&kind| source_descriptor(kind))
        .collect();
    let lock = lock::compute(&hour_spec, step_window, warmup, &template_sources);

    Ok(Workload {
        ops,
        warmup,
        window: step_window,
        lock,
        provisioning,
    })
}

/// The `(window, warmup)` timing for a profile (register 00 §3; the warmup is a
/// fixed floor inside the window, discarded symmetrically per register 01 §1).
#[must_use]
pub fn profile_timing(profile: Profile) -> (Duration, Duration) {
    match profile {
        Profile::Smoke => (Duration::from_mins(2), Duration::from_secs(15)),
        Profile::Hour | Profile::Day => (Duration::from_hours(1), Duration::from_mins(5)),
    }
}

/// The `workload.lock` source descriptor for a provisioned template: the
/// CKM-pack descriptor (`ckm:<slug>|…`) for a CKM kind, else the ECC-corpus
/// `template_id|opt|composition` descriptor.
fn source_descriptor(kind: TemplateKind) -> String {
    if let Some(tpl) = crate::pack::get(kind) {
        tpl.source_descriptor()
    } else if let Some(src) = render::template_source(kind) {
        format!("{}|{}|{}", src.template_id, src.opt_rel, src.comp_file)
    } else {
        format!("unknown:{kind:?}")
    }
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

    fn spec(profile: Profile, ward: usize) -> WorkloadSpec {
        WorkloadSpec {
            profile,
            ward_size: ward,
            load_factor: 1.0,
            seed: 5,
        }
    }

    #[test]
    fn build_produces_sorted_ops_and_a_lock() {
        let w = build(&spec(Profile::Hour, 16)).expect("build");
        assert!(!w.ops.is_empty());
        assert!(w.ops.windows(2).all(|p| p[0].at <= p[1].at));
        assert_eq!(w.lock.len(), 64);
        assert_eq!(w.window, Duration::from_hours(1));
        assert_eq!(w.warmup, Duration::from_mins(5));
    }

    #[test]
    fn provisioning_carries_both_packs() {
        // The retained ECC-corpus fixtures plus every CKM-pack template.
        let w = build(&spec(Profile::Hour, 40)).expect("build");
        assert!(w.provisioning.contains(&TemplateKind::Vitals));
        assert!(w.provisioning.contains(&TemplateKind::Nested));
        assert!(w.provisioning.contains(&TemplateKind::Persistent));
        for kind in crate::pack::KINDS {
            assert!(
                w.provisioning.contains(&kind),
                "provisioning must carry CKM {kind:?}"
            );
        }
        assert_eq!(w.provisioning.len(), 3 + crate::pack::KINDS.len());
    }

    #[test]
    fn ckm_pack_shifts_the_lock_source_list() {
        // The lock's template-source list gains the five `ckm:<slug>` entries;
        // dropping them (fixtures only) must change the lock value.
        let s = spec(Profile::Hour, 20);
        let (window, warmup) = profile_timing(s.profile);
        let with_ckm = build(&s).expect("build").lock;

        let fixtures_only: Vec<String> = [
            TemplateKind::Vitals,
            TemplateKind::Nested,
            TemplateKind::Persistent,
        ]
        .into_iter()
        .map(source_descriptor)
        .collect();
        assert!(fixtures_only.iter().all(|s| !s.starts_with("ckm:")));
        let without_ckm = lock::compute(&s, window, warmup, &fixtures_only);
        assert_ne!(with_ckm, without_ckm, "the CKM pack must shift the lock");
    }

    #[test]
    fn build_is_deterministic() {
        let a = build(&spec(Profile::Hour, 20)).expect("build");
        let b = build(&spec(Profile::Hour, 20)).expect("build");
        assert_eq!(a.lock, b.lock);
        assert_eq!(a.ops.len(), b.ops.len());
    }

    fn cap_spec(ward: usize, load_factor: f64) -> WorkloadSpec {
        WorkloadSpec {
            profile: Profile::Day, // proves build_capacity forces the Hour shape.
            ward_size: ward,
            load_factor,
            seed: 11,
        }
    }

    #[test]
    fn build_capacity_is_deterministic() {
        let window = Duration::from_mins(2);
        let warmup = Duration::from_secs(15);
        let a = build_capacity(&cap_spec(20, 1.0), window, warmup).expect("capacity");
        let b = build_capacity(&cap_spec(20, 1.0), window, warmup).expect("capacity");
        assert_eq!(a.lock, b.lock);
        assert_eq!(a.ops.len(), b.ops.len());
        assert_eq!(a.window, window);
        assert_eq!(a.warmup, warmup);
    }

    #[test]
    fn build_capacity_compresses_onto_the_step_window() {
        let window = Duration::from_mins(2);
        let warmup = Duration::from_secs(15);
        let w = build_capacity(&cap_spec(40, 1.0), window, warmup).expect("capacity");
        assert!(!w.ops.is_empty());
        // Every op — bootstrap and measured alike — lands inside the short
        // window: the clinical day is time-compressed, not truncated.
        assert!(
            w.ops.iter().all(|o| o.at <= window),
            "all ops within the {window:?} step window"
        );
    }

    /// The measured-window count of the pure rate-scaled write class
    /// (`comp-create-small`: shift vitals + medication rounds, both rate-driven).
    fn measured_small_writes(w: &Workload) -> usize {
        w.ops
            .iter()
            .filter(|o| o.at >= w.warmup && o.class == crate::OpClass::CompCreateSmall)
            .count()
    }

    #[test]
    fn build_capacity_scales_ops_with_load_factor() {
        let window = Duration::from_mins(2);
        let warmup = Duration::from_secs(15);
        // Same ward + seed, only L doubles → the rate-driven op count ~doubles.
        let l1 = build_capacity(&cap_spec(80, 1.0), window, warmup).expect("capacity");
        let l2 = build_capacity(&cap_spec(80, 2.0), window, warmup).expect("capacity");
        let (n1, n2) = (measured_small_writes(&l1), measured_small_writes(&l2));
        assert!(n1 > 0 && n2 > 0, "both ladders emit rate-scaled writes");
        let ratio = n2 as f64 / n1 as f64;
        assert!(
            (1.7..=2.3).contains(&ratio),
            "2× load factor should ~double the rate-scaled op count (got {ratio:.2}: {n1} → {n2})"
        );
    }

    #[test]
    fn build_capacity_forces_the_hour_shape() {
        // A `smoke`-profile spec through build_capacity still uses rate-driven
        // (not the tiny fixed smoke) counts: a large ward yields many ops.
        let smoke_spec = WorkloadSpec {
            profile: Profile::Smoke,
            ward_size: 60,
            load_factor: 1.0,
            seed: 3,
        };
        let w = build_capacity(
            &smoke_spec,
            Duration::from_mins(2),
            Duration::from_secs(15),
        )
        .expect("capacity");
        assert!(
            measured_small_writes(&w) > 200,
            "capacity must use the Hour rate shape, not the fixed smoke counts"
        );
    }
}
