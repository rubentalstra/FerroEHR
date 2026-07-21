//! Deterministic Markdown renderers for the three published submission
//! documents: the conformance report, the conformance statement (`SDoC`), and
//! the certificate.
//!
//! Every renderer is a pure function of its inputs, emits byte-deterministic
//! Markdown (stable ordering, no timestamps — the caller stamps a date if it
//! needs one), and ends with a trailing newline. The certificate's table
//! shape follows the CNF certificate book
//! (`CNF/docs/certificate/master03-certificate.adoc`).

use std::collections::BTreeMap;
use std::fmt::Write;

use crate::model::capability::CapabilityMatrix;
use crate::party::{OutcomeStatus, Results, Statement};
use crate::verdict::{Evidence, ProfileVerdict, SecBasicVerdict, VerdictReport};
use crate::vocab::{Family, FormatName, Tier};

/// Render the conformance report (`CONFORMANCE_REPORT.md`): the outcome
/// summary, a per-chapter table, and the honesty block (coverage bound +
/// every not-executed verdict's citation).
#[must_use]
pub fn render_report(results: &Results, verdicts: &VerdictReport) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "# Conformance Report\n");
    let _ = writeln!(
        out,
        "SUT: {} {} · schedule {} · ITS {}",
        results.sut.name,
        results.sut.version,
        results.schedule_release,
        its_token(results),
    );
    let _ = writeln!(
        out,
        "Runner: {} {} · verification pack: {}\n",
        results.runner.name,
        results.runner.version,
        verification_token(results),
    );

    // ── summary counts ──────────────────────────────────────────────────────
    let mut counts: BTreeMap<&'static str, usize> = BTreeMap::new();
    for outcome in &results.outcomes {
        *counts.entry(outcome.status.token()).or_default() += 1;
    }
    let _ = writeln!(out, "## Summary\n");
    let _ = writeln!(out, "| Status | Count |");
    let _ = writeln!(out, "| --- | --- |");
    for status in OutcomeStatus::ALL {
        let _ = writeln!(
            out,
            "| {} | {} |",
            status.token(),
            counts.get(status.token()).copied().unwrap_or(0)
        );
    }
    let _ = writeln!(out, "| total | {} |\n", results.outcomes.len());

    // ── per-chapter table ───────────────────────────────────────────────────
    let mut by_chapter: BTreeMap<String, BTreeMap<&'static str, usize>> = BTreeMap::new();
    for outcome in &results.outcomes {
        let chapter = chapter_of(outcome.case.as_str());
        *by_chapter
            .entry(chapter)
            .or_default()
            .entry(outcome.status.token())
            .or_default() += 1;
    }
    let _ = writeln!(out, "## By chapter\n");
    let _ = writeln!(
        out,
        "| Chapter | passed | failed | errored | skipped | not_applicable |"
    );
    let _ = writeln!(out, "| --- | --- | --- | --- | --- | --- |");
    for (chapter, c) in &by_chapter {
        let g = |s: OutcomeStatus| c.get(s.token()).copied().unwrap_or(0);
        let _ = writeln!(
            out,
            "| {} | {} | {} | {} | {} | {} |",
            chapter,
            g(OutcomeStatus::Passed),
            g(OutcomeStatus::Failed),
            g(OutcomeStatus::Errored),
            g(OutcomeStatus::Skipped),
            g(OutcomeStatus::NotApplicable),
        );
    }
    let _ = writeln!(out);

    // ── honesty block ───────────────────────────────────────────────────────
    let _ = writeln!(out, "## Honesty\n");
    let _ = writeln!(
        out,
        "Coverage: {} of {} selected cases driven.\n",
        verdicts.coverage.driven, verdicts.coverage.selected
    );
    let mut citations: Vec<(&str, &str)> = results
        .outcomes
        .iter()
        .filter(|o| o.status.needs_citation())
        .map(|o| {
            (
                o.case.as_str(),
                o.citation.as_deref().unwrap_or("(missing citation)"),
            )
        })
        .collect();
    citations.sort_unstable();
    if citations.is_empty() {
        let _ = writeln!(out, "No skipped or not-applicable verdicts.");
    } else {
        let _ = writeln!(out, "Not-executed verdicts (each cited):\n");
        let _ = writeln!(out, "| Case | Citation |");
        let _ = writeln!(out, "| --- | --- |");
        for (case, citation) in citations {
            let _ = writeln!(out, "| {case} | {citation} |");
        }
    }

    out
}

/// Render the conformance statement (`CONFORMANCE_STATEMENT.md`): the `SDoC`
/// text, the claims, the computed verdicts, and the attestation.
#[must_use]
pub fn render_statement(statement: &Statement, verdicts: &VerdictReport) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "# Conformance Statement (SDoC)\n");
    let _ = writeln!(
        out,
        "Product: {} {} — {} ({})",
        statement.product.name,
        statement.product.version,
        statement.product.vendor,
        statement.product.identifier,
    );
    let _ = writeln!(out, "Schedule release: {}\n", statement.schedule_release);

    let _ = writeln!(out, "## Declared spec versions\n");
    for (component, version) in declared_versions(statement) {
        let _ = writeln!(out, "- {component}: {version}");
    }
    let _ = writeln!(out);

    let _ = writeln!(out, "## Claims\n");
    let _ = writeln!(
        out,
        "Profiles claimed: {}\n",
        join_tiers(&statement.claims.profiles)
    );
    let _ = writeln!(out, "Capabilities claimed:\n");
    for cap in &statement.claims.capabilities {
        let _ = writeln!(out, "- {cap}");
    }
    let _ = writeln!(out);

    if !statement.options.is_empty() {
        let _ = writeln!(out, "Options declared: {}\n", join_options(statement));
    }

    // ── the computed verdicts ───────────────────────────────────────────────
    let _ = writeln!(out, "## Verdicts\n");
    let _ = writeln!(out, "| Profile | Verdict |");
    let _ = writeln!(out, "| --- | --- |");
    for (tier, verdict) in &verdicts.profiles {
        let _ = writeln!(
            out,
            "| {} | {} |",
            tier_token(*tier),
            profile_token(*verdict)
        );
    }
    if let Some(security) = verdicts.security {
        let _ = writeln!(out, "| SEC-BASIC | {} |", sec_token(security));
    }
    let _ = writeln!(out);
    if !verdicts.review.is_empty() {
        let _ = writeln!(out, "### Static-review findings\n");
        for finding in &verdicts.review {
            let _ = writeln!(out, "- {}", finding.message);
        }
        let _ = writeln!(out);
    }

    // ── attestation ─────────────────────────────────────────────────────────
    let _ = writeln!(out, "## Attestation\n");
    match &statement.attestation {
        Some(a) => {
            let _ = writeln!(out, "{}\n", a.statement);
            let _ = writeln!(out, "Signed: {} ({}) — {}", a.signatory, a.role, a.date);
        }
        None => {
            let _ = writeln!(out, "No attestation supplied.");
        }
    }

    out
}

/// Render the certificate (`CONFORMANCE_CERTIFICATE.md`), modeled on the CNF
/// certificate book's SUT / Scope / Profile-Report shape
/// (`CNF/docs/certificate/master03-certificate.adoc`).
///
/// # Judgment call
/// The book's Profile Report keys each capability by "Required in profile"
/// (Y/OPT/N), which is a capability-matrix property absent from the verdict
/// report, so the matrix is a parameter here (the sibling renderers do not
/// need it). A single [`Results`] covers one technology profile, so the
/// result column is that profile's ITS.
#[must_use]
pub fn render_certificate(
    statement: &Statement,
    results: &Results,
    verdicts: &VerdictReport,
    matrix: &CapabilityMatrix,
) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "# Conformance Certificate\n");

    // ── SUT ─────────────────────────────────────────────────────────────────
    let _ = writeln!(out, "## System Under Test\n");
    let _ = writeln!(out, "| Field | Value |");
    let _ = writeln!(out, "| --- | --- |");
    let _ = writeln!(
        out,
        "| Solution | {} {} |",
        results.sut.name, results.sut.version
    );
    let _ = writeln!(out, "| Vendor | {} |", statement.product.vendor);
    let _ = writeln!(
        out,
        "| Runner | {} {} |",
        results.runner.name, results.runner.version
    );
    let _ = writeln!(
        out,
        "| Infrastructure | {} |",
        statement
            .performance
            .as_ref()
            .map_or("—", |p| p.environment_ref.as_str())
    );

    // ── scope ───────────────────────────────────────────────────────────────
    let _ = writeln!(out, "\n## Scope of Test\n");
    let _ = writeln!(out, "| Dimension | Value |");
    let _ = writeln!(out, "| --- | --- |");
    let _ = writeln!(
        out,
        "| Functional | {} |",
        join_tiers(&statement.claims.profiles)
    );
    let _ = writeln!(
        out,
        "| Sec & Priv | {} |",
        verdicts
            .security
            .map_or_else(|| "—".to_owned(), |v| format!("SEC-BASIC {}", sec_token(v)))
    );
    let _ = writeln!(
        out,
        "| Ext Data Fmt | {} |",
        join_formats(&results.tech_profile.formats)
    );

    // ── profile report ──────────────────────────────────────────────────────
    let _ = writeln!(out, "\n## Profile Report\n");
    let _ = writeln!(
        out,
        "Result column: ITS {} ({})",
        its_token(results),
        join_formats(&results.tech_profile.formats)
    );
    let _ = writeln!(
        out,
        "\n| Family | Capability | Required in profile | Result |"
    );
    let _ = writeln!(out, "| --- | --- | --- | --- |");
    for (name, entry) in matrix.entries() {
        let evidence = verdicts
            .capabilities
            .iter()
            .find(|(n, _)| n == name)
            .map_or(Evidence::NoCases, |(_, e)| *e);
        let _ = writeln!(
            out,
            "| {} | {} | {} | {} |",
            family_token(entry.family),
            name,
            required_token(entry.required),
            evidence_token(evidence),
        );
    }
    let _ = writeln!(out);

    out
}

// ── shared formatting helpers ────────────────────────────────────────────────

/// The chapter grouping for a case id: the SM interface prefix for a
/// `I_X.op-variant` id, else the leading family token before the first `-`.
fn chapter_of(case_id: &str) -> String {
    if let Some((interface, _)) = case_id.split_once('.') {
        return interface.to_owned();
    }
    case_id
        .split_once('-')
        .map_or_else(|| case_id.to_owned(), |(head, _)| head.to_owned())
}

fn declared_versions(statement: &Statement) -> Vec<(&'static str, &str)> {
    use crate::vocab::SpecComponent;
    let mut out = Vec::new();
    for component in SpecComponent::ALL {
        if let Some(version) = statement.spec_versions.get(*component) {
            out.push((spec_component_token(*component), version));
        }
    }
    out
}

fn spec_component_token(component: crate::vocab::SpecComponent) -> &'static str {
    use crate::vocab::SpecComponent;
    match component {
        SpecComponent::Rm => "rm",
        SpecComponent::Base => "base",
        SpecComponent::Am => "am",
        SpecComponent::Aql => "aql",
        SpecComponent::ItsRest => "its_rest",
        SpecComponent::Term => "term",
    }
}

fn its_token(results: &Results) -> &'static str {
    match results.tech_profile.its {
        crate::vocab::ItsName::ItsRest => "its-rest",
    }
}

fn verification_token(results: &Results) -> &'static str {
    use crate::party::VerificationPackStatus;
    match results.runner.verification_pack_status {
        VerificationPackStatus::Passed => "passed",
        VerificationPackStatus::NotRun => "not_run",
        VerificationPackStatus::Failed => "failed",
    }
}

fn tier_token(tier: Tier) -> &'static str {
    match tier {
        Tier::Core => "CORE",
        Tier::Standard => "STANDARD",
        Tier::Options => "OPTIONS",
        Tier::SecBasic => "SEC-BASIC",
        Tier::EnterpriseD => "D",
        Tier::EnterpriseM => "M",
        Tier::EnterpriseX => "X",
    }
}

fn join_tiers(tiers: &[Tier]) -> String {
    if tiers.is_empty() {
        return "—".to_owned();
    }
    tiers
        .iter()
        .map(|t| tier_token(*t))
        .collect::<Vec<_>>()
        .join(", ")
}

fn join_options(statement: &Statement) -> String {
    statement
        .options
        .iter()
        .map(crate::ids::OptionTag::as_str)
        .collect::<Vec<_>>()
        .join(", ")
}

fn format_token(format: FormatName) -> &'static str {
    match format {
        FormatName::CanonicalJson => "canonical-json",
        FormatName::CanonicalXml => "canonical-xml",
        FormatName::WtFlat => "wt-flat",
        FormatName::WtStructured => "wt-structured",
        FormatName::Wt => "wt",
    }
}

fn join_formats(formats: &[FormatName]) -> String {
    if formats.is_empty() {
        return "—".to_owned();
    }
    formats
        .iter()
        .map(|f| format_token(*f))
        .collect::<Vec<_>>()
        .join(", ")
}

fn family_token(family: Family) -> &'static str {
    match family {
        Family::Platform => "Platform",
        Family::Enterprise => "Enterprise",
        Family::Security => "Security",
    }
}

/// The certificate's Required-in-profile column: required ⇒ Y, optional ⇒ OPT.
fn required_token(required: bool) -> &'static str {
    if required { "Y" } else { "OPT" }
}

fn evidence_token(evidence: Evidence) -> &'static str {
    match evidence {
        Evidence::Passed => "pass",
        Evidence::Failed => "FAIL",
        Evidence::NotEvidenced => "not evidenced",
        Evidence::NoCases => "no cases",
    }
}

fn profile_token(verdict: ProfileVerdict) -> &'static str {
    match verdict {
        ProfileVerdict::Pass => "PASS",
        ProfileVerdict::Fail => "FAIL",
        ProfileVerdict::NotClaimed => "not claimed",
    }
}

fn sec_token(verdict: SecBasicVerdict) -> &'static str {
    match verdict {
        SecBasicVerdict::Pass => "PASS",
        SecBasicVerdict::Fail => "FAIL",
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic)] // test assertions/fixtures
mod tests {
    use super::*;

    fn matrix() -> CapabilityMatrix {
        serde_json::from_value(serde_json::json!({
            "EhrOperations": { "family": "Platform", "tier": "CORE", "required": true },
            "SimplifiedFormats": { "family": "Platform", "tier": "OPTIONS", "required": false }
        }))
        .unwrap()
    }

    fn statement() -> Statement {
        serde_json::from_value(serde_json::json!({
            "product": { "name": "EHRbase-rs", "version": "3.5.0",
                          "vendor": "openHospi", "identifier": "urn:x" },
            "schedule_release": "CNF-2.0",
            "spec_versions": { "rm": "1.2.0", "its_rest": "1.1.0" },
            "claims": { "capabilities": ["EhrOperations"], "profiles": ["CORE"] },
            "tech_profiles": [ { "its": "its-rest", "formats": ["canonical-json"] } ],
            "attestation": { "signatory": "A", "role": "CTO", "date": "2026-07-21",
                              "statement": "We declare conformance." }
        }))
        .unwrap()
    }

    fn results() -> Results {
        serde_json::from_value(serde_json::json!({
            "sut": { "name": "ehrbase-rs", "version": "3.5.0" },
            "runner": { "name": "cnf-runner", "version": "0.1.0",
                         "verification_pack_status": "passed" },
            "schedule_release": "CNF-2.0",
            "tech_profile": { "its": "its-rest", "formats": ["canonical-json"] },
            "ixit_digest": "d",
            "outcomes": [
                { "case": "I_EHR_SERVICE.create_ehr-main", "format": "canonical-json",
                  "status": "passed", "rows_driven": 1, "rows_total": 1 },
                { "case": "I_ADMIN_SERVICE.list-x", "status": "not_applicable",
                  "rows_driven": 0, "rows_total": 1, "citation": "AMB-33" }
            ]
        }))
        .unwrap()
    }

    fn verdicts() -> VerdictReport {
        let cases = vec![
            serde_json::from_value::<CaseCore>(serde_json::json!({
                "id": "I_EHR_SERVICE.create_ehr-main", "kind": "functional", "component": "EHR",
                "sm_operation": "I_EHR_SERVICE.create_ehr",
                "capabilities": ["EhrOperations"], "profiles": ["CORE"],
                "test_purpose": "t", "description": "d",
                "spec_refs": ["CNF platform_test_schedule master06 §x"],
                "flow": [ { "step": 1, "call": "create_ehr", "expect": "created" } ]
            }))
            .unwrap(),
        ];
        let register: crate::model::register::AmbiguityRegister =
            serde_json::from_value(serde_json::json!({
                "AMB-1": { "ambiguity": "a", "source": "s", "handling": "h",
                            "disposition": "loose_assert" }
            }))
            .unwrap();
        crate::verdict::compute(&statement(), &results(), &cases, &matrix(), &register)
    }

    use crate::model::case::CaseCore;

    #[test]
    fn report_is_deterministic_and_lists_citations() {
        let v = verdicts();
        let a = render_report(&results(), &v);
        let b = render_report(&results(), &v);
        assert_eq!(a, b);
        assert!(a.ends_with('\n'));
        assert!(a.contains("Coverage: 1 of 1"));
        assert!(a.contains("AMB-33"));
    }

    #[test]
    fn statement_renders_verdicts_and_attestation() {
        let text = render_statement(&statement(), &verdicts());
        assert!(text.ends_with('\n'));
        assert!(text.contains("We declare conformance."));
        assert!(text.contains("CORE"));
    }

    #[test]
    fn certificate_has_profile_report_columns() {
        let text = render_certificate(&statement(), &results(), &verdicts(), &matrix());
        assert!(text.ends_with('\n'));
        assert!(text.contains("Required in profile"));
        assert!(text.contains("| Platform | EhrOperations | Y | pass |"));
        assert!(text.contains("| Platform | SimplifiedFormats | OPT |"));
    }
}
