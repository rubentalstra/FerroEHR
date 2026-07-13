//! The committed fixture manifest — every data set the conformance suites use
//! is named here, so ownership, generation, and provenance are deliberate
//! design rather than the accident of a path constant.
//!
//! Format (`testdata/MANIFEST.tsv`, one tab-separated row per fixture key):
//!
//! ```text
//! key <TAB> kind <TAB> source <TAB> adaptation <TAB> note
//! ```
//!
//! - **key** — the stable vocabulary the suites resolve through; unique.
//! - **kind** — one of the [`Kind`] tokens (`opt`, `composition`,
//!   `composition-xml`, `ehr-status`, `contribution`, `directory`,
//!   `aql-query`, `aql-golden`, `validation`, `flat`).
//! - **source** — one of four forms:
//!   - `owned:<rel>` — a reviewed committed file under `testdata/fixtures/`.
//!   - `corpus:<rel>` — a single vendored CNF corpus file (raw material).
//!   - `corpus-dir:<rel>|<ext>[|recursive]` — a vendored corpus directory
//!     sweep; `<ext>` filters by extension (empty matches any file); the
//!     optional `recursive` segment walks subdirectories.
//!   - `generated:<author-fn>` — programmatically authored by the named
//!     content-suite function; resolution returns a marker, never a file.
//! - **adaptation** — `none` or a named adaptation rule applied by the
//!   accessor (e.g. `adapt-ehr-status`, `flat-to-canonical`).
//! - **note** — required; cites the schedule section that references the data
//!   (or the defect/adjudication for owned/derived rows). Never empty.
//!
//! The register-80 ruling governs this file: the Robot corpus is raw material
//! only, named per fixture key; the runner resolves fixture keys through the
//! manifest ONLY (no free-path corpus seam).

use std::collections::BTreeMap;

/// Errors parsing the fixture manifest.
#[derive(Debug, thiserror::Error)]
pub enum ManifestError {
    /// A row did not have the five tab-separated columns.
    #[error("manifest line {line}: expected 5 tab-separated columns, found {found}")]
    Columns {
        /// 1-based source line number.
        line: usize,
        /// Number of columns actually present.
        found: usize,
    },
    /// Two rows shared a key.
    #[error("manifest line {line}: duplicate fixture key `{key}`")]
    DuplicateKey {
        /// 1-based source line number.
        line: usize,
        /// The repeated key.
        key: String,
    },
    /// The `kind` column was not a recognised [`Kind`].
    #[error("manifest line {line}: unknown kind `{kind}` for key `{key}`")]
    UnknownKind {
        /// 1-based source line number.
        line: usize,
        /// The offending kind token.
        kind: String,
        /// The row's key.
        key: String,
    },
    /// The `source` column was not a recognised source form.
    #[error("manifest line {line}: unknown source form `{form}` for key `{key}`")]
    UnknownSource {
        /// 1-based source line number.
        line: usize,
        /// The offending source token.
        form: String,
        /// The row's key.
        key: String,
    },
    /// The `note` column was empty (every row must carry a citation).
    #[error("manifest line {line}: empty note for key `{key}` (a citation is required)")]
    EmptyNote {
        /// 1-based source line number.
        line: usize,
        /// The row's key.
        key: String,
    },
}

/// The kind of data a fixture row names (the register-80 `kind` vocabulary).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// An operational template (`.opt`).
    Opt,
    /// A canonical-JSON COMPOSITION (or other versioned object).
    Composition,
    /// A canonical-XML COMPOSITION.
    CompositionXml,
    /// An `EHR_STATUS` data set.
    EhrStatus,
    /// A CONTRIBUTION data set.
    Contribution,
    /// A FOLDER (directory) data set.
    Directory,
    /// An AQL query fixture (carries a `q` field).
    AqlQuery,
    /// A golden `RESULT_SET`.
    AqlGolden,
    /// A content-validation data set.
    Validation,
    /// A FLAT (simSDT) instance, converted to canonical before commit.
    Flat,
}

impl Kind {
    fn parse(token: &str) -> Option<Self> {
        Some(match token {
            "opt" => Self::Opt,
            "composition" => Self::Composition,
            "composition-xml" => Self::CompositionXml,
            "ehr-status" => Self::EhrStatus,
            "contribution" => Self::Contribution,
            "directory" => Self::Directory,
            "aql-query" => Self::AqlQuery,
            "aql-golden" => Self::AqlGolden,
            "validation" => Self::Validation,
            "flat" => Self::Flat,
            _ => return None,
        })
    }
}

/// Where a fixture's bytes come from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Source {
    /// A reviewed committed file, `rel`-under `testdata/fixtures/`.
    Owned {
        /// Path relative to the owned-fixture root.
        rel: String,
    },
    /// A single vendored CNF corpus file, `rel`-under the corpus root.
    Corpus {
        /// Path relative to the corpus root.
        rel: String,
    },
    /// A vendored corpus directory sweep.
    CorpusDir {
        /// Directory relative to the corpus root.
        rel: String,
        /// Extension filter without the dot (empty matches any file).
        ext: String,
        /// Whether to walk subdirectories.
        recursive: bool,
    },
    /// A programmatically authored data set; `author_fn` names the
    /// content-suite function that produces it.
    Generated {
        /// The authoring function's documented name.
        author_fn: String,
    },
}

impl Source {
    fn parse(token: &str) -> Option<Self> {
        if let Some(rel) = token.strip_prefix("owned:") {
            return Some(Self::Owned {
                rel: rel.to_owned(),
            });
        }
        if let Some(author_fn) = token.strip_prefix("generated:") {
            return Some(Self::Generated {
                author_fn: author_fn.to_owned(),
            });
        }
        if let Some(spec) = token.strip_prefix("corpus-dir:") {
            let mut parts = spec.split('|');
            let rel = parts.next()?.to_owned();
            let ext = parts.next().unwrap_or("").to_owned();
            let recursive = match parts.next() {
                None | Some("") => false,
                Some("recursive") => true,
                Some(_) => return None,
            };
            if parts.next().is_some() || rel.is_empty() {
                return None;
            }
            return Some(Self::CorpusDir {
                rel,
                ext,
                recursive,
            });
        }
        if let Some(rel) = token.strip_prefix("corpus:") {
            if rel.is_empty() {
                return None;
            }
            return Some(Self::Corpus {
                rel: rel.to_owned(),
            });
        }
        None
    }
}

/// One manifest row.
#[derive(Debug, Clone)]
pub struct Entry {
    /// The fixture key.
    pub key: String,
    /// The kind of data.
    pub kind: Kind,
    /// Where the bytes come from.
    pub source: Source,
    /// The named adaptation rule, or `none`.
    pub adaptation: String,
    /// The citation / provenance note.
    pub note: String,
}

/// The parsed fixture manifest.
#[derive(Debug, Clone)]
pub struct Manifest {
    entries: Vec<Entry>,
    index: BTreeMap<String, usize>,
}

/// The manifest embedded at compile time (`testdata/MANIFEST.tsv`).
const MANIFEST_TSV: &str = include_str!("../../testdata/MANIFEST.tsv");

impl Manifest {
    /// Parse the committed manifest embedded in the binary.
    ///
    /// # Errors
    /// [`ManifestError`] on any malformed row (bad columns, unknown
    /// kind/source, duplicate key, empty note).
    pub fn load_default() -> Result<Self, ManifestError> {
        Self::parse(MANIFEST_TSV)
    }

    /// Parse a manifest from TSV text. Blank lines and `#`-comment lines are
    /// ignored.
    ///
    /// # Errors
    /// [`ManifestError`] on any malformed row.
    pub fn parse(text: &str) -> Result<Self, ManifestError> {
        let mut entries = Vec::new();
        let mut index = BTreeMap::new();
        for (i, raw) in text.lines().enumerate() {
            let line = i + 1;
            let trimmed = raw.trim_end_matches(['\r', '\n']);
            if trimmed.trim().is_empty() || trimmed.trim_start().starts_with('#') {
                continue;
            }
            let cols: Vec<&str> = trimmed.splitn(5, '\t').collect();
            if cols.len() != 5 {
                return Err(ManifestError::Columns {
                    line,
                    found: cols.len(),
                });
            }
            let key = cols[0].trim().to_owned();
            let kind = Kind::parse(cols[1].trim()).ok_or_else(|| ManifestError::UnknownKind {
                line,
                kind: cols[1].trim().to_owned(),
                key: key.clone(),
            })?;
            let source =
                Source::parse(cols[2].trim()).ok_or_else(|| ManifestError::UnknownSource {
                    line,
                    form: cols[2].trim().to_owned(),
                    key: key.clone(),
                })?;
            let adaptation = cols[3].trim().to_owned();
            let note = cols[4].trim().to_owned();
            if note.is_empty() {
                return Err(ManifestError::EmptyNote {
                    line,
                    key: key.clone(),
                });
            }
            if index.insert(key.clone(), entries.len()).is_some() {
                return Err(ManifestError::DuplicateKey { line, key });
            }
            entries.push(Entry {
                key,
                kind,
                source,
                adaptation,
                note,
            });
        }
        Ok(Self { entries, index })
    }

    /// The entry for `key`, if any.
    #[must_use]
    pub fn entry(&self, key: &str) -> Option<&Entry> {
        self.index.get(key).map(|&i| &self.entries[i])
    }

    /// All entries, in file order.
    #[must_use]
    pub fn entries(&self) -> &[Entry] {
        &self.entries
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_each_source_form() {
        let tsv = "\
# a comment line
k.owned\tcomposition\towned:valid/x.json\tnone\tregister note
k.corpus\tcomposition\tcorpus:a/b.json\tnone\tschedule note
k.dir\tehr-status\tcorpus-dir:ehr/valid|json\tadapt-ehr-status\tmaster06 note
k.dir.rec\topt\tcorpus-dir:valid_templates||recursive\tnone\tmaster07 note
k.gen\topt\tgenerated:author_content_opt\tnone\tmaster15 note
";
        let m = Manifest::parse(tsv).expect("parse");
        assert_eq!(m.entries().len(), 5);
        assert_eq!(
            m.entry("k.owned").unwrap().source,
            Source::Owned {
                rel: "valid/x.json".to_owned()
            }
        );
        assert_eq!(
            m.entry("k.corpus").unwrap().source,
            Source::Corpus {
                rel: "a/b.json".to_owned()
            }
        );
        assert_eq!(
            m.entry("k.dir").unwrap().source,
            Source::CorpusDir {
                rel: "ehr/valid".to_owned(),
                ext: "json".to_owned(),
                recursive: false,
            }
        );
        assert_eq!(
            m.entry("k.dir.rec").unwrap().source,
            Source::CorpusDir {
                rel: "valid_templates".to_owned(),
                ext: String::new(),
                recursive: true,
            }
        );
        assert_eq!(
            m.entry("k.gen").unwrap().source,
            Source::Generated {
                author_fn: "author_content_opt".to_owned()
            }
        );
    }

    #[test]
    fn rejects_duplicate_keys() {
        let tsv = "\
dup\tcomposition\tcorpus:a.json\tnone\tn1
dup\tcomposition\tcorpus:b.json\tnone\tn2
";
        assert!(matches!(
            Manifest::parse(tsv),
            Err(ManifestError::DuplicateKey { .. })
        ));
    }

    #[test]
    fn rejects_unknown_kind_and_source_and_empty_note() {
        assert!(matches!(
            Manifest::parse("k\tbogus\tcorpus:a.json\tnone\tn"),
            Err(ManifestError::UnknownKind { .. })
        ));
        assert!(matches!(
            Manifest::parse("k\tcomposition\tbogus:a.json\tnone\tn"),
            Err(ManifestError::UnknownSource { .. })
        ));
        assert!(matches!(
            Manifest::parse("k\tcomposition\tcorpus:a.json\tnone\t"),
            Err(ManifestError::EmptyNote { .. })
        ));
    }

    #[test]
    fn rejects_wrong_column_count() {
        assert!(matches!(
            Manifest::parse("k\tcomposition\tcorpus:a.json"),
            Err(ManifestError::Columns { found: 3, .. })
        ));
    }

    #[test]
    fn committed_manifest_loads() {
        // The embedded MANIFEST.tsv must always parse — a malformed row fails
        // the build here rather than at first fixture access.
        Manifest::load_default().expect("committed manifest parses");
    }
}
