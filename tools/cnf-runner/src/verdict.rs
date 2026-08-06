//! The verdict pipeline — a **pure** function of (statement, results,
//! catalogue cases, capability matrix, ambiguity register).
//!
//! Verdicts are computed, never asserted (the schedule's core discipline):
//! the same inputs always produce the same [`VerdictReport`].
//!
//! ISO/IEC 9646 frames this as relating the ICS (the [`Statement`] claims) to
//! the campaign [`Results`] through the Abstract Test Suite (the catalogue
//! cases) and the profile matrix. The four steps mirror that:
//!
//! 1. **Static review** — is the claim itself legal against the matrix and
//!    the option register?
//! 2. **Selection** — which catalogue cases are in scope for the claim and
//!    the declared spec versions, and what is each one's effective outcome?
//! 3. **(execution)** — performed by [`crate::exec`]; its records arrive as
//!    the [`Results`] outcomes this pipeline consumes.
//! 4. **Rollup** — per-capability evidence, per-tier profile verdicts, and
//!    the coverage bound.

#![allow(
    clippy::disallowed_types,
    reason = "dev/verification tooling over JSON artifacts (the catalogue, results, wire \
              exchanges) — not the application (#1694); the carriers here are cfg(test)-only, so \
              #[expect] would be unfulfilled in the non-test build"
)]

use serde::Serialize;

use crate::ids::{CapabilityName, CaseId, OptionTag};
use crate::model::capability::CapabilityMatrix;
use crate::model::case::CaseCore;
use crate::model::register::AmbiguityRegister;
use crate::party::{OutcomeStatus, Results, Statement};
use crate::perf::{ClassVerdict, PerfClass, PerformanceCase, class_verdict};
use crate::vocab::{CaseStatus, Disposition, Family, FormatName, Tier};

/// One static-review problem with the claim (never a per-case verdict).
#[derive(Debug, Clone, Serialize)]
pub struct ReviewFinding {
    /// Human-readable description of the illegal or incomplete claim.
    pub message: String,
}

/// The evidence a capability accumulated across its selected, gating cases.
/// (`Deserialize` because the committed verdicts.json is the render input
/// of the conformance assets.)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Evidence {
    /// A selected gating case for the capability passed and none failed.
    Passed,
    /// A selected gating case for the capability failed (a failed case marks
    /// every verdict-bearing capability it names Failed).
    Failed,
    /// A selected gating case ERRORED (an inconclusive exchange — a status
    /// mapping to no declared outcome, a transport fault, a step-resolution
    /// failure) and none failed. Inconclusive is never a SUT failure
    /// (`cnf-triage` law) but it must not be absorbed into a green
    /// capability either: an errored row blocks `Passed` until the run is
    /// clean (the 2026-07-28 posture run published `CompositionOps` `passed`
    /// while both minimal-Prefer cases sat errored on a wire-visible
    /// defect — luck, not design).
    Inconclusive,
    /// Cases exist in the catalogue but none produced a gating pass/fail
    /// (all not-applicable / skipped / not driven / errored / excused) — OR
    /// the catalogue names no case at all. This is the WHOLE not-evidenced
    /// space: the former `Unrealized` and `NoCases` variants are DELETED
    /// (#626, the final #610 ratchet) because the states they excused are
    /// unrepresentable now — the claim-completeness gate refuses a claimed
    /// capability with zero verdict-bearing cases before any SUT composes,
    /// and every excused claim is realized or corrected. The
    /// accepted consequence is absolute: a party claiming a tier whose
    /// required capability it cannot evidence FAILS that tier — the upstream
    /// Java product included; no excuse arm survives in
    /// this module's `required_all_passed`.
    NotEvidenced,
}

/// The selected gating cases behind one capability's [`Evidence`], counted by
/// effective outcome.
///
/// The headline `Evidence` is a single worst-wins token, so an inconclusive
/// row inside a capability that also has passes is invisible in it (issue
/// #629: 98 errored rows contributed nothing a reader of the report could
/// see). The tally is published beside the token so an inconclusive — which
/// is never a SUT failure, but is also never evidence of conformance — is
/// always countable, and a real divergence can never hide behind an errored
/// exchange.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, serde::Deserialize)]
pub struct CapabilityTally {
    /// Gating cases that passed.
    pub passed: usize,
    /// Gating cases that failed.
    pub failed: usize,
    /// Gating cases whose exchange was inconclusive (errored).
    pub inconclusive: usize,
    /// Gating cases selected but never driven to a conclusive or inconclusive
    /// result (not-applicable, register-excused, skipped, not driven,
    /// option-deselected).
    pub unevidenced: usize,
}

impl CapabilityTally {
    /// Total selected gating cases for the capability.
    #[must_use]
    pub fn selected(&self) -> usize {
        self.passed + self.failed + self.inconclusive + self.unevidenced
    }
}

/// A profile-tier verdict.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProfileVerdict {
    /// Every required capability of the tier chain is `Passed`.
    Pass,
    /// A required capability is not `Passed`.
    Fail,
    /// The tier is not among the statement's claimed profiles.
    NotClaimed,
}

/// The SEC-BASIC security-family verdict (present only when SEC-BASIC is
/// claimed).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SecBasicVerdict {
    /// Every required Security capability is `Passed`.
    Pass,
    /// A required Security capability is not `Passed`.
    Fail,
}

/// One measured performance-class verdict.
///
/// The second machinery (§8.11 step 5): re-derived from the measurement's
/// decoded histograms against the catalogue case's thresholds — the stored
/// verdict is never trusted.
#[derive(Debug, Clone, Serialize)]
pub struct PerformanceVerdict {
    /// The performance case measured.
    pub case: CaseId,
    /// The volumetric class the case belongs to.
    pub class: PerfClass,
    /// Whether the statement claims this class.
    pub claimed: bool,
    /// The re-derived verdict.
    pub verdict: ClassVerdict,
    /// The named threshold violations behind a `not-earned` verdict.
    pub violations: Vec<String>,
}

/// The coverage bound: how many in-scope cases were selected and how many
/// were actually driven (executed to a pass/fail/errored result).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Coverage {
    /// In-scope cases (intersect the claim, pass the `applies` filter).
    pub selected: usize,
    /// Of those, cases with an executed result (`passed`/`failed`/`errored`).
    pub driven: usize,
    /// Of the driven, cases whose effective outcome is passed — the
    /// IN-SCOPE pass count the published comparison headlines (the raw
    /// record books release-dated and unclaimed surfaces a party never
    /// claimed; these three fields never do).
    #[serde(default)]
    pub passed: usize,
    /// Of the driven, cases whose effective outcome is failed.
    #[serde(default)]
    pub failed: usize,
    /// Of the driven, cases whose effective outcome is errored
    /// (inconclusive — never a failure of the behaviour under test).
    #[serde(default)]
    pub inconclusive: usize,
}

/// The computed verdict report.
///
/// # Judgment call
/// The task pins the four data fields below; `review` is added because a
/// discarded static-review step is worthless — a runner that cannot surface
/// an illegal claim cannot be honest. It never gates the computed verdicts
/// (those follow only from evidence), it makes the claim's legality visible.
#[derive(Debug, Clone, Serialize)]
pub struct VerdictReport {
    /// Static-review findings against the claim (empty ⇒ the claim is legal).
    pub review: Vec<ReviewFinding>,
    /// Per-capability evidence, in capability-matrix authored order.
    pub capabilities: Vec<(CapabilityName, Evidence)>,
    /// The selected-gating-case counts behind each capability's evidence, in
    /// the same order — so an inconclusive row is countable per capability
    /// and never disappears behind a worst-wins token.
    pub capability_tallies: Vec<(CapabilityName, CapabilityTally)>,
    /// Per-tier platform-profile verdicts (`CORE`, `STANDARD`, `OPTIONS`).
    pub profiles: Vec<(Tier, ProfileVerdict)>,
    /// The SEC-BASIC verdict when the Security profile is claimed.
    pub security: Option<SecBasicVerdict>,
    /// The measured performance-class verdicts (one per measurement in the
    /// results; empty when the campaign carried no measured runs).
    pub performance: Vec<PerformanceVerdict>,
    /// The coverage bound.
    pub coverage: Coverage,
}

/// The effective, in-scope outcome of one selected case (results rollup +
/// option/report-only metadata).
#[derive(Debug, Clone, PartialEq, Eq)]
enum Effective {
    Passed,
    Failed,
    Errored,
    NotApplicable,
    /// Every record is not-applicable AND every citation references a
    /// registered ambiguity (`AMB-n`) — a schedule-registered technology-
    /// profile exclusion (e.g. an unrealized wire), never an environmental
    /// one. Excuses a required capability instead of failing its tier.
    ExcusedByRegister,
    Skipped,
    /// Selected but no results record drove it.
    NotDriven,
    /// Option-tagged, tag not declared by the ICS.
    Deselected,
}

impl Effective {
    /// Whether this outcome was executed to a conclusive/inconclusive result.
    fn driven(&self) -> bool {
        matches!(
            self,
            Effective::Passed | Effective::Failed | Effective::Errored
        )
    }
}

/// A catalogue case that survived the selection filter.
struct Selected<'a> {
    case: &'a CaseCore,
    /// Whether the case gates verdicts (false when subject to a `report_only`
    /// ambiguity — it reports but never contributes to profile computation).
    gating: bool,
    effective: Effective,
}

// ── the pipeline entry point ─────────────────────────────────────────────────

/// Compute the verdict report from a statement + its results against the
/// catalogue, matrix, and ambiguity register. Pure and total.
#[must_use]
pub fn compute(
    statement: &Statement,
    results: &Results,
    cases: &[CaseCore],
    perf_cases: &[PerformanceCase],
    matrix: &CapabilityMatrix,
    register: &AmbiguityRegister,
) -> VerdictReport {
    let mut review = static_review(statement, cases, matrix, register);
    // The results record's technology profile is the selection filter for
    // every gating roll-up below, so a divergence from the statement's claim
    // means records were (de)selected under a DIFFERENT profile than the one
    // being certified — stale results, or a run driven without the statement.
    // Surfaced as a review finding, never silently tolerated (a narrower
    // recorded profile deselects failed rows: the false-green shape found on
    // the #288 convergence run, 2026-07-28).
    if let Some(claimed) = statement
        .tech_profiles
        .iter()
        .find(|p| p.its == results.tech_profile.its)
    {
        let mut recorded = results.tech_profile.formats.clone();
        let mut declared = claimed.formats.clone();
        recorded.sort_unstable_by_key(|f| format!("{f:?}"));
        declared.sort_unstable_by_key(|f| format!("{f:?}"));
        if recorded != declared {
            review.push(ReviewFinding {
                message: format!(
                    "results.tech_profile.formats {:?} diverges from the statement's {:?} \
                     tech-profile claim {:?} — the run's gating selection does not match the \
                     certified claim; re-run against the current statement",
                    results.tech_profile.formats, results.tech_profile.its, claimed.formats
                ),
            });
        }
    }
    // The SUT's own System-manifest advertisement, when the campaign drove
    // it, must agree with the statement's declaration. The manifest is never
    // the source of truth (the released `Options` schema has no `required`
    // list, and a server could dodge every release-dated MUST by
    // under-advertising) — but a DISAGREEMENT means either the declaration
    // or the deployment is wrong, and a certification must not rest on that.
    if let (Some(served), Some(declared)) = (
        results.restapi_specs_version.as_deref(),
        statement.spec_versions.its_rest.as_deref(),
    ) {
        let same = match (
            semver::Version::parse(served),
            semver::Version::parse(declared),
        ) {
            (Ok(observed), Ok(claimed)) => observed == claimed,
            _ => served == declared,
        };
        if !same {
            review.push(ReviewFinding {
                message: format!(
                    "the SUT's System OPTIONS manifest advertises restapi_specs_version \
                     {served} but the statement declares spec_versions.its_rest {declared} — \
                     the declaration and the deployment disagree; fix whichever is wrong \
                     (the manifest member is optional and is never the source of truth)"
                ),
            });
        }
    }
    let performance = measured_verdicts(statement, results, perf_cases, &mut review);
    let selected = select(statement, results, cases, register);

    let capabilities: Vec<(CapabilityName, Evidence)> = matrix
        .entries()
        .iter()
        .map(|(name, _)| (name.clone(), capability_evidence(name, cases, &selected)))
        .collect();
    let capability_tallies: Vec<(CapabilityName, CapabilityTally)> = matrix
        .entries()
        .iter()
        .map(|(name, _)| (name.clone(), capability_tally(name, &selected)))
        .collect();

    let profiles = platform_profiles(statement, matrix, &capabilities);
    let security = security_verdict(statement, matrix, &capabilities);

    let coverage = Coverage {
        selected: selected.len(),
        driven: selected.iter().filter(|s| s.effective.driven()).count(),
        passed: selected
            .iter()
            .filter(|s| s.effective == Effective::Passed)
            .count(),
        failed: selected
            .iter()
            .filter(|s| s.effective == Effective::Failed)
            .count(),
        inconclusive: selected
            .iter()
            .filter(|s| s.effective == Effective::Errored)
            .count(),
    };

    VerdictReport {
        review,
        capabilities,
        capability_tallies,
        profiles,
        security,
        performance,
        coverage,
    }
}

// ── step 5: measured verdicts (the second machinery) ────────────────────────

/// Re-derive every measured class verdict from the measurement records
/// (decoded histograms, never the summary fields) against the catalogue's
/// performance cases, and statically review the performance claim: a claimed
/// class needs measured evidence, a measurement needs its catalogue case,
/// and a stored verdict that does not re-derive is flagged (tamper check).
fn measured_verdicts(
    statement: &Statement,
    results: &Results,
    perf_cases: &[PerformanceCase],
    review: &mut Vec<ReviewFinding>,
) -> Vec<PerformanceVerdict> {
    let claimed_class: Option<PerfClass> = match &statement.performance {
        None => None,
        Some(claim) => match PerfClass::parse(&claim.class) {
            Ok(class) => Some(class),
            Err(e) => {
                review.push(ReviewFinding {
                    message: format!("performance claim: {e}"),
                });
                None
            }
        },
    };

    let mut verdicts = Vec::new();
    for measurement in &results.measurements {
        let Some(case) = perf_cases.iter().find(|c| c.id == measurement.case) else {
            review.push(ReviewFinding {
                message: format!(
                    "measurement references performance case {} not in the catalogue",
                    measurement.case
                ),
            });
            continue;
        };
        match class_verdict(
            case,
            measurement.offered_load_sustained,
            &measurement.operations,
        ) {
            Ok((verdict, violations)) => {
                if verdict != measurement.verdict {
                    review.push(ReviewFinding {
                        message: format!(
                            "measurement {}: stored verdict does not re-derive from the \
                             embedded histograms (stored {:?}, derived {verdict:?})",
                            measurement.case, measurement.verdict
                        ),
                    });
                }
                verdicts.push(PerformanceVerdict {
                    case: measurement.case.clone(),
                    class: case.class,
                    claimed: claimed_class == Some(case.class),
                    verdict,
                    violations,
                });
            }
            Err(e) => review.push(ReviewFinding {
                message: format!("measurement {}: {e}", measurement.case),
            }),
        }
    }

    if let Some(class) = claimed_class
        && !verdicts.iter().any(|v| v.class == class)
    {
        review.push(ReviewFinding {
            message: format!(
                "performance class {} is claimed but the results carry no measurement for it",
                class.token()
            ),
        });
    }
    verdicts
}

// ── step 1: static review ────────────────────────────────────────────────────

fn static_review(
    statement: &Statement,
    cases: &[CaseCore],
    matrix: &CapabilityMatrix,
    register: &AmbiguityRegister,
) -> Vec<ReviewFinding> {
    let mut findings = Vec::new();
    let claimed = &statement.claims.capabilities;

    // Every claimed capability resolves in the matrix.
    for cap in claimed {
        if matrix.get(cap).is_none() {
            findings.push(ReviewFinding {
                message: format!("claimed capability {cap} is not in the capability matrix"),
            });
        }
    }

    // Tier-chain completeness: claiming a profile requires claiming every
    // required capability of that tier (and of the tiers it depends on).
    let claims_profile = |t: Tier| statement.claims.profiles.contains(&t);
    if claims_profile(Tier::Core) {
        require_tier_caps(&[Tier::Core], "CORE", claimed, matrix, &mut findings);
    }
    if claims_profile(Tier::Standard) {
        if !claims_profile(Tier::Core) {
            findings.push(ReviewFinding {
                message: "STANDARD is claimed but CORE is not (STANDARD requires CORE)".to_owned(),
            });
        }
        require_tier_caps(
            &[Tier::Core, Tier::Standard],
            "STANDARD",
            claimed,
            matrix,
            &mut findings,
        );
    }
    if claims_profile(Tier::SecBasic) {
        require_tier_caps(
            &[Tier::SecBasic],
            "SEC-BASIC",
            claimed,
            matrix,
            &mut findings,
        );
    }

    // Tier-family consistency: a claimed profile tier must be represented by
    // at least one claimed capability of that tier.
    for tier in &statement.claims.profiles {
        if *tier == Tier::Options {
            continue; // OPTIONS is a catch-all pseudo-profile
        }
        let represented = claimed
            .iter()
            .filter_map(|c| matrix.get(c))
            .any(|e| e.tier == *tier);
        if !represented {
            findings.push(ReviewFinding {
                message: format!(
                    "profile {tier:?} is claimed but no claimed capability carries that tier"
                ),
            });
        }
    }

    // Every option_select register entry whose sibling cases intersect the
    // claim must have a declared option.
    for (id, entry) in register.entries() {
        if entry.disposition != Disposition::OptionSelect {
            continue;
        }
        let sibling_intersects = cases.iter().any(|c| {
            c.status == CaseStatus::Active
                && c.option.as_ref().is_some_and(|t| entry.options.contains(t))
                && intersects(&c.capabilities, claimed)
        });
        if sibling_intersects && !entry.options.iter().any(|o| statement.options.contains(o)) {
            findings.push(ReviewFinding {
                message: format!(
                    "option_select {id} governs claimed cases but the ICS declares none of its options {:?}",
                    entry.options.iter().map(OptionTag::as_str).collect::<Vec<_>>()
                ),
            });
        }
    }

    // A performance claim requires an environment reference.
    if let Some(perf) = &statement.performance
        && perf.environment_ref.trim().is_empty()
    {
        findings.push(ReviewFinding {
            message: format!(
                "performance class {:?} is claimed without an environment reference",
                perf.class
            ),
        });
    }

    findings
}

fn require_tier_caps(
    tiers: &[Tier],
    label: &str,
    claimed: &[CapabilityName],
    matrix: &CapabilityMatrix,
    findings: &mut Vec<ReviewFinding>,
) {
    for (name, entry) in matrix.entries() {
        if entry.required && tiers.contains(&entry.tier) && !claimed.contains(name) {
            findings.push(ReviewFinding {
                message: format!(
                    "{label} is claimed but required capability {name} is not claimed"
                ),
            });
        }
    }
}

// ── step 2: selection ────────────────────────────────────────────────────────

fn select<'a>(
    statement: &Statement,
    results: &Results,
    cases: &'a [CaseCore],
    register: &AmbiguityRegister,
) -> Vec<Selected<'a>> {
    let mut selected = Vec::new();
    for case in cases {
        if case.status != CaseStatus::Active {
            continue;
        }
        if !intersects(&case.capabilities, &statement.claims.capabilities) {
            continue; // out of scope for the claim
        }
        if !applies_satisfied(case, &statement.spec_versions) {
            continue; // out of scope for the declared spec versions
        }

        let gating = !is_report_only(case, register);
        let effective = effective_outcome(case, statement, results);
        selected.push(Selected {
            case,
            gating,
            effective,
        });
    }
    selected
}

/// A case applies iff every declared `applies` range is satisfied by a
/// declared spec version. An undeclared or unparsable version fails the
/// filter (the case is out of scope, not a defect the pipeline reports) —
/// the one polarity [`crate::model::case::Applies::satisfied_by`] fixes for
/// every consulting site.
fn applies_satisfied(case: &CaseCore, versions: &crate::party::SpecVersions) -> bool {
    case.applies.satisfied_by(versions)
}

fn is_report_only(case: &CaseCore, register: &AmbiguityRegister) -> bool {
    case.ambiguities.iter().any(|id| {
        register
            .get(id)
            .is_some_and(|e| e.disposition == Disposition::ReportOnly)
    })
}

/// The effective outcome of a selected case: an undeclared option tag
/// deselects it; otherwise the results records (across the tech profile's
/// formats) roll up.
fn effective_outcome(case: &CaseCore, statement: &Statement, results: &Results) -> Effective {
    if let Some(tag) = &case.option
        && !statement.options.contains(tag)
    {
        return Effective::Deselected;
    }
    rollup_results(&case.id, &results.outcomes, &results.tech_profile.formats)
}

/// Roll the (possibly multiple, per-format) results records for a case up into
/// one effective outcome: any failed → `Failed`; else any errored →
/// `Errored`; else any passed → `Passed`; else all not-applicable →
/// `NotApplicable`; else all skipped → `Skipped`; no records → `NotDriven`.
fn rollup_results(
    case: &CaseId,
    outcomes: &[crate::party::OutcomeRecord],
    formats: &[FormatName],
) -> Effective {
    let relevant: Vec<&crate::party::OutcomeRecord> = outcomes
        .iter()
        .filter(|o| &o.case == case && format_in_profile(o.format, formats))
        .collect();
    if relevant.is_empty() {
        return Effective::NotDriven;
    }
    let has = |s: OutcomeStatus| relevant.iter().any(|o| o.status == s);
    let all_na = relevant
        .iter()
        .all(|o| o.status == OutcomeStatus::NotApplicable);
    if all_na
        && relevant
            .iter()
            .all(|o| o.citation.as_deref().is_some_and(|c| c.contains("AMB-")))
    {
        return Effective::ExcusedByRegister;
    }
    if has(OutcomeStatus::Failed) {
        Effective::Failed
    } else if has(OutcomeStatus::Errored) {
        Effective::Errored
    } else if has(OutcomeStatus::Passed) {
        Effective::Passed
    } else if relevant
        .iter()
        .all(|o| o.status == OutcomeStatus::NotApplicable)
    {
        Effective::NotApplicable
    } else if relevant.iter().all(|o| o.status == OutcomeStatus::Skipped) {
        Effective::Skipped
    } else {
        Effective::NotApplicable
    }
}

/// A format-less record (`None`) matches any profile; a format-tagged record
/// matches only when that format is in the profile.
fn format_in_profile(format: Option<FormatName>, formats: &[FormatName]) -> bool {
    format.is_none_or(|f| formats.is_empty() || formats.contains(&f))
}

// ── step 4: rollup ───────────────────────────────────────────────────────────

fn capability_evidence(
    cap: &CapabilityName,
    cases: &[CaseCore],
    selected: &[Selected<'_>],
) -> Evidence {
    let relevant: Vec<&Selected<'_>> = selected
        .iter()
        .filter(|s| s.gating && s.case.capabilities.contains(cap))
        .collect();

    if relevant.iter().any(|s| s.effective == Effective::Failed) {
        return Evidence::Failed;
    }
    if relevant.iter().any(|s| s.effective == Effective::Errored) {
        return Evidence::Inconclusive;
    }
    if relevant.iter().any(|s| s.effective == Effective::Passed) {
        return Evidence::Passed;
    }
    // All-excused, nothing-selected, and no-case-at-all are ONE state now:
    // not evidenced (#626 — the excuse variants are deleted; the
    // claim-completeness gate already refuses the catalogue shapes that
    // used to need them).
    let _ = cases;
    Evidence::NotEvidenced
}

/// Count the capability's selected gating cases by effective outcome — the
/// same population `capability_evidence` reduces to one token.
fn capability_tally(cap: &CapabilityName, selected: &[Selected<'_>]) -> CapabilityTally {
    let mut tally = CapabilityTally::default();
    for case in selected
        .iter()
        .filter(|s| s.gating && s.case.capabilities.contains(cap))
    {
        match case.effective {
            Effective::Passed => tally.passed += 1,
            Effective::Failed => tally.failed += 1,
            Effective::Errored => tally.inconclusive += 1,
            Effective::NotApplicable
            | Effective::ExcusedByRegister
            | Effective::Skipped
            | Effective::NotDriven
            | Effective::Deselected => tally.unevidenced += 1,
        }
    }
    tally
}

fn evidence_of(caps: &[(CapabilityName, Evidence)], name: &CapabilityName) -> Option<Evidence> {
    caps.iter().find(|(n, _)| n == name).map(|(_, e)| *e)
}

/// The capabilities a tier's verdict is judged on — the SAME set the verdict
/// functions below quantify over.
///
/// This exists so a published count and the verdict beside it cannot describe
/// different sets. Deriving a count independently is what produced a "FAIL 5/5
/// capabilities" badge once: the count was tier-local while the verdict was
/// cumulative, and both were individually correct.
///
/// - `CORE` / `STANDARD`: the CUMULATIVE required Platform capabilities
///   (`STANDARD` = CORE + STANDARD), matching [`required_all_passed`].
/// - `OPTIONS`: the optional Platform capabilities, matching the any-passes
///   rule in [`platform_profiles`].
/// - `SEC-BASIC`: the required Security-family capabilities, matching
///   [`security_verdict`].
/// - The Enterprise tiers have no verdict rule, so they have no member set.
#[must_use]
pub fn tier_members(tier: Tier, matrix: &CapabilityMatrix) -> Vec<CapabilityName> {
    let cumulative: &[Tier] = match tier {
        Tier::Core => &[Tier::Core],
        Tier::Standard => &[Tier::Core, Tier::Standard],
        _ => &[],
    };
    matrix
        .entries()
        .iter()
        .filter(|(_, e)| match tier {
            Tier::Core | Tier::Standard => e.required && cumulative.contains(&e.tier),
            Tier::Options => e.family == Family::Platform && !e.required && e.tier == Tier::Options,
            Tier::SecBasic => e.required && e.family == Family::Security,
            Tier::EnterpriseD | Tier::EnterpriseM | Tier::EnterpriseX => false,
        })
        .map(|(name, _)| name.clone())
        .collect()
}

/// Every required capability of the given tiers is `Passed`.
fn required_all_passed(
    tiers: &[Tier],
    matrix: &CapabilityMatrix,
    caps: &[(CapabilityName, Evidence)],
) -> bool {
    matrix
        .entries()
        .iter()
        .filter(|(_, e)| e.required && tiers.contains(&e.tier))
        .all(|(name, _)| {
            // ABSOLUTE (#626): only executed passing evidence satisfies a
            // required capability — no excuse arm. A tier claimed without
            // the evidence FAILS, whoever the party is.
            matches!(evidence_of(caps, name), Some(Evidence::Passed))
        })
}

fn platform_profiles(
    statement: &Statement,
    matrix: &CapabilityMatrix,
    caps: &[(CapabilityName, Evidence)],
) -> Vec<(Tier, ProfileVerdict)> {
    let claimed = |t: Tier| statement.claims.profiles.contains(&t);
    let mut out = Vec::new();

    for tier in [Tier::Core, Tier::Standard, Tier::Options] {
        let verdict = if claimed(tier) {
            match tier {
                Tier::Core => bool_verdict(required_all_passed(&[Tier::Core], matrix, caps)),
                Tier::Standard => bool_verdict(required_all_passed(
                    &[Tier::Core, Tier::Standard],
                    matrix,
                    caps,
                )),
                // OPTIONS is the catch-all pseudo-profile: any optional
                // Platform capability passing achieves it.
                Tier::Options => bool_verdict(matrix.entries().iter().any(|(name, e)| {
                    e.family == Family::Platform
                        && !e.required
                        && evidence_of(caps, name) == Some(Evidence::Passed)
                })),
                _ => ProfileVerdict::NotClaimed,
            }
        } else {
            ProfileVerdict::NotClaimed
        };
        out.push((tier, verdict));
    }
    out
}

fn security_verdict(
    statement: &Statement,
    matrix: &CapabilityMatrix,
    caps: &[(CapabilityName, Evidence)],
) -> Option<SecBasicVerdict> {
    if !statement.claims.profiles.contains(&Tier::SecBasic) {
        return None;
    }
    let pass = matrix
        .entries()
        .iter()
        .filter(|(_, e)| e.required && e.family == Family::Security)
        .all(|(name, _)| evidence_of(caps, name) == Some(Evidence::Passed));
    Some(if pass {
        SecBasicVerdict::Pass
    } else {
        SecBasicVerdict::Fail
    })
}

fn bool_verdict(pass: bool) -> ProfileVerdict {
    if pass {
        ProfileVerdict::Pass
    } else {
        ProfileVerdict::Fail
    }
}

fn intersects(a: &[CapabilityName], b: &[CapabilityName]) -> bool {
    a.iter().any(|x| b.contains(x))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn matrix() -> CapabilityMatrix {
        serde_json::from_value(serde_json::json!({
            "EhrOperations": { "family": "Platform", "tier": "CORE", "required": true },
            "AqlBasic": { "family": "Platform", "tier": "STANDARD", "required": true },
            "SimplifiedFormats": { "family": "Platform", "tier": "OPTIONS", "required": false },
            "AuthenticatedAccess": { "family": "Security", "tier": "SEC-BASIC", "required": true }
        }))
        .unwrap()
    }

    fn register() -> AmbiguityRegister {
        serde_json::from_value(serde_json::json!({
            "AMB-5": { "ambiguity": "a", "source": "s", "handling": "h",
                        "disposition": "report_only" },
            "AMB-39": { "ambiguity": "a", "source": "s", "handling": "h",
                         "disposition": "option_select",
                         "options": ["sf-deprecated-types-supported", "sf-deprecated-types-unsupported"] }
        }))
        .unwrap()
    }

    fn case(json: serde_json::Value) -> CaseCore {
        serde_json::from_value(json).unwrap()
    }

    fn functional_case(id: &str, caps: &[&str], profiles: &[&str]) -> CaseCore {
        case(serde_json::json!({
            "id": id, "kind": "functional", "component": "EHR",
            "sm_operation": "I_EHR_SERVICE.create_ehr",
            "capabilities": caps, "profiles": profiles,
            "test_purpose": "t", "description": "d",
            "spec_refs": ["CNF platform_test_schedule master06 §x"],
            "flow": [ { "step": 1, "call": "create_ehr", "expect": "created" } ]
        }))
    }

    fn statement(caps: &[&str], profiles: &[&str], options: &[&str]) -> Statement {
        serde_json::from_value(serde_json::json!({
            "product": { "name": "p", "version": "1", "vendor": "v", "identifier": "i" },
            "schedule_release": "CNF-2.0",
            "spec_versions": { "rm": "1.2.0", "its_rest": "1.1.0" },
            "claims": { "capabilities": caps, "profiles": profiles },
            "tech_profiles": [ { "its": "its-rest", "formats": ["canonical-json"] } ],
            "options": options
        }))
        .unwrap()
    }

    fn results(outcomes: serde_json::Value) -> Results {
        let mut value = serde_json::json!({
            "sut": { "name": "s", "version": "1" },
            "runner": { "name": "cnf-runner", "version": "0", "verification_pack_status": "passed" },
            "schedule_release": "CNF-2.0",
            "tech_profile": { "its": "its-rest", "formats": ["canonical-json"] },
            "ixit_digest": "d"
        });
        if let serde_json::Value::Object(map) = &mut value {
            map.insert("outcomes".to_owned(), outcomes);
        }
        serde_json::from_value(value).unwrap()
    }

    /// The System-manifest advertisement check (#634): a served
    /// `restapi_specs_version` that disagrees with the statement's
    /// `spec_versions.its_rest` is a static-review finding; agreement (or an
    /// absent member — it is optional in the released `Options` schema)
    /// raises nothing.
    #[test]
    fn manifest_advertised_its_rest_divergence_is_a_review_finding() {
        let cases = vec![functional_case(
            "I_EHR_SERVICE.create_ehr-main",
            &["EhrOperations"],
            &["CORE"],
        )];
        let st = statement(&["EhrOperations"], &["CORE"], &[]);
        let outcomes = serde_json::json!([
            { "case": "I_EHR_SERVICE.create_ehr-main", "format": "canonical-json",
              "status": "passed", "rows_driven": 1, "rows_total": 1 }
        ]);

        // Divergence: served 1.0.3 vs declared 1.1.0 → exactly one finding.
        let mut diverging = results(outcomes.clone());
        diverging.restapi_specs_version = Some("1.0.3".to_owned());
        let report = compute(&st, &diverging, &cases, &[], &matrix(), &register());
        assert_eq!(
            report
                .review
                .iter()
                .filter(|f| f.message.contains("restapi_specs_version"))
                .count(),
            1,
            "review: {:?}",
            report.review
        );

        // Agreement (semver-equal): no finding.
        let mut agreeing = results(outcomes.clone());
        agreeing.restapi_specs_version = Some("1.1.0".to_owned());
        let report = compute(&st, &agreeing, &cases, &[], &matrix(), &register());
        assert!(
            report
                .review
                .iter()
                .all(|f| !f.message.contains("restapi_specs_version")),
            "review: {:?}",
            report.review
        );

        // Absent member (optional in the released schema): no finding.
        let report = compute(&st, &results(outcomes), &cases, &[], &matrix(), &register());
        assert!(
            report
                .review
                .iter()
                .all(|f| !f.message.contains("restapi_specs_version"))
        );
    }

    #[test]
    fn passing_core_claim_passes() {
        let cases = vec![functional_case(
            "I_EHR_SERVICE.create_ehr-main",
            &["EhrOperations"],
            &["CORE"],
        )];
        let st = statement(&["EhrOperations"], &["CORE"], &[]);
        let rs = results(serde_json::json!([
            { "case": "I_EHR_SERVICE.create_ehr-main", "format": "canonical-json",
              "status": "passed", "rows_driven": 1, "rows_total": 1 }
        ]));
        let report = compute(&st, &rs, &cases, &[], &matrix(), &register());
        assert!(report.review.is_empty(), "{:?}", report.review);
        assert_eq!(
            evidence_of(
                &report.capabilities,
                &CapabilityName::parse("EhrOperations").unwrap()
            ),
            Some(Evidence::Passed)
        );
        assert_eq!(
            report
                .profiles
                .iter()
                .find(|(t, _)| *t == Tier::Core)
                .map(|(_, v)| *v),
            Some(ProfileVerdict::Pass)
        );
        assert_eq!(report.coverage.selected, 1);
        assert_eq!(report.coverage.driven, 1);
    }

    #[test]
    fn failing_case_marks_capability_failed() {
        let cases = vec![functional_case(
            "I_EHR_SERVICE.create_ehr-main",
            &["EhrOperations"],
            &["CORE"],
        )];
        let st = statement(&["EhrOperations"], &["CORE"], &[]);
        let rs = results(serde_json::json!([
            { "case": "I_EHR_SERVICE.create_ehr-main", "format": "canonical-json",
              "status": "failed", "rows_driven": 1, "rows_total": 1, "failing_step": 1 }
        ]));
        let report = compute(&st, &rs, &cases, &[], &matrix(), &register());
        assert_eq!(
            evidence_of(
                &report.capabilities,
                &CapabilityName::parse("EhrOperations").unwrap()
            ),
            Some(Evidence::Failed)
        );
        assert_eq!(
            report
                .profiles
                .iter()
                .find(|(t, _)| *t == Tier::Core)
                .map(|(_, v)| *v),
            Some(ProfileVerdict::Fail)
        );
    }

    #[test]
    fn diverging_recorded_tech_profile_is_a_review_finding() {
        // A results record whose tech profile is NARROWER than the statement's
        // claim deselects that format's rows from every gating roll-up — the
        // false-green shape (a failed canonical-xml case invisible behind a
        // PASS). The divergence itself must surface as a review finding.
        let cases = vec![functional_case(
            "I_EHR_SERVICE.create_ehr-main",
            &["EhrOperations"],
            &["CORE"],
        )];
        let mut st = statement(&["EhrOperations"], &["CORE"], &[]);
        st.tech_profiles[0].formats = vec![FormatName::CanonicalJson, FormatName::CanonicalXml];
        let rs = results(serde_json::json!([
            { "case": "I_EHR_SERVICE.create_ehr-main", "format": "canonical-json",
              "status": "passed", "rows_driven": 1, "rows_total": 1 }
        ]));
        let report = compute(&st, &rs, &cases, &[], &matrix(), &register());
        assert!(
            report
                .review
                .iter()
                .any(|f| f.message.contains("diverges from the statement's")),
            "expected the tech-profile divergence review finding, got {:?}",
            report.review
        );
        // And the matching profile stays clean.
        st.tech_profiles[0].formats = vec![FormatName::CanonicalJson];
        let report = compute(&st, &rs, &cases, &[], &matrix(), &register());
        assert!(
            !report
                .review
                .iter()
                .any(|f| f.message.contains("diverges from the statement's")),
            "{:?}",
            report.review
        );
    }

    #[test]
    fn option_deselection_yields_no_gating_evidence() {
        // A SimplifiedFormats case tagged with the unsupported option; the ICS
        // declares the supported branch instead, so the case is deselected and
        // contributes no gating evidence.
        let mut c = functional_case(
            "SF-FLAT-deprecated_unsupported",
            &["SimplifiedFormats"],
            &["OPTIONS"],
        );
        c.option = Some(OptionTag::parse("sf-deprecated-types-unsupported").unwrap());
        c.ambiguities = vec![crate::ids::AmbiguityId::parse("AMB-39").unwrap()];
        let cases = vec![c];
        let st = statement(
            &["SimplifiedFormats"],
            &["OPTIONS"],
            &["sf-deprecated-types-supported"],
        );
        let rs = results(serde_json::json!([
            { "case": "SF-FLAT-deprecated_unsupported", "format": "canonical-json",
              "status": "passed", "rows_driven": 1, "rows_total": 1 }
        ]));
        let report = compute(&st, &rs, &cases, &[], &matrix(), &register());
        // Deselected: the result is ignored, so no gating pass exists.
        assert_eq!(
            evidence_of(
                &report.capabilities,
                &CapabilityName::parse("SimplifiedFormats").unwrap()
            ),
            Some(Evidence::NotEvidenced)
        );
        assert!(report.review.is_empty(), "{:?}", report.review);
    }

    #[test]
    fn report_only_case_never_gates() {
        // A failing case subject to a report_only ambiguity must NOT mark its
        // capability Failed.
        let mut c = functional_case(
            "I_EHR_COMPOSITION.persistent-uniqueness",
            &["EhrOperations"],
            &["CORE"],
        );
        c.ambiguities = vec![crate::ids::AmbiguityId::parse("AMB-5").unwrap()];
        let cases = vec![c];
        let st = statement(&["EhrOperations"], &["CORE"], &[]);
        let rs = results(serde_json::json!([
            { "case": "I_EHR_COMPOSITION.persistent-uniqueness", "format": "canonical-json",
              "status": "failed", "rows_driven": 1, "rows_total": 1, "failing_step": 1 }
        ]));
        let report = compute(&st, &rs, &cases, &[], &matrix(), &register());
        // No gating case, catalogue has one -> NotEvidenced, not Failed.
        assert_eq!(
            evidence_of(
                &report.capabilities,
                &CapabilityName::parse("EhrOperations").unwrap()
            ),
            Some(Evidence::NotEvidenced)
        );
    }

    #[test]
    fn no_cases_coverage_prints() {
        // No catalogue case names AqlBasic -> the whole not-evidenced space
        // is one variant now (#626); selected/driven zero. The shape itself
        // (a claim with zero cases) is refused upstream by the
        // claim-completeness gate before any SUT composes.
        let cases: Vec<CaseCore> = Vec::new();
        let st = statement(&["AqlBasic"], &[], &[]);
        let rs = results(serde_json::json!([]));
        let report = compute(&st, &rs, &cases, &[], &matrix(), &register());
        assert_eq!(
            evidence_of(
                &report.capabilities,
                &CapabilityName::parse("AqlBasic").unwrap()
            ),
            Some(Evidence::NotEvidenced)
        );
        assert_eq!(report.coverage.selected, 0);
        assert_eq!(report.coverage.driven, 0);
    }

    #[test]
    fn security_verdict_is_optional_and_computed() {
        let cases = vec![functional_case(
            "I_EHR_SERVICE.auth",
            &["AuthenticatedAccess"],
            &["SEC-BASIC"],
        )];
        let unclaimed = statement(&["EhrOperations"], &["CORE"], &[]);
        let rs = results(serde_json::json!([]));
        assert!(
            compute(&unclaimed, &rs, &cases, &[], &matrix(), &register())
                .security
                .is_none()
        );

        let claimed = statement(&["AuthenticatedAccess"], &["SEC-BASIC"], &[]);
        let rs = results(serde_json::json!([
            { "case": "I_EHR_SERVICE.auth", "format": "canonical-json",
              "status": "passed", "rows_driven": 1, "rows_total": 1 }
        ]));
        assert_eq!(
            compute(&claimed, &rs, &cases, &[], &matrix(), &register()).security,
            Some(SecBasicVerdict::Pass)
        );
    }

    #[test]
    fn standard_requires_core_static_review() {
        let cases: Vec<CaseCore> = Vec::new();
        let st = statement(&["AqlBasic"], &["STANDARD"], &[]);
        let report = compute(
            &st,
            &results(serde_json::json!([])),
            &cases,
            &[],
            &matrix(),
            &register(),
        );
        assert!(
            report
                .review
                .iter()
                .any(|f| f.message.contains("STANDARD requires CORE")),
            "{:?}",
            report.review
        );
    }
}
