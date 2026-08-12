// SPDX-FileCopyrightText: FerroEHR contributors
// SPDX-License-Identifier: MIT

//! `openehr-codegen` binary entry point — a thin wrapper over the library's
//! [`openehr_codegen::run`]; all pipeline logic lives in the library so the
//! emitter invariants are testable (`tests/emitter_invariants.rs`).

fn main() -> std::process::ExitCode {
    openehr_codegen::run()
}
