//! The performance schedule machinery — conformance-by-MEASUREMENT: the
//! `kind: performance` case model (class, corpus, open-loop workload,
//! thresholds), the measurement record (counts, errors, percentiles, the
//! encoded HDR histogram so every threshold is RE-CHECKABLE from the
//! artifact), and the class-verdict pure function (earned | not-earned).
//!
//! The class floors are the population-anchored [legislated] defaults the
//! schedule publishes (POC 2/s · S 15/s · L 150/s · R 1,500/s peak
//! arrivals, p99 ≤ 1 s, error rate 0) — implemented exactly as specified;
//! upstream ratification owns any change. The workload model is OPEN-LOOP
//! (a seeded arrival schedule, never closed-loop users) so coordinated
//! omission cannot hide stalls.

use base64::Engine;
use hdrhistogram::Histogram;
use hdrhistogram::serialization::{Deserializer, Serializer, V2Serializer};
use serde::{Deserialize, Serialize};

use crate::ids::{CaseId, CorpusKey};

/// The volumetric class ladder (the §8.11 step-2c selection key).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PerfClass {
    #[serde(rename = "POC")]
    Poc,
    S,
    L,
    R,
}

impl PerfClass {
    /// The class's offered-load floor (peak API arrivals/s, sustained) —
    /// the published [legislated] defaults.
    #[must_use]
    pub fn arrival_floor_per_s(self) -> f64 {
        match self {
            PerfClass::Poc => 2.0,
            PerfClass::S => 15.0,
            PerfClass::L => 150.0,
            PerfClass::R => 1_500.0,
        }
    }
}

/// An offered rate (`15/s`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RatePerSecond(pub f64);

impl<'de> Deserialize<'de> for RatePerSecond {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        let raw = s
            .strip_suffix("/s")
            .ok_or_else(|| serde::de::Error::custom(format!("rate {s:?} must end in /s")))?;
        raw.trim()
            .parse::<f64>()
            .map(Self)
            .map_err(|e| serde::de::Error::custom(format!("rate {s:?}: {e}")))
    }
}

impl Serialize for RatePerSecond {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&format!("{}/s", self.0))
    }
}

/// An ISO 8601 duration in the restricted `PTnHnMnS`/`PTnM` shapes the
/// workload blocks use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorkloadDuration(pub u64);

impl<'de> Deserialize<'de> for WorkloadDuration {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        parse_iso_duration_secs(&s).map(Self).ok_or_else(|| {
            serde::de::Error::custom(format!("duration {s:?} is not PT[nH][nM][nS]"))
        })
    }
}

impl Serialize for WorkloadDuration {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let (h, m, s) = (self.0 / 3600, (self.0 % 3600) / 60, self.0 % 60);
        let mut out = String::from("PT");
        if h > 0 {
            out.push_str(&format!("{h}H"));
        }
        if m > 0 {
            out.push_str(&format!("{m}M"));
        }
        if s > 0 || (h == 0 && m == 0) {
            out.push_str(&format!("{s}S"));
        }
        serializer.serialize_str(&out)
    }
}

fn parse_iso_duration_secs(s: &str) -> Option<u64> {
    let rest = s.strip_prefix("PT")?;
    let mut total: u64 = 0;
    let mut number = String::new();
    for c in rest.chars() {
        if c.is_ascii_digit() {
            number.push(c);
        } else {
            let n: u64 = number.parse().ok()?;
            number.clear();
            total = total.checked_add(match c {
                'H' => n.checked_mul(3600)?,
                'M' => n.checked_mul(60)?,
                'S' => n,
                _ => return None,
            })?;
        }
    }
    number.is_empty().then_some(total)
}

/// A percentage share of scheduled arrivals (`61%`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Percent(pub f64);

impl<'de> Deserialize<'de> for Percent {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        let raw = s
            .strip_suffix('%')
            .ok_or_else(|| serde::de::Error::custom(format!("share {s:?} must end in %")))?;
        raw.trim()
            .parse::<f64>()
            .map(Self)
            .map_err(|e| serde::de::Error::custom(format!("share {s:?}: {e}")))
    }
}

impl Serialize for Percent {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&format!("{}%", self.0))
    }
}

/// The OPEN-LOOP offered load: a seeded arrival schedule.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Workload {
    pub arrival_rate: RatePerSecond,
    pub warmup: WorkloadDuration,
    pub duration: WorkloadDuration,
    /// mix = share of scheduled ARRIVALS per named operation.
    #[serde(deserialize_with = "crate::model::de::ordered_map")]
    pub mix: Vec<(String, Percent)>,
}

impl Workload {
    /// The mix must sum to 100% (±0.01).
    ///
    /// # Errors
    /// Returns the actual sum on violation.
    pub fn check_mix(&self) -> Result<(), String> {
        let sum: f64 = self.mix.iter().map(|(_, p)| p.0).sum();
        if (sum - 100.0).abs() < 0.01 {
            Ok(())
        } else {
            Err(format!("workload mix sums to {sum}%, must be 100%"))
        }
    }
}

/// One threshold (ALL must hold in the single measured run).
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Threshold {
    pub metric: Metric,
    /// The operation the metric is scoped to (absent = run-wide).
    #[serde(default)]
    pub operation: Option<String>,
    /// Upper bound (latencies: milliseconds; error_rate: fraction).
    #[serde(default)]
    pub max: Option<f64>,
    /// Lower bound (offered_load_sustained: arrivals/s).
    #[serde(default)]
    pub min: Option<f64>,
}

/// The closed threshold-metric vocabulary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Metric {
    LatencyP50,
    LatencyP90,
    LatencyP99,
    ErrorRate,
    OfferedLoadSustained,
}

/// A `kind: performance` case (its own schema family; carries `class`
/// instead of `capabilities`).
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PerformanceCase {
    pub id: CaseId,
    /// Always the literal `performance`.
    pub kind: String,
    pub component: String,
    pub description: String,
    pub test_purpose: String,
    pub spec_refs: Vec<String>,
    /// The selection key (§8.11 step 2c) — the claimed class selects.
    pub class: PerfClass,
    pub corpus: CorpusKey,
    pub workload: Workload,
    pub thresholds: Vec<Threshold>,
}

impl PerformanceCase {
    /// Shape invariants: kind literal, mix sums, thresholds carry a bound,
    /// and the offered-load floor is consistent with the class table.
    ///
    /// # Errors
    /// Returns the violated invariant.
    pub fn check_invariants(&self) -> Result<(), String> {
        if self.kind != "performance" {
            return Err(format!("kind must be `performance`, got {:?}", self.kind));
        }
        if self.component != "PERFORMANCE" {
            return Err(format!(
                "component must be PERFORMANCE, got {:?}",
                self.component
            ));
        }
        self.workload.check_mix()?;
        for t in &self.thresholds {
            if t.max.is_none() && t.min.is_none() {
                return Err("threshold carries neither max nor min".to_owned());
            }
        }
        let floor = self.class.arrival_floor_per_s();
        if self.workload.arrival_rate.0 < floor {
            return Err(format!(
                "workload arrival_rate {}/s is below the class floor {floor}/s",
                self.workload.arrival_rate.0
            ));
        }
        Ok(())
    }
}

/// One operation's measurement record — thresholds re-checkable from the
/// artifact: the histogram is the standard V2 encoding, base64-wrapped.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OperationMeasurement {
    pub operation: String,
    pub requests: u64,
    pub errors: u64,
    pub latency_ms_p50: f64,
    pub latency_ms_p90: f64,
    pub latency_ms_p99: f64,
    /// Standard HdrHistogram V2 encoding, base64 (values in microseconds).
    pub hdr_v2_base64: String,
}

impl OperationMeasurement {
    /// Build the record from a recorded histogram (values in microseconds).
    ///
    /// # Errors
    /// Returns a message on serialization failure.
    pub fn from_histogram(
        operation: &str,
        histogram: &Histogram<u64>,
        errors: u64,
    ) -> Result<Self, String> {
        let mut buffer = Vec::new();
        V2Serializer::new()
            .serialize(histogram, &mut buffer)
            .map_err(|e| format!("hdr serialize: {e}"))?;
        Ok(Self {
            operation: operation.to_owned(),
            requests: histogram.len(),
            errors,
            latency_ms_p50: histogram.value_at_quantile(0.50) as f64 / 1_000.0,
            latency_ms_p90: histogram.value_at_quantile(0.90) as f64 / 1_000.0,
            latency_ms_p99: histogram.value_at_quantile(0.99) as f64 / 1_000.0,
            hdr_v2_base64: base64::engine::general_purpose::STANDARD.encode(&buffer),
        })
    }

    /// Decode the embedded histogram (the RE-CHECK path: any consumer can
    /// recompute every percentile from the artifact alone).
    ///
    /// # Errors
    /// Returns a message on a corrupt encoding.
    pub fn decode_histogram(&self) -> Result<Histogram<u64>, String> {
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(&self.hdr_v2_base64)
            .map_err(|e| format!("hdr base64: {e}"))?;
        Deserializer::new()
            .deserialize(&mut bytes.as_slice())
            .map_err(|e| format!("hdr decode: {e}"))
    }
}

/// The whole measured run for one performance case.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Measurement {
    pub case: CaseId,
    pub class: PerfClass,
    /// The offered load the schedule actually sustained (arrivals/s).
    pub offered_load_sustained: f64,
    pub operations: Vec<OperationMeasurement>,
    /// The verdict — computed, never asserted.
    pub verdict: ClassVerdict,
}

/// Class verdicts (the second machinery's output).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ClassVerdict {
    Earned,
    NotEarned,
}

/// The pure class-verdict function: every threshold of the case holds in
/// the single measured run ⇒ `earned`, else `not-earned`. Latency metrics
/// are re-derived from the DECODED histograms (never trusted from the
/// summary fields), which is what makes the record re-checkable.
///
/// # Errors
/// Returns a message when a threshold references an operation the run did
/// not measure, or a histogram fails to decode.
pub fn class_verdict(
    case: &PerformanceCase,
    offered_load_sustained: f64,
    operations: &[OperationMeasurement],
) -> Result<(ClassVerdict, Vec<String>), String> {
    let mut violations = Vec::new();
    for threshold in &case.thresholds {
        match threshold.metric {
            Metric::OfferedLoadSustained => {
                if let Some(min) = threshold.min
                    && offered_load_sustained < min
                {
                    violations.push(format!(
                        "offered_load_sustained {offered_load_sustained}/s < min {min}/s"
                    ));
                }
            }
            Metric::ErrorRate => {
                let (requests, errors) = operations.iter().fold((0_u64, 0_u64), |(r, e), m| {
                    (r.saturating_add(m.requests), e.saturating_add(m.errors))
                });
                let rate = if requests == 0 {
                    1.0
                } else {
                    errors as f64 / requests as f64
                };
                if let Some(max) = threshold.max
                    && rate > max
                {
                    violations.push(format!("error_rate {rate} > max {max}"));
                }
            }
            Metric::LatencyP50 | Metric::LatencyP90 | Metric::LatencyP99 => {
                let quantile = match threshold.metric {
                    Metric::LatencyP50 => 0.50,
                    Metric::LatencyP90 => 0.90,
                    _ => 0.99,
                };
                let targets: Vec<&OperationMeasurement> = match &threshold.operation {
                    Some(op) => {
                        let found: Vec<_> =
                            operations.iter().filter(|m| &m.operation == op).collect();
                        if found.is_empty() {
                            return Err(format!("threshold references unmeasured operation {op}"));
                        }
                        found
                    }
                    None => operations.iter().collect(),
                };
                for m in targets {
                    let histogram = m.decode_histogram()?;
                    let value_ms = histogram.value_at_quantile(quantile) as f64 / 1_000.0;
                    if let Some(max) = threshold.max
                        && value_ms > max
                    {
                        violations.push(format!(
                            "{} {:?} {value_ms}ms > max {max}ms",
                            m.operation, threshold.metric
                        ));
                    }
                }
            }
        }
    }
    let verdict = if violations.is_empty() {
        ClassVerdict::Earned
    } else {
        ClassVerdict::NotEarned
    };
    Ok((verdict, violations))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic)] // test assertions/fixtures
mod tests {
    use super::*;

    fn case(rate: &str) -> PerformanceCase {
        serde_saphyr::from_str(&format!(
            "id: PERF-mixed_load-class_S\nkind: performance\ncomponent: PERFORMANCE\ndescription: d\ntest_purpose: t\nspec_refs: [\"CNF 2.0 performance schedule\"]\nclass: S\ncorpus: cnf.scale.100k\nworkload:\n  arrival_rate: {rate}\n  warmup: PT5M\n  duration: PT1H\n  mix: {{ composition_read: 61%, adhoc_query: 30%, composition_commit: 8%, ehr_create: 1% }}\nthresholds:\n  - {{ metric: latency_p99, operation: composition_read, max: 1000 }}\n  - {{ metric: error_rate, max: 0 }}\n  - {{ metric: offered_load_sustained, min: 15 }}\n"
        ))
        .unwrap()
    }

    fn histogram(values_us: &[u64]) -> Histogram<u64> {
        let mut h = Histogram::new(3).unwrap();
        for v in values_us {
            h.record(*v).unwrap();
        }
        h
    }

    #[test]
    fn the_class_s_case_parses_and_holds_its_floor() {
        let c = case("15/s");
        assert!(c.check_invariants().is_ok());
        assert!(case("40/s").check_invariants().is_ok());
        assert!(case("2/s").check_invariants().is_err()); // below the S floor
    }

    #[test]
    fn verdicts_recheck_from_the_encoded_histogram() {
        let c = case("15/s");
        // fast reads: p99 well under 1000ms
        let fast = histogram(&[10_000, 20_000, 30_000, 50_000]);
        let m = OperationMeasurement::from_histogram("composition_read", &fast, 0).unwrap();
        let decoded = m.decode_histogram().unwrap();
        assert_eq!(decoded.len(), 4);
        let (verdict, violations) = class_verdict(&c, 15.2, &[m.clone()]).unwrap();
        assert_eq!(verdict, ClassVerdict::Earned);
        assert!(violations.is_empty());

        // a stalled tail: p99 ~5s -> not earned, violation named
        let slow = histogram(&[10_000, 20_000, 5_000_000, 5_100_000]);
        let m2 = OperationMeasurement::from_histogram("composition_read", &slow, 0).unwrap();
        let (verdict, violations) = class_verdict(&c, 15.2, &[m2]).unwrap();
        assert_eq!(verdict, ClassVerdict::NotEarned);
        assert!(violations.iter().any(|v| v.contains("LatencyP99")));

        // under-offered load
        let (verdict, violations) = class_verdict(&c, 12.0, &[m]).unwrap();
        assert_eq!(verdict, ClassVerdict::NotEarned);
        assert!(
            violations
                .iter()
                .any(|v| v.contains("offered_load_sustained"))
        );
    }

    #[test]
    fn error_rate_and_tampered_summaries_cannot_hide() {
        let c = case("15/s");
        let h = histogram(&[10_000; 8]);
        let mut m = OperationMeasurement::from_histogram("composition_read", &h, 1).unwrap();
        let (verdict, _) = class_verdict(&c, 15.0, &[m.clone()]).unwrap();
        assert_eq!(verdict, ClassVerdict::NotEarned); // one error breaks error_rate 0

        // Tamper the SUMMARY p99 — the verdict still re-derives from the
        // histogram, so the tamper cannot flip it.
        m.errors = 0;
        m.latency_ms_p99 = 0.001;
        let slow = histogram(&[5_000_000; 8]);
        let mut buffer = Vec::new();
        V2Serializer::new().serialize(&slow, &mut buffer).unwrap();
        m.hdr_v2_base64 = base64::engine::general_purpose::STANDARD.encode(&buffer);
        let (verdict, violations) = class_verdict(&c, 15.0, &[m]).unwrap();
        assert_eq!(verdict, ClassVerdict::NotEarned);
        assert!(!violations.is_empty());
    }

    #[test]
    fn durations_and_rates_round_trip() {
        assert_eq!(parse_iso_duration_secs("PT5M"), Some(300));
        assert_eq!(parse_iso_duration_secs("PT1H"), Some(3600));
        assert_eq!(parse_iso_duration_secs("PT1H30M15S"), Some(5415));
        assert_eq!(parse_iso_duration_secs("P1D"), None);
        assert_eq!(PerfClass::R.arrival_floor_per_s(), 1_500.0);
    }
}
