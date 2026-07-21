//! The machine-readable capability→family→tier matrix
//! (`vocab/capability_matrix.yaml`) — the Profiles book's capability×tier
//! tables as data, the input the verdict machinery computes from.

use serde::Deserialize;

use crate::ids::CapabilityName;
use crate::vocab::{Family, Tier};

/// One matrix row.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CapabilityEntry {
    pub family: Family,
    /// Family-scoped tier; `tier.family()` must equal `family` (checked).
    pub tier: Tier,
    /// Whether the capability is required for its tier's profile verdict.
    pub required: bool,
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
}

impl<'de> Deserialize<'de> for CapabilityMatrix {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let entries = crate::model::de::ordered_map(deserializer)?;
        Ok(Self { entries })
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic)] // test assertions/fixtures
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
}
