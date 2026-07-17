//! `workload.lock` — a stable hash over the frozen workload model
//! (`docs/design/benchmark/00-workload-model.md` §1: the event catalogue +
//! rates + profile params + template set + generator seed). Frozen before the
//! first measured run; the same model + seed always hashes identically, and any
//! change to the model (rates, sequences, template set, seed) changes the lock.
//!
//! The template-source list is passed in as an ordered `&[String]` so the lock
//! input is extensible: adding the CKM template pack at B3 shifts the lock
//! without reshaping this API.

use std::time::Duration;

use sha2::{Digest, Sha256};

use crate::model::WorkloadSpec;
use crate::model::event::{ClinicalEvent, DIR_UPDATE_PROB, MED_CORRECTION_PROB};

/// A schema marker so the lock changes if the hashing scheme itself changes.
// v2 (payload-generator semantics change): variation became constraint-aware —
// numeric/temporal leaves are now jittered in FLAT space inside each leaf's AOM
// constraint (and reassembled via `from_flat`), rather than by a raw-JSON walk.
// The generated payload bytes therefore differ from a v1 run, so the lock must
// too — a run before and after this change must never be conflated.
const LOCK_SCHEME: &str = "benchmark-workload-lock-v2";

/// Compute the workload lock. `window`/`warmup` are the derived profile timings;
/// `template_sources` is the ordered list of template-source identifiers the run
/// provisions (each a stable `template_id|opt|composition` descriptor).
#[must_use]
pub fn compute(
    spec: &WorkloadSpec,
    window: Duration,
    warmup: Duration,
    template_sources: &[String],
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(LOCK_SCHEME.as_bytes());
    hasher.update(b"\n");

    // Event catalogue: key, rate, smoke count, and the op-class sequence.
    for ev in ClinicalEvent::ALL {
        hasher.update(ev.key().as_bytes());
        hasher.update(b"|");
        hasher.update(ev.rate_per_patient_day().to_bits().to_le_bytes());
        hasher.update(ev.smoke_count().to_le_bytes());
        for step in ev.steps() {
            hasher.update(b":");
            hasher.update(step.op_class().key().as_bytes());
        }
        hasher.update(b"\n");
    }

    // Probabilistic follow-up fractions.
    hasher.update(MED_CORRECTION_PROB.to_bits().to_le_bytes());
    hasher.update(DIR_UPDATE_PROB.to_bits().to_le_bytes());

    // Profile parameters.
    hasher.update(spec.profile.key().as_bytes());
    hasher.update(b"|");
    hasher.update(window.as_secs().to_le_bytes());
    hasher.update(warmup.as_secs().to_le_bytes());
    hasher.update((spec.ward_size as u64).to_le_bytes());
    hasher.update(spec.load_factor.to_bits().to_le_bytes());

    // Template set (ordered, extensible).
    for src in template_sources {
        hasher.update(b"\nT:");
        hasher.update(src.as_bytes());
    }

    // Generator seed.
    hasher.update(b"\nSEED:");
    hasher.update(spec.seed.to_le_bytes());

    hex(&hasher.finalize())
}

/// Lowercase hex of a byte slice.
fn hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push_str(&format!("{b:02x}"));
    }
    out
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
    use crate::Profile;

    fn spec(seed: u64) -> WorkloadSpec {
        WorkloadSpec {
            profile: Profile::Hour,
            ward_size: 20,
            load_factor: 1.0,
            seed,
        }
    }

    fn sources() -> Vec<String> {
        vec![
            "composition_evaluation_test|a.opt|a.json".to_owned(),
            "nested.en.v1|b.opt|b.json".to_owned(),
        ]
    }

    fn lock(spec: &WorkloadSpec) -> String {
        compute(
            spec,
            Duration::from_hours(1),
            Duration::from_mins(5),
            &sources(),
        )
    }

    #[test]
    fn stable_across_calls() {
        assert_eq!(lock(&spec(7)), lock(&spec(7)));
        assert_eq!(lock(&spec(7)).len(), 64, "sha256 hex is 64 chars");
    }

    #[test]
    fn sensitive_to_seed() {
        assert_ne!(lock(&spec(1)), lock(&spec(2)));
    }

    #[test]
    fn sensitive_to_template_set() {
        let s = spec(1);
        let base = compute(
            &s,
            Duration::from_hours(1),
            Duration::from_mins(5),
            &sources(),
        );
        let mut extra = sources();
        extra.push("vital-signs|ckm.opt|ckm.json".to_owned());
        let with_ckm = compute(&s, Duration::from_hours(1), Duration::from_mins(5), &extra);
        assert_ne!(base, with_ckm, "adding a template must shift the lock");
    }

    #[test]
    fn sensitive_to_profile_params() {
        let s = spec(1);
        let a = compute(
            &s,
            Duration::from_hours(1),
            Duration::from_mins(5),
            &sources(),
        );
        let b = compute(
            &s,
            Duration::from_secs(120),
            Duration::from_secs(15),
            &sources(),
        );
        assert_ne!(a, b);
    }
}
