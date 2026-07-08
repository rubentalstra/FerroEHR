//! Latency measurement with **coordinated-omission correction** (design §0, §4.3).
//!
//! A naive benchmark records only the service time of requests that actually
//! completed, so a server that stalls for a second hides that second from its
//! tail — the single most common way a latency chart lies. In a closed-loop
//! driver we know the *intended* send cadence, so when a request is delayed we
//! also record the "virtual" latencies of the requests that *would* have been
//! sent during the stall. [`LatencyRecorder::record_corrected`] implements the
//! standard `HdrHistogram` correction; [`LatencyRecorder::record`] is the plain
//! path for open-loop use where the interval is not fixed.

use hdrhistogram::Histogram;

/// A latency histogram in microseconds, 3 significant figures (≈0.1% error),
/// range 1 µs … 60 s.
#[derive(Debug)]
pub struct LatencyRecorder {
    hist: Histogram<u64>,
}

impl LatencyRecorder {
    /// A recorder covering 1 µs … 60 s at 3 significant figures.
    #[must_use]
    // parameters are compile-time-valid constants; construction is infallible,
    // so the expect neither fails nor warrants a `# Panics` section.
    #[allow(clippy::expect_used, clippy::missing_panics_doc)]
    pub fn new() -> Self {
        let hist = Histogram::new_with_bounds(1, 60_000_000, 3)
            .expect("histogram bounds 1µs..60s @ 3 sigfig are compile-time valid");
        Self { hist }
    }

    /// Record one observed latency (open-loop / plain path).
    pub fn record(&mut self, latency_us: u64) {
        // saturating_record clamps to the top bucket instead of erroring on an
        // out-of-range value — a 60 s+ request is pinned at the ceiling, never
        // dropped (dropping the worst sample is exactly the omission we avoid).
        self.hist.saturating_record(latency_us);
    }

    /// Record one observed latency **and** the coordinated-omission correction:
    /// if `latency_us` exceeds the `expected_interval_us` at which requests were
    /// meant to be issued, the recorder fills in the virtual latencies of the
    /// requests that could not be sent during the stall (design §4.3).
    pub fn record_corrected(&mut self, latency_us: u64, expected_interval_us: u64) {
        // Clamp to the histogram ceiling so the correction cannot fail on an
        // out-of-range value (a 60 s+ request is pinned at the top bucket,
        // never dropped — dropping the worst sample is the omission we avoid).
        let value = latency_us.min(60_000_000);
        if expected_interval_us == 0 {
            self.hist.saturating_record(value);
        } else {
            // record_correct only errors on an out-of-range value, which we
            // clamped above; the interval fills the coordinated-omission gap.
            let _ = self.hist.record_correct(value, expected_interval_us);
        }
    }

    /// The number of recorded samples (including corrected virtual ones).
    #[must_use]
    pub fn len(&self) -> u64 {
        self.hist.len()
    }

    /// Whether nothing has been recorded.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.hist.is_empty()
    }

    /// The value (µs) at a percentile in `[0, 100]`.
    #[must_use]
    pub fn percentile(&self, quantile: f64) -> u64 {
        self.hist.value_at_percentile(quantile)
    }

    /// The maximum recorded latency (µs).
    #[must_use]
    pub fn max(&self) -> u64 {
        self.hist.max()
    }

    /// The mean latency (µs).
    #[must_use]
    pub fn mean(&self) -> f64 {
        self.hist.mean()
    }

    /// The canonical percentile summary used throughout the report.
    #[must_use]
    pub fn summary(&self) -> LatencySummary {
        LatencySummary {
            count: self.len(),
            p50_us: self.percentile(50.0),
            p90_us: self.percentile(90.0),
            p99_us: self.percentile(99.0),
            p999_us: self.percentile(99.9),
            max_us: self.max(),
            mean_us: self.mean(),
        }
    }

    /// Merge another recorder into this one (for aggregating across runs).
    ///
    /// # Errors
    /// Returns an error string if the histograms are incompatible.
    pub fn merge(&mut self, other: &LatencyRecorder) -> Result<(), String> {
        self.hist.add(&other.hist).map_err(|e| e.to_string())
    }
}

impl Default for LatencyRecorder {
    fn default() -> Self {
        Self::new()
    }
}

/// A percentile summary (all latencies in microseconds) — the shape reported
/// for every scenario, always in full (design §0: never a cherry-picked
/// percentile).
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct LatencySummary {
    /// Number of samples (incl. coordinated-omission corrections).
    pub count: u64,
    /// Median.
    pub p50_us: u64,
    /// 90th percentile.
    pub p90_us: u64,
    /// 99th percentile.
    pub p99_us: u64,
    /// 99.9th percentile.
    pub p999_us: u64,
    /// Maximum observed.
    pub max_us: u64,
    /// Arithmetic mean (reported only alongside the distribution).
    pub mean_us: f64,
}

/// The coefficient of variation (stddev / mean) of a set of per-run values —
/// the inter-run stability signal (design §4.4). Returns `None` for < 2 values
/// or a zero mean. A value above ~0.10 flags a "high variance" result.
#[must_use]
pub fn coefficient_of_variation(values: &[f64]) -> Option<f64> {
    if values.len() < 2 {
        return None;
    }
    let n = values.len() as f64;
    let mean = values.iter().sum::<f64>() / n;
    if mean == 0.0 {
        return None;
    }
    let var = values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / (n - 1.0);
    Some(var.sqrt() / mean)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_and_reports_percentiles() {
        let mut r = LatencyRecorder::new();
        for v in 1..=1000 {
            r.record(v);
        }
        assert_eq!(r.len(), 1000);
        // p50 of 1..=1000 is ~500 (within the 3-sigfig bucket width).
        let p50 = r.percentile(50.0);
        assert!((490..=510).contains(&p50), "p50 was {p50}");
        assert!(r.percentile(99.0) >= 990);
    }

    #[test]
    fn coordinated_omission_inflates_the_tail() {
        // One 1 s stall at a 1 ms intended cadence must surface as a long tail,
        // not a single sample: the correction adds the ~999 virtual requests.
        let mut plain = LatencyRecorder::new();
        let mut corrected = LatencyRecorder::new();
        for _ in 0..999 {
            plain.record(1_000);
            corrected.record_corrected(1_000, 1_000);
        }
        plain.record(1_000_000);
        corrected.record_corrected(1_000_000, 1_000);

        // The plain histogram's p99 hides the stall; the corrected one exposes it.
        assert!(plain.percentile(99.0) < 10_000, "plain p99 hid the stall");
        assert!(
            corrected.percentile(99.0) > 100_000,
            "corrected p99 must expose the stall"
        );
        assert!(corrected.len() > plain.len());
    }

    #[test]
    fn cov_flags_variance() {
        assert!(coefficient_of_variation(&[100.0, 100.0, 100.0]).unwrap() < 0.01);
        assert!(coefficient_of_variation(&[50.0, 100.0, 150.0]).unwrap() > 0.2);
        assert!(coefficient_of_variation(&[1.0]).is_none());
    }
}
