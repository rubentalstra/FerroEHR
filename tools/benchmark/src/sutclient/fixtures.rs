//! Read-only access to the vendored CNF test-data corpus and the RM-version
//! adaptation overlay the benchmark needs to render valid payloads.
//!
//! Absorbed from the retired ECC fixture loader and pruned: the ECC
//! manifest/`owned:`-correction machinery is dropped, so the four corpus
//! directories the benchmark reads from are mapped to their vendored paths
//! directly here. The corpus itself stays read-only (`docs/specs/openehr/CNF/`);
//! the RM-1.2.0 adaptation is applied in code, not by copying files.

use std::path::{Path, PathBuf};

use serde_json::Value;

/// The vendored CNF corpus root, resolved relative to this crate.
const CORPUS_ROOT: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../docs/specs/openehr/CNF/tests/platform/robot/_resources/test_data_sets"
);

/// Errors accessing a fixture.
#[derive(Debug, thiserror::Error)]
pub enum FixtureError {
    /// The directory key is not one of the corpus directories the benchmark
    /// reads.
    #[error("fixture dir key `{0}` is not a known benchmark corpus directory")]
    UnknownKey(String),
    /// `file` tried to escape the corpus directory (absolute or containing
    /// `..`).
    #[error("fixture `{file}` escapes corpus directory `{key}`")]
    PathEscape {
        /// The requested directory key.
        key: String,
        /// The offending relative file path.
        file: String,
    },
    /// The file could not be read.
    #[error("fixture io: {path}: {source}")]
    Io {
        /// The absolute path attempted.
        path: String,
        /// The underlying I/O error.
        source: std::io::Error,
    },
}

/// Map a benchmark corpus directory key to its vendored corpus-relative path.
///
/// Spec grounding for each set (from the vendored CNF test-data schedule):
/// - `ehr-status.valid` → `ehr/valid` (master06 EHR API §`EHR_STATUS` valid
///   data sets; adapted to RM 1.2.0 wire by [`adapt_ehr_status`]);
/// - `composition.canonical-json` → `compositions/CANONICAL_JSON` (master07
///   Composition API §canonical JSON data sets);
/// - `template.valid` → `valid_templates` (master14 Definitions API §valid OPT
///   data sets, `valid_templates/**/*.opt`);
/// - `contribution.valid` → `contributions/valid` (master08 Contribution API
///   §valid CONTRIBUTION data sets).
fn corpus_rel(dir_key: &str) -> Option<&'static str> {
    match dir_key {
        "ehr-status.valid" => Some("ehr/valid"),
        "composition.canonical-json" => Some("compositions/CANONICAL_JSON"),
        "template.valid" => Some("valid_templates"),
        "contribution.valid" => Some("contributions/valid"),
        _ => None,
    }
}

fn read_path(path: &Path) -> Result<String, FixtureError> {
    std::fs::read_to_string(path).map_err(|source| FixtureError::Io {
        path: path.display().to_string(),
        source,
    })
}

/// Read one named file inside a corpus directory (e.g. a specific OPT under
/// `template.valid`). `file` may name a subdirectory path (`nested/…`); it is
/// rejected if it contains a `..` segment or is absolute, so it cannot escape
/// the corpus directory.
///
/// # Errors
/// [`FixtureError`] if the key is not a known benchmark corpus directory,
/// `file` escapes the directory, or the file cannot be read.
pub fn read_from(dir_key: &str, file: &str) -> Result<String, FixtureError> {
    let rel = corpus_rel(dir_key).ok_or_else(|| FixtureError::UnknownKey(dir_key.to_owned()))?;
    let candidate = Path::new(file);
    if candidate.is_absolute()
        || candidate
            .components()
            .any(|c| matches!(c, std::path::Component::ParentDir))
    {
        return Err(FixtureError::PathEscape {
            key: dir_key.to_owned(),
            file: file.to_owned(),
        });
    }
    read_path(&PathBuf::from(CORPUS_ROOT).join(rel).join(candidate))
}

// ── RM-version adaptation overlay ────────────────────────────────────────────

/// Adapt a vendored `EHR_STATUS` (RM-1.0.x-era) into an RM-1.2.0-wire-valid one:
/// inject the `_type` discriminators our canonical-JSON layer requires on
/// abstract slots (`subject`, `name`, `external_ref`), and set the subject's
/// external-ref id `value` + `namespace` to the caller's unique identity so the
/// EHR is addressable by subject.
///
/// The adaptation is additive and never removes or contradicts a fixture value;
/// applied only to the `ehr-status.valid` set.
///
// NOTE: this is the fixture overlay in code rather than a copied file, so
// the vendored corpus stays read-only; the change (added `_type` tags + unique
// subject) is recorded here as the provenance. RM ehr master04 §EHR Status:
// `EHR_STATUS.subject` is typed PARTY_SELF (monomorphic) — the subject identity
// travels on PARTY_SELF.external_ref, never as a PARTY_IDENTIFIED.
#[must_use]
pub fn adapt_ehr_status(mut status: Value, namespace: &str, subject_id: &str) -> Value {
    if let Value::Object(map) = &mut status {
        map.entry("_type")
            .or_insert_with(|| Value::String("EHR_STATUS".to_owned()));
        if let Some(name) = map.get_mut("name") {
            set_type(name, "DV_TEXT");
        }
        if let Some(Value::Object(subject)) = map.get_mut("subject") {
            subject
                .entry("_type")
                .or_insert_with(|| Value::String("PARTY_SELF".to_owned()));
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

/// Set `_type` on an object node if it is an object missing one.
fn set_type(node: &mut Value, ty: &str) {
    if let Value::Object(obj) = node {
        obj.entry("_type")
            .or_insert_with(|| Value::String(ty.to_owned()));
    }
}

#[cfg(test)]
#[allow(
    clippy::panic,
    clippy::print_stdout,
    clippy::print_stderr,
    let_underscore_drop
)] // test assertions/diagnostics/fixtures
mod tests {
    use super::*;

    #[test]
    fn read_from_reads_a_named_opt_and_rejects_escape() {
        let opt = read_from("template.valid", "time_series/time_series.opt")
            .expect("read time_series OPT via template.valid dir key");
        assert!(opt.contains("template"), "OPT looks like XML");
        assert!(matches!(
            read_from("template.valid", "../../../etc/passwd"),
            Err(FixtureError::PathEscape { .. })
        ));
    }

    #[test]
    fn read_from_rejects_an_unknown_dir_key() {
        assert!(matches!(
            read_from("no.such.dir", "x.json"),
            Err(FixtureError::UnknownKey(_))
        ));
    }

    #[test]
    fn adaptation_makes_subject_addressable() {
        let raw = serde_json::json!({
            "name": { "value": "EHR Status" },
            "subject": { "external_ref": { "id": {} } }
        });
        let adapted = adapt_ehr_status(raw, "conformance", "subj-123");
        assert_eq!(adapted["_type"], "EHR_STATUS");
        assert_eq!(adapted["name"]["_type"], "DV_TEXT");
        assert_eq!(adapted["subject"]["_type"], "PARTY_SELF");
        assert_eq!(
            adapted["subject"]["external_ref"]["namespace"],
            "conformance"
        );
        assert_eq!(
            adapted["subject"]["external_ref"]["id"]["value"],
            "subj-123"
        );
    }

    #[test]
    fn adaptation_leaves_a_subjectless_status_subjectless() {
        let invalid = serde_json::json!({ "name": { "value": "x" } });
        let adapted = adapt_ehr_status(invalid, "conformance", "x");
        assert!(adapted.get("subject").is_none());
    }
}
