//! Report generation (design §7): `results.json` + a human `REPORT.md`, both
//! **generated from the run, never hand-typed**. The markdown always carries
//! the full latency distribution (both directions), a mandatory "where `EHRbase`
//! wins" section, and a methodology-limitations block — the honesty guardrails
//! from §0/§7.

use crate::driver::ScenarioResult;
use crate::target::Implementation;

/// The environment block stamped into every report (design §7.1).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EnvBlock {
    /// ISO-8601 run timestamp.
    pub run_date: String,
    /// The auto-captured machine specs — always present, so a reader can never
    /// mistake numbers from different hardware as comparable (design §0, §7.1).
    pub host: crate::host::HostInfo,
    /// The frozen workload hash (design §2).
    pub workload_lock: String,
    /// The harness git revision, if known.
    pub harness_revision: String,
    /// Warmup iterations per run.
    pub warmup_iters: u64,
    /// Measured iterations per run.
    pub measure_iters: u64,
    /// Independent runs.
    pub runs: u32,
}

/// A full benchmark report over 1–2 targets.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BenchReport {
    /// The environment block.
    pub env: EnvBlock,
    /// Per-target, per-scenario results.
    pub results: Vec<ScenarioResult>,
}

impl BenchReport {
    /// Serialize the machine-readable results.
    ///
    /// # Errors
    /// [`serde_json::Error`] if serialization fails.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Render the human-readable `REPORT.md`.
    #[must_use]
    pub fn to_markdown(&self) -> String {
        let mut m = String::new();
        m.push_str("# Benchmark report — ehrbase-rs");
        if self.has_java() {
            m.push_str(" vs. EHRbase (Java)");
        }
        m.push_str("\n\n> Generated from a run (never hand-typed). Latencies are\n> microseconds; the full distribution is shown for every scenario, both\n> directions. See `docs/design/benchmarking.md` for the methodology.\n\n");

        // 1. Environment — the machine is stated first and in full, because a
        // number is not comparable across hardware (design §0).
        let h = &self.env.host;
        m.push_str("## Environment\n\n");
        m.push_str(&format!("> **Machine:** {}\n\n", h.summary_line()));
        m.push_str(&format!(
            "| Field | Value |\n|---|---|\n| Run date | {} |\n| Host name | {} |\n| CPU | {} |\n| Cores / threads | {} / {} |\n| CPU freq | {} MHz |\n| Memory | {} MiB |\n| OS | {} {} (kernel {}) |\n| Arch | {} |\n| Harness rev | {} |\n| Workload lock | `{}` |\n| Warmup / measure / runs | {} / {} / {} |\n\n",
            self.env.run_date,
            h.hostname,
            h.cpu_model,
            h.physical_cores
                .map_or_else(|| "?".to_owned(), |p| p.to_string()),
            h.logical_cpus,
            h.cpu_mhz,
            h.total_memory_mib,
            h.os_name,
            h.os_version,
            h.kernel_version,
            h.arch,
            self.env.harness_revision,
            self.env.workload_lock,
            self.env.warmup_iters,
            self.env.measure_iters,
            self.env.runs,
        ));
        m.push_str("> Numbers below are valid only for this machine. A report with a different **Machine** line is not directly comparable (design §3.1).\n\n");

        // 2. Per-scenario latency table.
        m.push_str("## Latency & throughput (per scenario, per target)\n\n");
        m.push_str(
            "| Scenario | Target | Gate | p50 | p90 | p99 | p99.9 | max | req/s | run CoV |\n",
        );
        m.push_str("|---|---|---|--:|--:|--:|--:|--:|--:|--:|\n");
        for r in &self.results {
            if r.gate_ok {
                let l = r.merged.as_ref();
                m.push_str(&format!(
                    "| {} {} | {} | ✓ {} | {} | {} | {} | {} | {} | {} | {} |\n",
                    r.scenario,
                    r.description,
                    r.target,
                    r.gate_status,
                    fmt_us(l.map(|l| l.p50_us)),
                    fmt_us(l.map(|l| l.p90_us)),
                    fmt_us(l.map(|l| l.p99_us)),
                    fmt_us(l.map(|l| l.p999_us)),
                    fmt_us(l.map(|l| l.max_us)),
                    r.throughput_median_rps
                        .map_or_else(|| "—".to_owned(), |v| format!("{v:.0}")),
                    r.throughput_cov
                        .map_or_else(|| "—".to_owned(), |v| format!("{v:.2}")),
                ));
            } else {
                m.push_str(&format!(
                    "| {} {} | {} | ✗ {} (excluded — wrong response, not timed) | — | — | — | — | — | — | — |\n",
                    r.scenario, r.description, r.target, r.gate_status,
                ));
            }
        }
        m.push('\n');

        // 3. Where EHRbase wins (mandatory).
        m.push_str("## Where EHRbase wins\n\n");
        if self.has_java() {
            let wins = self.java_wins();
            if wins.is_empty() {
                m.push_str("_No scenario where EHRbase was faster on p99 in this run. This is stated plainly, not omitted; re-verify across more runs before relying on it._\n\n");
            } else {
                for w in wins {
                    m.push_str(&format!("- {w}\n"));
                }
                m.push('\n');
            }
        } else {
            m.push_str("_No comparison run: EHRbase (Java) was not benchmarked in this run (single-target run against ehrbase-rs). This section is mandatory in a comparative report and is filled from the head-to-head numbers once the Java stack is included (design §6, §9 steps 3–4)._\n\n");
        }

        // 4. Methodology limitations (mandatory — a benchmark claiming none is lying).
        m.push_str("## Methodology & limitations\n\n");
        m.push_str("- **Closed-loop, single-client latency** — this run measures per-request service time; the open-loop throughput-vs-concurrency sweep and the empty→1M scale ladder (design §2.2–§2.3) are separate profiles.\n");
        if !self.has_java() {
            m.push_str("- **No comparison to EHRbase Java yet** — the numbers above characterize ehrbase-rs alone. A `X× faster` claim is only permitted from a head-to-head run with the config-parity controls (design §3), the JVM-warmup rule applied symmetrically, and the PG-version confound measured explicitly.\n");
        }
        m.push_str("- **Warmup is discarded and applied identically to both servers** — the JVM is warmed, not handicapped (§4.2).\n");
        m.push_str("- **Inter-run variance is reported** (throughput CoV); a difference inside the noise band is not a result (§4.4).\n");
        m.push_str("- Numbers depend on host, container resource pinning, and PostgreSQL configuration — all recorded in the environment block; cross-run comparison is only valid within the same environment.\n");

        m
    }

    fn has_java(&self) -> bool {
        self.results
            .iter()
            .any(|r| r.target == Implementation::EhrbaseJava.label())
    }

    /// Scenarios where `EHRbase` Java's p99 beat ehrbase-rs's (for the mandatory
    /// "where `EHRbase` wins" section).
    fn java_wins(&self) -> Vec<String> {
        let mut out = Vec::new();
        for r in &self.results {
            if r.target != Implementation::EhrbaseRs.label() || !r.gate_ok {
                continue;
            }
            let rs_p99 = r.merged.as_ref().map(|l| l.p99_us);
            let java = self.results.iter().find(|o| {
                o.scenario == r.scenario
                    && o.target == Implementation::EhrbaseJava.label()
                    && o.gate_ok
            });
            if let (Some(rs), Some(java)) = (rs_p99, java)
                && let Some(jl) = java.merged.as_ref()
                && jl.p99_us < rs
            {
                out.push(format!(
                    "{} ({}): EHRbase p99 {} µs vs. ehrbase-rs {} µs",
                    r.scenario, r.description, jl.p99_us, rs
                ));
            }
        }
        out
    }
}

fn fmt_us(v: Option<u64>) -> String {
    v.map_or_else(|| "—".to_owned(), |v| v.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::driver::ScenarioResult;

    fn env() -> EnvBlock {
        EnvBlock {
            run_date: "2026-07-08T00:00:00Z".to_owned(),
            host: crate::host::HostInfo::capture(),
            workload_lock: "abc".to_owned(),
            harness_revision: "deadbeef".to_owned(),
            warmup_iters: 10,
            measure_iters: 100,
            runs: 2,
        }
    }

    #[test]
    fn single_target_report_has_mandatory_sections() {
        let r = ScenarioResult {
            scenario: "W1".to_owned(),
            description: "create EHR".to_owned(),
            target: "ehrbase-rs".to_owned(),
            gate_ok: false,
            gate_status: 201,
            runs: Vec::new(),
            merged: None,
            throughput_median_rps: None,
            throughput_cov: None,
        };
        let report = BenchReport {
            env: env(),
            results: vec![r],
        };
        let md = report.to_markdown();
        assert!(md.contains("## Where EHRbase wins"));
        assert!(md.contains("## Methodology & limitations"));
        assert!(md.contains("No comparison run"));
        assert!(report.to_json().is_ok());
    }
}
