//! The machine record: `results.json` (register 01 §6).
//!
//! Serde structs mirroring the register-01 §6 schema exactly. The per-class
//! latency block is a local [`ClassRecord`] (independent of the [`measure`]
//! crate's internal type), so the on-disk schema is owned here and stable across
//! internal refactors. `results.json` is the source the report + histograms are
//! generated from — never hand-typed.

use std::collections::BTreeMap;

use sysinfo::System;

use crate::measure::ClassSummary;
use crate::sample::ContainerSeries;

/// The whole machine record for one run.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Results {
    /// The SUT identity.
    pub sut: SutBlock,
    /// The frozen workload parameters.
    pub workload: WorkloadBlock,
    /// The run environment (load-generator host + harness provenance).
    pub environment: EnvironmentBlock,
    /// Per operation-class latency summaries, keyed by the stable class name.
    pub classes: BTreeMap<String, ClassRecord>,
    /// Sustained throughput over the measurement window.
    pub throughput: ThroughputBlock,
    /// Clinical-event (business-transaction) throughput: TPC-style events/min
    /// beside the per-request req/s (checklist item 25b). `#[serde(default)]`
    /// so a pre-25b `results.json` still deserializes (empty events block).
    #[serde(default)]
    pub events: EventsBlock,
    /// Container resource series + cold start (absent fields = unavailable).
    pub resources: ResourcesBlock,
    /// Database on-disk footprint (`None` = unavailable, e.g. a BYO SUT).
    pub storage: Option<StorageBlock>,
    /// The exact command that reproduces this run (register 01 §5 reproduce-it).
    pub reproduce: String,
    /// `template_id`s the SUT refused to provision (fairness/limitations).
    #[serde(default)]
    pub excluded_templates: Vec<String>,
}

/// The SUT identity (register 01 §6 `sut`).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SutBlock {
    /// The output/lookup name (`ehrbase-rs`, `ehrbase-java`, a BYO name).
    pub name: String,
    /// `ours` | `foreign` (the conformance `SutKind`).
    pub kind: String,
    /// The ITS-REST base URL exercised.
    pub base_url: String,
    /// The product/version label.
    pub product_label: String,
    /// Container image digests, when known (compose-managed runs).
    #[serde(default)]
    pub image_digests: BTreeMap<String, String>,
    /// Advertised versions, when known.
    #[serde(default)]
    pub versions: BTreeMap<String, String>,
}

/// The frozen workload parameters (register 01 §6 `workload`).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WorkloadBlock {
    /// The `workload.lock` hash (register 00 §6).
    pub lock: String,
    /// `smoke` | `hour` | `day`.
    pub profile: String,
    /// `empty` | `10k` | `100k` | `1m`.
    pub scale: String,
    /// The ward size (admitted patients).
    pub ward_size: usize,
    /// The arrival-rate load factor `L`.
    pub load_factor: f64,
    /// The deterministic generator seed.
    pub seed: u64,
}

/// The run environment (register 01 §6 `environment`).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EnvironmentBlock {
    /// The load-generator host one-line summary.
    pub host: String,
    /// Logical CPUs on the load-generator host.
    pub cpus: usize,
    /// Total memory (MiB) on the load-generator host.
    pub mem_mib: u64,
    /// The harness git revision, if resolvable.
    pub harness_sha: String,
    /// ISO-8601 run start.
    pub started: String,
    /// The config-parity knobs the harness applied to the SUT stacks (DB pool
    /// ceiling, in-flight admission cap, signing, log level), captured from
    /// the environment so every published number carries the configuration it
    /// was measured under. Absent in pre-P20 artefacts.
    #[serde(default)]
    pub sut_config: BTreeMap<String, String>,
}

impl EnvironmentBlock {
    /// Capture the current environment (the machine driving the load + harness
    /// provenance). The host figures are the *load generator*, not the SUT
    /// containers (whose pinned resources ride the resources block).
    #[must_use]
    pub fn capture() -> Self {
        let mut sys = System::new();
        sys.refresh_cpu_all();
        sys.refresh_memory();
        let cpu_model = sys
            .cpus()
            .first()
            .map(|c| c.brand().trim().to_owned())
            .filter(|b| !b.is_empty())
            .unwrap_or_else(|| "unknown".to_owned());
        let cpus = sys.cpus().len();
        let mem_mib = sys.total_memory() / (1024 * 1024);
        let os = format!(
            "{} {}",
            System::name().unwrap_or_else(|| "unknown".to_owned()),
            System::os_version().unwrap_or_default()
        );
        let host = format!(
            "{cpu_model} ({cpus} logical CPUs, {mem_mib} MiB RAM) · {os} · {}",
            System::cpu_arch()
        );
        EnvironmentBlock {
            host,
            cpus,
            mem_mib,
            harness_sha: harness_sha(),
            started: jiff::Timestamp::now().to_string(),
            sut_config: sut_config(),
        }
    }
}

/// The config-parity knobs `scripts/benchmark.sh` exports for the SUT stacks,
/// recorded verbatim when present so the artefact says what it measured.
fn sut_config() -> BTreeMap<String, String> {
    [
        "BENCH_DB_POOL",
        "EHRBASE_REST_MAX_IN_FLIGHT",
        "EHRBASE_SIGNING_ENABLED",
        "EHRBASE_LOG_FILTER",
        "LOGGING_LEVEL_ROOT",
        "BENCH_PG_SHARED_BUFFERS",
        "BENCH_PG_MAX_WAL_SIZE",
        "BENCH_PG_WORK_MEM",
    ]
    .into_iter()
    .filter_map(|k| std::env::var(k).ok().map(|v| (k.to_owned(), v)))
    .collect()
}

/// The harness git SHA from the environment, else `git rev-parse`, else
/// `unknown`. CI providers export `GITHUB_SHA`; the env var wins so a container
/// build with no `.git` still stamps provenance.
fn harness_sha() -> String {
    for var in ["BENCH_HARNESS_SHA", "GITHUB_SHA", "GIT_SHA"] {
        if let Ok(sha) = std::env::var(var)
            && !sha.trim().is_empty()
        {
            return sha.trim().to_owned();
        }
    }
    std::process::Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_owned())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "unknown".to_owned())
}

/// A per operation-class latency summary (register 01 §6 `classes.<class>`).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ClassRecord {
    /// Measured (post-warmup) operations.
    pub count: u64,
    /// Errored operations (excluded from the latency percentiles).
    pub errors: u64,
    /// 50th-percentile latency (µs).
    pub p50_us: u64,
    /// 90th-percentile latency (µs).
    pub p90_us: u64,
    /// 99th-percentile latency (µs).
    pub p99_us: u64,
    /// 99.9th-percentile latency (µs).
    pub p999_us: u64,
    /// Maximum latency (µs).
    pub max_us: u64,
    /// The raw `HdrHistogram` (base64 V2), the reproducible source of the above.
    pub histogram: String,
}

impl ClassRecord {
    /// Build a record from a [`measure::ClassSummary`].
    #[must_use]
    pub fn from_summary(s: &ClassSummary) -> Self {
        ClassRecord {
            count: s.count,
            errors: s.errors,
            p50_us: s.p50_us,
            p90_us: s.p90_us,
            p99_us: s.p99_us,
            p999_us: s.p999_us,
            max_us: s.max_us,
            histogram: s.histogram_b64.clone(),
        }
    }
}

/// Sustained throughput (register 01 §6 `throughput`).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ThroughputBlock {
    /// The measurement window (seconds).
    pub window_s: f64,
    /// Measured (post-warmup) requests.
    pub requests: u64,
    /// Sustained requests/second.
    pub rps: f64,
    /// Error rate over `requests + errors`.
    pub error_rate: f64,
}

/// Clinical-event (business-transaction) throughput (checklist item 25b). A
/// clinical event (admission, medication round, lab batch, discharge…) is a
/// multi-request business transaction; it is *completed* only when every one of
/// its steps succeeded within the measured window (warmup applied per event by
/// its last step — symmetric with the per-request warmup discard). The TPC-style
/// events/min analogue of the per-request req/s.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct EventsBlock {
    /// Per event class, keyed by the stable event key (`E1`..`E10`).
    pub classes: BTreeMap<String, EventClassRecord>,
    /// Total attempted occurrences over the window.
    pub attempted: u64,
    /// Total completed occurrences over the window.
    pub completed: u64,
    /// Completed clinical events per minute (same window denominator as req/s).
    pub events_per_min: f64,
}

/// One event class's business-transaction tally (register 01 §6 `events.<E>`).
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct EventClassRecord {
    /// The human label ("admission", …).
    pub label: String,
    /// Attempted occurrences (last step landed in the measurement window).
    pub attempted: u64,
    /// Completed occurrences (every step succeeded).
    pub completed: u64,
    /// Completed occurrences per minute (same window denominator as req/s).
    pub events_per_min: f64,
}

/// Container resource series + cold start (register 01 §6 `resources`).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ResourcesBlock {
    /// The app container summary, when sampled.
    pub app: Option<ContainerSummary>,
    /// The db container summary, when sampled.
    pub db: Option<ContainerSummary>,
    /// Cold start (compose-up → first successful HTTP answer), when measured.
    pub cold_start_ms: Option<u64>,
}

/// A container's summarized + raw resource series.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ContainerSummary {
    /// The container name.
    pub name: String,
    /// Idle RSS (mean over the pre-warmup baseline), when sampled.
    pub idle_rss: Option<u64>,
    /// Peak RSS over the run.
    pub peak_rss: u64,
    /// Mean CPU percent over the run.
    pub mean_cpu: f64,
    /// The raw run series.
    pub series: Vec<ResourceSample>,
}

/// One resource observation (re-exported schema type).
pub use crate::sample::ResourceSample;

impl ContainerSummary {
    /// Summarize a run series with an optional idle baseline for the same name.
    #[must_use]
    pub fn from_series(run: &ContainerSeries, idle: Option<&ContainerSeries>) -> Self {
        ContainerSummary {
            name: run.name.clone(),
            idle_rss: idle.map(ContainerSeries::mean_rss),
            peak_rss: run.peak_rss(),
            mean_cpu: run.mean_cpu(),
            series: run.samples.clone(),
        }
    }
}

/// The database on-disk footprint (register 01 §6 `storage`).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct StorageBlock {
    /// Total bytes over ordinary tables/indexes/TOAST/matviews.
    pub bytes_total: u64,
    /// The seeded composition count at measurement time.
    pub compositions: u64,
    /// Bytes per composition (`bytes_total / compositions`; `0` if none).
    pub bytes_per_composition: u64,
}

impl StorageBlock {
    /// Build the footprint from a probed total and the seeded composition count.
    #[must_use]
    pub fn new(bytes_total: u64, compositions: u64) -> Self {
        let bytes_per_composition = bytes_total.checked_div(compositions).unwrap_or(0);
        StorageBlock {
            bytes_total,
            compositions,
            bytes_per_composition,
        }
    }
}

impl Results {
    /// Pretty-print the machine record.
    ///
    /// # Errors
    /// [`serde_json::Error`] if serialization fails.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_results() -> Results {
        let mut classes = BTreeMap::new();
        classes.insert(
            "ehr-create".to_owned(),
            ClassRecord {
                count: 100,
                errors: 0,
                p50_us: 4000,
                p90_us: 6000,
                p99_us: 9000,
                p999_us: 11000,
                max_us: 12000,
                histogram: "SGRSAAAAAA==".to_owned(),
            },
        );
        Results {
            sut: SutBlock {
                name: "ehrbase-rs".to_owned(),
                kind: "ours".to_owned(),
                base_url: "http://localhost:8080/ehrbase/rest/openehr/v1".to_owned(),
                product_label: "ehrbase-rs 3.0.0".to_owned(),
                image_digests: BTreeMap::new(),
                versions: BTreeMap::new(),
            },
            workload: WorkloadBlock {
                lock: "abc123".to_owned(),
                profile: "smoke".to_owned(),
                scale: "empty".to_owned(),
                ward_size: 20,
                load_factor: 1.0,
                seed: 42,
            },
            environment: EnvironmentBlock {
                host: "test host".to_owned(),
                cpus: 8,
                mem_mib: 16000,
                harness_sha: "deadbeef".to_owned(),
                started: "2026-07-13T00:00:00Z".to_owned(),
                sut_config: BTreeMap::new(),
            },
            classes,
            throughput: ThroughputBlock {
                window_s: 120.0,
                requests: 100,
                rps: 0.83,
                error_rate: 0.0,
            },
            events: EventsBlock {
                classes: BTreeMap::from([(
                    "E2".to_owned(),
                    EventClassRecord {
                        label: "shift-vitals".to_owned(),
                        attempted: 20,
                        completed: 19,
                        events_per_min: 9.5,
                    },
                )]),
                attempted: 20,
                completed: 19,
                events_per_min: 9.5,
            },
            resources: ResourcesBlock {
                app: None,
                db: None,
                cold_start_ms: Some(4200),
            },
            storage: None,
            reproduce: "scripts/benchmark.sh".to_owned(),
            excluded_templates: Vec::new(),
        }
    }

    #[test]
    fn results_round_trip_through_json() {
        let r = sample_results();
        let json = r.to_json().expect("serialize");
        let back: Results = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.sut.name, "ehrbase-rs");
        assert_eq!(back.workload.seed, 42);
        assert_eq!(back.classes["ehr-create"].p99_us, 9000);
        assert_eq!(back.resources.cold_start_ms, Some(4200));
        // The clinical-event (business-transaction) block survives the round trip.
        assert_eq!(back.events.completed, 19);
        assert_eq!(back.events.classes["E2"].label, "shift-vitals");
        assert!((back.events.events_per_min - 9.5).abs() < f64::EPSILON);
    }

    #[test]
    fn events_default_when_absent_from_json() {
        // A pre-25b results.json (no `events` key) still deserializes: the
        // #[serde(default)] events block is empty, not an error.
        let mut v = serde_json::to_value(sample_results()).expect("to value");
        v.as_object_mut().expect("object").remove("events");
        let back: Results = serde_json::from_value(v).expect("deserialize without events");
        assert_eq!(back.events.attempted, 0);
        assert!(back.events.classes.is_empty());
    }

    #[test]
    fn storage_bytes_per_composition() {
        let s = StorageBlock::new(1_000_000, 1000);
        assert_eq!(s.bytes_per_composition, 1000);
        let empty = StorageBlock::new(500, 0);
        assert_eq!(empty.bytes_per_composition, 0);
    }
}
