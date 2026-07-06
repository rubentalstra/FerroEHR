---
paths: ["crates/**/*.rs"]
---

# Rust style — idiomatic application code (ADR-006)

Applies to hand-written `.rs`: the application (`ehrbase`,
`ehrbase-rest`, `ehrbase-compat`), the hand-written spec crates (`openehr-its`,
`openehr-flat`, `openehr-term`, the tooling crates), and `*_impl.rs` behaviour
files. **The application is modern idiomatic Rust of our own design, built on the
generated `openehr-*` crates** (ADR-006/008). The openEHR specifications are
the authority; EHRbase and other CDRs are prior art only.

**Generated files are off-limits.** The openEHR spec/ITS crates (`openehr-base`,
`openehr-rm`, `openehr-am`, and the generated code in `openehr-its`) are produced
by `openehr-codegen`; every `// @generated` file **must never be hand-edited** —
change the emitter and regenerate (`cargo run -p openehr-codegen -- emit`/
`emit-xml`/`emit-rest`).

## Build idiomatic, compiling, tested code

- **Consume the generated crates directly** as the domain model (`openehr-rm`,
  `openehr-am`, `openehr-term`, `openehr-query`, `openehr-its`). Never re-model
  the RM or re-serialize.
- **Use proper crates; don't hand-roll** what the ecosystem provides — `axum`/
  `tower-http`, `sqlx`+`sea-query`, `oauth2`/`openidconnect`/`jsonwebtoken`/
  `argon2`, `utoipa`, `moka`, `jiff`, `uuid`. Add deps only from
  `Cargo.toml [workspace.dependencies]` (`dep.workspace = true`).
- **Every crate you touch compiles + is clippy-clean + tested before you move
  on** (ADR-006 retired the "phases need not compile" gate for the app layer).
- **Read the vendored spec first for any spec-facing behaviour** — the
  normative text + CNF schedule live at `docs/specs/openehr/` (use
  `/spec-lookup`; full rule in `spec-adherence.md`).
- **The specs are the authority; design the bespoke logic ourselves** (AQL
  engine, versioning, validation, node codec — ADR-008), verified by the CNF
  conformance suite + corpus tests. Consult prior art (upstream EHRbase, other
  CDRs) when useful; never port it blindly.

## Annotation vocabulary (grep-able, where relevant)

`// TODO(port):` unfinished work · `// PERF(port):` optimize after conformance ·
`// PORT NOTE:` a deliberate spec-gap or design decision (with the reason)
· `// SAFETY:` justification for any `unsafe` (expect almost none — this is a
web service). No PORT STATUS trailer on application code (that was the retired
1:1-port convention).

## Type and error conventions

- `thiserror` error enums in library crates; `anyhow` only in the `ehrbase`
  binary. No `unwrap`/`expect` outside `#[cfg(test)]`; use `todo!("<why>")`
  where a dependency isn't ready, but keep the crate compiling.
- Closed openEHR subtype sets are already Rust `enum`s in `openehr-rm`; consume
  them. Trait objects only for genuinely open, runtime polymorphism.
- Back-references use `Weak<..>` or an index, never an owning reference; recursive
  containment is boxed (the generated crates already do this).
- `std::sync::LazyLock` (edition 2024) for statics, not `once_cell`.
- Edition 2024, resolver v3, MSRV 1.96. `cargo fmt` clean; run `cargo clippy` on
  the crate you touched before considering it done.

## What not to do

- Do not port JVM plumbing (classloaders, Spring context internals, PF4J) —
  design the idiomatic Rust equivalent (tower middleware, axum state, a Rust
  plugin model) or defer to Stage 2 with a `// PORT NOTE:`.
- Do not hand-edit generated code; do not re-model what `openehr-*` provides.
