//! Honest benchmark harness — ehrbase-rs vs. `EHRbase` (Java).
//!
//! Implements `docs/design/benchmarking.md`: a defensible, reproducible
//! performance + resource comparison at the openEHR REST surface, built as the
//! point-by-point antidote to "trust me bro" benchmark fakery. Every number in
//! a published report is reproducible from committed scripts against pinned
//! images, or it does not ship.
//!
//! The load-bearing fairness guarantees, encoded here rather than asserted:
//! - **Identical client** ([`target`] drives both SUTs through the conformance
//!   crate's `SutClient` — the same code path for ehrbase-rs and `EHRbase` Java).
//! - **Coordinated-omission correction** ([`measure`]) so a stalled server
//!   cannot hide its tail latency.
//! - **Pre-registered workload** ([`workload`], W1–W13 from the CNF fixture
//!   corpus, hashed into a `workload.lock`) — frozen before the first measured
//!   run so results cannot be tuned to win.
//! - **Warmup discarded** and **≥N runs with reported variance** ([`driver`]),
//!   applied symmetrically to both servers (the JVM is not handicapped).
//!
//! The report ([`report`]) is generated from the run — never hand-typed — and
//! carries a mandatory "where `EHRbase` wins" section and a full methodology-
//! limitations block.
//!
//! Two pedantic lints are allowed crate-wide as they fight this crate's nature,
//! not any defect: `format_push_string` (the report builder appends `format!`
//! to a `String` — the natural idiom) and `cast_precision_loss` (metric math
//! casts request counts / microsecond latencies to `f64`; sub-millisecond loss
//! on such magnitudes is irrelevant to a throughput or `CoV` figure).
#![allow(clippy::format_push_string, clippy::cast_precision_loss)]

pub mod driver;
pub mod host;
pub mod measure;
pub mod report;
pub mod seed;
pub mod target;
pub mod workload;

/// A harness error.
#[derive(Debug, thiserror::Error)]
pub enum BenchError {
    /// A transport-level failure reaching the SUT.
    #[error("transport: {0}")]
    Transport(#[from] conformance::harness::TransportError),
    /// A fixture could not be read.
    #[error("fixture: {0}")]
    Fixture(String),
    /// The SUT returned an unexpected response during setup.
    #[error("unexpected: {0}")]
    Unexpected(String),
}
