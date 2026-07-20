//! The Conformance **Certificate** (`CONFORMANCE_CERTIFICATE.md`).
//!
//! Owner ruling (2026-07-13): the Certificate is emitted for **any** SUT — the
//! framework aims to be the industry-standard CNF validator, so any operator
//! certifying their own CDR (ours, upstream, or a bring-your-own endpoint) gets
//! the artefact. It therefore carries a mandatory **assessment-basis honesty
//! block**: the assessor identity (default: self-assessment via the ECC
//! framework, overridable with `--assessor`), an explicit statement that this
//! is **not** an official openEHR certification (no such program exists), and
//! the machine-computed provenance (the attached `results.json`, the run date,
//! the ECC framework version + catalogue identity).
//!
//! The structure follows the vendored CNF template
//! (`certificate/master03-certificate.adoc`): a **System Under Test** block, a
//! **Scope of Test** block (which lists any fairness-register not-applicable
//! adjudications, so a foreign-SUT claim is scoped to applicable capabilities),
//! a **Detailed Test Report** (one row per conformance point, per-format result
//! columns), and a **Profile Report** (capability × required × result).

use std::collections::BTreeMap;
use std::fmt::Write as _;

use crate::model::case::Profile;
use crate::model::catalog::Catalog;
use crate::reporting::report::{claim_word, evidence_word, profile_lines, profile_verdict};
use crate::reporting::results::{CaseOutcome, CaseStatus, RunResults};

/// The three profiles, in report order.
const PROFILES: [Profile; 3] = [Profile::Core, Profile::Standard, Profile::Options];

/// The default assessor line when no `--assessor` override is given.
const DEFAULT_ASSESSOR: &str =
    "self-assessment via the ehrbase-rs Conformance Catalogue (ECC) framework";

/// Render the Conformance Certificate for any SUT. `assessor` overrides the
/// default self-assessment attribution.
#[must_use]
pub fn render_certificate_md(
    results: &RunResults,
    catalog: &Catalog,
    assessor: Option<&str>,
) -> String {
    let assessor = assessor.unwrap_or(DEFAULT_ASSESSOR);
    let mut out = String::new();
    let _ = writeln!(
        out,
        "# {} — Conformance Certificate (generated)\n",
        results.sut.product.name
    );

    render_honesty_block(&mut out, results, assessor);
    render_sut_block(&mut out, results, assessor);
    render_scope(&mut out, results);
    render_detailed_report(&mut out, results, catalog);
    render_profile_report(&mut out, results);
    out
}

fn render_honesty_block(out: &mut String, results: &RunResults, assessor: &str) {
    out.push_str("## Assessment basis (read first)\n\n");
    let _ = writeln!(out, "- **Assessor:** {assessor}");
    out.push_str(
        "- **This is NOT an official openEHR conformance certification.** No official openEHR \
         certification program exists; this artefact is a self-assessment produced by an \
         independent framework.\n\
         - **Machine-computed:** every verdict below is a pure function of the attached run \
         (`results.json`) — never hand-asserted.\n",
    );
    let _ = writeln!(
        out,
        "- **ECC framework version:** {} · catalogue `inventory/ecc-catalog.tsv`",
        env!("CARGO_PKG_VERSION")
    );
    let _ = writeln!(out, "- **Machine record:** `results.json` (this directory)");
    let _ = writeln!(out, "- **Run date:** {}\n", results.started);
}

fn render_sut_block(out: &mut String, results: &RunResults, assessor: &str) {
    let product = &results.sut.product;
    out.push_str("## System Under Test (SUT)\n\n| | |\n|---|---|\n");
    let _ = writeln!(
        out,
        "| Solution | {} {} @ `{}` |",
        product.name, product.version, results.sut.base_url
    );
    if let Some(digest) = &product.image_digest {
        let _ = writeln!(out, "| Image digest | `{digest}` |");
    }
    let _ = writeln!(out, "| Vendor | {} |", product.name);
    let _ = writeln!(out, "| Assessor | {assessor} |");
    let _ = writeln!(
        out,
        "| Infrastructure | reference corpus {}@{}; SUT auth mode {} |",
        results.corpus.repo, results.corpus.commit, results.sut.auth_mode
    );
    let _ = writeln!(out, "| Date | {} |\n", results.started);
}

fn render_scope(out: &mut String, results: &RunResults) {
    out.push_str("## Scope of Test\n\n| | |\n|---|---|\n");
    let functional: Vec<String> = PROFILES
        .iter()
        .map(|&p| format!("{p:?} ({})", claim_word(p, profile_verdict(results, p))))
        .collect();
    let _ = writeln!(out, "| Functional | {} |", functional.join(", "));
    let _ = writeln!(
        out,
        "| Sec & Priv | Signing {}, Anonymous EHRs {} |",
        capability_word(results, "Signing"),
        capability_word(results, "AnonymousEhrs"),
    );
    let _ = writeln!(
        out,
        "| Ext Data Fmt | {} |\n",
        if results.selection.formats.is_empty() {
            "—".to_owned()
        } else {
            results.selection.formats.join(", ")
        }
    );

    // The claim is scoped to applicable capabilities: any fairness-register
    // not-applicable adjudication (a foreign SUT's extension / RM-version
    // exclusions) is listed here so the scope is explicit.
    let mut order: Vec<String> = Vec::new();
    let mut seen: BTreeMap<String, &CaseOutcome> = BTreeMap::new();
    for c in results
        .cases
        .iter()
        .filter(|c| c.status == CaseStatus::NotApplicable)
    {
        let key = if c.ecc_id.is_empty() {
            c.id.clone()
        } else {
            c.ecc_id.clone()
        };
        seen.entry(key.clone()).or_insert_with(|| {
            order.push(key.clone());
            c
        });
    }
    if !order.is_empty() {
        out.push_str("### Scope exclusions (adjudicated not-applicable)\n\n");
        out.push_str(
            "The following capabilities are excluded from this claim per the committed \
             fairness register (adjudicated extensions / RM-version-sensitive comparisons); \
             the claim is scoped to the applicable capabilities.\n\n",
        );
        for key in &order {
            let c = seen[key];
            let _ = writeln!(
                out,
                "- **{}** {} — {} _(cite: {})_",
                c.ecc_id,
                c.title,
                c.message.as_deref().unwrap_or("(no reason)"),
                if c.citation.is_empty() {
                    "—"
                } else {
                    c.citation.as_str()
                },
            );
        }
        out.push('\n');
    }
}

/// The evidence word of a named capability in the STANDARD scope (or `n/a`).
fn capability_word(results: &RunResults, capability_debug: &str) -> &'static str {
    profile_lines(results, Profile::Standard)
        .into_iter()
        .find(|l| format!("{:?}", l.capability) == capability_debug)
        .map_or("n/a", |l| evidence_word(l.evidence))
}

/// One collapsed Detailed-Test-Report row: one conformance point (formats
/// spread across their own result columns).
struct DetailRow {
    component: &'static str,
    capability: String,
    conformance_point: String,
    ecc_id: String,
    title: String,
    json: &'static str,
    xml: &'static str,
}

fn render_detailed_report(out: &mut String, results: &RunResults, catalog: &Catalog) {
    out.push_str("## Detailed Test Report\n\n");
    out.push_str(
        "One row per ECC case. *Conformance point* is the CNF-schedule \
         `<SERVICE>.<operation>` trace where the case concretizes one, else the ITS-REST \
         binding (an ECC-original case is never presented as schedule-conformant — see the \
         report's ECC-original section). Results are per data format; a format not run \
         shows `—`. (There is no protobuf technology under test — the CNF template's \
         protobuf column is omitted.)\n\n",
    );
    out.push_str(
        "| openEHR Component | Capability | Conformance point | Test Case | JSON | XML |\n\
         |---|---|---|---|---|---|\n",
    );
    for row in collapse_cases(results, catalog) {
        let _ = writeln!(
            out,
            "| {} | {} | {} | {} — {} | {} | {} |",
            row.component,
            row.capability,
            row.conformance_point,
            row.ecc_id,
            row.title,
            row.json,
            row.xml,
        );
    }
    out.push('\n');
}

fn collapse_cases(results: &RunResults, catalog: &Catalog) -> Vec<DetailRow> {
    let area_of = |ecc_id: &str| -> &'static str {
        catalog
            .entries()
            .iter()
            .find(|e| e.ecc_id == ecc_id)
            .map_or("—", |e| e.area.title())
    };

    let mut order: Vec<String> = Vec::new();
    let mut rows: BTreeMap<String, DetailRow> = BTreeMap::new();
    for c in &results.cases {
        let key = if c.ecc_id.is_empty() {
            c.id.clone()
        } else {
            c.ecc_id.clone()
        };
        let row = rows.entry(key.clone()).or_insert_with(|| {
            order.push(key.clone());
            let conformance_point = c.schedule_ref.clone().unwrap_or_else(|| c.binding.clone());
            DetailRow {
                component: area_of(&c.ecc_id),
                capability: c.capability.clone(),
                conformance_point,
                ecc_id: key.clone(),
                title: c.title.clone(),
                json: "—",
                xml: "—",
            }
        });
        let word = verdict_word(c.status);
        match c.format.as_str() {
            "json" => row.json = word,
            "xml" => row.xml = word,
            _ => {}
        }
    }

    order
        .into_iter()
        .filter_map(|key| rows.remove(&key))
        .collect()
}

fn render_profile_report(out: &mut String, results: &RunResults) {
    out.push_str("## Profile Report\n\n");
    for profile in PROFILES {
        let verdict = profile_verdict(results, profile);
        let _ = writeln!(out, "### {profile:?} — {}\n", claim_word(profile, verdict));
        out.push_str("| Capability | Required in profile | Result |\n|---|:--:|---|\n");
        let required = required_marker(profile);
        for line in profile_lines(results, profile) {
            let _ = writeln!(
                out,
                "| {:?} | {required} | {} |",
                line.capability,
                evidence_word(line.evidence),
            );
        }
        out.push('\n');
    }
}

/// The "Required in profile" marker: gated profiles require every listed
/// capability (`Y`); OPTIONS capabilities are optional (`OPT`).
fn required_marker(profile: Profile) -> &'static str {
    match profile {
        Profile::Core | Profile::Standard => "Y",
        Profile::Options => "OPT",
    }
}

/// The per-format cell word for a case status.
fn verdict_word(status: CaseStatus) -> &'static str {
    match status {
        CaseStatus::Passed => "pass",
        CaseStatus::Failed => "**FAIL**",
        CaseStatus::Errored => "ERROR",
        CaseStatus::Skipped => "skipped",
        CaseStatus::NotApplicable => "n/a",
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
    use crate::edition::{Edition, EditionPolicy};
    use crate::model::versions::SpecVersions;
    use crate::reporting::results::{
        CaseOutcome, CorpusPin, ProductIdentity, SelectionInfo, SutIdentity,
    };
    use crate::sut::descriptor::SutKind;

    fn outcome(ecc_id: &str, capability: &str, format: &str, status: CaseStatus) -> CaseOutcome {
        CaseOutcome {
            ecc_id: ecc_id.to_owned(),
            id: "slug".to_owned(),
            title: "A case".to_owned(),
            capability: capability.to_owned(),
            format: format.to_owned(),
            status,
            passed_data_sets: 1,
            total_data_sets: 1,
            schedule_rows: None,
            message: None,
            citation: String::new(),
            schedule_ref: Some("I_EHR_SERVICE.create_ehr-main".to_owned()),
            ecc_original: None,
            binding: "POST /ehr".to_owned(),
            edition_level: None,
            edition_findings: Vec::new(),
            duration_ms: 0,
        }
    }

    fn results(kind: SutKind, cases: Vec<CaseOutcome>) -> RunResults {
        RunResults {
            sut: SutIdentity {
                base_url: "http://sut".to_owned(),
                product: ProductIdentity::default(),
                kind,
                edition_policy: EditionPolicy::Pinned(Edition::Release110),
                versions: SpecVersions::latest(),
                auth_mode: "basic".to_owned(),
            },
            corpus: CorpusPin::default(),
            started: "2026-07-13T00:00:00Z".to_owned(),
            selection: SelectionInfo {
                filter: None,
                profile: None,
                formats: vec!["json".to_owned(), "xml".to_owned()],
            },
            terminology: None,
            cases,
        }
    }

    #[test]
    fn foreign_sut_is_certified_with_honesty_block() {
        let mut r = results(
            SutKind::Foreign,
            vec![outcome(
                "ECC-EHR-001",
                "EhrOperations",
                "json",
                CaseStatus::Passed,
            )],
        );
        r.sut.product = ProductIdentity {
            name: "ehrbase-java".to_owned(),
            version: "2.34.0".to_owned(),
            image_digest: None,
        };
        let cert = render_certificate_md(&r, &Catalog::default(), None);
        assert!(cert.starts_with("# ehrbase-java — Conformance Certificate"));
        assert!(cert.contains("NOT an official openEHR conformance certification"));
        assert!(cert.contains(DEFAULT_ASSESSOR));
        assert!(cert.contains("System Under Test"));
        assert!(cert.contains("Profile Report"));
    }

    #[test]
    fn assessor_override_and_scope_exclusions_are_listed() {
        let mut na = outcome(
            "ECC-DEM-001",
            "PartyOperations",
            "json",
            CaseStatus::NotApplicable,
        );
        na.message = Some("Upstream has no demographic REST API.".to_owned());
        na.citation =
            "no openEHR spec governs this fairness call — our own design/extension".to_owned();
        let r = results(SutKind::Foreign, vec![na]);
        let cert = render_certificate_md(&r, &Catalog::default(), Some("ACME Assessors Ltd"));
        assert!(cert.contains("| Assessor | ACME Assessors Ltd |"));
        assert!(cert.contains("Scope exclusions"));
        assert!(cert.contains("no demographic REST API"));
    }

    #[test]
    fn ours_renders_per_format_columns() {
        let cert = render_certificate_md(
            &results(
                SutKind::Ours,
                vec![
                    outcome("ECC-EHR-001", "EhrOperations", "json", CaseStatus::Passed),
                    outcome("ECC-EHR-001", "EhrOperations", "xml", CaseStatus::Failed),
                ],
            ),
            &Catalog::default(),
            None,
        );
        assert!(cert.contains("I_EHR_SERVICE.create_ehr-main"));
        assert!(cert.contains("| pass | **FAIL** |"));
    }
}
