---
name: expect-used-in-integration-test-helpers
description: A new integration test file under tests/it needs the file-level expect(clippy::expect_used) attribute; clippy.toml's allow-*-in-tests only reaches #[test] fns, not the module's helpers
metadata:
  type: feedback
---

A new file under `app/*/tests/it/` that uses `.expect()` in helper functions
(anything not directly `#[test]`/`#[tokio::test]`-annotated, including async
test bodies' helpers) fails `cargo clippy --all-targets -- -D warnings` with
`clippy::expect_used`, even though `clippy.toml` sets `allow-expect-in-tests`.

**Why:** clippy's in-test scoping covers only `#[test]`-annotated functions;
every existing suite carries a file-level
`#![expect(clippy::expect_used, reason = "clippy's in-test lint scoping ...")]`
(copy the exact reason text from `service_contribution.rs`). Discovered on the
#3212 `access_origins.rs` suite, 2026-09-11.

**How to apply:** when creating a new `tests/it/*.rs` file, add the inner
`#![expect(clippy::expect_used, ...)]` (plus `indexing_slicing` if fixtures are
indexed) right after the module doc comment, before the first `use`. Run the
`--all-targets` clippy lane before nextest, since nextest alone compiles fine.
See [[slim-feature-lanes-gate-helpers]] and [[gate-parity-and-caller-sweeps]].
