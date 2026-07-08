//! The capability → profile matrix and the machine-computed profile verdict
//! (design v4 §2.6, §8).
//!
//! A profile claim is **all-or-nothing per capability**: a capability passes
//! when at least one of its cases executed and none failed or errored; a
//! profile passes when every capability it requires passes. The generated
//! Conformance Statement's claim line comes from [`verdict`] — never from a
//! hand-written sentence.

use serde::Serialize;

use crate::case::{Capability, Profile};
use crate::results::{CaseStatus, RunResults};

/// The capabilities a profile requires (design §8, our curated matrix).
#[must_use]
pub const fn required_capabilities(profile: Profile) -> &'static [Capability] {
    match profile {
        Profile::Core => &[
            Capability::Adl14ArchetypeProvisioning,
            Capability::Adl14OptProvisioning,
            Capability::EhrOperations,
            Capability::EhrStatus,
            Capability::CompositionOps,
            Capability::ChangeSets,
            Capability::Versioning,
            Capability::ArchetypeValidation,
            Capability::AnonymousEhrs,
        ],
        Profile::Standard => &[
            // STANDARD = CORE plus the four below.
            Capability::Adl14ArchetypeProvisioning,
            Capability::Adl14OptProvisioning,
            Capability::EhrOperations,
            Capability::EhrStatus,
            Capability::CompositionOps,
            Capability::ChangeSets,
            Capability::Versioning,
            Capability::ArchetypeValidation,
            Capability::AnonymousEhrs,
            Capability::DirectoryOps,
            Capability::QueryProvisioning,
            Capability::AqlBasic,
            Capability::Signing,
        ],
        Profile::Options => &[Capability::AdminApi, Capability::DemographicApi],
    }
}

/// The verdict for one capability within a profile.
#[derive(Debug, Clone, Serialize)]
pub struct CapabilityVerdict {
    /// The capability (stable label, matching `CaseOutcome::capability`).
    pub capability: String,
    /// Outcomes tallied for this capability in the run.
    pub passed: usize,
    /// Failed outcomes.
    pub failed: usize,
    /// Errored outcomes (runner/SUT faults — also block a claim).
    pub errored: usize,
    /// Skipped outcomes (reported; do not pass a capability by themselves).
    pub skipped: usize,
    /// Whether the capability passes: `passed ≥ 1 && failed == 0 && errored == 0`.
    pub pass: bool,
}

/// The machine-computed verdict for one profile.
#[derive(Debug, Clone, Serialize)]
pub struct ProfileVerdict {
    /// The profile.
    pub profile: Profile,
    /// Per-required-capability verdicts, in matrix order.
    pub capabilities: Vec<CapabilityVerdict>,
    /// All-or-nothing: every required capability passes.
    pub pass: bool,
}

/// Compute the all-or-nothing verdict for `profile` over a run's outcomes.
#[must_use]
pub fn verdict(profile: Profile, results: &RunResults) -> ProfileVerdict {
    let capabilities: Vec<CapabilityVerdict> = required_capabilities(profile)
        .iter()
        .map(|cap| {
            let label = format!("{cap:?}");
            let mut v = CapabilityVerdict {
                capability: label.clone(),
                passed: 0,
                failed: 0,
                errored: 0,
                skipped: 0,
                pass: false,
            };
            for case in results.cases.iter().filter(|c| c.capability == label) {
                match case.status {
                    CaseStatus::Passed => v.passed += 1,
                    CaseStatus::Failed => v.failed += 1,
                    CaseStatus::Errored => v.errored += 1,
                    CaseStatus::Skipped => v.skipped += 1,
                }
            }
            v.pass = v.passed >= 1 && v.failed == 0 && v.errored == 0;
            v
        })
        .collect();
    let pass = capabilities.iter().all(|c| c.pass);
    ProfileVerdict {
        profile,
        capabilities,
        pass,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::results::{CaseOutcome, CorpusPin, SelectionInfo, SutIdentity};
    use crate::version::SpecVersions;

    fn outcome(capability: &str, status: CaseStatus) -> CaseOutcome {
        CaseOutcome {
            ecc_id: "ECC-EHR-001".to_owned(),
            id: "k".to_owned(),
            chapter: String::new(),
            capability: capability.to_owned(),
            profiles: vec![],
            provenance: String::new(),
            format: "json".to_owned(),
            status,
            passed_data_sets: 0,
            total_data_sets: 0,
            message: None,
            schedule_ref: String::new(),
            duration_ms: 0,
        }
    }

    fn results(cases: Vec<CaseOutcome>) -> RunResults {
        RunResults {
            sut: SutIdentity {
                base_url: "x".to_owned(),
                versions: SpecVersions::latest(),
                auth_mode: "none".to_owned(),
            },
            corpus: CorpusPin::default(),
            started: String::new(),
            selection: SelectionInfo::default(),
            cases,
        }
    }

    #[test]
    fn capability_is_all_or_nothing() {
        let r = results(vec![
            outcome("EhrOperations", CaseStatus::Passed),
            outcome("EhrOperations", CaseStatus::Failed),
        ]);
        let v = verdict(Profile::Core, &r);
        let ehr = v
            .capabilities
            .iter()
            .find(|c| c.capability == "EhrOperations")
            .expect("EhrOperations in CORE");
        assert!(!ehr.pass, "one failure fails the capability");
        assert!(!v.pass, "one failing capability fails the profile");
    }

    #[test]
    fn unevidenced_required_capability_fails_the_profile() {
        let r = results(vec![outcome("EhrOperations", CaseStatus::Passed)]);
        let v = verdict(Profile::Core, &r);
        assert!(
            !v.pass,
            "capabilities with zero executed cases are unevidenced"
        );
    }

    #[test]
    fn standard_is_a_superset_of_core() {
        let core = required_capabilities(Profile::Core);
        let standard = required_capabilities(Profile::Standard);
        for cap in core {
            assert!(standard.contains(cap), "{cap:?} missing from STANDARD");
        }
        assert!(standard.len() > core.len());
    }
}
