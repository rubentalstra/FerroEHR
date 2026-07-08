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
//!
//! # v3: variant rows (design §3.2)
//!
//! The schedule's case headings carry **normative truth tables** — in the
//! content chapters every table row ending in an `accepted`/`rejected` verdict
//! cell is one executable data-set variant (1,300+ across master15–17.7). The
//! v3 extractor parses those rows into [`VariantRow`]s: each gets a
//! deterministic ordinal id (`r01`, `r02`, … in document order within its
//! case), the named `===== Data set …` sub-block where one applies (master17.3
//! style), and a **content fingerprint** (prefix of the SHA-256 of the
//! normalized row text) so a re-vendor that inserts or edits a row breaks the
//! coverage guard loudly instead of silently shifting ordinals.
//!
//! Chapter-level data-set tables (master06's 16-row valid `EHR_STATUS` matrix,
//! master08's `[[one_commit]]`/EHR_STATUS/`[[folder_commit]]` matrices,
//! master09's `$path/$result` sets) are extracted as [`DataSetTable`]s keyed by
//! their nearest anchor/heading, for the functional suites' data-set gates.

use std::collections::HashMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

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

/// The verdict of a normative truth-table row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Verdict {
    /// The data set must be accepted by the SUT.
    Accepted,
    /// The data set must be rejected by the SUT.
    Rejected,
}

/// One normative truth-table row inside a test case: a data-set variant
/// (design §3.2). The executable unit of the content chapters.
#[derive(Debug, Clone)]
pub struct VariantRow {
    /// The raw id of the owning case (heading id, not the classification key —
    /// pair with the occurrence ordinal via document order when disambiguating
    /// the one upstream duplicate).
    pub case_id: String,
    /// The deterministic ordinal id within the case: `"r01"`, `"r02"`, … in
    /// document order.
    pub id: String,
    /// The named `===== Data set …` sub-block this row belongs to, if any
    /// (master17.3 `DV_INTERVAL<DV_PROPORTION>` style).
    pub data_set: Option<String>,
    /// The row's normative verdict.
    pub verdict: Verdict,
    /// A stable content fingerprint: the first 12 hex chars of the SHA-256 of
    /// [`VariantRow::text`].
    pub fingerprint: String,
    /// The normalized row text (cells trimmed, joined with `" | "`).
    pub text: String,
    /// The source file name.
    pub file: String,
    /// The 1-based line number of the row.
    pub line: usize,
}

/// One AsciiDoc table extracted at chapter level (or anywhere), with enough
/// context to key the functional data-set matrices (design §3.2): master06's
/// valid `EHR_STATUS` matrix, master08's anchored commit matrices, master09's
/// path sets.
#[derive(Debug, Clone)]
pub struct DataSetTable {
    /// The source file name.
    pub file: String,
    /// The nearest preceding `[[anchor]]` if one directly precedes the table
    /// (e.g. `one_commit`, `folder_commit`).
    pub anchor: Option<String>,
    /// The nearest preceding heading text (without the `=` markers).
    pub heading: String,
    /// The 1-based line number of the opening `|===`.
    pub line: usize,
    /// The number of data rows (row lines after the header row).
    pub data_rows: usize,
    /// Fingerprints of the data rows, in order (same scheme as [`VariantRow`]).
    pub row_fingerprints: Vec<String>,
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
    /// Every normative truth-table row found inside a case span, in document
    /// order (the content-chapter data-set variants, design §3.2).
    pub variants: Vec<VariantRow>,
    /// Every table found across the schedule files, with anchor/heading
    /// context (the functional data-set matrices live here).
    pub tables: Vec<DataSetTable>,
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
    let mut variants = Vec::new();
    let mut tables = Vec::new();

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
        scan_file(
            &file_name,
            &text,
            &mut cases,
            &mut raw_heading_count,
            &mut variants,
            &mut tables,
        );
    }

    Ok(Schedule {
        cases,
        raw_heading_count,
        variants,
        tables,
    })
}

/// The per-file scan: case headings, case spans, tables, and truth-table rows.
///
/// A case's span runs from its heading to the next heading at the same or a
/// shallower level (`===== Data set …` sub-blocks are deeper, so they stay
/// inside). Verdict rows are single-line table rows whose cells contain an
/// exact `accepted`/`rejected` cell — verified against the vendored table
/// shapes (master15/16/17.x all use single-line `|a |b |accepted |…` rows).
fn scan_file(
    file_name: &str,
    text: &str,
    cases: &mut Vec<ScheduleCase>,
    raw_heading_count: &mut usize,
    variants: &mut Vec<VariantRow>,
    tables: &mut Vec<DataSetTable>,
) {
    /// Table-parse state for the table currently being scanned.
    struct OpenTable {
        line: usize,
        anchor: Option<String>,
        heading: String,
        seen_header_row: bool,
        data_rows: usize,
        row_fingerprints: Vec<String>,
    }

    // The case span currently open: (case id, heading level, next row ordinal).
    let mut open_case: Option<(String, usize, u32)> = None;
    let mut current_data_set: Option<String> = None;
    let mut current_heading = String::new();
    let mut pending_anchor: Option<String> = None;
    let mut open_table: Option<OpenTable> = None;

    for (idx, raw_line) in text.lines().enumerate() {
        let line_no = idx + 1;
        let trimmed = raw_line.trim_end();

        // Table delimiter (`|===`). Toggles table state; never a row.
        if trimmed.starts_with("|===") {
            match open_table.take() {
                Some(t) => tables.push(DataSetTable {
                    file: file_name.to_owned(),
                    anchor: t.anchor,
                    heading: t.heading,
                    line: t.line,
                    data_rows: t.data_rows,
                    row_fingerprints: t.row_fingerprints,
                }),
                None => {
                    open_table = Some(OpenTable {
                        line: line_no,
                        anchor: pending_anchor.take(),
                        heading: current_heading.clone(),
                        seen_header_row: false,
                        data_rows: 0,
                        row_fingerprints: Vec::new(),
                    });
                }
            }
            continue;
        }

        // A block anchor (`[[name]]`) — remembered for the next table.
        if let Some(anchor) = trimmed
            .strip_prefix("[[")
            .and_then(|rest| rest.strip_suffix("]]"))
        {
            pending_anchor = Some(anchor.to_owned());
            continue;
        }

        // Headings: raw count, case spans, data-set sub-blocks, table context.
        let eq_len = trimmed.bytes().take_while(|&b| b == b'=').count();
        if eq_len > 0 && trimmed[eq_len..].starts_with(' ') {
            let heading_text = trimmed[eq_len..].trim().to_owned();

            if let Some(id) = heading_id(trimmed) {
                *raw_heading_count += 1;
                // The `master03` documentation template heading carries angle
                // brackets and is not a test case — drop it from the inventory.
                if !(id.contains('<') || id.contains('>')) {
                    cases.push(ScheduleCase {
                        id: id.to_owned(),
                        file: file_name.to_owned(),
                        line: line_no,
                        placeholder: PLACEHOLDER_IDS.contains(&id),
                    });
                    open_case = Some((id.to_owned(), eq_len, 0));
                    current_data_set = None;
                    current_heading = heading_text;
                    pending_anchor = None;
                    continue;
                }
            }

            // A named data-set sub-block inside the open case (deeper level)?
            if let Some((_, case_level, _)) = &open_case {
                if eq_len > *case_level {
                    if let Some(name) = heading_text.strip_prefix("Data set") {
                        current_data_set = Some(name.trim().to_owned());
                    }
                } else {
                    // Same-or-shallower heading closes the case span.
                    open_case = None;
                    current_data_set = None;
                }
            }
            current_heading = heading_text;
            pending_anchor = None;
            continue;
        }

        // Table rows.
        if let Some(table) = &mut open_table {
            if trimmed.starts_with('|') {
                if table.seen_header_row {
                    table.data_rows += 1;
                    table
                        .row_fingerprints
                        .push(fingerprint(&normalize_row(trimmed)));
                } else {
                    table.seen_header_row = true;
                }

                // A verdict row inside an open case span is a variant.
                if let Some((case_id, _, ordinal)) = &mut open_case {
                    if let Some(verdict) = row_verdict(trimmed) {
                        *ordinal += 1;
                        let text = normalize_row(trimmed);
                        variants.push(VariantRow {
                            case_id: case_id.clone(),
                            id: format!("r{ordinal:02}"),
                            data_set: current_data_set.clone(),
                            verdict,
                            fingerprint: fingerprint(&text),
                            text,
                            file: file_name.to_owned(),
                            line: line_no,
                        });
                    }
                }
            }
        }
    }

    // An unterminated table at EOF would be an upstream syntax defect; record
    // what was seen rather than dropping it silently.
    if let Some(t) = open_table {
        tables.push(DataSetTable {
            file: file_name.to_owned(),
            anchor: t.anchor,
            heading: t.heading,
            line: t.line,
            data_rows: t.data_rows,
            row_fingerprints: t.row_fingerprints,
        });
    }
}

/// Normalize a table row: split on `|`, trim each cell, drop the empty lead
/// cell, collapse inner whitespace, join with `" | "`.
fn normalize_row(row: &str) -> String {
    let mut out = String::new();
    for cell in row.split('|').skip(1) {
        let mut first = true;
        if !out.is_empty() {
            out.push_str(" | ");
        }
        for word in cell.split_whitespace() {
            if !first {
                out.push(' ');
            }
            out.push_str(word);
            first = false;
        }
    }
    out
}

/// The first 12 hex chars of the SHA-256 of `text` — the row fingerprint.
fn fingerprint(text: &str) -> String {
    let digest = Sha256::digest(text.as_bytes());
    let mut hex = String::with_capacity(12);
    for byte in &digest[..6] {
        let _ = write!(hex, "{byte:02x}");
    }
    hex
}

/// If the row carries an exact `accepted`/`rejected` verdict cell, return it.
fn row_verdict(row: &str) -> Option<Verdict> {
    row.split('|').map(str::trim).find_map(|cell| match cell {
        "accepted" => Some(Verdict::Accepted),
        "rejected" => Some(Verdict::Rejected),
        _ => None,
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

    /// The variant-row totals, pinned per content chapter (design §3.2). These
    /// are the normative truth-table rows — the executable units of the
    /// content chapters — independently cross-checked against the 2026-07-08
    /// inventory (`grep -icE '\|\s*(accepted|rejected)'` per file).
    #[test]
    fn variant_rows_pinned_per_chapter() {
        let s = schedule();
        let count = |file: &str| s.variants.iter().filter(|v| v.file == file).count();
        assert_eq!(count("master15-content_tc_composition.adoc"), 108);
        assert_eq!(count("master16-content_tc_entry.adoc"), 138);
        assert_eq!(count("master17.1-content_tc_data_types-basic.adoc"), 30);
        assert_eq!(count("master17.2-content_tc_data_types-text.adoc"), 24);
        assert_eq!(count("master17.3-content_tc_data_types-quantity.adoc"), 406);
        assert_eq!(
            count("master17.4-content_tc_data_types-date_time.adoc"),
            604
        );
        assert_eq!(
            count("master17.5-content_tc_data_types-time_specification.adoc"),
            0,
            "17.5 is empty upstream (TBD)"
        );
        assert_eq!(
            count("master17.6-content_tc_data_types-encapsulated.adoc"),
            23
        );
        assert_eq!(count("master17.7-content_tc_data_types-uri.adoc"), 38);
        assert_eq!(
            s.variants.len(),
            1371,
            "content chapters only — the functional matrices live in `tables`, not in case spans"
        );
    }

    /// Variant ids are unique within their case and ordinals are contiguous
    /// from r01 in document order.
    #[test]
    fn variant_ids_contiguous_per_case() {
        let s = schedule();
        let mut last: HashMap<(String, String), u32> = HashMap::new();
        for v in &s.variants {
            let n: u32 =
                v.id.strip_prefix('r')
                    .expect("r-prefix")
                    .parse()
                    .expect("ordinal");
            let key = (v.file.clone(), v.case_id.clone());
            let prev = last.insert(key, n);
            // `r01` legitimately restarts a sequence: the one upstream
            // duplicate heading (CONT-DV_TEXT-validate_open, master17.2)
            // opens a second occurrence of the same case id.
            if n != 1 {
                assert_eq!(
                    n,
                    prev.unwrap_or(0) + 1,
                    "{}/{} ordinal gap",
                    v.file,
                    v.case_id
                );
            }
        }
    }

    /// The named `===== Data set …` sub-blocks (master17.3 interval-proportion
    /// style) attach to their rows.
    #[test]
    fn named_data_set_blocks_attach() {
        let s = schedule();
        let named: Vec<&VariantRow> = s.variants.iter().filter(|v| v.data_set.is_some()).collect();
        assert!(
            !named.is_empty(),
            "master17.3 DV_INTERVAL<DV_PROPORTION> data-set sub-blocks expected"
        );
        assert!(named.iter().all(|v| v.file.contains("master17.3")));
    }

    /// The functional data-set matrices are extracted as chapter-level tables:
    /// master06's 16-row valid EHR_STATUS matrix and master08's anchored
    /// commit matrices are the load-bearing ones (design §3.2).
    #[test]
    fn functional_matrices_extracted_as_tables() {
        let s = schedule();
        let m06 = s
            .tables
            .iter()
            .find(|t| t.file == "master06-func_tc_ehr.adoc" && t.data_rows == 16)
            .expect("master06 16-row valid EHR_STATUS matrix");
        assert_eq!(m06.row_fingerprints.len(), 16);

        let anchored: Vec<&DataSetTable> = s.tables.iter().filter(|t| t.anchor.is_some()).collect();
        assert!(
            anchored
                .iter()
                .any(|t| t.anchor.as_deref() == Some("one_commit")),
            "master08 [[one_commit]] matrix must be anchored; found anchors: {:?}",
            anchored
                .iter()
                .map(|t| t.anchor.as_deref())
                .collect::<Vec<_>>()
        );
    }
}
