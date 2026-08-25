// SPDX-FileCopyrightText: FerroEHR contributors
// SPDX-License-Identifier: MIT

//! The machine-readable capability→family→tier matrix
//! (`vocab/capability_matrix.yaml`) — the Profiles book's capability×tier
//! tables as data, the input the verdict machinery computes from.

#![allow(
    clippy::disallowed_types,
    reason = "dev/verification tooling over JSON artifacts (the catalogue, results, wire \
              exchanges) — not the application (#1694); the carriers here are cfg(test)-only, so \
              #[expect] would be unfulfilled in the non-test build"
)]

use serde::{Deserialize, Serialize};

use crate::ids::{AmbiguityId, CapabilityName};
use crate::vocab::{Family, Tier};

/// Where a capability's verdict-bearing cases drive: the openEHR release's
/// own wire, or a route the product serves of its own design.
///
/// Wire-level ITS-REST conformance is always judged on `released-wire`
/// operations; an `extension` row says the CAPABILITY is verified over a
/// surface no openEHR specification governs (our own design/extension), so
/// it may never gate an openEHR profile tier — `check_realization_scoping`
/// makes that a machine finding rather than a convention.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Realization {
    /// The capability's cases drive released ITS-REST operations.
    #[default]
    ReleasedWire,
    /// The capability's cases drive routes the product serves outside the
    /// openEHR resource set (declared in `vocab/wire_surface.yaml`).
    Extension,
}

impl Realization {
    /// All variants, in vocabulary order (schema emission derives from this).
    pub const ALL: &[Realization] = &[Realization::ReleasedWire, Realization::Extension];

    /// The vocabulary token (matrix rows, certificate column).
    #[must_use]
    pub fn token(self) -> &'static str {
        match self {
            Realization::ReleasedWire => "released-wire",
            Realization::Extension => "extension",
        }
    }
}

/// A register-linked adjudication carried by a capability row: the entry that
/// decided the exception, plus the one-line reason the certificate renders.
///
/// Never free prose — the register id resolves or it is a finding.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RegisterAdjudication {
    /// The `registers/ambiguities.yaml` entry that adjudicated the exception.
    pub register: AmbiguityId,
    /// Why the exception holds (rendered verbatim; one sentence).
    pub reason: String,
}

/// One matrix row.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CapabilityEntry {
    /// The rating family this capability belongs to.
    pub family: Family,
    /// Family-scoped tier; `tier.family()` must equal `family` (checked).
    pub tier: Tier,
    /// Whether the capability is required for its tier's profile verdict.
    pub required: bool,
    /// Where the capability's cases drive (default: the released wire).
    #[serde(default)]
    pub realization: Realization,
    /// The verdict-bearing case-count FLOOR: one token case never certifies
    /// a capability, so the row records the depth its battery must keep.
    /// Floors ratchet UP only — a battery that shrinks below its floor is a
    /// `capability-depth` finding naming the shortfall.
    ///
    /// The model defaults to 0 so in-code fixtures stay terse; the published
    /// schema lists `min_cases` as REQUIRED, so every authored matrix row
    /// must state its floor (the load-time failing check).
    #[serde(default)]
    pub min_cases: usize,
    /// The adjudication for a capability EVERY one of whose catalogue cases
    /// resolves excused or deselected (an unrealized wire, an undeclared
    /// option branch): declaring a capability is the obligation to run the
    /// framework against it, so a row that can never carry executed evidence
    /// must name the register entry that decided that is acceptable.
    #[serde(default)]
    pub evidence_exception: Option<RegisterAdjudication>,
    /// The adjudication for a claimed capability the measured
    /// hospital-simulation workload does not exercise. Without it, a
    /// workload gap is a `workload-coverage` finding — the certificate may
    /// never carry an undecided `NO — catalogue gap` row.
    #[serde(default)]
    pub workload_exclusion: Option<RegisterAdjudication>,
    /// The Profiles-book (or proposal) anchor for the row.
    #[serde(default)]
    pub source: Option<String>,
}

/// The whole matrix, keyed by capability name, authored order preserved.
#[derive(Debug, Clone)]
pub struct CapabilityMatrix {
    entries: Vec<(CapabilityName, CapabilityEntry)>,
}

impl CapabilityMatrix {
    /// Look up a capability.
    #[must_use]
    pub fn get(&self, name: &CapabilityName) -> Option<&CapabilityEntry> {
        self.entries.iter().find(|(n, _)| n == name).map(|(_, e)| e)
    }

    /// All rows in authored order.
    #[must_use]
    pub fn entries(&self) -> &[(CapabilityName, CapabilityEntry)] {
        &self.entries
    }

    /// Family-scoping invariant: every row's tier belongs to its family.
    ///
    /// # Errors
    /// Returns the offending capability names.
    pub fn check_tier_scoping(&self) -> Result<(), Vec<String>> {
        let bad: Vec<String> = self
            .entries
            .iter()
            .filter(|(_, e)| e.tier.family() != e.family)
            .map(|(n, e)| {
                format!(
                    "{n}: tier {:?} is not scoped to family {:?}",
                    e.tier, e.family
                )
            })
            .collect();
        if bad.is_empty() { Ok(()) } else { Err(bad) }
    }

    /// Realization scoping: an `extension` row is verified over a surface no
    /// openEHR specification governs, so it may never be `required` — a
    /// required capability gates an openEHR profile tier, and no profile
    /// verdict may rest on our own extension routes (owner ruling
    /// 2026-07-28; no openEHR spec governs the extension surface — our own
    /// design/extension, declared in `vocab/wire_surface.yaml`).
    ///
    /// # Errors
    /// Returns the offending capability names.
    pub fn check_realization_scoping(&self) -> Result<(), Vec<String>> {
        let bad: Vec<String> = self
            .entries
            .iter()
            .filter(|(_, e)| e.required && e.realization == Realization::Extension)
            .map(|(n, _)| {
                format!(
                    "{n}: realization `extension` may not be `required` — an openEHR profile \
                     tier may not rest on a surface no openEHR specification governs"
                )
            })
            .collect();
        if bad.is_empty() { Ok(()) } else { Err(bad) }
    }
}

impl<'de> Deserialize<'de> for CapabilityMatrix {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let entries = crate::model::de::ordered_map(deserializer)?;
        Ok(Self { entries })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tier_scoping_is_enforced() {
        let m: CapabilityMatrix = serde_json::from_value(serde_json::json!({
            "EhrOperations": { "family": "Platform", "tier": "CORE", "required": true },
            "Signing": { "family": "Platform", "tier": "STANDARD", "required": false }
        }))
        .unwrap();
        assert!(m.check_tier_scoping().is_ok());

        let m: CapabilityMatrix = serde_json::from_value(serde_json::json!({
            "AuditAccountability": { "family": "Platform", "tier": "SEC-BASIC", "required": true }
        }))
        .unwrap();
        assert!(m.check_tier_scoping().is_err());
    }

    /// The row's new columns: `realization` defaults to the released wire,
    /// `min_cases` parses as the depth floor, and both adjudication blocks
    /// resolve to a register id + reason.
    #[test]
    fn row_carries_realization_floor_and_adjudications() {
        let m: CapabilityMatrix = serde_json::from_value(serde_json::json!({
            "EhrOperations": { "family": "Platform", "tier": "CORE", "required": true,
                                "min_cases": 23 },
            "Tds": { "family": "Platform", "tier": "OPTIONS", "required": false,
                      "min_cases": 4, "realization": "extension",
                      "evidence_exception": { "register": "AMB-34", "reason": "no released wire" },
                      "workload_exclusion": { "register": "AMB-170", "reason": "not in the load mix" } }
        }))
        .unwrap();
        let ehr = m
            .get(&CapabilityName::parse("EhrOperations").unwrap())
            .unwrap();
        assert_eq!(ehr.realization, Realization::ReleasedWire);
        assert_eq!(ehr.min_cases, 23);
        assert!(ehr.evidence_exception.is_none());
        let tds = m.get(&CapabilityName::parse("Tds").unwrap()).unwrap();
        assert_eq!(tds.realization, Realization::Extension);
        assert_eq!(
            tds.evidence_exception.as_ref().unwrap().register.as_str(),
            "AMB-34"
        );
        assert_eq!(
            tds.workload_exclusion.as_ref().unwrap().register.as_str(),
            "AMB-170"
        );
        assert!(m.check_realization_scoping().is_ok());
    }

    /// An `extension` row that is `required` would let an openEHR profile
    /// tier rest on a surface no openEHR specification governs.
    #[test]
    fn required_extension_row_is_rejected() {
        let m: CapabilityMatrix = serde_json::from_value(serde_json::json!({
            "MessageApi": { "family": "Platform", "tier": "CORE", "required": true,
                             "min_cases": 1, "realization": "extension" }
        }))
        .unwrap();
        assert!(m.check_realization_scoping().is_err());
    }
}
