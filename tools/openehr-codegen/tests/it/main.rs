// SPDX-FileCopyrightText: FerroEHR contributors
// SPDX-License-Identifier: MIT

//! The crate's single integration-test binary (one binary per crate: cargo
//! compiles and links every top-level `tests/*.rs` separately —
//! <https://doc.rust-lang.org/cargo/reference/cargo-targets.html>). Each topic
//! is a module; nextest still runs every test in its own process.

mod emitter_invariants;
mod model_query;
