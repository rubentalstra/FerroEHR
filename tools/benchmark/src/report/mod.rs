//! Artefact generation (register 01 §5): the machine record ([`json`]) + the
//! generated human report ([`markdown`]) + the raw per-class `HdrHistogram`
//! exports. Every artefact is generated from [`Results`], never hand-typed.
//!
//! Output layout per SUT (the caller passes `docs/benchmarks/<sut-name>`):
//!
//! ```text
//! <out>/
//! ├── results.json
//! ├── REPORT.md
//! ├── charts/{latency,cpu,rss}.svg
//! └── histograms/<class>.hdr.b64
//! ```

pub mod chart;
pub mod compare;
pub mod json;
pub mod knee;
pub mod markdown;

use std::path::Path;

pub use json::Results;

use crate::BenchError;

/// Write the full artefact set for a run into `out_dir`: `results.json`,
/// `REPORT.md`, and one `histograms/<class>.hdr.b64` per measured class.
///
/// # Errors
/// [`BenchError::Io`] on a filesystem failure, [`BenchError::Json`] on a
/// serialization failure.
pub fn write_all(results: &Results, out_dir: &Path) -> Result<(), BenchError> {
    std::fs::create_dir_all(out_dir)?;

    let json = results.to_json()?;
    std::fs::write(out_dir.join("results.json"), json)?;

    let md = markdown::render(results);
    std::fs::write(out_dir.join("REPORT.md"), md)?;

    // Charts (generated SVG, embedded by REPORT.md §Charts).
    let chart_dir = out_dir.join("charts");
    std::fs::create_dir_all(&chart_dir)?;
    let latency = chart::latency_chart(&results.classes);
    if !latency.is_empty() {
        std::fs::write(chart_dir.join("latency.svg"), latency)?;
    }
    if let Some(cpu) = chart::cpu_chart(
        results.resources.app.as_ref(),
        results.resources.db.as_ref(),
    ) {
        std::fs::write(chart_dir.join("cpu.svg"), cpu)?;
    }
    if let Some(rss) = chart::rss_chart(
        results.resources.app.as_ref(),
        results.resources.db.as_ref(),
    ) {
        std::fs::write(chart_dir.join("rss.svg"), rss)?;
    }

    let hist_dir = out_dir.join("histograms");
    std::fs::create_dir_all(&hist_dir)?;
    for (class, record) in &results.classes {
        if record.histogram.is_empty() {
            continue;
        }
        std::fs::write(
            hist_dir.join(format!("{class}.hdr.b64")),
            record.histogram.as_bytes(),
        )?;
    }
    Ok(())
}

/// Read a `results.json` back into [`Results`] (the `report` subcommand's input).
///
/// # Errors
/// [`BenchError::Io`] if the file cannot be read, [`BenchError::Json`] if it
/// cannot be parsed.
pub fn from_results_file(path: &Path) -> Result<Results, BenchError> {
    let text = std::fs::read_to_string(path)?;
    let results = serde_json::from_str(&text)?;
    Ok(results)
}

/// Format a sustained rate for prose/tables: requests per second with the
/// per-minute equivalent beside it — the same measurement in the unit most
/// readers reason in (owner rule: both units on every published throughput
/// figure; checklist item 25a).
#[must_use]
pub fn fmt_rate(rps: f64) -> String {
    format!("{rps:.1} req/s ({:.0} req/min)", rps * 60.0)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::report::json::{
        ClassRecord, EnvironmentBlock, EventsBlock, ResourcesBlock, SutBlock, ThroughputBlock,
        WorkloadBlock,
    };

    fn minimal_results() -> Results {
        let mut classes = BTreeMap::new();
        classes.insert(
            "ehr-create".to_owned(),
            ClassRecord {
                count: 1,
                errors: 0,
                p50_us: 1,
                p90_us: 1,
                p99_us: 1,
                p999_us: 1,
                max_us: 1,
                histogram: "SGRSM= base64".to_owned(),
            },
        );
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
                lock: "l".to_owned(),
                profile: "smoke".to_owned(),
                scale: "empty".to_owned(),
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
                window_s: 120.0,
                requests: 1,
                rps: 0.1,
                error_rate: 0.0,
            },
            events: EventsBlock::default(),
            resources: ResourcesBlock {
                app: None,
                db: None,
                cold_start_ms: None,
            },
            storage: None,
            reproduce: "scripts/benchmark.sh".to_owned(),
            excluded_templates: Vec::new(),
        }
    }

    #[test]
    fn writes_and_reads_back_the_artefacts() {
        let dir = std::env::temp_dir().join(format!("bench-report-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let results = minimal_results();
        write_all(&results, &dir).expect("write");

        assert!(dir.join("results.json").exists());
        assert!(dir.join("REPORT.md").exists());
        assert!(dir.join("histograms/ehr-create.hdr.b64").exists());

        let back = from_results_file(&dir.join("results.json")).expect("read back");
        assert_eq!(back.sut.name, "ehrbase-rs");
        assert_eq!(back.classes["ehr-create"].count, 1);

        let _ = std::fs::remove_dir_all(&dir);
    }
}
