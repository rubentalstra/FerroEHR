//! The cross-SUT comparison matrix (`COMPARISON.md`) — the X1 honest
//! comparison (`docs/plans/x1-comparison.md`): per capability, one column per
//! SUT, every cell derived from a committed `results.json`.
//!
//! Honesty constitution (X1 rules 1/4/8/10, carried into the target design §6):
//! measured numbers only; the fairness register is applied before a foreign
//! run is published (its extension routes read *not-applicable*, never
//! *failure*); no conformance certification is claimed for a foreign run
//! (Certificates are our own product only); and where upstream wins, the cell
//! says so plainly.

use std::fmt::Write as _;

use crate::model::case::Capability;
use crate::reporting::report::{ALL_CAPABILITIES, capability_count, edition_policy_label};
use crate::reporting::results::RunResults;
use crate::sut::descriptor::SutKind;

/// How a capability reads for one SUT in the comparison — a strict superset of
/// the profile evidence classification, keeping `not-applicable` (an
/// adjudicated extension) distinct from `not-evidenced` (skipped) and from
/// `no cases`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Cell {
    Pass,
    Fail,
    NotApplicable,
    NotEvidenced,
    NoCases,
}

impl Cell {
    fn of(results: &RunResults, cap: Capability) -> Self {
        let cc = capability_count(results, cap);
        if cc.failed + cc.errored > 0 {
            Cell::Fail
        } else if cc.passed > 0 {
            Cell::Pass
        } else if cc.not_applicable > 0 {
            Cell::NotApplicable
        } else if cc.skipped > 0 {
            Cell::NotEvidenced
        } else {
            Cell::NoCases
        }
    }

    fn word(self) -> &'static str {
        match self {
            Cell::Pass => "pass",
            Cell::Fail => "**fail**",
            Cell::NotApplicable => "not-applicable",
            Cell::NotEvidenced => "not-evidenced",
            Cell::NoCases => "—",
        }
    }

    /// Whether this cell counts as a capability being exercised at all.
    fn has_cases(self) -> bool {
        self != Cell::NoCases
    }
}

/// Render the cross-SUT comparison from two or more runs.
#[must_use]
pub fn render_comparison_md(runs: &[RunResults]) -> String {
    let mut out = String::from("# openEHR CDR conformance comparison (generated)\n\n");
    render_preamble(&mut out);
    render_sut_header(&mut out, runs);
    render_capability_matrix(&mut out, runs);
    out
}

fn render_preamble(out: &mut String) {
    out.push_str(
        "> **Measured, not asserted.** Every cell below is derived from a committed \
         `results.json`; nothing here is hand-entered.\n>\n\
         > - Foreign SUTs are triaged through a committed fairness register before \
         publication: an ehrbase-rs *extension* route (a capability the SUT does not \
         implement) reads `not-applicable`, never `fail`; a genuine spec gap reads `fail`.\n>\n\
         > - This published comparison makes **no certification claim on behalf of any other \
         vendor**: each cell is a capability result computed from that SUT's own run, never a \
         certificate reference. (Each run does produce its own self-assessment Certificate, \
         which is that operator's to publish, not ours.)\n>\n\
         > - Where a comparison SUT out-performs ehrbase-rs on a capability, its cell reads \
         `pass` while ours reads `fail`/`not-evidenced` — stated plainly, not hidden.\n\n",
    );
}

fn render_sut_header(out: &mut String, runs: &[RunResults]) {
    out.push_str("## Systems under test\n\n");
    out.push_str("| # | Product | Class | Base URL | Run date | Edition level |\n");
    out.push_str("|--:|---|---|---|---|---|\n");
    for (i, r) in runs.iter().enumerate() {
        let _ = writeln!(
            out,
            "| {} | {} {} | {} | `{}` | {} | {} |",
            i + 1,
            r.sut.product.name,
            r.sut.product.version,
            class_label(r.sut.kind),
            r.sut.base_url,
            r.started,
            sut_edition_label(r),
        );
    }
    out.push('\n');
}

fn class_label(kind: SutKind) -> &'static str {
    match kind {
        SutKind::Ours => "ours",
        SutKind::Foreign => "foreign",
    }
}

/// The edition rung(s) a SUT satisfied: the distinct laddered findings if any
/// assertion stepped below the newest form, else the run's edition policy.
fn sut_edition_label(results: &RunResults) -> String {
    let mut rungs: Vec<String> = results
        .cases
        .iter()
        .filter_map(|c| c.edition_level.clone())
        .collect();
    rungs.sort();
    rungs.dedup();
    if rungs.is_empty() {
        edition_policy_label(results.sut.edition_policy)
    } else {
        format!("findings: {}", rungs.join(", "))
    }
}

fn render_capability_matrix(out: &mut String, runs: &[RunResults]) {
    out.push_str("## Capability comparison\n\n");
    // Header row: Capability | SUT#1 | SUT#2 | …
    out.push_str("| Capability |");
    for r in runs {
        let _ = write!(out, " {} |", r.sut.product.name);
    }
    out.push('\n');
    out.push_str("|---|");
    for _ in runs {
        out.push_str("---|");
    }
    out.push('\n');

    for cap in ALL_CAPABILITIES {
        let cells: Vec<Cell> = runs.iter().map(|r| Cell::of(r, cap)).collect();
        // Skip a capability no run exercises at all.
        if cells.iter().all(|c| !c.has_cases()) {
            continue;
        }
        // A row where every exercising SUT reads not-applicable is an extension
        // row — label it.
        let exercised: Vec<Cell> = cells.iter().copied().filter(|c| c.has_cases()).collect();
        let is_extension =
            !exercised.is_empty() && exercised.iter().all(|c| *c == Cell::NotApplicable);
        let label = if is_extension {
            format!("{cap:?} _(extension — not applicable)_")
        } else {
            format!("{cap:?}")
        };
        let _ = write!(out, "| {label} |");
        for cell in cells {
            let _ = write!(out, " {} |", cell.word());
        }
        out.push('\n');
    }
    out.push('\n');
    out.push_str(
        "_Cells: `pass` (evidenced), `**fail**` (a conformance finding or transport error), \
         `not-applicable` (adjudicated extension / RM-version-sensitive, fairness register), \
         `not-evidenced` (only skipped cases), `—` (no case exercises it for that SUT)._\n",
    );
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
    use crate::edition::EditionPolicy;
    use crate::model::versions::SpecVersions;
    use crate::reporting::results::{
        CaseOutcome, CaseStatus, CorpusPin, ProductIdentity, SelectionInfo, SutIdentity,
    };

    fn outcome(capability: &str, status: CaseStatus) -> CaseOutcome {
        CaseOutcome {
            ecc_id: "ECC-DEM-001".to_owned(),
            id: "slug".to_owned(),
            title: "t".to_owned(),
            capability: capability.to_owned(),
            format: "json".to_owned(),
            status,
            passed_data_sets: 1,
            total_data_sets: 1,
            schedule_rows: None,
            message: None,
            citation: String::new(),
            schedule_ref: None,
            ecc_original: None,
            binding: String::new(),
            edition_level: None,
            edition_findings: Vec::new(),
            duration_ms: 0,
        }
    }

    fn run(name: &str, kind: SutKind, cases: Vec<CaseOutcome>) -> RunResults {
        RunResults {
            sut: SutIdentity {
                base_url: "http://sut".to_owned(),
                product: ProductIdentity {
                    name: name.to_owned(),
                    version: "1.0".to_owned(),
                    image_digest: None,
                },
                kind,
                edition_policy: EditionPolicy::Auto,
                versions: SpecVersions::latest(),
                auth_mode: "basic".to_owned(),
            },
            corpus: CorpusPin::default(),
            started: "2026-07-13T00:00:00Z".to_owned(),
            selection: SelectionInfo::default(),
            terminology: None,
            cases,
        }
    }

    #[test]
    fn extension_row_is_labelled_when_all_exercising_suts_are_na() {
        let ours = run(
            "ehrbase-rs",
            SutKind::Ours,
            vec![outcome("PartyOperations", CaseStatus::NotApplicable)],
        );
        let java = run(
            "ehrbase-java",
            SutKind::Foreign,
            vec![outcome("PartyOperations", CaseStatus::NotApplicable)],
        );
        let md = render_comparison_md(&[ours, java]);
        assert!(md.contains("extension — not applicable"));
        assert!(md.contains("Measured, not asserted"));
    }

    #[test]
    fn plainly_states_where_a_sut_wins() {
        let ours = run(
            "ehrbase-rs",
            SutKind::Ours,
            vec![outcome("EhrOperations", CaseStatus::Failed)],
        );
        let java = run(
            "ehrbase-java",
            SutKind::Foreign,
            vec![outcome("EhrOperations", CaseStatus::Passed)],
        );
        let md = render_comparison_md(&[ours, java]);
        // The row shows our fail beside their pass, in that column order.
        assert!(md.contains("| EhrOperations | **fail** | pass |"));
    }
}
