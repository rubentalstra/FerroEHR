//! Resource + storage sampling (register 01 §2, §4).
//!
//! For a compose-managed SUT the harness samples the app and db containers over
//! the Docker stats CLI (~1 Hz) for CPU% and memory RSS, takes a pre-warmup idle
//! baseline, and probes the database's on-disk footprint via `docker exec …
//! psql`. A bring-your-own SUT (no container names) records these as
//! `unavailable` — an honest gap, never a fabricated number.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use tokio::process::Command;
use tokio::task::JoinHandle;
use tokio::time::Instant;

/// The poll cadence of the Docker stats sampler.
const SAMPLE_INTERVAL: Duration = Duration::from_secs(1);

/// One resource observation of one container.
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct ResourceSample {
    /// Milliseconds since the sampler started.
    pub t_ms: u64,
    /// CPU utilisation percent (Docker's `CPUPerc`; 100 = one full core).
    pub cpu_pct: f64,
    /// Memory RSS in bytes (the used side of Docker's `MemUsage`).
    pub mem_bytes: u64,
}

/// A container's sampled time series.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ContainerSeries {
    /// The container name sampled.
    pub name: String,
    /// The observations, in time order.
    pub samples: Vec<ResourceSample>,
}

impl ContainerSeries {
    /// Mean CPU percent over the series (`0.0` when empty).
    #[must_use]
    pub fn mean_cpu(&self) -> f64 {
        if self.samples.is_empty() {
            return 0.0;
        }
        self.samples.iter().map(|s| s.cpu_pct).sum::<f64>() / self.samples.len() as f64
    }

    /// Peak RSS bytes over the series (`0` when empty).
    #[must_use]
    pub fn peak_rss(&self) -> u64 {
        self.samples.iter().map(|s| s.mem_bytes).max().unwrap_or(0)
    }

    /// Mean RSS bytes over the series (`0` when empty) — the idle-baseline figure.
    #[must_use]
    pub fn mean_rss(&self) -> u64 {
        if self.samples.is_empty() {
            return 0;
        }
        let sum: u128 = self.samples.iter().map(|s| u128::from(s.mem_bytes)).sum();
        u64::try_from(sum / self.samples.len() as u128).unwrap_or(u64::MAX)
    }
}

/// A running Docker-stats sampler over one or more containers.
#[derive(Debug)]
pub struct ResourceSampler {
    stop: Arc<AtomicBool>,
    handle: JoinHandle<Vec<ContainerSeries>>,
}

impl ResourceSampler {
    /// Start sampling `containers` at ~1 Hz. An empty list yields a no-op sampler
    /// that stops to an empty series set (the BYO case).
    #[must_use]
    pub fn start(containers: Vec<String>) -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let handle = tokio::spawn(sample_loop(containers, Arc::clone(&stop)));
        Self { stop, handle }
    }

    /// Stop sampling and collect the per-container series.
    ///
    /// # Errors
    /// [`std::io::Error`] if the sampler task panicked (never in normal use).
    pub async fn stop(self) -> std::io::Result<Vec<ContainerSeries>> {
        self.stop.store(true, Ordering::Relaxed);
        self.handle
            .await
            .map_err(|e| std::io::Error::other(format!("resource sampler task: {e}")))
    }
}

/// Sample `containers` for a fixed `duration` (the pre-warmup idle baseline).
///
/// # Errors
/// [`std::io::Error`] if the sampler task fails.
pub async fn idle_baseline(
    containers: Vec<String>,
    duration: Duration,
) -> std::io::Result<Vec<ContainerSeries>> {
    let sampler = ResourceSampler::start(containers);
    tokio::time::sleep(duration).await;
    sampler.stop().await
}

async fn sample_loop(containers: Vec<String>, stop: Arc<AtomicBool>) -> Vec<ContainerSeries> {
    let mut series: Vec<ContainerSeries> = containers
        .iter()
        .map(|name| ContainerSeries {
            name: name.clone(),
            samples: Vec::new(),
        })
        .collect();
    if containers.is_empty() {
        return series;
    }
    let started = Instant::now();
    while !stop.load(Ordering::Relaxed) {
        if let Some(out) = docker_stats(&containers).await {
            let t_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
            for line in out.lines().filter(|l| !l.trim().is_empty()) {
                if let Some((name, cpu, mem)) = parse_stats_line(line)
                    && let Some(s) = series.iter_mut().find(|s| s.name == name)
                {
                    s.samples.push(ResourceSample {
                        t_ms,
                        cpu_pct: cpu,
                        mem_bytes: mem,
                    });
                }
            }
        }
        tokio::time::sleep(SAMPLE_INTERVAL).await;
    }
    series
}

/// Run `docker stats --no-stream --format '{{json .}}' <containers…>`, returning
/// the newline-delimited JSON lines, or `None` if the CLI is unavailable/failed.
async fn docker_stats(containers: &[String]) -> Option<String> {
    let output = Command::new("docker")
        .args(["stats", "--no-stream", "--format", "{{json .}}"])
        .args(containers)
        .output()
        .await
        .ok()?;
    if !output.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// Parse one `docker stats --format '{{json .}}'` line into
/// `(container-name, cpu-percent, mem-rss-bytes)`.
fn parse_stats_line(line: &str) -> Option<(String, f64, u64)> {
    let v: serde_json::Value = serde_json::from_str(line).ok()?;
    let name = v
        .get("Name")
        .and_then(serde_json::Value::as_str)?
        .to_owned();
    let cpu = v
        .get("CPUPerc")
        .and_then(serde_json::Value::as_str)
        .and_then(parse_percent)
        .unwrap_or(0.0);
    let mem = v
        .get("MemUsage")
        .and_then(serde_json::Value::as_str)
        .and_then(parse_mem_usage)
        .unwrap_or(0);
    Some((name, cpu, mem))
}

/// Parse Docker's `"12.34%"` into `12.34`.
fn parse_percent(s: &str) -> Option<f64> {
    s.trim().trim_end_matches('%').trim().parse().ok()
}

/// Parse the used side of Docker's `"45.6MiB / 7.667GiB"` into bytes.
fn parse_mem_usage(s: &str) -> Option<u64> {
    let used = s.split('/').next()?.trim();
    parse_size(used)
}

/// Parse a Docker byte-size token (`"1.5GiB"`, `"512MB"`, `"900kB"`, `"42B"`)
/// into bytes. Docker uses binary IEC units (`KiB`) and decimal SI units (`kB`);
/// both are handled.
// The product is a non-negative byte count; truncating the fractional byte and
// the (impossible) negative case is exactly the intended rounding.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn parse_size(tok: &str) -> Option<u64> {
    let tok = tok.trim();
    let split = tok.find(|c: char| c.is_ascii_alphabetic())?;
    let (num, unit) = tok.split_at(split);
    let value: f64 = num.trim().parse().ok()?;
    let mult: f64 = match unit.trim() {
        "B" => 1.0,
        "kB" | "KB" => 1e3,
        "MB" => 1e6,
        "GB" => 1e9,
        "TB" => 1e12,
        "KiB" => 1024.0,
        "MiB" => 1024.0 * 1024.0,
        "GiB" => 1024.0 * 1024.0 * 1024.0,
        "TiB" => 1024.0 * 1024.0 * 1024.0 * 1024.0,
        _ => return None,
    };
    Some((value * mult) as u64)
}

// ── Storage probe ─────────────────────────────────────────────────────────────

/// Database-connection facts for the storage probe (per-SUT credentials passed
/// by the caller; compose defaults documented in `scripts/benchmark.sh`).
#[derive(Debug, Clone)]
pub struct DbAccess {
    /// The db container name (`docker exec` target).
    pub container: String,
    /// The psql role.
    pub user: String,
    /// The database name.
    pub db: String,
}

/// Probe the total on-disk relation size of the SUT's database (schema-agnostic:
/// sums `pg_total_relation_size` over ordinary tables, indexes, TOAST, and
/// materialized views). Returns `None` if `docker exec`/`psql` is unavailable —
/// recorded as an honest gap, never a fabricated figure.
pub async fn probe_storage(db: &DbAccess) -> Option<u64> {
    const SQL: &str = "SELECT COALESCE(sum(pg_total_relation_size(oid)),0) \
         FROM pg_class WHERE relkind IN ('r','i','t','m')";
    let output = Command::new("docker")
        .args([
            "exec",
            &db.container,
            "psql",
            "-U",
            &db.user,
            "-d",
            &db.db,
            "-Atc",
            SQL,
        ])
        .output()
        .await
        .ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8_lossy(&output.stdout).trim().parse().ok()
}

/// Pay the database's maintenance debt outside the measured windows: seeding
/// (and each rung's writes) accumulates dead tuples and analyze debt, and
/// autovacuum's threshold then fires DURING a later window — a multi-minute,
/// multi-core VACUUM/ANALYZE of the jsonb-heavy `node` table was observed
/// saturating the database engine mid-rung (2026-07-14: the `L=16` rung breached the SLO
/// while `last_autovacuum` on 1.19M `node` rows landed inside its window; the
/// first post-vacuum rung ran clean at 73 ms p99). Running `VACUUM ANALYZE`
/// deterministically after seeding and in each inter-rung drain moves that
/// work outside every measured window, identically for every SUT (fairness).
/// Returns `false` when `docker exec`/`psql` is unavailable (BYO) — the
/// caller logs the gap honestly instead of pretending the state is settled.
pub async fn settle_maintenance(db: &DbAccess) -> bool {
    let output = Command::new("docker")
        .args([
            "exec",
            &db.container,
            "psql",
            "-U",
            &db.user,
            "-d",
            &db.db,
            "-Atc",
            "VACUUM ANALYZE",
        ])
        .output()
        .await;
    matches!(output, Ok(o) if o.status.success())
}

#[cfg(test)]
#[allow(
    clippy::panic,
    clippy::print_stdout,
    clippy::print_stderr,
    let_underscore_drop
)]
// test assertions/diagnostics/fixtures
// The expected-value arithmetic mirrors the parser's own byte casts.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_docker_stats_json_line() {
        let line = r#"{"BlockIO":"0B / 0B","CPUPerc":"12.34%","Container":"abc","ID":"abc","MemPerc":"1.2%","MemUsage":"45.6MiB / 7.667GiB","Name":"ehrbase-rs-ehrbase-1","NetIO":"1kB / 2kB","PIDs":"30"}"#;
        let (name, cpu, mem) = parse_stats_line(line).expect("parse");
        assert_eq!(name, "ehrbase-rs-ehrbase-1");
        assert!((cpu - 12.34).abs() < 1e-9);
        assert_eq!(mem, (45.6 * 1024.0 * 1024.0) as u64);
    }

    #[test]
    fn parses_binary_and_decimal_sizes() {
        assert_eq!(parse_size("42B"), Some(42));
        assert_eq!(
            parse_size("1.5GiB"),
            Some((1.5 * 1024.0 * 1024.0 * 1024.0) as u64)
        );
        assert_eq!(parse_size("512MB"), Some(512_000_000));
        assert_eq!(parse_size("900kB"), Some(900_000));
        assert_eq!(parse_size("bogus"), None);
    }

    #[test]
    fn parses_mem_usage_used_side() {
        assert_eq!(
            parse_mem_usage("128MiB / 8GiB"),
            Some((128.0 * 1024.0 * 1024.0) as u64)
        );
    }

    #[test]
    fn percent_parse() {
        assert_eq!(parse_percent("0.00%"), Some(0.0));
        assert_eq!(parse_percent("150.5%"), Some(150.5));
    }

    #[test]
    fn series_aggregates() {
        let s = ContainerSeries {
            name: "x".to_owned(),
            samples: vec![
                ResourceSample {
                    t_ms: 0,
                    cpu_pct: 10.0,
                    mem_bytes: 100,
                },
                ResourceSample {
                    t_ms: 1000,
                    cpu_pct: 30.0,
                    mem_bytes: 300,
                },
            ],
        };
        assert!((s.mean_cpu() - 20.0).abs() < 1e-9);
        assert_eq!(s.peak_rss(), 300);
        assert_eq!(s.mean_rss(), 200);
    }
}
