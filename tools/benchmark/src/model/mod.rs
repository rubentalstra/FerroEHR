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
use crate::{Action, BenchError, PlannedOp, Profile, TemplateKind};
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

    // Provisioning = the templates any composition-producing op uses, in a
    // stable order (honest: only what the run actually exercises).
    let mut provisioning = Vec::new();
    for kind in [
        TemplateKind::Vitals,
        TemplateKind::Nested,
        TemplateKind::Persistent,
    ] {
        if ops.iter().any(|op| op_uses_template(&op.action, kind)) {
            provisioning.push(kind);
        }
    }

    // The extensible template-source list the lock hashes over.
    let template_sources: Vec<String> = provisioning
        .iter()
        .map(|&kind| {
            let src = render::template_source(kind);
            format!("{}|{}|{}", src.template_id, src.opt_rel, src.comp_file)
        })
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

/// Whether an action provisions or writes a composition of `kind` (the
/// contribution batch uses the vitals template).
fn op_uses_template(action: &Action, kind: TemplateKind) -> bool {
    match action {
        Action::CreateComposition { template, .. }
        | Action::UpdateComposition { template, .. }
        | Action::UploadOpt { template } => *template == kind,
        Action::CommitContribution { .. } => kind == TemplateKind::Vitals,
        _ => false,
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
    fn provisioning_covers_the_used_templates() {
        // A ward with turnover exercises all three templates (admission=nested,
        // discharge=persistent, events=vitals).
        let w = build(&spec(Profile::Hour, 40)).expect("build");
        assert!(w.provisioning.contains(&TemplateKind::Vitals));
        assert!(w.provisioning.contains(&TemplateKind::Nested));
        assert!(w.provisioning.contains(&TemplateKind::Persistent));
    }

    #[test]
    fn build_is_deterministic() {
        let a = build(&spec(Profile::Hour, 20)).expect("build");
        let b = build(&spec(Profile::Hour, 20)).expect("build");
        assert_eq!(a.lock, b.lock);
        assert_eq!(a.ops.len(), b.ops.len());
    }
}
