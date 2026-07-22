//! The SUT client the benchmark drives every target through.
//!
//! Absorbed from the retired ECC conformance harness so the benchmark keeps a
//! provably-identical request path for both SUTs (the core fairness guarantee —
//! `lib.rs` §methodology) and stays a self-contained tool. Pruned to what the
//! benchmark uses: the [`harness`] request/response types + [`Transport`]
//! trait, the reqwest [`transport`] client, the [`descriptor`]/[`builtin`]
//! target config, the versioned-id [`headers`] parse, and read-only
//! [`fixtures`] access to the vendored CNF corpus. The ECC-specific machinery
//! (catalog, suites, reporting, adjudications, the edition ladder, the fixture
//! manifest) is not carried over.
//!
//! [`Transport`]: harness::Transport

pub mod builtin;
pub mod descriptor;
pub mod fixtures;
pub mod harness;
pub mod headers;
pub mod transport;
