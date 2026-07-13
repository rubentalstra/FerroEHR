//! The generated human report: `REPORT.md` (register 01 §5 layout).
//!
//! environment → per-class latency table → throughput/knee → resource
//! efficiency → storage → cold start → limitations → reproduce-it. Generated
//! from [`Results`], never hand-typed; every honest gap (an unavailable sampler,
//! an excluded template) is stated in the limitations section, not hidden.

use crate::OpClass;
use crate::report::json::{ContainerSummary, Results};

/// Render `REPORT.md` for one run.
#[must_use]
pub fn render(results: &Results) -> String {
    let mut m = String::new();
    header(&mut m, results);
    environment(&mut m, results);
    latency(&mut m, results);
    throughput(&mut m, results);
    resource_efficiency(&mut m, results);
    storage(&mut m, results);
    cold_start(&mut m, results);
    limitations(&mut m, results);
    reproduce(&mut m, results);
    m
}

fn header(m: &mut String, r: &Results) {
    m.push_str(&format!("# Benchmark report — {}\n\n", r.sut.product_label));
    m.push_str(&format!(
        "> Generated from `results.json` (never hand-typed). Workload **{}** · scale **{}** · ward **{}** · load factor **{}** · seed `{}`. Latencies are microseconds, coordinated-omission-corrected against planned send times. Methodology: `docs/design/benchmarking.md`; workload: `docs/design/benchmark/00-workload-model.md`.\n\n",
        r.workload.profile, r.workload.scale, r.workload.ward_size, r.workload.load_factor, r.workload.seed
    ));
}

fn environment(m: &mut String, r: &Results) {
    let e = &r.environment;
    m.push_str("## 1. Environment\n\n");
    m.push_str(&format!("> **Load generator:** {}\n\n", e.host));
    m.push_str("| Field | Value |\n|---|---|\n");
    m.push_str(&format!(
        "| SUT | {} ({}) |\n",
        r.sut.product_label, r.sut.kind
    ));
    m.push_str(&format!("| Base URL | {} |\n", r.sut.base_url));
    m.push_str(&format!("| Run start | {} |\n", e.started));
    m.push_str(&format!(
        "| Load-gen host | {} logical CPUs, {} MiB RAM |\n",
        e.cpus, e.mem_mib
    ));
    m.push_str(&format!("| Harness rev | {} |\n", e.harness_sha));
    m.push_str(&format!("| Workload lock | `{}` |\n", r.workload.lock));
    if !r.sut.image_digests.is_empty() {
        for (k, v) in &r.sut.image_digests {
            m.push_str(&format!("| Image `{k}` | `{v}` |\n"));
        }
    }
    m.push_str("\n> A report with a different load-generator line is not directly comparable.\n\n");
}

fn latency(m: &mut String, r: &Results) {
    m.push_str("## 2. Latency — per operation class\n\n");
    m.push_str("p50 / p90 / p99 / p99.9 / max latency (µs) and error count per class. Raw HdrHistograms are exported to `histograms/<class>.hdr.b64`.\n\n");
    m.push_str("| Class | count | errors | p50 | p90 | p99 | p99.9 | max |\n|---|--:|--:|--:|--:|--:|--:|--:|\n");
    // Report order = the stable OpClass::ALL order; a class with no samples is
    // shown as `—` rather than omitted (honest coverage).
    for class in OpClass::ALL {
        let key = class.key();
        match r.classes.get(key) {
            Some(c) => m.push_str(&format!(
                "| {key} | {} | {} | {} | {} | {} | {} | {} |\n",
                c.count, c.errors, c.p50_us, c.p90_us, c.p99_us, c.p999_us, c.max_us
            )),
            None => m.push_str(&format!("| {key} | — | — | — | — | — | — | — |\n")),
        }
    }
    m.push('\n');
}

fn throughput(m: &mut String, r: &Results) {
    let t = &r.throughput;
    m.push_str("## 3. Throughput\n\n");
    m.push_str(&format!(
        "Sustained **{:.1} req/s** over a {:.0} s window ({} measured requests, error rate {:.3}%). The knee/saturation series (register 01 §3) is the multi-run publication step.\n\n",
        t.rps, t.window_s, t.requests, t.error_rate * 100.0
    ));
}

fn resource_efficiency(m: &mut String, r: &Results) {
    m.push_str("## 4. Resource efficiency\n\n");
    let (Some(app), rps) = (&r.resources.app, r.throughput.rps) else {
        m.push_str("_Resource sampling unavailable for this SUT (see limitations)._\n\n");
        return;
    };
    m.push_str("| Container | mean CPU | peak RSS | idle RSS |\n|---|--:|--:|--:|\n");
    row_container(m, app);
    if let Some(db) = &r.resources.db {
        row_container(m, db);
    }
    m.push('\n');
    // Efficiency: Docker CPUPerc of 100 = one full core; peak RSS in bytes.
    let cores = app.mean_cpu / 100.0;
    if cores > 0.0 {
        m.push_str(&format!(
            "- **{:.1} req/s per app CPU-core** ({rps:.1} req/s ÷ {cores:.2} cores).\n",
            rps / cores
        ));
    }
    if app.peak_rss > 0 {
        let gb = app.peak_rss as f64 / 1e9;
        m.push_str(&format!(
            "- **{:.1} req/s per GB peak app RSS** ({rps:.1} req/s ÷ {gb:.3} GB).\n",
            rps / gb
        ));
    }
    m.push('\n');
}

fn row_container(m: &mut String, c: &ContainerSummary) {
    m.push_str(&format!(
        "| {} | {:.1}% | {} | {} |\n",
        c.name,
        c.mean_cpu,
        human_bytes(c.peak_rss),
        c.idle_rss.map_or_else(|| "—".to_owned(), human_bytes),
    ));
}

fn storage(m: &mut String, r: &Results) {
    m.push_str("## 5. Storage footprint\n\n");
    match &r.storage {
        Some(s) => m.push_str(&format!(
            "Database on-disk size **{}** over **{}** compositions = **{}/composition** (`pg_total_relation_size` over tables/indexes/TOAST/matviews).\n\n",
            human_bytes(s.bytes_total),
            s.compositions,
            human_bytes(s.bytes_per_composition),
        )),
        None => m.push_str("_Storage probe unavailable for this SUT (see limitations)._\n\n"),
    }
}

fn cold_start(m: &mut String, r: &Results) {
    m.push_str("## 6. Cold start\n\n");
    match r.resources.cold_start_ms {
        Some(ms) => m.push_str(&format!(
            "Compose-up → first successful HTTP answer: **{ms} ms** ({:.1} s).\n\n",
            ms as f64 / 1000.0
        )),
        None => {
            m.push_str(
                "_Cold start not measured for this run (BYO SUT or compose unmanaged)._\n\n",
            );
        }
    }
}

fn limitations(m: &mut String, r: &Results) {
    m.push_str("## 7. Limitations\n\n");
    let mut any = false;
    if r.resources.app.is_none() {
        m.push_str(
            "- Resource sampling (CPU/RSS) unavailable — no container names supplied (BYO SUT).\n",
        );
        any = true;
    }
    if r.storage.is_none() {
        m.push_str("- Storage footprint unavailable — the database container was not reachable for a `docker exec … psql` probe.\n");
        any = true;
    }
    if r.resources.cold_start_ms.is_none() {
        m.push_str(
            "- Cold start not measured — compose lifecycle was not managed by the harness.\n",
        );
        any = true;
    }
    if r.storage.as_ref().is_some_and(|s| s.compositions == 0) {
        m.push_str("- Storage measured on an empty (unseeded) database — bytes/composition is not meaningful at this scale.\n");
        any = true;
    }
    if r.excluded_templates.is_empty() {
        m.push_str(
            "- Templates excluded for this SUT: none (all provisioning uploads accepted).\n",
        );
    } else {
        m.push_str("- **Templates excluded for this SUT** (upload rejected — operations against them are absent from the mix, not counted as errors):\n");
        for t in &r.excluded_templates {
            m.push_str(&format!("  - {t}\n"));
        }
        any = true;
    }
    if !any {
        m.push_str("- No sampler gaps: latency, throughput, resources, storage, and cold start were all captured.\n");
    }
    m.push_str("- Single-host, single-run figures. Publication requires ≥5 runs + coefficient of variation (benchmarking.md §4.4) and a config-parity table (§3.4) for any cross-SUT claim.\n\n");
}

fn reproduce(m: &mut String, r: &Results) {
    m.push_str("## 8. Reproduce it\n\n");
    m.push_str("```bash\n");
    m.push_str(&r.reproduce);
    m.push_str("\n```\n");
}

/// A compact human byte size (binary units).
fn human_bytes(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KiB", "MiB", "GiB", "TiB"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} B")
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::report::json::{
        ClassRecord, ContainerSummary, EnvironmentBlock, ResourceSample, ResourcesBlock,
        StorageBlock, SutBlock, ThroughputBlock, WorkloadBlock,
    };

    fn results(with_resources: bool, excluded: Vec<String>) -> Results {
        let mut classes = BTreeMap::new();
        classes.insert(
            "ehr-create".to_owned(),
            ClassRecord {
                count: 50,
                errors: 1,
                p50_us: 4000,
                p90_us: 6000,
                p99_us: 9000,
                p999_us: 11000,
                max_us: 12000,
                histogram: "hist".to_owned(),
            },
        );
        let resources = if with_resources {
            ResourcesBlock {
                app: Some(ContainerSummary {
                    name: "app".to_owned(),
                    idle_rss: Some(50_000_000),
                    peak_rss: 200_000_000,
                    mean_cpu: 150.0,
                    series: vec![ResourceSample {
                        t_ms: 0,
                        cpu_pct: 150.0,
                        mem_bytes: 200_000_000,
                    }],
                }),
                db: None,
                cold_start_ms: Some(4200),
            }
        } else {
            ResourcesBlock {
                app: None,
                db: None,
                cold_start_ms: None,
            }
        };
        Results {
            sut: SutBlock {
                name: "ehrbase-rs".to_owned(),
                kind: "ours".to_owned(),
                base_url: "http://x/v1".to_owned(),
                product_label: "ehrbase-rs 3.0.0".to_owned(),
                image_digests: BTreeMap::new(),
                versions: BTreeMap::new(),
            },
            workload: WorkloadBlock {
                lock: "lock".to_owned(),
                profile: "hour".to_owned(),
                scale: "10k".to_owned(),
                ward_size: 20,
                load_factor: 1.0,
                seed: 7,
            },
            environment: EnvironmentBlock {
                host: "host".to_owned(),
                cpus: 8,
                mem_mib: 16000,
                harness_sha: "abc".to_owned(),
                started: "2026-07-13T00:00:00Z".to_owned(),
            },
            classes,
            throughput: ThroughputBlock {
                window_s: 3600.0,
                requests: 50,
                rps: 12.5,
                error_rate: 0.0196,
            },
            resources,
            storage: if with_resources {
                Some(StorageBlock::new(1_000_000_000, 10_000))
            } else {
                None
            },
            reproduce: "scripts/benchmark.sh".to_owned(),
            excluded_templates: excluded,
        }
    }

    #[test]
    fn report_has_every_mandatory_section() {
        let md = render(&results(true, Vec::new()));
        for heading in [
            "## 1. Environment",
            "## 2. Latency",
            "## 3. Throughput",
            "## 4. Resource efficiency",
            "## 5. Storage footprint",
            "## 6. Cold start",
            "## 7. Limitations",
            "## 8. Reproduce it",
        ] {
            assert!(md.contains(heading), "missing {heading}\n{md}");
        }
        // Every op class appears (coverage), even the un-sampled ones.
        assert!(md.contains("comp-create-small"));
        assert!(md.contains("req/s per app CPU-core"));
        assert!(md.contains("/composition"));
        assert!(md.contains("4200 ms"));
    }

    #[test]
    fn limitations_state_unavailable_samplers_honestly() {
        let md = render(&results(false, Vec::new()));
        assert!(md.contains("Resource sampling (CPU/RSS) unavailable"));
        assert!(md.contains("Storage footprint unavailable"));
        assert!(md.contains("Cold start not measured"));
        assert!(md.contains("Templates excluded for this SUT: none"));
    }

    #[test]
    fn excluded_templates_are_surfaced() {
        let md = render(&results(
            true,
            vec!["ips.en.v1 (upload → HTTP 422)".to_owned()],
        ));
        assert!(md.contains("Templates excluded for this SUT"));
        assert!(md.contains("ips.en.v1 (upload → HTTP 422)"));
    }

    #[test]
    fn human_bytes_scales() {
        assert_eq!(human_bytes(512), "512 B");
        assert_eq!(human_bytes(1536), "1.5 KiB");
        assert_eq!(human_bytes(1_073_741_824), "1.0 GiB");
    }
}
