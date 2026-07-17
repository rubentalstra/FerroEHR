//! The Conformance **Statement** (`CONFORMANCE_STATEMENT.md`) — emitted for
//! every SUT (the framework certifies any CDR; the target design §6 mandates a
//! Statement per SUT, not only for our own product).
//!
//! The Statement declares the SUT's identity, the specification versions it
//! asserts against — including the **aggregated edition findings** the run
//! discovered, which the CNF schedule requires be stated
//! (`platform_test_schedule/master03-overview.adoc` §46: *"The supported RM
//! version(s) by the SUT should be stated in the Conformance Statement …
//! minimum required version is RM 1.0.2"*) — the external data formats, the
//! capability scope with its profile requirement + machine result, the
//! machine-computed profile claims, and the adjudicated skips / not-applicable
//! listing with citations. Every value is a pure function of the run
//! ([`crate::reporting::report`] hosts the shared claim math).

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

use crate::model::case::{Capability, Profile};
use crate::model::profile::{OPTIONAL_CAPABILITIES, required_capabilities};
use crate::reporting::report::{
    ALL_CAPABILITIES, capability_verdict, claim_word, edition_policy_label, evidence_word,
    kind_label, profile_verdict,
};
use crate::reporting::results::{CaseStatus, RunResults};

/// The three profiles, in report order.
const PROFILES: [Profile; 3] = [Profile::Core, Profile::Standard, Profile::Options];

/// Render the Conformance Statement.
#[must_use]
pub fn render_statement_md(results: &RunResults) -> String {
    let mut out = String::new();
    let _ = writeln!(
        out,
        "# {} — Conformance Statement (generated)\n",
        results.sut.product.name
    );
    out.push_str(
        "> Generated from a conformance run (`results.json`) — never hand-asserted. Every\n\
         > claim below is a pure function of the machine profile verdicts.\n\n",
    );

    render_sut(&mut out, results);
    render_versions(&mut out, results);
    render_formats(&mut out, results);
    render_capability_scope(&mut out, results);
    render_profile_claims(&mut out, results);
    render_adjudications(&mut out, results);
    render_selection(&mut out, results);
    out
}

fn render_sut(out: &mut String, results: &RunResults) {
    let s = &results.sut;
    out.push_str("## System under test\n\n| Field | Value |\n|---|---|\n");
    let _ = writeln!(
        out,
        "| Product | {} {} |",
        s.product.name, s.product.version
    );
    if let Some(digest) = &s.product.image_digest {
        let _ = writeln!(out, "| Image digest | `{digest}` |");
    }
    let _ = writeln!(out, "| SUT class | {} |", kind_label(s.kind));
    let _ = writeln!(out, "| Base URL | `{}` |", s.base_url);
    let _ = writeln!(out, "| Auth mode | {} |", s.auth_mode);
    let _ = writeln!(
        out,
        "| Edition policy | {} |",
        edition_policy_label(s.edition_policy)
    );
    let _ = writeln!(out, "| Run started | {} |", results.started);
    let _ = writeln!(
        out,
        "| Reference corpus | {}@{} |\n",
        results.corpus.repo, results.corpus.commit
    );
}

fn render_versions(out: &mut String, results: &RunResults) {
    let v = &results.sut.versions;
    out.push_str("## Supported specification versions\n\n");
    out.push_str("| Specification | Version |\n|---|---|\n");
    let _ = writeln!(out, "| Reference Model (RM) | {} |", v.rm);
    let _ = writeln!(out, "| ITS-REST contract | {} |", v.its_rest);
    let _ = writeln!(out, "| AQL (QUERY) | {} |", v.aql);
    let _ = writeln!(out, "| Terminology (TERM) | {} |\n", v.term);
    let _ = writeln!(
        out,
        "> CNF requires the Conformance Statement to state the supported RM version(s); \
         the minimum required is RM 1.0.2 (`master03-overview.adoc` §46). This SUT declares \
         **RM {}**.\n",
        v.rm
    );

    // The aggregated edition findings — the edition/RM rung the SUT actually
    // satisfied where a laddered assertion stepped below the newest form.
    out.push_str("\n### Discovered edition profile\n\n");
    let mut by_rung: BTreeMap<String, usize> = BTreeMap::new();
    let mut observations: BTreeSet<String> = BTreeSet::new();
    for c in &results.cases {
        if let Some(level) = &c.edition_level {
            *by_rung.entry(level.clone()).or_insert(0) += 1;
        }
        for f in &c.edition_findings {
            observations.insert(f.clone());
        }
    }
    if by_rung.is_empty() {
        out.push_str(
            "Every laddered assertion matched the newest edition form — no lower-rung findings.\n",
        );
    } else {
        out.push_str(
            "The SUT satisfied the normative core of some assertions only at a rung below the \
             newest edition:\n\n",
        );
        out.push_str("| Satisfied rung | Cases |\n|---|--:|\n");
        for (rung, count) in &by_rung {
            let _ = writeln!(out, "| {rung} | {count} |");
        }
        if !observations.is_empty() {
            out.push_str("\nObservations:\n\n");
            for obs in &observations {
                let _ = writeln!(out, "- {obs}");
            }
        }
    }
    out.push('\n');
}

fn render_formats(out: &mut String, results: &RunResults) {
    out.push_str("## External data formats\n\n");
    let _ = writeln!(
        out,
        "Declared: XML, JSON (`master03-profiles.adoc` §Other Non-Functional). This run \
         exercised: {}.\n",
        if results.selection.formats.is_empty() {
            "—".to_owned()
        } else {
            results.selection.formats.join(", ")
        }
    );
}

fn render_capability_scope(out: &mut String, results: &RunResults) {
    out.push_str("\n## Capability scope\n\n");
    out.push_str("| Capability | Required in | Result |\n|---|---|---|\n");
    for cap in ALL_CAPABILITIES {
        let v = capability_verdict(results, cap);
        // Skip capabilities with no cases AND no profile role — pure noise.
        let requirement = profile_requirement(cap);
        if v.passed == 0
            && v.failed == 0
            && v.skipped == 0
            && v.not_applicable == 0
            && requirement == "—"
        {
            continue;
        }
        let _ = writeln!(
            out,
            "| {cap:?} | {requirement} | {} |",
            evidence_word(v.evidence)
        );
    }
    out.push('\n');
}

/// Which profile requires a capability: CORE/STANDARD (required), OPTIONS
/// (optional), or non-gating (`Authentication`/`Terminology` — reported, never
/// profile-gating per `master03-profiles.adoc`).
fn profile_requirement(cap: Capability) -> &'static str {
    if required_capabilities(Profile::Core).contains(&cap) {
        "CORE (required)"
    } else if required_capabilities(Profile::Standard).contains(&cap) {
        "STANDARD (required)"
    } else if OPTIONAL_CAPABILITIES.contains(&cap) {
        "OPTIONS (optional)"
    } else {
        // Authentication + Terminology: reported, never gating.
        "reported (non-gating)"
    }
}

fn render_profile_claims(out: &mut String, results: &RunResults) {
    out.push_str("## Profile claims (machine-computed)\n\n");
    out.push_str("| Profile | Aggregation | Result |\n|---|---|---|\n");
    for profile in PROFILES {
        let verdict = profile_verdict(results, profile);
        let _ = writeln!(
            out,
            "| {profile:?} | {} | {} |",
            aggregation(profile),
            claim_word(profile, verdict),
        );
    }

    // The obtained optional capabilities — the substance of an OPTIONS claim.
    out.push_str("\n### OPTIONS — obtained optional capabilities\n\n");
    let obtained: Vec<Capability> = OPTIONAL_CAPABILITIES
        .iter()
        .copied()
        .filter(|&c| {
            capability_verdict(results, c).evidence
                == crate::model::profile::CapabilityEvidence::Passed
        })
        .collect();
    if obtained.is_empty() {
        out.push_str("_None obtained in this run._\n");
    } else {
        for cap in obtained {
            let _ = writeln!(out, "- {cap:?}");
        }
    }
    out.push('\n');
}

fn aggregation(profile: Profile) -> &'static str {
    match profile {
        Profile::Core | Profile::Standard => "all listed capabilities",
        Profile::Options => "any optional capability",
    }
}

fn render_adjudications(out: &mut String, results: &RunResults) {
    out.push_str("## Adjudicated skips and not-applicable cases\n\n");
    let mut any = false;

    // Not-applicable (foreign-SUT fairness register), collapsed per ECC id.
    let mut na_order: Vec<String> = Vec::new();
    let mut na_seen: BTreeMap<String, (&str, &str, &str)> = BTreeMap::new();
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
        na_seen.entry(key.clone()).or_insert_with(|| {
            na_order.push(key.clone());
            (
                c.ecc_id.as_str(),
                c.message.as_deref().unwrap_or("(no reason)"),
                if c.citation.is_empty() {
                    "—"
                } else {
                    c.citation.as_str()
                },
            )
        });
    }
    if !na_order.is_empty() {
        any = true;
        out.push_str("### Not applicable (fairness register)\n\n");
        for key in &na_order {
            let (ecc, reason, citation) = na_seen[key];
            let _ = writeln!(out, "- **{ecc}** — {reason} _(cite: {citation})_");
        }
        out.push('\n');
    }

    // Adjudicated skips (own-corpus register + structural skips).
    let skips: Vec<_> = results
        .cases
        .iter()
        .filter(|c| c.status == CaseStatus::Skipped)
        .collect();
    if !skips.is_empty() {
        any = true;
        out.push_str("### Skipped (adjudicated / structural), by reason\n\n");
        let mut reasons: BTreeMap<&str, usize> = BTreeMap::new();
        for c in skips {
            *reasons
                .entry(c.message.as_deref().unwrap_or("(unstated)"))
                .or_insert(0) += 1;
        }
        out.push_str("| Reason | Cases |\n|---|--:|\n");
        for (reason, count) in &reasons {
            let _ = writeln!(out, "| {reason} | {count} |");
        }
        out.push('\n');
    }

    if !any {
        out.push_str("_No adjudicated skips or not-applicable cases in this run._\n\n");
    }
}

fn render_selection(out: &mut String, results: &RunResults) {
    out.push_str("## Selection\n\n| Field | Value |\n|---|---|\n");
    let _ = writeln!(
        out,
        "| Profile filter | {} |",
        results.selection.profile.as_deref().unwrap_or("all")
    );
    let _ = writeln!(
        out,
        "| Id filter | {} |",
        results.selection.filter.as_deref().unwrap_or("—")
    );
    let _ = writeln!(
        out,
        "| Formats | {} |\n",
        if results.selection.formats.is_empty() {
            "—".to_owned()
        } else {
            results.selection.formats.join(", ")
        }
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
    use crate::edition::{Edition, EditionPolicy};
    use crate::model::versions::SpecVersions;
    use crate::reporting::results::{
        CaseOutcome, CorpusPin, ProductIdentity, SelectionInfo, SutIdentity,
    };
    use crate::sut::descriptor::SutKind;

    fn outcome(ecc_id: &str, capability: &str, status: CaseStatus) -> CaseOutcome {
        CaseOutcome {
            ecc_id: ecc_id.to_owned(),
            id: "slug".to_owned(),
            title: "A case".to_owned(),
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

    fn results(cases: Vec<CaseOutcome>) -> RunResults {
        RunResults {
            sut: SutIdentity {
                base_url: "http://sut".to_owned(),
                product: ProductIdentity::default(),
                kind: SutKind::Ours,
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
    fn statement_states_rm_version_and_profile_claims() {
        let s = render_statement_md(&results(vec![outcome(
            "ECC-EHR-001",
            "EhrOperations",
            CaseStatus::Passed,
        )]));
        assert!(s.contains("Supported specification versions"));
        assert!(s.contains("RM 1.2.0") || s.contains("| Reference Model (RM) | 1.2.0 |"));
        assert!(s.contains("Profile claims"));
        // CORE is all-of, so one passing capability is not claimable.
        assert!(s.contains("not claimable"));
    }

    #[test]
    fn statement_titled_by_product_and_reports_edition_findings() {
        let mut r = results(vec![outcome(
            "ECC-COM-001",
            "CompositionOps",
            CaseStatus::Passed,
        )]);
        r.sut.product = ProductIdentity {
            name: "ehrbase-java".to_owned(),
            version: "2.34.0".to_owned(),
            image_digest: None,
        };
        r.sut.kind = SutKind::Foreign;
        r.cases[0].edition_level = Some("release-1.0.3".to_owned());
        r.cases[0].edition_findings = vec!["release-1.0.3: ETag emitted in bare form".to_owned()];
        let s = render_statement_md(&r);
        assert!(s.starts_with("# ehrbase-java — Conformance Statement"));
        assert!(s.contains("Discovered edition profile"));
        assert!(s.contains("release-1.0.3"));
        assert!(s.contains("bare form"));
    }
}
