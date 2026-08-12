// SPDX-FileCopyrightText: FerroEHR contributors
// SPDX-License-Identifier: MIT

//! Shields.io badge endpoints, derived from the verdict report.
//!
//! A badge is a pure function of `(VerdictReport, CapabilityMatrix, case
//! counts)` — the same inputs the verdict pipeline already takes — so it is
//! computed here, beside the tier predicates, rather than re-derived by
//! whatever publishes it.
//!
//! The count a badge shows quantifies over
//! [`crate::verdict::tier_members`], the same capability set its
//! verdict is judged on, with the same `Passed` evidence predicate. That is what
//! makes a badge like `FAIL 5/5 capabilities` unrepresentable rather than merely
//! guarded against: there is one rule, so there is nothing for a second
//! derivation to contradict.
//!
//! No openEHR spec governs badges — our own publication surface.

#![allow(
    clippy::disallowed_types,
    reason = "the JSON carriers here are cfg(test)-only fixtures over the catalogue \
              artifacts (dev/verification tooling, #1694), so #[expect] would be \
              unfulfilled in the non-test build"
)]

use serde::Serialize;

use crate::ids::CapabilityName;
use crate::model::capability::CapabilityMatrix;
use crate::party::{OutcomeStatus, Results};
use crate::perf::{ClassVerdict, PerfClass};
use crate::verdict::{
    Evidence, PerformanceVerdict, ProfileVerdict, SecBasicVerdict, VerdictReport, tier_members,
};
use crate::vocab::Tier;

/// A shields.io endpoint document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Badge {
    /// The endpoint-schema version shields.io expects.
    #[serde(rename = "schemaVersion")]
    pub schema_version: u8,
    /// The badge's left-hand label.
    pub label: String,
    /// The badge's right-hand message.
    pub message: String,
    /// The badge colour.
    pub color: String,
}

/// A badge together with the file it publishes as.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NamedBadge {
    /// File name, relative to the run's output directory.
    pub file: String,
    /// The endpoint document.
    pub badge: Badge,
}

/// The per-case status counts the overall badge reads.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CaseCounts {
    /// Cases whose outcome was `passed`.
    pub passed: usize,
    /// Cases whose outcome was `failed`.
    pub failed: usize,
    /// Cases whose outcome was `errored`.
    pub errored: usize,
}

impl CaseCounts {
    /// The counts a results document reports.
    #[must_use]
    pub fn of(results: &Results) -> Self {
        let count = |status: OutcomeStatus| {
            results
                .outcomes
                .iter()
                .filter(|o| o.status == status)
                .count()
        };
        Self {
            passed: count(OutcomeStatus::Passed),
            failed: count(OutcomeStatus::Failed),
            errored: count(OutcomeStatus::Errored),
        }
    }

    /// Cases the run actually drove — the denominator the overall badge shows.
    #[must_use]
    pub const fn driven(&self) -> usize {
        self.passed + self.failed + self.errored
    }

    /// Rows that are red in any sense.
    #[must_use]
    pub const fn red(&self) -> usize {
        self.failed + self.errored
    }
}

/// Every badge for a run, in publication order.
#[must_use]
pub fn badges(
    report: &VerdictReport,
    matrix: &CapabilityMatrix,
    cases: CaseCounts,
) -> Vec<NamedBadge> {
    let mut out: Vec<NamedBadge> = report
        .profiles
        .iter()
        .map(|(tier, verdict)| profile_badge(*tier, *verdict, report, matrix))
        .collect();
    if let Some(security) = report.security {
        out.push(security_badge(security, report, matrix));
    }
    out.push(performance_badge(&report.performance));
    out.push(overall_badge(&report.profiles, cases));
    out
}

/// The badge for one platform profile.
///
/// A `NOT CLAIMED` tier shows no count: a count is only meaningful beside a
/// computed verdict, and `NOT CLAIMED 3/5` invites the misread a bare token
/// avoids.
fn profile_badge(
    tier: Tier,
    verdict: ProfileVerdict,
    report: &VerdictReport,
    matrix: &CapabilityMatrix,
) -> NamedBadge {
    let members = tier_members(tier, matrix);
    let token = token_of(verdict);
    let noun = if tier == Tier::Options {
        "optional capabilities"
    } else {
        "capabilities"
    };
    let amount = if members.is_empty() || verdict == ProfileVerdict::NotClaimed {
        String::new()
    } else {
        format!(
            " {}/{} {noun}",
            passed_count(report, &members),
            members.len()
        )
    };
    NamedBadge {
        file: format!("badge-{}.json", slug(tier)),
        badge: Badge {
            schema_version: 1,
            label: format!("openEHR CNF {}", label(tier)),
            message: format!("{}{amount}", shout(token)),
            color: colour(token).to_owned(),
        },
    }
}

/// The SEC-BASIC badge, over the required Security-family capabilities.
fn security_badge(
    verdict: SecBasicVerdict,
    report: &VerdictReport,
    matrix: &CapabilityMatrix,
) -> NamedBadge {
    let members = tier_members(Tier::SecBasic, matrix);
    let token = match verdict {
        SecBasicVerdict::Pass => "pass",
        SecBasicVerdict::Fail => "fail",
    };
    NamedBadge {
        file: "badge-sec-basic.json".to_owned(),
        badge: Badge {
            schema_version: 1,
            label: "openEHR CNF SEC-BASIC".to_owned(),
            message: format!(
                "{} {}/{} capabilities",
                shout(token),
                passed_count(report, &members),
                members.len()
            ),
            color: colour(token).to_owned(),
        },
    }
}

/// The measured-performance badge.
///
/// It always NAMES the class it speaks about: a bare verdict is meaningless
/// without the volumetric class it was measured against. An un-measured run says
/// so, rather than letting a stale badge outlive its record.
fn performance_badge(performance: &[PerformanceVerdict]) -> NamedBadge {
    let highest = |classes: Vec<PerfClass>| -> Option<PerfClass> {
        // `PerfClass::ALL` is the ladder in ascending order, so its index IS the
        // rank — no second ordering to keep in step with it.
        PerfClass::ALL
            .iter()
            .rev()
            .find(|c| classes.contains(c))
            .copied()
    };
    let earned = highest(
        performance
            .iter()
            .filter(|p| p.verdict == ClassVerdict::Earned)
            .map(|p| p.class)
            .collect(),
    );
    let measured = highest(performance.iter().map(|p| p.class).collect());
    let (message, color) = match (earned, measured) {
        (Some(class), _) => (format!("class {} earned", class.token()), "brightgreen"),
        (None, Some(class)) => (format!("class {} not earned", class.token()), "red"),
        (None, None) => ("not measured".to_owned(), "lightgrey"),
    };
    NamedBadge {
        file: "badge-performance.json".to_owned(),
        badge: Badge {
            schema_version: 1,
            label: "openEHR CNF performance".to_owned(),
            message,
            color: color.to_owned(),
        },
    }
}

/// The overall badge.
///
/// Full green ONLY on a completely clean run: a red row anywhere — even in an
/// OPTIONS-tier capability that cannot fail CORE or STANDARD — is never an
/// acceptable resting state, because green comes from fixing the guilty
/// component and a brightgreen badge over a visible failure invites a misread. A
/// passing-tier run with red rows goes yellow and names the count.
fn overall_badge(profiles: &[(Tier, ProfileVerdict)], cases: CaseCounts) -> NamedBadge {
    let passing = |tier: Tier| {
        profiles
            .iter()
            .any(|(t, v)| *t == tier && *v == ProfileVerdict::Pass)
    };
    let tiers_pass = passing(Tier::Core) && passing(Tier::Standard);
    let driven = cases.driven();
    let passed = cases.passed;
    let (message, color) = if tiers_pass && cases.red() == 0 {
        (
            format!("CORE+STANDARD PASS · {passed}/{driven} cases"),
            "brightgreen",
        )
    } else if tiers_pass {
        (
            format!(
                "CORE+STANDARD PASS · {passed}/{driven} cases ({} failing)",
                cases.red()
            ),
            "yellow",
        )
    } else {
        (format!("NOT PASSING · {passed}/{driven} cases"), "red")
    };
    NamedBadge {
        file: "badge.json".to_owned(),
        badge: Badge {
            schema_version: 1,
            label: "openEHR conformance".to_owned(),
            message,
            color: color.to_owned(),
        },
    }
}

/// How many of `members` carry executed passing evidence.
///
/// The predicate is `Evidence::Passed` and nothing else, matching
/// `required_all_passed`: an excused or unrealized capability does not satisfy
/// its tier, so it must not be counted as if it did.
fn passed_count(report: &VerdictReport, members: &[CapabilityName]) -> usize {
    members
        .iter()
        .filter(|name| {
            report
                .capabilities
                .iter()
                .any(|(n, e)| n == *name && *e == Evidence::Passed)
        })
        .count()
}

const fn token_of(verdict: ProfileVerdict) -> &'static str {
    match verdict {
        ProfileVerdict::Pass => "pass",
        ProfileVerdict::Fail => "fail",
        ProfileVerdict::NotClaimed => "not_claimed",
    }
}

fn shout(token: &str) -> String {
    token.to_uppercase().replace('_', " ")
}

fn colour(token: &str) -> &'static str {
    match token {
        "pass" => "brightgreen",
        "fail" => "red",
        _ => "lightgrey",
    }
}

const fn slug(tier: Tier) -> &'static str {
    match tier {
        Tier::Core => "core",
        Tier::Standard => "standard",
        Tier::Options => "options",
        Tier::SecBasic => "sec-basic",
        Tier::EnterpriseD => "enterprise-d",
        Tier::EnterpriseM => "enterprise-m",
        Tier::EnterpriseX => "enterprise-x",
    }
}

const fn label(tier: Tier) -> &'static str {
    match tier {
        Tier::Core => "CORE",
        Tier::Standard => "STANDARD",
        Tier::Options => "OPTIONS",
        Tier::SecBasic => "SEC-BASIC",
        Tier::EnterpriseD => "D",
        Tier::EnterpriseM => "M",
        Tier::EnterpriseX => "X",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::verdict::Coverage;

    fn matrix() -> CapabilityMatrix {
        serde_json::from_value(serde_json::json!({
            "EhrOperations":       { "family": "Platform", "tier": "CORE",      "required": true  },
            "CompositionCrud":     { "family": "Platform", "tier": "CORE",      "required": true  },
            "AqlBasic":            { "family": "Platform", "tier": "STANDARD",  "required": true  },
            "SimplifiedFormats":   { "family": "Platform", "tier": "OPTIONS",   "required": false },
            "AuthenticatedAccess": { "family": "Security",  "tier": "SEC-BASIC", "required": true  }
        }))
        .unwrap()
    }

    fn report(
        capabilities: &[(&str, Evidence)],
        profiles: &[(Tier, ProfileVerdict)],
        security: Option<SecBasicVerdict>,
    ) -> VerdictReport {
        VerdictReport {
            review: Vec::new(),
            capabilities: capabilities
                .iter()
                .map(|(n, e)| {
                    (
                        (*n).parse::<CapabilityName>()
                            .expect("test capability name"),
                        *e,
                    )
                })
                .collect(),
            capability_tallies: Vec::new(),
            profiles: profiles.to_vec(),
            security,
            performance: Vec::new(),
            coverage: Coverage {
                selected: 0,
                driven: 0,
                passed: 0,
                failed: 0,
                inconclusive: 0,
            },
        }
    }

    fn message(badges: &[NamedBadge], file: &str) -> String {
        badges
            .iter()
            .find(|b| b.file == file)
            .unwrap_or_else(|| panic!("no {file} badge"))
            .badge
            .message
            .clone()
    }

    fn all_passed() -> Vec<(&'static str, Evidence)> {
        vec![
            ("EhrOperations", Evidence::Passed),
            ("CompositionCrud", Evidence::Passed),
            ("AqlBasic", Evidence::Passed),
            ("SimplifiedFormats", Evidence::Passed),
            ("AuthenticatedAccess", Evidence::Passed),
        ]
    }

    /// The four tier badges on a clean run, each counting the set its verdict
    /// was judged on. STANDARD is CUMULATIVE (3, not 1): a tier-local count
    /// beside a cumulative verdict is the misread this derivation exists to
    /// make impossible.
    #[test]
    fn a_clean_run_counts_the_set_each_verdict_was_judged_on() {
        let out = badges(
            &report(
                &all_passed(),
                &[
                    (Tier::Core, ProfileVerdict::Pass),
                    (Tier::Standard, ProfileVerdict::Pass),
                    (Tier::Options, ProfileVerdict::Pass),
                ],
                Some(SecBasicVerdict::Pass),
            ),
            &matrix(),
            CaseCounts {
                passed: 40,
                failed: 0,
                errored: 0,
            },
        );
        assert_eq!(message(&out, "badge-core.json"), "PASS 2/2 capabilities");
        assert_eq!(
            message(&out, "badge-standard.json"),
            "PASS 3/3 capabilities"
        );
        assert_eq!(
            message(&out, "badge-options.json"),
            "PASS 1/1 optional capabilities"
        );
        assert_eq!(
            message(&out, "badge-sec-basic.json"),
            "PASS 1/1 capabilities"
        );
        assert_eq!(
            message(&out, "badge.json"),
            "CORE+STANDARD PASS · 40/40 cases"
        );
    }

    /// A failing tier's count and its verdict agree BY CONSTRUCTION: the count
    /// falls below the total exactly when the verdict is `fail`, because both
    /// read the same members and the same evidence.
    ///
    /// This is the "FAIL 5/5 capabilities" incident, made unrepresentable. The
    /// shell derivation needed a self-check to catch it; there is nothing here
    /// for a self-check to compare.
    #[test]
    fn a_failing_tier_cannot_show_a_full_count() {
        let mut caps = all_passed();
        caps[1] = ("CompositionCrud", Evidence::Failed);
        let out = badges(
            &report(
                &caps,
                &[
                    (Tier::Core, ProfileVerdict::Fail),
                    (Tier::Standard, ProfileVerdict::Fail),
                    (Tier::Options, ProfileVerdict::Pass),
                ],
                None,
            ),
            &matrix(),
            CaseCounts {
                passed: 38,
                failed: 2,
                errored: 0,
            },
        );
        assert_eq!(message(&out, "badge-core.json"), "FAIL 1/2 capabilities");
        assert_eq!(
            message(&out, "badge-standard.json"),
            "FAIL 2/3 capabilities"
        );
        assert_eq!(
            message(&out, "badge.json"),
            "NOT PASSING · 38/40 cases",
            "a failing tier is never CORE+STANDARD PASS"
        );
    }

    /// A NOT-EVIDENCED capability does not satisfy its tier, so it is not
    /// counted as if it did — the count uses the same `Passed` predicate
    /// `required_all_passed` does, which has no excuse arm (#626).
    #[test]
    fn a_not_evidenced_capability_is_not_counted_as_satisfied() {
        let mut caps = all_passed();
        caps[0] = ("EhrOperations", Evidence::NotEvidenced);
        let out = badges(
            &report(&caps, &[(Tier::Core, ProfileVerdict::Fail)], None),
            &matrix(),
            CaseCounts::default(),
        );
        assert_eq!(message(&out, "badge-core.json"), "FAIL 1/2 capabilities");
    }

    /// A NOT CLAIMED tier shows no count: a count is only meaningful beside a
    /// computed verdict.
    #[test]
    fn a_not_claimed_tier_shows_a_bare_token() {
        let out = badges(
            &report(
                &all_passed(),
                &[(Tier::Standard, ProfileVerdict::NotClaimed)],
                None,
            ),
            &matrix(),
            CaseCounts::default(),
        );
        assert_eq!(message(&out, "badge-standard.json"), "NOT CLAIMED");
    }

    /// A passing pair of tiers with a red row anywhere is YELLOW, never green:
    /// a brightgreen badge over a visible failure is the misread that made the
    /// overall badge worth deriving carefully.
    #[test]
    fn a_red_row_under_passing_tiers_is_yellow_and_named() {
        let out = badges(
            &report(
                &all_passed(),
                &[
                    (Tier::Core, ProfileVerdict::Pass),
                    (Tier::Standard, ProfileVerdict::Pass),
                ],
                None,
            ),
            &matrix(),
            CaseCounts {
                passed: 39,
                failed: 0,
                errored: 1,
            },
        );
        let overall = out
            .iter()
            .find(|b| b.file == "badge.json")
            .expect("overall");
        assert_eq!(
            overall.badge.message,
            "CORE+STANDARD PASS · 39/40 cases (1 failing)"
        );
        assert_eq!(overall.badge.color, "yellow");
    }

    /// An unmeasured run says so, rather than leaving a stale class claim.
    #[test]
    fn performance_names_its_class_or_says_unmeasured() {
        let out = badges(
            &report(&all_passed(), &[], None),
            &matrix(),
            CaseCounts::default(),
        );
        assert_eq!(message(&out, "badge-performance.json"), "not measured");
    }
}
