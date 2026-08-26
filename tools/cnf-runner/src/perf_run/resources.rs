// SPDX-FileCopyrightText: FerroEHR contributors
// SPDX-License-Identifier: Apache-2.0

//! The resource-telemetry sampler for measured runs.
//!
//! It records per-container
//! CPU/RSS/block-IO/network series on a fixed cadence via the Docker
//! Engine API stats endpoint (one-shot stats per container per tick), plus
//! the database volume's disk-anchor probe. Measured CONTEXT only — never
//! verdict-bearing, and every failure degrades to an absent/truncated
//! series with the reason in the progress log, never a run failure.
//!
//! NOTE: no openEHR spec governs measured performance or its telemetry —
//! our own design/extension (the CNF guide excludes non-functional
//! conformance; see `crate::perf`).

#![expect(
    clippy::disallowed_types,
    reason = "dev/verification tooling over JSON artifacts (the catalogue, results, wire \
              exchanges) — not the application (#1694)"
)]

use std::process::Command;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use crate::ixit::Containers;
use crate::perf::{ContainerResourceSeries, ContainerRole, ResourcePhase, ResourceSample};

/// The fixed sampling cadence of a measured run (the schedule the record's
/// `sample_interval_s` publishes).
pub const SAMPLE_INTERVAL: Duration = Duration::from_secs(10);

/// The DB container path the disk anchors probe — the compose volume mount
/// (`docker/sut-ferroehr.yml` mounts `ferroehr-pgdata` at
/// `/var/lib/postgresql`; the stock postgres images keep PGDATA under the
/// same prefix).
const DB_VOLUME_DIR: &str = "/var/lib/postgresql";

/// How long one stats/probe subprocess may take before it counts as a
/// failed tick (well under the cadence, so a hung runtime cannot skew the
/// schedule).
const PROBE_TIMEOUT: Duration = Duration::from_secs(8);

/// The cumulative counters one Engine-API stats reply carries (the raw
/// material a sample derives from; CPU% needs two consecutive readings).
#[derive(Debug, Clone, Copy)]
struct RawCounters {
    /// Total CPU consumed since container start, nanoseconds.
    cpu_total_ns: u64,
    /// Resident-set memory, bytes (usage minus the page cache the kernel
    /// can reclaim — the same subtraction the docker CLI shows).
    rss_bytes: u64,
    blk_read_bytes: u64,
    blk_write_bytes: u64,
    net_rx_bytes: u64,
    net_tx_bytes: u64,
}

/// A running resource sampler over the ixit-declared SUT + DB containers.
#[derive(Debug)]
pub struct ResourceSampler {
    stop: Arc<AtomicBool>,
    handle: JoinHandle<(Vec<ContainerResourceSeries>, Vec<String>)>,
}

impl ResourceSampler {
    /// Start sampling at [`SAMPLE_INTERVAL`]. `warmup_s`/`duration_s` are
    /// the planned window bounds the samples' phase stamps derive from
    /// (offsets past `warmup_s + duration_s` stamp as `drain` — the
    /// trailing completions still draining).
    #[must_use]
    pub fn start(containers: &Containers, warmup_s: u64, duration_s: u64) -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let stop_flag = Arc::clone(&stop);
        let targets = vec![
            (ContainerRole::Sut, containers.sut.clone()),
            (ContainerRole::Db, containers.db.clone()),
        ];
        let handle =
            std::thread::spawn(move || sample_loop(&targets, warmup_s, duration_s, &stop_flag));
        Self { stop, handle }
    }

    /// Stop sampling and collect the per-container series plus the
    /// degradation notes (failed ticks and their reasons — logged by the
    /// caller, never a run failure).
    #[must_use]
    pub fn stop(self) -> (Vec<ContainerResourceSeries>, Vec<String>) {
        self.stop.store(true, Ordering::Relaxed);
        self.handle.join().unwrap_or_else(|_| {
            (
                Vec::new(),
                vec!["resource sampler thread panicked — series lost".to_owned()],
            )
        })
    }
}

/// The phase a run-clock offset falls in against the planned window.
fn phase_of(offset_s: u64, warmup_s: u64, duration_s: u64) -> ResourcePhase {
    if offset_s < warmup_s {
        ResourcePhase::Warmup
    } else if offset_s < warmup_s.saturating_add(duration_s) {
        ResourcePhase::Measured
    } else {
        ResourcePhase::Drain
    }
}

fn sample_loop(
    targets: &[(ContainerRole, String)],
    warmup_s: u64,
    duration_s: u64,
    stop: &AtomicBool,
) -> (Vec<ContainerResourceSeries>, Vec<String>) {
    let started = Instant::now();
    let mut notes: Vec<String> = Vec::new();
    let mut series: Vec<ContainerResourceSeries> = targets
        .iter()
        .map(|(role, name)| ContainerResourceSeries {
            role: *role,
            name: name.clone(),
            samples: Vec::new(),
        })
        .collect();
    // Baseline counters at t0 (CPU% needs a delta, so the first emitted
    // sample lands one interval in).
    let mut prev: Vec<Option<(Instant, RawCounters)>> = targets
        .iter()
        .map(|(_, name)| {
            container_counters(name).map_or_else(
                |e| {
                    notes.push(format!("resource baseline for {name}: {e}"));
                    None
                },
                |c| Some((started, c)),
            )
        })
        .collect();

    let mut tick: u32 = 1;
    loop {
        let next = started + SAMPLE_INTERVAL * tick;
        while Instant::now() < next {
            if stop.load(Ordering::Relaxed) {
                return (series, dedup_notes(notes));
            }
            std::thread::sleep(Duration::from_millis(250));
        }
        for (i, (_, name)) in targets.iter().enumerate() {
            let now = Instant::now();
            match container_counters(name) {
                Ok(counters) => {
                    if let (Some(slot), Some(target)) = (prev.get_mut(i), series.get_mut(i)) {
                        if let Some((prev_at, prev_counters)) = *slot {
                            let wall_ns = now.duration_since(prev_at).as_nanos().max(1);
                            let cpu_ns = counters
                                .cpu_total_ns
                                .saturating_sub(prev_counters.cpu_total_ns);
                            // % of one core over the interval (100 = one
                            // full core) — both deltas are far below 2^52.
                            #[expect(
                                clippy::as_conversions,
                                clippy::cast_precision_loss,
                                reason = "nanosecond counters within a sampling window are far below 2^52"
                            )]
                            let cpu_pct = cpu_ns as f64 / wall_ns as f64 * 100.0;
                            let offset_s = started.elapsed().as_secs();
                            target.samples.push(ResourceSample {
                                offset_s,
                                phase: phase_of(offset_s, warmup_s, duration_s),
                                cpu_pct,
                                rss_bytes: counters.rss_bytes,
                                blk_read_bytes: counters.blk_read_bytes,
                                blk_write_bytes: counters.blk_write_bytes,
                                net_rx_bytes: counters.net_rx_bytes,
                                net_tx_bytes: counters.net_tx_bytes,
                            });
                        }
                        *slot = Some((now, counters));
                    }
                }
                Err(e) => {
                    notes.push(format!("resource sample for {name}: {e}"));
                    // A failed tick breaks the CPU delta chain honestly.
                    if let Some(slot) = prev.get_mut(i) {
                        *slot = None;
                    }
                }
            }
        }
        tick = tick.saturating_add(1);
        if stop.load(Ordering::Relaxed) {
            return (series, dedup_notes(notes));
        }
    }
}

/// Collapse repeated failure reasons so a long outage logs once with a
/// count instead of once per tick.
fn dedup_notes(notes: Vec<String>) -> Vec<String> {
    let mut out: Vec<(String, u32)> = Vec::new();
    for note in notes {
        if let Some((_, n)) = out.iter_mut().find(|(text, _)| *text == note) {
            *n += 1;
        } else {
            out.push((note, 1));
        }
    }
    out.into_iter()
        .map(|(text, n)| {
            if n > 1 {
                format!("{text} (x{n})")
            } else {
                text
            }
        })
        .collect()
}

/// One one-shot Engine-API stats reading for `container`.
fn container_counters(container: &str) -> Result<RawCounters, String> {
    let body = engine_api_get(&format!(
        "/containers/{}/stats?stream=false&one-shot=true",
        urlencoding::encode(container)
    ))?;
    let value: serde_json::Value =
        serde_json::from_str(&body).map_err(|e| format!("stats JSON: {e}"))?;
    parse_stats(&value).ok_or_else(|| "stats reply carries no cpu/memory counters".to_owned())
}

/// GET `path` on the Docker Engine API over the local socket (`curl
/// --unix-socket`; honours a `unix://` `DOCKER_HOST`). One short-lived
/// subprocess per tick — the sampling cost is negligible against the 10 s
/// cadence.
#[expect(
    clippy::disallowed_methods,
    reason = "`DOCKER_HOST` is the Docker CLI's own published environment contract \
              (docs.docker.com/reference/cli/docker/#environment-variables), read \
              here exactly as the CLI would; it is not server configuration"
)]
fn engine_api_get(path: &str) -> Result<String, String> {
    let socket = std::env::var("DOCKER_HOST")
        .ok()
        .and_then(|host| host.strip_prefix("unix://").map(str::to_owned))
        .unwrap_or_else(|| "/var/run/docker.sock".to_owned());
    let output = Command::new("curl")
        .args([
            "-sf",
            "--max-time",
            &PROBE_TIMEOUT.as_secs().to_string(),
            "--unix-socket",
            &socket,
            &format!("http://localhost{path}"),
        ])
        .output()
        .map_err(|e| format!("curl: {e}"))?;
    if !output.status.success() {
        return Err(format!(
            "engine API GET {path} failed (curl exit {:?})",
            output.status.code()
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// Extract the cumulative counters from one Engine-API stats reply
/// (cgroup v2 field shapes, with the v1 fallbacks the API still serves).
fn parse_stats(v: &serde_json::Value) -> Option<RawCounters> {
    let cpu_total_ns = v
        .pointer("/cpu_stats/cpu_usage/total_usage")
        .and_then(serde_json::Value::as_u64)?;
    let usage = v
        .pointer("/memory_stats/usage")
        .and_then(serde_json::Value::as_u64)?;
    // The docker CLI's "used" subtracts the reclaimable page cache:
    // inactive_file on cgroup v2, cache on v1; absent both, raw usage.
    let reclaimable = v
        .pointer("/memory_stats/stats/inactive_file")
        .or_else(|| v.pointer("/memory_stats/stats/cache"))
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0);
    let rss_bytes = usage.saturating_sub(reclaimable);
    // Block IO: sum the per-device read/write entries (op is lowercase on
    // cgroup v2, capitalized on v1). A runtime that reports none (e.g. a
    // VM-backed engine without blkio accounting) yields honest zeros.
    let (mut blk_read_bytes, mut blk_write_bytes) = (0_u64, 0_u64);
    if let Some(entries) = v
        .pointer("/blkio_stats/io_service_bytes_recursive")
        .and_then(serde_json::Value::as_array)
    {
        for entry in entries {
            let op = entry
                .get("op")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("");
            let bytes = entry
                .get("value")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0);
            if op.eq_ignore_ascii_case("read") {
                blk_read_bytes = blk_read_bytes.saturating_add(bytes);
            } else if op.eq_ignore_ascii_case("write") {
                blk_write_bytes = blk_write_bytes.saturating_add(bytes);
            }
        }
    }
    // Network: sum across interfaces.
    let (mut received, mut transmitted) = (0_u64, 0_u64);
    if let Some(networks) = v.get("networks").and_then(serde_json::Value::as_object) {
        for iface in networks.values() {
            received = received.saturating_add(
                iface
                    .get("rx_bytes")
                    .and_then(serde_json::Value::as_u64)
                    .unwrap_or(0),
            );
            transmitted = transmitted.saturating_add(
                iface
                    .get("tx_bytes")
                    .and_then(serde_json::Value::as_u64)
                    .unwrap_or(0),
            );
        }
    }
    Some(RawCounters {
        cpu_total_ns,
        rss_bytes,
        blk_read_bytes,
        blk_write_bytes,
        net_rx_bytes: received,
        net_tx_bytes: transmitted,
    })
}

/// Pays the database's maintenance debt outside the measured windows.
///
/// Seeding outruns autovacuum/autoanalyze (a stale-statistics plan cost a
/// measured ~9x on the ward-worklist query, 2026-07-23), and an
/// autovacuum firing INSIDE a window saturates the engine
/// mid-measurement. `vacuumdb --all --analyze` through the DB container
/// settles both, deterministically, identically for every SUT.
/// NOTE: no openEHR spec governs measured performance — instrument-side
/// fairness, our own design/extension (the retired benchmark lab's
/// documented prior art).
///
/// # Errors
/// A message when the runtime/exec/`vacuumdb` is unavailable or fails
/// (logged and skipped by callers — never a run failure).
pub fn settle_maintenance(db_container: &str) -> Result<(), String> {
    let output = Command::new("docker")
        .args([
            "exec",
            db_container,
            "vacuumdb",
            "-U",
            "postgres",
            "--all",
            "--analyze",
        ])
        .output()
        .map_err(|e| format!("docker exec: {e}"))?;
    if output.status.success() {
        Ok(())
    } else {
        Err(format!(
            "vacuumdb failed (exit {:?}): {}",
            output.status.code(),
            String::from_utf8_lossy(&output.stderr).trim()
        ))
    }
}

/// Probe the database volume's on-disk size (bytes): a read-only `du`
/// over the volume mount inside the DB container — instrument telemetry
/// through the container runtime, never a clinical-data path.
///
/// # Errors
/// A message when the runtime/exec/`du` is unavailable or unparseable
/// (recorded as an absent anchor, never a run failure).
pub fn db_volume_bytes(db_container: &str) -> Result<u64, String> {
    let output = Command::new("docker")
        .args(["exec", db_container, "du", "-sb", DB_VOLUME_DIR])
        .output()
        .map_err(|e| format!("docker exec: {e}"))?;
    // `du` exits non-zero when files vanish mid-walk (a live database) but
    // still prints the total — parse stdout regardless, fail only when no
    // total is present.
    let stdout = String::from_utf8_lossy(&output.stdout);
    stdout
        .split_whitespace()
        .next()
        .and_then(|token| token.parse::<u64>().ok())
        .ok_or_else(|| {
            format!(
                "du on {db_container}:{DB_VOLUME_DIR} yielded no byte total (exit {:?})",
                output.status.code()
            )
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A cgroup-v2-shaped Engine API stats reply (the fields the sampler
    /// reads, per the Docker Engine API `ContainerStats` reference).
    fn stats_fixture() -> serde_json::Value {
        serde_json::json!({
            "cpu_stats": { "cpu_usage": { "total_usage": 5_000_000_000_u64 },
                            "system_cpu_usage": 100_000_000_000_u64, "online_cpus": 8 },
            "memory_stats": { "usage": 300_000_000_u64,
                               "stats": { "inactive_file": 50_000_000_u64 } },
            "blkio_stats": { "io_service_bytes_recursive": [
                { "major": 254, "minor": 0, "op": "read", "value": 1_000_u64 },
                { "major": 254, "minor": 0, "op": "write", "value": 2_000_u64 },
                { "major": 254, "minor": 16, "op": "Read", "value": 10_u64 }
            ] },
            "networks": {
                "eth0": { "rx_bytes": 111_u64, "tx_bytes": 222_u64 },
                "eth1": { "rx_bytes": 9_u64, "tx_bytes": 1_u64 }
            }
        })
    }

    #[test]
    fn parses_a_cgroup_v2_stats_reply() {
        let c = parse_stats(&stats_fixture()).unwrap();
        assert_eq!(c.cpu_total_ns, 5_000_000_000);
        assert_eq!(c.rss_bytes, 250_000_000); // usage - inactive_file
        assert_eq!(c.blk_read_bytes, 1_010); // v2 + v1 op casings summed
        assert_eq!(c.blk_write_bytes, 2_000);
        assert_eq!(c.net_rx_bytes, 120);
        assert_eq!(c.net_tx_bytes, 223);
    }

    #[test]
    fn a_reply_without_counters_is_rejected() {
        assert!(parse_stats(&serde_json::json!({})).is_none());
        // Missing blkio/networks degrade to zeros, not a rejection.
        let minimal = serde_json::json!({
            "cpu_stats": { "cpu_usage": { "total_usage": 1_u64 } },
            "memory_stats": { "usage": 2_u64 }
        });
        let c = parse_stats(&minimal).unwrap();
        assert_eq!(c.rss_bytes, 2);
        assert_eq!(c.blk_read_bytes, 0);
        assert_eq!(c.net_tx_bytes, 0);
    }

    #[test]
    fn phases_stamp_against_the_planned_window() {
        assert_eq!(phase_of(0, 300, 3600), ResourcePhase::Warmup);
        assert_eq!(phase_of(299, 300, 3600), ResourcePhase::Warmup);
        assert_eq!(phase_of(300, 300, 3600), ResourcePhase::Measured);
        assert_eq!(phase_of(3899, 300, 3600), ResourcePhase::Measured);
        assert_eq!(phase_of(3900, 300, 3600), ResourcePhase::Drain);
    }

    #[test]
    fn repeated_failure_notes_collapse() {
        let notes = vec![
            "a".to_owned(),
            "b".to_owned(),
            "a".to_owned(),
            "a".to_owned(),
        ];
        assert_eq!(
            dedup_notes(notes),
            vec!["a (x3)".to_owned(), "b".to_owned()]
        );
    }
}
