//! Report generation (design v4): from a [`RunResults`] write the committed,
//! public artifact set — `results.json` (machine-readable), `RESULTS.md` (the
//! per-area execution matrix), `CATALOG.md` (the full ECC catalogue per
//! area), `CONFORMANCE_STATEMENT.md` (the scoped claim + deviations), and
//! `badge.json` (shields endpoint schema).
//!
//! Everything is **catalogue-driven**: the denominator is our own ECC
//! catalogue, the identities are ECC numbers, and the claim is a pure
//! function of the run — never hand-asserted.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::Path;

use crate::catalog::{Area, Catalog, EccStatus};
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
    let catalog = Catalog::load_default().unwrap_or_default();
    let json =
        serde_json::to_string_pretty(results).map_err(|e| ReportError::Codec(e.to_string()))?;
    write_file(&out_dir.join("results.json"), &json)?;
    write_file(
        &out_dir.join("RESULTS.md"),
        &render_results_md(results, &catalog),
    )?;
    write_file(
        &out_dir.join("CATALOG.md"),
        &render_catalog_md(results, &catalog),
    )?;
    write_file(
        &out_dir.join("CONFORMANCE_STATEMENT.md"),
        &render_statement_md(results, &catalog),
    )?;
    write_file(
        &out_dir.join("badge.json"),
        &render_badge(results, &catalog),
    )?;
    Ok(())
}

fn write_file(path: &Path, content: &str) -> Result<(), ReportError> {
    std::fs::write(path, content).map_err(|source| ReportError::Io {
        path: path.display().to_string(),
        source,
    })
}

/// The catalogue's active-case count per area (the coverage denominator).
fn active_per_area(catalog: &Catalog) -> BTreeMap<Area, usize> {
    let mut by: BTreeMap<Area, usize> = BTreeMap::new();
    for e in catalog.entries() {
        if e.status == EccStatus::Active {
            *by.entry(e.area).or_insert(0) += 1;
        }
    }
    by
}

/// Per-area execution tallies from the run outcomes, resolved through the
/// catalogue (outcomes carry the ECC id).
#[derive(Default, Clone, Copy)]
struct AreaTally {
    passed: usize,
    failed: usize,
    errored: usize,
    skipped: usize,
}

fn tally_by_area(results: &RunResults, catalog: &Catalog) -> BTreeMap<Area, AreaTally> {
    let mut by: BTreeMap<Area, AreaTally> = BTreeMap::new();
    for case in &results.cases {
        let Some(area) = catalog
            .entries()
            .iter()
            .find(|e| e.ecc_id == case.ecc_id)
            .map(|e| e.area)
        else {
            continue;
        };
        let t = by.entry(area).or_default();
        match case.status {
            CaseStatus::Passed => t.passed += 1,
            CaseStatus::Failed => t.failed += 1,
            CaseStatus::Errored => t.errored += 1,
            CaseStatus::Skipped => t.skipped += 1,
        }
    }
    by
}

fn render_header(out: &mut String, results: &RunResults) {
    let _ = write!(
        out,
        "- SUT: `{}`\n- Spec versions: RM {} · ITS-REST {} · AQL {} · TERM {}\n\
         - Auth mode: {}\n- Started: {}\n\n",
        results.sut.base_url,
        results.sut.versions.rm,
        results.sut.versions.its_rest,
        results.sut.versions.aql,
        results.sut.versions.term,
        results.sut.auth_mode,
        results.started
    );
}

fn render_results_md(results: &RunResults, catalog: &Catalog) -> String {
    let by = tally_by_area(results, catalog);
    let denominators = active_per_area(catalog);
    let mut out = String::new();
    out.push_str("# ehrbase-rs Conformance — Test Execution Report\n\n");
    render_header(&mut out, results);
    let _ = write!(
        out,
        "**{} case×format executions · {} passed · {} failed.**\n\n",
        results.executed(),
        results.passed(),
        results.failed(),
    );

    out.push_str("## Per-area matrix\n\n");
    out.push_str("| Area | Catalogue (active) | Passed | Failed | Errored | Skipped |\n");
    out.push_str("|---|--:|--:|--:|--:|--:|\n");
    for area in Area::ALL {
        let denom = denominators.get(&area).copied().unwrap_or(0);
        if denom == 0 && !by.contains_key(&area) {
            continue;
        }
        let t = by.get(&area).copied().unwrap_or_default();
        let _ = writeln!(
            out,
            "| {} — {} | {} | {} | {} | {} | {} |",
            area.tag(),
            area.title(),
            denom,
            t.passed,
            t.failed,
            t.errored,
            t.skipped
        );
    }

    // Failures section — each links to the finding workflow.
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
            "Each failure must become a finding (`F-AA-NN`) before/with the fix — never an exclusion.\n\n",
        );
        for c in failures {
            let _ = writeln!(
                out,
                "- **{}** (`{}`, {}): {}",
                c.ecc_id,
                c.id,
                c.format,
                c.message.as_deref().unwrap_or("(no message)")
            );
        }
    }
    out
}

/// Render `CATALOG.md` — the full ECC catalogue grouped per area: every
/// allocated case with its status and its latest run outcome.
fn render_catalog_md(results: &RunResults, catalog: &Catalog) -> String {
    let mut out = String::from(
        "# The ehrbase-rs Conformance Catalogue (ECC)\n\n\
         Generated per run — do not edit. Numbers are allocated once in\n\
         `crates/ehrbase-conformance/inventory/ecc-catalog.tsv` and never reused.\n\n",
    );

    let outcome_of = |ecc_id: &str| {
        results
            .cases
            .iter()
            .filter(|c| c.ecc_id == ecc_id)
            .map(|c| format!("{:?}", c.status).to_lowercase())
            .collect::<Vec<_>>()
            .join("/")
    };

    for area in Area::ALL {
        let lines: Vec<_> = catalog
            .entries()
            .iter()
            .filter(|e| e.area == area)
            .collect();
        if lines.is_empty() {
            continue;
        }
        let active = lines
            .iter()
            .filter(|e| e.status == EccStatus::Active)
            .count();
        let _ = writeln!(
            out,
            "## {} — {} ({} cases, {} active)\n",
            area.tag(),
            area.title(),
            lines.len(),
            active
        );
        out.push_str("| ECC id | Status | Title | Last run |\n");
        out.push_str("|---|---|---|---|\n");
        for e in lines {
            let run = outcome_of(&e.ecc_id);
            let _ = writeln!(
                out,
                "| {} | {:?} | {} | {} |",
                e.ecc_id,
                e.status,
                e.title,
                if run.is_empty() {
                    "—".to_owned()
                } else {
                    run
                }
            );
        }
        out.push('\n');
    }
    out
}

fn render_statement_md(results: &RunResults, catalog: &Catalog) -> String {
    let mut out = String::new();
    out.push_str("# ehrbase-rs Conformance Statement (generated)\n\n");
    out.push_str(
        "> Generated from a conformance run — never hand-asserted. Scoped and\n\
         > honest: the deviations section lists every skip with its reason.\n\n",
    );

    out.push_str("## 1. SUT identity\n\n");
    render_header(&mut out, results);

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
    let active = catalog
        .entries()
        .iter()
        .filter(|e| e.status == EccStatus::Active)
        .count();
    let _ = write!(
        out,
        "| Catalogue (active cases) | {active} |\n| Executed | {} |\n| Passed | {} |\n| Failed | {} |\n\n",
        results.executed(),
        results.passed(),
        results.failed()
    );

    out.push_str("## 3. Detailed test report\n\n");
    if results.cases.is_empty() {
        out.push_str("_No cases executed in this run._\n\n");
    } else {
        out.push_str(
            "| ECC id | Capability | Format | Data sets | Result |\n|---|---|---|--:|---|\n",
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
                "| {} | {} | {} | {}/{} | {result} |",
                c.ecc_id, c.capability, c.format, c.passed_data_sets, c.total_data_sets
            );
        }
        out.push('\n');
    }

    out.push_str("## 4. Profile verdict (machine-computed, all-or-nothing)\n\n");
    for profile in [
        crate::case::Profile::Core,
        crate::case::Profile::Standard,
        crate::case::Profile::Options,
    ] {
        let v = crate::profile::verdict(profile, results);
        let _ = writeln!(
            out,
            "### {profile:?} — {}\n",
            if v.pass { "**PASS**" } else { "not claimable" }
        );
        out.push_str("| Capability | Passed | Failed | Errored | Skipped | Verdict |\n");
        out.push_str("|---|--:|--:|--:|--:|---|\n");
        for c in &v.capabilities {
            let _ = writeln!(
                out,
                "| {} | {} | {} | {} | {} | {} |",
                c.capability,
                c.passed,
                c.failed,
                c.errored,
                c.skipped,
                if c.pass { "pass" } else { "fail" }
            );
        }
        out.push('\n');
    }

    out.push_str("## 5. Deviations (skips), by reason\n\n");
    let mut reasons: BTreeMap<&str, usize> = BTreeMap::new();
    for c in &results.cases {
        if c.status == CaseStatus::Skipped {
            *reasons
                .entry(c.message.as_deref().unwrap_or("(unstated)"))
                .or_insert(0) += 1;
        }
    }
    if reasons.is_empty() {
        out.push_str("_No skips in this run._\n");
    } else {
        out.push_str("| Reason | Cases |\n|---|--:|\n");
        for (reason, count) in &reasons {
            let _ = writeln!(out, "| {reason} | {count} |");
        }
    }
    out
}

fn render_badge(results: &RunResults, catalog: &Catalog) -> String {
    let active = catalog
        .entries()
        .iter()
        .filter(|e| e.status == EccStatus::Active)
        .count();
    let passed = results.passed();
    let failed = results.failed();
    let color = if failed > 0 {
        "red"
    } else if active > 0 && passed >= active {
        "brightgreen"
    } else {
        "yellow"
    };
    let badge = serde_json::json!({
        "schemaVersion": 1,
        "label": "ECC conformance",
        "message": format!("{passed}/{active}"),
        "color": color,
    });
    serde_json::to_string_pretty(&badge).unwrap_or_else(|_| "{}".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::results::{CorpusPin, SelectionInfo, SutIdentity};
    use crate::version::SpecVersions;

    fn zero_state_results() -> RunResults {
        RunResults {
            sut: SutIdentity {
                base_url: "http://sut".to_owned(),
                versions: SpecVersions::latest(),
                auth_mode: "none".to_owned(),
            },
            corpus: CorpusPin::default(),
            started: "2026-07-07T00:00:00Z".to_owned(),
            selection: SelectionInfo::default(),
            cases: Vec::new(),
        }
    }

    #[test]
    fn zero_state_renders_all_artifacts() {
        let r = zero_state_results();
        let catalog = Catalog::default();
        let md = render_results_md(&r, &catalog);
        assert!(md.contains("0 passed"));
        assert!(md.contains("_No failures in this run._"));
        let statement = render_statement_md(&r, &catalog);
        assert!(statement.contains("Scope of test"));
        let badge = render_badge(&r, &catalog);
        assert!(badge.contains("\"message\": \"0/0\""));
        assert!(badge.contains("yellow"));
    }

    #[test]
    fn write_all_produces_the_artifact_set() {
        let dir = std::env::temp_dir().join(format!("ecc-report-{}", std::process::id()));
        write_all(&zero_state_results(), &dir).expect("write");
        for name in [
            "results.json",
            "RESULTS.md",
            "CATALOG.md",
            "CONFORMANCE_STATEMENT.md",
            "badge.json",
        ] {
            assert!(dir.join(name).exists(), "{name} written");
        }
        let back = from_results_file(&dir.join("results.json")).expect("read back");
        assert_eq!(back.executed(), 0);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
