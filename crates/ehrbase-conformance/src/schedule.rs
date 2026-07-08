//! The `.adoc` schedule inventory parser (design §4.2).
//!
//! Parses every `.adoc` file in the configured Platform Conformance Test
//! Schedule directory for the test-case headings (`^=+ Test Case <id>$`) and
//! returns the ordered inventory. The parse is deliberately keyed on the heading
//! regex over a **configured directory**, never on hard-coded `masterNN` paths,
//! so a future re-vendor (the upstream `development` branch moves the files to
//! `modules/<book>/pages/*.adoc`) is a path change, not a parser rewrite
//! (design §2, §4.2).
//!
//! Three upstream realities are handled deterministically:
//!
//! - **Documentation template** — `master03-overview.adoc` carries one heading
//!   whose id is the literal form `<SERVICE_COMPONENT>.<operation>-…`. It is a
//!   documentation example, not a test case; ids containing `<`/`>` are dropped
//!   from the inventory (still counted in [`Schedule::raw_heading_count`]).
//! - **Placeholders** — 57 headings are the literal upstream stubs `aaaa`/`bbbb`.
//!   They get a synthesized, stable classification key
//!   `PLACEHOLDER-<file-stem>-<ordinal>` and are marked [`ScheduleCase::placeholder`].
//! - **One real duplicate** — `CONT-DV_TEXT-validate_open` appears twice
//!   (master17.2). The first occurrence keeps its id; the second is keyed
//!   `<id>#2` and flagged as a duplicate.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::case::Chapter;

/// The default schedule directory, resolved relative to this crate.
///
/// Keep this a single `concat!` so it is a compile-time constant; override it in
/// tests or a re-vendored layout by passing an explicit path to
/// [`parse_schedule`].
pub const SCHEDULE_DIR: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../docs/specs/openehr/CNF/docs/platform_test_schedule"
);

/// The literal upstream placeholder ids (design §2, verified 2026-07-07).
const PLACEHOLDER_IDS: [&str; 2] = ["aaaa", "bbbb"];

/// Errors raised while parsing the schedule.
#[derive(Debug, thiserror::Error)]
pub enum ScheduleError {
    /// The schedule directory could not be read.
    #[error("reading schedule directory {path}: {source}")]
    Io {
        /// The path being read.
        path: PathBuf,
        /// The underlying I/O error.
        source: std::io::Error,
    },
    /// A non-template test-case heading was found in a file that maps to no
    /// [`Chapter`] — a re-vendor moved the layout and the [`Chapter`] mapping
    /// needs updating (design §4.2 coverage-guard intent).
    #[error("test case {id:?} in {file} (line {line}) maps to no known chapter")]
    UnknownChapter {
        /// The offending case id.
        id: String,
        /// The file it was found in.
        file: String,
        /// The 1-based line number.
        line: usize,
    },
}

/// One heading extracted from the schedule, in document order.
#[derive(Debug, Clone)]
pub struct ScheduleCase {
    /// The raw case id from the heading (e.g. `"I_EHR_SERVICE.create_ehr-main"`,
    /// or the literal `"aaaa"`/`"bbbb"` for a placeholder).
    pub id: String,
    /// The source file name (with extension).
    pub file: String,
    /// The 1-based line number of the heading.
    pub line: usize,
    /// Whether this heading is a literal upstream placeholder (`aaaa`/`bbbb`).
    pub placeholder: bool,
}

/// One fully-classified inventory entry: a [`ScheduleCase`] plus its stable
/// classification `key` and derived facts the registry classifies on.
#[derive(Debug, Clone)]
pub struct InventoryItem {
    /// The stable classification key (unique across the inventory): the raw id
    /// for a first, real occurrence; `PLACEHOLDER-<file-stem>-<n>` for a
    /// placeholder; `<id>#<n>` for a duplicate occurrence.
    pub key: String,
    /// The raw case id.
    pub id: String,
    /// The chapter the case belongs to.
    pub chapter: Chapter,
    /// Whether this is a literal upstream placeholder.
    pub placeholder: bool,
    /// Whether this is a second-or-later occurrence of a real id (a duplicate).
    pub duplicate: bool,
    /// The source file name.
    pub file: String,
    /// The 1-based line number.
    pub line: usize,
}

/// The parsed schedule: the ordered case inventory plus the raw heading count.
#[derive(Debug, Clone)]
pub struct Schedule {
    /// The ordered, template-filtered case inventory.
    pub cases: Vec<ScheduleCase>,
    /// The total number of `^=+ Test Case …$` heading lines matched across all
    /// `.adoc` files, **including** the documentation template that
    /// [`Schedule::cases`] drops (design asserts this equals 325).
    pub raw_heading_count: usize,
}

impl Schedule {
    /// The number of placeholder cases (`aaaa`/`bbbb`).
    #[must_use]
    pub fn placeholder_count(&self) -> usize {
        self.cases.iter().filter(|c| c.placeholder).count()
    }

    /// The number of real (non-placeholder) case occurrences.
    #[must_use]
    pub fn real_count(&self) -> usize {
        self.cases.iter().filter(|c| !c.placeholder).count()
    }

    /// The fully-classified inventory: each case with its stable key and derived
    /// duplicate/placeholder facts, in document order.
    ///
    /// # Errors
    /// [`ScheduleError::UnknownChapter`] if a non-template heading lives in a
    /// file that maps to no [`Chapter`].
    pub fn inventory(&self) -> Result<Vec<InventoryItem>, ScheduleError> {
        let mut placeholder_ordinal: HashMap<String, u32> = HashMap::new();
        let mut real_seen: HashMap<String, u32> = HashMap::new();
        let mut items = Vec::with_capacity(self.cases.len());

        for case in &self.cases {
            let chapter = Chapter::from_source_file(&case.file).ok_or_else(|| {
                ScheduleError::UnknownChapter {
                    id: case.id.clone(),
                    file: case.file.clone(),
                    line: case.line,
                }
            })?;

            let (key, duplicate) = if case.placeholder {
                let n = placeholder_ordinal
                    .entry(case.file.clone())
                    .and_modify(|n| *n += 1)
                    .or_insert(1);
                let stem = case.file.strip_suffix(".adoc").unwrap_or(&case.file);
                (format!("PLACEHOLDER-{stem}-{n}"), false)
            } else {
                let n = real_seen
                    .entry(case.id.clone())
                    .and_modify(|n| *n += 1)
                    .or_insert(1);
                if *n == 1 {
                    (case.id.clone(), false)
                } else {
                    (format!("{}#{n}", case.id), true)
                }
            };

            items.push(InventoryItem {
                key,
                id: case.id.clone(),
                chapter,
                placeholder: case.placeholder,
                duplicate,
                file: case.file.clone(),
                line: case.line,
            });
        }
        Ok(items)
    }
}

/// Parse the schedule from the default vendored directory ([`SCHEDULE_DIR`]).
///
/// # Errors
/// [`ScheduleError::Io`] if the directory or a file cannot be read.
pub fn parse_default() -> Result<Schedule, ScheduleError> {
    parse_schedule(Path::new(SCHEDULE_DIR))
}

/// Parse every `.adoc` file in `dir` for test-case headings.
///
/// # Errors
/// [`ScheduleError::Io`] if the directory or a file cannot be read.
pub fn parse_schedule(dir: &Path) -> Result<Schedule, ScheduleError> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(dir)
        .map_err(|source| ScheduleError::Io {
            path: dir.to_path_buf(),
            source,
        })?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|ext| ext == "adoc"))
        .collect();
    files.sort();

    let mut cases = Vec::new();
    let mut raw_heading_count = 0;

    for path in &files {
        let file_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default()
            .to_owned();
        let text = std::fs::read_to_string(path).map_err(|source| ScheduleError::Io {
            path: path.clone(),
            source,
        })?;
        for (idx, raw_line) in text.lines().enumerate() {
            let Some(id) = heading_id(raw_line) else {
                continue;
            };
            raw_heading_count += 1;
            // The `master03` documentation template heading carries angle
            // brackets and is not a test case — drop it from the inventory.
            if id.contains('<') || id.contains('>') {
                continue;
            }
            cases.push(ScheduleCase {
                id: id.to_owned(),
                file: file_name.clone(),
                line: idx + 1,
                placeholder: PLACEHOLDER_IDS.contains(&id),
            });
        }
    }

    Ok(Schedule {
        cases,
        raw_heading_count,
    })
}

/// If `line` is a `=+ Test Case <id>` heading, return the trimmed `<id>`.
///
/// The whitespace between the `=` run and `Test Case` is `trim_start`-ed, not
/// matched as a single space: the vendored master04 contains one heading with
/// a double space (`====  Test Case I_DEFINITION_ADL14.get_opts-retrieve_all`)
/// that a strict single-space match silently drops from the identified
/// inventory.
fn heading_id(line: &str) -> Option<&str> {
    let trimmed = line.trim_end();
    let eq_len = trimmed.bytes().take_while(|&b| b == b'=').count();
    if eq_len == 0 {
        return None;
    }
    let rest = trimmed[eq_len..].trim_start();
    let id = rest.strip_prefix("Test Case ")?.trim();
    (!id.is_empty()).then_some(id)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn schedule() -> Schedule {
        parse_default().expect("parse vendored schedule")
    }

    #[test]
    fn raw_heading_count_is_323() {
        // The verified total across all chapters: includes the master03
        // documentation template AND the two double-spaced headings the original
        // 2026-07-07 count missed (master04 `get_opts-retrieve_all`, master07
        // `get_composition_at_time`).
        assert_eq!(schedule().raw_heading_count, 325);
    }

    #[test]
    fn inventory_is_324_real_cases() {
        // 325 raw − 1 documentation template = 324 identified cases (two
        // upstream headings carry a double space after `====`).
        let s = schedule();
        assert_eq!(s.cases.len(), 324);
        // No documentation-template ids leak into the inventory.
        assert!(!s.cases.iter().any(|c| c.id.contains('<')));
    }

    #[test]
    fn placeholder_and_real_split() {
        let s = schedule();
        assert_eq!(s.placeholder_count(), 57, "aaaa (28) + bbbb (29)");
        assert_eq!(s.real_count(), 267, "267 real occurrences (incl. 1 dup)");
    }

    #[test]
    fn exactly_one_duplicate_keyed_hash_2() {
        let items = schedule().inventory().expect("inventory");
        let dups: Vec<&InventoryItem> = items.iter().filter(|i| i.duplicate).collect();
        assert_eq!(dups.len(), 1, "exactly one upstream duplicate");
        assert_eq!(dups[0].id, "CONT-DV_TEXT-validate_open");
        assert_eq!(dups[0].key, "CONT-DV_TEXT-validate_open#2");
        assert_eq!(dups[0].chapter, Chapter::Master17_2);
    }

    #[test]
    fn distinct_real_ids_is_266() {
        let items = schedule().inventory().expect("inventory");
        let distinct: std::collections::BTreeSet<&str> = items
            .iter()
            .filter(|i| !i.placeholder)
            .map(|i| i.id.as_str())
            .collect();
        assert_eq!(distinct.len(), 266, "267 occurrences − 1 duplicate");
    }

    #[test]
    fn inventory_keys_are_unique() {
        let items = schedule().inventory().expect("inventory");
        let keys: std::collections::BTreeSet<&str> = items.iter().map(|i| i.key.as_str()).collect();
        assert_eq!(
            keys.len(),
            items.len(),
            "every classification key is unique"
        );
        assert_eq!(items.len(), 324);
    }
}
