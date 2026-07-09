//! Automatic host capture (design §0, §3.1, §7.1).
//!
//! A benchmark number is meaningless without the machine that produced it —
//! different CPUs/RAM/OS differ by multiples, so **every report auto-stamps the
//! host specs** (never a hand-typed label a reader must trust). Cross-machine
//! comparison is therefore visibly invalid: two reports with different `HostInfo`
//! are not comparable, and the report says so.
//!
//! What is captured is the *load-generator* machine. For the
//! containerized dual-stack comparison, the per-container CPU/RAM pinning
//! (design §3.1) is recorded separately alongside this.

use sysinfo::System;

/// A snapshot of the machine running the benchmark — stamped into every report.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HostInfo {
    /// CPU brand/model string (e.g. "Apple M2 Pro", "AMD EPYC 7763").
    pub cpu_model: String,
    /// Physical CPU cores.
    pub physical_cores: Option<usize>,
    /// Logical CPUs (threads).
    pub logical_cpus: usize,
    /// Nominal CPU frequency (MHz), if reported.
    pub cpu_mhz: u64,
    /// Total physical memory (MiB).
    pub total_memory_mib: u64,
    /// OS name (e.g. "Darwin", "Ubuntu").
    pub os_name: String,
    /// OS version.
    pub os_version: String,
    /// Kernel version.
    pub kernel_version: String,
    /// CPU architecture (e.g. "aarch64", "`x86_64`").
    pub arch: String,
    /// Host name.
    pub hostname: String,
}

impl HostInfo {
    /// Capture the current machine's specs.
    #[must_use]
    pub fn capture() -> Self {
        let mut sys = System::new();
        sys.refresh_cpu_all();
        sys.refresh_memory();

        let cpus = sys.cpus();
        let cpu_model = cpus
            .first()
            .map(|c| c.brand().trim().to_owned())
            .filter(|b| !b.is_empty())
            .unwrap_or_else(|| "unknown".to_owned());
        let cpu_mhz = cpus.first().map_or(0, sysinfo::Cpu::frequency);

        HostInfo {
            cpu_model,
            physical_cores: System::physical_core_count(),
            logical_cpus: cpus.len(),
            cpu_mhz,
            // sysinfo reports bytes; MiB is the readable unit for a report.
            total_memory_mib: sys.total_memory() / (1024 * 1024),
            os_name: System::name().unwrap_or_else(|| "unknown".to_owned()),
            os_version: System::os_version().unwrap_or_else(|| "unknown".to_owned()),
            kernel_version: System::kernel_version().unwrap_or_else(|| "unknown".to_owned()),
            arch: System::cpu_arch(),
            hostname: System::host_name().unwrap_or_else(|| "unknown".to_owned()),
        }
    }

    /// A one-line human summary for headings.
    #[must_use]
    pub fn summary_line(&self) -> String {
        let cores = self.physical_cores.map_or_else(
            || format!("{} logical", self.logical_cpus),
            |p| format!("{p} cores / {} threads", self.logical_cpus),
        );
        format!(
            "{} ({cores}, {} MiB RAM) · {} {} · {}",
            self.cpu_model, self.total_memory_mib, self.os_name, self.os_version, self.arch
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capture_populates_the_machine() {
        let h = HostInfo::capture();
        // The load generator always has at least one CPU and some RAM.
        assert!(h.logical_cpus >= 1);
        assert!(h.total_memory_mib > 0);
        assert!(!h.arch.is_empty());
        // The summary line names the CPU and RAM — the fields a reader needs to
        // know a number is not comparable across machines.
        let line = h.summary_line();
        assert!(line.contains("RAM"));
    }
}
