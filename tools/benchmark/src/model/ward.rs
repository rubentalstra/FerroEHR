//! The simulated ward (`docs/design/benchmark/00-workload-model.md` §1): a set
//! of patients, each one EHR, plus an implicit staff pool used to rotate the
//! composer on written compositions. Membership is deterministic in the
//! `WorkloadSpec` (subject ids and staff names are stable across runs).
//!
//! NOTE: no openEHR spec governs the benchmark ward; this is our own model.

use crate::model::WorkloadSpec;

/// A patient's role over the simulated day (register 00 §1 admission state
/// machine `admitted → on-ward → discharged`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    /// Present for the whole day; bootstrapped in warmup, stays admitted.
    Standing,
    /// Present at day start but discharged during the measured window (E9).
    Discharged,
    /// Admitted during the measured window (E1) then stays.
    NewAdmit,
}

/// One ward patient.
#[derive(Debug, Clone)]
pub struct Patient {
    /// Stable ward index (the [`crate::PlannedOp::patient`] value).
    pub index: usize,
    /// Deterministic subject id stamped into the patient's `EHR_STATUS`.
    pub subject_id: String,
    /// The composer name used on this patient's written compositions.
    pub composer: String,
    /// The patient's role over the day.
    pub role: Role,
}

/// The staff pool (composer rotation). Small and fixed; register 00 §1 models
/// staff only as event *rates*, so the pool exists purely to label composers.
const STAFF: [&str; 8] = [
    "Dr. A. Bench",
    "Dr. B. Load",
    "RN C. Steady",
    "RN D. Curve",
    "Dr. E. Percentile",
    "RN F. Tail",
    "Dr. G. Throughput",
    "RN H. Latency",
];

/// The simulated ward: `standing` present patients plus `turnover` new admits,
/// of which `turnover` standing patients are discharged during the day.
#[derive(Debug, Clone)]
pub struct Ward {
    /// Every patient touched over the day (standing + new admits).
    pub patients: Vec<Patient>,
    /// The number of patients present at day start (`WorkloadSpec::ward_size`).
    pub standing: usize,
    /// The number of admissions/discharges during the day (~10% of the ward).
    pub turnover: usize,
}

impl Ward {
    /// Build the ward for a spec. `standing = ward_size`; `turnover = round(10%
    /// of ward_size)`. The last `turnover` standing patients are discharged; an
    /// additional `turnover` patients are admitted during the window.
    #[must_use]
    pub fn new(spec: &WorkloadSpec) -> Self {
        let standing = spec.ward_size.max(1);
        // Round to nearest; a single-patient ward has no turnover. The product
        // is a small non-negative count well within usize.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let turnover = ((standing as f64) * 0.1).round() as usize;

        let mut patients = Vec::with_capacity(standing + turnover);
        for index in 0..standing {
            let role = if index >= standing - turnover && turnover > 0 {
                Role::Discharged
            } else {
                Role::Standing
            };
            patients.push(Self::patient(spec.seed, index, role));
        }
        for index in standing..standing + turnover {
            patients.push(Self::patient(spec.seed, index, Role::NewAdmit));
        }

        Self {
            patients,
            standing,
            turnover,
        }
    }

    fn patient(seed: u64, index: usize, role: Role) -> Patient {
        Patient {
            index,
            // The seed is part of the identity: both first-class SUTs enforce
            // one EHR per subject (RM ehr master04 §EHR Status), so every
            // distinct-seed run (knee ladder steps, re-runs on a non-fresh
            // database) must admit fresh subjects.
            subject_id: format!("bench-{seed:08x}-patient-{index:06}"),
            composer: STAFF[index % STAFF.len()].to_owned(),
            role,
        }
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
    use crate::Profile;

    fn spec(ward: usize) -> WorkloadSpec {
        WorkloadSpec {
            profile: Profile::Hour,
            ward_size: ward,
            load_factor: 1.0,
            seed: 1,
        }
    }

    #[test]
    fn turnover_is_about_ten_percent() {
        let w = Ward::new(&spec(20));
        assert_eq!(w.standing, 20);
        assert_eq!(w.turnover, 2);
        // 20 standing + 2 new admits.
        assert_eq!(w.patients.len(), 22);
        let discharged = w
            .patients
            .iter()
            .filter(|p| p.role == Role::Discharged)
            .count();
        let admits = w
            .patients
            .iter()
            .filter(|p| p.role == Role::NewAdmit)
            .count();
        assert_eq!(discharged, 2);
        assert_eq!(admits, 2);
    }

    #[test]
    fn subject_ids_and_indices_are_stable_and_unique() {
        let w = Ward::new(&spec(16));
        let mut ids: Vec<&str> = w.patients.iter().map(|p| p.subject_id.as_str()).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), w.patients.len(), "subject ids are unique");
        for (i, p) in w.patients.iter().enumerate() {
            assert_eq!(p.index, i);
        }
    }

    #[test]
    fn tiny_ward_has_no_turnover() {
        let w = Ward::new(&spec(1));
        assert_eq!(w.turnover, 0);
        assert_eq!(w.patients.len(), 1);
        assert_eq!(w.patients[0].role, Role::Standing);
    }
}
