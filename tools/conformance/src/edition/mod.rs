//! The spec-edition/version ladder (owner ruling 2026-07-13; register 90 §4).
//!
//! Different CDRs speak different editions of the same specifications: our
//! server implements the ITS-REST *development* edition (weak `W/"…"` ETags,
//! RM 1.2.0 wire); upstream EHRbase speaks Release-1.0.3-era forms and an RM
//! 1.1.0-era wire. A single-edition instrument would fail a foreign SUT on
//! edition deltas rather than defects. So every assertion separates its
//! **normative core** (what every edition mandates) from **edition-specific
//! forms**, ordered newest→oldest; the runner tries the highest first and
//! steps down, recording the satisfied level as an *edition finding* — never
//! a silent pass. A failure is only "no supported form satisfies the
//! normative core".
//!
//! CNF backing: `platform_test_schedule/master03-overview.adoc` §API
//! Conformance Test Design (NOTE): *"The supported RM version(s) by the SUT
//! should be stated in the Conformance Statement"* — the aggregated findings
//! feed exactly that Statement field.

pub mod probe;

use std::sync::Mutex;

use serde::Serialize;

/// A spec edition rung, newest first. `Ord`: a *later* edition compares
/// greater ([`Edition::Development`] > [`Edition::Release103`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Edition {
    /// ITS-REST Release 1.0.3-era forms (bare `"…"` ETags, RM 1.1.0-era
    /// wire shapes) — the older rung.
    Release103,
    /// The ITS-REST development edition (the vendored `-codegen` OAS line:
    /// weak `W/"…"` ETags, RM 1.2.0 wire) — the newest rung, tried first.
    Development,
}

impl Edition {
    /// All rungs, newest first (the ladder order).
    pub const LADDER: [Edition; 2] = [Edition::Development, Edition::Release103];

    /// The human label used in reports.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Edition::Development => "development",
            Edition::Release103 => "release-1.0.3",
        }
    }

    /// Parse a CLI/config label.
    #[must_use]
    pub fn parse(s: &str) -> Option<Edition> {
        match s {
            "development" | "dev" => Some(Edition::Development),
            "release-1.0.3" | "1.0.3" => Some(Edition::Release103),
            _ => None,
        }
    }
}

/// The per-run edition policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum EditionPolicy {
    /// Only the pinned rung is accepted — a lower-rung match is a FAILURE.
    /// Our own CI runs pin [`Edition::Development`] so the ladder can never
    /// mask a regression in ehrbase-rs (the zero-drift gate compares at the
    /// pinned level).
    Pinned(Edition),
    /// Try the highest rung first, step down, record the satisfied level as
    /// an edition finding — the default for bring-your-own-endpoint SUTs.
    Auto,
}

impl EditionPolicy {
    /// Whether `observed` satisfies this policy; `Ok(observed)` on
    /// acceptance, `Err(pinned)` naming the required rung otherwise.
    ///
    /// # Errors
    /// The pinned rung the observation missed.
    pub fn accept(self, observed: Edition) -> Result<Edition, Edition> {
        match self {
            EditionPolicy::Auto => Ok(observed),
            EditionPolicy::Pinned(pinned) if observed == pinned => Ok(observed),
            EditionPolicy::Pinned(pinned) => Err(pinned),
        }
    }
}

/// One recorded edition observation: an assertion satisfied at a
/// (usually lower) rung.
#[derive(Debug, Clone, Serialize)]
pub struct EditionFinding {
    /// The rung that matched.
    pub edition: Edition,
    /// What was observed, e.g. `"ETag emitted in the deprecated bare form"`.
    pub what: String,
}

/// The per-case edition recorder — interior-mutable because cases hold a
/// shared [`crate::engine::harness::RunContext`].
#[derive(Debug, Default)]
pub struct EditionRecorder {
    findings: Mutex<Vec<EditionFinding>>,
}

impl EditionRecorder {
    /// Record an observation at `edition`. Only sub-newest rungs are worth
    /// noting; recording [`Edition::Development`] is a no-op (the newest
    /// rung is the expected form, not a finding).
    pub fn note(&self, edition: Edition, what: impl Into<String>) {
        if edition == Edition::Development {
            return;
        }
        if let Ok(mut findings) = self.findings.lock() {
            findings.push(EditionFinding {
                edition,
                what: what.into(),
            });
        }
    }

    /// Drain the recorded findings (called by the executor after each case).
    #[must_use]
    pub fn take(&self) -> Vec<EditionFinding> {
        self.findings
            .lock()
            .map(|mut f| std::mem::take(&mut *f))
            .unwrap_or_default()
    }

    /// The lowest rung observed so far, if any — the case's `edition_level`.
    #[must_use]
    pub fn floor(&self) -> Option<Edition> {
        self.findings
            .lock()
            .ok()
            .and_then(|f| f.iter().map(|x| x.edition).min())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ladder_orders_newest_first_and_ord_matches() {
        assert!(Edition::Development > Edition::Release103);
        assert_eq!(Edition::LADDER[0], Edition::Development);
    }

    #[test]
    fn pinned_policy_rejects_lower_rungs() {
        let pinned = EditionPolicy::Pinned(Edition::Development);
        assert!(pinned.accept(Edition::Development).is_ok());
        assert_eq!(pinned.accept(Edition::Release103), Err(Edition::Development));
        assert!(EditionPolicy::Auto.accept(Edition::Release103).is_ok());
    }

    #[test]
    fn recorder_notes_only_sub_newest_and_reports_floor() {
        let rec = EditionRecorder::default();
        rec.note(Edition::Development, "expected form");
        rec.note(Edition::Release103, "bare ETag");
        assert_eq!(rec.floor(), Some(Edition::Release103));
        let drained = rec.take();
        assert_eq!(drained.len(), 1);
        assert!(rec.take().is_empty());
    }
}
