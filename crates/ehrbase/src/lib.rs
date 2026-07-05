//! `EHRbase` application library (Stage-1 build, ADR-006).
//!
//! Modern idiomatic Rust on top of the generated `openehr-*` crates:
//! persistence, RM↔JSONB mapping, service layer, and the AQL execution
//! engine. The server binary entry point lives in `main.rs`; this library
//! exposes the application modules so integration tests (and later phases)
//! can drive them directly.

pub mod db;
pub mod service;
pub mod storage;
