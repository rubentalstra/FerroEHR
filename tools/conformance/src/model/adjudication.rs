//! The **own-corpus adjudication register**: committed data reclassifying
//! vendored test-data defects for *our own* baseline — the counterpart of the
//! foreign-SUT fairness register ([`crate::model::fairness`]).
//!
//! Standing rule 3: a corpus/golden defect is never fixed by editing the case
//! or the golden — it is adjudicated here with a spec citation, and the case
//! reports `Skipped(adjudicated: …)`. Register 07 §4 mandated moving
//! these rulings out of suite code into committed data. Two dispositions:
//!
//! - `corpus-dialect` → the vendored data contradicts the pinned spec (e.g.
//! the 2019-era AQL goldens place `LIMIT` before `ORDER BY`, invalid under
//! the AQL 1.1 grammar) → the case is skipped with the citation.
//! - `spec-supersedes-corpus` → the pinned spec's meaning replaces the
//! corpus expectation; the case *runs* against the spec-derived
//! expectation, and the entry documents why the corpus value is not used.
//!
//! Format (`tools/conformance/adjudications/ecc-own.toml`):
//!
//! ```toml
//! [meta]
//! description = "Own-corpus adjudications (vendored-data defects, spec-cited)."
//!
//! [[entry]]
//! ecc_id = "ECC-QRY-009"
//! disposition = "corpus-dialect"
//! reason = "Golden A/106 places LIMIT before ORDER BY; AQL 1.1 grammar requires orderByClause? limitClause?."
//! citation = "QUERY AqlParser.g4 selectQuery; CNF tests corpus aql_queries_valid/A/106"
//! ```

use std::collections::HashMap;
use std::path::Path;

use serde::Deserialize;

/// How an own-corpus entry reclassifies a case.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum OwnDisposition {
    /// The vendored data is defective against the pinned spec → skip with
    /// the citation.
    CorpusDialect,
    /// The pinned spec's meaning supersedes the corpus expectation → the
    /// case runs with a spec-derived expectation; documented here.
    SpecSupersedesCorpus,
}

/// One committed adjudication entry.
#[derive(Debug, Clone, Deserialize)]
pub struct OwnAdjudication {
    /// The ECC id the entry applies to.
    pub ecc_id: String,
    /// The disposition.
    pub disposition: OwnDisposition,
    /// Why (non-empty).
    pub reason: String,
    /// The spec citation (non-empty).
    pub citation: String,
}

/// Register-format errors.
#[derive(Debug, thiserror::Error)]
pub enum OwnRegisterError {
    /// The file could not be read.
    #[error("own-adjudication register io at {path}: {source}")]
    Io {
        /// The path.
        path: String,
        /// The underlying error.
        source: std::io::Error,
    },
    /// The TOML is malformed.
    #[error("own-adjudication register parse: {0}")]
    Parse(String),
    /// An entry omits its reason or citation (honesty invariant).
    #[error("own-adjudication entry {ecc_id} must carry a non-empty reason and citation")]
    Uncited {
        /// The offending entry.
        ecc_id: String,
    },
    /// Two entries claim the same case.
    #[error("own-adjudication register has duplicate entry for {ecc_id}")]
    Duplicate {
        /// The duplicated id.
        ecc_id: String,
    },
}

#[derive(Debug, Deserialize)]
struct FileShape {
    #[expect(dead_code, reason = "meta is informational; parsed for validity")]
    meta: Option<toml::Value>,
    #[serde(default)]
    entry: Vec<OwnAdjudication>,
}

/// The loaded own-corpus register.
#[derive(Debug, Default)]
pub struct OwnRegister {
    by_id: HashMap<String, OwnAdjudication>,
}

impl OwnRegister {
    /// Load from `path`. A missing file is an error, never an empty register:
    /// a silently-empty register flips `corpus-dialect` skips into green
    /// passes and misreports the run (no-silent-fallback rule).
    ///
    /// # Errors
    /// [`OwnRegisterError`] on missing/unreadable/malformed/uncited/duplicate
    /// content.
    pub fn load(path: &Path) -> Result<Self, OwnRegisterError> {
        let text = std::fs::read_to_string(path).map_err(|source| OwnRegisterError::Io {
            path: path.display().to_string(),
            source,
        })?;
        Self::parse(&text)
    }

    /// Parse the TOML text.
    ///
    /// # Errors
    /// [`OwnRegisterError`] on malformed/uncited/duplicate content.
    pub fn parse(text: &str) -> Result<Self, OwnRegisterError> {
        let shape: FileShape =
            toml::from_str(text).map_err(|e| OwnRegisterError::Parse(e.to_string()))?;
        let mut by_id = HashMap::new();
        for entry in shape.entry {
            if entry.reason.trim().is_empty() || entry.citation.trim().is_empty() {
                return Err(OwnRegisterError::Uncited {
                    ecc_id: entry.ecc_id,
                });
            }
            if by_id.contains_key(&entry.ecc_id) {
                return Err(OwnRegisterError::Duplicate {
                    ecc_id: entry.ecc_id,
                });
            }
            by_id.insert(entry.ecc_id.clone(), entry);
        }
        Ok(Self { by_id })
    }

    /// The entry for an ECC id, if adjudicated.
    #[must_use]
    pub fn lookup(&self, ecc_id: &str) -> Option<&OwnAdjudication> {
        self.by_id.get(ecc_id)
    }

    /// Number of entries.
    #[must_use]
    pub fn len(&self) -> usize {
        self.by_id.len()
    }

    /// Whether the register is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.by_id.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_and_rejects_uncited() {
        let good = r#"
[meta]
description = "test"

[[entry]]
ecc_id = "ECC-QRY-009"
disposition = "corpus-dialect"
reason = "LIMIT before ORDER BY is invalid under the AQL 1.1 grammar."
citation = "QUERY AqlParser.g4 selectQuery"
"#;
        let reg = OwnRegister::parse(good).expect("parse");
        assert_eq!(reg.len(), 1);
        assert!(reg.lookup("ECC-QRY-009").is_some());

        let uncited = r#"
[[entry]]
ecc_id = "ECC-QRY-010"
disposition = "corpus-dialect"
reason = ""
citation = "x"
"#;
        assert!(matches!(
            OwnRegister::parse(uncited),
            Err(OwnRegisterError::Uncited { .. })
        ));
    }

    #[test]
    fn rejects_duplicates_and_missing_file() {
        let dup = r#"
[[entry]]
ecc_id = "ECC-A-001"
disposition = "corpus-dialect"
reason = "r"
citation = "c"

[[entry]]
ecc_id = "ECC-A-001"
disposition = "spec-supersedes-corpus"
reason = "r"
citation = "c"
"#;
        assert!(matches!(
            OwnRegister::parse(dup),
            Err(OwnRegisterError::Duplicate { .. })
        ));
        // A missing register is an error, never a silent empty: an empty
        // register flips corpus-dialect skips into green passes.
        assert!(matches!(
            OwnRegister::load(Path::new("/nonexistent/ecc-own.toml")),
            Err(OwnRegisterError::Io { .. })
        ));
    }
}
