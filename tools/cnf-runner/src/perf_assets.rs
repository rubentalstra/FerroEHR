//! Deterministic SVG assets rendered FROM committed measurement records —
//! the published performance visuals derive from `results.json` exactly like
//! the conformance stats derive from the outcomes (no hand-drawn numbers,
//! CI-guarded regeneration).
//!
//! Every chart is a pure function of its inputs: stable ordering, fixed
//! precision, no timestamps. Latency percentiles are RE-DERIVED from the
//! embedded HDR V2 histograms, never read from the summary fields. Both
//! charts style for light and dark via `prefers-color-scheme`.

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

/// The class ladder: every class's offered-load floor (dashed outline)
/// with the measured sustained load and verdict overlaid. Fixed columns —
/// class + floor on the left, the log-scale bars in a fixed plot area, the
/// measured/verdict status right-aligned at a fixed edge — so no label
/// ever chases a bar end or leaves the canvas.
#[must_use]
#[allow(clippy::too_many_lines)] // one linear chart emitter
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
    for (i, line) in [
        "Floors are the request rate the hospital-simulation workload must sustain",
        "against the class corpus; a class is earned only when every measured operation",
        "holds p99 &#8804; 1 s with zero errors.",
    ]
    .iter()
    .enumerate()
    {
        #[allow(clippy::cast_precision_loss)] // 3 caption lines
        let y = 46.0 + i as f64 * 14.0;
        let _ = writeln!(
            out,
            "<text x=\"24\" y=\"{y}\" class=\"muted\">{line}</text>"
        );
    }

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
        #[allow(clippy::cast_precision_loss)] // row counts are tiny
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

/// Per-operation latency percentiles for one measured run — a VERTICAL
/// layout that grows organically with the operation count (the journey
/// workload measures ~20+ operations; a horizontal grouping cannot fit):
/// one row per operation, three horizontal bars (p50/p90/p99, one hue
/// light→dark), latency on a log x-axis in milliseconds, the p99 ≤ 1 s
/// SLO as a vertical line, the p99 value printed at its bar end.
/// Percentiles re-derived from the decoded histograms.
///
/// # Errors
/// A message when a histogram fails to decode.
#[allow(clippy::too_many_lines)] // one linear chart emitter
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
    #[allow(clippy::cast_precision_loss)] // operation counts are tiny
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
        #[allow(clippy::cast_precision_loss)] // row counts are tiny
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
            #[allow(clippy::cast_precision_loss)] // us << 2^52
            let ms = histogram.value_at_quantile(*quantile) as f64 / 1_000.0;
            #[allow(clippy::cast_precision_loss)] // 3 bars per row
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

/// The latency-throughput curve from a committed stress report: offered rate
/// (x, log) vs the worst per-operation p99 (y, log, re-derived from the
/// decoded histograms), stable and breached steps distinguished by mark AND
/// color, the p99 budget line, the class floors as context verticals, and
/// the maximum-sustainable-throughput marker.
///
/// # Errors
/// A message when a histogram fails to decode.
#[allow(clippy::too_many_lines)] // one linear chart emitter
pub fn stress_curve_svg(report: &crate::stress::StressReport) -> Result<String, String> {
    let (width, height) = (760.0, 400.0);
    let (x0, x1) = (90.0, 700.0);
    let (y_top, y_bottom) = (64.0, 330.0);
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
        "<text x=\"24\" y=\"44\" class=\"muted\">Short intense load steps ({} s hold) on the {} corpus · exploration only — classes are earned exclusively by the hour-long class runs</text>",
        report.step_hold_s, report.corpus,
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
    // Class floors as CONTEXT verticals.
    for floor in &report.floors_context {
        let x = x_of(floor.floor_per_s);
        let _ = writeln!(
            out,
            "<line x1=\"{x:.1}\" y1=\"{y_top}\" x2=\"{x:.1}\" y2=\"{y_bottom}\" class=\"floor\"/>\
             <text x=\"{x:.1}\" y=\"{:.1}\" class=\"muted\" text-anchor=\"middle\">{}</text>",
            y_top - 6.0,
            floor.class.token(),
        );
    }

    // Steps, in rate order: worst per-operation p99 per step (re-derived).
    let mut points: Vec<(f64, f64, bool)> = Vec::new();
    for step in &report.steps {
        let mut worst_ms: f64 = 0.0;
        for op in &step.operations {
            let histogram = op.decode_histogram()?;
            #[allow(clippy::cast_precision_loss)] // latencies << 2^52 µs
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
        "<path d=\"{}\" fill=\"none\" class=\"s1s\" stroke-width=\"2\"/>",
        path.join(" ")
    );
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
    // The maximum-sustainable-throughput marker.
    let mst_x = x_of(report.max_sustainable_throughput_per_s.max(min_rate));
    let _ = writeln!(
        out,
        "<line x1=\"{mst_x:.1}\" y1=\"{y_top}\" x2=\"{mst_x:.1}\" y2=\"{y_bottom}\" class=\"s2s\" stroke-width=\"2\" stroke-dasharray=\"2 3\"/>\
         <text x=\"{:.1}\" y=\"{:.1}\" class=\"earned\">max sustainable {:.0}/s</text>",
        mst_x + 6.0,
        y_top + 14.0,
        report.max_sustainable_throughput_per_s,
    );
    out.push_str("</svg>\n");
    Ok(out)
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

/// The generated Markdown summary the book includes at build time: the
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
            #[allow(clippy::cast_precision_loss)] // µs << 2^52
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
    }
    Ok(out)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic)] // test assertions/fixtures
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
}
