//! Report generation (design §7): `results.json` + a human `REPORT.md`, both
//! **generated from the run, never hand-typed**. The markdown is a *full
//! overview* — coverage across the REST surface, side-by-side latency **and**
//! throughput per operation, a head-to-head win/loss summary, the machine, the
//! payload, a mandatory "where `EHRbase` wins" section, and a methodology block.

use std::collections::BTreeMap;

use crate::driver::ScenarioResult;
use crate::measure::LatencySummary;

/// The environment block stamped into every report (design §7.1).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EnvBlock {
    /// ISO-8601 run timestamp.
    pub run_date: String,
    /// The auto-captured machine specs — always present, so a reader can never
    /// mistake numbers from different hardware as comparable (design §0, §7.1).
    pub host: crate::host::HostInfo,
    /// A description of the payload the workload commits (template + size).
    pub payload: String,
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

const RS: &str = "ehrbase-rs";
const JAVA: &str = "ehrbase-java";

impl BenchReport {
    /// Serialize the machine-readable results.
    ///
    /// # Errors
    /// [`serde_json::Error`] if serialization fails.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Render the human-readable `REPORT.md` — the full overview.
    #[must_use]
    pub fn to_markdown(&self) -> String {
        let mut m = String::new();
        self.write_header(&mut m);
        self.write_environment(&mut m);
        self.write_coverage(&mut m);
        self.write_per_operation(&mut m);
        if self.has_java() {
            self.write_head_to_head(&mut m);
        }
        self.write_where_ehrbase_wins(&mut m);
        self.write_methodology(&mut m);
        m
    }

    fn write_header(&self, m: &mut String) {
        m.push_str("# Benchmark report — ehrbase-rs");
        if self.has_java() {
            m.push_str(" vs. EHRbase (Java)");
        }
        let ops = self.results.iter().filter(|r| r.target == RS).count();
        let groups = self.groups().len();
        m.push_str(&format!(
            "\n\n> Generated from a run (never hand-typed). **{ops} operations across {groups} openEHR resource groups**, latency **and** throughput, on the machine below. Latencies are microseconds; the full distribution is shown for every operation, both directions. Methodology: `docs/design/benchmarking.md`.\n\n"
        ));
    }

    fn write_environment(&self, m: &mut String) {
        let h = &self.env.host;
        m.push_str("## 1. Environment\n\n");
        m.push_str(&format!("> **Machine:** {}\n\n", h.summary_line()));
        m.push_str(&format!(
            "| Field | Value |\n|---|---|\n| Run date | {} |\n| Host name | {} |\n| CPU | {} ({} cores / {} threads @ {} MHz) |\n| Memory | {} MiB |\n| OS | {} {} (kernel {}) |\n| Arch | {} |\n| Payload | {} |\n| Harness rev | {} |\n| Workload lock | `{}` |\n| Warmup / measure / runs | {} / {} / {} |\n\n",
            self.env.run_date,
            h.hostname,
            h.cpu_model,
            h.physical_cores.map_or_else(|| "?".to_owned(), |p| p.to_string()),
            h.logical_cpus,
            h.cpu_mhz,
            h.total_memory_mib,
            h.os_name, h.os_version, h.kernel_version,
            h.arch,
            self.env.payload,
            self.env.harness_revision,
            self.env.workload_lock,
            self.env.warmup_iters, self.env.measure_iters, self.env.runs,
        ));
        m.push_str("> Numbers below are valid only for this machine. A report with a different **Machine** line is not directly comparable (design §3.1).\n\n");
    }

    fn write_coverage(&self, m: &mut String) {
        m.push_str("## 2. Coverage overview\n\n");
        m.push_str("Every openEHR REST resource group is exercised. A group is green only if **every** operation in it passed the pre-flight conformance gate (a wrong response is never timed — design §4.1).\n\n");
        m.push_str(
            "| Resource group | Operations | ehrbase-rs | EHRbase Java |\n|---|--:|:--:|:--:|\n",
        );
        for group in self.groups() {
            let ops: Vec<_> = self
                .results
                .iter()
                .filter(|r| r.group == group && r.target == RS)
                .collect();
            let rs_ok = self.group_all_pass(&group, RS);
            let java_cell = if self.has_java() {
                gate_mark(self.group_all_pass(&group, JAVA))
            } else {
                "—".to_owned()
            };
            m.push_str(&format!(
                "| {} | {} | {} | {} |\n",
                group,
                ops.len(),
                gate_mark(rs_ok),
                java_cell
            ));
        }
        m.push('\n');
    }

    fn write_per_operation(&self, m: &mut String) {
        m.push_str("## 3. Latency & throughput — per operation\n\n");
        m.push_str("Median (p50) / tail (p99, p99.9) latency in µs, and sustained requests/second, per operation per server. `CoV` is inter-run variance (>0.10 = noisy).\n\n");
        for group in self.groups() {
            m.push_str(&format!("### {group}\n\n"));
            m.push_str("| Operation | Server | Gate | p50 | p90 | p99 | p99.9 | max | req/s | CoV |\n|---|---|---|--:|--:|--:|--:|--:|--:|--:|\n");
            for r in self.results.iter().filter(|r| r.group == group) {
                write_row(m, r);
            }
            m.push('\n');
        }
    }

    fn write_head_to_head(&self, m: &mut String) {
        m.push_str("## 4. Head-to-head summary\n\n");
        m.push_str("Per operation: which server has the lower median (p50), lower tail (p99), and higher throughput. `=` = within a bucket; only gate-passing operations on both servers.\n\n");
        m.push_str(
            "| Operation | p50 winner | p99 winner | throughput winner |\n|---|:--:|:--:|:--:|\n",
        );
        let mut rs_p50 = 0;
        let mut rs_p99 = 0;
        let mut rs_tp = 0;
        let mut total = 0;
        for (op, desc) in self.paired_ops() {
            let Some((rs, java)) = self.pair(&op) else {
                continue;
            };
            total += 1;
            let p50 = winner(rs.p50_us, java.p50_us, true);
            let p99 = winner(rs.p99_us, java.p99_us, true);
            let (rtp, jtp) = self.throughputs(&op);
            let tp = winner_f(rtp, jtp, false);
            if p50 == "rs" {
                rs_p50 += 1;
            }
            if p99 == "rs" {
                rs_p99 += 1;
            }
            if tp == "rs" {
                rs_tp += 1;
            }
            m.push_str(&format!(
                "| {op} ({desc}) | {} | {} | {} |\n",
                mark(p50),
                mark(p99),
                mark(tp)
            ));
        }
        m.push_str(&format!(
            "\n**Tally (of {total} comparable operations):** ehrbase-rs leads on p50 in {rs_p50}, p99 in {rs_p99}, throughput in {rs_tp}. The rest go to EHRbase or are within a bucket — see the per-operation table and the section below.\n\n"
        ));
    }

    fn write_where_ehrbase_wins(&self, m: &mut String) {
        m.push_str("## 5. Where EHRbase wins\n\n");
        if !self.has_java() {
            m.push_str("_No comparison run: EHRbase (Java) was not benchmarked here (single-target run). This section is mandatory in a comparative report; run `docker/benchmark/run.sh` for the head-to-head._\n\n");
            return;
        }
        let wins = self.java_wins();
        if wins.is_empty() {
            m.push_str("_No metric where EHRbase was faster in this run (checked p50, p99, and throughput on every operation). Stated plainly, not omitted; re-verify across ≥5 runs before relying on it._\n\n");
        } else {
            for w in wins {
                m.push_str(&format!("- {w}\n"));
            }
            m.push('\n');
        }
    }

    fn write_methodology(&self, m: &mut String) {
        m.push_str("## 6. Methodology & limitations\n\n");
        m.push_str("- **Closed-loop, single-client latency + sustained throughput** per operation; the open-loop concurrency sweep and the empty→1M scale ladder (design §2.2–§2.3) are separate profiles.\n");
        if self.has_java() {
            m.push_str("- **PG-version confound** (§3.3): ehrbase-rs on PG 18, EHRbase Java on the PG 16 its image ships — the deployment comparison. A controlled both-on-PG-16 run isolates engine from database.\n");
        } else {
            m.push_str("- **No EHRbase Java comparison in this run** — a `X× faster` claim needs the head-to-head run with config parity (§3).\n");
        }
        m.push_str("- **Warmup discarded, applied identically to both** — the JVM is warmed, not handicapped (§4.2).\n");
        m.push_str("- **Inter-run variance reported** (CoV); a difference inside the noise band is not a result (§4.4).\n");
        m.push_str("- Numbers depend on host, container resource pinning, and PostgreSQL config — recorded above; comparison is valid only within the same environment.\n");
    }

    // ── helpers ──────────────────────────────────────────────────────────────

    fn has_java(&self) -> bool {
        self.results.iter().any(|r| r.target == JAVA)
    }

    /// Resource groups in first-seen order.
    fn groups(&self) -> Vec<String> {
        let mut seen = Vec::new();
        for r in &self.results {
            if !seen.contains(&r.group) {
                seen.push(r.group.clone());
            }
        }
        seen
    }

    fn group_all_pass(&self, group: &str, target: &str) -> bool {
        let ops: Vec<_> = self
            .results
            .iter()
            .filter(|r| r.group == group && r.target == target)
            .collect();
        !ops.is_empty() && ops.iter().all(|r| r.gate_ok)
    }

    /// (scenario id, description) pairs in order, unique.
    fn paired_ops(&self) -> Vec<(String, String)> {
        let mut out = Vec::new();
        for r in &self.results {
            let key = (r.scenario.clone(), r.description.clone());
            if !out.contains(&key) {
                out.push(key);
            }
        }
        out
    }

    /// The merged latency of (rs, java) for a scenario, both gate-passing.
    fn pair(&self, scenario: &str) -> Option<(LatencySummary, LatencySummary)> {
        let rs = self.find(scenario, RS)?.merged?;
        let java = self.find(scenario, JAVA)?.merged?;
        Some((rs, java))
    }

    fn throughputs(&self, scenario: &str) -> (f64, f64) {
        let rs = self
            .find(scenario, RS)
            .and_then(|r| r.throughput_median_rps)
            .unwrap_or(0.0);
        let java = self
            .find(scenario, JAVA)
            .and_then(|r| r.throughput_median_rps)
            .unwrap_or(0.0);
        (rs, java)
    }

    fn find(&self, scenario: &str, target: &str) -> Option<&ScenarioResult> {
        self.results
            .iter()
            .find(|r| r.scenario == scenario && r.target == target && r.gate_ok)
    }

    /// Every metric (p50, p99, throughput) where `EHRbase` Java beat ehrbase-rs —
    /// a win on *any* is reported, so a better-median-worse-tail JVM result is
    /// surfaced, never hidden behind a single percentile.
    fn java_wins(&self) -> Vec<String> {
        let mut out = Vec::new();
        for (op, desc) in self.paired_ops() {
            let Some((rs, java)) = self.pair(&op) else {
                continue;
            };
            if java.p50_us < rs.p50_us {
                out.push(format!(
                    "{op} ({desc}): p50 — EHRbase {} µs vs. ehrbase-rs {} µs",
                    java.p50_us, rs.p50_us
                ));
            }
            if java.p99_us < rs.p99_us {
                out.push(format!(
                    "{op} ({desc}): p99 — EHRbase {} µs vs. ehrbase-rs {} µs",
                    java.p99_us, rs.p99_us
                ));
            }
            let (rtp, jtp) = self.throughputs(&op);
            if jtp > rtp {
                out.push(format!(
                    "{op} ({desc}): throughput — EHRbase {jtp:.0} req/s vs. ehrbase-rs {rtp:.0} req/s"
                ));
            }
        }
        out
    }
}

fn write_row(m: &mut String, r: &ScenarioResult) {
    if r.gate_ok {
        let l = r.merged.as_ref();
        m.push_str(&format!(
            "| {} ({}) | {} | ✓ {} | {} | {} | {} | {} | {} | {} | {} |\n",
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
            "| {} ({}) | {} | ✗ {} (excluded — wrong response) | — | — | — | — | — | — | — |\n",
            r.scenario, r.description, r.target, r.gate_status
        ));
    }
}

/// The winner of two latencies (lower is better) with a ~10% dead band.
fn winner(rs: u64, java: u64, lower_better: bool) -> &'static str {
    let (a, b) = (rs as f64, java as f64);
    let band = a.max(b) * 0.10;
    if (a - b).abs() <= band {
        "="
    } else if (a < b) == lower_better {
        "rs"
    } else {
        "java"
    }
}

fn winner_f(rs: f64, java: f64, lower_better: bool) -> &'static str {
    let band = rs.max(java) * 0.10;
    if (rs - java).abs() <= band {
        "="
    } else if (rs < java) == lower_better {
        "rs"
    } else {
        "java"
    }
}

fn mark(w: &str) -> &'static str {
    match w {
        "rs" => "ehrbase-rs",
        "java" => "EHRbase",
        _ => "=",
    }
}

fn gate_mark(ok: bool) -> String {
    if ok {
        "✓".to_owned()
    } else {
        "✗".to_owned()
    }
}

fn fmt_us(v: Option<u64>) -> String {
    v.map_or_else(|| "—".to_owned(), |v| v.to_string())
}

/// A per-group operation tally (unused publicly; kept for potential JSON export).
#[must_use]
pub fn group_counts(results: &[ScenarioResult]) -> BTreeMap<String, usize> {
    let mut out = BTreeMap::new();
    for r in results {
        if r.target == RS {
            *out.entry(r.group.clone()).or_insert(0) += 1;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::driver::ScenarioResult;

    fn env() -> EnvBlock {
        EnvBlock {
            run_date: "2026-07-08T00:00:00Z".to_owned(),
            host: crate::host::HostInfo::capture(),
            payload: "test (1 KB)".to_owned(),
            workload_lock: "abc".to_owned(),
            harness_revision: "deadbeef".to_owned(),
            warmup_iters: 10,
            measure_iters: 100,
            runs: 2,
        }
    }

    fn passing(
        scenario: &str,
        group: &str,
        target: &str,
        p50: u64,
        p99: u64,
        rps: f64,
    ) -> ScenarioResult {
        ScenarioResult {
            scenario: scenario.to_owned(),
            group: group.to_owned(),
            description: "op".to_owned(),
            target: target.to_owned(),
            gate_ok: true,
            gate_status: 200,
            runs: Vec::new(),
            merged: Some(LatencySummary {
                count: 100,
                p50_us: p50,
                p90_us: p50,
                p99_us: p99,
                p999_us: p99,
                max_us: p99,
                mean_us: p50 as f64,
            }),
            throughput_median_rps: Some(rps),
            throughput_cov: Some(0.05),
        }
    }

    #[test]
    fn full_report_has_all_sections() {
        let report = BenchReport {
            env: env(),
            results: vec![
                passing("ehr_create", "EHR", "ehrbase-rs", 4000, 9000, 220.0),
                passing("ehr_create", "EHR", "ehrbase-java", 5000, 11000, 190.0),
                passing(
                    "composition_get",
                    "COMPOSITION",
                    "ehrbase-rs",
                    3700,
                    6200,
                    260.0,
                ),
                passing(
                    "composition_get",
                    "COMPOSITION",
                    "ehrbase-java",
                    1900,
                    9100,
                    400.0,
                ),
            ],
        };
        let md = report.to_markdown();
        assert!(md.contains("## 1. Environment"));
        assert!(md.contains("## 2. Coverage overview"));
        assert!(md.contains("## 3. Latency & throughput"));
        assert!(md.contains("## 4. Head-to-head summary"));
        assert!(md.contains("## 5. Where EHRbase wins"));
        assert!(md.contains("## 6. Methodology"));
        // EHRbase wins composition_get on p50 + throughput — must be surfaced.
        assert!(md.contains("composition_get"));
        assert!(report.to_json().is_ok());
        assert_eq!(group_counts(&report.results).len(), 2);
    }

    #[test]
    fn single_target_report_flags_missing_comparison() {
        let report = BenchReport {
            env: env(),
            results: vec![passing(
                "ehr_create",
                "EHR",
                "ehrbase-rs",
                4000,
                9000,
                220.0,
            )],
        };
        let md = report.to_markdown();
        assert!(md.contains("No comparison run"));
    }
}
