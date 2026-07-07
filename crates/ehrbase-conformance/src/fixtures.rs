//! Typed, read-only access to the vendored CNF fixture corpus (design §2.2, §6):
//! `docs/specs/openehr/CNF/tests/platform/robot/_resources/test_data_sets/`.
//!
//! The whole corpus is exposed — every category, valid **and** invalid — so the
//! runner drives real openEHR payloads rather than synthetic ones wherever a
//! fixture exists. Categories:
//!
//! - `ehr/valid`, `ehr/invalid` — `EHR_STATUS` data sets;
//! - `compositions/{CANONICAL_JSON,CANONICAL_XML,FLAT,STRUCTURED,TDD,valid}`,
//!   `xml_compositions`, `flat_compositions` — committable content
//!   (FLAT/STRUCTURED/TDD are the `EhrScape` interop layer, not CNF-scored per
//!   design §3.3, but still accessible);
//! - `valid_templates/**` (121 files: 32 `.opt`, plus `.xml`/`.json` variants)
//!   and `invalid_templates/**` (21 files: `alien_tags`, `empty_file`,
//!   `multiple_elements`, `removed_mandatory_elements`, `removed_template_id`,
//!   nested, `minimal_persistent`);
//! - `contributions/{valid,invalid}/{minimal,minimal_persistent}`;
//! - `directory/`, `directory/update/`;
//! - `query/aql_queries_{valid,invalid}/{A,B,C,D}`, `query/data_load/**`,
//!   `query/expected_results/{empty_db,loaded_db}/{A,B,C,D}` (golden sets);
//! - `validation/`.
//!
//! ## RM-version adaptation (§6)
//!
//! The fixtures are authored in the RM-1.0.x era, where the canonical JSON omits
//! the `_type` discriminator on many nodes. Our RM 1.2.0 canonical-JSON layer
//! requires `_type` on abstract slots (`subject: PARTY_PROXY`, `name: DV_TEXT`,
//! …). [`adapt_ehr_status`] applies the minimal, documented overlay to make a
//! vendored `EHR_STATUS` wire-valid for RM 1.2.0 **without** changing its meaning —
//! never touching a value whose absence is the fixture's intended defect, so the
//! `invalid/` set stays invalid.

use std::path::{Path, PathBuf};

use serde_json::Value;

/// The corpus root, resolved relative to this crate.
pub const CORPUS_ROOT: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../docs/specs/openehr/CNF/tests/platform/robot/_resources/test_data_sets"
);

/// Errors accessing the fixture corpus.
#[derive(Debug, thiserror::Error)]
pub enum FixtureError {
    /// A file or directory could not be read.
    #[error("fixture I/O at {path}: {source}")]
    Io {
        /// The offending path.
        path: String,
        /// The underlying error.
        source: std::io::Error,
    },
    /// A fixture was not valid JSON.
    #[error("fixture {path} is not valid JSON: {source}")]
    Json {
        /// The offending path.
        path: String,
        /// The parse error.
        source: serde_json::Error,
    },
}

/// The corpus root as a path.
#[must_use]
pub fn root() -> PathBuf {
    PathBuf::from(CORPUS_ROOT)
}

/// Resolve a path relative to the corpus root.
#[must_use]
pub fn path(rel: &str) -> PathBuf {
    root().join(rel)
}

/// One fixture file.
#[derive(Debug, Clone)]
pub struct Fixture {
    /// The file name (with extension).
    pub name: String,
    /// The absolute path.
    pub path: PathBuf,
}

impl Fixture {
    /// The file's text content.
    ///
    /// # Errors
    /// [`FixtureError::Io`] if the file cannot be read.
    pub fn read(&self) -> Result<String, FixtureError> {
        std::fs::read_to_string(&self.path).map_err(|source| FixtureError::Io {
            path: self.path.display().to_string(),
            source,
        })
    }

    /// The file parsed as JSON.
    ///
    /// # Errors
    /// [`FixtureError`] on I/O or parse failure.
    pub fn json(&self) -> Result<Value, FixtureError> {
        let text = self.read()?;
        serde_json::from_str(&text).map_err(|source| FixtureError::Json {
            path: self.path.display().to_string(),
            source,
        })
    }

    /// The fixture's base name without extension (its logical id).
    #[must_use]
    pub fn stem(&self) -> &str {
        self.path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or(&self.name)
    }
}

/// Read a fixture file (path relative to the corpus root) as text.
///
/// # Errors
/// [`FixtureError::Io`] if the file cannot be read.
pub fn read(rel: &str) -> Result<String, FixtureError> {
    let p = path(rel);
    std::fs::read_to_string(&p).map_err(|source| FixtureError::Io {
        path: p.display().to_string(),
        source,
    })
}

/// Read a fixture file as JSON.
///
/// # Errors
/// [`FixtureError`] on I/O or parse failure.
pub fn read_json(rel: &str) -> Result<Value, FixtureError> {
    let text = read(rel)?;
    serde_json::from_str(&text).map_err(|source| FixtureError::Json {
        path: rel.to_owned(),
        source,
    })
}

/// List fixtures in a directory (relative to the corpus root) with `ext`
/// (without the dot; empty matches any file), sorted by name.
///
/// # Errors
/// [`FixtureError::Io`] if the directory cannot be read.
pub fn list(rel_dir: &str, ext: &str) -> Result<Vec<Fixture>, FixtureError> {
    let dir = path(rel_dir);
    let mut out = Vec::new();
    let entries = std::fs::read_dir(&dir).map_err(|source| FixtureError::Io {
        path: dir.display().to_string(),
        source,
    })?;
    for entry in entries.filter_map(Result::ok) {
        let p = entry.path();
        if !p.is_file() {
            continue;
        }
        if !ext.is_empty() && p.extension().and_then(|e| e.to_str()) != Some(ext) {
            continue;
        }
        let name = p
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default()
            .to_owned();
        out.push(Fixture { name, path: p });
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(out)
}

/// Recursively list fixtures under a directory (relative to the corpus root)
/// with `ext` (empty matches any), sorted by path.
///
/// # Errors
/// [`FixtureError::Io`] if a directory cannot be read.
pub fn list_recursive(rel_dir: &str, ext: &str) -> Result<Vec<Fixture>, FixtureError> {
    let mut out = Vec::new();
    walk(&path(rel_dir), ext, &mut out)?;
    out.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(out)
}

fn walk(dir: &Path, ext: &str, out: &mut Vec<Fixture>) -> Result<(), FixtureError> {
    let entries = std::fs::read_dir(dir).map_err(|source| FixtureError::Io {
        path: dir.display().to_string(),
        source,
    })?;
    for entry in entries.filter_map(Result::ok) {
        let p = entry.path();
        if p.is_dir() {
            walk(&p, ext, out)?;
        } else if p.is_file()
            && (ext.is_empty() || p.extension().and_then(|e| e.to_str()) == Some(ext))
        {
            let name = p
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or_default()
                .to_owned();
            out.push(Fixture { name, path: p });
        }
    }
    Ok(())
}

// ── Category accessors (the whole corpus) ────────────────────────────────────

/// Valid `EHR_STATUS` data sets (`ehr/valid/*.json`).
///
/// # Errors
/// [`FixtureError::Io`] on read failure.
pub fn ehr_valid() -> Result<Vec<Fixture>, FixtureError> {
    list("ehr/valid", "json")
}

/// Invalid `EHR_STATUS` data sets (`ehr/invalid/*.json`) — the negative path.
///
/// # Errors
/// [`FixtureError::Io`] on read failure.
pub fn ehr_invalid() -> Result<Vec<Fixture>, FixtureError> {
    list("ehr/invalid", "json")
}

/// Canonical-JSON compositions (`compositions/CANONICAL_JSON/*.json`).
///
/// # Errors
/// [`FixtureError::Io`] on read failure.
pub fn compositions_canonical_json() -> Result<Vec<Fixture>, FixtureError> {
    list("compositions/CANONICAL_JSON", "json")
}

/// Canonical-XML compositions (`compositions/CANONICAL_XML/*.xml`).
///
/// # Errors
/// [`FixtureError::Io`] on read failure.
pub fn compositions_canonical_xml() -> Result<Vec<Fixture>, FixtureError> {
    list("compositions/CANONICAL_XML", "xml")
}

/// Valid operational templates (`valid_templates/**/*.opt`).
///
/// # Errors
/// [`FixtureError::Io`] on read failure.
pub fn opts_valid() -> Result<Vec<Fixture>, FixtureError> {
    list_recursive("valid_templates", "opt")
}

/// Invalid operational templates (`invalid_templates/**`) — every class
/// (`alien_tags`, `empty_file`, `multiple_elements`, `removed_mandatory_elements`,
/// `removed_template_id`, nested, `minimal_persistent`).
///
/// # Errors
/// [`FixtureError::Io`] on read failure.
pub fn opts_invalid() -> Result<Vec<Fixture>, FixtureError> {
    list_recursive("invalid_templates", "")
}

/// Valid contributions (`contributions/valid/**/*.json`).
///
/// # Errors
/// [`FixtureError::Io`] on read failure.
pub fn contributions_valid() -> Result<Vec<Fixture>, FixtureError> {
    list_recursive("contributions/valid", "json")
}

/// Invalid contributions (`contributions/invalid/**/*.json`).
///
/// # Errors
/// [`FixtureError::Io`] on read failure.
pub fn contributions_invalid() -> Result<Vec<Fixture>, FixtureError> {
    list_recursive("contributions/invalid", "json")
}

/// Directory (FOLDER) fixtures (`directory/*.json`).
///
/// # Errors
/// [`FixtureError::Io`] on read failure.
pub fn directory() -> Result<Vec<Fixture>, FixtureError> {
    list("directory", "json")
}

/// Directory update fixtures (`directory/update/*.json`).
///
/// # Errors
/// [`FixtureError::Io`] on read failure.
pub fn directory_update() -> Result<Vec<Fixture>, FixtureError> {
    list("directory/update", "json")
}

/// Valid AQL queries for a group (`query/aql_queries_valid/<group>/*.json`),
/// `group` in `A`–`D`.
///
/// # Errors
/// [`FixtureError::Io`] on read failure.
pub fn aql_valid(group: &str) -> Result<Vec<Fixture>, FixtureError> {
    list(&format!("query/aql_queries_valid/{group}"), "json")
}

/// Invalid AQL queries for a group (`query/aql_queries_invalid/<group>/*.json`).
///
/// # Errors
/// [`FixtureError::Io`] on read failure.
pub fn aql_invalid(group: &str) -> Result<Vec<Fixture>, FixtureError> {
    list(&format!("query/aql_queries_invalid/{group}"), "json")
}

/// Golden query results for a group under a DB state
/// (`query/expected_results/<db>/<group>/*.json`), `db` in `empty_db`/`loaded_db`.
///
/// # Errors
/// [`FixtureError::Io`] on read failure.
pub fn aql_expected(db: &str, group: &str) -> Result<Vec<Fixture>, FixtureError> {
    list(&format!("query/expected_results/{db}/{group}"), "json")
}

/// AQL data-load compositions (`query/data_load/compositions/*.json`).
///
/// # Errors
/// [`FixtureError::Io`] on read failure.
pub fn aql_data_load_compositions() -> Result<Vec<Fixture>, FixtureError> {
    list("query/data_load/compositions", "json")
}

/// AQL data-load EHRs (`query/data_load/ehrs/*.json`).
///
/// # Errors
/// [`FixtureError::Io`] on read failure.
pub fn aql_data_load_ehrs() -> Result<Vec<Fixture>, FixtureError> {
    list("query/data_load/ehrs", "json")
}

/// Content-validation fixtures (`validation/*`).
///
/// # Errors
/// [`FixtureError::Io`] on read failure.
pub fn validation() -> Result<Vec<Fixture>, FixtureError> {
    list("validation", "")
}

// ── RM-version adaptation overlay (§6) ───────────────────────────────────────

/// Adapt a vendored `EHR_STATUS` (RM-1.0.x-era) into an RM-1.2.0-wire-valid one:
/// inject the `_type` discriminators our canonical-JSON layer requires on
/// abstract slots (`subject`, `name`, `external_ref`), and set the subject's
/// external-ref id `value` + `namespace` to the caller's unique identity so the
/// EHR is addressable by subject.
///
/// The adaptation is additive and never removes or contradicts a fixture value;
/// applied only to the `valid/` set (a missing value that *is* the defect in an
/// `invalid/` fixture is left untouched — those are posted verbatim).
///
// PORT NOTE: this is the design §6 fixture overlay, in code rather than a copied
// file, so the vendored corpus stays read-only; the change (added `_type` tags +
// unique subject) is recorded here as the provenance.
#[must_use]
pub fn adapt_ehr_status(mut status: Value, namespace: &str, subject_id: &str) -> Value {
    if let Value::Object(map) = &mut status {
        map.entry("_type")
            .or_insert_with(|| Value::String("EHR_STATUS".to_owned()));
        set_type(map.get_mut("name"), "DV_TEXT");
        if let Some(Value::Object(subject)) = map.get_mut("subject") {
            subject
                .entry("_type")
                .or_insert_with(|| Value::String("PARTY_IDENTIFIED".to_owned()));
            if let Some(Value::Object(ext)) = subject.get_mut("external_ref") {
                ext.entry("_type")
                    .or_insert_with(|| Value::String("PARTY_REF".to_owned()));
                ext.insert("namespace".to_owned(), Value::String(namespace.to_owned()));
                if let Some(Value::Object(id)) = ext.get_mut("id") {
                    id.entry("_type")
                        .or_insert_with(|| Value::String("GENERIC_ID".to_owned()));
                    id.insert("value".to_owned(), Value::String(subject_id.to_owned()));
                    id.entry("scheme")
                        .or_insert_with(|| Value::String("id_scheme".to_owned()));
                }
            }
        }
    }
    status
}

/// Set `_type` on an optional object node if it is an object missing one.
fn set_type(node: Option<&mut Value>, ty: &str) {
    if let Some(Value::Object(obj)) = node {
        obj.entry("_type")
            .or_insert_with(|| Value::String(ty.to_owned()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_whole_corpus_is_present_and_pinned() {
        // Pin the load-bearing categories so a re-vendor that drops fixtures
        // fails the build (the same guard discipline as the schedule inventory).
        assert_eq!(ehr_valid().unwrap().len(), 7, "ehr/valid");
        assert_eq!(ehr_invalid().unwrap().len(), 11, "ehr/invalid");
        assert_eq!(compositions_canonical_json().unwrap().len(), 10);
        assert_eq!(compositions_canonical_xml().unwrap().len(), 7);
        assert_eq!(opts_valid().unwrap().len(), 32, "valid_templates/**/*.opt");
        assert_eq!(opts_invalid().unwrap().len(), 21, "invalid_templates/**");
        assert_eq!(directory().unwrap().len(), 8);
        assert_eq!(directory_update().unwrap().len(), 3);
        assert_eq!(aql_valid("A").unwrap().len(), 31);
        assert_eq!(aql_valid("D").unwrap().len(), 37);
        assert!(!aql_invalid("A").unwrap().is_empty());
        assert!(!aql_expected("empty_db", "A").unwrap().is_empty());
        assert!(!aql_expected("loaded_db", "A").unwrap().is_empty());
        assert!(!contributions_valid().unwrap().is_empty());
        assert!(!contributions_invalid().unwrap().is_empty());
    }

    #[test]
    fn valid_ehr_status_fixtures_parse_as_json() {
        for f in ehr_valid().unwrap() {
            let v = f.json().unwrap_or_else(|e| panic!("{}: {e}", f.name));
            assert_eq!(v["_type"], "EHR_STATUS", "{}", f.name);
        }
    }

    #[test]
    fn adaptation_makes_subject_addressable_without_breaking_invalid() {
        let raw = read_json("ehr/valid/000_ehr_status.json").unwrap();
        let adapted = adapt_ehr_status(raw, "conformance", "subj-123");
        assert_eq!(adapted["subject"]["_type"], "PARTY_IDENTIFIED");
        assert_eq!(
            adapted["subject"]["external_ref"]["namespace"],
            "conformance"
        );
        assert_eq!(
            adapted["subject"]["external_ref"]["id"]["value"],
            "subj-123"
        );

        // An invalid fixture missing its subject stays subject-less after
        // adaptation (the defect is preserved).
        let invalid = read_json("ehr/invalid/001_ehr_status_subject_missing.json").unwrap();
        let adapted_invalid = adapt_ehr_status(invalid, "conformance", "x");
        assert!(adapted_invalid.get("subject").is_none());
    }
}
