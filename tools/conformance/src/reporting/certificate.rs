//! The Conformance **Certificate** (`CONFORMANCE_CERTIFICATE.md`) — a
//! self-assessment artefact of **our own** product only (X1 fairness rule 4;
//! `docs/design/conformance/90-target-design.md` §7.5). Rendering a foreign
//! SUT returns [`CertificateError::ForeignSut`]; the caller
//! ([`crate::reporting::report::write_all`]) enforces the rule and explains the
//! suppression.
//!
//! The structure follows the vendored CNF template
//! (`certificate/master03-certificate.adoc`): a **System Under Test** block
//! (solution / vendor / assessor / infrastructure / date), a **Scope of Test**
//! block, a **Detailed Test Report** (one row per conformance point, with a
//! per-format result column), and a **Profile Report** (capability × required ×
//! result). The template is a filled-in fictional example — the only structural
//! template the DEVELOPMENT CNF guide ships — so the ECC is named as the
//! (self-)assessor rather than an external authority.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use crate::model::case::Profile;
use crate::model::catalog::Catalog;
use crate::reporting::report::{claim_word, evidence_word, profile_lines, profile_verdict};
use crate::reporting::results::{CaseStatus, RunResults};
use crate::sut::descriptor::SutKind;

/// The three profiles, in report order.
const PROFILES: [Profile; 3] = [Profile::Core, Profile::Standard, Profile::Options];

/// Why a Certificate could not be produced.
#[derive(Debug, thiserror::Error)]
pub enum CertificateError {
    /// The SUT is foreign — a Certificate is never manufactured for another
    /// product (X1 fairness rule 4).
    #[error(
        "refusing to emit a Conformance Certificate for foreign product `{product}` \
         (a self-assessment artefact is our own product only)"
    )]
    ForeignSut {
        /// The foreign product name.
        product: String,
    },
}

/// Render the Conformance Certificate for our own SUT.
///
/// # Errors
/// [`CertificateError::ForeignSut`] when the recorded SUT is not
/// [`SutKind::Ours`].
pub fn render_certificate_md(
    results: &RunResults,
    catalog: &Catalog,
) -> Result<String, CertificateError> {
    if results.sut.kind != SutKind::Ours {
        return Err(CertificateError::ForeignSut {
            product: results.sut.product.name.clone(),
        });
    }

    let mut out = String::new();
    let _ = writeln!(
        out,
        "# {} — Conformance Certificate (generated, self-assessed)\n",
        results.sut.product.name
    );
    out.push_str(
        "> A self-assessed certificate produced by the ehrbase-rs Conformance Catalogue (ECC)\n\
         > from a conformance run. Its structure follows the CNF\n\
         > `certificate/master03-certificate.adoc` template; every verdict is machine-computed\n\
         > from `results.json`.\n\n",
    );

    render_sut_block(&mut out, results);
    render_scope(&mut out, results);
    render_detailed_report(&mut out, results, catalog);
    render_profile_report(&mut out, results);
    Ok(out)
}

fn render_sut_block(out: &mut String, results: &RunResults) {
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
    let _ = writeln!(out, "| Vendor | {} (self-assessed) |", product.name);
    out.push_str("| Assessor | ehrbase-rs Conformance Catalogue (ECC) — self-assessment |\n");
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
mod tests {
    use super::*;
    use crate::edition::{Edition, EditionPolicy};
    use crate::model::versions::SpecVersions;
    use crate::reporting::results::{
        CaseOutcome, CorpusPin, ProductIdentity, SelectionInfo, SutIdentity,
    };

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
                edition_policy: EditionPolicy::Pinned(Edition::Development),
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
    fn foreign_sut_is_refused() {
        let r = results(SutKind::Foreign, Vec::new());
        let err = render_certificate_md(&r, &Catalog::default()).expect_err("must refuse");
        assert!(matches!(err, CertificateError::ForeignSut { .. }));
    }

    #[test]
    fn ours_renders_per_format_columns_and_profile_report() {
        let cert = render_certificate_md(
            &results(
                SutKind::Ours,
                vec![
                    outcome("ECC-EHR-001", "EhrOperations", "json", CaseStatus::Passed),
                    outcome("ECC-EHR-001", "EhrOperations", "xml", CaseStatus::Failed),
                ],
            ),
            &Catalog::default(),
        )
        .expect("ours must render");
        assert!(cert.contains("System Under Test"));
        assert!(cert.contains("Detailed Test Report"));
        assert!(cert.contains("Profile Report"));
        assert!(cert.contains("I_EHR_SERVICE.create_ehr-main"));
        // JSON passed, XML failed — one row, two columns.
        assert!(cert.contains("| pass | **FAIL** |"));
    }
}
