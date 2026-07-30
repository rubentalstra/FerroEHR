---
name: rust-hardening-baseline
description: "The #1311 official-books lint baseline is LIVE — what changed for all future work (suppressions, docs, test layout, generated-crate docs, CI lanes)"
metadata: 
  node_type: memory
  type: project
  originSessionId: d949d3c9-d2b6-4161-896b-f79a991124ad
  modified: 2026-07-30T16:49:30.795Z
---

PR #1317 (merged 2026-07-30) enforces the official-Rust-books baseline workspace-wide. Standing consequences for every future change:

- Suppressions are `#[expect(lint, reason = "…")]`; bare `#[allow]` is a compile error (`allow_attributes_without_reason` deny). Reasoned `#[allow]` only for cfg/target-conditional fire (e.g. pointer-width-dependent lints differ between x86_64 and the console's wasm32 lane — prefer restructuring over suppressing, e.g. by-value receivers for small Copy types).
- `missing_docs` + rustdoc deny lints are live; a `cargo doc -D warnings` CI job exists. Generated-crate docs come from the emitter (BMM `documentation` propagation) — a new emitter output path must emit docs + reasoned suppressions or the doc/clippy lanes go red.
- Integration tests: one `tests/it/main.rs` binary per crate (163→~15 consolidation); a new suite = a `mod` line, never a new top-level `tests/*.rs`. Exceptions: admin-ui `e2e_*` binaries (ui-e2e.sh filters by binary name). The nextest `containers` group matches `package(ehrbase) & binary(it) & test(/^(events_amqp|fhir_outbound_amqp|multimedia_s3)::/)` — renaming those modules breaks serialization.
- clippy.toml bans (compile-time): SystemTime::now→jiff, std::env::var→config tree, Uuid::new_v4→now_v7, chrono types→jiff. clippy in-tests scoping (allow-unwrap/expect/panic/print/indexing-slicing-in-tests) covers `#[test]` fns only — integration-binary helpers outside test fns still need file-level `#![expect]`.
- CI lanes split: the console is excluded from `--all-features` (hydrate+ssr `compile_error!` guard) and linted/tested per-feature; all cargo jobs run `--locked`; new msrv (cargo-hack) + scheduled latest-deps + rustdoc jobs.
- `[profile.release]`: `panic = "unwind"` is load-bearing (catch-panic clean-500; tests ignore the panic setting so an abort regression is untestable) — never change; `debug = "line-tables-only"`.

**Why:** owner mandate 2026-07-30 ("very important we follow industry standards"); everything machine-enforced per reliability.md.
**How to apply:** write new code to this bar from the start; when a lint fights a legitimate case, use the reliability.md §escape shapes (never `unwrap_or_default` to dodge a deny). Related: [[pause-after-767-for-rust-practices-update]] (resolved — the hold-point cleared when #1311 merged).
