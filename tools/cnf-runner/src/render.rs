// SPDX-FileCopyrightText: FerroEHR contributors
// SPDX-License-Identifier: Apache-2.0

//! Deterministic Markdown renderers for the three published submission
//! documents: the conformance report, the conformance statement (`SDoC`), and
//! the certificate.
//!
//! Every renderer is a pure function of its inputs, emits byte-deterministic
//! Markdown (stable ordering, no timestamps — the caller stamps a date if it
//! needs one), and ends with a trailing newline. The certificate's table
//! shape follows the CNF certificate book
//! (`CNF/docs/certificate/master03-certificate.adoc`).

#![allow(
    clippy::disallowed_types,
    reason = "dev/verification tooling over JSON artifacts (the catalogue, results, wire \
              exchanges) — not the application (#1694); the carriers here are cfg(test)-only, so \
              #[expect] would be unfulfilled in the non-test build"
)]

use std::collections::BTreeMap;
use std::fmt::Write;

use crate::model::capability::CapabilityMatrix;
use crate::model::wire_surface::ServedExtension;
use crate::party::{OutcomeStatus, Results, Statement};
use crate::verdict::{Evidence, ProfileVerdict, SecBasicVerdict, VerdictReport};
use crate::vocab::{Family, FormatName, Tier};

/// Render the conformance report (`CONFORMANCE_REPORT.md`): the outcome
/// summary, a per-chapter table, and the honesty block (coverage bound +
/// every not-executed verdict's citation).
///
/// The per-chapter table groups by the SAME two-level taxonomy the published
/// chapter-bars chart renders ([`crate::conf_assets::TAXONOMY`], via
/// [`crate::conf_assets::chapter_counts`]) — one taxonomy, one place, so the
/// report and the chart can never disagree about what a chapter contains.
///
/// # Errors
///
/// [`crate::conf_assets::TaxonomyError`] when an outcome's case id maps to no
/// taxonomy band — a taxonomy gap to close, never a silent bucket; the report
/// refuses to publish rather than mis-group.
#[expect(
    clippy::too_many_lines,
    reason = "one linear document renderer per published artifact"
)]
pub fn render_report(
    results: &Results,
    verdicts: &VerdictReport,
    statement: &Statement,
) -> Result<String, crate::conf_assets::TaxonomyError> {
    let mut out = String::new();
    let _ = writeln!(out, "# Conformance Report\n");
    // The party's declared product DISPLAY name leads; the machine `sut` key
    // (which names the artifact directories) stays visible beside it.
    let _ = writeln!(
        out,
        "SUT: {} {} (sut `{}`) · schedule {} · ITS {}",
        statement.product.name,
        results.sut.version,
        results.sut.name,
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
    // The published chart's two-level taxonomy, verbatim: chapter rows with
    // their band sub-rows, in declaration order. `cited n/a` merges the two
    // citation-bearing statuses (`not_applicable` + `skipped`) exactly as the
    // chart's hatched segment does. All-empty bands are elided from the
    // table (the chart draws them as zero-width segments; a table row of
    // zeros carries no information), but an all-empty CHAPTER still prints
    // its total row so the taxonomy stays visibly total.
    let chapters = crate::conf_assets::chapter_counts(results)?;
    let _ = writeln!(out, "## By chapter\n");
    let _ = writeln!(
        out,
        "Grouping is the published per-chapter chart's taxonomy: chapters \
         with their bands; `cited n/a` counts the not-executed outcomes that \
         carry a citation (`not_applicable` + `skipped`).\n"
    );
    let _ = writeln!(
        out,
        "| Chapter / band | passed | failed | errored | cited n/a |"
    );
    let _ = writeln!(out, "| --- | --- | --- | --- | --- |");
    for chapter in &chapters {
        let t = chapter.total;
        let _ = writeln!(
            out,
            "| **{}** | {} | {} | {} | {} |",
            chapter.chapter, t.passed, t.failed, t.errored, t.cited_na,
        );
        for (band, counts) in &chapter.bands {
            if counts.is_empty() {
                continue;
            }
            let _ = writeln!(
                out,
                "| — {} | {} | {} | {} | {} |",
                band, counts.passed, counts.failed, counts.errored, counts.cited_na,
            );
        }
    }
    let _ = writeln!(out);

    // ── per-capability rows ─────────────────────────────────────────────────
    // The headline evidence token is worst-wins, so on its own it hides an
    // inconclusive row inside a capability that also has passes. The counts
    // travel with it (issue #629): an errored exchange is never a SUT failure,
    // but it is never evidence of conformance either, and a divergence must
    // not be able to sit behind one unseen.
    if !verdicts.capability_tallies.is_empty() {
        let _ = writeln!(out, "## By capability\n");
        let _ = writeln!(
            out,
            "Selected gating cases per claimed capability. `inconclusive` counts cases whose \
             exchange did not conclude (transport fault, unmapped status, step resolution) — \
             they block a `passed` evidence token and are triaged, never absorbed.\n"
        );
        let _ = writeln!(
            out,
            "| Capability | Evidence | passed | failed | inconclusive | unevidenced |"
        );
        let _ = writeln!(out, "| --- | --- | --- | --- | --- | --- |");
        for (name, tally) in &verdicts.capability_tallies {
            if tally.selected() == 0 {
                continue; // nothing selected for this claim — the matrix row says so
            }
            let evidence = verdicts
                .capabilities
                .iter()
                .find(|(n, _)| n == name)
                .map_or(Evidence::NotEvidenced, |(_, e)| *e);
            let _ = writeln!(
                out,
                "| {} | {} | {} | {} | {} | {} |",
                name,
                evidence_token(evidence),
                tally.passed,
                tally.failed,
                tally.inconclusive,
                tally.unevidenced,
            );
        }
        let _ = writeln!(out);
    }

    // ── performance measurements ────────────────────────────────────────────
    if !results.measurements.is_empty() {
        let _ = writeln!(out, "## Performance measurements\n");
        for m in &results.measurements {
            let _ = writeln!(
                out,
                "### {} — class {} · {}\n",
                m.case,
                m.class.token(),
                verdict_token(m.verdict),
            );
            let _ = writeln!(
                out,
                "Offered load sustained: {:.2}/s over {} s (after {} s warmup) · environment: {} ({} cores, {} GB, {}, {})\n",
                m.offered_load_sustained,
                m.duration_s,
                m.warmup_s,
                m.environment.hardware_class,
                m.environment.cores,
                m.environment.memory_gb,
                m.environment.storage_class,
                m.environment.topology,
            );
            let _ = writeln!(
                out,
                "| Operation | Requests | Errors | p50 (ms) | p90 (ms) | p99 (ms) |"
            );
            let _ = writeln!(out, "| --- | --- | --- | --- | --- | --- |");
            for op in &m.operations {
                let _ = writeln!(
                    out,
                    "| {} | {} | {} | {:.1} | {:.1} | {:.1} |",
                    op.operation,
                    op.requests,
                    op.errors,
                    op.latency_ms_p50,
                    op.latency_ms_p90,
                    op.latency_ms_p99,
                );
            }
            if !m.violations.is_empty() {
                let _ = writeln!(out, "\nViolations:\n");
                for v in &m.violations {
                    let _ = writeln!(out, "- {v}");
                }
            }
            let _ = writeln!(out);
        }
        let _ = writeln!(
            out,
            "Percentiles re-derive from the embedded HDR V2 histograms; the class verdict is recomputed from them by the verdict pipeline, never trusted from this table.\n"
        );
    }

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

    Ok(out)
}

/// Render the conformance statement (`CONFORMANCE_STATEMENT.md`): the `SDoC`
/// text, the claims, the declared non-openEHR surface, the computed verdicts,
/// and the attestation.
///
/// `served_extensions` is the catalogue's outward wire-surface axis
/// (`vocab/wire_surface.yaml`), the ROUTE DETAIL of each family. What a given
/// statement publishes is scoped by that party's own
/// [`Statement::served_extensions`] declaration — a party declaring none says
/// so explicitly — because a route family is one product's own design and a
/// statement may never declare another vendor's surface. It is a declaration
/// and never enters a verdict.
#[must_use]
pub fn render_statement(
    statement: &Statement,
    verdicts: &VerdictReport,
    served_extensions: &[ServedExtension],
) -> String {
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

    // ── the outward surface (a declaration, never a claim) ──────────────────
    write_served_extensions(&mut out, statement, served_extensions);

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
    for perf in &verdicts.performance {
        let _ = writeln!(
            out,
            "| Performance class {}{} | {} |",
            perf.class.token(),
            if perf.claimed { " (claimed)" } else { "" },
            verdict_token(perf.verdict),
        );
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

/// The statement's "Additional non-openEHR surface" section: the families THIS
/// party declares (`statement.served_extensions`), rendered with the route and
/// configuration detail the catalogue axis carries for each, under
/// release-pinned wording that puts the whole surface outside every
/// conformance claim in the document.
///
/// A party declaring no family gets the section with an explicit statement of
/// none — silence would leave a reader unable to tell "declares nothing" from
/// "the question was never asked". A declared family the catalogue axis does
/// not carry is rendered as such rather than dropped; the `served-extension-
/// declaration` validate gate refuses it before any run.
fn write_served_extensions(
    out: &mut String,
    statement: &Statement,
    served_extensions: &[ServedExtension],
) {
    let _ = writeln!(out, "## Additional non-openEHR surface\n");
    let its_rest = statement
        .spec_versions
        .get(crate::vocab::SpecComponent::ItsRest)
        .unwrap_or("(unstated)");
    if statement.served_extensions.is_empty() {
        let _ = writeln!(
            out,
            "Beside the openEHR resources of ITS-REST {its_rest}, this product declares no \
             additional route family in this statement.\n"
        );
        return;
    }
    let _ = writeln!(
        out,
        "Beside the openEHR resources of ITS-REST {its_rest}, this product serves the route \
         families below. **None of them is part of any conformance claim in this \
         statement**: no openEHR specification governs them, no conformance case \
         exercises them, and no verdict below depends on them. They are declared here \
         so a reader of this document learns the surface exists rather than \
         discovering it on the wire. Paths are the default deployment spelling; a \
         non-default API base path moves the base-path-relative ones.\n",
    );
    let _ = writeln!(out, "| Family | Routes | Enabled by |");
    let _ = writeln!(out, "| --- | --- | --- |");
    for family in &statement.served_extensions {
        match served_extensions.iter().find(|e| &e.family == family) {
            Some(extension) => {
                let routes: Vec<String> =
                    extension.routes.iter().map(|r| format!("`{r}`")).collect();
                let _ = writeln!(
                    out,
                    "| {} | {} | {} |",
                    extension.family,
                    routes.join("<br>"),
                    extension.config_gate,
                );
            }
            None => {
                let _ = writeln!(
                    out,
                    "| {family} | (declared by this party; the catalogue wire-surface axis \
                     carries no route detail for it) | — |"
                );
            }
        }
    }
    let _ = writeln!(out);
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
#[expect(
    clippy::too_many_lines,
    reason = "one linear document renderer per published artifact"
)]
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
        "| Performance | {} |",
        performance_scope_cell(verdicts)
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
        "\nThe Realization column says what the row's cases were verified against: \
         `released-wire` = released ITS-REST operations; `extension` = routes this product \
         serves of its own design, which no openEHR specification governs and which therefore \
         never gate an openEHR profile tier (those rows are always OPT)."
    );
    let _ = writeln!(
        out,
        "\n| Family | Capability | Required in profile | Realization | Result |"
    );
    let _ = writeln!(out, "| --- | --- | --- | --- | --- |");
    for (name, entry) in matrix.entries() {
        let evidence = verdicts
            .capabilities
            .iter()
            .find(|(n, _)| n == name)
            .map_or(Evidence::NotEvidenced, |(_, e)| *e);
        let _ = writeln!(
            out,
            "| {} | {} | {} | {} | {} |",
            family_token(entry.family),
            name,
            required_token(entry.required),
            entry.realization.token(),
            evidence_token(evidence),
        );
    }
    let _ = writeln!(out);

    // ── workload coverage (claimed vs exercised by the hospital simulation) ─
    if !results.measurements.is_empty() {
        let _ = writeln!(out, "## Workload Coverage\n");
        let _ = writeln!(
            out,
            "The exercised-capability set of the measured hospital-simulation workload \
             against the claimed matrix. A claimed capability the simulation never touches is \
             either an ADJUDICATED exclusion — the capability-matrix row names the register \
             entry that decided it and the reason is printed in the row — or an undecided \
             catalogue gap, which the `workload-coverage` validate gate fails on, so no \
             published certificate reaches this section carrying one."
        );
        // Exercised = the union of every measured operation label's
        // capability set (labels come from the committed measurement
        // records, so this reflects what actually ran, not the plan).
        let mut exercised: Vec<&'static str> = Vec::new();
        for m in &results.measurements {
            for op in &m.operations {
                if op.requests == 0 {
                    continue;
                }
                if let Ok(parsed) = crate::perf::PerfOp::parse(&op.operation) {
                    for capability in parsed.capabilities() {
                        if !exercised.contains(capability) {
                            exercised.push(capability);
                        }
                    }
                }
            }
        }
        let _ = writeln!(out, "\n| Capability | Claimed | Exercised by workload |");
        let _ = writeln!(out, "| --- | --- | --- |");
        let mut gaps: Vec<&str> = Vec::new();
        let mut excluded: Vec<&str> = Vec::new();
        for (name, entry) in matrix.entries() {
            let claimed = statement.claims.capabilities.iter().any(|c| c == name);
            if !claimed {
                continue;
            }
            let touched = exercised.contains(&name.as_str());
            let cell = if touched {
                "yes".to_owned()
            } else if let Some(adjudication) = &entry.workload_exclusion {
                excluded.push(name.as_str());
                format!(
                    "no — adjudicated exclusion ({}): {}",
                    adjudication.register, adjudication.reason
                )
            } else {
                gaps.push(name.as_str());
                "NO — catalogue gap (UNADJUDICATED)".to_owned()
            };
            let _ = writeln!(out, "| {name} | yes | {cell} |");
        }
        if !excluded.is_empty() {
            let _ = writeln!(
                out,
                "\nClaimed capabilities excluded from the measured workload by adjudication \
                 ({}): {}. Each row above names its register entry; the exclusion bounds the \
                 LOAD instrument only — the functional catalogue still owes every one of them \
                 verdict-bearing cases at its `min_cases` floor.",
                excluded.len(),
                excluded.join(", "),
            );
        }
        if gaps.is_empty() {
            let _ = writeln!(
                out,
                "\nEvery claimed capability is exercised by the simulation or carries an \
                 adjudicated exclusion — no undecided rows."
            );
        } else {
            let _ = writeln!(
                out,
                "\nUNADJUDICATED gaps ({}): {}. These rows are a defect in this submission, \
                 not a property of the product: the `workload-coverage` validate gate fails on \
                 each of them, so this certificate was rendered from an artifact tree that does \
                 not pass its own gates.",
                gaps.len(),
                gaps.join(", "),
            );
        }
        let _ = writeln!(out);
    }

    // ── performance rating ──────────────────────────────────────────────────
    if !verdicts.performance.is_empty() {
        let _ = writeln!(out, "## Performance Rating\n");
        let _ = writeln!(
            out,
            "Classes are EARNED by measurement (never declared); every earned class is bound to the measured environment recorded in the results."
        );
        let _ = writeln!(out, "\n| Class | Case | Claimed | Verdict |");
        let _ = writeln!(out, "| --- | --- | --- | --- |");
        for perf in &verdicts.performance {
            let _ = writeln!(
                out,
                "| {} | {} | {} | {} |",
                perf.class.token(),
                perf.case,
                if perf.claimed { "yes" } else { "no" },
                verdict_token(perf.verdict),
            );
        }
        for m in &results.measurements {
            let _ = writeln!(
                out,
                "\nEnvironment ({}): {} · {} cores · {} GB · {} · {}",
                m.case,
                m.environment.hardware_class,
                m.environment.cores,
                m.environment.memory_gb,
                m.environment.storage_class,
                m.environment.topology,
            );
        }
        let _ = writeln!(out);
    }

    out
}

// ── shared formatting helpers ────────────────────────────────────────────────

fn declared_versions(statement: &Statement) -> Vec<(&'static str, &str)> {
    use crate::vocab::SpecComponent;
    let mut out = Vec::new();
    for component in SpecComponent::ALL {
        if let Some(version) = statement.spec_versions.get(*component) {
            out.push((component.token(), version));
        }
    }
    out
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
        Evidence::Inconclusive => "INCONCLUSIVE (errored rows — never green by absorption)",
        Evidence::NotEvidenced => "not evidenced",
        Evidence::NotClaimed => "not claimed",
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

fn verdict_token(verdict: crate::perf::ClassVerdict) -> &'static str {
    match verdict {
        crate::perf::ClassVerdict::Earned => "EARNED",
        crate::perf::ClassVerdict::NotEarned => "not earned",
    }
}

/// The certificate's Scope-of-Test performance cell: the earned classes, or
/// an explicit dash when nothing was measured.
fn performance_scope_cell(verdicts: &VerdictReport) -> String {
    let earned: Vec<&'static str> = verdicts
        .performance
        .iter()
        .filter(|p| p.verdict == crate::perf::ClassVerdict::Earned)
        .map(|p| p.class.token())
        .collect();
    if earned.is_empty() {
        "—".to_owned()
    } else {
        format!("class {} (earned)", earned.join(", "))
    }
}

#[cfg(test)]
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
            "product": { "name": "FerroEHR", "version": "3.5.0",
                          "vendor": "Ruben Talstra", "identifier": "urn:x" },
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
            "sut": { "name": "ferroehr", "version": "3.5.0" },
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
        crate::verdict::compute(&statement(), &results(), &cases, &[], &matrix(), &register)
    }

    use crate::model::case::CaseCore;

    #[test]
    fn report_is_deterministic_and_lists_citations() {
        let v = verdicts();
        let a = render_report(&results(), &v, &statement()).unwrap();
        let b = render_report(&results(), &v, &statement()).unwrap();
        assert_eq!(a, b);
        assert!(a.ends_with('\n'));
        assert!(a.contains("Coverage: 1 of 1"));
        assert!(a.contains("AMB-33"));
        // The by-chapter table groups by the published chart's taxonomy:
        // the fixture's create_ehr case lands in the EHR chapter's
        // "EHR lifecycle" band, exactly as conf_assets::TAXONOMY declares.
        assert!(a.contains("| **EHR** |"));
    }

    /// The catalogue's extension families, as the axis carries them.
    fn served() -> Vec<ServedExtension> {
        serde_json::from_value(serde_json::json!([
            { "family": "management", "routes": ["GET /management/info"],
              "config_gate": "management.enabled (default off)",
              "spec_silence": "no released clause governs the URI space beyond the resource set",
              "never_gates": true }
        ]))
        .unwrap()
    }

    #[test]
    fn statement_renders_verdicts_and_attestation() {
        let text = render_statement(&statement(), &verdicts(), &[]);
        assert!(text.ends_with('\n'));
        assert!(text.contains("We declare conformance."));
        assert!(text.contains("CORE"));
        // A party declaring no family SAYS so — silence would leave a reader
        // unable to tell "declares nothing" from "never asked".
        assert!(text.contains("declares no additional route family"));
    }

    /// The outward axis renders as a declaration: the family, its routes and
    /// its gate, under wording that says it is in no conformance claim.
    #[test]
    fn statement_declares_the_non_openehr_surface() {
        let mut statement = statement();
        statement.served_extensions = vec!["management".to_owned()];
        let text = render_statement(&statement, &verdicts(), &served());
        assert!(text.contains("## Additional non-openEHR surface"));
        assert!(text.contains("ITS-REST 1.1.0"));
        assert!(text.contains(
            "| management | `GET /management/info` | management.enabled (default off) |"
        ));
        assert!(text.contains("None of them is part of any conformance claim"));
    }

    /// A statement never publishes a family the party did not declare.
    ///
    /// The catalogue axis is ONE product's outward surface, so rendering it
    /// into every party's `SDoC` declared another vendor's routes as that
    /// vendor's own (#2377).
    #[test]
    fn a_party_declaring_no_family_publishes_none_of_the_catalogue_table() {
        let text = render_statement(&statement(), &verdicts(), &served());
        assert!(text.contains("## Additional non-openEHR surface"));
        assert!(text.contains("declares no additional route family"));
        assert!(!text.contains("management"));
        assert!(!text.contains("GET /management/info"));
    }

    /// A declared family the catalogue axis does not carry is rendered as
    /// such, never dropped — the `served-extension-declaration` gate refuses
    /// it before any run, and a silent omission would hide the defect.
    #[test]
    fn a_declared_family_without_route_detail_is_rendered_as_such() {
        let mut statement = statement();
        statement.served_extensions = vec!["not-in-the-axis".to_owned()];
        let text = render_statement(&statement, &verdicts(), &served());
        assert!(text.contains("| not-in-the-axis |"));
        assert!(text.contains("carries no route detail for it"));
    }

    #[test]
    fn certificate_has_profile_report_columns() {
        let text = render_certificate(&statement(), &results(), &verdicts(), &matrix());
        assert!(text.ends_with('\n'));
        assert!(text.contains("Required in profile"));
        assert!(text.contains("| Platform | EhrOperations | Y | released-wire | pass |"));
        assert!(text.contains("| Platform | SimplifiedFormats | OPT | released-wire |"));
    }

    /// The realization marker is per row: an `extension` capability is
    /// verified over routes no openEHR specification governs, and the
    /// certificate says so instead of letting the row read like released
    /// wire.
    #[test]
    fn certificate_marks_extension_realization() {
        let matrix: CapabilityMatrix = serde_json::from_value(serde_json::json!({
            "EhrOperations": { "family": "Platform", "tier": "CORE", "required": true,
                                "min_cases": 1 },
            "Tds": { "family": "Platform", "tier": "OPTIONS", "required": false,
                      "min_cases": 4, "realization": "extension" }
        }))
        .unwrap();
        let text = render_certificate(&statement(), &results(), &verdicts(), &matrix);
        assert!(text.contains("| Platform | Tds | OPT | extension |"));
        assert!(text.contains("| Platform | EhrOperations | Y | released-wire |"));
    }

    /// The Workload Coverage table renders an adjudicated exclusion with its
    /// register id and reason, and names an unadjudicated gap as a defect of
    /// the submission rather than a neutral "catalogue gap" row.
    #[test]
    fn workload_coverage_renders_adjudications_and_flags_bare_gaps() {
        let matrix: CapabilityMatrix = serde_json::from_value(serde_json::json!({
            "EhrOperations": { "family": "Platform", "tier": "CORE", "required": true,
                                "min_cases": 1 },
            "SimplifiedFormats": { "family": "Platform", "tier": "OPTIONS", "required": false,
                                    "min_cases": 1,
                                    "workload_exclusion": { "register": "AMB-170",
                                                             "reason": "outside the load mix" } }
        }))
        .unwrap();
        let statement: Statement = serde_json::from_value(serde_json::json!({
            "product": { "name": "FerroEHR", "version": "3.5.0",
                          "vendor": "Ruben Talstra", "identifier": "urn:x" },
            "schedule_release": "CNF-2.0",
            "spec_versions": { "rm": "1.2.0", "its_rest": "1.1.0" },
            "claims": { "capabilities": ["EhrOperations", "SimplifiedFormats"],
                         "profiles": ["CORE"] },
            "tech_profiles": [ { "its": "its-rest", "formats": ["canonical-json"] } ]
        }))
        .unwrap();
        let mut results = results();
        results.measurements = vec![measurement()];

        let text = render_certificate(&statement, &results, &verdicts(), &matrix);
        assert!(text.contains(
            "| SimplifiedFormats | yes | no — adjudicated exclusion (AMB-170): outside the load mix |"
        ));
        // ehr_create is the measured label below, so EhrOperations is exercised.
        assert!(text.contains("| EhrOperations | yes | yes |"));
        assert!(!text.contains("UNADJUDICATED"));

        // Drop the adjudication: the same row must read as a defect.
        let bare: CapabilityMatrix = serde_json::from_value(serde_json::json!({
            "EhrOperations": { "family": "Platform", "tier": "CORE", "required": true,
                                "min_cases": 1 },
            "SimplifiedFormats": { "family": "Platform", "tier": "OPTIONS", "required": false,
                                    "min_cases": 1 }
        }))
        .unwrap();
        let text = render_certificate(&statement, &results, &verdicts(), &bare);
        assert!(text.contains("| SimplifiedFormats | yes | NO — catalogue gap (UNADJUDICATED) |"));
        assert!(text.contains("UNADJUDICATED gaps (1): SimplifiedFormats"));
        assert!(text.contains("does not pass its own gates"));
    }

    /// One measured record whose only operation label is `ehr_create`.
    fn measurement() -> crate::perf::Measurement {
        serde_json::from_value(serde_json::json!({
            "case": "PERF-hospital_sim-class_POC",
            "class": "POC",
            "verdict": "not-earned",
            "offered_load_sustained": 1.0,
            "duration_s": 3600,
            "warmup_s": 60,
            "environment": { "hardware_class": "laptop", "cores": 8, "memory_gb": 16,
                              "storage_class": "nvme", "topology": "single-node" },
            "operations": [
                { "operation": "ehr_create", "requests": 10, "errors": 0,
                   "latency_ms_p50": 1.0, "latency_ms_p90": 2.0, "latency_ms_p99": 3.0,
                   "hdr_v2_base64": "" }
            ],
            "violations": []
        }))
        .unwrap()
    }
}
