//! The cross-SUT comparison artefact (`bench compare`): a side-by-side
//! Markdown matrix + grouped-bar SVG charts generated from two committed
//! `results.json` records. Measured numbers only, both directions — the
//! "where the other side wins" section is computed, never editorial
//! (`docs/design/benchmarking.md` §7).

use std::collections::BTreeMap;

use super::chart;
use super::json::{EventClassRecord, Results};
use super::knee::KneeResults;
use crate::model::event::ClinicalEvent;

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
/// one SUT at a time; a wider matrix is a publication-step concern). When two
/// `knee.json` records are also supplied (via `--knee-from`), the maximum-
/// sustained-throughput (knee) section is added; otherwise it is omitted.
#[must_use]
pub fn render(results: &[Results], knees: &[KneeResults]) -> Comparison {
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
    md.push_str("| | Product | Profile | Scale | Ward | Requests | req/s | req/min | Error rate |\n|---|---|---|---|--:|--:|--:|--:|--:|\n");
    for r in [a, b] {
        md.push_str(&format!(
            "| **{}** | {} | {} | {} | {} | {} | {:.1} | {:.0} | {:.3}% |\n",
            r.sut.name,
            r.sut.product_label,
            r.workload.profile,
            r.workload.scale,
            r.workload.ward_size,
            r.throughput.requests,
            r.throughput.rps,
            r.throughput.rps * 60.0,
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
    knee_section(&mut md, &mut charts, knees);
    events_section(&mut md, a, b);

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

/// The maximum-sustained-throughput (knee) section: a side-by-side table (knee
/// L, req/s, p99 at the knee) + a grouped bar of max sustained req/s at the SLO.
/// Added only when two `knee.json` records are supplied; measured only, both
/// directions.
fn knee_section(md: &mut String, charts: &mut Vec<(String, String)>, knees: &[KneeResults]) {
    if knees.len() < 2 {
        return;
    }
    let (a, b) = (&knees[0], &knees[1]);
    md.push_str("## Maximum sustained throughput (knee)\n\n");
    md.push_str(
        "> The last sustainable step on the load-factor ladder (p99 ≤ 1 s, error \
         ≤ 0.1%), per SUT — the honest capacity signal, not peak req/s. Each SUT's \
         own `KNEE.md` carries the full ladder and the single-run/same-host \
         lower-bound caveat.\n\n",
    );
    md.push_str(
        "| | Knee L | Sustained req/s | Sustained req/min | Clinical events/min | p99 at knee |\n|---|--:|--:|--:|--:|--:|\n",
    );
    for k in [a, b] {
        match &k.knee {
            Some(step) => md.push_str(&format!(
                "| **{}** | {} | {:.1} | {:.0} | {:.0} | {} |\n",
                k.sut.name,
                step.load_factor,
                step.rps,
                step.rps * 60.0,
                step.events_per_min,
                crate::report::fmt_latency_us(step.p99_us)
            )),
            None => md.push_str(&format!("| **{}** | — | — | — | — | — |\n", k.sut.name)),
        }
    }
    md.push('\n');

    let bars: Vec<(String, f64)> = [a, b]
        .iter()
        .map(|k| (k.sut.name.clone(), k.knee.as_ref().map_or(0.0, |s| s.rps)))
        .collect();
    let chart = chart::metric_bar_chart("Max sustained req/s at the SLO", &bars, |v| {
        format!("{v:.1} req/s")
    });
    if !chart.is_empty() {
        charts.push(("comparison-knee.svg".to_owned(), chart));
        md.push_str("![Max sustained req/s at the SLO](charts/comparison-knee.svg)\n\n");
    }
}

/// The clinical-transaction (business-transaction) throughput section: a
/// side-by-side per-class attempted/completed/events-min table + the computed
/// wins-ledger row for total events/min (checklist item 25b). Measured only,
/// both directions; the workload is identical by construction.
fn events_section(md: &mut String, a: &Results, b: &Results) {
    md.push_str("## Clinical transactions (events)\n\n");
    md.push_str(
        "> The TPC-style business-transaction metric: a clinical event (admission, \
         medication round, lab batch, discharge…) counts **completed** only when every \
         one of its requests succeeded. Events/min beside the per-request req/s — both \
         directions, same workload by construction.\n\n",
    );
    md.push_str(&format!(
        "| Event | {a} attempted | {a} completed | {a} events/min | {b} attempted | {b} completed | {b} events/min |\n|---|--:|--:|--:|--:|--:|--:|\n",
        a = a.sut.name,
        b = b.sut.name
    ));
    let cell = |rec: Option<&EventClassRecord>| {
        rec.map_or_else(
            || ("—".to_owned(), "—".to_owned(), "—".to_owned()),
            |c| {
                (
                    c.attempted.to_string(),
                    c.completed.to_string(),
                    format!("{:.1}", c.events_per_min),
                )
            },
        )
    };
    for ev in ClinicalEvent::ALL {
        let ar = a.events.classes.get(ev.key());
        let br = b.events.classes.get(ev.key());
        if ar.is_none() && br.is_none() {
            continue;
        }
        let (aa, ac, am) = cell(ar);
        let (ba, bc, bm) = cell(br);
        md.push_str(&format!(
            "| {} {} | {aa} | {ac} | {am} | {ba} | {bc} | {bm} |\n",
            ev.key(),
            ev.label(),
        ));
    }
    md.push_str(&format!(
        "| **total** | **{}** | **{}** | **{:.1}** | **{}** | **{}** | **{:.1}** |\n\n",
        a.events.attempted,
        a.events.completed,
        a.events.events_per_min,
        b.events.attempted,
        b.events.completed,
        b.events.events_per_min,
    ));

    // The wins-ledger row for total events/min (computed, both directions).
    let (ae, be) = (a.events.events_per_min, b.events.events_per_min);
    if ae > 0.0 || be > 0.0 {
        let (winner, high, low) = if ae >= be {
            (&a.sut.name, ae, be)
        } else {
            (&b.sut.name, be, ae)
        };
        md.push_str(&format!(
            "**Higher total clinical-event throughput: {winner}** — {high:.1} vs {low:.1} \
             events/min ({}).\n\n",
            pct(low, high)
        ));
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
#[allow(
    clippy::panic,
    clippy::print_stdout,
    clippy::print_stderr,
    let_underscore_drop
)] // test assertions/diagnostics/fixtures
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
                dep_misses: 0,
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
                sut_config: std::collections::BTreeMap::new(),
            },
            classes,
            throughput: ThroughputBlock {
                window_s: 3600.0,
                requests: 1000,
                rps: 2.0,
                error_rate: 0.0,
            },
            events: EventsBlock {
                classes: BTreeMap::from([(
                    "E2".to_owned(),
                    EventClassRecord {
                        label: "shift-vitals".to_owned(),
                        attempted: 100,
                        completed: 95,
                        events_per_min: 1.6,
                    },
                )]),
                attempted: 100,
                completed: 95,
                events_per_min: 1.6,
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

    fn knee(name: &str, rps: f64, p99: u64) -> KneeResults {
        use super::super::knee::{KneeResults, KneeStep};
        let step = KneeStep {
            load_factor: 8.0,
            rps,
            error_rate: 0.0,
            p99_us: p99,
            requests: 1000,
            events_per_min: 0.0,
            max_dispatch_lag_ms: 0,
        };
        KneeResults {
            sut: SutBlock {
                name: name.to_owned(),
                kind: "ours".to_owned(),
                base_url: "http://x/v1".to_owned(),
                product_label: name.to_owned(),
                image_digests: BTreeMap::new(),
                versions: BTreeMap::new(),
            },
            scale: "10k".to_owned(),
            steps: vec![step.clone()],
            knee: Some(step),
            sut_died: false,
        }
    }

    #[test]
    fn renders_both_directions_and_charts() {
        let a = results("ehrbase-rs", 10_000);
        let b = results("ehrbase-java", 40_000);
        let c = render(&[a, b], &[]);
        assert!(c.markdown.contains("Where ehrbase-rs wins"));
        assert!(c.markdown.contains("Where ehrbase-java wins"));
        assert!(c.markdown.contains("4.0×"));
        // p99 + p50 latency, throughput, cold start (no app resources in the
        // fixture → no memory/CPU charts; no knee records → no knee chart).
        assert_eq!(c.charts.len(), 4);
        let names: Vec<&str> = c.charts.iter().map(|(n, _)| n.as_str()).collect();
        assert!(names.contains(&"comparison-throughput.svg"));
        assert!(names.contains(&"comparison-coldstart.svg"));
        assert!(c.markdown.contains("req/s"));
        assert!(!c.markdown.contains("different workload locks"));
        assert!(!c.markdown.contains("Maximum sustained throughput"));
    }

    #[test]
    fn clinical_transactions_section_and_wins_row() {
        let a = results("ehrbase-rs", 10_000);
        let mut b = results("ehrbase-java", 40_000);
        // Make the java SUT complete fewer events so the wins row resolves to us.
        b.events.completed = 60;
        b.events.events_per_min = 1.0;
        b.events.classes.get_mut("E2").expect("E2").completed = 60;
        b.events.classes.get_mut("E2").expect("E2").events_per_min = 1.0;
        let c = render(&[a, b], &[]);
        assert!(c.markdown.contains("## Clinical transactions (events)"));
        assert!(c.markdown.contains("E2 shift-vitals"));
        // The computed wins-ledger row for total events/min, in our favour.
        assert!(
            c.markdown
                .contains("Higher total clinical-event throughput: ehrbase-rs")
        );
        assert!(c.markdown.contains("1.6 vs 1.0 events/min"));
    }

    #[test]
    fn knee_section_added_when_both_knees_present() {
        let a = results("ehrbase-rs", 10_000);
        let b = results("ehrbase-java", 40_000);
        let c = render(
            &[a, b],
            &[
                knee("ehrbase-rs", 190.0, 60_000),
                knee("ehrbase-java", 70.0, 900_000),
            ],
        );
        assert!(
            c.markdown
                .contains("## Maximum sustained throughput (knee)")
        );
        assert!(c.markdown.contains("190.0"));
        assert!(c.markdown.contains("70.0"));
        let names: Vec<&str> = c.charts.iter().map(|(n, _)| n.as_str()).collect();
        assert!(names.contains(&"comparison-knee.svg"));
    }

    #[test]
    fn flags_mismatched_locks() {
        let a = results("ehrbase-rs", 10_000);
        let mut b = results("ehrbase-java", 40_000);
        b.workload.lock = "other".to_owned();
        let c = render(&[a, b], &[]);
        assert!(c.markdown.contains("different workload locks"));
    }
}
