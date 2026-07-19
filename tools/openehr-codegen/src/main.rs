#![allow(clippy::format_push_string, clippy::too_many_lines)]
// Build-time codegen CLI (never ships in the server): the console IS its
// user interface and a malformed vendored spec must abort loudly, so the
// reliability deny-tier for shipped code is deliberately relaxed here
// (.claude/rules/reliability.md §tools). `let _ = writeln!(String)` is the
// infallible in-memory emit idiom.
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
//! Usage:
//!   `openehr-codegen check`          — load + validate the vendored BMM schemas.
//!   `openehr-codegen emit [OUTDIR]`  — emit Rust into OUTDIR (default:
//!                                       `target/codegen-preview`).

mod analyze;
mod cli;
mod load;
mod plan;
mod render;

fn main() {
    cli::run();
}
