---
paths: ["crates/**/*.rs"]
---

# Rust style — faithful port first (application code)

This applies to hand-written `.rs` files: the EHRbase application port
(`ehrbase-*`) and the hand-written spec crates (`openehr-serde`, `openehr-flat`,
`openehr-term`, the tooling crates, and `*_impl.rs` behaviour files). For the
EHRbase Java port, faithfulness beats idiomatic Rust during Stage 1
(PORT_MASTER_PLAN.md Section 4).

**Generated files do not follow these rules.** The openEHR spec crates
(`openehr-base`, `openehr-rm`, `openehr-am`) are generated from BMM (ADR-004);
every `// @generated` file is idiomatic-by-construction and **must never be
hand-edited** — change the emitter (`openehr-codegen`) and regenerate. The
"mirror the source" and PORT STATUS-trailer rules below do **not** apply to
them.

## Mirror the source

- Same file/module name (CamelCase Java basename → snake_case Rust basename,
  e.g. `AqlSqlLayer.java` → `aql_sql_layer.rs`), same type names, same method
  names, same field order, same control flow, same order of declarations. A
  reviewer should be able to diff the two side by side.
- Do not redesign or "improve" during translation. Note an idiomatic
  alternative in a comment instead of applying it.
- Two standing exceptions need no special justification: a Java constructor
  that throws becomes `fn new(...) -> Result<Self, E>`; `AutoCloseable`/
  `close()` becomes `impl Drop`.
- A borrow-checker reshape is allowed only when marked
  `// PORT NOTE: reshaped for borrowck`, and only when a faithful shape is
  genuinely impossible.

## Annotation vocabulary (grep-able, mandatory where relevant)

`// TODO(port):` unfinished translation · `// PERF(port):` a place to
optimize after parity · `// PORT NOTE:` a deliberate structural deviation ·
`// SAFETY:` justification for any `unsafe` (expect almost none — this is a
web service, not a runtime).

## PORT STATUS trailer (required at end of every ported/transcribed file)

```rust
// ─────────────────────────────────────────────
// PORT STATUS
//   source: <java file or spec section this replaces>
//   source_loc: <line count of the Java file, or spec section id>
//   confidence: high | medium | low
//   todos: <count of TODO(port) in this file>
//   note: <one line for Phase B triage>
// ─────────────────────────────────────────────
```

## Type and error conventions

- `thiserror` error enums in library crates; `anyhow` only in the
  `ehrbase` binary. No `unwrap`/`expect` outside `#[cfg(test)]`; use
  `todo!()` or `// TODO(port):` where the real value is not yet available.
- Closed openEHR subtype sets (`DATA_VALUE`, `ITEM`, `CONTENT_ITEM`,
  `PARTY_PROXY`, `VERSION<T>`) are Rust `enum`s. Trait objects only for
  genuinely archetype-driven runtime polymorphism.
- `PATHABLE.parent()` and any other back-reference use `Weak<..>` or a
  path-index, never an owning reference.
- Recursive containment (`FOLDER`, `CLUSTER`, `ITEM_TREE`, `SECTION`,
  `DV_MULTIMEDIA.thumbnail`) is boxed.
- `std::sync::LazyLock` (edition 2024) for statics, not `once_cell`.
- Edition 2024, resolver v3, MSRV 1.96. Code must be `cargo fmt` clean; run
  `cargo clippy` on the crate you touched before considering a file done.

## What not to do

- Do not port JVM plumbing (classloaders, Spring context internals, PF4J
  internals) literally — record `// PORT NOTE:` and design the Rust
  equivalent in the relevant phase, or defer to Stage 2.
- Do not chase compiler errors in Phases P1-P16; capturing intent is the
  goal, not compilation (Phase A of the three-phase gate).
