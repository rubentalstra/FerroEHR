//! Generated SVG charts embedded by the Markdown artefacts.
//!
//! Hand-rolled SVG (no plotting dependency): the charts are deterministic
//! text files committed beside `REPORT.md`/`COMPARISON.md`, rendering on
//! GitHub and in the docs book. Colors are the validated reference palette
//! (2-hue categorical: blue = slot 1, aqua = slot 2; CVD ΔE 73.6 light /
//! 69.8 dark — validator-passed for both surfaces) with a
//! `prefers-color-scheme` swap inside each SVG; every mark carries a direct
//! value label, and the Markdown always keeps the full percentile table (the
//! relief rule for the one sub-3:1 light-mode hue).
//!
//! Latency axes are log₁₀ over microseconds: request latencies span three-plus
//! decades and a linear axis would flatten every class below the tail.

use std::collections::BTreeMap;

use super::json::{ClassRecord, ContainerSummary};

/// Per-SVG stylesheet: text/grid tokens + the two categorical slots, themed
/// for light and dark via `prefers-color-scheme`.
const STYLE: &str = "<style>\n\
  text { fill: #52514e; font: 12px -apple-system, 'Segoe UI', Helvetica, Arial, sans-serif; }\n\
  .title { fill: #0b0b0b; font-weight: 600; }\n\
  .muted { fill: #8a8880; font-size: 11px; }\n\
  .grid { stroke: #e4e2dd; stroke-width: 1; }\n\
  .s1 { fill: #2a78d6; } .s1s { stroke: #2a78d6; }\n\
  .s2 { fill: #1baf7a; } .s2s { stroke: #1baf7a; }\n\
  @media (prefers-color-scheme: dark) {\n\
    text { fill: #c3c2b7; }\n\
    .title { fill: #ffffff; }\n\
    .muted { fill: #8f8e85; }\n\
    .grid { stroke: #3a3a38; }\n\
    .s1 { fill: #3987e5; } .s1s { stroke: #3987e5; }\n\
    .s2 { fill: #199e70; } .s2s { stroke: #199e70; }\n\
  }\n\
</style>\n";

/// Format a µs latency for a direct label (µs → ms → s at decade dignity).
fn fmt_us(us: u64) -> String {
    if us >= 1_000_000 {
        format!("{:.1}s", us as f64 / 1_000_000.0)
    } else if us >= 1_000 {
        format!("{:.1}ms", us as f64 / 1_000.0)
    } else {
        format!("{us}µs")
    }
}

/// log₁₀ position of `us` between `lo` and `hi` mapped onto `[x0, x1]`.
fn log_x(us: u64, lo: f64, hi: f64, x0: f64, x1: f64) -> f64 {
    let v = (us.max(1) as f64).log10().clamp(lo, hi);
    x0 + (v - lo) / (hi - lo) * (x1 - x0)
}

/// Gridline positions for a log axis spanning the exponents `[lo, hi]`
/// (as computed by the charts: `lo = floor(log10(min))`,
/// `hi = ceil(log10(max))`): every decade `10^n`, plus the 2× and 5× steps
/// when the span is two decades or fewer (a narrow span would otherwise show
/// a single line at the edge).
// The clamp to [0, 12] before rounding makes truncation and sign loss
// impossible — the axis exponents are small non-negative decades.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn gridlines(lo: f64, hi: f64) -> Vec<u64> {
    let span = (hi - lo).max(1.0);
    let mut out = Vec::new();
    // Axis exponents are small non-negative decades (µs → minutes ≈ 0..=8);
    // clamp defensively before the integer walk.
    let mut n = lo.clamp(0.0, 12.0).round() as u32;
    let top = hi.clamp(0.0, 12.0).round() as u32;
    while n <= top {
        let d = 10u64.saturating_pow(n);
        out.push(d.max(1));
        if span <= 2.0 && n < top {
            out.push(d.saturating_mul(2));
            out.push(d.saturating_mul(5));
        }
        n += 1;
    }
    out
}

/// XML-escape a label.
fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;")
}

/// Per-class latency ranges (p50 → p99.9 bar, p99 tick) on a log µs axis,
/// one hue, direct labels at both ends.
#[must_use]
pub fn latency_chart(classes: &BTreeMap<String, ClassRecord>) -> String {
    let rows: Vec<(&str, &ClassRecord)> = classes
        .iter()
        .filter(|(_, r)| r.count > 0)
        .map(|(k, r)| (k.as_str(), r))
        .collect();
    if rows.is_empty() {
        return String::new();
    }
    let lo_us = rows.iter().map(|(_, r)| r.p50_us).min().unwrap_or(1);
    let hi_us = rows.iter().map(|(_, r)| r.p999_us).max().unwrap_or(1);
    let lo = (lo_us.max(1) as f64).log10().floor();
    let hi = (hi_us.max(2) as f64).log10().ceil();
    let (x0, x1) = (170.0, 640.0);
    let row_h = 26.0;
    let top = 42.0;
    let height = top + rows.len() as f64 * row_h + 34.0;
    let width = 760.0;

    let mut s = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {width} {height}\" \
         role=\"img\" aria-label=\"Latency ranges per operation class\">\n{STYLE}"
    );
    s.push_str("<text x=\"16\" y=\"22\" class=\"title\">Latency by operation class — p50 → p99.9 (log scale)</text>\n");
    for d in gridlines(lo, hi) {
        let x = log_x(d, lo, hi, x0, x1);
        s.push_str(&format!(
            "<line class=\"grid\" x1=\"{x:.1}\" y1=\"{top}\" x2=\"{x:.1}\" y2=\"{:.1}\"/>\n\
             <text class=\"muted\" x=\"{x:.1}\" y=\"{:.1}\" text-anchor=\"middle\">{}</text>\n",
            height - 26.0,
            height - 10.0,
            fmt_us(d)
        ));
    }
    for (i, (name, r)) in rows.iter().enumerate() {
        let y = top + i as f64 * row_h + row_h / 2.0;
        let bx0 = log_x(r.p50_us, lo, hi, x0, x1);
        let bx1 = log_x(r.p999_us, lo, hi, x0, x1).max(bx0 + 2.0);
        let p99x = log_x(r.p99_us, lo, hi, x0, x1);
        s.push_str(&format!(
            "<text x=\"{:.1}\" y=\"{:.1}\" text-anchor=\"end\">{}</text>\n",
            x0 - 10.0,
            y + 4.0,
            esc(name)
        ));
        s.push_str(&format!(
            "<rect class=\"s1\" x=\"{bx0:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"8\" rx=\"4\"/>\n",
            y - 4.0,
            bx1 - bx0
        ));
        s.push_str(&format!(
            "<line class=\"s1s\" x1=\"{p99x:.1}\" y1=\"{:.1}\" x2=\"{p99x:.1}\" y2=\"{:.1}\" stroke-width=\"2\"/>\n",
            y - 8.0,
            y + 8.0
        ));
        s.push_str(&format!(
            "<text class=\"muted\" x=\"{:.1}\" y=\"{:.1}\" text-anchor=\"end\">{}</text>\n",
            bx0 - 4.0,
            y + 4.0,
            fmt_us(r.p50_us)
        ));
        s.push_str(&format!(
            "<text class=\"muted\" x=\"{:.1}\" y=\"{:.1}\">{}</text>\n",
            bx1 + 4.0,
            y + 4.0,
            fmt_us(r.p999_us)
        ));
    }
    s.push_str("</svg>\n");
    s
}

/// A resource time series (one measure, one axis): app + db lines with a
/// legend and end-of-line direct labels. `pick` selects the measure;
/// `fmt_y` renders an axis/label value.
fn series_chart(
    title: &str,
    app: Option<&ContainerSummary>,
    db: Option<&ContainerSummary>,
    pick: impl Fn(&crate::sample::ResourceSample) -> f64,
    fmt_y: impl Fn(f64) -> String,
) -> Option<String> {
    let series: Vec<(&str, &ContainerSummary, &str)> =
        [app.map(|c| ("app", c, "s1")), db.map(|c| ("db", c, "s2"))]
            .into_iter()
            .flatten()
            .collect();
    lines_chart(title, series, pick, fmt_y)
}

/// Cross-SUT overlay of one container measure over the run (slot per SUT).
pub fn overlay_series_chart(
    title: &str,
    entries: &[(String, ContainerSummary)],
    pick: impl Fn(&crate::sample::ResourceSample) -> f64,
    fmt_y: impl Fn(f64) -> String,
) -> Option<String> {
    let series: Vec<(&str, &ContainerSummary, &str)> = entries
        .iter()
        .take(2)
        .enumerate()
        .map(|(i, (name, c))| (name.as_str(), c, if i == 0 { "s1" } else { "s2" }))
        .collect();
    lines_chart(title, series, pick, fmt_y)
}

/// Estimated rendered width of `text` at the chart's 12px system font. SVG has
/// no layout engine to ask, so a conservative (wide) per-character factor is
/// used for every placement decision — over-estimating costs a few pixels of
/// whitespace, under-estimating produces the overlap this exists to prevent.
fn text_w(text: &str) -> f64 {
    text.chars().count() as f64 * 7.2
}

/// One legend item's footprint: 10px chip + 6px gap + label.
fn legend_item_w(label: &str) -> f64 {
    16.0 + text_w(label)
}

/// Gap between legend items.
const LEGEND_GAP: f64 = 20.0;

/// Emit a legend row starting at `x`, chips at `chip_y` (labels baseline
/// `chip_y + 9`), spaced by each label's measured width — never a fixed step.
fn legend_row(s: &mut String, items: &[(&str, &str)], x: f64, chip_y: f64) {
    let mut lx = x;
    for (label, cls) in items {
        s.push_str(&format!(
            "<rect class=\"{cls}\" x=\"{lx:.1}\" y=\"{chip_y:.1}\" width=\"10\" height=\"10\" \
             rx=\"2\"/>\n<text x=\"{:.1}\" y=\"{:.1}\">{}</text>\n",
            lx + 16.0,
            chip_y + 9.0,
            esc(label)
        ));
        lx += legend_item_w(label) + LEGEND_GAP;
    }
}

/// The shared line-chart body (one measure, one axis, ≤2 series with legend +
/// end-of-line direct labels). Layout is collision-free by construction: the
/// legend is right-aligned on the title band when the measured widths fit with
/// clearance, else it drops to its own row (the chart grows); the right margin
/// is sized to the longest direct label; two direct labels ending close
/// together are pushed apart vertically.
fn lines_chart(
    title: &str,
    series: Vec<(&str, &ContainerSummary, &str)>,
    pick: impl Fn(&crate::sample::ResourceSample) -> f64,
    fmt_y: impl Fn(f64) -> String,
) -> Option<String> {
    let series: Vec<(&str, &ContainerSummary, &str)> = series
        .into_iter()
        .filter(|(_, c, _)| c.series.len() >= 2)
        .collect();
    if series.is_empty() {
        return None;
    }
    let t_max = series
        .iter()
        .flat_map(|(_, c, _)| c.series.iter().map(|p| p.t_ms))
        .max()
        .unwrap_or(1) as f64;
    let y_max = series
        .iter()
        .flat_map(|(_, c, _)| c.series.iter().map(&pick))
        .fold(1.0_f64, f64::max);
    let width = 760.0;
    // Right margin fits the longest direct label (8px gap + label + 8px edge).
    let label_margin = series
        .iter()
        .map(|(label, _, _)| text_w(label))
        .fold(0.0_f64, f64::max)
        + 16.0;
    let x0 = 64.0;
    let x1 = width - label_margin.max(24.0);
    // Legend beside the title only when the measured widths leave ≥24px between
    // the title's end and the right-aligned legend block; else its own row.
    let legend_items: Vec<(&str, &str)> = series.iter().map(|(l, _, c)| (*l, *c)).collect();
    let legend_w = legend_items
        .iter()
        .map(|(l, _)| legend_item_w(l))
        .sum::<f64>()
        + LEGEND_GAP * (legend_items.len().saturating_sub(1)) as f64;
    let title_end = 16.0 + text_w(title) + 24.0;
    let legend_inline = x1 - legend_w >= title_end;
    let head_extra = if legend_inline { 0.0 } else { 22.0 };
    let (y0, y1) = (40.0 + head_extra, 190.0 + head_extra);
    let height = 232.0 + head_extra;

    let mut s = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {width} {height}\" \
         role=\"img\" aria-label=\"{}\">\n{STYLE}",
        esc(title)
    );
    s.push_str(&format!(
        "<text x=\"16\" y=\"22\" class=\"title\">{}</text>\n",
        esc(title)
    ));
    if legend_inline {
        legend_row(&mut s, &legend_items, x1 - legend_w, 14.0);
    } else {
        legend_row(&mut s, &legend_items, x0, 32.0);
    }
    for frac in [0.0_f64, 0.5, 1.0] {
        let y = y1 - frac * (y1 - y0);
        s.push_str(&format!(
            "<line class=\"grid\" x1=\"{x0}\" y1=\"{y:.1}\" x2=\"{x1:.1}\" y2=\"{y:.1}\"/>\n\
             <text class=\"muted\" x=\"{:.1}\" y=\"{:.1}\" text-anchor=\"end\">{}</text>\n",
            x0 - 8.0,
            y + 4.0,
            fmt_y(frac * y_max)
        ));
    }
    s.push_str(&format!(
        "<text class=\"muted\" x=\"{x1:.1}\" y=\"{:.1}\" text-anchor=\"end\">{:.0} min</text>\n",
        y1 + 18.0,
        t_max / 60_000.0
    ));
    // Direct end-of-line labels: collected first, then pushed apart so two
    // lines ending close together never overprint (12px font → 14px minimum
    // separation), clamped to the plot band.
    let mut direct: Vec<(f64, &str)> = Vec::new();
    for (label, c, cls) in &series {
        let pts: String = c
            .series
            .iter()
            .map(|p| {
                let x = x0 + (p.t_ms as f64 / t_max) * (x1 - x0);
                let y = y1 - (pick(p) / y_max).min(1.0) * (y1 - y0);
                format!("{x:.1},{y:.1}")
            })
            .collect::<Vec<_>>()
            .join(" ");
        s.push_str(&format!(
            "<polyline class=\"{cls}s\" fill=\"none\" stroke-width=\"2\" points=\"{pts}\"/>\n"
        ));
        if let Some(last) = c.series.last() {
            let y = y1 - (pick(last) / y_max).min(1.0) * (y1 - y0);
            direct.push((y + 4.0, label));
        }
    }
    direct.sort_by(|a, b| a.0.total_cmp(&b.0));
    for i in 1..direct.len() {
        if direct[i].0 - direct[i - 1].0 < 14.0 {
            direct[i].0 = direct[i - 1].0 + 14.0;
        }
    }
    for &(y, label) in &direct {
        s.push_str(&format!(
            "<text x=\"{:.1}\" y=\"{:.1}\">{}</text>\n",
            x1 + 8.0,
            y.min(y1 + 14.0),
            esc(label)
        ));
    }
    s.push_str("</svg>\n");
    Some(s)
}

/// CPU% over the run (app + db).
#[must_use]
pub fn cpu_chart(app: Option<&ContainerSummary>, db: Option<&ContainerSummary>) -> Option<String> {
    series_chart(
        "CPU over the run (%)",
        app,
        db,
        |p| p.cpu_pct,
        |v| format!("{v:.0}%"),
    )
}

/// Memory (RSS) over the run (app + db).
#[must_use]
pub fn rss_chart(app: Option<&ContainerSummary>, db: Option<&ContainerSummary>) -> Option<String> {
    series_chart(
        "Memory (RSS) over the run",
        app,
        db,
        |p| p.mem_bytes as f64,
        |v| format!("{:.0} MB", v / 1_048_576.0),
    )
}

/// Cross-SUT grouped bars for one percentile metric per operation class
/// (log µs axis; slot 1 = the first SUT, slot 2 = the second; 2px gap
/// between grouped bars; direct labels; legend).
#[must_use]
pub fn comparison_chart(title: &str, suts: &[(String, BTreeMap<String, u64>)]) -> String {
    if suts.len() < 2 {
        return String::new();
    }
    // Only classes measured on every SUT are comparable.
    let classes: Vec<&String> = suts[0]
        .1
        .keys()
        .filter(|k| suts.iter().all(|(_, m)| m.contains_key(*k)))
        .collect();
    let all: Vec<u64> = suts
        .iter()
        .flat_map(|(_, m)| m.values().copied())
        .filter(|v| *v > 0)
        .collect();
    let (Some(&lo_us), Some(&hi_us)) = (all.iter().min(), all.iter().max()) else {
        return String::new();
    };
    let lo = (lo_us.max(1) as f64).log10().floor();
    let hi = (hi_us.max(2) as f64).log10().ceil();
    let (x0, x1) = (170.0, 620.0);
    let bar_h = 10.0;
    let group_h = suts.len().min(2) as f64 * (bar_h + 2.0) + 10.0;
    let top = 58.0;
    let height = top + classes.len() as f64 * group_h + 34.0;
    let width = 760.0;

    let mut s = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {width} {height}\" \
         role=\"img\" aria-label=\"{}\">\n{STYLE}",
        esc(title)
    );
    s.push_str(&format!(
        "<text x=\"16\" y=\"22\" class=\"title\">{} (log scale)</text>\n",
        esc(title)
    ));
    // Legend: color follows the SUT, fixed slot order; items spaced by each
    // name's measured width (never a fixed step — SUT names are arbitrary).
    let legend_items: Vec<(&str, &str)> = suts
        .iter()
        .take(2)
        .enumerate()
        .map(|(i, (name, _))| (name.as_str(), if i == 0 { "s1" } else { "s2" }))
        .collect();
    legend_row(&mut s, &legend_items, 170.0, 30.0);
    for d in gridlines(lo, hi) {
        let x = log_x(d, lo, hi, x0, x1);
        s.push_str(&format!(
            "<line class=\"grid\" x1=\"{x:.1}\" y1=\"{top}\" x2=\"{x:.1}\" y2=\"{:.1}\"/>\n\
             <text class=\"muted\" x=\"{x:.1}\" y=\"{:.1}\" text-anchor=\"middle\">{}</text>\n",
            height - 26.0,
            height - 10.0,
            fmt_us(d)
        ));
    }
    for (gi, class) in classes.iter().enumerate() {
        let gy = top + gi as f64 * group_h;
        s.push_str(&format!(
            "<text x=\"{:.1}\" y=\"{:.1}\" text-anchor=\"end\">{}</text>\n",
            x0 - 10.0,
            gy + group_h / 2.0,
            esc(class)
        ));
        for (i, (_, m)) in suts.iter().take(2).enumerate() {
            let Some(&v) = m.get(*class) else { continue };
            let cls = if i == 0 { "s1" } else { "s2" };
            let y = gy + 4.0 + i as f64 * (bar_h + 2.0);
            let bx = log_x(v.max(1), lo, hi, x0, x1).max(x0 + 2.0);
            s.push_str(&format!(
                "<rect class=\"{cls}\" x=\"{x0}\" y=\"{y:.1}\" width=\"{:.1}\" height=\"{bar_h}\" rx=\"4\"/>\n\
                 <text class=\"muted\" x=\"{:.1}\" y=\"{:.1}\">{}</text>\n",
                bx - x0,
                bx + 4.0,
                y + bar_h - 1.0,
                fmt_us(v)
            ));
        }
    }
    s.push_str("</svg>\n");
    s
}

/// A linear-scale horizontal bar pair for one scalar metric (throughput,
/// memory, cold start …): slot per SUT, direct value labels, no axis clutter.
#[must_use]
pub fn metric_bar_chart(
    title: &str,
    entries: &[(String, f64)],
    fmt: impl Fn(f64) -> String,
) -> String {
    if entries.len() < 2 {
        return String::new();
    }
    let max = entries
        .iter()
        .map(|(_, v)| *v)
        .fold(f64::MIN_POSITIVE, f64::max);
    let (x0, x1) = (170.0, 620.0);
    let row_h = 26.0;
    let top = 42.0;
    let height = top + entries.len().min(2) as f64 * row_h + 16.0;
    let width = 760.0;
    let mut s = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {width} {height}\" \
         role=\"img\" aria-label=\"{}\">\n{STYLE}",
        esc(title)
    );
    s.push_str(&format!(
        "<text x=\"16\" y=\"22\" class=\"title\">{}</text>\n",
        esc(title)
    ));
    for (i, (name, v)) in entries.iter().take(2).enumerate() {
        let cls = if i == 0 { "s1" } else { "s2" };
        let y = top + i as f64 * row_h;
        let bw = ((v / max).max(0.0) * (x1 - x0)).max(2.0);
        s.push_str(&format!(
            "<text x=\"{:.1}\" y=\"{:.1}\" text-anchor=\"end\">{}</text>\n\
             <rect class=\"{cls}\" x=\"{x0}\" y=\"{y:.1}\" width=\"{bw:.1}\" height=\"12\" rx=\"4\"/>\n\
             <text class=\"muted\" x=\"{:.1}\" y=\"{:.1}\">{}</text>\n",
            x0 - 10.0,
            y + 10.0,
            esc(name),
            x0 + bw + 6.0,
            y + 10.0,
            fmt(*v)
        ));
    }
    s.push_str("</svg>\n");
    s
}

/// The knee/saturation curve (register 01 §3): the offered-load ladder plotted
/// as sustained req/s (x, linear) against p99 latency (y, **log₁₀ µs** — three-
/// plus decades), each point directly labelled with its load factor `L`. Points
/// are `(rps, p99_us, load_factor)` in ladder order; the 1 s p99 SLO ceiling is
/// drawn as a reference gridline when it falls in range.
#[must_use]
pub fn knee_chart(points: &[(f64, u64, f64)]) -> String {
    if points.is_empty() {
        return String::new();
    }
    let rps_max = points
        .iter()
        .map(|(r, _, _)| *r)
        .fold(f64::MIN_POSITIVE, f64::max);
    let lo_us = points.iter().map(|(_, p, _)| *p).min().unwrap_or(1);
    let hi_us = points.iter().map(|(_, p, _)| *p).max().unwrap_or(1);
    let lo = (lo_us.max(1) as f64).log10().floor();
    let hi = (hi_us.max(2) as f64).log10().ceil();
    let (x0, x1, y0, y1) = (64.0, 660.0, 48.0, 210.0);
    let (width, height) = (760.0, 252.0);

    // Map an (rps, p99) point to chart coordinates.
    let px = |rps: f64| x0 + (rps / rps_max).clamp(0.0, 1.0) * (x1 - x0);
    let py = |us: u64| {
        let v = (us.max(1) as f64).log10().clamp(lo, hi);
        y1 - (v - lo) / (hi - lo).max(f64::MIN_POSITIVE) * (y1 - y0)
    };

    let mut s = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {width} {height}\" \
         role=\"img\" aria-label=\"Knee/saturation curve — sustained req/s vs p99 latency\">\n{STYLE}"
    );
    s.push_str(
        "<text x=\"16\" y=\"22\" class=\"title\">Knee — sustained req/s vs p99 latency (log scale)</text>\n",
    );
    // Y (log µs) gridlines + labels, including the 1 s SLO ceiling as a marker.
    for d in gridlines(lo, hi) {
        let y = py(d);
        s.push_str(&format!(
            "<line class=\"grid\" x1=\"{x0}\" y1=\"{y:.1}\" x2=\"{x1}\" y2=\"{y:.1}\"/>\n\
             <text class=\"muted\" x=\"{:.1}\" y=\"{:.1}\" text-anchor=\"end\">{}</text>\n",
            x0 - 8.0,
            y + 4.0,
            fmt_us(d)
        ));
    }
    if (lo..=hi).contains(&6.0) {
        let y = py(1_000_000);
        s.push_str(&format!(
            "<line class=\"s2s\" x1=\"{x0}\" y1=\"{y:.1}\" x2=\"{x1}\" y2=\"{y:.1}\" stroke-dasharray=\"4 3\"/>\n\
             <text class=\"muted\" x=\"{:.1}\" y=\"{:.1}\">SLO 1s</text>\n",
            x1 - 40.0,
            y - 4.0
        ));
    }
    // X axis label (req/s at the right edge).
    s.push_str(&format!(
        "<text class=\"muted\" x=\"{x1}\" y=\"{:.1}\" text-anchor=\"end\">{rps_max:.0} req/s</text>\n",
        y1 + 18.0
    ));
    // The curve (ladder order) + per-point markers with L + p99 labels.
    let pts: String = points
        .iter()
        .map(|(rps, us, _)| format!("{:.1},{:.1}", px(*rps), py(*us)))
        .collect::<Vec<_>>()
        .join(" ");
    s.push_str(&format!(
        "<polyline class=\"s1s\" fill=\"none\" stroke-width=\"2\" points=\"{pts}\"/>\n"
    ));
    for (rps, us, lf) in points {
        let (x, y) = (px(*rps), py(*us));
        s.push_str(&format!(
            "<circle class=\"s1\" cx=\"{x:.1}\" cy=\"{y:.1}\" r=\"3.5\"/>\n\
             <text x=\"{:.1}\" y=\"{:.1}\">L={lf}</text>\n\
             <text class=\"muted\" x=\"{:.1}\" y=\"{:.1}\">{}</text>\n",
            x + 6.0,
            y - 4.0,
            x + 6.0,
            y + 12.0,
            fmt_us(*us)
        ));
    }
    s.push_str("</svg>\n");
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(p50: u64, p99: u64, p999: u64) -> ClassRecord {
        ClassRecord {
            count: 10,
            errors: 0,
            dep_misses: 0,
            p50_us: p50,
            p90_us: p50,
            p99_us: p99,
            p999_us: p999,
            max_us: p999,
            histogram: String::new(),
        }
    }

    #[test]
    fn latency_chart_has_bars_labels_and_theme() {
        let mut classes = BTreeMap::new();
        classes.insert(
            "comp-create-small".to_owned(),
            record(15_000, 40_000, 90_000),
        );
        classes.insert("aql-ward".to_owned(), record(9_000, 20_000, 30_000));
        let svg = latency_chart(&classes);
        assert!(svg.contains("comp-create-small"));
        assert!(svg.contains("prefers-color-scheme: dark"));
        assert!(svg.contains("15.0ms"), "direct p50 label present");
        assert!(svg.matches("<rect class=\"s1\"").count() == 2);
    }

    #[test]
    fn log_scale_is_monotonic() {
        let a = log_x(1_000, 3.0, 6.0, 0.0, 100.0);
        let b = log_x(10_000, 3.0, 6.0, 0.0, 100.0);
        let c = log_x(100_000, 3.0, 6.0, 0.0, 100.0);
        assert!(a < b && b < c);
    }

    #[test]
    fn comparison_chart_carries_both_suts() {
        let mut a = BTreeMap::new();
        a.insert("comp-create-small".to_owned(), 12_000u64);
        let mut b = BTreeMap::new();
        b.insert("comp-create-small".to_owned(), 48_000u64);
        let svg = comparison_chart(
            "p99 latency — ehrbase-rs vs ehrbase-java",
            &[("ehrbase-rs".to_owned(), a), ("ehrbase-java".to_owned(), b)],
        );
        assert!(svg.contains("ehrbase-rs") && svg.contains("ehrbase-java"));
        assert!(svg.contains("class=\"s1\"") && svg.contains("class=\"s2\""));
        assert!(svg.contains("12.0ms") && svg.contains("48.0ms"));
    }

    #[test]
    fn empty_inputs_render_nothing() {
        assert!(latency_chart(&BTreeMap::new()).is_empty());
        assert!(comparison_chart("t", &[]).is_empty());
        assert!(knee_chart(&[]).is_empty());
    }

    fn summary(name: &str, mem: &[u64]) -> ContainerSummary {
        ContainerSummary {
            name: name.to_owned(),
            idle_rss: None,
            peak_rss: mem.iter().copied().max().unwrap_or(0),
            mean_cpu: 1.0,
            series: mem
                .iter()
                .enumerate()
                .map(|(i, m)| crate::sample::ResourceSample {
                    t_ms: i as u64 * 1000,
                    cpu_pct: 1.0,
                    mem_bytes: *m,
                })
                .collect(),
        }
    }

    /// Every `<rect …><text …>` legend pair and the title, as (x, estimated
    /// end-x, y) extents parsed back out of the emitted SVG.
    fn text_extents(svg: &str) -> Vec<(f64, f64, f64)> {
        let mut out = Vec::new();
        for line in svg.lines() {
            let Some(tstart) = line.find("<text ") else {
                continue;
            };
            let frag = &line[tstart..];
            let attr = |name: &str| -> Option<f64> {
                let key = format!("{name}=\"");
                let s = frag.find(&key)? + key.len();
                let e = frag[s..].find('"')? + s;
                frag[s..e].parse().ok()
            };
            let (Some(x), Some(y)) = (attr("x"), attr("y")) else {
                continue;
            };
            // Right-anchored texts extend LEFT of x — not band-collision
            // candidates for this guard (axis labels).
            if frag.contains("text-anchor=\"end\"") {
                continue;
            }
            let content = frag
                .split_once('>')
                .and_then(|(_, rest)| rest.split_once('<'))
                .map(|(c, _)| c)
                .unwrap_or_default();
            out.push((x, x + text_w(content), y));
        }
        out
    }

    /// The regression for the legend/title collisions: with a long title AND
    /// long series names, nothing on the same text band may overlap, and the
    /// two direct end-of-line labels of near-identical series stay ≥14px apart.
    #[test]
    fn lines_chart_layout_never_overlaps() {
        let a = summary("ehrbase-rs-with-a-long-name", &[100_000_000; 20]);
        let b = summary("ehrbase-java-2.34.0-upstream", &[100_500_000; 20]);
        let entries = vec![
            ("ehrbase-rs-with-a-long-name".to_owned(), a.clone()),
            ("ehrbase-java-2.34.0-upstream".to_owned(), b.clone()),
        ];
        let svg = overlay_series_chart(
            "App memory (RSS) over the run — a deliberately long title",
            &entries,
            |p| p.mem_bytes as f64,
            |v| format!("{:.0} MB", v / 1_048_576.0),
        )
        .expect("chart renders");

        // Same-band horizontal overlap check across every left-anchored text
        // (title, legend labels, direct labels) + the legend chips.
        let texts = text_extents(&svg);
        assert!(texts.len() >= 5, "title + 2 legend + 2 direct labels");
        for (i, a) in texts.iter().enumerate() {
            for b in texts.iter().skip(i + 1) {
                let same_band = (a.2 - b.2).abs() < 12.0;
                let h_overlap = a.0 < b.1 && b.0 < a.1;
                assert!(
                    !(same_band && h_overlap),
                    "text overlap: ({a:?}) vs ({b:?}) in\n{svg}"
                );
            }
        }

        // Direct labels of near-identical series are pushed ≥14px apart. They
        // are the texts at the plot's right edge — the maximal x among all
        // left-anchored texts.
        let max_x = texts.iter().map(|(x, _, _)| *x).fold(0.0_f64, f64::max);
        let direct: Vec<f64> = texts
            .iter()
            .filter(|(x, _, _)| (*x - max_x).abs() < 0.5)
            .map(|(_, _, y)| *y)
            .collect();
        assert_eq!(direct.len(), 2, "both direct labels present");
        assert!(
            (direct[0] - direct[1]).abs() >= 14.0,
            "direct labels separated: {direct:?}"
        );
    }

    /// Short title + short names still fit on one band (the legend stays
    /// inline, the chart does not grow).
    #[test]
    fn lines_chart_inline_legend_when_it_fits() {
        let entries = vec![
            ("app".to_owned(), summary("app", &[1_000; 5])),
            ("db".to_owned(), summary("db", &[2_000; 5])),
        ];
        let svg = overlay_series_chart("CPU", &entries, |p| p.cpu_pct, |v| format!("{v:.0}%"))
            .expect("chart renders");
        assert!(
            svg.contains("viewBox=\"0 0 760 232\""),
            "no extra legend row for a short title: {svg}"
        );
    }

    #[test]
    fn knee_chart_labels_every_step_and_draws_the_slo() {
        let svg = knee_chart(&[
            (10.0, 5_000, 1.0),
            (20.0, 40_000, 2.0),
            (25.0, 1_200_000, 4.0),
        ]);
        // Per-point load-factor labels and the log-µs latency labels.
        assert!(svg.contains("L=1") && svg.contains("L=2") && svg.contains("L=4"));
        assert!(svg.contains("5.0ms") && svg.contains("40.0ms") && svg.contains("1.2s"));
        // The 1 s SLO ceiling is in range (max p99 is 1.2 s) → drawn.
        assert!(svg.contains("SLO 1s"));
        assert!(svg.contains("prefers-color-scheme: dark"));
        assert!(svg.contains("25 req/s"));
    }
}
