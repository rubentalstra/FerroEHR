//! Profile verdict computation — the machine realization of
//! `CNF/docs/profiles/master03-profiles.adoc`.
//!
//! The rules, verbatim from the profiles preamble: *"In order to obtain
//! `CORE` or `STANDARD` conformance, all mentioned capabilities must be met
//! in testing. The `OPTIONS` profile is a catch-all pseudo-profile that
//! covers all testable capabilities not included in `CORE` or `STANDARD`;
//! `OPTIONS` is obtained if any optional capability is passed in testing."*
//!
//! Verdicts are machine-computed from run results only — never
//! hand-asserted (honesty invariant 4, register 90 §7).

use serde::Serialize;

use crate::model::case::{Capability, Profile};

/// The capabilities a profile requires — the functional + non-functional
/// tables of `profiles/master03-profiles.adoc`.
///
/// CORE rows: ADL 1.4 Archetype provisioning, ADL 1.4 OPT provisioning, EHR
/// Operations, EHR Status, Composition Operations, Change sets, Versioning,
/// Archetype Validation (+ the DEFINITION and EHR REST APIs, which those
/// capabilities exercise) and non-functional Anonymous EHRs.
///
/// STANDARD adds: Query provisioning, Directory Operations, AQL basic (+ the
/// QUERY API) and non-functional Signing.
#[must_use]
pub fn required_capabilities(profile: Profile) -> &'static [Capability] {
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
            // STANDARD is CORE plus:
            Capability::Adl14ArchetypeProvisioning,
            Capability::Adl14OptProvisioning,
            Capability::EhrOperations,
            Capability::EhrStatus,
            Capability::CompositionOps,
            Capability::ChangeSets,
            Capability::Versioning,
            Capability::ArchetypeValidation,
            Capability::AnonymousEhrs,
            Capability::QueryProvisioning,
            Capability::DirectoryOps,
            Capability::AqlBasic,
            Capability::Signing,
        ],
        Profile::Options => OPTIONAL_CAPABILITIES,
    }
}

/// The OPTIONS surface — every capability of the profiles tables not in
/// CORE/STANDARD. Reported per capability; the OPTIONS pseudo-profile is
/// obtained if **any** of these passes.
pub const OPTIONAL_CAPABILITIES: &[Capability] = &[
    Capability::Adl2Provisioning,
    Capability::PartyOperations,
    Capability::PartyRelationshipOperations,
    Capability::AqlAdvanced,
    Capability::AqlTerminology,
    Capability::AdminActivityReport,
    Capability::AdminPhysicalDeletion,
    Capability::AdminEhrDumpLoad,
    Capability::AdminBulkEhrLoad,
    Capability::AdminEhrArchive,
    Capability::AdminDemographicArchive,
    Capability::MessagingEhrExtract,
    Capability::MessagingTds,
];

/// How a capability was evidenced in a run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityEvidence {
    /// At least one case ran and every non-adjudicated case passed.
    Passed,
    /// At least one non-adjudicated case failed.
    Failed,
    /// Cases exist but all were skipped/not-applicable (adjudicated, config-
    /// gated, or no REST binding) — the capability is not wire-evidenced.
    NotEvidenced,
    /// No case in the catalogue exercises the capability — a coverage bound,
    /// logged per honesty invariant 3.
    NoCases,
}

/// A per-capability line of the profile report.
#[derive(Debug, Clone, Serialize)]
pub struct CapabilityVerdict {
    /// The capability.
    pub capability: Capability,
    /// Cases passed / failed / skipped / not-applicable.
    pub passed: u32,
    /// Failed count.
    pub failed: u32,
    /// Skipped count (adjudicated or config-gated).
    pub skipped: u32,
    /// Not-applicable count (fairness register, foreign SUT).
    pub not_applicable: u32,
    /// The evidence classification.
    pub evidence: CapabilityEvidence,
}

impl CapabilityVerdict {
    /// Classify the evidence from the counters.
    #[must_use]
    pub fn classify(
        capability: Capability,
        passed: u32,
        failed: u32,
        skipped: u32,
        not_applicable: u32,
    ) -> Self {
        let evidence = if failed > 0 {
            CapabilityEvidence::Failed
        } else if passed > 0 {
            CapabilityEvidence::Passed
        } else if skipped > 0 || not_applicable > 0 {
            CapabilityEvidence::NotEvidenced
        } else {
            CapabilityEvidence::NoCases
        };
        Self {
            capability,
            passed,
            failed,
            skipped,
            not_applicable,
            evidence,
        }
    }
}

/// A profile verdict.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProfileVerdict {
    /// Every required capability passed (CORE/STANDARD) / at least one
    /// optional capability passed (OPTIONS).
    Pass,
    /// A required capability failed or was unevidenced.
    Fail,
}

/// Compute the CORE/STANDARD verdict: **all** required capabilities must be
/// [`CapabilityEvidence::Passed`] — an unevidenced required capability fails
/// the claim (a claim cannot rest on untested capability).
#[must_use]
pub fn all_of_verdict(lines: &[CapabilityVerdict]) -> ProfileVerdict {
    if lines
        .iter()
        .all(|l| l.evidence == CapabilityEvidence::Passed)
    {
        ProfileVerdict::Pass
    } else {
        ProfileVerdict::Fail
    }
}

/// Compute the OPTIONS verdict: obtained if **any** optional capability
/// passed (per the profiles preamble); the per-capability lines are always
/// reported individually.
#[must_use]
pub fn any_of_verdict(lines: &[CapabilityVerdict]) -> ProfileVerdict {
    if lines
        .iter()
        .any(|l| l.evidence == CapabilityEvidence::Passed)
    {
        ProfileVerdict::Pass
    } else {
        ProfileVerdict::Fail
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unevidenced_required_capability_fails_the_profile() {
        let lines = vec![
            CapabilityVerdict::classify(Capability::EhrOperations, 5, 0, 0, 0),
            CapabilityVerdict::classify(Capability::Versioning, 0, 0, 2, 0),
        ];
        assert_eq!(all_of_verdict(&lines), ProfileVerdict::Fail);
    }

    #[test]
    fn options_is_any_of() {
        let lines = vec![
            CapabilityVerdict::classify(Capability::Adl2Provisioning, 0, 0, 0, 0),
            CapabilityVerdict::classify(Capability::AdminPhysicalDeletion, 3, 0, 0, 0),
        ];
        assert_eq!(any_of_verdict(&lines), ProfileVerdict::Pass);
        let none = vec![CapabilityVerdict::classify(
            Capability::Adl2Provisioning,
            0,
            0,
            0,
            0,
        )];
        assert_eq!(any_of_verdict(&none), ProfileVerdict::Fail);
    }

    #[test]
    fn core_is_a_subset_of_standard() {
        let core = required_capabilities(Profile::Core);
        let standard = required_capabilities(Profile::Standard);
        for c in core {
            assert!(standard.contains(c), "{c:?} missing from STANDARD");
        }
    }
}
