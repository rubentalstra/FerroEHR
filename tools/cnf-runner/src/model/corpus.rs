//! The governed corpus manifest (`corpus/MANIFEST.yaml`).
//!
//! Every fixture and generated set is a manifest entry: verdict + defect
//! live in the manifest (never only in a filename), generated sets are
//! committed seeded deterministic recipes, and adjudication happens in a
//! register, never by silent edits.

use serde::Deserialize;

use crate::ids::{CorpusKey, RecipeName, ViewName};
use crate::vocab::{CorpusFormat, FixtureVerdict, PlaceholderPolicy};

/// The adjudicated validity of a fixture.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Validity {
    pub verdict: FixtureVerdict,
    /// Mandatory for invalid fixtures: why the payload is invalid.
    #[serde(default)]
    pub defect: Option<String>,
    /// Mandatory for invalid fixtures: the violated spec rule.
    #[serde(default)]
    pub spec_ref: Option<String>,
}

/// A named declarative projection over the set, referenced as
/// `${ds:<key>#<view>}` — evaluated over the corpus data,
/// runner-independent.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ViewDecl {
    /// Path expression over the set.
    pub select: String,
    #[serde(default, rename = "where")]
    pub where_clause: Option<String>,
    #[serde(default)]
    pub order_by: Option<String>,
}

/// A named row-to-instance synthesis recipe: name + content digest so any
/// runner can verify it executes the same recipe version.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecipeDecl {
    /// Content digest of the committed recipe implementation.
    pub digest: String,
}

/// A generated set's producing recipe.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GeneratedBy {
    pub recipe: RecipeName,
    /// Content digest of the generator.
    pub digest: String,
}

/// One corpus manifest entry.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CorpusEntry {
    /// Fixture file path, relative to the manifest (static fixtures).
    #[serde(default)]
    pub source: Option<String>,
    /// The committed, seeded, deterministic generator (generated sets).
    #[serde(default)]
    pub generated_by: Option<GeneratedBy>,
    pub format: CorpusFormat,
    #[serde(default)]
    pub rm_versions: Vec<String>,
    pub validity: Validity,
    /// The `__AUTO-GENERATED__` convention, formalized.
    #[serde(default, deserialize_with = "crate::model::de::optional_ordered_map")]
    pub placeholders: Option<Vec<(String, PlaceholderPolicy)>>,
    /// Where the payload came from and how it was re-adjudicated.
    pub provenance: String,
    #[serde(default, deserialize_with = "crate::model::de::optional_ordered_map")]
    pub views: Option<Vec<(ViewName, ViewDecl)>>,
    #[serde(default, deserialize_with = "crate::model::de::optional_ordered_map")]
    pub recipes: Option<Vec<(RecipeName, RecipeDecl)>>,
}

impl CorpusEntry {
    /// Entry-level invariants: exactly one payload origin; invalid fixtures
    /// carry defect + `spec_ref`.
    ///
    /// # Errors
    /// Returns a message naming the violated invariant.
    pub fn check_invariants(&self) -> Result<(), String> {
        match (&self.source, &self.generated_by) {
            (Some(_), Some(_)) => {
                return Err("entry declares both source and generated_by".to_owned());
            }
            (None, None) => {
                return Err("entry declares neither source nor generated_by".to_owned());
            }
            _ => {}
        }
        if self.validity.verdict == FixtureVerdict::Invalid
            && (self.validity.defect.is_none() || self.validity.spec_ref.is_none())
        {
            return Err(
                "invalid fixture must carry validity.defect and validity.spec_ref".to_owned(),
            );
        }
        Ok(())
    }

    /// A named view, if declared.
    #[must_use]
    pub fn view(&self, name: &ViewName) -> Option<&ViewDecl> {
        self.views
            .as_deref()
            .unwrap_or_default()
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, v)| v)
    }
}

/// The whole manifest, keyed by corpus key.
#[derive(Debug, Clone)]
pub struct CorpusManifest {
    entries: Vec<(CorpusKey, CorpusEntry)>,
}

impl CorpusManifest {
    /// Look up an entry.
    #[must_use]
    pub fn get(&self, key: &CorpusKey) -> Option<&CorpusEntry> {
        self.entries.iter().find(|(k, _)| k == key).map(|(_, e)| e)
    }

    /// All entries in authored order.
    #[must_use]
    pub fn entries(&self) -> &[(CorpusKey, CorpusEntry)] {
        &self.entries
    }
}

impl<'de> Deserialize<'de> for CorpusManifest {
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
    fn entry_invariants() {
        let e: CorpusEntry = serde_json::from_value(serde_json::json!({
            "source": "fixtures/ehr/invalid/007.json",
            "format": "canonical-json",
            "validity": { "verdict": "invalid",
                          "defect": "RM/Schema: is_modifiable is mandatory",
                          "spec_ref": "RM ehr §EHR_STATUS" },
            "provenance": "openEHR CNF Robot corpus @33251d2a; re-adjudicated 2026-07-21"
        }))
        .unwrap();
        assert!(e.check_invariants().is_ok());

        let e: CorpusEntry = serde_json::from_value(serde_json::json!({
            "source": "x.json",
            "format": "canonical-json",
            "validity": { "verdict": "invalid" },
            "provenance": "p"
        }))
        .unwrap();
        assert!(e.check_invariants().is_err()); // invalid without defect/spec_ref

        let e: CorpusEntry = serde_json::from_value(serde_json::json!({
            "format": "canonical-json",
            "validity": { "verdict": "valid" },
            "provenance": "p"
        }))
        .unwrap();
        assert!(e.check_invariants().is_err()); // no origin
    }
}
