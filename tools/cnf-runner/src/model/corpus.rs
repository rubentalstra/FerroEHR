// SPDX-FileCopyrightText: FerroEHR contributors
// SPDX-License-Identifier: Apache-2.0

//! The governed corpus manifest (`corpus/MANIFEST.yaml`).
//!
//! Every fixture and generated set is a manifest entry: verdict + defect
//! live in the manifest (never only in a filename), generated sets are
//! committed seeded deterministic recipes, and adjudication happens in a
//! register, never by silent edits.

#![allow(
    clippy::disallowed_types,
    reason = "dev/verification tooling over JSON artifacts (the catalogue, results, wire \
              exchanges) — not the application (#1694); the carriers here are cfg(test)-only, so \
              #[expect] would be unfulfilled in the non-test build"
)]

use serde::Deserialize;

use crate::ids::{CorpusKey, RecipeName, ViewName};
use crate::vocab::{CorpusFormat, FixtureVerdict, PlaceholderPolicy};

/// The adjudicated validity of a fixture.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Validity {
    /// Whether a conformant server must accept or refuse the payload.
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
    /// Row filter applied before selection.
    #[serde(default, rename = "where")]
    pub where_clause: Option<String>,
    /// Total ordering applied to the projected rows.
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
    /// The registered recipe that produced the set.
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
    /// The payload's wire/source format.
    pub format: CorpusFormat,
    /// The openEHR template identity the payload declares (OPTs and
    /// template-bound instances) — the `openehr-template-id` header source.
    #[serde(default)]
    pub template_id: Option<String>,
    /// The RM versions the payload is valid against (empty = unconstrained).
    #[serde(default)]
    pub rm_versions: Vec<String>,
    /// The payload's adjudicated validity.
    pub validity: Validity,
    /// The `__AUTO-GENERATED__` convention, formalized.
    #[serde(default, deserialize_with = "crate::model::de::optional_ordered_map")]
    pub placeholders: Option<Vec<(String, PlaceholderPolicy)>>,
    /// Where the payload came from and how it was re-adjudicated.
    pub provenance: String,
    /// Named projections over this set, in declaration order.
    #[serde(default, deserialize_with = "crate::model::de::optional_ordered_map")]
    pub views: Option<Vec<(ViewName, ViewDecl)>>,
    /// Row-to-instance synthesis recipes this set exposes.
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
        // A `raw-json` entry exists to deliver SOURCE BYTES unrepaired, so
        // the two structural ways of having no bytes are refused here rather
        // than at drive time.
        if self.format == CorpusFormat::RawJson {
            if self.generated_by.is_some() {
                return Err(
                    "raw-json entry is generated: a recipe yields a Value, which has no \
                     byte-level form to preserve — declare a source"
                        .to_owned(),
                );
            }
            if self.views.as_deref().is_some_and(|v| !v.is_empty()) {
                return Err(
                    "raw-json entry declares views: a view projects PARSED structure, which \
                     the raw carrier deliberately does not have"
                        .to_owned(),
                );
            }
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

    /// A `raw-json` entry carries SOURCE BYTES (issue #1725), so the two
    /// structural ways of having none are refused: a recipe yields a `Value`,
    /// and a view projects parsed structure the raw carrier does not have.
    #[test]
    fn raw_json_entries_must_carry_source_bytes() {
        let entry = |extra: serde_json::Value| -> CorpusEntry {
            let mut doc = serde_json::json!({
                "format": "raw-json",
                "validity": { "verdict": "invalid",
                              "defect": "JSON: `name` appears twice in the COMPOSITION object",
                              "spec_ref": "ITS-REST Resources.md §JSON Format" },
                "provenance": "p"
            });
            if let (Some(map), Some(more)) = (doc.as_object_mut(), extra.as_object()) {
                map.extend(more.clone());
            }
            serde_json::from_value(doc).unwrap()
        };

        assert!(
            entry(serde_json::json!({ "source": "fixtures/raw/dup_member.json" }))
                .check_invariants()
                .is_ok()
        );
        assert!(
            entry(serde_json::json!({
                "generated_by": { "recipe": "bp_series", "digest": "sha256:x" }
            }))
            .check_invariants()
            .is_err()
        );
        assert!(
            entry(serde_json::json!({
                "source": "fixtures/raw/dup_member.json",
                "views": { "magnitude_ge_140_by_uid": { "select": "s" } }
            }))
            .check_invariants()
            .is_err()
        );
    }
}
