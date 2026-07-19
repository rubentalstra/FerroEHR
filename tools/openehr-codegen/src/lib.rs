// Build-time codegen library (never ships in the server): the console IS its
// user interface and a malformed vendored spec must abort loudly, so the
// reliability deny-tier for shipped code is deliberately relaxed here
// (.claude/rules/reliability.md §tools). `let _ = writeln!(String)` is the
// infallible in-memory emit idiom.
#![allow(clippy::format_push_string, clippy::too_many_lines)]
#![allow(
    clippy::panic,
    clippy::print_stdout,
    clippy::print_stderr,
    clippy::let_underscore_must_use
)]

//! `openehr-codegen` — generates the openEHR spec crates from the vendored BMM
//! meta-model, structured as a four-stage pipeline:
//! [`load`] → [`analyze`] → [`plan`] → [`render`], driven by [`cli`].
//!
//! The stages are library modules (not a binary-only tree) so the emitter's
//! invariants — completeness, constructibility, determinism, source-package
//! mirroring, downstream-closure correctness, decision-map integrity — are
//! tested as properties over the real pipeline on the real vendored inputs
//! (`tests/emitter_invariants.rs`), through the curated [`testsupport`] surface.
//!
//! Usage:
//!   `openehr-codegen check`          — load + validate the vendored BMM schemas.
//!   `openehr-codegen emit [OUTDIR]`  — emit Rust into the spec crates.

mod analyze;
mod cli;
mod load;
mod plan;
mod render;
pub mod testsupport;

/// Run the codegen CLI (the binary entry point): parse `argv`, dispatch the
/// subcommand, and `std::process::exit` on error.
///
/// # Panics
/// Panics (the loud tool-mode backstop) if a vendored input is malformed in a
/// way the emitter's invariants forbid — e.g. a non-constructible type would be
/// emitted (see `analyze::Model::assert_constructible`).
pub fn run() {
    cli::run();
}
