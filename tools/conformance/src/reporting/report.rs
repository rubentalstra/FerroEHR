//! Per-SUT `CONFORMANCE_REPORT.md` (+ the `results.json` machine record and
//! the per-run artifact set) rendered from a [`RunResults`].
//!
//! Everything is a pure function of the run: identities and spec versions
//! come from the recorded [`SutIdentity`], the capability/profile verdicts are
//! machine-computed ([`crate::model::profile`]), and every coverage bound
//! (data-set truncation, ECC-original stubs, edition findings) is printed —
//! a coverage bound is always logged, never silent.
//!
//! This module also hosts the shared capability/profile accounting
//! ([`capability_verdict`], [`profile_lines`], [`profile_verdict`],
//! [`ALL_CAPABILITIES`]) the Statement, Certificate, badge, and comparison
//! renderers reuse, so the claim math has exactly one implementation.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::Path;

use crate::edition::EditionPolicy;
use crate::model::case::{Capability, Profile};
use crate::model::catalog::{Area, Catalog, EccStatus};
use crate::model::profile::{
    CapabilityEvidence, CapabilityVerdict, OPTIONAL_CAPABILITIES, ProfileVerdict, all_of_verdict,
    any_of_verdict, required_capabilities,
};
use crate::reporting::results::{CaseOutcome, CaseStatus, RunResults};
use crate::sut::descriptor::SutKind;

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

/// Every [`Capability`] variant, in report order. There is no `ALL` const on
/// the enum (it lives in the generated-adjacent `model::case`), so the full
/// list is maintained here for the capability matrix; a capability with no
/// cases renders as a logged coverage bound (`NoCases`), never silently
/// dropped.
pub const ALL_CAPABILITIES: [Capability; 29] = [
    Capability::Adl14ArchetypeProvisioning,
    Capability::Adl14OptProvisioning,
    Capability::Adl2Provisioning,
    Capability::EhrOperations,
    Capability::EhrStatus,
    Capability::CompositionOps,
    Capability::ChangeSets,
    Capability::Versioning,
    Capability::ArchetypeValidation,
    Capability::DirectoryOps,
    Capability::QueryProvisioning,
    Capability::AqlBasic,
    Capability::AqlAdvanced,
    Capability::AqlTerminology,
    Capability::PartyOperations,
    Capability::PartyRelationshipOperations,
    Capability::AdminActivityReport,
    Capability::AdminPhysicalDeletion,
    Capability::AdminEhrDumpLoad,
    Capability::AdminBulkEhrLoad,
    Capability::AdminEhrArchive,
    Capability::AdminDemographicArchive,
    Capability::MessagingEhrExtract,
    Capability::MessagingTds,
    Capability::Signing,
    Capability::AnonymousEhrs,
    Capability::Authentication,
    Capability::Terminology,
    Capability::SimplifiedFormats,
];

// ── Shared capability / profile accounting ──────────────────────────────────

/// The per-status case tally for one capability.
#[derive(Debug, Clone, Copy, Default)]
pub struct CapCount {
    /// Cases passed.
    pub passed: u32,
    /// Cases failed (conformance finding).
    pub failed: u32,
    /// Cases errored (runner/SUT transport fault — not a conformance finding,
    /// but never counts as passed for a machine verdict).
    pub errored: u32,
    /// Cases skipped for a stated reason.
    pub skipped: u32,
    /// Cases adjudicated not-applicable (foreign-SUT fairness register).
    pub not_applicable: u32,
}

impl CapCount {
    /// The number of case×format outcomes that touched this capability.
    #[must_use]
    pub fn total(self) -> u32 {
        self.passed + self.failed + self.errored + self.skipped + self.not_applicable
    }
}

/// Tally every recorded outcome for `capability` (matched on the outcome's
/// `capability` string, which is the `Debug` form of the enum — the exact
/// value the executor records).
#[must_use]
pub fn capability_count(results: &RunResults, capability: Capability) -> CapCount {
    let name = format!("{capability:?}");
    let mut cc = CapCount::default();
    for c in results.cases.iter().filter(|c| c.capability == name) {
        match c.status {
            CaseStatus::Passed => cc.passed += 1,
            CaseStatus::Failed => cc.failed += 1,
            CaseStatus::Errored => cc.errored += 1,
            CaseStatus::Skipped => cc.skipped += 1,
            CaseStatus::NotApplicable => cc.not_applicable += 1,
        }
    }
    cc
}

/// The machine verdict line for one capability. A transport error folds into
/// the `failed` bucket for evidence purposes (an errored capability can never
/// be claimed as passed), while the display keeps the counts distinct.
#[must_use]
pub fn capability_verdict(results: &RunResults, capability: Capability) -> CapabilityVerdict {
    let cc = capability_count(results, capability);
    CapabilityVerdict::classify(
        capability,
        cc.passed,
        cc.failed + cc.errored,
        cc.skipped,
        cc.not_applicable,
    )
}

/// The capability lines a profile is scored over: the required set for
/// CORE/STANDARD, the optional set for OPTIONS.
#[must_use]
pub fn profile_lines(results: &RunResults, profile: Profile) -> Vec<CapabilityVerdict> {
    let caps: &[Capability] = match profile {
        Profile::Options => OPTIONAL_CAPABILITIES,
        other => required_capabilities(other),
    };
    caps.iter()
        .map(|&c| capability_verdict(results, c))
        .collect()
}

/// The machine-computed profile verdict: all-of for CORE/STANDARD, any-of for
/// OPTIONS (`profiles/master03-profiles.adoc`).
#[must_use]
pub fn profile_verdict(results: &RunResults, profile: Profile) -> ProfileVerdict {
    let lines = profile_lines(results, profile);
    match profile {
        Profile::Options => any_of_verdict(&lines),
        _ => all_of_verdict(&lines),
    }
}

/// The claim word for a profile verdict (OPTIONS is "obtained"; the gated
/// profiles are "PASS"/"not claimable").
#[must_use]
pub fn claim_word(profile: Profile, verdict: ProfileVerdict) -> &'static str {
    match (profile, verdict) {
        (Profile::Options, ProfileVerdict::Pass) => "OBTAINED",
        (Profile::Options, ProfileVerdict::Fail) => "not obtained",
        (_, ProfileVerdict::Pass) => "PASS",
        (_, ProfileVerdict::Fail) => "not claimable",
    }
}

/// The Markdown cell word for a capability's evidence classification.
#[must_use]
pub fn evidence_word(evidence: CapabilityEvidence) -> &'static str {
    match evidence {
        CapabilityEvidence::Passed => "pass",
        CapabilityEvidence::Failed => "**FAIL**",
        CapabilityEvidence::NotEvidenced => "not evidenced",
        CapabilityEvidence::NoCases => "no cases",
    }
}

/// A human label for the SUT class.
#[must_use]
pub fn kind_label(kind: SutKind) -> &'static str {
    match kind {
        SutKind::Ours => "ours (ehrbase-rs)",
        SutKind::Foreign => "foreign (comparison data)",
    }
}

/// A human label for the edition policy the run executed under.
#[must_use]
pub fn edition_policy_label(policy: EditionPolicy) -> String {
    match policy {
        EditionPolicy::Pinned(e) => format!("pinned ({})", e.label()),
        EditionPolicy::Auto => "auto (ladder: newest form first, step down)".to_owned(),
    }
}

// ── The public artifact set ─────────────────────────────────────────────────

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

/// Write the per-run artifact set into `out_dir`: `results.json`,
/// `CONFORMANCE_REPORT.md`, the Conformance **Statement** (every SUT), the
/// Conformance **Certificate** (every SUT — owner ruling 2026-07-13; it carries
/// an assessment-basis honesty block making clear it is a self-assessment, not
/// an official openEHR certification), and the badge set. `assessor` overrides
/// the default self-assessment attribution on the Certificate.
///
/// # Errors
/// [`ReportError`] on I/O or serialization failure.
pub fn write_all(
    results: &RunResults,
    out_dir: &Path,
    assessor: Option<&str>,
) -> Result<(), ReportError> {
    std::fs::create_dir_all(out_dir).map_err(|source| ReportError::Io {
        path: out_dir.display().to_string(),
        source,
    })?;
    let catalog = Catalog::load_default().unwrap_or_default();

    let json =
        serde_json::to_string_pretty(results).map_err(|e| ReportError::Codec(e.to_string()))?;
    write_file(&out_dir.join("results.json"), &json)?;
    write_file(
        &out_dir.join("CONFORMANCE_REPORT.md"),
        &render_report_md(results, &catalog),
    )?;

    // The Conformance Statement is emitted for EVERY SUT (the framework
    // certifies any CDR): it is the SUT's declared scope + machine claims, not
    // a self-assessment badge. (The earlier instrument suppressed the
    // Statement for non-ehrbase-rs SUTs; a Statement is now emitted per SUT.)
    write_file(
        &out_dir.join("CONFORMANCE_STATEMENT.md"),
        &crate::reporting::statement::render_statement_md(results),
    )?;

    // The Certificate is emitted for every SUT (owner ruling 2026-07-13): the
    // framework aims to be the industry-standard CNF validator, so any operator
    // certifying their own CDR gets the artefact. Its mandatory honesty block
    // states it is a self-assessment, NOT an official openEHR certification.
    write_file(
        &out_dir.join("CONFORMANCE_CERTIFICATE.md"),
        &crate::reporting::certificate::render_certificate_md(results, &catalog, assessor),
    )?;

    crate::reporting::badges::write_badges(results, &catalog, out_dir)?;
    Ok(())
}

fn write_file(path: &Path, content: &str) -> Result<(), ReportError> {
    std::fs::write(path, content).map_err(|source| ReportError::Io {
        path: path.display().to_string(),
        source,
    })
}

// ── CONFORMANCE_REPORT.md ────────────────────────────────────────────────────

/// Render the per-SUT `CONFORMANCE_REPORT.md`.
#[must_use]
pub fn render_report_md(results: &RunResults, catalog: &Catalog) -> String {
    let mut out = String::new();
    out.push_str("# Conformance Report (generated)\n\n");
    out.push_str(
        "> Generated from a conformance run — never hand-asserted. Every claim is a\n\
         > pure function of the recorded outcomes; every coverage bound is printed.\n\n",
    );

    render_identity(&mut out, results);
    render_summary(&mut out, results);
    render_area_matrix(&mut out, results, catalog);
    render_capability_matrix(&mut out, results);
    render_profile_verdicts(&mut out, results);
    render_failures(&mut out, results);
    render_errors(&mut out, results);
    render_skips(&mut out, results);
    render_not_applicable(&mut out, results);
    render_edition_findings(&mut out, results);
    render_coverage_bounds(&mut out, results);
    render_ecc_original(&mut out, results);
    render_detailed(&mut out, results);
    render_terminology(&mut out, results);
    out
}

fn render_identity(out: &mut String, results: &RunResults) {
    let s = &results.sut;
    out.push_str("## 1. System under test\n\n| Field | Value |\n|---|---|\n");
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
    let _ = writeln!(
        out,
        "| Spec versions | RM {} · ITS-REST {} · AQL {} · TERM {} |",
        s.versions.rm, s.versions.its_rest, s.versions.aql, s.versions.term
    );
    let _ = writeln!(
        out,
        "| Reference corpus | {}@{} |",
        results.corpus.repo, results.corpus.commit
    );
    let _ = writeln!(out, "| Run started | {} |\n", results.started);
}

fn render_summary(out: &mut String, results: &RunResults) {
    let errored = status_count(results, CaseStatus::Errored);
    let skipped = status_count(results, CaseStatus::Skipped);
    let _ = write!(
        out,
        "**{} case×format executions · {} passed · {} failed · {} errored · {} skipped · \
         {} not applicable.**\n\n",
        results.executed(),
        results.passed(),
        results.failed(),
        errored,
        skipped,
        results.not_applicable(),
    );
}

fn status_count(results: &RunResults, status: CaseStatus) -> usize {
    results.cases.iter().filter(|c| c.status == status).count()
}

#[derive(Default, Clone, Copy)]
struct AreaTally {
    passed: usize,
    failed: usize,
    errored: usize,
    skipped: usize,
    not_applicable: usize,
}

fn render_area_matrix(out: &mut String, results: &RunResults, catalog: &Catalog) {
    // Resolve each outcome to an area through the catalogue (outcomes carry
    // the ECC id).
    let area_of = |ecc_id: &str| -> Option<Area> {
        catalog
            .entries()
            .iter()
            .find(|e| e.ecc_id == ecc_id)
            .map(|e| e.area)
    };
    let mut by: BTreeMap<Area, AreaTally> = BTreeMap::new();
    for c in &results.cases {
        let Some(area) = area_of(&c.ecc_id) else {
            continue;
        };
        let t = by.entry(area).or_default();
        match c.status {
            CaseStatus::Passed => t.passed += 1,
            CaseStatus::Failed => t.failed += 1,
            CaseStatus::Errored => t.errored += 1,
            CaseStatus::Skipped => t.skipped += 1,
            CaseStatus::NotApplicable => t.not_applicable += 1,
        }
    }
    let mut denominators: BTreeMap<Area, usize> = BTreeMap::new();
    for e in catalog
        .entries()
        .iter()
        .filter(|e| e.status == EccStatus::Active)
    {
        *denominators.entry(e.area).or_insert(0) += 1;
    }

    out.push_str("## 2. Per-area matrix\n\n");
    out.push_str("| Area | Catalogue (active) | Passed | Failed | Errored | Skipped | N/A |\n");
    out.push_str("|---|--:|--:|--:|--:|--:|--:|\n");
    for area in Area::ALL {
        let denom = denominators.get(&area).copied().unwrap_or(0);
        if denom == 0 && !by.contains_key(&area) {
            continue;
        }
        let t = by.get(&area).copied().unwrap_or_default();
        let _ = writeln!(
            out,
            "| {} — {} | {} | {} | {} | {} | {} | {} |",
            area.tag(),
            area.title(),
            denom,
            t.passed,
            t.failed,
            t.errored,
            t.skipped,
            t.not_applicable,
        );
    }
    out.push('\n');
}

fn render_capability_matrix(out: &mut String, results: &RunResults) {
    out.push_str("## 3. Capability matrix\n\n");
    out.push_str(
        "Cases grouped by capability; the evidence classification folds a transport \
         error into `failed` (an errored capability is never claimed as passed).\n\n",
    );
    out.push_str("| Capability | Passed | Failed | Errored | Skipped | N/A | Evidence |\n");
    out.push_str("|---|--:|--:|--:|--:|--:|---|\n");
    for cap in ALL_CAPABILITIES {
        let cc = capability_count(results, cap);
        if cc.total() == 0 {
            continue; // no cases touch this capability — a bound, printed in profile tables
        }
        let v = capability_verdict(results, cap);
        let _ = writeln!(
            out,
            "| {cap:?} | {} | {} | {} | {} | {} | {} |",
            cc.passed,
            cc.failed,
            cc.errored,
            cc.skipped,
            cc.not_applicable,
            evidence_word(v.evidence),
        );
    }
    out.push('\n');
}

fn render_profile_verdicts(out: &mut String, results: &RunResults) {
    out.push_str("## 4. Profile verdict (machine-computed)\n\n");
    out.push_str(
        "CORE/STANDARD are all-of (every listed capability must be `pass`); OPTIONS is \
         any-of (obtained if any optional capability passes) — `master03-profiles.adoc`. \
         An unevidenced required capability fails the claim.\n\n",
    );
    for profile in [Profile::Core, Profile::Standard, Profile::Options] {
        let verdict = profile_verdict(results, profile);
        let _ = writeln!(out, "### {profile:?} — {}\n", claim_word(profile, verdict));
        out.push_str("| Capability | Passed | Failed | Skipped | N/A | Evidence |\n");
        out.push_str("|---|--:|--:|--:|--:|---|\n");
        for line in profile_lines(results, profile) {
            let _ = writeln!(
                out,
                "| {:?} | {} | {} | {} | {} | {} |",
                line.capability,
                line.passed,
                line.failed,
                line.skipped,
                line.not_applicable,
                evidence_word(line.evidence),
            );
        }
        out.push('\n');
    }
}

fn render_failures(out: &mut String, results: &RunResults) {
    let failures: Vec<&CaseOutcome> = results
        .cases
        .iter()
        .filter(|c| c.status == CaseStatus::Failed)
        .collect();
    out.push_str("## 5. Failures\n\n");
    if failures.is_empty() {
        out.push_str("_No failures in this run._\n\n");
        return;
    }
    out.push_str(
        "Each failure is a conformance finding — never an exclusion (standing rule 3).\n\n",
    );
    for c in failures {
        let _ = writeln!(
            out,
            "- **{}** {} (`{}`, {}): {}\n  _cite: {}_",
            c.ecc_id,
            c.title,
            c.id,
            c.format,
            c.message.as_deref().unwrap_or("(no message)"),
            citation_or_dash(&c.citation),
        );
    }
    out.push('\n');
}

fn render_errors(out: &mut String, results: &RunResults) {
    let errors: Vec<&CaseOutcome> = results
        .cases
        .iter()
        .filter(|c| c.status == CaseStatus::Errored)
        .collect();
    if errors.is_empty() {
        return;
    }
    out.push_str("## 5b. Runner/SUT errors (transport)\n\n");
    out.push_str(
        "Transport-level errors — not conformance findings, but the affected \
         capabilities cannot be claimed as passed.\n\n",
    );
    for c in errors {
        let _ = writeln!(
            out,
            "- **{}** {} ({}): {}",
            c.ecc_id,
            c.title,
            c.format,
            c.message.as_deref().unwrap_or("(no message)"),
        );
    }
    out.push('\n');
}

fn render_skips(out: &mut String, results: &RunResults) {
    out.push_str("## 6. Skipped, by reason\n\n");
    let mut reasons: BTreeMap<&str, usize> = BTreeMap::new();
    for c in results
        .cases
        .iter()
        .filter(|c| c.status == CaseStatus::Skipped)
    {
        *reasons
            .entry(c.message.as_deref().unwrap_or("(unstated)"))
            .or_insert(0) += 1;
    }
    if reasons.is_empty() {
        out.push_str("_No skips in this run._\n\n");
        return;
    }
    out.push_str("| Reason | Cases |\n|---|--:|\n");
    for (reason, count) in &reasons {
        let _ = writeln!(out, "| {reason} | {count} |");
    }
    out.push('\n');
}

fn render_not_applicable(out: &mut String, results: &RunResults) {
    // Collapse formats — one line per ECC id.
    let mut order: Vec<String> = Vec::new();
    let mut seen: BTreeMap<String, &CaseOutcome> = BTreeMap::new();
    for c in results
        .cases
        .iter()
        .filter(|c| c.status == CaseStatus::NotApplicable)
    {
        let key = na_key(c);
        seen.entry(key.clone()).or_insert_with(|| {
            order.push(key.clone());
            c
        });
    }
    out.push_str("## 7. Not applicable to this SUT (extensions / RM-version-sensitive)\n\n");
    if order.is_empty() {
        out.push_str("_None — every catalogued case applies to this SUT._\n\n");
        return;
    }
    out.push_str(
        "Adjudicated in the committed fairness register (foreign SUTs only), not a \
         conformance finding — excluded from pass/fail and capability math.\n\n",
    );
    for key in &order {
        let c = seen[key];
        let _ = writeln!(
            out,
            "- **{}** {} — {} _(cite: {})_",
            c.ecc_id,
            c.title,
            c.message.as_deref().unwrap_or("(no reason)"),
            citation_or_dash(&c.citation),
        );
    }
    out.push('\n');
}

fn render_edition_findings(out: &mut String, results: &RunResults) {
    let with_findings: Vec<&CaseOutcome> = results
        .cases
        .iter()
        .filter(|c| c.edition_level.is_some() || !c.edition_findings.is_empty())
        .collect();
    out.push_str("## 8. Edition findings (the SUT's discovered edition profile)\n\n");
    out.push_str(
        "A case satisfied its normative core at a rung below the newest edition — recorded, \
         never a silent pass (`master03-overview.adoc` §API Conformance; the aggregated \
         findings feed the Conformance Statement's supported-versions field).\n\n",
    );
    if with_findings.is_empty() {
        out.push_str("_None — every laddered assertion matched the newest edition form._\n\n");
        return;
    }
    out.push_str("| ECC id | Format | Satisfied rung | Observations |\n|---|---|---|---|\n");
    for c in with_findings {
        let _ = writeln!(
            out,
            "| {} | {} | {} | {} |",
            c.ecc_id,
            c.format,
            c.edition_level.as_deref().unwrap_or("—"),
            if c.edition_findings.is_empty() {
                "—".to_owned()
            } else {
                c.edition_findings.join("; ")
            },
        );
    }
    out.push('\n');
}

fn render_coverage_bounds(out: &mut String, results: &RunResults) {
    let bounded: Vec<&CaseOutcome> = results
        .cases
        .iter()
        .filter(|c| c.schedule_rows.is_some_and(|rows| c.total_data_sets < rows))
        .collect();
    out.push_str("## 9. Coverage bounds (driven vs schedule data-set rows)\n\n");
    out.push_str(
        "Cases whose driven data-set count is below the governing schedule table's row \
         count — a bound is logged, never silent. \
         Widening the driven set is data, not a new case.\n\n",
    );
    if bounded.is_empty() {
        out.push_str("_No coverage bounds — every case drives its full schedule data set._\n\n");
        return;
    }
    out.push_str("| ECC id | Format | Driven / schedule rows |\n|---|---|--:|\n");
    for c in bounded {
        let _ = writeln!(
            out,
            "| {} | {} | {}/{} |",
            c.ecc_id,
            c.format,
            c.total_data_sets,
            c.schedule_rows.unwrap_or(c.total_data_sets),
        );
    }
    out.push('\n');
}

fn render_ecc_original(out: &mut String, results: &RunResults) {
    // Collapse formats — one line per ECC id.
    let mut order: Vec<String> = Vec::new();
    let mut seen: BTreeMap<String, &CaseOutcome> = BTreeMap::new();
    for c in results.cases.iter().filter(|c| c.ecc_original.is_some()) {
        let key = na_key(c);
        seen.entry(key.clone()).or_insert_with(|| {
            order.push(key.clone());
            c
        });
    }
    out.push_str("## 10. ECC-original cases (no direct schedule backing)\n\n");
    out.push_str(
        "Stub-derived / extension cases — labelled here and **never presented as \
         schedule-conformant**. Their result stands, but the claim \
         is against our own derivation, not an abstract schedule test case.\n\n",
    );
    if order.is_empty() {
        out.push_str("_None — every executed case traces to a schedule test case._\n\n");
        return;
    }
    for key in &order {
        let c = seen[key];
        let _ = writeln!(
            out,
            "- **{}** {} — {}",
            c.ecc_id,
            c.title,
            c.ecc_original.as_deref().unwrap_or("ECC-original"),
        );
    }
    out.push('\n');
}

fn render_detailed(out: &mut String, results: &RunResults) {
    out.push_str("## 11. Detailed test report\n\n");
    if results.cases.is_empty() {
        out.push_str("_No cases executed in this run._\n\n");
        return;
    }
    out.push_str(
        "| ECC id | Capability | Format | Data sets | Rung | Result |\n\
         |---|---|---|--:|---|---|\n",
    );
    for c in &results.cases {
        let _ = writeln!(
            out,
            "| {} | {} | {} | {}/{} | {} | {} |",
            c.ecc_id,
            c.capability,
            c.format,
            c.passed_data_sets,
            c.total_data_sets,
            c.edition_level.as_deref().unwrap_or("—"),
            result_word(c.status),
        );
    }
    out.push('\n');
}

fn render_terminology(out: &mut String, results: &RunResults) {
    out.push_str("## 12. Terminology server (TS area)\n\n");
    let Some(tx) = &results.terminology else {
        out.push_str("_No terminology server was established for this run._\n");
        return;
    };
    let _ = writeln!(out, "- Server: `{}`\n- Mode: {}\n", tx.base_url, tx.mode);
    if tx.exchanges.is_empty() {
        out.push_str("_No FHIR-tx exchange recorded._\n");
        return;
    }
    let _ = writeln!(
        out,
        "Recorded FHIR-tx exchange ({} request(s)):\n",
        tx.exchanges.len()
    );
    out.push_str("| # | Method | Path | Query |\n|--:|---|---|---|\n");
    for (i, e) in tx.exchanges.iter().enumerate() {
        let _ = writeln!(
            out,
            "| {} | {} | `{}` | {} |",
            i + 1,
            e.method,
            e.path,
            e.query.as_deref().unwrap_or("—"),
        );
    }
}

/// The result word for a case status in the detailed report.
fn result_word(status: CaseStatus) -> &'static str {
    match status {
        CaseStatus::Passed => "PASS",
        CaseStatus::Failed => "**FAIL**",
        CaseStatus::Errored => "ERROR",
        CaseStatus::Skipped => "skipped",
        CaseStatus::NotApplicable => "N/A",
    }
}

fn na_key(c: &CaseOutcome) -> String {
    if c.ecc_id.is_empty() {
        c.id.clone()
    } else {
        c.ecc_id.clone()
    }
}

fn citation_or_dash(citation: &str) -> &str {
    if citation.is_empty() { "—" } else { citation }
}

// ── CATALOG.md (the `catalog` subcommand) ────────────────────────────────────

/// Render `CATALOG.md` — the full ECC catalogue grouped per area, optionally
/// annotated with the last run's outcome per case.
#[must_use]
pub fn render_catalog_md(results: Option<&RunResults>, catalog: &Catalog) -> String {
    let mut out = String::from(
        "# The Conformance Catalogue (ECC)\n\n\
         Generated — do not edit. Numbers are allocated once in\n\
         `tools/conformance/inventory/ecc-catalog.tsv` and never reused.\n\n",
    );

    let outcome_of = |ecc_id: &str| -> String {
        let Some(results) = results else {
            return String::new();
        };
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
            active,
        );
        out.push_str("| ECC id | Status | Title | Last run |\n|---|---|---|---|\n");
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
                },
            );
        }
        out.push('\n');
    }
    out
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
    use crate::model::versions::SpecVersions;
    use crate::reporting::results::{
        CaseOutcome, CorpusPin, ProductIdentity, SelectionInfo, SutIdentity,
    };

    fn outcome(ecc_id: &str, capability: &str, status: CaseStatus) -> CaseOutcome {
        CaseOutcome {
            ecc_id: ecc_id.to_owned(),
            id: "area/slug".to_owned(),
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
                edition_policy: EditionPolicy::Pinned(crate::edition::Edition::Release110),
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
    fn errored_capability_never_passes_the_profile() {
        let r = results(vec![outcome(
            "ECC-EHR-001",
            "EhrOperations",
            CaseStatus::Errored,
        )]);
        let v = capability_verdict(&r, Capability::EhrOperations);
        assert_eq!(v.evidence, CapabilityEvidence::Failed);
    }

    #[test]
    fn zero_state_report_renders_every_section() {
        let md = render_report_md(&results(Vec::new()), &Catalog::default());
        assert!(md.contains("System under test"));
        assert!(md.contains("Capability matrix"));
        assert!(md.contains("Profile verdict"));
        assert!(md.contains("Edition findings"));
        assert!(md.contains("Coverage bounds"));
        assert!(md.contains("ECC-original"));
        assert!(md.contains("0 passed"));
    }

    #[test]
    fn coverage_bound_is_printed_when_driven_below_schedule() {
        let mut c = outcome("ECC-VAL-001", "ArchetypeValidation", CaseStatus::Passed);
        c.total_data_sets = 3;
        c.schedule_rows = Some(27);
        let md = render_report_md(&results(vec![c]), &Catalog::default());
        assert!(md.contains("3/27"), "the driven/schedule ratio is printed");
    }
}
