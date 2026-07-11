//! The upstream fairness adjudication register (X1, §3a.4): committed **data**,
//! not code, that reclassifies conformance outcomes for a *non-`ehrbase-rs`*
//! SUT — never edits a case.
//!
//! The ECC catalogue is our own instrument, authored against the pinned specs
//! (RM 1.2.0, ITS-REST development@e8a093e) with adjudicated skips for *our*
//! server. Running it unmodified against upstream `EHRbase` would unfairly fail
//! it on version skew and on our own extensions. Before any upstream result is
//! published, every upstream failure is triaged into a committed, cited
//! register (`tools/conformance/adjudications/<sut>.toml`) and applied at the
//! executor seam ([`crate::run`]). Three dispositions:
//!
//! - `extension` → the case exercises an `ehrbase-rs` extension the SUT does
//!   not implement (e.g. the demographic REST API, version signing) → reported
//!   [`NotApplicable`](crate::results::CaseStatus::NotApplicable).
//! - `rm-version-sensitive` → the case's request payload or response comparison
//!   depends on RM 1.2.0 shapes the SUT's older RM/ITS surface cannot produce →
//!   [`NotApplicable`](crate::results::CaseStatus::NotApplicable), with the
//!   RM/ITS-version citation.
//! - `defect` → a genuine spec gap that survives triage → the case runs
//!   normally and its natural outcome (a **failure**) stands, documented here
//!   with the spec citation.
//!
//! **Hard rule (honesty rule 10 / standing rule 3):** the register only
//! *reclassifies with a citation*; it never weakens, edits, or skips a case.
//! Every entry carries a non-empty `reason` and `citation` — [`parse`] rejects
//! a register that omits either. Running with no register is today's behaviour,
//! byte-for-byte (the zero-drift gate on our own baseline).
//!
//! ## File format (TOML)
//!
//! ```toml
//! [meta]
//! sut = "ehrbase-java"        # informational; the product name lives in results.json
//! version = "2.34.0"
//! description = "Upstream fairness adjudication register for EHRbase (Java) 2.34.0."
//!
//! # An area-wide rule: every case in the area gets this disposition.
//! [[area]]
//! area = "DEM"                # the ECC area tag (see `catalog::Area::tag`)
//! disposition = "extension"
//! reason = "Upstream EHRbase has no demographic REST API."
//! citation = "docs/plans/x1-comparison.md §2c"
//!
//! # A per-case rule (keyed by ECC id) — wins over an area-wide rule.
//! [[case]]
//! ecc_id = "ECC-QRY-014"
//! disposition = "rm-version-sensitive"
//! reason = "Response compares RM 1.2.0 shapes; upstream archie 3.13.0 emits RM 1.1.0-era wire."
//! citation = "docs/VERSIONS.md §RM-version divergence"
//! ```

use std::collections::HashMap;
use std::path::Path;

use serde::Deserialize;

/// How a case is reclassified for a non-`ehrbase-rs` SUT.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Disposition {
    /// An `ehrbase-rs` extension the SUT does not implement → `NotApplicable`.
    Extension,
    /// A comparison sensitive to the RM/ITS version the SUT predates →
    /// `NotApplicable` (with the version citation).
    RmVersionSensitive,
    /// A genuine spec gap that survives triage → the case runs and its failure
    /// stands (reclassifies nothing at runtime; documents the finding).
    Defect,
}

impl Disposition {
    /// Whether this disposition reclassifies the case to
    /// [`NotApplicable`](crate::results::CaseStatus::NotApplicable) (so the
    /// executor short-circuits and never runs it). `Defect` returns `false`:
    /// the case runs and its natural outcome stands.
    #[must_use]
    pub const fn is_not_applicable(self) -> bool {
        matches!(
            self,
            Disposition::Extension | Disposition::RmVersionSensitive
        )
    }
}

/// One adjudication: a disposition plus its mandatory reason and citation.
#[derive(Debug, Clone)]
pub struct Adjudication {
    /// The disposition applied to the case(s).
    pub disposition: Disposition,
    /// The human-readable reason (why this reclassification is fair).
    pub reason: String,
    /// The spec / research citation backing the reason (never empty).
    pub citation: String,
}

/// Register provenance (informational; the authoritative product identity is in
/// `results.json`).
#[derive(Debug, Clone, Default)]
pub struct RegisterMeta {
    /// The SUT the register was authored for (e.g. `"ehrbase-java"`).
    pub sut: String,
    /// The SUT version (e.g. `"2.34.0"`).
    pub version: String,
    /// A one-line description.
    pub description: String,
}

/// A loaded, validated adjudication register.
#[derive(Debug, Clone, Default)]
pub struct AdjudicationRegister {
    meta: RegisterMeta,
    by_ecc_id: HashMap<String, Adjudication>,
    by_area: HashMap<String, Adjudication>,
}

/// Errors raised loading/validating a register.
#[derive(Debug, thiserror::Error)]
pub enum AdjudicationError {
    /// The register file could not be read.
    #[error("adjudication register I/O at {path}: {source}")]
    Io {
        /// The offending path.
        path: String,
        /// The underlying error.
        source: std::io::Error,
    },
    /// The TOML could not be parsed.
    #[error("adjudication register TOML parse: {0}")]
    Parse(String),
    /// The register parsed but is not honest (a missing citation/reason, or a
    /// duplicate/empty key).
    #[error("adjudication register invalid: {0}")]
    Invalid(String),
}

// ── The on-disk shape (serde) ────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct RegisterFile {
    #[serde(default)]
    meta: MetaFile,
    #[serde(default)]
    area: Vec<AreaRule>,
    #[serde(default)]
    case: Vec<CaseRule>,
}

#[derive(Debug, Default, Deserialize)]
struct MetaFile {
    #[serde(default)]
    sut: String,
    #[serde(default)]
    version: String,
    #[serde(default)]
    description: String,
}

#[derive(Debug, Deserialize)]
struct AreaRule {
    area: String,
    disposition: Disposition,
    reason: String,
    citation: String,
}

#[derive(Debug, Deserialize)]
struct CaseRule {
    ecc_id: String,
    disposition: Disposition,
    reason: String,
    citation: String,
}

impl AdjudicationRegister {
    /// Load and validate a register from `path`.
    ///
    /// # Errors
    /// [`AdjudicationError`] on I/O, parse, or validation failure.
    pub fn load(path: &Path) -> Result<Self, AdjudicationError> {
        let text = std::fs::read_to_string(path).map_err(|source| AdjudicationError::Io {
            path: path.display().to_string(),
            source,
        })?;
        Self::parse(&text)
    }

    /// Parse and validate a register from TOML text.
    ///
    /// # Errors
    /// [`AdjudicationError::Parse`] on malformed TOML or an unknown disposition;
    /// [`AdjudicationError::Invalid`] on an empty/duplicate key or a missing
    /// reason/citation (honesty rule: every entry is cited).
    pub fn parse(text: &str) -> Result<Self, AdjudicationError> {
        let file: RegisterFile =
            toml::from_str(text).map_err(|e| AdjudicationError::Parse(e.to_string()))?;

        let mut by_area: HashMap<String, Adjudication> = HashMap::new();
        for rule in file.area {
            let key = rule.area.trim().to_owned();
            require(&key, "area", &rule.reason, &rule.citation)?;
            if by_area.contains_key(&key) {
                return Err(AdjudicationError::Invalid(format!(
                    "duplicate area rule for {key:?}"
                )));
            }
            by_area.insert(
                key,
                Adjudication {
                    disposition: rule.disposition,
                    reason: rule.reason,
                    citation: rule.citation,
                },
            );
        }

        let mut by_ecc_id: HashMap<String, Adjudication> = HashMap::new();
        for rule in file.case {
            let key = rule.ecc_id.trim().to_owned();
            require(&key, "ecc_id", &rule.reason, &rule.citation)?;
            if by_ecc_id.contains_key(&key) {
                return Err(AdjudicationError::Invalid(format!(
                    "duplicate case rule for {key:?}"
                )));
            }
            by_ecc_id.insert(
                key,
                Adjudication {
                    disposition: rule.disposition,
                    reason: rule.reason,
                    citation: rule.citation,
                },
            );
        }

        Ok(Self {
            meta: RegisterMeta {
                sut: file.meta.sut,
                version: file.meta.version,
                description: file.meta.description,
            },
            by_ecc_id,
            by_area,
        })
    }

    /// The register provenance.
    #[must_use]
    pub fn meta(&self) -> &RegisterMeta {
        &self.meta
    }

    /// The number of registered rules (per-case + area-wide).
    #[must_use]
    pub fn len(&self) -> usize {
        self.by_ecc_id.len() + self.by_area.len()
    }

    /// Whether the register has no rules.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.by_ecc_id.is_empty() && self.by_area.is_empty()
    }

    /// The adjudication for a case, if any: an exact `ecc_id` rule wins over an
    /// area-wide rule (`area_tag` is [`crate::catalog::Area::tag`]).
    #[must_use]
    pub fn lookup(&self, ecc_id: &str, area_tag: &str) -> Option<&Adjudication> {
        self.by_ecc_id
            .get(ecc_id)
            .or_else(|| self.by_area.get(area_tag))
    }
}

/// Validate one rule's key/reason/citation are all present (honesty rule).
fn require(
    key: &str,
    key_field: &str,
    reason: &str,
    citation: &str,
) -> Result<(), AdjudicationError> {
    if key.is_empty() {
        return Err(AdjudicationError::Invalid(format!("empty {key_field}")));
    }
    if reason.trim().is_empty() {
        return Err(AdjudicationError::Invalid(format!(
            "{key} has no reason (every adjudication must be justified)"
        )));
    }
    if citation.trim().is_empty() {
        return Err(AdjudicationError::Invalid(format!(
            "{key} has no citation (every adjudication must be cited)"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"
[meta]
sut = "ehrbase-java"
version = "2.34.0"
description = "test register"

[[area]]
area = "Dem"
disposition = "extension"
reason = "Upstream EHRbase has no demographic REST API."
citation = "docs/plans/x1-comparison.md §2c"

[[case]]
ecc_id = "ECC-QRY-014"
disposition = "rm-version-sensitive"
reason = "Response compares RM 1.2.0 shapes."
citation = "docs/VERSIONS.md §RM-version divergence"

[[case]]
ecc_id = "ECC-SQR-002"
disposition = "defect"
reason = "Upstream rejects ALL_VERSIONS."
citation = "ADR-008 §2"
"#;

    #[test]
    fn parses_meta_and_rules() {
        let reg = AdjudicationRegister::parse(SAMPLE).expect("parse");
        assert_eq!(reg.meta().sut, "ehrbase-java");
        assert_eq!(reg.meta().version, "2.34.0");
        assert_eq!(reg.len(), 3);
        assert!(!reg.is_empty());
    }

    #[test]
    fn per_case_rule_wins_over_area_rule() {
        let text = r#"
[[area]]
area = "Qry"
disposition = "extension"
reason = "area rule"
citation = "cite"

[[case]]
ecc_id = "ECC-QRY-001"
disposition = "defect"
reason = "case rule"
citation = "cite"
"#;
        let reg = AdjudicationRegister::parse(text).expect("parse");
        // The exact ecc_id rule wins.
        assert_eq!(
            reg.lookup("ECC-QRY-001", "Qry").expect("hit").disposition,
            Disposition::Defect
        );
        // A different case in the same area falls back to the area rule.
        assert_eq!(
            reg.lookup("ECC-QRY-099", "Qry").expect("hit").disposition,
            Disposition::Extension
        );
        // A case in another area with no rule misses.
        assert!(reg.lookup("ECC-EHR-001", "Ehr").is_none());
    }

    #[test]
    fn dispositions_map_to_not_applicable_correctly() {
        assert!(Disposition::Extension.is_not_applicable());
        assert!(Disposition::RmVersionSensitive.is_not_applicable());
        assert!(!Disposition::Defect.is_not_applicable());
    }

    #[test]
    fn rejects_missing_citation() {
        let text = r#"
[[case]]
ecc_id = "ECC-DEM-001"
disposition = "extension"
reason = "no demographic API"
citation = ""
"#;
        let err = AdjudicationRegister::parse(text).expect_err("must reject");
        assert!(matches!(err, AdjudicationError::Invalid(_)), "{err:?}");
        assert!(err.to_string().contains("citation"));
    }

    #[test]
    fn rejects_missing_reason() {
        let text = r#"
[[area]]
area = "Sig"
disposition = "extension"
reason = "   "
citation = "cite"
"#;
        let err = AdjudicationRegister::parse(text).expect_err("must reject");
        assert!(matches!(err, AdjudicationError::Invalid(_)), "{err:?}");
        assert!(err.to_string().contains("reason"));
    }

    #[test]
    fn rejects_unknown_disposition() {
        let text = r#"
[[case]]
ecc_id = "ECC-EHR-001"
disposition = "wontfix"
reason = "r"
citation = "c"
"#;
        let err = AdjudicationRegister::parse(text).expect_err("must reject");
        assert!(matches!(err, AdjudicationError::Parse(_)), "{err:?}");
    }

    #[test]
    fn rejects_duplicate_case_key() {
        let text = r#"
[[case]]
ecc_id = "ECC-EHR-001"
disposition = "defect"
reason = "r"
citation = "c"

[[case]]
ecc_id = "ECC-EHR-001"
disposition = "extension"
reason = "r"
citation = "c"
"#;
        let err = AdjudicationRegister::parse(text).expect_err("must reject");
        assert!(matches!(err, AdjudicationError::Invalid(_)), "{err:?}");
        assert!(err.to_string().contains("duplicate"));
    }

    #[test]
    fn empty_register_is_valid_and_empty() {
        let reg = AdjudicationRegister::parse("").expect("parse empty");
        assert!(reg.is_empty());
        assert_eq!(reg.len(), 0);
        assert!(reg.lookup("ECC-EHR-001", "Ehr").is_none());
    }
}
