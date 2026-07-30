---
name: rust-practices-1311
description: "The #1311 hardened Rust baseline (2026-07-30) — expect-with-reason suppressions, banned APIs, one tests/it binary per crate, Book-ch11 test shapes"
metadata:
  type: feedback
---

Issue #1311 (closed 2026-07-30) hardened the whole-workspace Rust baseline.
The full spec lives in `.claude/rules/{rust-style,reliability,testing}.md` +
root `Cargo.toml [workspace.lints]` + `clippy.toml` — those are authoritative;
this memory is the working checklist:

- Suppressions: `#[expect(lint, reason = "…")]`, smallest scope;
  `#[allow]` only for cfg-conditional fire; the one expect_used escape needs
  a should-phrased message + inspection-proving reason.
- New deny tier incl. `unreachable!`, `indexing_slicing`, `string_slice`,
  `iter_over_hash_type` (determinism), `unused_result_ok`, numeric honesty;
  `missing_docs` warn workspace-wide (CI-fatal).
- Banned APIs (clippy.toml disallowed): chrono types, `SystemTime::now`,
  `env::var`, `Uuid::new_v4` — jiff/config-tree/uuidv7 instead.
- Tests: ONE `tests/it/main.rs` binary per crate, one mod per topic — never
  a new top-level `tests/*.rs`; Result-returning tests preferred;
  `should_panic` always with `expected`; container suites matched by module
  prefix in `.config/nextest.toml` (renames must update it).
- Docs: one-sentence first line; `# Errors`/`# Panics`; rustdoc CI job is a
  gate (`-D warnings`); `doc(alias = "SPEC_NAME")` encouraged.

**How to apply:** name these in every implementer-worker prompt; run
`cargo clippy --workspace --all-targets --all-features` (never scoped-only)
before any merge.
