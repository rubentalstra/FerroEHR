//! Deterministic conformance SVG assets rendered FROM the committed party
//! artifacts (`verdicts.json` + `results.json` + the capability matrix) —
//! the `perf_assets` pattern applied to functional conformance: no
//! hand-drawn numbers, CI-guarded regeneration, light+dark via
//! `prefers-color-scheme`, evidence encoded twice (a CVD-safe fill AND a
//! glyph — never color alone).
//!
//! Two charts:
//! - the **capability heat grid** — one cell per capability, grouped by
//!   tier (CORE → STANDARD → OPTIONS → SEC-BASIC), the whole conformance
//!   story in one picture;
//! - the **per-chapter outcome bars** — passed/failed/errored/N-A per
//!   schedule chapter, counts printed on the segments.

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
  .ev-not-evidenced { fill: #cbc9c2; }\n\
  .ev-unrealized { fill: #e8e6e0; }\n\
  .ev-no-cases { fill: #f4f2ec; }\n\
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
    .ev-not-evidenced { fill: #4a4a47; }\n\
    .ev-unrealized { fill: #3a3a38; }\n\
    .ev-no-cases { fill: #2e2e2c; }\n\
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
        Evidence::NotEvidenced => ("ev-not-evidenced", "○", "not evidenced"),
        Evidence::Unrealized => ("ev-unrealized", "◇", "excused (unrealized)"),
        Evidence::NoCases => ("ev-no-cases", "∅", "no cases"),
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
#[allow(
    clippy::too_many_lines,
    clippy::items_after_statements,
    clippy::cast_precision_loss
)] // one linear chart emitter; counts/cells << 2^52
pub fn heat_grid_svg(
    sut_label: &str,
    matrix: &CapabilityMatrix,
    capabilities: &[(String, Evidence)],
) -> String {
    let evidence_of = |name: &str| {
        capabilities
            .iter()
            .find(|(n, _)| n == name)
            .map_or(Evidence::NoCases, |(_, e)| *e)
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
        "<text x=\"{MARGIN}\" y=\"28\" class=\"title\">Capability conformance — {sut_label}</text>"
    );
    // Legend (glyph + label per evidence kind; swatch + glyph = both channels).
    let mut lx = MARGIN;
    for evidence in [
        Evidence::Passed,
        Evidence::Failed,
        Evidence::NotEvidenced,
        Evidence::Unrealized,
        Evidence::NoCases,
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

/// A schedule chapter and its outcome counts.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ChapterCounts {
    pub passed: u64,
    pub failed: u64,
    pub errored: u64,
    pub not_applicable: u64,
}

/// The schedule chapter of a case id — the SM component of an
/// `I_<INTERFACE>.*` case (the `docs/architecture.md` SM component map),
/// or the schedule family of the prefixed case groups.
#[must_use]
pub fn chapter_of(case_id: &str) -> &'static str {
    let interface = case_id.split('.').next().unwrap_or(case_id);
    match interface {
        i if i.starts_with("I_EHR_EXTRACT") || i.starts_with("I_TDD") => "Messaging",
        i if i.starts_with("I_EHR") => "EHR",
        i if i.starts_with("I_DEFINITION") => "Definitions",
        i if i.starts_with("I_QUERY") => "Query",
        i if i.starts_with("I_DEMOGRAPHIC") => "Demographic",
        i if i.starts_with("I_ADMIN") => "Admin",
        i if i.starts_with("CONT-") => "Content validation",
        i if i.starts_with("SF-") => "Simplified formats",
        i if i.starts_with("SEC-") => "Security & privacy",
        i if i.starts_with("SIG-") => "Signing",
        i if i.starts_with("PERF-") => "Performance",
        _ => "Other",
    }
}

/// Chapter presentation order (fixed, so the chart is stable regardless of
/// outcome order in the artifact).
const CHAPTER_ORDER: [&str; 11] = [
    "EHR",
    "Definitions",
    "Query",
    "Demographic",
    "Admin",
    "Messaging",
    "Content validation",
    "Simplified formats",
    "Security & privacy",
    "Signing",
    "Other",
];

/// Group the results' outcomes into chapter counts (fixed chapter order,
/// absent chapters omitted).
#[must_use]
pub fn chapter_counts(results: &Results) -> Vec<(&'static str, ChapterCounts)> {
    let mut counts: Vec<(&'static str, ChapterCounts)> = Vec::new();
    for outcome in &results.outcomes {
        let chapter = chapter_of(outcome.case.as_str());
        let entry = if let Some(i) = counts.iter().position(|(c, _)| *c == chapter) {
            &mut counts[i].1
        } else {
            counts.push((chapter, ChapterCounts::default()));
            let last = counts.len() - 1;
            &mut counts[last].1
        };
        match outcome.status {
            crate::party::OutcomeStatus::Passed => entry.passed += 1,
            crate::party::OutcomeStatus::Failed => entry.failed += 1,
            crate::party::OutcomeStatus::Errored => entry.errored += 1,
            crate::party::OutcomeStatus::NotApplicable | crate::party::OutcomeStatus::Skipped => {
                entry.not_applicable += 1;
            }
        }
    }
    counts.sort_by_key(|(chapter, _)| {
        CHAPTER_ORDER
            .iter()
            .position(|c| c == chapter)
            .unwrap_or(usize::MAX)
    });
    counts
}

/// The per-chapter outcome bars: one horizontal stacked bar per chapter
/// (passed / failed / errored / N-A), segment counts printed on (or beside)
/// the segments, totals at the row end.
#[must_use]
#[allow(
    clippy::too_many_lines,
    clippy::items_after_statements,
    clippy::cast_precision_loss
)] // one linear chart emitter; counts/cells << 2^52
pub fn chapter_bars_svg(sut_label: &str, chapters: &[(&str, ChapterCounts)]) -> String {
    const LABEL_W: f64 = 150.0;
    const BAR_MAX_W: f64 = 640.0;
    const ROW_H: f64 = 26.0;
    const ROW_GAP: f64 = 8.0;

    let max_total: u64 = chapters
        .iter()
        .map(|(_, c)| c.passed + c.failed + c.errored + c.not_applicable)
        .max()
        .unwrap_or(1)
        .max(1);

    let width = MARGIN * 2.0 + LABEL_W + BAR_MAX_W + 70.0;
    let height = 64.0 + chapters.len() as f64 * (ROW_H + ROW_GAP) + 8.0;

    let mut out = String::new();
    svg_open(&mut out, width, height);
    let _ = writeln!(
        out,
        "<text x=\"{MARGIN}\" y=\"28\" class=\"title\">Schedule outcomes by chapter — {sut_label}</text>\n"
    );
    // Legend.
    let mut lx = MARGIN;
    for (class, label) in [
        ("bar-passed", "passed"),
        ("bar-failed", "failed"),
        ("bar-errored", "errored"),
        ("bar-na", "N/A (cited)"),
    ] {
        let _ = write!(
            out,
            "<rect x=\"{lx}\" y=\"38\" width=\"14\" height=\"14\" rx=\"3\" class=\"{class}\"/>\
             <text x=\"{tx}\" y=\"49\" class=\"muted\">{label}</text>",
            tx = lx + 18.0,
        );
        lx += 20.0 + 7.2 * label.chars().count() as f64 + 16.0;
    }

    let mut y = 64.0;
    for (chapter, counts) in chapters {
        let total = counts.passed + counts.failed + counts.errored + counts.not_applicable;
        let _ = writeln!(
            out,
            "<text x=\"{x}\" y=\"{ty}\" text-anchor=\"end\">{chapter}</text>",
            x = MARGIN + LABEL_W - 8.0,
            ty = y + 17.0,
        );
        let mut x = MARGIN + LABEL_W;
        #[allow(clippy::cast_precision_loss)] // case counts << 2^52
        let scale = BAR_MAX_W / max_total as f64;
        for (value, class, label_class_name) in [
            (counts.passed, "bar-passed", "seg-label"),
            (counts.failed, "bar-failed", "seg-label"),
            (counts.errored, "bar-errored", "seg-label"),
            (counts.not_applicable, "bar-na", "seg-label-dim"),
        ] {
            if value == 0 {
                continue;
            }
            #[allow(clippy::cast_precision_loss)] // case counts << 2^52
            let w = value as f64 * scale;
            let _ = writeln!(
                out,
                "<rect x=\"{x}\" y=\"{y}\" width=\"{w}\" height=\"{ROW_H}\" class=\"{class}\"/>\n"
            );
            // The count: on the segment when it fits, beside the bar's end
            // otherwise (small segments never lose their number).
            if w >= 22.0 {
                let _ = write!(
                    out,
                    "<text x=\"{cx}\" y=\"{ty}\" class=\"{label_class_name}\" \
                     text-anchor=\"middle\">{value}</text>",
                    cx = x + w / 2.0,
                    ty = y + 17.0,
                );
            }
            x += w;
        }
        let _ = writeln!(
            out,
            "<text x=\"{tx}\" y=\"{ty}\" class=\"muted\">{total}</text>",
            tx = x + 8.0,
            ty = y + 17.0,
        );
        y += ROW_H + ROW_GAP;
    }
    out.push_str("</svg>\n");
    out
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic)] // test assertions/fixtures
mod tests {
    use super::*;

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
        let a = heat_grid_svg("EHRbase-rs 3.7.0", &m, &caps);
        let b = heat_grid_svg("EHRbase-rs 3.7.0", &m, &caps);
        assert_eq!(a, b);
        // Both encodings present: the fill class AND the glyph.
        assert!(a.contains("ev-passed"));
        assert!(a.contains('✓'));
        assert!(a.contains("ev-failed"));
        assert!(a.contains('✕'));
        // The unmapped capability renders as no-cases with its glyph.
        assert!(a.contains("ev-no-cases"));
        assert!(a.contains('∅'));
        // Tier bands + border encodings.
        for band in ["CORE", "STANDARD", "OPTIONS", "SEC-BASIC"] {
            assert!(a.contains(band), "band {band} missing");
        }
        assert!(a.contains("cell-required"));
        assert!(a.contains("cell-optional"));
        assert!(a.contains("prefers-color-scheme: dark"));
        assert!(!a.contains("Date"));
    }

    #[test]
    fn chapters_map_the_schedule_families() {
        assert_eq!(chapter_of("I_EHR_COMPOSITION.create-x"), "EHR");
        assert_eq!(chapter_of("I_EHR_EXTRACT_SERVICE.request-x"), "Messaging");
        assert_eq!(chapter_of("I_TDD_SERVICE.convert-x"), "Messaging");
        assert_eq!(chapter_of("I_DEFINITION_ADL2.get-x"), "Definitions");
        assert_eq!(chapter_of("I_QUERY_SERVICE.adhoc-x"), "Query");
        assert_eq!(chapter_of("I_DEMOGRAPHIC_SERVICE.create-x"), "Demographic");
        assert_eq!(chapter_of("I_ADMIN_ARCHIVE.archive-x"), "Admin");
        assert_eq!(
            chapter_of("CONT-DV_TEXT-validate_open"),
            "Content validation"
        );
        assert_eq!(chapter_of("SF-FLAT-commit"), "Simplified formats");
        assert_eq!(chapter_of("SEC-AUDIT-x"), "Security & privacy");
        assert_eq!(chapter_of("SIG-VERSION-x"), "Signing");
        assert_eq!(chapter_of("SOMETHING-else"), "Other");
    }

    #[test]
    fn the_chapter_bars_print_every_count() {
        let chapters = vec![
            (
                "EHR",
                ChapterCounts {
                    passed: 100,
                    failed: 2,
                    errored: 1,
                    not_applicable: 20,
                },
            ),
            (
                "Query",
                ChapterCounts {
                    passed: 15,
                    ..ChapterCounts::default()
                },
            ),
        ];
        let svg = chapter_bars_svg("EHRbase-rs 3.7.0", &chapters);
        let again = chapter_bars_svg("EHRbase-rs 3.7.0", &chapters);
        assert_eq!(svg, again);
        assert!(svg.contains(">100<"));
        assert!(svg.contains(">123<")); // the EHR row total
        assert!(svg.contains(">15<"));
        assert!(svg.contains("bar-na"));
        assert!(svg.contains("prefers-color-scheme: dark"));
    }
}
