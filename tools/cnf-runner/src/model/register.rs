// SPDX-FileCopyrightText: FerroEHR contributors
// SPDX-License-Identifier: MIT

//! The ambiguity register (`registers/ambiguities.yaml`) — every entry a
//! real, verified spec divergence or silence with the normative handling a
//! runner must apply.
//!
//! The register is normative: a runner that "resolves" an ambiguity privately
//! is non-conformant to the schedule.

#![allow(
    clippy::disallowed_types,
    reason = "dev/verification tooling over JSON artifacts (the catalogue, results, wire \
              exchanges) — not the application (#1694); the carriers here are cfg(test)-only, so \
              #[expect] would be unfulfilled in the non-test build"
)]

use serde::Deserialize;

use crate::ids::{AmbiguityId, OptionTag};
use crate::vocab::Disposition;

/// One register entry.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AmbiguityEntry {
    /// The divergence/silence, with its source citation.
    pub ambiguity: String,
    /// Where it was verified (spec file/section).
    pub source: String,
    /// The normative handling a runner must apply.
    pub handling: String,
    /// The machine-readable branch the pipeline takes.
    pub disposition: Disposition,
    /// For `option_select` entries: the option tags the sibling cases carry
    /// (the ICS `options` declaration selects among them).
    #[serde(default)]
    pub options: Vec<OptionTag>,
    /// The GitHub issue number of the `upstream-report` issue this ambiguity
    /// was raised as (the tracker issue labeled `upstream-report` carries the
    /// full report — what the released spec says, what this implementation
    /// does, the resolution sought; an openEHR channel key, once filed, is
    /// recorded on that issue). REQUIRED for `report_only` and `editorial`
    /// entries — a divergence the framework carries must be reported back so
    /// openEHR can fix the spec; optional but expected for the other
    /// dispositions that flag an upstream candidate. The register never hides
    /// a divergence: it documents it and points at the report that pushes the
    /// fix upstream.
    #[serde(default)]
    pub upstream_issue: Option<u64>,
}

impl AmbiguityEntry {
    /// Disposition-shape invariants:
    /// - `option_select` entries enumerate ≥ 2 option tags; other dispositions
    ///   carry none.
    /// - `report_only` and `editorial` entries MUST carry an `upstream_issue`
    ///   — a divergence the framework carries (a gating suspension, or a spec/
    ///   schedule defect the catalogue corrects) is reported back to openEHR,
    ///   never silently absorbed.
    ///
    /// # Errors
    /// Returns a message naming the violated invariant.
    pub fn check_invariants(&self) -> Result<(), String> {
        if matches!(
            self.disposition,
            Disposition::ReportOnly | Disposition::Editorial
        ) && self.upstream_issue.is_none()
        {
            return Err(format!(
                "disposition {:?} must carry an upstream_issue (the GitHub issue number of the outbound upstream-report issue)",
                self.disposition
            ));
        }
        match self.disposition {
            Disposition::OptionSelect if self.options.len() < 2 => {
                Err("option_select entry must enumerate at least two option tags".to_owned())
            }
            Disposition::OptionSelect => Ok(()),
            _ if !self.options.is_empty() => Err(format!(
                "disposition {:?} carries option tags (only option_select may)",
                self.disposition
            )),
            _ => Ok(()),
        }
    }
}

/// The whole register, keyed by `AMB-<n>`.
#[derive(Debug, Clone)]
pub struct AmbiguityRegister {
    entries: Vec<(AmbiguityId, AmbiguityEntry)>,
}

impl AmbiguityRegister {
    /// Look up an entry.
    #[must_use]
    pub fn get(&self, id: &AmbiguityId) -> Option<&AmbiguityEntry> {
        self.entries.iter().find(|(k, _)| k == id).map(|(_, e)| e)
    }

    /// All entries in authored order.
    #[must_use]
    pub fn entries(&self) -> &[(AmbiguityId, AmbiguityEntry)] {
        &self.entries
    }

    /// Whether any entry declares the option tag.
    #[must_use]
    pub fn declares_option(&self, tag: &OptionTag) -> bool {
        self.entries.iter().any(|(_, e)| e.options.contains(tag))
    }
}

impl<'de> Deserialize<'de> for AmbiguityRegister {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let entries = crate::model::de::ordered_map(deserializer)?;
        Ok(Self { entries })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disposition_shapes() {
        let r: AmbiguityRegister = serde_json::from_value(serde_json::json!({
            "AMB-4": {
                "ambiguity": "a duplicate ADL 1.4 template_id upload may be refused as a conflict or replace the stored template",
                "source": "SM i_definition_adl14.adoc §upload_opt (silent on duplicates) vs ITS-REST 409_template_already_exists.yaml",
                "handling": "sibling cases carry option tags; the ICS options declaration selects",
                "disposition": "option_select",
                "options": ["adl14-duplicate-conflict", "adl14-duplicate-replace"]
            }
        }))
        .unwrap();
        let (_, entry) = &r.entries()[0];
        assert!(entry.check_invariants().is_ok());
        assert!(r.declares_option(&OptionTag::parse("adl14-duplicate-conflict").unwrap()));

        let e: AmbiguityEntry = serde_json::from_value(serde_json::json!({
            "ambiguity": "x", "source": "s", "handling": "h",
            "disposition": "report_only", "options": ["stray"]
        }))
        .unwrap();
        assert!(e.check_invariants().is_err());
    }
}
