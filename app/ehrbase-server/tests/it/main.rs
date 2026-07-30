//! Integration tests for `ehrbase-server` — the wiring binary's testable
//! library half (`src/lib.rs` behind a thin `main.rs`, the Rust Book ch12.3
//! shape, because a bin-only crate cannot be imported from `tests/`).
//!
//! Only the seam that needs no database, no listener, and no network is
//! exercised here: [`Cli`](ehrbase_server::Cli) parsing (including the
//! `--set key=value` override parser and the subcommand shapes) and the
//! pure-stdout `ehrbase config default` path through
//! [`run`](ehrbase_server::run). Everything past that seam is tested where it
//! lives — platform behaviour in `app/ehrbase/tests/it/`, the assembled
//! ITS-REST router in `app/ehrbase-rest/tests/it/`.
//!
//! One binary per crate, split into topic modules
//! (`.claude/rules/testing.md` §One integration-test binary per crate).

mod wiring;
