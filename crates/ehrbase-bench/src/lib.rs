//! Honest benchmark harness — ehrbase-rs vs. EHRbase (Java).
//!
//! Implements `docs/design/benchmarking.md`: a defensible, reproducible
//! performance + resource comparison at the openEHR REST surface, built as the
//! point-by-point antidote to "trust me bro" benchmark fakery. Every number in
//! a published report is reproducible from committed scripts against pinned
//! images, or it does not ship.
//!
//! The load-bearing fairness guarantees, encoded here rather than asserted:
//! - **Identical client** ([`target`] drives both SUTs through the conformance
//!   crate's `SutClient` — the same code path for ehrbase-rs and EHRbase Java).
//! - **Coordinated-omission correction** ([`measure`]) so a stalled server
//!   cannot hide its tail latency.
//! - **Pre-registered workload** ([`workload`], W1–W13 from the CNF fixture
//!   corpus, hashed into a `workload.lock`) — frozen before the first measured
//!   run so results cannot be tuned to win.
//! - **Warmup discarded** and **≥N runs with reported variance** ([`driver`]),
//!   applied symmetrically to both servers (the JVM is not handicapped).
//!
//! The report ([`report`]) is generated from the run — never hand-typed — and
//! carries a mandatory "where EHRbase wins" section and a full methodology-
//! limitations block.

pub mod driver;
pub mod measure;
pub mod report;
pub mod seed;
pub mod target;
pub mod workload;
