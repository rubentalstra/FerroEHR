//! The capability → profile matrix and the machine-computed profile verdict
//! (design v4 §2.6, §8).
//!
//! A capability passes when at least one of its cases executed and none failed
//! or errored. How a profile aggregates its capabilities depends on the profile
//! (`master03-profiles.adoc` preamble):
//!
//! - **CORE / STANDARD are all-or-nothing:** *"all mentioned capabilities must
//!   be met in testing"* — the profile passes iff every required capability
//!   passes.
//! - **OPTIONS is any-passes:** *"OPTIONS is obtained if any optional capability
//!   is passed in testing"* — a catch-all pseudo-profile that enumerates every
//!   testable capability outside CORE/STANDARD and is obtained when **≥1** of
//!   them passes. Each OPTIONS capability is therefore reported individually:
//!   `pass` when it has a passing case, honest **not-evidenced** (0 cases run)
//!   or **fail** (a case failed/errored) otherwise (D4, `docs/blueprint/07-cnf.md`).
//!   Wire-unreachable capabilities (`Messaging` has no ITS-REST binding;
//!   `Terminology`/`Authentication` are config-gated) simply do not contribute a
//!   pass — they never block anything (the B3 decision preserved: an any-passes
//!   profile has nothing to block anyway).
//!
//! The generated Conformance Statement's claim line comes from [`verdict`] —
//! never from a hand-written sentence.

use serde::Serialize;

use crate::case::{Capability, Profile};
use crate::results::{CaseStatus, RunResults};

/// The capabilities a profile requires (design §8, our curated matrix).
///
/// **CORE capability evidencing (D5, `docs/blueprint/07-cnf.md`).** Three CORE
/// capabilities had zero tagged cases before B5, making CORE structurally
/// unclaimable even against a perfect server. They are now evidenced by real,
/// tagged cases:
/// - `Versioning` — the version-read cases `com/get-versioned-composition*` and
///   `dir/get-versioned-directory-*` (`suites::composition`/`suites::directory`).
/// - `AnonymousEhrs` — `ehr/create-anonymous-ehr` (`suites::ehr`), the no-body
///   `POST /ehr` (`master03-profiles.adoc` §Non-Functional).
/// - `Adl14ArchetypeProvisioning` — ITS-REST has no standalone ADL 1.4 archetype
///   resource; archetypes are provisioned to the platform **inside** OPTs, so it
///   is evidenced by the OPT upload case `tpl/upload-opt-valid-opt`
///   (`suites::definition_adl14` module docs). This mirrors the schedule's
///   EHRbase-derived reality (OPTs, not raw archetypes).
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
        // OPTIONS — the full optional surface of `master03-profiles.adoc`
        // (Functional §Definitions/Demographic/Querying/Admin/Messaging + the
        // OPTIONS REST APIs), modeled in matrix order (D4). This is the
        // *reported* set, not an all-of gate: OPTIONS is any-passes (see
        // [`verdict`]), so an unimplemented capability here reads as an honest
        // "not evidenced" line rather than a blocker. Evidence today:
        // `DemographicApi` (DEM cases), `AdminApi` (the ADMIN-API physical-delete
        // cases), `Terminology` (TS cases); `Messaging` is native-API-only
        // (`SKIPPED`); the rest are modeled-but-unimplemented (not evidenced).
        Profile::Options => &[
            Capability::Adl2Provisioning,
            Capability::DemographicApi,
            Capability::AqlAdvanced,
            Capability::Terminology,
            Capability::AdminApi,
            Capability::AdminActivityReport,
            Capability::AdminPhysicalDeletion,
            Capability::AdminEhrDumpLoad,
            Capability::AdminBulkEhrLoad,
            Capability::AdminEhrArchive,
            Capability::AdminDemographicArchive,
            Capability::Messaging,
        ],
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
    /// Not-applicable outcomes (adjudicated extension / RM-version-sensitive —
    /// reported but excluded from the pass formula entirely, §3a.3).
    pub not_applicable: usize,
    /// Whether the capability passes: `passed ≥ 1 && failed == 0 && errored == 0`.
    pub pass: bool,
}

impl CapabilityVerdict {
    /// The display verdict for this capability: `"pass"` when it passes, `"fail"`
    /// when it has a failed/errored case, and `"not evidenced"` when no case ran
    /// (0 passed / 0 failed / 0 errored) — the honest OPTIONS distinction (D4)
    /// between an unimplemented optional capability and a broken one.
    #[must_use]
    pub const fn label(&self) -> &'static str {
        if self.pass {
            "pass"
        } else if self.failed > 0 || self.errored > 0 {
            "fail"
        } else {
            "not evidenced"
        }
    }
}

/// The machine-computed verdict for one profile.
#[derive(Debug, Clone, Serialize)]
pub struct ProfileVerdict {
    /// The profile.
    pub profile: Profile,
    /// Per-required-capability verdicts, in matrix order.
    pub capabilities: Vec<CapabilityVerdict>,
    /// Whether the profile is met. CORE/STANDARD are all-or-nothing (every
    /// capability passes); OPTIONS is any-passes (≥1 capability passes) —
    /// `master03-profiles.adoc` preamble (D4).
    pub pass: bool,
}

/// Compute the profile verdict over a run's outcomes: all-or-nothing for
/// CORE/STANDARD, any-passes for OPTIONS (`master03-profiles.adoc` preamble, D4).
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
                not_applicable: 0,
                pass: false,
            };
            for case in results.cases.iter().filter(|c| c.capability == label) {
                match case.status {
                    CaseStatus::Passed => v.passed += 1,
                    CaseStatus::Failed => v.failed += 1,
                    CaseStatus::Errored => v.errored += 1,
                    CaseStatus::Skipped => v.skipped += 1,
                    // NotApplicable is excluded from capability computation
                    // entirely (§3a.3): it neither evidences nor breaks a
                    // capability.
                    CaseStatus::NotApplicable => v.not_applicable += 1,
                }
            }
            v.pass = v.passed >= 1 && v.failed == 0 && v.errored == 0;
            v
        })
        .collect();
    // CORE/STANDARD: all mentioned capabilities must be met. OPTIONS: obtained
    // if any optional capability is passed (`master03-profiles.adoc` preamble).
    let pass = match profile {
        Profile::Options => capabilities.iter().any(|c| c.pass),
        Profile::Core | Profile::Standard => capabilities.iter().all(|c| c.pass),
    };
    ProfileVerdict {
        profile,
        capabilities,
        pass,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::results::{CaseOutcome, CorpusPin, ProductIdentity, SelectionInfo, SutIdentity};
    use crate::version::SpecVersions;

    fn outcome(capability: &str, status: CaseStatus) -> CaseOutcome {
        CaseOutcome {
            ecc_id: "ECC-EHR-001".to_owned(),
            id: "k".to_owned(),
            title: String::new(),
            capability: capability.to_owned(),
            profiles: vec![],
            format: "json".to_owned(),
            status,
            passed_data_sets: 0,
            total_data_sets: 0,
            message: None,
            citation: String::new(),
            schedule_ref: None,
            duration_ms: 0,
        }
    }

    fn results(cases: Vec<CaseOutcome>) -> RunResults {
        RunResults {
            sut: SutIdentity {
                base_url: "x".to_owned(),
                product: ProductIdentity::default(),
                versions: SpecVersions::latest(),
                auth_mode: "none".to_owned(),
            },
            corpus: CorpusPin::default(),
            started: String::new(),
            selection: SelectionInfo::default(),
            terminology: None,
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
    fn options_is_any_passes_and_reports_unevidenced_honestly() {
        // OPTIONS is obtained if ANY optional capability passes, even while the
        // rest are unimplemented (not evidenced) — master03-profiles.adoc.
        let r = results(vec![outcome("DemographicApi", CaseStatus::Passed)]);
        let v = verdict(Profile::Options, &r);
        assert!(v.pass, "one passing optional capability obtains OPTIONS");
        let dem = v
            .capabilities
            .iter()
            .find(|c| c.capability == "DemographicApi")
            .expect("DemographicApi in OPTIONS");
        assert_eq!(dem.label(), "pass");
        // A modeled-but-unimplemented capability reads "not evidenced", not "fail".
        let adl2 = v
            .capabilities
            .iter()
            .find(|c| c.capability == "Adl2Provisioning")
            .expect("Adl2Provisioning modeled in OPTIONS (D4)");
        assert!(!adl2.pass);
        assert_eq!(adl2.label(), "not evidenced");
    }

    #[test]
    fn options_not_obtained_when_no_optional_capability_passes() {
        // A failing optional capability does not, by itself, obtain OPTIONS; nor
        // does it block (any-passes) — but with nothing passing, OPTIONS is not
        // obtained.
        let r = results(vec![outcome("DemographicApi", CaseStatus::Failed)]);
        let v = verdict(Profile::Options, &r);
        assert!(
            !v.pass,
            "no optional capability passed → OPTIONS not obtained"
        );
    }

    #[test]
    fn not_applicable_is_excluded_from_capability_math() {
        // A capability with one passing case and one NotApplicable case still
        // passes: NotApplicable neither evidences nor breaks it (§3a.3). It is
        // tallied separately for reporting only.
        let r = results(vec![
            outcome("EhrOperations", CaseStatus::Passed),
            outcome("EhrOperations", CaseStatus::NotApplicable),
        ]);
        let v = verdict(Profile::Core, &r);
        let ehr = v
            .capabilities
            .iter()
            .find(|c| c.capability == "EhrOperations")
            .expect("EhrOperations in CORE");
        assert!(ehr.pass, "one pass + one N/A still passes the capability");
        assert_eq!(ehr.passed, 1);
        assert_eq!(ehr.not_applicable, 1);
        assert_eq!(ehr.failed, 0);
        assert_eq!(ehr.errored, 0);
    }

    #[test]
    fn not_applicable_alone_does_not_evidence_a_capability() {
        // A capability whose only case is NotApplicable is *not* evidenced (it
        // has zero passes) — it reads "not evidenced", never "pass".
        let r = results(vec![outcome("DemographicApi", CaseStatus::NotApplicable)]);
        let v = verdict(Profile::Options, &r);
        let dem = v
            .capabilities
            .iter()
            .find(|c| c.capability == "DemographicApi")
            .expect("DemographicApi in OPTIONS");
        assert!(!dem.pass);
        assert_eq!(dem.not_applicable, 1);
        assert_eq!(dem.label(), "not evidenced");
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
