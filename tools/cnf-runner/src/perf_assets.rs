// SPDX-FileCopyrightText: FerroEHR contributors
// SPDX-License-Identifier: Apache-2.0

//! Deterministic SVG assets rendered FROM committed measurement records.
//!
//! The published performance visuals derive from `results.json` exactly like
//! the conformance stats derive from the outcomes (no hand-drawn numbers,
//! CI-guarded regeneration).
//!
//! Every chart is a pure function of its inputs: stable ordering, fixed
//! precision, no timestamps. Latency percentiles are RE-DERIVED from the
//! embedded HDR V2 histograms, never read from the summary fields. Both
//! charts style for light and dark via `prefers-color-scheme`.

#![allow(
    clippy::disallowed_types,
    reason = "dev/verification tooling over JSON artifacts (the catalogue, results, wire \
              exchanges) — not the application (#1694); the carriers here are cfg(test)-only, so \
              #[expect] would be unfulfilled in the non-test build"
)]

use std::fmt::Write;

use crate::perf::{ClassVerdict, Measurement, PerformanceCase};

/// Shared SVG style block: text/grid tokens plus the percentile ramp (one
/// blue hue, light→dark for p50→p90→p99 — a sequential job, not categorical),
/// the measured-bar blue, the floor outline, and the reserved status green
/// for EARNED. Dark mode restyles from the same hues against the dark
/// surface.
const STYLE: &str = "<style>\n\
  text { fill: #52514e; font: 12px -apple-system, 'Segoe UI', Helvetica, Arial, sans-serif; }\n\
  .title { fill: #0b0b0b; font-weight: 600; }\n\
  .muted { fill: #8a8880; font-size: 11px; }\n\
  .grid { stroke: #e4e2dd; stroke-width: 1; }\n\
  .p50 { fill: #7db4ea; } .p90 { fill: #3d7fd0; } .p99 { fill: #1b5cab; }\n\
  .measured { fill: #2a78d6; }\n\
  .floor { fill: none; stroke: #8a8880; stroke-width: 1.5; stroke-dasharray: 4 3; }\n\
  .earned { fill: #1baf7a; font-weight: 600; }\n\
  .notearned { fill: #b3261e; font-weight: 600; }\n\
  .slo { stroke: #b3261e; stroke-width: 1.5; stroke-dasharray: 6 4; }\n\
  .slotext { fill: #b3261e; font-size: 11px; }\n\
  .sut { stroke: #2a78d6; fill: none; stroke-width: 1.8; }\n\
  .db { stroke: #8a8880; fill: none; stroke-width: 1.6; stroke-dasharray: 5 3; }\n\
  .warm { fill: #efede8; }\n\
  .curve { stroke: #2a78d6; fill: none; stroke-width: 2; }\n\
  .cmp { stroke: #8a8880; fill: none; stroke-width: 2; stroke-dasharray: 7 4; }\n\
  .cmpfill { fill: #8a8880; }\n\
  .knee { stroke: #1baf7a; stroke-width: 2; stroke-dasharray: 2 3; }\n\
  @media (prefers-color-scheme: dark) {\n\
    text { fill: #c3c2b7; }\n\
    .title { fill: #ffffff; }\n\
    .muted { fill: #8f8e85; }\n\
    .grid { stroke: #3a3a38; }\n\
    .p50 { fill: #3d5f82; } .p90 { fill: #3987e5; } .p99 { fill: #9ec7ef; }\n\
    .measured { fill: #3987e5; }\n\
    .floor { stroke: #8f8e85; }\n\
    .earned { fill: #26c78d; }\n\
    .notearned { fill: #e5484d; }\n\
    .slo { stroke: #e5484d; }\n\
    .slotext { fill: #e5484d; }\n\
    .sut { stroke: #3987e5; }\n\
    .db { stroke: #8f8e85; }\n\
    .warm { fill: #2a2a28; }\n\
    .curve { stroke: #3987e5; }\n\
    .cmp { stroke: #8f8e85; }\n\
    .cmpfill { fill: #8f8e85; }\n\
    .knee { stroke: #26c78d; }\n\
  }\n\
</style>\n";

fn svg_open(out: &mut String, width: f64, height: f64) {
    let _ = write!(
        out,
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {width} {height}\" \
         width=\"{width}\" height=\"{height}\" role=\"img\">\n{STYLE}"
    );
}

/// log₁₀ position of `value` on an axis spanning `min..max` mapped to
/// `x0..x1`.
fn log_pos(value: f64, min: f64, max: f64, x0: f64, x1: f64) -> f64 {
    let v = value.max(min);
    x0 + (v.log10() - min.log10()) / (max.log10() - min.log10()) * (x1 - x0)
}

/// The class ladder: every class's offered-load floor (dashed outline) with
/// the measured sustained load and verdict overlaid.
///
/// Fixed columns — class + floor on the left, the log-scale bars in a fixed
/// plot area, the measured/verdict status right-aligned at a fixed edge — so
/// no label ever chases a bar end or leaves the canvas.
#[must_use]
pub fn class_ladder_svg(cases: &[PerformanceCase], measurements: &[Measurement]) -> String {
    let (width, height) = (640.0, 292.0);
    let (x0, x1) = (170.0, 500.0);
    // The status column starts a fixed gap after the plot area — one short
    // verdict word per row (the sustained rate is what the measured bar's
    // length shows; the exact number lives in the generated summary table).
    let status_x = x1 + 16.0;
    let top = 92.0;
    let row_h = 44.0;
    let bar_h = 16.0;
    let (min, max) = (1.0, 3000.0);

    let mut out = String::new();
    svg_open(&mut out, width, height);
    let _ = writeln!(
        out,
        "<text x=\"24\" y=\"28\" class=\"title\">Performance class ladder — offered-load floors vs measured sustained load</text>"
    );
    let _ = writeln!(
        out,
        "<text x=\"24\" y=\"48\" class=\"muted\">Floors are the request rate the hospital-simulation workload must sustain against the class corpus.</text>"
    );
    let _ = writeln!(
        out,
        "<text x=\"24\" y=\"63\" class=\"muted\">A class is earned only with p99 &#8804; 1 s on every measured operation and zero errors under that load.</text>"
    );

    // Grid at decades.
    for decade in [1.0, 10.0, 100.0, 1000.0] {
        let x = log_pos(decade, min, max, x0, x1);
        let _ = writeln!(
            out,
            "<line x1=\"{x:.1}\" y1=\"{top}\" x2=\"{x:.1}\" y2=\"{:.1}\" class=\"grid\"/>\
             <text x=\"{x:.1}\" y=\"{:.1}\" class=\"muted\" text-anchor=\"middle\">{decade}/s</text>",
            top + 4.0 * row_h,
            top + 4.0 * row_h + 16.0,
        );
    }

    let mut classes: Vec<&PerformanceCase> = cases.iter().collect();
    classes.sort_by(|a, b| {
        a.class
            .arrival_floor_per_s()
            .total_cmp(&b.class.arrival_floor_per_s())
    });
    for (row, case) in classes.iter().enumerate() {
        #[expect(
            clippy::as_conversions,
            clippy::cast_precision_loss,
            reason = "row counts are tiny"
        )]
        let y = top + row as f64 * row_h + 8.0;
        let floor = case.class.arrival_floor_per_s();
        let floor_x = log_pos(floor, min, max, x0, x1);
        // Left column: the class and its floor, fixed position.
        let _ = writeln!(
            out,
            "<text x=\"24\" y=\"{:.1}\">class {}</text>\
             <text x=\"24\" y=\"{:.1}\" class=\"muted\">floor {floor}/s</text>",
            y + 8.0,
            case.class.token(),
            y + 22.0,
        );
        let _ = writeln!(
            out,
            "<rect x=\"{x0}\" y=\"{y:.1}\" width=\"{:.1}\" height=\"{bar_h}\" rx=\"4\" class=\"floor\"/>",
            floor_x - x0,
        );
        // Right column: the status, right-aligned at a fixed edge.
        let status_y = y + bar_h - 3.0;
        if let Some(m) = measurements.iter().find(|m| m.class == case.class) {
            let mx = log_pos(m.offered_load_sustained, min, max, x0, x1);
            let _ = writeln!(
                out,
                "<rect x=\"{x0}\" y=\"{y:.1}\" width=\"{:.1}\" height=\"{bar_h}\" rx=\"4\" class=\"measured\"/>",
                (mx - x0).max(2.0),
            );
            let (class_name, verdict_text) = match m.verdict {
                ClassVerdict::Earned => ("earned", "EARNED"),
                ClassVerdict::NotEarned => ("notearned", "NOT EARNED"),
            };
            let _ = writeln!(
                out,
                "<text x=\"{status_x}\" y=\"{status_y:.1}\" class=\"{class_name}\">{verdict_text}</text>",
            );
        } else {
            let _ = writeln!(
                out,
                "<text x=\"{status_x}\" y=\"{status_y:.1}\" class=\"muted\">not measured</text>",
            );
        }
    }
    out.push_str("</svg>\n");
    out
}

/// Per-operation latency percentiles for one measured run.
///
/// A VERTICAL
/// layout that grows organically with the operation count (the journey
/// workload measures ~20+ operations; a horizontal grouping cannot fit):
/// one row per operation, three horizontal bars (p50/p90/p99, one hue
/// light→dark), latency on a log x-axis in milliseconds, the p99 ≤ 1 s
/// SLO as a vertical line, the p99 value printed at its bar end.
/// Percentiles re-derived from the decoded histograms.
///
/// # Errors
/// A message when a histogram fails to decode.
#[expect(
    clippy::as_conversions,
    reason = "SVG geometry and latency scaling: row/bar indices and microsecond \
              latencies are far below 2^52, so the widening is exact"
)]
pub fn latency_percentiles_svg(measurement: &Measurement) -> Result<String, String> {
    const LABEL_W: f64 = 214.0;
    const BAR_H: f64 = 9.0;
    const BAR_GAP: f64 = 2.0;
    const ROW_H: f64 = 3.0 * (BAR_H + BAR_GAP) + 10.0;
    const MARGIN: f64 = 24.0;
    let width = 900.0;
    let x0 = MARGIN + LABEL_W;
    let x1 = width - 96.0;
    let (min_ms, max_ms) = (0.1, 3000.0);
    let header_h = 78.0;
    #[expect(clippy::cast_precision_loss, reason = "operation counts are tiny")]
    let rows_h = measurement.operations.len() as f64 * ROW_H;
    let height = header_h + rows_h + 34.0;

    let mut out = String::new();
    svg_open(&mut out, width, height);
    let _ = writeln!(
        out,
        "<text x=\"{MARGIN}\" y=\"28\" class=\"title\">Latency percentiles — class {} measured run ({})</text>",
        measurement.class.token(),
        measurement.case,
    );
    let _ = writeln!(
        out,
        "<text x=\"{MARGIN}\" y=\"44\" class=\"muted\">Re-derived from the committed HDR V2 histograms · offered load {:.1}/s sustained for {} s · errors are a verdict input, not shown</text>",
        measurement.offered_load_sustained, measurement.duration_s,
    );
    // Legend (the percentile ramp).
    let mut lx = MARGIN;
    for (class, label) in [("p50", "p50"), ("p90", "p90"), ("p99", "p99")] {
        let _ = writeln!(
            out,
            "<rect x=\"{lx}\" y=\"54\" width=\"14\" height=\"10\" rx=\"3\" class=\"{class}\"/>\
             <text x=\"{:.1}\" y=\"63\" class=\"muted\">{label}</text>",
            lx + 18.0,
        );
        lx += 62.0;
    }

    let x_of = |ms: f64| {
        let clamped = ms.clamp(min_ms, max_ms);
        x0 + (clamped.log10() - min_ms.log10()) / (max_ms.log10() - min_ms.log10()) * (x1 - x0)
    };

    // Vertical decade grid + axis labels along the bottom.
    let grid_bottom = header_h + rows_h;
    for ms in [0.1, 1.0, 10.0, 100.0, 1000.0] {
        let x = x_of(ms);
        let _ = writeln!(
            out,
            "<line x1=\"{x:.1}\" y1=\"{header_h}\" x2=\"{x:.1}\" y2=\"{grid_bottom:.1}\" class=\"grid\"/>\
             <text x=\"{x:.1}\" y=\"{:.1}\" class=\"muted\" text-anchor=\"middle\">{ms} ms</text>",
            grid_bottom + 16.0,
        );
    }
    // The SLO line (vertical at 1 s).
    let slo_x = x_of(1000.0);
    let _ = writeln!(
        out,
        "<line x1=\"{slo_x:.1}\" y1=\"{header_h}\" x2=\"{slo_x:.1}\" y2=\"{grid_bottom:.1}\" class=\"slo\"/>\
         <text x=\"{:.1}\" y=\"{:.1}\" class=\"slotext\">SLO p99 &#8804; 1 s</text>",
        slo_x + 6.0,
        header_h + 12.0,
    );

    for (row, op) in measurement.operations.iter().enumerate() {
        let histogram = op.decode_histogram()?;
        #[expect(clippy::cast_precision_loss, reason = "row counts are tiny")]
        let ry = header_h + row as f64 * ROW_H + 5.0;
        let _ = writeln!(
            out,
            "<text x=\"{:.1}\" y=\"{:.1}\" text-anchor=\"end\">{}</text>",
            x0 - 10.0,
            ry + 1.5 * (BAR_H + BAR_GAP) + 3.0,
            op.operation,
        );
        for (i, (class, quantile)) in [("p50", 0.50), ("p90", 0.90), ("p99", 0.99)]
            .iter()
            .enumerate()
        {
            #[expect(clippy::cast_precision_loss, reason = "us << 2^52")]
            let ms = histogram.value_at_quantile(*quantile) as f64 / 1_000.0;
            #[expect(clippy::cast_precision_loss, reason = "3 bars per row")]
            let y = ry + i as f64 * (BAR_H + BAR_GAP);
            let bar_end = x_of(ms);
            let _ = writeln!(
                out,
                "<rect x=\"{x0:.1}\" y=\"{y:.1}\" width=\"{:.1}\" height=\"{BAR_H}\" rx=\"2\" class=\"{class}\"/>\
                 <text x=\"{:.1}\" y=\"{:.1}\" class=\"muted\">{}</text>",
                (bar_end - x0).max(2.0),
                bar_end + 6.0,
                y + BAR_H - 1.0,
                format_ms(ms),
            );
        }
    }
    out.push_str("</svg>\n");
    Ok(out)
}

/// The latency-throughput curve from a committed stress report.
///
/// It shows WHERE THE
/// SYSTEM BREAKS, nothing else: offered rate (x, log) vs the worst
/// per-operation p99 (y, log, re-derived from the decoded histograms),
/// envelope-holding steps as circles, breached steps as crosses, the p99
/// budget line, and the maximum-sustainable-throughput marker (the knee).
/// Deliberately class-free: the volumetric class ladder belongs to the
/// measured class runs, and no class token appears here.
///
/// # Errors
/// A message when a histogram fails to decode.
#[expect(clippy::too_many_lines, reason = "one linear chart emitter")]
pub fn stress_curve_svg(report: &crate::stress::StressReport) -> Result<String, String> {
    let (width, height) = (760.0, 400.0);
    let (x0, x1) = (90.0, 700.0);
    let (y_top, y_bottom) = (80.0, 330.0);
    let (min_rate, max_rate) = (1.0, 6000.0);
    let (min_ms, max_ms) = (1.0, 6000.0);

    let mut out = String::new();
    svg_open(&mut out, width, height);
    let _ = writeln!(
        out,
        "<text x=\"24\" y=\"28\" class=\"title\">Latency-throughput curve — step-load stress to the maximum sustainable throughput</text>"
    );
    let _ = writeln!(
        out,
        "<text x=\"24\" y=\"44\" class=\"muted\">Short intense load steps ({} s hold) on the {} corpus · exploration only — never a conformance record</text>",
        report.step_hold_s, report.corpus,
    );
    let _ = writeln!(
        out,
        "<text x=\"24\" y=\"59\" class=\"muted\">Circles hold the envelope, crosses breach it; the dotted line is the last rate held inside the envelope — the knee.</text>"
    );

    let x_of = |rate: f64| log_pos(rate, min_rate, max_rate, x0, x1);
    let y_of = |ms: f64| {
        let clamped = ms.clamp(min_ms, max_ms);
        y_bottom
            - (clamped.log10() - min_ms.log10()) / (max_ms.log10() - min_ms.log10())
                * (y_bottom - y_top)
    };

    // Grid: rate decades + latency decades.
    for rate in [1.0, 10.0, 100.0, 1000.0] {
        let x = x_of(rate);
        let _ = writeln!(
            out,
            "<line x1=\"{x:.1}\" y1=\"{y_top}\" x2=\"{x:.1}\" y2=\"{y_bottom}\" class=\"grid\"/>\
             <text x=\"{x:.1}\" y=\"{:.1}\" class=\"muted\" text-anchor=\"middle\">{rate}/s</text>",
            y_bottom + 16.0,
        );
    }
    for ms in [1.0, 10.0, 100.0, 1000.0] {
        let y = y_of(ms);
        let _ = writeln!(
            out,
            "<line x1=\"{x0}\" y1=\"{y:.1}\" x2=\"{x1}\" y2=\"{y:.1}\" class=\"grid\"/>\
             <text x=\"{:.1}\" y=\"{:.1}\" class=\"muted\" text-anchor=\"end\">{ms} ms</text>",
            x0 - 8.0,
            y + 4.0,
        );
    }
    // The p99 budget line.
    let budget_y = y_of(report.p99_budget_ms);
    let _ = writeln!(
        out,
        "<line x1=\"{x0}\" y1=\"{budget_y:.1}\" x2=\"{x1}\" y2=\"{budget_y:.1}\" class=\"slo\"/>\
         <text x=\"{:.1}\" y=\"{:.1}\" class=\"slotext\" text-anchor=\"end\">p99 budget</text>",
        x1 - 4.0,
        budget_y - 6.0,
    );
    // Steps, in rate order: worst per-operation p99 per step (re-derived).
    let mut points: Vec<(f64, f64, bool)> = Vec::new();
    for step in &report.steps {
        let mut worst_ms: f64 = 0.0;
        for op in &step.operations {
            let histogram = op.decode_histogram()?;
            #[expect(
                clippy::as_conversions,
                clippy::cast_precision_loss,
                reason = "latencies << 2^52 µs"
            )]
            let ms = histogram.value_at_quantile(0.99) as f64 / 1_000.0;
            worst_ms = worst_ms.max(ms);
        }
        points.push((step.rate, worst_ms, step.stable));
    }
    points.sort_by(|a, b| a.0.total_cmp(&b.0));
    let path: Vec<String> = points
        .iter()
        .enumerate()
        .map(|(i, (rate, ms, _))| {
            format!(
                "{}{:.1},{:.1}",
                if i == 0 { "M" } else { "L" },
                x_of(*rate),
                y_of(*ms)
            )
        })
        .collect();
    let _ = writeln!(out, "<path d=\"{}\" class=\"curve\"/>", path.join(" "));
    for (rate, ms, stable) in &points {
        let (x, y) = (x_of(*rate), y_of(*ms));
        if *stable {
            let _ = writeln!(
                out,
                "<circle cx=\"{x:.1}\" cy=\"{y:.1}\" r=\"5\" class=\"measured\"/>"
            );
        } else {
            // A breached step: a distinct MARK (cross), not color alone.
            let _ = writeln!(
                out,
                "<path d=\"M{:.1},{:.1} L{:.1},{:.1} M{:.1},{:.1} L{:.1},{:.1}\" class=\"slo\" stroke-width=\"2\"/>",
                x - 5.0,
                y - 5.0,
                x + 5.0,
                y + 5.0,
                x - 5.0,
                y + 5.0,
                x + 5.0,
                y - 5.0,
            );
        }
    }
    // The maximum-sustainable-throughput marker; the label sits at the
    // plot bottom (clear of any top-clamped breach crosses) and flips to
    // the marker's left past the midline so it can never leave the canvas.
    let mst_x = x_of(report.max_sustainable_throughput_per_s.max(min_rate));
    let (label_x, anchor) = if mst_x > f64::midpoint(x0, x1) {
        (mst_x - 6.0, "end")
    } else {
        (mst_x + 6.0, "start")
    };
    let _ = writeln!(
        out,
        "<line x1=\"{mst_x:.1}\" y1=\"{y_top}\" x2=\"{mst_x:.1}\" y2=\"{y_bottom}\" class=\"knee\"/>\
         <text x=\"{label_x:.1}\" y=\"{:.1}\" class=\"earned\" text-anchor=\"{anchor}\">max sustainable {:.0}/s</text>",
        y_bottom - 10.0,
        report.max_sustainable_throughput_per_s,
    );
    out.push_str("</svg>\n");
    Ok(out)
}

/// The cross-SUT stress overlay.
///
/// Both systems' latency-throughput curves
/// (offered rate vs worst per-operation p99, log-log, re-derived from each
/// side's decoded histograms) on one canvas — the left/primary SUT a solid
/// line with circle markers, the right/comparison SUT a dashed line with
/// square markers (distinguishable by color AND mark), each side's
/// maximum-sustainable-throughput marked, breached steps as crosses, the
/// shared p99 budget line and the class floors as context. Both directions
/// published on equal footing — where the comparison SUT's knee sits
/// higher, the chart says so exactly like the reverse.
///
/// # Errors
/// A message when a histogram fails to decode.
#[expect(clippy::too_many_lines, reason = "one linear chart emitter")]
pub fn stress_compare_svg(
    left: (&str, &crate::stress::StressReport),
    right: (&str, &crate::stress::StressReport),
) -> Result<String, String> {
    let (width, height) = (760.0, 430.0);
    let (x0, x1) = (90.0, 700.0);
    let (y_top, y_bottom) = (96.0, 360.0);
    let (min_rate, max_rate) = (1.0, 6000.0);
    let (min_ms, max_ms) = (1.0, 6000.0);

    let mut out = String::new();
    svg_open(&mut out, width, height);
    let _ = writeln!(
        out,
        "<text x=\"24\" y=\"28\" class=\"title\">Latency-throughput curves — both systems, the same step-load stress instrument</text>"
    );
    let _ = writeln!(
        out,
        "<text x=\"24\" y=\"44\" class=\"muted\">Identical committed workload and ladder per side ({} s holds) · exploration only — never a conformance record</text>",
        left.1.step_hold_s,
    );
    // Legend: label + line sample + marker per side, fixed columns.
    let legend_y = 58.0;
    let _ = writeln!(
        out,
        "<line x1=\"24\" y1=\"{legend_y}\" x2=\"52\" y2=\"{legend_y}\" class=\"curve\"/>\
         <circle cx=\"38\" cy=\"{legend_y}\" r=\"4\" class=\"measured\"/>\
         <text x=\"58\" y=\"{:.1}\" class=\"muted\">{}</text>",
        legend_y + 4.0,
        left.0,
    );
    let _ = writeln!(
        out,
        "<line x1=\"300\" y1=\"{legend_y}\" x2=\"328\" y2=\"{legend_y}\" class=\"cmp\"/>\
         <rect x=\"310\" y=\"{:.1}\" width=\"8\" height=\"8\" class=\"cmpfill\"/>\
         <text x=\"334\" y=\"{:.1}\" class=\"muted\">{}</text>",
        legend_y - 4.0,
        legend_y + 4.0,
        right.0,
    );

    let x_of = |rate: f64| log_pos(rate, min_rate, max_rate, x0, x1);
    let y_of = |ms: f64| {
        let clamped = ms.clamp(min_ms, max_ms);
        y_bottom
            - (clamped.log10() - min_ms.log10()) / (max_ms.log10() - min_ms.log10())
                * (y_bottom - y_top)
    };

    for rate in [1.0, 10.0, 100.0, 1000.0] {
        let x = x_of(rate);
        let _ = writeln!(
            out,
            "<line x1=\"{x:.1}\" y1=\"{y_top}\" x2=\"{x:.1}\" y2=\"{y_bottom}\" class=\"grid\"/>\
             <text x=\"{x:.1}\" y=\"{:.1}\" class=\"muted\" text-anchor=\"middle\">{rate}/s</text>",
            y_bottom + 16.0,
        );
    }
    for ms in [1.0, 10.0, 100.0, 1000.0] {
        let y = y_of(ms);
        let _ = writeln!(
            out,
            "<line x1=\"{x0}\" y1=\"{y:.1}\" x2=\"{x1}\" y2=\"{y:.1}\" class=\"grid\"/>\
             <text x=\"{:.1}\" y=\"{:.1}\" class=\"muted\" text-anchor=\"end\">{ms} ms</text>",
            x0 - 8.0,
            y + 4.0,
        );
    }
    let budget_y = y_of(left.1.p99_budget_ms);
    let _ = writeln!(
        out,
        "<line x1=\"{x0}\" y1=\"{budget_y:.1}\" x2=\"{x1}\" y2=\"{budget_y:.1}\" class=\"slo\"/>\
         <text x=\"{:.1}\" y=\"{:.1}\" class=\"slotext\" text-anchor=\"end\">p99 budget</text>",
        x1 - 4.0,
        budget_y - 6.0,
    );
    // One curve per side: worst per-operation p99 per step (re-derived).
    let side = |report: &crate::stress::StressReport,
                line_class: &str,
                out: &mut String|
     -> Result<Vec<(f64, f64, bool)>, String> {
        let mut points: Vec<(f64, f64, bool)> = Vec::new();
        for step in &report.steps {
            let mut worst_ms: f64 = 0.0;
            for op in &step.operations {
                let histogram = op.decode_histogram()?;
                #[expect(
                    clippy::as_conversions,
                    clippy::cast_precision_loss,
                    reason = "latencies << 2^52 µs"
                )]
                let ms = histogram.value_at_quantile(0.99) as f64 / 1_000.0;
                worst_ms = worst_ms.max(ms);
            }
            points.push((step.rate, worst_ms, step.stable));
        }
        points.sort_by(|a, b| a.0.total_cmp(&b.0));
        let path: Vec<String> = points
            .iter()
            .enumerate()
            .map(|(i, (rate, ms, _))| {
                format!(
                    "{}{:.1},{:.1}",
                    if i == 0 { "M" } else { "L" },
                    x_of(*rate),
                    y_of(*ms)
                )
            })
            .collect();
        let _ = writeln!(
            out,
            "<path d=\"{}\" class=\"{line_class}\"/>",
            path.join(" ")
        );
        Ok(points)
    };
    let left_points = side(left.1, "curve", &mut out)?;
    let right_points = side(right.1, "cmp", &mut out)?;
    for (rate, ms, stable) in &left_points {
        let (x, y) = (x_of(*rate), y_of(*ms));
        if *stable {
            let _ = writeln!(
                out,
                "<circle cx=\"{x:.1}\" cy=\"{y:.1}\" r=\"5\" class=\"measured\"/>"
            );
        } else {
            let _ = writeln!(
                out,
                "<path d=\"M{:.1},{:.1} L{:.1},{:.1} M{:.1},{:.1} L{:.1},{:.1}\" class=\"slo\" stroke-width=\"2\"/>",
                x - 5.0,
                y - 5.0,
                x + 5.0,
                y + 5.0,
                x - 5.0,
                y + 5.0,
                x + 5.0,
                y - 5.0,
            );
        }
    }
    for (rate, ms, stable) in &right_points {
        let (x, y) = (x_of(*rate), y_of(*ms));
        if *stable {
            let _ = writeln!(
                out,
                "<rect x=\"{:.1}\" y=\"{:.1}\" width=\"8\" height=\"8\" class=\"cmpfill\"/>",
                x - 4.0,
                y - 4.0,
            );
        } else {
            let _ = writeln!(
                out,
                "<path d=\"M{:.1},{:.1} L{:.1},{:.1} M{:.1},{:.1} L{:.1},{:.1}\" class=\"slo\" stroke-width=\"2\"/>",
                x - 5.0,
                y - 5.0,
                x + 5.0,
                y + 5.0,
                x - 5.0,
                y + 5.0,
                x + 5.0,
                y - 5.0,
            );
        }
    }
    // Each side's maximum sustainable throughput, labels stacked at fixed
    // rows so they can never collide.
    let left_mst = x_of(left.1.max_sustainable_throughput_per_s.max(min_rate));
    let right_mst = x_of(right.1.max_sustainable_throughput_per_s.max(min_rate));
    let _ = writeln!(
        out,
        "<line x1=\"{left_mst:.1}\" y1=\"{y_top}\" x2=\"{left_mst:.1}\" y2=\"{y_bottom}\" class=\"knee\"/>\
         <text x=\"{:.1}\" y=\"{:.1}\" class=\"earned\">{} max sustainable {:.0}/s</text>",
        x0,
        y_bottom + 34.0,
        left.0,
        left.1.max_sustainable_throughput_per_s,
    );
    let _ = writeln!(
        out,
        "<line x1=\"{right_mst:.1}\" y1=\"{y_top}\" x2=\"{right_mst:.1}\" y2=\"{y_bottom}\" class=\"cmp\"/>\
         <text x=\"{:.1}\" y=\"{:.1}\" class=\"muted\">{} max sustainable {:.0}/s</text>",
        x0,
        y_bottom + 52.0,
        right.0,
        right.1.max_sustainable_throughput_per_s,
    );
    out.push_str("</svg>\n");
    Ok(out)
}

/// Fixed-precision byte label (decimal units — KB/MB/GB, the vocabulary
/// every reader knows; the exact byte counts stay in the record):
/// sub-10 values keep one decimal, larger values round whole.
fn format_bytes(bytes: f64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut value = bytes.max(0.0);
    let mut unit = 0;
    while value >= 1000.0 && unit < UNITS.len() - 1 {
        value /= 1000.0;
        unit += 1;
    }
    let text = if unit == 0 {
        format!("{value:.0}")
    } else if value < 10.0 {
        format!("{value:.1}")
    } else {
        format!("{value:.0}")
    };
    format!("{text} {}", UNITS.get(unit).copied().unwrap_or("B"))
}

/// A count with thousands separators (`1,000,000`) — fixed formatting,
/// locale-independent.
fn format_count(n: u64) -> String {
    let digits = n.to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

/// The smallest 1/2/2.5/5 × 10^k value at or above `v` — a nice axis
/// ceiling (`v <= 0` yields 1).
fn nice_ceil(v: f64) -> f64 {
    if v <= 0.0 {
        return 1.0;
    }
    let magnitude = 10.0_f64.powf(v.log10().floor());
    for step in [1.0, 2.0, 2.5, 5.0, 10.0] {
        if step * magnitude >= v {
            return step * magnitude;
        }
    }
    10.0 * magnitude
}

/// One derived series of a resource time-series chart: run-clock offset
/// (seconds) → value.
type ResourcePoints = Vec<(f64, f64)>;

/// A cumulative-counter selector on a resource sample (the I/O strips).
type CounterOf = &'static dyn Fn(&crate::perf::ResourceSample) -> u64;

/// Extract one per-container value series from the sampled record.
fn value_series(
    series: &crate::perf::ContainerResourceSeries,
    value: &dyn Fn(&crate::perf::ResourceSample) -> f64,
) -> ResourcePoints {
    series
        .samples
        .iter()
        .map(|s| {
            #[expect(
                clippy::as_conversions,
                clippy::cast_precision_loss,
                reason = "offsets << 2^52"
            )]
            (s.offset_s as f64, value(s))
        })
        .collect()
}

/// Derive a rate series (units/s) from a cumulative byte counter: deltas
/// between consecutive samples over their spacing. A counter reset (a
/// restarted container) clamps to zero rather than plunging negative.
fn rate_series(
    series: &crate::perf::ContainerResourceSeries,
    counter: &dyn Fn(&crate::perf::ResourceSample) -> u64,
) -> ResourcePoints {
    series
        .samples
        .windows(2)
        .filter_map(|pair| {
            let (a, b) = (pair.first()?, pair.get(1)?);
            let span = b.offset_s.saturating_sub(a.offset_s);
            if span == 0 {
                return None;
            }
            #[expect(
                clippy::as_conversions,
                clippy::cast_precision_loss,
                reason = "counters/offsets << 2^52"
            )]
            Some((
                b.offset_s as f64,
                counter(b).saturating_sub(counter(a)) as f64 / span as f64,
            ))
        })
        .collect()
}

/// Append one polyline path for a value series.
fn polyline(
    out: &mut String,
    points: &[(f64, f64)],
    x_of: &dyn Fn(f64) -> f64,
    y_of: &dyn Fn(f64) -> f64,
    class: &str,
) {
    if points.len() < 2 {
        return;
    }
    let d: Vec<String> = points
        .iter()
        .enumerate()
        .map(|(i, (t, v))| {
            format!(
                "{}{:.1},{:.1}",
                if i == 0 { "M" } else { "L" },
                x_of(*t),
                y_of(*v)
            )
        })
        .collect();
    let _ = writeln!(out, "<path d=\"{}\" class=\"{class}\"/>", d.join(" "));
}

/// The resource time-series for one measured run.
///
/// CPU and RSS panels over
/// the run clock (SUT and DB as two series, distinguishable by color AND
/// dash), warmup shaded, with small-multiple I/O rate strips (block
/// read/write, network receive/transmit) under them on the shared x-axis.
/// Fixed canvas — the series are lines, so width never grows with
/// duration. `None` when the record carries no drawable series (a chart
/// of nothing would be a fabrication).
#[must_use]
#[expect(clippy::too_many_lines, reason = "one linear chart emitter")]
pub fn resources_timeseries_svg(measurement: &Measurement) -> Option<String> {
    let resources = measurement.resources.as_ref()?;
    if !resources.containers.iter().any(|c| c.samples.len() >= 2) {
        return None;
    }
    let width = 760.0;
    let (x0, x1) = (86.0, 724.0);
    let sut = resources
        .containers
        .iter()
        .find(|c| c.role == crate::perf::ContainerRole::Sut);
    let db = resources
        .containers
        .iter()
        .find(|c| c.role == crate::perf::ContainerRole::Db);
    let both = [sut, db];

    // The x-domain: the planned window (warmup + sustained), extended by
    // any trailing drain samples.
    let last_offset = resources
        .containers
        .iter()
        .flat_map(|c| c.samples.iter().map(|s| s.offset_s))
        .max()
        .unwrap_or(0);
    #[expect(
        clippy::as_conversions,
        clippy::cast_precision_loss,
        reason = "spans << 2^52"
    )]
    let span_s = (measurement.warmup_s + measurement.duration_s).max(last_offset) as f64;
    let x_of = move |t: f64| x0 + (t / span_s).clamp(0.0, 1.0) * (x1 - x0);

    let mut out = String::new();
    // Header (title + caption + legend) 84, two panels of 110 with 26-px
    // titles, four 40-px strips with 22-px titles, 38 for the time axis.
    let cpu_top = 110.0;
    let panel_h = 110.0;
    let rss_top = cpu_top + panel_h + 26.0;
    let strips_top = rss_top + panel_h + 26.0;
    let strip_h = 40.0;
    let strip_step = strip_h + 22.0;
    // The last strip's bottom edge; the time axis sits in a fixed band
    // below it, inside the canvas by construction.
    let bottom = strips_top + 3.0 * strip_step + strip_h;
    let height = bottom + 34.0;
    svg_open(&mut out, width, height);
    let _ = writeln!(
        out,
        "<text x=\"24\" y=\"28\" class=\"title\">Resource telemetry — class {} measured run ({})</text>",
        measurement.class.token(),
        measurement.case,
    );
    let _ = writeln!(
        out,
        "<text x=\"24\" y=\"44\" class=\"muted\">Sampled every {} s across the whole window · measured context, never a verdict input — the class is earned on latency, errors and offered load alone</text>",
        resources.sample_interval_s,
    );
    // Legend at fixed columns (the container identities live in the
    // generated summary table — the legend never grows with a name).
    let legend_y = 58.0;
    for (class, label, lx) in [
        ("sut", "SUT container", 24.0),
        ("db", "database container", 170.0),
    ] {
        let _ = writeln!(
            out,
            "<line x1=\"{lx}\" y1=\"{:.1}\" x2=\"{:.1}\" y2=\"{:.1}\" class=\"{class}\"/>\
             <text x=\"{:.1}\" y=\"{:.1}\" class=\"muted\">{label}</text>",
            legend_y + 4.0,
            lx + 22.0,
            legend_y + 4.0,
            lx + 28.0,
            legend_y + 8.0,
        );
    }
    let _ = writeln!(
        out,
        "<rect x=\"344\" y=\"{legend_y}\" width=\"14\" height=\"10\" class=\"warm\"/>\
         <text x=\"364\" y=\"{:.1}\" class=\"muted\">warmup</text>",
        legend_y + 8.0,
    );

    // Time ticks: a minute step giving <= 8 ticks.
    let minutes = span_s / 60.0;
    let tick_step_min = [1.0, 2.0, 5.0, 10.0, 15.0, 30.0, 60.0, 120.0, 240.0]
        .into_iter()
        .find(|step| minutes / step <= 8.0)
        .unwrap_or(240.0);

    #[expect(
        clippy::as_conversions,
        clippy::cast_precision_loss,
        reason = "window seconds << 2^52"
    )]
    let warm_w = x_of(measurement.warmup_s as f64) - x0;

    // One panel or strip: warmup shade, gridlines, y-labels, series.
    let panel = |out: &mut String,
                 top: f64,
                 h: f64,
                 title: &str,
                 points: &[(&str, ResourcePoints)],
                 label_of: &dyn Fn(f64) -> String| {
        let y_max = nice_ceil(
            points
                .iter()
                .flat_map(|(_, p)| p.iter().map(|(_, v)| *v))
                .fold(0.0, f64::max),
        );
        let y_of = move |v: f64| top + h - (v / y_max).clamp(0.0, 1.0) * h;
        let _ = writeln!(
            out,
            "<text x=\"{x0}\" y=\"{:.1}\" class=\"muted\">{title}</text>",
            top - 7.0,
        );
        let _ = writeln!(
            out,
            "<rect x=\"{x0}\" y=\"{top:.1}\" width=\"{warm_w:.1}\" height=\"{h}\" class=\"warm\"/>",
        );
        // Gridlines at 0 / half / full scale, labeled on the left.
        for frac in [0.0, 0.5, 1.0] {
            let y = y_of(y_max * frac);
            let _ = writeln!(
                out,
                "<line x1=\"{x0}\" y1=\"{y:.1}\" x2=\"{x1}\" y2=\"{y:.1}\" class=\"grid\"/>\
                 <text x=\"{:.1}\" y=\"{:.1}\" class=\"muted\" text-anchor=\"end\">{}</text>",
                x0 - 8.0,
                y + 4.0,
                label_of(y_max * frac),
            );
        }
        for (class, series_points) in points {
            polyline(out, series_points, &x_of, &y_of, class);
        }
    };

    let pair = |value: &dyn Fn(&crate::perf::ResourceSample) -> f64| -> Vec<(&'static str, ResourcePoints)> {
        let mut set = Vec::new();
        if let Some(c) = sut {
            set.push(("sut", value_series(c, value)));
        }
        if let Some(c) = db {
            set.push(("db", value_series(c, value)));
        }
        set
    };
    let rate_pair = |counter: &dyn Fn(&crate::perf::ResourceSample) -> u64| -> Vec<(&'static str, ResourcePoints)> {
        both.iter()
            .flatten()
            .map(|c| {
                (
                    if c.role == crate::perf::ContainerRole::Sut {
                        "sut"
                    } else {
                        "db"
                    },
                    rate_series(c, counter),
                )
            })
            .collect()
    };

    panel(
        &mut out,
        cpu_top,
        panel_h,
        "CPU (% of one core)",
        &pair(&|s| s.cpu_pct),
        &|v| format!("{v:.0}%"),
    );
    panel(
        &mut out,
        rss_top,
        panel_h,
        "Resident memory",
        &pair(&|s| {
            #[expect(
                clippy::as_conversions,
                clippy::cast_precision_loss,
                reason = "bytes << 2^52"
            )]
            {
                s.rss_bytes as f64
            }
        }),
        &format_bytes,
    );
    let strips: [(&str, CounterOf); 4] = [
        ("Block read", &|s| s.blk_read_bytes),
        ("Block write", &|s| s.blk_write_bytes),
        ("Network receive", &|s| s.net_rx_bytes),
        ("Network transmit", &|s| s.net_tx_bytes),
    ];
    for (i, (title, counter)) in strips.iter().enumerate() {
        #[expect(
            clippy::as_conversions,
            clippy::cast_precision_loss,
            reason = "four strips"
        )]
        let top = strips_top + i as f64 * strip_step;
        panel(&mut out, top, strip_h, title, &rate_pair(counter), &|v| {
            format!("{}/s", format_bytes(v))
        });
    }

    // The shared time axis under the last strip.
    let axis_y = bottom + 6.0;
    let mut t_min = 0.0;
    while t_min * 60.0 <= span_s {
        let x = x_of(t_min * 60.0);
        let _ = writeln!(
            out,
            "<text x=\"{x:.1}\" y=\"{:.1}\" class=\"muted\" text-anchor=\"middle\">{t_min:.0} min</text>",
            axis_y + 12.0,
        );
        t_min += tick_step_min;
    }
    out.push_str("</svg>\n");
    Some(out)
}

/// The disk-growth waterfall.
///
/// The database volume's on-disk size at the
/// run's four anchors (empty → scale seed → ward seed → after the
/// window), each present anchor a bar labeled with its absolute size, the
/// scale-seed step annotated with the derived bytes per committed
/// composition. Rendered from the highest measured class carrying
/// anchors; `None` when no record carries any. Fixed columns — nothing
/// can overflow by construction.
#[must_use]
#[expect(clippy::too_many_lines, reason = "one linear chart emitter")]
pub fn disk_growth_svg(measurements: &[Measurement]) -> Option<String> {
    let measurement = measurements
        .iter()
        .filter(|m| {
            m.resources
                .as_ref()
                .and_then(|r| r.disk)
                .is_some_and(|disk| {
                    disk.before_scale_seed_bytes.is_some()
                        || disk.after_scale_seed_bytes.is_some()
                        || disk.after_ward_seed_bytes.is_some()
                        || disk.after_window_bytes.is_some()
                })
        })
        .max_by(|a, b| {
            a.class
                .arrival_floor_per_s()
                .total_cmp(&b.class.arrival_floor_per_s())
        })?;
    let disk = measurement.resources.as_ref()?.disk?;

    let (width, height) = (640.0, 312.0);
    let (y_top, y_bottom) = (100.0, 244.0);
    let anchors: [(&str, Option<u64>); 4] = [
        ("empty", disk.before_scale_seed_bytes),
        ("after scale seed", disk.after_scale_seed_bytes),
        ("after ward seed", disk.after_ward_seed_bytes),
        ("after window", disk.after_window_bytes),
    ];
    #[expect(
        clippy::as_conversions,
        clippy::cast_precision_loss,
        reason = "volume sizes << 2^52"
    )]
    let max_bytes = anchors
        .iter()
        .filter_map(|(_, v)| *v)
        .max()
        .unwrap_or(1)
        .max(1) as f64;

    let mut out = String::new();
    svg_open(&mut out, width, height);
    let _ = writeln!(
        out,
        "<text x=\"24\" y=\"28\" class=\"title\">Disk growth — database volume across the class {} measured run</text>",
        measurement.class.token(),
    );
    let _ = writeln!(
        out,
        "<text x=\"24\" y=\"48\" class=\"muted\">The volume's on-disk size at the run's four anchors, probed read-only inside the DB container.</text>"
    );
    let _ = writeln!(
        out,
        "<text x=\"24\" y=\"63\" class=\"muted\">The scale-seed step yields the storage cost per committed composition · measured context, never a verdict input.</text>"
    );
    let _ = writeln!(
        out,
        "<line x1=\"60\" y1=\"{y_bottom}\" x2=\"580\" y2=\"{y_bottom}\" class=\"grid\"/>"
    );

    let bar_w = 84.0;
    for (i, (label, value)) in anchors.iter().enumerate() {
        #[expect(
            clippy::as_conversions,
            clippy::cast_precision_loss,
            reason = "four columns"
        )]
        let cx = 120.0 + i as f64 * 140.0;
        let _ = writeln!(
            out,
            "<text x=\"{cx:.1}\" y=\"{:.1}\" class=\"muted\" text-anchor=\"middle\">{label}</text>",
            y_bottom + 18.0,
        );
        match value {
            Some(bytes) => {
                #[expect(
                    clippy::as_conversions,
                    clippy::cast_precision_loss,
                    reason = "volume sizes << 2^52"
                )]
                let bytes_f = *bytes as f64;
                let h = (bytes_f / max_bytes) * (y_bottom - y_top);
                let y = y_bottom - h.max(2.0);
                let _ = writeln!(
                    out,
                    "<rect x=\"{:.1}\" y=\"{y:.1}\" width=\"{bar_w}\" height=\"{:.1}\" rx=\"4\" class=\"measured\"/>\
                     <text x=\"{cx:.1}\" y=\"{:.1}\" text-anchor=\"middle\">{}</text>",
                    cx - bar_w / 2.0,
                    h.max(2.0),
                    y - 8.0,
                    format_bytes(bytes_f),
                );
            }
            None => {
                let _ = writeln!(
                    out,
                    "<text x=\"{cx:.1}\" y=\"{:.1}\" class=\"muted\" text-anchor=\"middle\">not probed</text>",
                    f64::midpoint(y_top, y_bottom),
                );
            }
        }
    }
    // The storage-efficiency headline under the scale-seed column.
    if let (Some(before), Some(after), Some(n)) = (
        disk.before_scale_seed_bytes,
        disk.after_scale_seed_bytes,
        disk.seed_compositions,
    ) && n > 0
    {
        #[expect(
            clippy::as_conversions,
            clippy::cast_precision_loss,
            reason = "sizes/counts << 2^52"
        )]
        let per = after.saturating_sub(before) as f64 / n as f64;
        let _ = writeln!(
            out,
            "<text x=\"260\" y=\"{:.1}\" class=\"muted\" text-anchor=\"middle\">&#8776; {} / composition ({} committed)</text>",
            y_bottom + 34.0,
            format_bytes(per),
            format_count(n),
        );
    }
    out.push_str("</svg>\n");
    Some(out)
}

/// Fixed-precision label: sub-10 ms values keep one decimal, larger values
/// round to whole milliseconds.
fn format_ms(ms: f64) -> String {
    if ms < 10.0 {
        format!("{ms:.1}")
    } else {
        format!("{ms:.0}")
    }
}

/// The generated Markdown summary the book includes at build time.
///
/// It carries the
/// normative class ladder (floors from the schedule cases) with the measured
/// state per class, plus per-operation detail for every measured run. Every
/// number derives from the committed artifacts; percentiles re-derive from
/// the decoded histograms.
///
/// # Errors
/// A message when a histogram fails to decode.
pub fn summary_markdown(
    cases: &[PerformanceCase],
    measurements: &[Measurement],
) -> Result<String, String> {
    let mut out = String::new();
    let _ = writeln!(
        out,
        "| Class | Corpus | Offered-load floor | p99 budget | Error budget | Measured sustained | Verdict |"
    );
    let _ = writeln!(out, "| --- | --- | --- | --- | --- | --- | --- |");
    let mut ladder: Vec<&PerformanceCase> = cases.iter().collect();
    ladder.sort_by(|a, b| {
        a.class
            .arrival_floor_per_s()
            .total_cmp(&b.class.arrival_floor_per_s())
    });
    for case in &ladder {
        let p99_budget = case
            .thresholds
            .iter()
            .find(|t| matches!(t.metric, crate::perf::Metric::LatencyP99))
            .and_then(|t| t.max)
            .map_or_else(|| "—".to_owned(), |ms| format!("≤ {} ms", format_ms(ms)));
        let error_budget = case
            .thresholds
            .iter()
            .find(|t| matches!(t.metric, crate::perf::Metric::ErrorRate))
            .and_then(|t| t.max)
            .map_or_else(|| "—".to_owned(), |rate| format!("{rate}"));
        let measured = measurements.iter().find(|m| m.class == case.class);
        let (sustained, verdict) = match measured {
            Some(m) => (
                format!("{:.1}/s", m.offered_load_sustained),
                match m.verdict {
                    ClassVerdict::Earned => "**EARNED**".to_owned(),
                    ClassVerdict::NotEarned => "not earned".to_owned(),
                },
            ),
            None => ("—".to_owned(), "not measured".to_owned()),
        };
        let _ = writeln!(
            out,
            "| {} | {} | {}/s | {} | {} | {} | {} |",
            case.class.token(),
            case.corpus.as_str(),
            case.class.arrival_floor_per_s(),
            p99_budget,
            error_budget,
            sustained,
            verdict,
        );
    }
    for m in measurements {
        let _ = writeln!(
            out,
            "\nMeasured run `{}` — class {}, offered load {:.2}/s sustained over {} s (after {} s warmup), environment: {} ({} cores, {} GB, {}, {}).\n",
            m.case,
            m.class.token(),
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
            let histogram = op.decode_histogram()?;
            #[expect(
                clippy::as_conversions,
                clippy::cast_precision_loss,
                reason = "µs << 2^52"
            )]
            let ms = |q: f64| histogram.value_at_quantile(q) as f64 / 1_000.0;
            let _ = writeln!(
                out,
                "| {} | {} | {} | {} | {} | {} |",
                op.operation,
                op.requests,
                op.errors,
                format_ms(ms(0.50)),
                format_ms(ms(0.90)),
                format_ms(ms(0.99)),
            );
        }
        resources_markdown(&mut out, m.resources.as_ref());
    }
    Ok(out)
}

/// The derived resources table of one measured run (peak/mean CPU, peak
/// RSS, the disk anchors — every number derived from the committed series,
/// never stored beside it), or the honest absence line.
fn resources_markdown(out: &mut String, resources: Option<&crate::perf::ResourcesRecord>) {
    let Some(r) = resources else {
        let _ = writeln!(out, "\nResources: not sampled.");
        return;
    };
    let _ = writeln!(
        out,
        "\nResources (measured context, never a verdict input) — sampled every {} s; \
         CPU/RSS derived over the measured phase:\n",
        r.sample_interval_s,
    );
    let _ = writeln!(out, "| Container | CPU mean | CPU peak | RSS peak |");
    let _ = writeln!(out, "| --- | --- | --- | --- |");
    for c in &r.containers {
        let role = c.role.label();
        let set = c.measured_samples();
        if set.is_empty() {
            let _ = writeln!(out, "| {role} `{}` | — | — | — |", c.name);
            continue;
        }
        #[expect(
            clippy::as_conversions,
            clippy::cast_precision_loss,
            reason = "sample counts << 2^52"
        )]
        let cpu_mean = set.iter().map(|s| s.cpu_pct).sum::<f64>() / set.len() as f64;
        let cpu_peak = set.iter().map(|s| s.cpu_pct).fold(0.0, f64::max);
        let rss_peak = set.iter().map(|s| s.rss_bytes).max().unwrap_or(0);
        #[expect(
            clippy::as_conversions,
            clippy::cast_precision_loss,
            reason = "bytes << 2^52"
        )]
        let _ = writeln!(
            out,
            "| {role} `{}` | {cpu_mean:.1}% | {cpu_peak:.1}% | {} |",
            c.name,
            format_bytes(rss_peak as f64),
        );
    }
    // The disk anchors exist only on measured class runs (stress-style
    // records sample containers without them).
    let Some(disk) = r.disk else {
        return;
    };
    #[expect(
        clippy::as_conversions,
        clippy::cast_precision_loss,
        reason = "volume sizes << 2^52"
    )]
    let anchor =
        |v: Option<u64>| v.map_or_else(|| "not probed".to_owned(), |b| format_bytes(b as f64));
    let per_composition = match (
        disk.before_scale_seed_bytes,
        disk.after_scale_seed_bytes,
        disk.seed_compositions,
    ) {
        (Some(before), Some(after), Some(n)) if n > 0 => {
            #[expect(
                clippy::as_conversions,
                clippy::cast_precision_loss,
                reason = "sizes/counts << 2^52"
            )]
            let per = after.saturating_sub(before) as f64 / n as f64;
            format!(
                " (≈ {} / composition over {} committed)",
                format_bytes(per),
                format_count(n)
            )
        }
        _ => String::new(),
    };
    let _ = writeln!(
        out,
        "\nDisk anchors: empty {} → after scale seed {}{per_composition} → after ward seed {} → after window {}.",
        anchor(disk.before_scale_seed_bytes),
        anchor(disk.after_scale_seed_bytes),
        anchor(disk.after_ward_seed_bytes),
        anchor(disk.after_window_bytes),
    );
}

#[cfg(test)]
mod tests {
    use hdrhistogram::Histogram;

    use super::*;
    use crate::perf::{OperationMeasurement, PerfClass};

    fn case(yaml_class: &str, rate: &str) -> PerformanceCase {
        serde_saphyr::from_str(&format!(
            "id: PERF-hospital_sim-class_{yaml_class}\nkind: performance\ncomponent: PERFORMANCE\ndescription: d\ntest_purpose: t\nspec_refs: [\"CNF 2.0 performance schedule\"]\nclass: {yaml_class}\ncorpus: cnf.scale.10k\nworkload:\n  arrival_rate: {rate}\n  warmup: PT5M\n  duration: PT1H\n  journeys: {{ chart_review: 88%, vitals_round: 12% }}\nthresholds:\n  - {{ metric: error_rate, max: 0 }}\n"
        ))
        .unwrap()
    }

    fn measurement() -> Measurement {
        let mut h = Histogram::<u64>::new(3).unwrap();
        for v in [2_000_u64, 5_000, 9_000, 40_000] {
            h.record(v).unwrap();
        }
        let op = OperationMeasurement::from_histogram("composition_read", &h, 0).unwrap();
        Measurement {
            case: crate::ids::CaseId::parse("PERF-hospital_sim-class_POC").unwrap(),
            class: PerfClass::Poc,
            environment: serde_json::from_value(serde_json::json!({
                "hardware_class": "test", "cores": 1, "memory_gb": 1,
                "storage_class": "ram", "topology": "stub"
            }))
            .unwrap(),
            offered_load_sustained: 2.02,
            warmup_s: 300,
            duration_s: 3600,
            operations: vec![op],
            verdict: ClassVerdict::Earned,
            violations: Vec::new(),
            resources: None,
        }
    }

    #[test]
    fn charts_are_deterministic_and_carry_the_data() {
        let cases = [
            case("POC", "2/s"),
            case("S", "15/s"),
            case("L", "150/s"),
            case("R", "1500/s"),
        ];
        let m = [measurement()];
        let ladder_a = class_ladder_svg(&cases, &m);
        let ladder_b = class_ladder_svg(&cases, &m);
        assert_eq!(ladder_a, ladder_b);
        assert!(ladder_a.contains("class POC"));
        assert!(ladder_a.contains(">EARNED<"));
        assert!(ladder_a.contains("EARNED"));
        assert!(ladder_a.contains("not measured")); // the unmeasured classes say so
        assert!(ladder_a.contains("prefers-color-scheme: dark"));

        let latency = latency_percentiles_svg(&m[0]).unwrap();
        assert_eq!(latency, latency_percentiles_svg(&m[0]).unwrap());
        assert!(latency.contains("composition_read"));
        assert!(latency.contains("SLO p99"));
        // p99 of the fixture histogram is 40ms — the label re-derives from
        // the decoded histogram.
        assert!(latency.contains(">40<"));
    }

    fn sample(
        offset_s: u64,
        phase: crate::perf::ResourcePhase,
        cpu_pct: f64,
        rss_bytes: u64,
    ) -> crate::perf::ResourceSample {
        crate::perf::ResourceSample {
            offset_s,
            phase,
            cpu_pct,
            rss_bytes,
            blk_read_bytes: offset_s * 1_000,
            blk_write_bytes: offset_s * 5_000,
            net_rx_bytes: offset_s * 200,
            net_tx_bytes: offset_s * 300,
        }
    }

    fn measurement_with_resources() -> Measurement {
        use crate::perf::ResourcePhase;
        let mut m = measurement();
        m.resources = Some(crate::perf::ResourcesRecord {
            sample_interval_s: 10,
            containers: vec![
                crate::perf::ContainerResourceSeries {
                    role: crate::perf::ContainerRole::Sut,
                    name: "ferroehr-ferroehr-1".to_owned(),
                    samples: vec![
                        sample(10, ResourcePhase::Warmup, 12.0, 400_000_000),
                        sample(310, ResourcePhase::Measured, 55.0, 600_000_000),
                        sample(3910, ResourcePhase::Drain, 5.0, 500_000_000),
                    ],
                },
                crate::perf::ContainerResourceSeries {
                    role: crate::perf::ContainerRole::Db,
                    name: "ferroehr-ferroehr-postgres-1".to_owned(),
                    samples: vec![
                        sample(10, ResourcePhase::Warmup, 30.0, 900_000_000),
                        sample(310, ResourcePhase::Measured, 140.0, 1_400_000_000),
                    ],
                },
            ],
            disk: Some(crate::perf::DiskAnchors {
                before_scale_seed_bytes: Some(64_000_000),
                after_scale_seed_bytes: Some(10_000_000_000),
                after_ward_seed_bytes: None,
                after_window_bytes: Some(11_000_000_000),
                seed_compositions: Some(1_000_000),
            }),
        });
        m
    }

    #[test]
    fn the_resource_charts_are_deterministic_and_honest() {
        // A record without resources renders nothing — never a fabricated
        // chart.
        assert!(resources_timeseries_svg(&measurement()).is_none());
        assert!(disk_growth_svg(&[measurement()]).is_none());

        let m = measurement_with_resources();
        let series = resources_timeseries_svg(&m).unwrap();
        assert_eq!(series, resources_timeseries_svg(&m).unwrap());
        assert!(series.contains("Resource telemetry — class POC measured run"));
        assert!(series.contains("never a verdict input"));
        assert!(series.contains("CPU (% of one core)"));
        assert!(series.contains("Resident memory"));
        assert!(series.contains("Block write"));
        assert!(series.contains("Network transmit"));
        assert!(series.contains("class=\"warm\"")); // the warmup shade
        assert!(series.contains("class=\"sut\""));
        assert!(series.contains("class=\"db\""));
        assert!(series.contains("prefers-color-scheme: dark"));

        let disk = disk_growth_svg(std::slice::from_ref(&m)).unwrap();
        assert_eq!(disk, disk_growth_svg(std::slice::from_ref(&m)).unwrap());
        assert!(disk.contains("Disk growth"));
        assert!(disk.contains("64 MB")); // the empty anchor
        assert!(disk.contains("10 GB"));
        assert!(disk.contains("not probed")); // the absent ward anchor stays honest
        // (10 GB - 64 MB) / 1M compositions ≈ 9.9 KB/composition.
        assert!(disk.contains("/ composition"));
        assert!(disk.contains("9.9 KB"));
    }

    #[test]
    fn the_summary_carries_the_derived_resources_table() {
        let cases = [case("POC", "2/s")];
        // Without a record the summary says so.
        let bare = summary_markdown(&cases, &[measurement()]).unwrap();
        assert!(bare.contains("Resources: not sampled."));

        let with = summary_markdown(&cases, &[measurement_with_resources()]).unwrap();
        assert!(with.contains("| Container | CPU mean | CPU peak | RSS peak |"));
        // The SUT row derives over the measured phase only (one sample:
        // 55% / 600 MB).
        assert!(with.contains("| sut `ferroehr-ferroehr-1` | 55.0% | 55.0% | 600 MB |"));
        assert!(with.contains("Disk anchors: empty 64 MB"));
        assert!(with.contains("not probed"));
        assert!(with.contains("/ composition over 1,000,000 committed"));
    }

    #[test]
    fn the_stress_overlay_is_deterministic_and_two_sided() {
        use crate::stress::{LoadStep, StressReport};
        let step = |rate: f64, p99_us: u64, stable: bool| {
            let mut h = Histogram::<u64>::new(3).unwrap();
            for _ in 0..99 {
                h.record(10_000).unwrap();
            }
            h.record(p99_us).unwrap();
            LoadStep {
                rate,
                offered_load_sustained: rate,
                operations: vec![
                    OperationMeasurement::from_histogram("composition_read", &h, 0).unwrap(),
                ],
                stable,
                breaches: if stable {
                    vec![]
                } else {
                    vec!["p99".to_owned()]
                },
                generator_bound: false,
                resources: None,
            }
        };
        let report = |mst: f64| StressReport {
            corpus: "cnf.scale.10k".to_owned(),
            environment: serde_json::from_value(serde_json::json!({
                "hardware_class": "test", "cores": 1, "memory_gb": 1,
                "storage_class": "ram", "topology": "stub"
            }))
            .unwrap(),
            step_warmup_s: 30,
            step_hold_s: 120,
            p99_budget_ms: 1000.0,
            error_budget: 0.001,
            steps: vec![
                step(2.0, 40_000, true),
                step(mst, 200_000, true),
                step(mst * 2.0, 4_000_000, false),
            ],
            max_sustainable_throughput_per_s: mst,
            ladder_capped: false,
            generator_bound: false,
            remark: "r".to_owned(),
        };
        let ours = report(256.0);
        let theirs = report(512.0); // the comparison side winning is drawn plainly
        let svg = stress_compare_svg(("ferroehr", &ours), ("EHRbase", &theirs)).unwrap();
        assert_eq!(
            svg,
            stress_compare_svg(("ferroehr", &ours), ("EHRbase", &theirs)).unwrap()
        );
        assert!(svg.contains("ferroehr max sustainable 256/s"));
        assert!(svg.contains("EHRbase max sustainable 512/s"));
        assert!(svg.contains("class=\"curve\"") && svg.contains("class=\"cmp\""));
        assert!(svg.contains("prefers-color-scheme: dark"));
    }

    #[test]
    fn byte_and_ceiling_helpers_are_fixed_precision() {
        assert_eq!(format_bytes(0.0), "0 B");
        assert_eq!(format_bytes(999.0), "999 B");
        assert_eq!(format_bytes(1536.0), "1.5 KB");
        assert_eq!(format_bytes(64_000_000.0), "64 MB");
        assert_eq!(format_bytes(10_000_000_000.0), "10 GB");
        assert!((nice_ceil(0.0) - 1.0).abs() < f64::EPSILON);
        assert!((nice_ceil(3.0) - 5.0).abs() < f64::EPSILON);
        assert!((nice_ceil(140.0) - 200.0).abs() < f64::EPSILON);
        assert!((nice_ceil(730.0) - 1000.0).abs() < f64::EPSILON);
    }
}
