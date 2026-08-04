//! The performance measurement machinery.
//!
//! This is the OPEN-LOOP driver that
//! executes a `kind: performance` case's hospital-simulation workload
//! against a live SUT and produces the re-checkable
//! [`crate::perf::Measurement`] record.
//!
//! The offered load is a deterministic seeded arrival schedule of clinical
//! JOURNEYS (`vocab/journey_catalogue.yaml`): every stage of every journey
//! instance is a planned arrival instant on the global schedule (an order
//! at `t`, its administrations at `t + k·interval`, the laboratory result
//! at `t + Δ`), and the dispatcher fires each at its instant regardless of
//! any other operation's completion — journeys interleave exactly as wards
//! do, never closed-loop users. Every latency is measured from the PLANNED
//! arrival instant, so coordinated omission cannot hide stalls (the
//! `hdrhistogram` crate documents the same correction model). A dependent
//! stage whose prerequisite has not landed at fire time (a stalled SUT)
//! records honestly as an error arrival.
//!
//! Module map: [`client`] the blocking SUT client · [`pack`] the CKM
//! template pack + payload stamping · [`corpus`] the seeded scale corpus +
//! the standing ward · [`schedule`] journey expansion into planned
//! arrivals (uniform + diurnal curves) · [`execute`] the per-stage wire
//! realization + captured-id state · [`window`] the measured window core
//! shared by the class runs (conformance) and the stress ladder
//! (exploration) · [`resources`] the per-container resource sampler + disk
//! anchors (measured context, never verdict-bearing).

pub mod client;
pub mod corpus;
pub mod execute;
pub mod pack;
pub mod resources;
pub mod schedule;
pub mod window;
