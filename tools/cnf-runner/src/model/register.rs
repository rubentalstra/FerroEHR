//! The ambiguity register (`registers/ambiguities.yaml`) — every entry a
//! real, verified spec divergence or silence with the normative handling a
//! runner must apply. The register is normative: a runner that "resolves"
//! an ambiguity privately is non-conformant to the schedule.

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
}

impl AmbiguityEntry {
    /// Disposition-shape invariant: `option_select` entries enumerate ≥ 2
    /// option tags; other dispositions carry none.
    ///
    /// # Errors
    /// Returns a message naming the violated invariant.
    pub fn check_invariants(&self) -> Result<(), String> {
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
#[allow(clippy::unwrap_used, clippy::panic)] // test assertions/fixtures
mod tests {
    use super::*;

    #[test]
    fn disposition_shapes() {
        let r: AmbiguityRegister = serde_json::from_value(serde_json::json!({
            "AMB-4": {
                "ambiguity": "ADL 1.4 templates have no formal versioning — duplicate template_id handling is implementation-defined",
                "source": "CNF platform_test_schedule master04 NOTE",
                "handling": "sibling cases carry option tags; the ICS options declaration selects",
                "disposition": "option_select",
                "options": ["adl14-duplicate-conflict", "adl14-duplicate-versioned"]
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
