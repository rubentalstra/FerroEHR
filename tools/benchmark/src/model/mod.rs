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
#[allow(clippy::duration_suboptimal_units)]
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
        assert_eq!(w.window, Duration::from_secs(3600));
        assert_eq!(w.warmup, Duration::from_secs(300));
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
}
