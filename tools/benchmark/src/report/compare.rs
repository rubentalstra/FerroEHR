//! The cross-SUT comparison artefact (`bench compare`): a side-by-side
//! Markdown matrix + grouped-bar SVG charts generated from two committed
//! `results.json` records. Measured numbers only, both directions — the
//! "where the other side wins" section is computed, never editorial
//! (`docs/design/benchmarking.md` §7).

use std::collections::BTreeMap;

use super::chart;
use super::json::Results;

/// A rendered comparison: the Markdown text and the SVG files it embeds
/// (filename → content, written under `<out-dir>/charts/`).
#[derive(Debug)]
pub struct Comparison {
    pub markdown: String,
    pub charts: Vec<(String, String)>,
}

fn pct(base: f64, other: f64) -> String {
    if base <= 0.0 || other <= 0.0 {
        return "—".to_owned();
    }
    let ratio = other / base;
    if ratio >= 1.0 {
        format!("{ratio:.1}×")
    } else {
        format!("{:.1}×", 1.0 / ratio)
    }
}

fn metric_map(
    r: &Results,
    pick: impl Fn(&super::json::ClassRecord) -> u64,
) -> BTreeMap<String, u64> {
    r.classes
        .iter()
        .filter(|(_, c)| c.count > 0)
        .map(|(k, c)| (k.clone(), pick(c)))
        .collect()
}

/// Render the comparison for exactly the first two results (the harness runs
/// one SUT at a time; a wider matrix is a publication-step concern).
#[must_use]
pub fn render(results: &[Results]) -> Comparison {
    let mut md = String::new();
    md.push_str("# Benchmark comparison (generated)\n\n");
    md.push_str(
        "> **Measured, not asserted.** Every number below is read from a committed \
         `results.json`; both directions are reported. The workload, client, and \
         host are identical by construction (`docs/design/benchmarking.md` §3).\n\n",
    );
    if results.len() < 2 {
        md.push_str("Fewer than two runs supplied — nothing to compare.\n");
        return Comparison {
            markdown: md,
            charts: Vec::new(),
        };
    }
    let a = &results[0];
    let b = &results[1];

    md.push_str("## Runs\n\n");
    md.push_str("| | Product | Profile | Scale | Ward | Requests | req/s | Error rate |\n|---|---|---|---|--:|--:|--:|--:|\n");
    for r in [a, b] {
        md.push_str(&format!(
            "| **{}** | {} | {} | {} | {} | {} | {:.1} | {:.3}% |\n",
            r.sut.name,
            r.sut.product_label,
            r.workload.profile,
            r.workload.scale,
            r.workload.ward_size,
            r.throughput.requests,
            r.throughput.rps,
            r.throughput.error_rate * 100.0
        ));
    }
    md.push('\n');
    if a.workload.lock != b.workload.lock {
        md.push_str(
            "> [!WARNING]\n> The two runs carry **different workload locks** — \
             they did not execute the identical pre-registered workload and the \
             comparison below is not fair. Re-run both on the same harness commit.\n\n",
        );
    }

    let mut charts = Vec::new();
    headline_charts(&mut md, &mut charts, a, b);

    // Charts: p99 + p50 grouped bars over the classes both runs measured.
    let p99 = vec![
        (a.sut.name.clone(), metric_map(a, |c| c.p99_us)),
        (b.sut.name.clone(), metric_map(b, |c| c.p99_us)),
    ];
    let p50 = vec![
        (a.sut.name.clone(), metric_map(a, |c| c.p50_us)),
        (b.sut.name.clone(), metric_map(b, |c| c.p50_us)),
    ];
    let c99 = chart::comparison_chart("p99 latency per operation class", &p99);
    if !c99.is_empty() {
        charts.push(("comparison-p99.svg".to_owned(), c99));
        md.push_str("## Latency — p99 per operation class\n\n");
        md.push_str("![p99 latency per operation class](charts/comparison-p99.svg)\n\n");
    }
    let c50 = chart::comparison_chart("p50 latency per operation class", &p50);
    if !c50.is_empty() {
        charts.push(("comparison-p50.svg".to_owned(), c50));
        md.push_str("## Latency — p50 per operation class\n\n");
        md.push_str("![p50 latency per operation class](charts/comparison-p50.svg)\n\n");
    }

    let (a_wins, b_wins) = per_class_table(&mut md, a, b);

    resources_table(&mut md, a, b);
    wins_ledger(&mut md, &a.sut.name, &a_wins, &b.sut.name, &b_wins);

    md.push_str("## Limitations\n\n");
    md.push_str(
        "Single run per SUT (no inter-run variance yet — the ≥5-run protocol is \
         the publication step); same host, sequential execution; see each run's \
         own `REPORT.md` §Limitations for sampler availability.\n",
    );

    Comparison {
        markdown: md,
        charts,
    }
}

/// Headline scalar comparisons: throughput, app resources (+ over-the-run
/// overlays), cold start, and the computed efficiency ratios.
#[allow(clippy::too_many_lines)]
fn headline_charts(md: &mut String, charts: &mut Vec<(String, String)>, a: &Results, b: &Results) {
    // Headline scalar comparisons: throughput + resources + cold start.
    let tp = vec![
        (a.sut.name.clone(), a.throughput.rps),
        (b.sut.name.clone(), b.throughput.rps),
    ];
    let c_tp = chart::metric_bar_chart("Sustained throughput (req/s)", &tp, |v| {
        format!("{v:.1} req/s")
    });
    if !c_tp.is_empty() {
        charts.push(("comparison-throughput.svg".to_owned(), c_tp));
        md.push_str("## Throughput\n\n");
        md.push_str("![Sustained throughput](charts/comparison-throughput.svg)\n\n");
    }
    if let (Some(aa), Some(ba)) = (&a.resources.app, &b.resources.app) {
        let mem = vec![
            (a.sut.name.clone(), aa.peak_rss as f64),
            (b.sut.name.clone(), ba.peak_rss as f64),
        ];
        let c_mem = chart::metric_bar_chart("App peak memory (RSS)", &mem, |v| {
            format!("{:.0} MB", v / 1_048_576.0)
        });
        charts.push(("comparison-memory.svg".to_owned(), c_mem));
        let cpu = vec![
            (a.sut.name.clone(), aa.mean_cpu),
            (b.sut.name.clone(), ba.mean_cpu),
        ];
        let c_cpu =
            chart::metric_bar_chart("App mean CPU over the run", &cpu, |v| format!("{v:.1}%"));
        charts.push(("comparison-cpu.svg".to_owned(), c_cpu));
        md.push_str("## Resources — app container\n\n");
        md.push_str("![App peak memory](charts/comparison-memory.svg)\n\n");
        md.push_str("![App mean CPU](charts/comparison-cpu.svg)\n\n");
        // Over-the-run overlays (both SUTs' app containers, one measure each).
        let overlay: Vec<(String, super::json::ContainerSummary)> = vec![
            (a.sut.name.clone(), aa.clone()),
            (b.sut.name.clone(), ba.clone()),
        ];
        if let Some(c) = chart::overlay_series_chart(
            "App CPU over the run (%)",
            &overlay,
            |p| p.cpu_pct,
            |v| format!("{v:.0}%"),
        ) {
            charts.push(("comparison-cpu-series.svg".to_owned(), c));
            md.push_str("![App CPU over the run](charts/comparison-cpu-series.svg)\n\n");
        }
        if let Some(c) = chart::overlay_series_chart(
            "App memory (RSS) over the run",
            &overlay,
            |p| p.mem_bytes as f64,
            |v| format!("{:.0} MB", v / 1_048_576.0),
        ) {
            charts.push(("comparison-rss-series.svg".to_owned(), c));
            md.push_str("![App memory over the run](charts/comparison-rss-series.svg)\n\n");
        }
    }
    if let (Some(ac), Some(bc)) = (a.resources.cold_start_ms, b.resources.cold_start_ms) {
        let cs = vec![
            (a.sut.name.clone(), ac as f64),
            (b.sut.name.clone(), bc as f64),
        ];
        let c_cs = chart::metric_bar_chart("Cold start (compose-up → first answer)", &cs, |v| {
            format!("{v:.0} ms")
        });
        charts.push(("comparison-coldstart.svg".to_owned(), c_cs));
        md.push_str("## Cold start\n\n");
        md.push_str("![Cold start](charts/comparison-coldstart.svg)\n\n");
    }
    // Efficiency ratios (register 01 §2): req/s per core + per GB peak RSS.
    if let (Some(aa), Some(ba)) = (&a.resources.app, &b.resources.app) {
        md.push_str("## Efficiency (computed)\n\n");
        md.push_str("| | req/s per CPU-core | req/s per GB peak RSS |\n|---|--:|--:|\n");
        for (r, app) in [(a, aa), (b, ba)] {
            let cores = (app.mean_cpu / 100.0).max(0.001);
            let gb = (app.peak_rss as f64 / 1_073_741_824.0).max(0.001);
            md.push_str(&format!(
                "| **{}** | {:.1} | {:.1} |\n",
                r.sut.name,
                r.throughput.rps / cores,
                r.throughput.rps / gb
            ));
        }
        md.push('\n');
    }
}

/// The per-class percentile table; returns the computed win lists (p99).
#[allow(clippy::type_complexity)]
fn per_class_table(
    md: &mut String,
    a: &Results,
    b: &Results,
) -> (Vec<(String, u64, u64)>, Vec<(String, u64, u64)>) {
    // The per-class table + computed two-direction ledger.
    md.push_str("## Per-class detail (µs)\n\n");
    md.push_str(&format!(
        "| Class | {a} p50 | {b} p50 | {a} p90 | {b} p90 | {a} p99 | {b} p99 | {a} p99.9 | {b} p99.9 | {a} max | {b} max | {a} err | {b} err | p99 gap |\n|---|--:|--:|--:|--:|--:|--:|--:|--:|--:|--:|--:|--:|---|\n",
        a = a.sut.name,
        b = b.sut.name
    ));
    let mut a_wins = Vec::new();
    let mut b_wins = Vec::new();
    for (class, ar) in a.classes.iter().filter(|(_, c)| c.count > 0) {
        let Some(br) = b.classes.get(class).filter(|c| c.count > 0) else {
            continue;
        };
        md.push_str(&format!(
            "| {class} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |\n",
            ar.p50_us,
            br.p50_us,
            ar.p90_us,
            br.p90_us,
            ar.p99_us,
            br.p99_us,
            ar.p999_us,
            br.p999_us,
            ar.max_us,
            br.max_us,
            ar.errors,
            br.errors,
            pct(ar.p99_us as f64, br.p99_us as f64)
        ));
        if ar.p99_us < br.p99_us {
            a_wins.push((class.clone(), ar.p99_us, br.p99_us));
        } else if br.p99_us < ar.p99_us {
            b_wins.push((class.clone(), br.p99_us, ar.p99_us));
        }
    }
    md.push('\n');

    (a_wins, b_wins)
}

/// The side-by-side resources/storage table.
fn resources_table(md: &mut String, a: &Results, b: &Results) {
    md.push_str("## Resources\n\n");
    md.push_str("| | Idle RSS | Peak RSS | Mean CPU | Cold start | Storage bytes/composition |\n|---|--:|--:|--:|--:|--:|\n");
    for r in [a, b] {
        let (idle, peak, cpu) = r.resources.app.as_ref().map_or(
            ("—".to_owned(), "—".to_owned(), "—".to_owned()),
            |c| {
                (
                    c.idle_rss.map_or("—".to_owned(), |v| {
                        format!("{:.0} MB", v as f64 / 1_048_576.0)
                    }),
                    format!("{:.0} MB", c.peak_rss as f64 / 1_048_576.0),
                    format!("{:.1}%", c.mean_cpu),
                )
            },
        );
        md.push_str(&format!(
            "| **{}** | {idle} | {peak} | {cpu} | {} | {} |\n",
            r.sut.name,
            r.resources
                .cold_start_ms
                .map_or("—".to_owned(), |v| format!("{v} ms")),
            r.storage
                .as_ref()
                .map_or("—".to_owned(), |s| format!("{}", s.bytes_per_composition)),
        ));
    }
    md.push('\n');
}

/// Both directions, computed (benchmarking.md §7.6 — mandatory).
fn wins_ledger(
    md: &mut String,
    a_name: &str,
    a_wins: &[(String, u64, u64)],
    b_name: &str,
    b_wins: &[(String, u64, u64)],
) {
    for (name, wins) in [(a_name, a_wins), (b_name, b_wins)] {
        md.push_str(&format!("## Where {name} wins (p99, computed)\n\n"));
        if wins.is_empty() {
            md.push_str("No class won on p99 in this run pair.\n\n");
        } else {
            for (class, ours, theirs) in wins {
                md.push_str(&format!("- `{class}`: {ours} µs vs {theirs} µs\n"));
            }
            md.push('\n');
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::json::*;
    use super::*;
    use std::collections::BTreeMap;

    fn results(name: &str, p99: u64) -> Results {
        let mut classes = BTreeMap::new();
        classes.insert(
            "comp-create-small".to_owned(),
            ClassRecord {
                count: 10,
                errors: 0,
                p50_us: p99 / 2,
                p90_us: p99,
                p99_us: p99,
                p999_us: p99 * 2,
                max_us: p99 * 3,
                histogram: String::new(),
            },
        );
        Results {
            sut: SutBlock {
                name: name.to_owned(),
                kind: "ours".to_owned(),
                base_url: "http://x/v1".to_owned(),
                product_label: name.to_owned(),
                image_digests: BTreeMap::new(),
                versions: BTreeMap::new(),
            },
            workload: WorkloadBlock {
                lock: "same".to_owned(),
                profile: "hour".to_owned(),
                scale: "10k".to_owned(),
                ward_size: 20,
                load_factor: 1.0,
                seed: 1,
            },
            environment: EnvironmentBlock {
                host: "h".to_owned(),
                cpus: 4,
                mem_mib: 8000,
                harness_sha: "x".to_owned(),
                started: "2026-07-13T00:00:00Z".to_owned(),
            },
            classes,
            throughput: ThroughputBlock {
                window_s: 3600.0,
                requests: 1000,
                rps: 2.0,
                error_rate: 0.0,
            },
            resources: ResourcesBlock {
                app: None,
                db: None,
                cold_start_ms: Some(1000),
            },
            storage: None,
            reproduce: "scripts/benchmark.sh".to_owned(),
            excluded_templates: Vec::new(),
        }
    }

    #[test]
    fn renders_both_directions_and_charts() {
        let a = results("ehrbase-rs", 10_000);
        let b = results("ehrbase-java", 40_000);
        let c = render(&[a, b]);
        assert!(c.markdown.contains("Where ehrbase-rs wins"));
        assert!(c.markdown.contains("Where ehrbase-java wins"));
        assert!(c.markdown.contains("4.0×"));
        // p99 + p50 latency, throughput, cold start (no app resources in the
        // fixture → no memory/CPU charts).
        assert_eq!(c.charts.len(), 4);
        let names: Vec<&str> = c.charts.iter().map(|(n, _)| n.as_str()).collect();
        assert!(names.contains(&"comparison-throughput.svg"));
        assert!(names.contains(&"comparison-coldstart.svg"));
        assert!(c.markdown.contains("req/s"));
        assert!(!c.markdown.contains("different workload locks"));
    }

    #[test]
    fn flags_mismatched_locks() {
        let a = results("ehrbase-rs", 10_000);
        let mut b = results("ehrbase-java", 40_000);
        b.workload.lock = "other".to_owned();
        let c = render(&[a, b]);
        assert!(c.markdown.contains("different workload locks"));
    }
}
