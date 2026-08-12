// SPDX-FileCopyrightText: FerroEHR contributors
// SPDX-License-Identifier: MIT

//! Deterministic conformance SVG assets rendered from committed artifacts.
//!
//! The inputs are the committed party artifacts (`verdicts.json` +
//! `results.json` + the capability matrix) —
//! the `perf_assets` pattern applied to functional conformance: no
//! hand-drawn numbers, CI-guarded regeneration, light+dark via
//! `prefers-color-scheme`, evidence encoded twice (a CVD-safe fill AND a
//! glyph — never color alone).
//!
//! Two charts:
//! - the **capability heat grid** — one cell per capability, grouped by
//!   tier (CORE → STANDARD → OPTIONS → SEC-BASIC), the whole conformance
//!   story in one picture;
//! - the **per-chapter outcome bars** — a TWO-LEVEL taxonomy (chapter →
//!   band) over the catalogue's case ids: a chapter header carrying the
//!   chapter total, expanded into one scaled bar per band, with the exact
//!   passed / FAILED / errored / cited-N-A counts printed beside every row.
//!
//! The taxonomy ([`TAXONOMY`]) is TOTAL by contract: every case id maps to a
//! declared `(chapter, band)` pair and an unmapped id is a render ERROR
//! naming the id, never a silent `Other` bucket. A band with no case for a
//! given SUT still renders (as an explicit "no cases" row), so two SUTs'
//! charts read band-for-band.
//!
//! NOTE: no openEHR spec governs the visuals' chapter taxonomy — our own
//! presentation design over the catalogue's case-id families.

use std::fmt::Write;

use crate::model::capability::CapabilityMatrix;
use crate::party::Results;
use crate::verdict::Evidence;
use crate::vocab::Tier;

/// Shared SVG style block. Evidence colors are a CVD-safe set (blue for
/// pass, vermilion for fail, grey ramps for the non-verdict states — the
/// Okabe-Ito palette anchors); the glyph column is the second encoding.
const STYLE: &str = "<style>\n\
  text { fill: #52514e; font: 12px -apple-system, 'Segoe UI', Helvetica, Arial, sans-serif; }\n\
  .title { fill: #0b0b0b; font-weight: 600; }\n\
  .muted { fill: #8a8880; font-size: 11px; }\n\
  .grid { stroke: #e4e2dd; stroke-width: 1; }\n\
  .cell-label { fill: #ffffff; font-size: 11px; }\n\
  .cell-label-dim { fill: #0b0b0b; font-size: 11px; }\n\
  .ev-passed { fill: #0072b2; }\n\
  .ev-failed { fill: #d55e00; }\n\
  .ev-inconclusive { fill: #e69f00; }\n\
  .ev-not-evidenced { fill: #cbc9c2; }\n\
  .cell-required { stroke: #52514e; stroke-width: 1.2; }\n\
  .cell-optional { stroke: #cbc9c2; stroke-width: 1; stroke-dasharray: 3 2; }\n\
  .bar-passed { fill: #0072b2; }\n\
  .bar-failed { fill: #d55e00; }\n\
  .bar-errored { fill: #e69f00; }\n\
  .bar-na { fill: #cbc9c2; }\n\
  .seg-label { fill: #ffffff; font-size: 10.5px; }\n\
  .seg-label-dim { fill: #0b0b0b; font-size: 10.5px; }\n\
  @media (prefers-color-scheme: dark) {\n\
    text { fill: #c3c2b7; }\n\
    .title { fill: #ffffff; }\n\
    .muted { fill: #8f8e85; }\n\
    .grid { stroke: #3a3a38; }\n\
    .cell-label { fill: #ffffff; }\n\
    .cell-label-dim { fill: #e8e6e0; }\n\
    .ev-passed { fill: #3987e5; }\n\
    .ev-failed { fill: #e5484d; }\n\
    .ev-inconclusive { fill: #f5a623; }\n\
    .ev-not-evidenced { fill: #4a4a47; }\n\
    .cell-required { stroke: #c3c2b7; }\n\
    .cell-optional { stroke: #4a4a47; }\n\
    .bar-passed { fill: #3987e5; }\n\
    .bar-failed { fill: #e5484d; }\n\
    .bar-errored { fill: #d9a514; }\n\
    .bar-na { fill: #4a4a47; }\n\
    .seg-label-dim { fill: #e8e6e0; }\n\
  }\n\
</style>\n";

fn svg_open(out: &mut String, width: f64, height: f64) {
    let _ = writeln!(
        out,
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {width} {height}\" \
         width=\"{width}\" height=\"{height}\" role=\"img\">\n{STYLE}"
    );
}

/// A tier band of the grid: the tier + its (name, required, evidence)
/// cells in authored matrix order.
type TierBand<'a> = (Tier, Vec<(&'a str, bool, Evidence)>);

/// The evidence encoding: CSS class + glyph + legend label (the glyph and
/// label are the non-color channel).
fn evidence_encoding(evidence: Evidence) -> (&'static str, &'static str, &'static str) {
    match evidence {
        Evidence::Passed => ("ev-passed", "✓", "passed"),
        Evidence::Failed => ("ev-failed", "✕", "FAILED"),
        Evidence::Inconclusive => ("ev-inconclusive", "?", "INCONCLUSIVE"),
        Evidence::NotEvidenced => ("ev-not-evidenced", "○", "not evidenced"),
    }
}

/// Light text on the saturated fills, dark text on the grey ones.
fn label_class(evidence: Evidence) -> &'static str {
    match evidence {
        Evidence::Passed | Evidence::Failed => "cell-label",
        _ => "cell-label-dim",
    }
}

fn tier_title(tier: Tier) -> &'static str {
    match tier {
        Tier::Core => "CORE",
        Tier::Standard => "STANDARD",
        Tier::Options => "OPTIONS",
        Tier::SecBasic => "SEC-BASIC",
        Tier::EnterpriseD => "ENTERPRISE-D",
        Tier::EnterpriseM => "ENTERPRISE-M",
        Tier::EnterpriseX => "ENTERPRISE-X",
    }
}

/// The tier band order of the grid.
const TIER_ORDER: [Tier; 4] = [Tier::Core, Tier::Standard, Tier::Options, Tier::SecBasic];

const CELL_W: f64 = 190.0;
const CELL_H: f64 = 26.0;
const CELL_GAP: f64 = 6.0;
const GRID_COLS: usize = 5;
const MARGIN: f64 = 24.0;

/// The capability heat grid: one cell per capability, grouped by tier;
/// evidence encoded as fill color + glyph; required capabilities carry a
/// solid border, optional a dashed one.
///
/// Pure over its inputs: matrix order (authored) within tier bands, no
/// timestamps.
#[must_use]
#[expect(
    clippy::as_conversions,
    clippy::cast_precision_loss,
    reason = "grid counts/cell coordinates are far below 2^52"
)]
pub fn heat_grid_svg(
    sut_label: &str,
    matrix: &CapabilityMatrix,
    capabilities: &[(String, Evidence)],
) -> String {
    let evidence_of = |name: &str| {
        capabilities
            .iter()
            .find(|(n, _)| n == name)
            .map_or(Evidence::NotEvidenced, |(_, e)| *e)
    };

    // Tier bands in fixed order, entries in authored matrix order.
    let mut bands: Vec<TierBand<'_>> = Vec::new();
    for tier in TIER_ORDER {
        let entries: Vec<(&str, bool, Evidence)> = matrix
            .entries()
            .iter()
            .filter(|(_, e)| e.tier == tier)
            .map(|(name, e)| (name.as_str(), e.required, evidence_of(name.as_str())))
            .collect();
        if !entries.is_empty() {
            bands.push((tier, entries));
        }
    }

    let width = MARGIN * 2.0 + GRID_COLS as f64 * (CELL_W + CELL_GAP) - CELL_GAP;
    let mut height = 64.0; // title + legend
    for (_, entries) in &bands {
        let rows = entries.len().div_ceil(GRID_COLS);
        height += 26.0 + rows as f64 * (CELL_H + CELL_GAP) + 10.0;
    }
    height += 8.0;

    let mut out = String::new();
    svg_open(&mut out, width, height);
    let _ = writeln!(
        out,
        "<text x=\"{MARGIN}\" y=\"28\" class=\"title\">Capability conformance — {}</text>",
        xml_escape(sut_label)
    );
    // Legend (glyph + label per evidence kind; swatch + glyph = both channels).
    let mut lx = MARGIN;
    for evidence in [
        Evidence::Passed,
        Evidence::Failed,
        Evidence::Inconclusive,
        Evidence::NotEvidenced,
    ] {
        let (class, glyph, label) = evidence_encoding(evidence);
        let _ = write!(
            out,
            "<rect x=\"{lx}\" y=\"38\" width=\"14\" height=\"14\" rx=\"3\" class=\"{class}\"/>\
             <text x=\"{gx}\" y=\"49\" class=\"{lc}\" text-anchor=\"middle\" font-size=\"10\">{glyph}</text>\
             <text x=\"{tx}\" y=\"49\" class=\"muted\">{label}</text>",
            gx = lx + 7.0,
            lc = label_class(evidence),
            tx = lx + 18.0,
        );
        lx += 20.0 + 7.2 * label.chars().count() as f64 + 16.0;
    }
    let _ = writeln!(
        out,
        "<text x=\"{x}\" y=\"49\" class=\"muted\" text-anchor=\"end\">solid border = required in tier · dashed = optional</text>",
        x = width - MARGIN,
    );

    let mut y = 64.0;
    for (tier, entries) in &bands {
        y += 18.0;
        let _ = writeln!(
            out,
            "<text x=\"{MARGIN}\" y=\"{y}\" class=\"title\" font-size=\"12\">{}</text>",
            tier_title(*tier),
        );
        y += 8.0;
        for (i, (name, required, evidence)) in entries.iter().enumerate() {
            let col = i % GRID_COLS;
            #[expect(
                clippy::integer_division,
                reason = "grid row = index / columns: exact integer arithmetic is the intent"
            )]
            let row = i / GRID_COLS;
            let x = MARGIN + col as f64 * (CELL_W + CELL_GAP);
            let cy = y + row as f64 * (CELL_H + CELL_GAP);
            let (class, glyph, _) = evidence_encoding(*evidence);
            let border = if *required {
                "cell-required"
            } else {
                "cell-optional"
            };
            let _ = writeln!(
                out,
                "<rect x=\"{x}\" y=\"{cy}\" width=\"{CELL_W}\" height=\"{CELL_H}\" rx=\"5\" \
                 class=\"{class} {border}\"/>\
                 <text x=\"{gx}\" y=\"{ty}\" class=\"{lc}\" text-anchor=\"middle\">{glyph}</text>\
                 <text x=\"{nx}\" y=\"{ty}\" class=\"{lc}{sm}\">{name}</text>",
                gx = x + 13.0,
                nx = x + 26.0,
                ty = cy + 17.0,
                lc = label_class(*evidence),
                // Long names step down a font size instead of overflowing.
                sm = if name.chars().count() > 24 { " sm" } else { "" },
            );
        }
        let rows = entries.len().div_ceil(GRID_COLS);
        y += rows as f64 * (CELL_H + CELL_GAP) + 10.0;
    }
    out.push_str("</svg>\n");
    out
}

/// The outcome counts of one taxonomy band — or of a whole chapter, as the
/// sum over its bands.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct BandCounts {
    /// Executed and passed.
    pub passed: u64,
    /// Executed and failed.
    pub failed: u64,
    /// Executed but inconclusive (transport fault / unmapped response).
    pub errored: u64,
    /// NOT executed, with a citation — the two statuses
    /// [`crate::party::OutcomeStatus::needs_citation`] marks
    /// (`not_applicable` + `skipped`). Rendered with a hatched fill so the
    /// segment can never read as an executed pass, nor as a failure.
    pub cited_na: u64,
}

impl BandCounts {
    /// Every recorded outcome of the band.
    #[must_use]
    pub fn total(self) -> u64 {
        self.passed + self.failed + self.errored + self.cited_na
    }

    /// Whether this SUT's run recorded no outcome at all for the band.
    #[must_use]
    pub fn is_empty(self) -> bool {
        self.total() == 0
    }
}

/// One chapter of the two-level taxonomy: the chapter, its rolled-up counts,
/// and its bands in [`TAXONOMY`] declaration order.
///
/// Bands the SUT recorded no outcome for stay in the list with zero counts,
/// so two SUTs' charts compare band-for-band.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChapterRow {
    /// The chapter name (also the taxonomy key).
    pub chapter: &'static str,
    /// The sum over `bands`.
    pub total: BandCounts,
    /// `(band, counts)` in declaration order.
    pub bands: Vec<(&'static str, BandCounts)>,
}

/// Escape a string for SVG text content (XML character data): `&` and `<`
/// break strict XML parsers when emitted raw — "Security & privacy" once
/// shipped an invalid document.
fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// The two-level presentation taxonomy.
///
/// `(chapter, bands)` in the order both
/// charts render them (chapters in this declaration order, bands in theirs —
/// never alphabetical, so a chapter reads in resource/lifecycle order).
///
/// This table is the single source of truth for the chart's structure:
/// [`band_of`] resolves a case id to one of these pairs and
/// [`chapter_counts`] rejects any pair the table does not declare, so the
/// mapping function and the table cannot drift apart silently.
///
/// Chapters follow the openEHR SM platform interfaces
/// (`docs/specs/openehr/SM/docs/openehr_platform/`) where the case ids name an
/// SM interface, and
/// the catalogue's own case-id families otherwise. NOTE: no openEHR spec
/// governs this grouping — our own presentation design.
pub const TAXONOMY: &[(&str, &[&str])] = &[
    (
        "EHR",
        &[
            "EHR resource",
            "EHR_STATUS",
            "COMPOSITION",
            "DIRECTORY",
            "CONTRIBUTION",
            "Item tags",
            "Revision history",
        ],
    ),
    (
        "Definitions",
        &["ADL 1.4 templates", "ADL 2 artefacts", "Stored queries"],
    ),
    ("Query", &["Ad-hoc AQL", "Stored query execution"]),
    (
        "Demographic",
        &["Parties", "Party relationships", "Versioned party"],
    ),
    ("Messaging", &["EHR Extract", "TDD"]),
    ("Admin", &["Admin service", "Archive", "Dump & load"]),
    ("System", &["Conformance manifest"]),
    (
        "Content validation",
        &[
            "Data types",
            "Interval data types",
            "Structure & cardinality",
        ],
    ),
    (
        "Simplified formats",
        &[
            "FLAT & STRUCTURED",
            "Web Template",
            "Path mapping",
            "Scope & legacy media",
        ],
    ),
    (
        "Security & privacy",
        &[
            "Authenticated access",
            "Authorization separation",
            "Audit accountability",
            "Anonymous EHRs",
            "EHR/demographic separation",
        ],
    ),
    ("Signing", &["Version signing"]),
    ("SMART App Launch", &["Discovery", "Resource scopes"]),
    ("Performance", &["Hospital simulation"]),
];

/// A case id the presentation taxonomy does not cover. The renderer fails
/// loudly on one rather than inventing a bucket: an unmapped id is a
/// taxonomy gap to close in [`TAXONOMY`], never a silent `Other`.
#[derive(Debug, thiserror::Error)]
pub enum TaxonomyError {
    /// No `(chapter, band)` pair claims this case id.
    #[error(
        "conformance-assets: case id `{0}` maps to no (chapter, band) pair — \
         extend TAXONOMY + band_of in tools/cnf-runner/src/conf_assets.rs \
         (the taxonomy is total by contract; there is no `Other` bucket)"
    )]
    UnmappedCase(String),
    /// `band_of` produced a pair [`TAXONOMY`] does not declare.
    #[error(
        "conformance-assets: case id `{case}` maps to band `{chapter}` / `{band}`, \
         which TAXONOMY does not declare — the mapping and the table have drifted"
    )]
    UndeclaredBand {
        /// The case id whose mapping is undeclared.
        case: String,
        /// The chapter `band_of` produced.
        chapter: &'static str,
        /// The band `band_of` produced.
        band: &'static str,
    },
}

/// The `(chapter, band)` of an `I_<INTERFACE>.<operation>…` case id.
///
/// One band per SM interface, except where a single interface spans several
/// resources a reader tells apart (`I_DEMOGRAPHIC_SERVICE`) or where the
/// operation is the surface (`I_QUERY_SERVICE`).
fn interface_band(interface: &str, operation: &str) -> Option<(&'static str, &'static str)> {
    let band = match interface {
        "I_EHR_SERVICE" => ("EHR", "EHR resource"),
        "I_EHR_STATUS" => ("EHR", "EHR_STATUS"),
        "I_EHR_COMPOSITION" => ("EHR", "COMPOSITION"),
        "I_EHR_DIRECTORY" => ("EHR", "DIRECTORY"),
        "I_EHR_CONTRIBUTION" => ("EHR", "CONTRIBUTION"),
        // The reserved `I_ITS_REST_*` pseudo-interfaces are released ITS-REST
        // operations the SM models no interface for; they band with the
        // resource whose path they hang off.
        "I_ITS_REST_ITEM_TAGS" => ("EHR", "Item tags"),
        "I_ITS_REST_REVISION_HISTORY" => ("EHR", "Revision history"),
        "I_DEFINITION_ADL14" => ("Definitions", "ADL 1.4 templates"),
        "I_DEFINITION_ADL2" => ("Definitions", "ADL 2 artefacts"),
        "I_DEFINITION_QUERY" => ("Definitions", "Stored queries"),
        "I_QUERY_SERVICE" => match operation {
            "execute_stored_query" => ("Query", "Stored query execution"),
            // `smoke_test` is the master11 stub case, anchored in its own
            // case core to `sm_operation: I_QUERY_SERVICE.execute_ad_hoc_query`.
            "execute_ad_hoc_query" | "smoke_test" => ("Query", "Ad-hoc AQL"),
            _ => return None,
        },
        "I_DEMOGRAPHIC_SERVICE" => {
            if operation.starts_with("versioned_party") {
                ("Demographic", "Versioned party")
            } else if operation.contains("party_relationship") {
                ("Demographic", "Party relationships")
            } else if operation.contains("party") {
                ("Demographic", "Parties")
            } else {
                return None;
            }
        }
        "I_ITS_REST_VERSIONED_PARTY" => ("Demographic", "Versioned party"),
        "I_EHR_EXTRACT_SERVICE" => ("Messaging", "EHR Extract"),
        "I_TDD_SERVICE" => ("Messaging", "TDD"),
        "I_ADMIN_SERVICE" => ("Admin", "Admin service"),
        "I_ADMIN_ARCHIVE" => ("Admin", "Archive"),
        "I_ADMIN_DUMP_LOAD" => ("Admin", "Dump & load"),
        "I_ITS_REST_SYSTEM" => ("System", "Conformance manifest"),
        _ => return None,
    };
    Some(band)
}

/// The `(chapter, band)` of a `<FAMILY>-<TOPIC>-<slug>` case id.
///
/// The topic token is grouped rather than banded one-for-one: the content and
/// simplified-format families enumerate dozens of RM constructs, and a band
/// per construct would be a wall of two-case rows. Grouping also survives a
/// catalogue rename (both the `CONT-OBS` and the retired `CONT-OBSERVATION`
/// spellings land in the same band, so an older committed `results.json`
/// still renders).
fn family_band(family: &str, topic: &str) -> Option<(&'static str, &'static str)> {
    let band = match family {
        "CONT" => {
            if topic.starts_with("DV_INTERVAL_") {
                ("Content validation", "Interval data types")
            } else if topic.starts_with("DV_") {
                ("Content validation", "Data types")
            } else {
                match topic {
                    "COMP" | "COMPOSITION" | "HIST" | "HISTORY" | "EVENT" | "ITEM_STR"
                    | "ITEM_STRUCTURE" | "OBS" | "OBSERVATION" => {
                        ("Content validation", "Structure & cardinality")
                    }
                    _ => return None,
                }
            }
        }
        "SF" => match topic {
            "FLAT" | "STRUCT" | "STRUCTURED" | "CONTRIB" | "EXAMPLE" => {
                ("Simplified formats", "FLAT & STRUCTURED")
            }
            "WT" | "NODEID" | "FIELDID" => ("Simplified formats", "Web Template"),
            "MAP" | "INDEX" | "RMATTR" | "LEVELS" | "RAW" | "CTX" => {
                ("Simplified formats", "Path mapping")
            }
            "SCOPE" | "DEPRECATED" | "LEGACY" => ("Simplified formats", "Scope & legacy media"),
            _ => return None,
        },
        "SEC" => match topic {
            "AUTHENTICATED_ACCESS" => ("Security & privacy", "Authenticated access"),
            "AUTHORIZATION_SEPARATION" => ("Security & privacy", "Authorization separation"),
            "AUDIT_ACCOUNTABILITY" => ("Security & privacy", "Audit accountability"),
            "ANONYMOUS_EHRS" => ("Security & privacy", "Anonymous EHRs"),
            "EHR_DEMOGRAPHIC_SEPARATION" => ("Security & privacy", "EHR/demographic separation"),
            _ => return None,
        },
        "SIG" => match topic {
            "VERSION" => ("Signing", "Version signing"),
            _ => return None,
        },
        // The SMART on openEHR surface (ITS-REST `docs/smart_app_launch`) is
        // its own chapter: a config-gated resource-server posture a reader
        // must be able to tell apart from the openEHR resource chapters.
        "SMART" => match topic {
            "DISCOVERY" => ("SMART App Launch", "Discovery"),
            "RESOURCE_SCOPES" => ("SMART App Launch", "Resource scopes"),
            _ => return None,
        },
        "PERF" => match topic {
            "hospital_sim" => ("Performance", "Hospital simulation"),
            _ => return None,
        },
        _ => return None,
    };
    Some(band)
}

/// The `(chapter, band)` of a case id — total over the committed catalogue.
///
/// Case ids come in two shapes: `I_<INTERFACE>.<operation>[-<slug>]` for the
/// SM (and reserved `I_ITS_REST_*`) interfaces, and `<FAMILY>-<TOPIC>-<slug>`
/// for the prefixed case groups.
///
/// # Errors
///
/// [`TaxonomyError::UnmappedCase`] when no band claims the id — a taxonomy
/// gap to close, never a silent bucket.
pub fn band_of(case_id: &str) -> Result<(&'static str, &'static str), TaxonomyError> {
    // `next()` on a `Split` always yields, so the `unwrap_or` defaults are
    // unreachable and merely keep this panic-free.
    let band = if let Some((interface, rest)) = case_id.split_once('.') {
        let operation = rest.split('-').next().unwrap_or(rest);
        interface_band(interface, operation)
    } else if let Some((family, rest)) = case_id.split_once('-') {
        let topic = rest.split('-').next().unwrap_or(rest);
        family_band(family, topic)
    } else {
        None
    };
    band.ok_or_else(|| TaxonomyError::UnmappedCase(case_id.to_owned()))
}

/// Group the results' outcomes into the full two-level taxonomy: every
/// declared chapter and every declared band, in [`TAXONOMY`] order, with
/// zero counts where this SUT recorded nothing.
///
/// # Errors
///
/// [`TaxonomyError`] on the first case id that maps to no band, or to a band
/// [`TAXONOMY`] does not declare — the renderer refuses to publish a chart
/// whose taxonomy is incomplete.
pub fn chapter_counts(results: &Results) -> Result<Vec<ChapterRow>, TaxonomyError> {
    let mut rows: Vec<ChapterRow> = TAXONOMY
        .iter()
        .map(|(chapter, bands)| ChapterRow {
            chapter,
            total: BandCounts::default(),
            bands: bands
                .iter()
                .map(|band| (*band, BandCounts::default()))
                .collect(),
        })
        .collect();

    for outcome in &results.outcomes {
        let (chapter, band) = band_of(outcome.case.as_str())?;
        let entry = rows
            .iter_mut()
            .find(|row| row.chapter == chapter)
            .and_then(|row| {
                row.bands
                    .iter_mut()
                    .find(|(name, _)| *name == band)
                    .map(|(_, counts)| counts)
            })
            .ok_or_else(|| TaxonomyError::UndeclaredBand {
                case: outcome.case.as_str().to_owned(),
                chapter,
                band,
            })?;
        match outcome.status {
            crate::party::OutcomeStatus::Passed => entry.passed += 1,
            crate::party::OutcomeStatus::Failed => entry.failed += 1,
            crate::party::OutcomeStatus::Errored => entry.errored += 1,
            // Both citation-bearing statuses are one visual class: not
            // executed, with a machine-readable reason.
            crate::party::OutcomeStatus::NotApplicable | crate::party::OutcomeStatus::Skipped => {
                entry.cited_na += 1;
            }
        }
    }

    for row in &mut rows {
        let mut total = BandCounts::default();
        for (_, counts) in &row.bands {
            total.passed += counts.passed;
            total.failed += counts.failed;
            total.errored += counts.errored;
            total.cited_na += counts.cited_na;
        }
        row.total = total;
    }
    Ok(rows)
}

// ---------------------------------------------------------------------------
// The two-level outcome bars
// ---------------------------------------------------------------------------

/// Style additions used only by the outcome bars. Kept out of the shared
/// [`STYLE`] block so the heat grid's bytes do not move when this chart's
/// palette changes.
///
/// The count-strip text colors are darker than the matching bar fills on
/// light backgrounds: a 10.5px glyph needs more contrast against paper than a
/// 12px-tall filled rectangle does.
const BARS_STYLE: &str = "<style>\n\
  .chap-row { fill: #f1efe9; }\n\
  .chap-name { fill: #0b0b0b; font-size: 12.5px; font-weight: 600; }\n\
  .band-label { fill: #52514e; font-size: 11px; }\n\
  .band-empty { fill: #a5a39b; font-size: 10.5px; font-style: italic; }\n\
  .count { font-size: 10.5px; }\n\
  .count-strong { font-weight: 600; }\n\
  .count-passed { fill: #00639b; }\n\
  .count-failed { fill: #c0530b; font-weight: 600; }\n\
  .count-errored { fill: #9a7100; }\n\
  .count-na { fill: #7d7b74; }\n\
  .na-line { stroke: #9d9b93; stroke-width: 2; }\n\
  @media (prefers-color-scheme: dark) {\n\
    .chap-row { fill: #2b2b29; }\n\
    .chap-name { fill: #ffffff; }\n\
    .band-label { fill: #c3c2b7; }\n\
    .band-empty { fill: #6f6e68; }\n\
    .count-passed { fill: #4f97ee; }\n\
    .count-failed { fill: #f2696d; }\n\
    .count-errored { fill: #d9a514; }\n\
    .count-na { fill: #8f8e85; }\n\
    .na-line { stroke: #6d6d69; }\n\
  }\n\
</style>\n";

/// The cited-N/A hatch: a diagonal ruling over the neutral fill, so a
/// not-executed segment is legible as neither an executed pass nor a
/// failure — the third state gets its own texture, not just a grey.
const NA_DEFS: &str = "<defs>\n\
  <pattern id=\"na-hatch\" width=\"6\" height=\"6\" patternUnits=\"userSpaceOnUse\" \
patternTransform=\"rotate(45)\">\n\
    <rect width=\"6\" height=\"6\" class=\"bar-na\"/>\n\
    <line x1=\"3\" y1=\"0\" x2=\"3\" y2=\"6\" class=\"na-line\"/>\n\
  </pattern>\n\
</defs>\n";

/// The label column (chapter names left-aligned in it, band names
/// right-aligned against the bar origin).
const LABEL_W: f64 = 206.0;
/// The widest a band bar may draw (the largest band in the chart).
const BAR_MAX_W: f64 = 470.0;
/// One column of the printed count strip.
const SLOT_W: f64 = 42.0;
/// The chapter header row's pitch.
const CHAP_H: f64 = 24.0;
/// A band row's pitch.
const BAND_H: f64 = 18.0;
/// The drawn height of a band bar (centred in its row).
const BAND_BAR_H: f64 = 12.0;
/// Breathing room after each chapter block.
const CHAP_GAP: f64 = 8.0;
/// Title + legend block above the first chapter.
const BARS_HEAD_H: f64 = 66.0;

/// Where the bars start.
const BAR_X: f64 = MARGIN + LABEL_W;
/// Where the printed count strip starts.
const COUNTS_X: f64 = BAR_X + BAR_MAX_W + 16.0;
/// The chart width — pinned by the column layout, and unchanged from the
/// single-level chart so the pages embedding the SVG keep their contract.
const BARS_W: f64 = COUNTS_X + 4.0 * SLOT_W + MARGIN;

/// The four outcome kinds in stacking (and count-strip) order: the bar fill
/// class, the count text class, the glyph (the non-color channel), and the
/// legend label.
const OUTCOMES: [(&str, &str, &str, &str); 4] = [
    ("bar-passed", "count-passed", "✓", "passed"),
    ("bar-failed", "count-failed", "✕", "FAILED"),
    ("bar-errored", "count-errored", "?", "errored"),
    ("bar-na", "count-na", "○", "cited N/A (not executed)"),
];

/// The counts in [`OUTCOMES`] order.
fn outcome_values(counts: BandCounts) -> [u64; 4] {
    [
        counts.passed,
        counts.failed,
        counts.errored,
        counts.cited_na,
    ]
}

/// The exact counts, right-aligned in four fixed slots so every row's numbers
/// line up as a table. A zero prints nothing — its empty slot reads as zero
/// and keeps the column alignment. Numbers are printed for EVERY band,
/// however short its bar, so a one-case band never loses its count.
fn write_count_strip(out: &mut String, baseline: f64, counts: BandCounts, strong: bool) {
    let weight = if strong { " count-strong" } else { "" };
    for (slot, (value, (_, text_class, glyph, _))) in
        outcome_values(counts).into_iter().zip(OUTCOMES).enumerate()
    {
        if value == 0 {
            continue;
        }
        #[expect(
            clippy::as_conversions,
            clippy::cast_precision_loss,
            reason = "four slots"
        )]
        let slot_end = COUNTS_X + (slot as f64 + 1.0) * SLOT_W - 6.0;
        let _ = write!(
            out,
            "<text x=\"{slot_end}\" y=\"{baseline}\" class=\"count {text_class}{weight}\" \
             text-anchor=\"end\">{glyph} {value}</text>"
        );
    }
}

/// `n cases` / `1 case` / `no cases` — the row's headline number in words a
/// reader does not have to decode.
fn cases_phrase(total: u64) -> String {
    match total {
        0 => "no cases".to_owned(),
        1 => "1 case".to_owned(),
        n => format!("{n} cases"),
    }
}

/// The two-level outcome bars: a tinted chapter header carrying the chapter's
/// total and rolled-up counts, expanded into one scaled bar per band.
///
/// Every declared band renders — a band with no case for this SUT becomes an
/// explicit "no cases" row rather than vanishing — so the ferroehr and
/// ehrbase charts read band-for-band.
///
/// Pure over its inputs: [`TAXONOMY`] order, no timestamps, no randomness.
#[must_use]
#[expect(
    clippy::as_conversions,
    clippy::too_many_lines,
    clippy::cast_precision_loss,
    reason = "one linear chart emitter; counts/rows << 2^52"
)]
pub fn chapter_bars_svg(sut_label: &str, chapters: &[ChapterRow]) -> String {
    // All bands share ONE scale, and it is DYNAMIC: the widest band in THIS chart
    // is the full bar, so two SUTs compare band-for-band by position and by the
    // printed counts, not by bar length — the legend states the scale in force.
    //
    // Chapter rows carry NO bar: at 3-4x the widest band they would clip or force a
    // second scale into one picture; a chapter's magnitude is its printed total.
    let max_band: u64 = chapters
        .iter()
        .flat_map(|row| row.bands.iter().map(|(_, counts)| counts.total()))
        .max()
        .unwrap_or(1)
        .max(1);
    let scale = BAR_MAX_W / max_band as f64;

    let band_rows: usize = chapters.iter().map(|row| row.bands.len()).sum();
    let height =
        BARS_HEAD_H + chapters.len() as f64 * (CHAP_H + CHAP_GAP) + band_rows as f64 * BAND_H + 8.0;

    let mut out = String::new();
    svg_open(&mut out, BARS_W, height);
    out.push_str(BARS_STYLE);
    out.push_str(NA_DEFS);
    let _ = writeln!(
        out,
        "<text x=\"{MARGIN}\" y=\"28\" class=\"title\">Schedule outcomes by chapter — {}</text>",
        xml_escape(sut_label)
    );

    // Legend: swatch + glyph + label per outcome (color AND glyph, never
    // color alone), then the scale note.
    let mut lx = MARGIN;
    for (fill_class, _, glyph, label) in OUTCOMES {
        // Light glyph on the saturated fills, dark on the pale ones — the
        // heat grid's convention.
        let (fill, glyph_class) = match fill_class {
            "bar-na" => ("fill=\"url(#na-hatch)\"".to_owned(), "seg-label-dim"),
            "bar-errored" => (format!("class=\"{fill_class}\""), "seg-label-dim"),
            _ => (format!("class=\"{fill_class}\""), "seg-label"),
        };
        let _ = write!(
            out,
            "<rect x=\"{lx}\" y=\"38\" width=\"14\" height=\"14\" rx=\"3\" {fill}/>\
             <text x=\"{gx}\" y=\"49\" class=\"{glyph_class}\" \
             text-anchor=\"middle\">{glyph}</text>\
             <text x=\"{tx}\" y=\"49\" class=\"muted\">{label}</text>",
            gx = lx + 7.0,
            tx = lx + 20.0,
        );
        lx += 20.0 + 7.2 * label.chars().count() as f64 + 16.0;
    }
    let _ = writeln!(
        out,
        "<text x=\"{x}\" y=\"49\" class=\"muted\" text-anchor=\"end\">\
         this chart's scale: {max_band} cases = full bar</text>",
        x = BARS_W - MARGIN,
    );

    let mut y = BARS_HEAD_H;
    for row in chapters {
        // Chapter header: tinted full-width band, name, total, bold counts.
        let _ = writeln!(
            out,
            "<rect x=\"{MARGIN}\" y=\"{y}\" width=\"{w}\" height=\"20\" rx=\"4\" \
             class=\"chap-row\"/>\
             <text x=\"{nx}\" y=\"{ty}\" class=\"chap-name\">{name}</text>\
             <text x=\"{tx}\" y=\"{ty}\" class=\"muted\" text-anchor=\"end\">{phrase}</text>",
            w = BARS_W - 2.0 * MARGIN,
            nx = MARGIN + 10.0,
            tx = BAR_X + BAR_MAX_W,
            ty = y + 14.5,
            name = xml_escape(row.chapter),
            phrase = cases_phrase(row.total.total()),
        );
        write_count_strip(&mut out, y + 14.5, row.total, true);
        out.push('\n');
        y += CHAP_H;

        for (band, counts) in &row.bands {
            let baseline = y + 12.5;
            let _ = write!(
                out,
                "<text x=\"{label_x}\" y=\"{baseline}\" class=\"band-label\" \
                 text-anchor=\"end\">{name}</text>",
                label_x = BAR_X - 10.0,
                name = xml_escape(band),
            );
            if counts.is_empty() {
                // An explicit zero row: the band exists in the taxonomy, this
                // SUT's run recorded nothing for it.
                let _ = writeln!(
                    out,
                    "<text x=\"{BAR_X}\" y=\"{baseline}\" class=\"band-empty\">no cases</text>"
                );
            } else {
                let mut x = BAR_X;
                for (value, (fill_class, _, _, _)) in
                    outcome_values(*counts).into_iter().zip(OUTCOMES)
                {
                    if value == 0 {
                        continue;
                    }
                    let w = value as f64 * scale;
                    let fill = if fill_class == "bar-na" {
                        "fill=\"url(#na-hatch)\"".to_owned()
                    } else {
                        format!("class=\"{fill_class}\"")
                    };
                    let _ = write!(
                        out,
                        "<rect x=\"{x}\" y=\"{by}\" width=\"{w}\" height=\"{BAND_BAR_H}\" {fill}/>",
                        by = y + (BAND_H - BAND_BAR_H) / 2.0,
                    );
                    x += w;
                }
                write_count_strip(&mut out, baseline, *counts, false);
                out.push('\n');
            }
            y += BAND_H;
        }
        y += CHAP_GAP;
    }
    out.push_str("</svg>\n");
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::party::OutcomeStatus;

    fn matrix() -> CapabilityMatrix {
        serde_saphyr::from_str(
            "EhrOperations: { family: Platform, tier: CORE, required: true, source: s }\n\
             AqlBasic: { family: Platform, tier: STANDARD, required: true, source: s }\n\
             EhrExtract: { family: Platform, tier: OPTIONS, required: false, source: s }\n\
             AuthenticatedAccess: { family: Security, tier: SEC-BASIC, required: true, source: s }\n",
        )
        .unwrap()
    }

    #[test]
    fn the_heat_grid_is_deterministic_and_encodes_twice() {
        let m = matrix();
        let caps = vec![
            ("EhrOperations".to_owned(), Evidence::Passed),
            ("AqlBasic".to_owned(), Evidence::Failed),
            ("AuthenticatedAccess".to_owned(), Evidence::NotEvidenced),
        ];
        let a = heat_grid_svg("FerroEHR 3.7.0", &m, &caps);
        let b = heat_grid_svg("FerroEHR 3.7.0", &m, &caps);
        assert_eq!(a, b);
        // Both encodings present: the fill class AND the glyph.
        assert!(a.contains("ev-passed"));
        assert!(a.contains('✓'));
        assert!(a.contains("ev-failed"));
        assert!(a.contains('✕'));
        // The unmapped capability renders as not-evidenced with its glyph
        // (the former no-cases state — variant deleted, #626).
        assert!(a.contains("ev-not-evidenced"));
        assert!(a.contains('○'));
        // The LEGEND carries every evidence state the grid can render —
        // including inconclusive, which no cell in this sample shows (the
        // legend enumerates the vocabulary, not the sample; its omission
        // shipped once, 2026-07-29).
        for label in ["passed", "FAILED", "INCONCLUSIVE", "not evidenced"] {
            assert!(a.contains(label), "legend label {label} missing");
        }
        assert!(a.contains("ev-inconclusive"));
        assert!(a.contains('?'));
        // Tier bands + border encodings.
        for band in ["CORE", "STANDARD", "OPTIONS", "SEC-BASIC"] {
            assert!(a.contains(band), "band {band} missing");
        }
        assert!(a.contains("cell-required"));
        assert!(a.contains("cell-optional"));
        assert!(a.contains("prefers-color-scheme: dark"));
        assert!(!a.contains("Date"));
    }

    fn crate_dir() -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    }

    /// Every `id:` declared by the committed case cores.
    fn committed_case_ids() -> Vec<String> {
        fn walk(dir: &std::path::Path, out: &mut Vec<String>) {
            for entry in std::fs::read_dir(dir).unwrap() {
                let path = entry.unwrap().path();
                if path.is_dir() {
                    walk(&path, out);
                } else if path.extension().is_some_and(|e| e == "yaml") {
                    let text = std::fs::read_to_string(&path).unwrap();
                    for line in text.lines() {
                        if let Some(id) = line.strip_prefix("id: ") {
                            out.push(id.trim().to_owned());
                            break;
                        }
                    }
                }
            }
        }
        let mut ids = Vec::new();
        walk(&crate_dir().join("artifacts/schedule"), &mut ids);
        assert!(ids.len() > 500, "case-core walk found only {}", ids.len());
        ids.sort();
        ids
    }

    /// Every case id appearing in a committed party `results.json` — the
    /// charts render from those, and they can carry retired ids a catalogue
    /// rename left behind.
    fn committed_result_ids() -> Vec<String> {
        #[derive(serde::Deserialize)]
        struct Slice {
            outcomes: Vec<Outcome>,
        }
        #[derive(serde::Deserialize)]
        struct Outcome {
            case: String,
        }
        let conformance = crate_dir().join("../../docs/conformance");
        let mut ids = Vec::new();
        for entry in std::fs::read_dir(&conformance).unwrap() {
            let results = entry.unwrap().path().join("results.json");
            if !results.is_file() {
                continue;
            }
            let text = std::fs::read_to_string(&results).unwrap();
            let slice: Slice = serde_json::from_str(&text).unwrap();
            ids.extend(slice.outcomes.into_iter().map(|o| o.case));
        }
        assert!(
            ids.len() > 500,
            "results walk found only {} outcomes",
            ids.len()
        );
        ids.sort();
        ids.dedup();
        ids
    }

    /// The never-`Other` ratchet, in test form: the taxonomy is TOTAL over
    /// every committed case id — both the catalogue's case cores and the
    /// published party results — and every band it produces is declared.
    #[test]
    fn the_taxonomy_is_total_over_every_committed_case_id() {
        let declared: Vec<(&str, &str)> = TAXONOMY
            .iter()
            .flat_map(|(chapter, bands)| bands.iter().map(move |band| (*chapter, *band)))
            .collect();
        let mut unmapped: Vec<String> = Vec::new();
        for id in committed_case_ids()
            .into_iter()
            .chain(committed_result_ids())
        {
            let mapped = band_of(&id).ok();
            match mapped {
                Some(band) => assert!(
                    declared.contains(&band),
                    "case `{id}` maps to undeclared band {band:?}"
                ),
                None => unmapped.push(id),
            }
        }
        assert!(
            unmapped.is_empty(),
            "these case ids map to no band (the taxonomy must be total — \
             there is no `Other` bucket): {unmapped:#?}"
        );
    }

    /// The declared taxonomy has no duplicate chapter or band names — the
    /// count aggregation resolves a band by name.
    #[test]
    fn the_taxonomy_names_are_unique() {
        let mut chapters: Vec<&str> = TAXONOMY.iter().map(|(chapter, _)| *chapter).collect();
        let before = chapters.len();
        chapters.sort_unstable();
        chapters.dedup();
        assert_eq!(chapters.len(), before, "duplicate chapter name");
        for (chapter, bands) in TAXONOMY {
            let mut names: Vec<&str> = bands.to_vec();
            let before = names.len();
            names.sort_unstable();
            names.dedup();
            assert_eq!(names.len(), before, "duplicate band name in {chapter}");
        }
    }

    #[test]
    fn the_two_levels_come_from_the_case_id() {
        assert_eq!(
            band_of("I_EHR_COMPOSITION.create_composition-event").unwrap(),
            ("EHR", "COMPOSITION")
        );
        assert_eq!(
            band_of("I_EHR_SERVICE.create_ehr").unwrap(),
            ("EHR", "EHR resource")
        );
        assert_eq!(
            band_of("I_ITS_REST_ITEM_TAGS.upsert_tags-x").unwrap(),
            ("EHR", "Item tags")
        );
        assert_eq!(
            band_of("I_ITS_REST_REVISION_HISTORY.get-x").unwrap(),
            ("EHR", "Revision history")
        );
        // One interface, three reader-visible resources.
        assert_eq!(
            band_of("I_DEMOGRAPHIC_SERVICE.create_party-x").unwrap(),
            ("Demographic", "Parties")
        );
        assert_eq!(
            band_of("I_DEMOGRAPHIC_SERVICE.get_party_relationship-x").unwrap(),
            ("Demographic", "Party relationships")
        );
        assert_eq!(
            band_of("I_DEMOGRAPHIC_SERVICE.versioned_party_version_read").unwrap(),
            ("Demographic", "Versioned party")
        );
        assert_eq!(
            band_of("I_ITS_REST_VERSIONED_PARTY.get-x").unwrap(),
            ("Demographic", "Versioned party")
        );
        // The query surface bands by operation; the master11 stub anchors to
        // execute_ad_hoc_query in its own case core.
        assert_eq!(
            band_of("I_QUERY_SERVICE.execute_stored_query-x").unwrap(),
            ("Query", "Stored query execution")
        );
        assert_eq!(
            band_of("I_QUERY_SERVICE.smoke_test").unwrap(),
            ("Query", "Ad-hoc AQL")
        );
        // The System API has a real chapter now — never an `Other` bucket.
        assert_eq!(
            band_of("I_ITS_REST_SYSTEM.get_conformance-options").unwrap(),
            ("System", "Conformance manifest")
        );
        assert_eq!(
            band_of("I_EHR_EXTRACT_SERVICE.request-x").unwrap(),
            ("Messaging", "EHR Extract")
        );
        assert_eq!(
            band_of("I_TDD_SERVICE.convert-x").unwrap(),
            ("Messaging", "TDD")
        );
        assert_eq!(
            band_of("I_ADMIN_DUMP_LOAD.export_ehrs-export_all").unwrap(),
            ("Admin", "Dump & load")
        );
        // Prefixed families: the topic token groups.
        assert_eq!(
            band_of("CONT-DV_TEXT-validate_open").unwrap(),
            ("Content validation", "Data types")
        );
        assert_eq!(
            band_of("CONT-DV_INTERVAL_DV_COUNT-validate_open").unwrap(),
            ("Content validation", "Interval data types")
        );
        // Both the current and the retired structure spellings land together,
        // so an older committed results.json still renders.
        for id in [
            "CONT-OBS-state_ex_opt-protocol_ex_opt",
            "CONT-OBSERVATION-state_protocol_existence",
            "CONT-ITEM_STR-type_any",
            "CONT-ITEM_STRUCTURE-type_narrowing",
        ] {
            assert_eq!(
                band_of(id).unwrap(),
                ("Content validation", "Structure & cardinality"),
                "{id}"
            );
        }
        assert_eq!(
            band_of("SF-FLAT-commit_roundtrip_ctx_defaults").unwrap(),
            ("Simplified formats", "FLAT & STRUCTURED")
        );
        assert_eq!(
            band_of("SF-WT-web_template_get").unwrap(),
            ("Simplified formats", "Web Template")
        );
        assert_eq!(
            band_of("SEC-AUDIT_ACCOUNTABILITY-server_set_commit_audit").unwrap(),
            ("Security & privacy", "Audit accountability")
        );
        assert_eq!(
            band_of("SIG-VERSION-verifiable").unwrap(),
            ("Signing", "Version signing")
        );
        assert_eq!(
            band_of("SMART-DISCOVERY-document_shape").unwrap(),
            ("SMART App Launch", "Discovery")
        );
        assert_eq!(
            band_of("PERF-hospital_sim-class_POC").unwrap(),
            ("Performance", "Hospital simulation")
        );
    }

    #[test]
    fn an_unmapped_case_id_fails_the_render() {
        for id in [
            "SOMETHING-else-entirely",
            "I_NOT_AN_INTERFACE.do_something",
            "CONT-NOT_A_CONSTRUCT-validate",
            "I_DEMOGRAPHIC_SERVICE.list_widgets",
            "bare_id",
        ] {
            let err = band_of(id).unwrap_err();
            assert!(
                matches!(&err, TaxonomyError::UnmappedCase(seen) if seen == id),
                "{id} produced {err:?}"
            );
            // The error names the offending id, so the fix is obvious.
            assert!(err.to_string().contains(id));
        }

        let results = results_with(&[("SOMETHING-else-entirely", OutcomeStatus::Passed)]);
        let err = chapter_counts(&results).unwrap_err();
        assert!(matches!(err, TaxonomyError::UnmappedCase(_)));
    }

    /// A minimal [`Results`] carrying just the outcomes the chart reads.
    fn results_with(outcomes: &[(&str, OutcomeStatus)]) -> Results {
        Results {
            sut: crate::party::Sut {
                name: "sut".to_owned(),
                version: "0".to_owned(),
            },
            runner: crate::party::Runner {
                name: "cnf-runner".to_owned(),
                version: "0".to_owned(),
                verification_pack_status: crate::party::VerificationPackStatus::Passed,
            },
            schedule_release: "test".to_owned(),
            tech_profile: crate::party::TechProfile {
                its: crate::vocab::ItsName::ItsRest,
                formats: Vec::new(),
            },
            ixit_digest: "test".to_owned(),
            restapi_specs_version: None,
            outcomes: outcomes
                .iter()
                .map(|(case, status)| crate::party::OutcomeRecord {
                    case: crate::ids::CaseId::parse(case).unwrap(),
                    format: None,
                    status: *status,
                    rows_driven: 1,
                    rows_total: 1,
                    failing_step: None,
                    reason: None,
                    citation: None,
                    failed_rows: Vec::new(),
                })
                .collect(),
            measurements: Vec::new(),
            ambiguity_dispositions: Vec::new(),
        }
    }

    #[test]
    fn the_counts_aggregate_per_band_and_roll_up_per_chapter() {
        let rows = chapter_counts(&results_with(&[
            (
                "I_EHR_COMPOSITION.create_composition-a",
                OutcomeStatus::Passed,
            ),
            (
                "I_EHR_COMPOSITION.create_composition-b",
                OutcomeStatus::Passed,
            ),
            (
                "I_EHR_COMPOSITION.delete_composition-c",
                OutcomeStatus::Failed,
            ),
            ("I_EHR_STATUS.get_ehr_status-a", OutcomeStatus::Errored),
            (
                "I_ITS_REST_ITEM_TAGS.list_tags-a",
                OutcomeStatus::NotApplicable,
            ),
            ("I_ITS_REST_ITEM_TAGS.list_tags-b", OutcomeStatus::Skipped),
            ("SIG-VERSION-verifiable", OutcomeStatus::Passed),
        ]))
        .unwrap();

        // Every declared chapter and band is present, in declaration order.
        assert_eq!(rows.len(), TAXONOMY.len());
        for (row, (chapter, bands)) in rows.iter().zip(TAXONOMY) {
            assert_eq!(row.chapter, *chapter);
            let names: Vec<&str> = row.bands.iter().map(|(name, _)| *name).collect();
            assert_eq!(names, bands.to_vec());
        }

        let ehr = rows.iter().find(|r| r.chapter == "EHR").unwrap();
        let band = |name: &str| {
            ehr.bands
                .iter()
                .find(|(b, _)| *b == name)
                .map(|(_, c)| *c)
                .unwrap()
        };
        assert_eq!(
            band("COMPOSITION"),
            BandCounts {
                passed: 2,
                failed: 1,
                ..BandCounts::default()
            }
        );
        assert_eq!(
            band("EHR_STATUS"),
            BandCounts {
                errored: 1,
                ..BandCounts::default()
            }
        );
        // Both citation-bearing statuses land in the one cited-N/A column.
        assert_eq!(
            band("Item tags"),
            BandCounts {
                cited_na: 2,
                ..BandCounts::default()
            }
        );
        assert!(band("DIRECTORY").is_empty());
        assert_eq!(
            ehr.total,
            BandCounts {
                passed: 2,
                failed: 1,
                errored: 1,
                cited_na: 2
            }
        );
        assert_eq!(ehr.total.total(), 6);

        let signing = rows.iter().find(|r| r.chapter == "Signing").unwrap();
        assert_eq!(signing.total.passed, 1);
        // A chapter nothing exercised still renders, zeroed.
        let perf = rows.iter().find(|r| r.chapter == "Performance").unwrap();
        assert!(perf.total.is_empty());
    }

    #[test]
    fn the_chapter_bars_render_two_levels_with_printed_counts() {
        let rows = chapter_counts(&results_with(&[
            (
                "I_EHR_COMPOSITION.create_composition-a",
                OutcomeStatus::Passed,
            ),
            (
                "I_EHR_COMPOSITION.delete_composition-b",
                OutcomeStatus::Failed,
            ),
            ("I_EHR_DIRECTORY.get_folder-a", OutcomeStatus::Errored),
            (
                "SF-FLAT-commit_roundtrip_ctx_defaults",
                OutcomeStatus::NotApplicable,
            ),
        ]))
        .unwrap();
        let svg = chapter_bars_svg("FerroEHR 3.7.0", &rows);
        assert_eq!(svg, chapter_bars_svg("FerroEHR 3.7.0", &rows));

        // Both levels are drawn: every chapter header and every band label.
        for (chapter, bands) in TAXONOMY {
            assert!(
                svg.contains(&xml_escape(chapter)),
                "chapter {chapter} missing"
            );
            for band in *bands {
                assert!(svg.contains(&xml_escape(band)), "band {band} missing");
            }
        }
        assert!(svg.contains("chap-row"));
        // Counts are printed per band, with the glyph as the second channel.
        assert!(svg.contains("✓ 1"));
        assert!(svg.contains("✕ 1"));
        assert!(svg.contains("? 1"));
        assert!(svg.contains("○ 1"));
        // A chapter's rolled-up headline.
        assert!(svg.contains(">3 cases<"));
        assert!(svg.contains(">1 case<"));
        // Bands with nothing recorded render as explicit zero rows.
        assert!(svg.contains(">no cases<"));
        // The cited-N/A segment carries its own texture, so it reads as
        // neither an executed pass nor a failure.
        assert!(svg.contains("id=\"na-hatch\""));
        assert!(svg.contains("url(#na-hatch)"));
        // Light + dark palettes both present, no clock in the output.
        assert!(svg.contains("prefers-color-scheme: dark"));
        assert!(!svg.contains("Date"));
    }

    #[test]
    fn the_chart_keeps_its_embedded_width_and_grows_only_downwards() {
        // Geometry invariants over constants — compile-time (const items,
        // declared before any statement): the canvas width is pinned and
        // nothing draws past the right margin.
        const _: () = assert!(BARS_W.to_bits() == 908.0f64.to_bits());
        const _: () = assert!(COUNTS_X + 4.0 * SLOT_W <= BARS_W - MARGIN);
        const _: () = assert!(BAR_X + BAR_MAX_W < COUNTS_X);

        let rows = chapter_counts(&results_with(&[(
            "I_EHR_COMPOSITION.create_composition-a",
            OutcomeStatus::Passed,
        )]))
        .unwrap();
        let svg = chapter_bars_svg("FerroEHR 3.7.0", &rows);
        // The book + landing pages embed the SVG at this width; the two-level
        // layout is allowed to grow taller, never wider.
        let bands: usize = TAXONOMY.iter().map(|(_, b)| b.len()).sum();
        #[expect(
            clippy::as_conversions,
            clippy::cast_precision_loss,
            reason = "taxonomy rows << 2^52"
        )]
        let height =
            BARS_HEAD_H + TAXONOMY.len() as f64 * (CHAP_H + CHAP_GAP) + bands as f64 * BAND_H + 8.0;
        assert!(
            svg.contains(&format!("viewBox=\"0 0 908 {height}\" width=\"908\"")),
            "unexpected canvas: {}",
            svg.lines().next().unwrap_or_default()
        );
    }

    #[test]
    fn svg_text_is_xml_escaped() {
        // "Security & privacy" once shipped a raw ampersand — invalid XML
        // strict renderers refuse. Both emitters escape text content.
        let rows = chapter_counts(&results_with(&[])).unwrap();
        let bars = chapter_bars_svg("sut & co", &rows);
        for raw in [
            "Security & privacy",
            "Dump & load",
            "FLAT & STRUCTURED",
            "Structure & cardinality",
            "Scope & legacy media",
            "sut & co",
        ] {
            assert!(!bars.contains(raw), "raw ampersand in {raw}");
            assert!(bars.contains(&xml_escape(raw)), "missing escaped {raw}");
        }
    }
}
