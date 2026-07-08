//! Report generation (design §4.5): from a [`RunResults`] write the committed,
//! public artifact set — `results.json` (machine-readable), `RESULTS.md` (the
//! per-chapter matrix), `CONFORMANCE_STATEMENT.md` (the certificate-template
//! structure + deviations register), and `badge.json` (shields endpoint schema).
//!
//! At the honest zero state these all generate and show `0` implemented / `N`
//! not-yet — the backlog is enforced and visible, which is the point.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::Path;

use crate::case::Chapter;
use crate::results::{CaseStatus, RunResults};

/// Errors raised while writing the report set.
#[derive(Debug, thiserror::Error)]
pub enum ReportError {
    /// A file could not be read/written.
    #[error("report I/O at {path}: {source}")]
    Io {
        /// The offending path.
        path: String,
        /// The underlying error.
        source: std::io::Error,
    },
    /// `results.json` could not be (de)serialized.
    #[error("results.json codec: {0}")]
    Codec(String),
}

/// Read a `results.json` back into a [`RunResults`] (the `report --from` path).
///
/// # Errors
/// [`ReportError`] on I/O or deserialization failure.
pub fn from_results_file(path: &Path) -> Result<RunResults, ReportError> {
    let text = std::fs::read_to_string(path).map_err(|source| ReportError::Io {
        path: path.display().to_string(),
        source,
    })?;
    serde_json::from_str(&text).map_err(|e| ReportError::Codec(e.to_string()))
}

/// Write the full report set into `out_dir`.
///
/// # Errors
/// [`ReportError`] on I/O or serialization failure.
pub fn write_all(results: &RunResults, out_dir: &Path) -> Result<(), ReportError> {
    std::fs::create_dir_all(out_dir).map_err(|source| ReportError::Io {
        path: out_dir.display().to_string(),
        source,
    })?;
    let json =
        serde_json::to_string_pretty(results).map_err(|e| ReportError::Codec(e.to_string()))?;
    write_file(&out_dir.join("results.json"), &json)?;
    write_file(&out_dir.join("RESULTS.md"), &render_results_md(results))?;
    write_file(
        &out_dir.join("CONFORMANCE_STATEMENT.md"),
        &render_statement_md(results),
    )?;
    write_file(&out_dir.join("badge.json"), &render_badge(results))?;
    Ok(())
}

fn write_file(path: &Path, content: &str) -> Result<(), ReportError> {
    std::fs::write(path, content).map_err(|source| ReportError::Io {
        path: path.display().to_string(),
        source,
    })
}

/// Aggregate counts for one chapter.
#[derive(Default, Clone, Copy)]
struct ChapterTally {
    implemented: usize,
    passed: usize,
    failed: usize,
    excluded: usize,
    not_yet: usize,
}

impl ChapterTally {
    fn total(self) -> usize {
        self.implemented + self.excluded
    }
}

fn tally_by_chapter(results: &RunResults) -> BTreeMap<&str, ChapterTally> {
    let mut by: BTreeMap<&str, ChapterTally> = BTreeMap::new();
    for item in &results.inventory {
        let t = by.entry(item.chapter.as_str()).or_default();
        if item.is_implemented() {
            t.implemented += 1;
        } else {
            t.excluded += 1;
            if item.kind == "not_yet_transcribed" {
                t.not_yet += 1;
            }
        }
    }
    for case in &results.cases {
        let t = by.entry(case.chapter.as_str()).or_default();
        match case.status {
            CaseStatus::Passed => t.passed += 1,
            CaseStatus::Failed => t.failed += 1,
            CaseStatus::Errored | CaseStatus::Skipped => {}
        }
    }
    by
}

fn implemented_count(results: &RunResults) -> usize {
    results
        .inventory
        .iter()
        .filter(|i| i.is_implemented())
        .count()
}

fn render_results_md(results: &RunResults) -> String {
    let by = tally_by_chapter(results);
    let mut out = String::new();
    out.push_str("# openEHR CNF — Test Execution Report\n\n");
    let _ = write!(
        out,
        "- SUT: `{}`\n- RM version: {}\n- Auth mode: {}\n- Corpus: `{}` @ `{}`\n- Started: {}\n\n",
        results.sut.base_url,
        results.sut.rm_version,
        results.sut.auth_mode,
        results.corpus.repo,
        results.corpus.commit,
        results.started
    );
    let _ = write!(
        out,
        "**{} identified cases · {} implemented · {} passed · {} failed.**\n\n",
        results.identified(),
        implemented_count(results),
        results.passed(),
        results.failed(),
    );

    out.push_str("## Per-chapter matrix\n\n");
    out.push_str("| Chapter | Implemented | Passed | Failed | Excluded | Not-yet | Total |\n");
    out.push_str("|---|--:|--:|--:|--:|--:|--:|\n");
    for chapter in Chapter::ALL {
        let label = chapter.label();
        let t = by.get(label).copied().unwrap_or_default();
        let _ = writeln!(
            out,
            "| {label} | {} | {} | {} | {} | {} | {} |",
            t.implemented,
            t.passed,
            t.failed,
            t.excluded,
            t.not_yet,
            t.total()
        );
    }

    // Failures section — each links to the finding workflow (§4.5).
    let failures: Vec<_> = results
        .cases
        .iter()
        .filter(|c| c.status == CaseStatus::Failed)
        .collect();
    out.push_str("\n## Failures\n\n");
    if failures.is_empty() {
        out.push_str("_No failures in this run._\n");
    } else {
        out.push_str(
            "Each failure must become a finding (`F-AA-NN`) before/with the fix (§4.5).\n\n",
        );
        for c in failures {
            let _ = writeln!(
                out,
                "- `{}` ({}, {}): {}",
                c.id,
                c.format,
                c.schedule_ref,
                c.message.as_deref().unwrap_or("(no message)")
            );
        }
    }
    out
}

fn render_statement_md(results: &RunResults) -> String {
    let mut out = String::new();
    out.push_str("# openEHR Conformance Statement (generated)\n\n");
    out.push_str(
        "> Generated from a conformance run — never hand-asserted. This is a\n> scoped, honest statement; the deviations register below lists every\n> excluded capability with its structural reason.\n\n",
    );

    out.push_str("## 1. SUT identity\n\n");
    out.push_str("| Field | Value |\n|---|---|\n");
    let _ = writeln!(out, "| Base URL | `{}` |", results.sut.base_url);
    let _ = writeln!(out, "| RM version | {} |", results.sut.rm_version);
    let _ = writeln!(out, "| Auth mode | {} |", results.sut.auth_mode);
    let _ = write!(
        out,
        "| Corpus | `{}` @ `{}` |\n\n",
        results.corpus.repo, results.corpus.commit
    );

    out.push_str("## 2. Scope of test\n\n");
    out.push_str("| Field | Value |\n|---|---|\n");
    let _ = writeln!(
        out,
        "| Profiles requested | {} |",
        results
            .selection
            .profile
            .clone()
            .unwrap_or_else(|| "all".to_owned())
    );
    let _ = writeln!(
        out,
        "| Data formats | {} |",
        results.selection.formats.join(", ")
    );
    let _ = write!(
        out,
        "| Identified cases | {} |\n| Implemented | {} |\n| Passed | {} |\n| Failed | {} |\n\n",
        results.identified(),
        implemented_count(results),
        results.passed(),
        results.failed()
    );

    out.push_str("## 3. Detailed test report\n\n");
    if results.cases.is_empty() {
        out.push_str("_No cases executed in this run._\n\n");
    } else {
        out.push_str(
            "| Case | Capability | Format | Data sets | Result |\n|---|---|---|--:|---|\n",
        );
        for c in &results.cases {
            let result = match c.status {
                CaseStatus::Passed => "PASS",
                CaseStatus::Failed => "**FAIL**",
                CaseStatus::Errored => "ERROR",
                CaseStatus::Skipped => "skipped",
            };
            let _ = writeln!(
                out,
                "| `{}` | {} | {} | {}/{} | {result} |",
                c.id, c.capability, c.format, c.passed_data_sets, c.total_data_sets
            );
        }
        out.push('\n');
    }

    out.push_str("## 4. Deviations register\n\n");
    out.push_str(
        "Excluded capabilities/cases, by structural reason (never \"currently failing\"):\n\n",
    );
    let mut reasons: BTreeMap<&str, usize> = BTreeMap::new();
    for item in &results.inventory {
        if !item.is_implemented() {
            *reasons.entry(item.kind.as_str()).or_insert(0) += 1;
        }
    }
    out.push_str("| Reason | Cases |\n|---|--:|\n");
    for (reason, count) in &reasons {
        let _ = writeln!(out, "| {reason} | {count} |");
    }
    out
}

fn render_badge(results: &RunResults) -> String {
    let identified = results.identified();
    let passed = results.passed();
    let failed = results.failed();
    let color = if failed > 0 {
        "red"
    } else if identified > 0 && passed == identified {
        "brightgreen"
    } else {
        "yellow"
    };
    let message = format!("{passed}/{identified}");
    // shields.io endpoint schema.
    let badge = serde_json::json!({
        "schemaVersion": 1,
        "label": "openEHR CNF",
        "message": message,
        "color": color,
    });
    serde_json::to_string_pretty(&badge).unwrap_or_else(|_| "{}".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::results::{CorpusPin, InventoryClass, SelectionInfo, SutIdentity};

    fn zero_state_results() -> RunResults {
        let inventory = vec![
            InventoryClass {
                key: "A".to_owned(),
                id: "A".to_owned(),
                chapter: "master06".to_owned(),
                kind: "not_yet_transcribed".to_owned(),
            },
            InventoryClass {
                key: "B".to_owned(),
                id: "B".to_owned(),
                chapter: "master06".to_owned(),
                kind: "upstream_placeholder".to_owned(),
            },
        ];
        RunResults {
            sut: SutIdentity {
                base_url: "http://sut".to_owned(),
                rm_version: "1.2.0".to_owned(),
                auth_mode: "none".to_owned(),
            },
            corpus: CorpusPin::default(),
            started: "2026-07-07T00:00:00Z".to_owned(),
            selection: SelectionInfo::default(),
            cases: Vec::new(),
            inventory,
        }
    }

    #[test]
    fn zero_state_renders_all_artifacts() {
        let r = zero_state_results();
        let md = render_results_md(&r);
        assert!(md.contains("0 passed"));
        assert!(md.contains("_No failures in this run._"));
        let statement = render_statement_md(&r);
        assert!(statement.contains("not_yet_transcribed"));
        let badge = render_badge(&r);
        assert!(badge.contains("\"message\": \"0/2\""));
        assert!(badge.contains("yellow"));
    }

    #[test]
    fn write_all_produces_four_files() {
        let dir = std::env::temp_dir().join(format!("cnf-report-{}", std::process::id()));
        write_all(&zero_state_results(), &dir).expect("write");
        for name in [
            "results.json",
            "RESULTS.md",
            "CONFORMANCE_STATEMENT.md",
            "badge.json",
        ] {
            assert!(dir.join(name).exists(), "{name} written");
        }
        // Round-trip results.json.
        let back = from_results_file(&dir.join("results.json")).expect("read back");
        assert_eq!(back.identified(), 2);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
