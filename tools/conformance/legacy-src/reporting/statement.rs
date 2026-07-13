//! The Conformance **Statement** and **Certificate** artefacts (R9/R10,
//! `docs/blueprint/07-cnf.md`), emitted from a [`RunResults`] alongside the run
//! report.
//!
//! Both are pure functions of the machine profile verdicts ([`crate::profile`])
//! — never hand-asserted — and follow the shapes templated in the vendored CNF
//! `certificate/master03-certificate.adoc` (a filled-in fictional example, the
//! only structural template the DEVELOPMENT CNF guide ships; the guide's own
//! `master05-assessment.adoc` leaves *"Conformance Statement — TBD"* /
//! *"Conformance Certification — TBD"*):
//!
//! - **Statement** (`CONFORMANCE_STATEMENT.md`): the SUT's declared identity, the
//!   supported specification versions (the CNF-required RM-version statement,
//!   `master03-overview.adoc`), the external data formats, and the
//!   machine-computed profile claims.
//! - **Certificate** (`CONFORMANCE_CERTIFICATE.md`): the certificate template's
//!   System-Under-Test + Scope-of-Test tables, a **Profile Report**
//!   (capability × required × result — the report §4 verdict tables as raw
//!   material), and a **Detailed Test Report** keyed per conformance point (the
//!   case's [`crate::case::CaseMeta::schedule_ref`] where one exists).
//!
//! Self-assessed: the ECC is the assessing instrument, so the certificate names
//! the ECC as assessor rather than an external authority (the CNF certificate is
//! otherwise issued by "an assessing authority", `guide/master04-framework.adoc`).

use std::collections::BTreeMap;
use std::fmt::Write as _;

use crate::case::Profile;
use crate::catalog::Catalog;
use crate::profile::{ProfileVerdict, verdict};
use crate::results::{CaseStatus, RunResults};

/// The three profiles, in report order.
const PROFILES: [Profile; 3] = [Profile::Core, Profile::Standard, Profile::Options];

/// Render the Conformance Statement (`CONFORMANCE_STATEMENT.md`).
#[must_use]
pub fn render_statement_md(results: &RunResults) -> String {
    let mut out = String::from(
        "# ehrbase-rs Conformance Statement (generated)\n\n\
         > Generated from a conformance run (`results.json`) — never hand-asserted.\n\
         > The claim on every line is a pure function of the machine profile\n\
         > verdicts (`tools/conformance` §profile).\n\n",
    );

    out.push_str("## System under test\n\n| Field | Value |\n|---|---|\n");
    let _ = writeln!(
        out,
        "| Product | {} {} |",
        results.sut.product.name, results.sut.product.version
    );
    if let Some(digest) = &results.sut.product.image_digest {
        let _ = writeln!(out, "| Image digest | `{digest}` |");
    }
    let _ = writeln!(out, "| SUT | `{}` |", results.sut.base_url);
    let _ = writeln!(out, "| Auth mode | {} |", results.sut.auth_mode);
    let _ = writeln!(out, "| Run started | {} |", results.started);
    let _ = writeln!(
        out,
        "| Reference corpus | {}@{} |",
        results.corpus.repo, results.corpus.commit
    );

    out.push_str("\n## Supported specification versions\n\n");
    out.push_str("| Specification | Version |\n|---|---|\n");
    let v = &results.sut.versions;
    let _ = writeln!(out, "| Reference Model (RM) | {} |", v.rm);
    let _ = writeln!(out, "| ITS-REST contract | {} |", v.its_rest);
    let _ = writeln!(out, "| AQL (QUERY) | {} |", v.aql);
    let _ = writeln!(out, "| Terminology (TERM) | {} |", v.term);
    let _ = writeln!(
        out,
        "\n> CNF requires the Conformance Statement to state the supported RM \
         version(s); the minimum required is RM 1.0.2 \
         (`master03-overview.adoc`). This SUT states **RM {}**.\n",
        v.rm
    );

    out.push_str("\n## External data formats\n\n");
    let _ = writeln!(
        out,
        "Declared: XML, JSON (`master03-profiles.adoc` §Other Non-Functional). \
         This run exercised: {}.\n",
        if results.selection.formats.is_empty() {
            "—".to_owned()
        } else {
            results.selection.formats.join(", ")
        }
    );

    out.push_str("\n## Profile claims (machine-computed)\n\n");
    out.push_str("| Profile | Aggregation | Result |\n|---|---|---|\n");
    for profile in PROFILES {
        let pv = verdict(profile, results);
        let _ = writeln!(
            out,
            "| {profile:?} | {} | {} |",
            aggregation(profile),
            claim_word(profile, &pv)
        );
    }

    // Non-functional attributes (master03-profiles §Non-Functional).
    out.push_str("\n### Non-functional attributes\n\n");
    let standard = verdict(Profile::Standard, results);
    let signing = capability_label(&standard, "Signing");
    let anon = capability_label(&standard, "AnonymousEhrs");
    let _ = writeln!(out, "- Signing (STANDARD): {signing}");
    let _ = writeln!(out, "- Anonymous EHRs (CORE + STANDARD): {anon}");

    // The obtained optional capabilities (the substance of an OPTIONS claim).
    out.push_str("\n### OPTIONS — obtained optional capabilities\n\n");
    let options = verdict(Profile::Options, results);
    let obtained: Vec<&str> = options
        .capabilities
        .iter()
        .filter(|c| c.pass)
        .map(|c| c.capability.as_str())
        .collect();
    if obtained.is_empty() {
        out.push_str("_None obtained in this run._\n");
    } else {
        for cap in obtained {
            let _ = writeln!(out, "- {cap}");
        }
    }
    out
}

/// Render the Conformance Certificate (`CONFORMANCE_CERTIFICATE.md`).
#[must_use]
pub fn render_certificate_md(results: &RunResults, catalog: &Catalog) -> String {
    let mut out = String::from(
        "# ehrbase-rs Conformance Certificate (generated, self-assessed)\n\n\
         > A self-assessed certificate produced by the ehrbase-rs Conformance\n\
         > Catalogue (ECC) from a conformance run. Its structure follows the CNF\n\
         > `certificate/master03-certificate.adoc` template; every verdict is\n\
         > machine-computed from `results.json`.\n\n",
    );

    // ── System Under Test (SUT) ─────────────────────────────────────────────
    // Solution/Vendor are driven by the recorded product identity, never
    // hard-coded (§3a.1/§3a.2): this artifact is only emitted for an
    // `ehrbase-rs` SUT (`report::write_all`), so the values below are our own,
    // but they now read from the data rather than a literal.
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
    let _ = writeln!(out, "| Date | {} |", results.started);

    // ── Scope of Test ───────────────────────────────────────────────────────
    out.push_str("\n## Scope of Test\n\n| | |\n|---|---|\n");
    let functional: Vec<String> = PROFILES
        .iter()
        .map(|&p| format!("{p:?} ({})", claim_word(p, &verdict(p, results))))
        .collect();
    let _ = writeln!(out, "| Functional | {} |", functional.join(", "));
    let standard = verdict(Profile::Standard, results);
    let _ = writeln!(
        out,
        "| Sec & Priv | Signing {}, Anonymous EHRs {} |",
        capability_label(&standard, "Signing"),
        capability_label(&standard, "AnonymousEhrs"),
    );
    let _ = writeln!(
        out,
        "| Ext Data Fmt | {} |",
        if results.selection.formats.is_empty() {
            "—".to_owned()
        } else {
            results.selection.formats.join(", ")
        }
    );

    // ── Profile Report (report §4 verdict tables as raw material) ────────────
    out.push_str("\n## Profile Report\n\n");
    for profile in PROFILES {
        let pv = verdict(profile, results);
        let _ = writeln!(out, "### {profile:?} — {}\n", claim_word(profile, &pv));
        out.push_str("| Capability | Required in profile | Result |\n|---|:--:|---|\n");
        let required = required_marker(profile);
        for c in &pv.capabilities {
            let _ = writeln!(out, "| {} | {required} | {} |", c.capability, c.label());
        }
        out.push('\n');
    }

    // ── Detailed Test Report (per conformance point) ─────────────────────────
    out.push_str("## Detailed Test Report\n\n");
    out.push_str(
        "One row per ECC case (formats collapsed to a combined REST verdict). \
         *Conformance point* is the CNF-schedule `<SERVICE>.<operation>` id where \
         the case traces to one, else `—`. (There is no protobuf technology under \
         test — the CNF template's protobuf column is omitted.)\n\n",
    );
    out.push_str(
        "| openEHR Component | Capability | Conformance point | Test Case | REST |\n\
         |---|---|---|---|---|\n",
    );
    for row in collapse_cases(results, catalog) {
        let _ = writeln!(
            out,
            "| {} | {} | {} | {} — {} | {} |",
            row.component,
            row.capability,
            row.conformance_point,
            row.ecc_id,
            row.title,
            row.verdict,
        );
    }
    out
}

/// The all-or-nothing / any-passes aggregation word for a profile.
fn aggregation(profile: Profile) -> &'static str {
    match profile {
        Profile::Core | Profile::Standard => "all capabilities",
        Profile::Options => "any optional capability",
    }
}

/// The claim word for a profile verdict (OPTIONS is "obtained", the gated
/// profiles "PASS"/"not claimable").
fn claim_word(profile: Profile, v: &ProfileVerdict) -> &'static str {
    match profile {
        Profile::Options if v.pass => "OBTAINED",
        Profile::Options => "not obtained",
        _ if v.pass => "PASS",
        _ => "not claimable",
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

/// The display label of a named capability within a profile verdict, or `n/a`
/// if the profile does not require it.
fn capability_label(v: &ProfileVerdict, capability: &str) -> &'static str {
    v.capabilities
        .iter()
        .find(|c| c.capability == capability)
        .map_or("n/a", crate::profile::CapabilityVerdict::label)
}

/// One collapsed Detailed-Test-Report row (formats merged).
struct DetailRow {
    component: &'static str,
    capability: String,
    conformance_point: String,
    ecc_id: String,
    title: String,
    verdict: &'static str,
}

/// Collapse the per-case×format outcomes into one row per ECC case, preserving
/// first-seen order and merging the format verdicts into a single REST result
/// (FAIL if any format failed, ERROR if any errored, PASS if any passed, else
/// skipped).
fn collapse_cases(results: &RunResults, catalog: &Catalog) -> Vec<DetailRow> {
    // Resolve area title through the catalogue (outcomes carry the ECC id).
    let area_of = |ecc_id: &str| -> &'static str {
        catalog
            .entries()
            .iter()
            .find(|e| e.ecc_id == ecc_id)
            .map_or("—", |e| e.area.title())
    };

    let mut order: Vec<String> = Vec::new();
    let mut merged: BTreeMap<String, DetailRow> = BTreeMap::new();
    let mut worst: BTreeMap<String, CaseStatus> = BTreeMap::new();
    for c in &results.cases {
        let key = if c.ecc_id.is_empty() {
            c.id.clone()
        } else {
            c.ecc_id.clone()
        };
        if !merged.contains_key(&key) {
            order.push(key.clone());
            merged.insert(
                key.clone(),
                DetailRow {
                    component: area_of(&c.ecc_id),
                    capability: c.capability.clone(),
                    conformance_point: c.schedule_ref.clone().unwrap_or_else(|| "—".to_owned()),
                    ecc_id: if c.ecc_id.is_empty() {
                        c.id.clone()
                    } else {
                        c.ecc_id.clone()
                    },
                    title: c.title.clone(),
                    verdict: "",
                },
            );
        }
        // Merge statuses: keep the worst-so-far (Failed > Errored > Passed >
        // Skipped for reporting salience).
        let current = worst.get(&key).copied();
        worst.insert(key.clone(), merge_status(current, c.status));
    }

    order
        .into_iter()
        .filter_map(|key| {
            let mut row = merged.remove(&key)?;
            row.verdict = verdict_word(worst.get(&key).copied().unwrap_or(CaseStatus::Skipped));
            Some(row)
        })
        .collect()
}

/// Merge two case statuses into the more salient one for the combined verdict.
fn merge_status(acc: Option<CaseStatus>, next: CaseStatus) -> CaseStatus {
    let rank = |s: CaseStatus| match s {
        CaseStatus::Failed => 3,
        CaseStatus::Errored => 2,
        CaseStatus::Passed => 1,
        // Skipped / NotApplicable are lowest salience: a passing format wins the
        // merged row. (NotApplicable never reaches the certificate — it is only
        // emitted for `ehrbase-rs` SUTs, which the register never touches — but
        // the match stays exhaustive.)
        CaseStatus::Skipped | CaseStatus::NotApplicable => 0,
    };
    match acc {
        Some(a) if rank(a) >= rank(next) => a,
        _ => next,
    }
}

/// The certificate REST-column word for a merged status.
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
    use crate::results::{CaseOutcome, CorpusPin, ProductIdentity, SelectionInfo, SutIdentity};
    use crate::version::SpecVersions;

    fn outcome(ecc_id: &str, capability: &str, format: &str, status: CaseStatus) -> CaseOutcome {
        CaseOutcome {
            ecc_id: ecc_id.to_owned(),
            id: "slug".to_owned(),
            title: "A case".to_owned(),
            capability: capability.to_owned(),
            profiles: vec![],
            format: format.to_owned(),
            status,
            passed_data_sets: 0,
            total_data_sets: 0,
            message: None,
            citation: String::new(),
            schedule_ref: None,
            duration_ms: 0,
        }
    }

    fn results(cases: Vec<CaseOutcome>) -> RunResults {
        RunResults {
            sut: SutIdentity {
                base_url: "http://sut".to_owned(),
                product: ProductIdentity::default(),
                versions: SpecVersions::latest(),
                auth_mode: "basic".to_owned(),
            },
            corpus: CorpusPin::default(),
            started: "2026-07-10T00:00:00Z".to_owned(),
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
    fn statement_states_rm_version_and_profile_claims() {
        let s = render_statement_md(&results(vec![outcome(
            "ECC-EHR-001",
            "EhrOperations",
            "json",
            CaseStatus::Passed,
        )]));
        assert!(s.contains("Supported specification versions"));
        assert!(s.contains("Reference Model (RM)"));
        assert!(s.contains("1.2.0"), "the RM version must be stated");
        assert!(s.contains("Profile claims"));
        // CORE is not claimable on a single-capability run (all-or-nothing).
        assert!(s.contains("not claimable"));
    }

    #[test]
    fn certificate_has_profile_report_and_detailed_rows() {
        let cat = Catalog::default();
        let cert = render_certificate_md(
            &results(vec![
                outcome("ECC-EHR-001", "EhrOperations", "json", CaseStatus::Passed),
                outcome("ECC-EHR-001", "EhrOperations", "xml", CaseStatus::Failed),
            ]),
            &cat,
        );
        assert!(cert.contains("System Under Test"));
        assert!(cert.contains("Profile Report"));
        assert!(cert.contains("Detailed Test Report"));
        assert!(cert.contains("Conformance point"));
        // The two formats collapse to one row; the worse (FAIL) wins.
        assert!(cert.contains("ECC-EHR-001 — A case"));
        assert!(cert.contains("**FAIL**"));
    }

    #[test]
    fn certificate_solution_and_vendor_are_data_driven_not_hard_coded() {
        // The certificate no longer hard-codes `ehrbase-rs` as Solution/Vendor
        // (§3a.1); it reads the recorded product identity. (In practice the
        // certificate is only *emitted* for an ehrbase-rs SUT — the suppression
        // lives in `report::write_all` — but the render must be honest either
        // way.)
        let cat = Catalog::default();
        let mut r = results(vec![outcome(
            "ECC-EHR-001",
            "EhrOperations",
            "json",
            CaseStatus::Passed,
        )]);
        r.sut.product = ProductIdentity {
            name: "ehrbase-java".to_owned(),
            version: "2.34.0".to_owned(),
            image_digest: Some("sha256:deadbeef".to_owned()),
        };
        let cert = render_certificate_md(&r, &cat);
        assert!(
            cert.contains("ehrbase-java 2.34.0"),
            "Solution reads the product identity"
        );
        assert!(
            cert.contains("| Vendor | ehrbase-java (self-assessed) |"),
            "Vendor reads the product name, not a hard-coded ehrbase-rs"
        );
        assert!(cert.contains("sha256:deadbeef"), "image digest is shown");
    }
}
