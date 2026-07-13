//! The ehrbase-rs Conformance Catalogue (ECC) engine — our own
//! acceptance instrument (`docs/design/conformance-framework.md`, v3.1).
//!
//! **Our own conformance framework.** The primary identity system is the ECC
//! catalogue ([`model::catalog`]): every case carries a stable
//! `ECC-<AREA>-<NNN>` number allocated in the committed
//! `inventory/ecc-catalog.tsv`, never reused. The official openEHR CNF corpus
//! (schedule, Robot suite, ITS-REST OAS, AQL corpus — vendored under
//! `docs/specs/openehr/`) is **design-time reference reading only** — we
//! studied it, took what is good, and build better: a spec-first case
//! universe over the *current* pinned specs (RM 1.2.0, ITS-REST 1.0.3,
//! AQL 1.1, TERM 3.1.0), generated data sets instead of 2019-era hand-copied
//! fixtures, and machine-enforced profile verdicts. No runtime mapping to
//! the legacy corpus exists anywhere in this crate.
//!
//! Layered layout (enterprise shape, one responsibility per layer):
//!
//! - [`model`] — the domain: case metadata, areas, profiles, the ECC catalogue.
//! - [`testdata`] — typed access to test data (vendored fixtures we reuse as
//!   inputs, and our own generated data sets).
//! - [`engine`] — execution: transport, SUT lifecycles, assertions, registry,
//!   the runner.
//! - [`reporting`] — the machine/human result artifacts (`results.json`,
//!   `CONFORMANCE_REPORT`/`CATALOG` markdown, the Conformance
//!   Statement + Certificate, the four badges).
//! - [`suites`] — the case implementations, one module per area/chapter.
//!
//! Two pedantic lints are allowed crate-wide because they fight the natural
//! shape of a data-heavy conformance registry, not any real defect:
//! `too_many_lines` (the per-chapter `entries()` functions are long, flat
//! `vec![]` case tables) and `needless_pass_by_value` (the case-builder helpers
//! take small owned payloads by value for call-site ergonomics — a consistent
//! idiom across every `suites/*` module).
#![allow(clippy::too_many_lines, clippy::needless_pass_by_value)]

pub mod engine;
pub mod model;
pub mod reporting;
pub mod suites;
pub mod testdata;
pub mod ts;

// Stable public facade: the flat module paths are the crate API (used by the
// suites, the CLI, and the integration tests); the directories above are the
// maintenance layout.
pub use engine::{assert, client, flow, harness, registry, run, sut};
pub use model::{adjudication, case, catalog, profile, provenance, version};
pub use reporting::{report, results};
pub use testdata::fixtures;
