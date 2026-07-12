//! `EHRbase` application library (the Stage-1 application build).
//!
//! Modern idiomatic Rust on top of the generated `openehr-*` crates:
//! persistence, RM↔JSONB mapping, service layer, and the AQL execution
//! engine. The server binary entry point lives in `main.rs`; this library
//! exposes the application modules so integration tests (and later phases)
//! can drive them directly.

pub mod aql;
pub mod db;
pub mod events;
pub mod fhir_outbound;
pub mod multimedia;
pub mod service;
pub mod signing;
pub mod storage;
pub mod system_log;
pub mod telemetry;
pub mod terminology;
