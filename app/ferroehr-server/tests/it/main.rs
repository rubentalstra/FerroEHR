// SPDX-FileCopyrightText: FerroEHR contributors
// SPDX-License-Identifier: MIT

//! Integration tests for `ferroehr-server` — the wiring binary's testable
//! library half (`src/lib.rs` behind a thin `main.rs`, the Rust Book ch12.3
//! shape, because a bin-only crate cannot be imported from `tests/`).
//!
//! Only the seam that needs no database, no listener, and no network is
//! exercised here: the authorization construction seam
//! ([`build_authz`](ferroehr_server::build_authz), `authz_wiring`), [`Cli`](ferroehr_server::Cli) parsing (including the
//! `--set key=value` override parser and the subcommand shapes) and the
//! pure-stdout `ferroehr config default` path through
//! [`run`](ferroehr_server::run). Everything past that seam is tested where it
//! lives — platform behaviour in `app/ferroehr/tests/it/`, the assembled
//! ITS-REST router in `app/ferroehr-rest/tests/it/`.
//!
//! One binary per crate, split into topic modules
//! (`.claude/rules/testing.md` §One integration-test binary per crate).

mod authz_wiring;
mod wiring;
