---
paths: ["crates/**/*.rs", "app/**/*.rs", "tools/**/*.rs"]
---

# Rust style — idiomatic application code

Applies to hand-written `.rs`: the application (`ferroehr`, `ferroehr-rest`,
`ferroehr-server`, `ferroehr-admin-ui`), the tools (`cnf-runner`,
`testkit`, `openehr-codegen`), the hand-written spec
crates (`openehr-its`, `openehr-term`, `openehr-query`, the
tooling crates), and `*_impl.rs` behaviour files. **The application is modern idiomatic Rust of our own design, built on the
generated `openehr-*` crates**. The openEHR specifications are
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
  on.**
- **Read the vendored spec first for any spec-facing behaviour** — the
  normative text + CNF schedule live at `docs/specs/openehr/` (use
  `/spec-lookup`; full rule in `spec-adherence.md`).
- **The specs are the authority; design the bespoke logic ourselves** (AQL
  engine, versioning, validation, node codec), verified by the CNF
  conformance suite + corpus tests. Consult prior art (upstream EHRbase, other
  CDRs) when useful; never port it blindly.

## Comments & documentation

The comment/doc-comment discipline lives in **`comments.md`** (RFC 505 +
RFC 1574): line comments only, `// TODO(#NNNN):` / `// NOTE:` / `// SAFETY:`
as the only markers, NOTE = citation + one sentence (≤3 lines), `//` runs
≤8 lines, doc-comment summary-line + section conventions. Enforced by
`scripts/check-comment-style.sh` (hook + CI) and
`clippy::too_long_first_doc_paragraph`.

## Type and error conventions

- `thiserror` error enums in library crates; `anyhow` only in the `ferroehr`
  binary. No `unwrap`/`expect` outside `#[cfg(test)]`; `todo!()` is denied
  workspace-wide (owner rule 2026-07-12) — an unready dependency gets a typed
  error or real code, never a panic placeholder.
- Closed openEHR subtype sets are already Rust `enum`s in `openehr-rm`; consume
  them. Trait objects only for genuinely open, runtime polymorphism.
- Back-references use `Weak<..>` or an index, never an owning reference; recursive
  containment is boxed (the generated crates already do this).
- `std::sync::LazyLock` (edition 2024) for statics, not `once_cell`.
- **No `use X as Y` import renaming (owner hard rule, 2026-07-11).** Import
  types under their direct names. An alias papers over a naming problem —
  if the name is bad, FIX THE NAME at its definition; if two imports
  genuinely collide, qualify one at the use site (full path) instead of
  renaming. Only alias in highly exceptional cases where no other solution
  exists, with a comment saying why. (Trait imports as `use Trait as _;`
  are not renames and are fine.)
- Edition 2024, resolver v3, MSRV 1.96. `cargo fmt` clean; run `cargo clippy` on
  the crate you touched before considering it done.
- **Suppressions are `#[expect(lint, reason = "…")]`**, scoped to the smallest
  item; `#[allow(lint, reason = "…")]` only for cfg/feature-conditional fire
  (full policy: reliability.md §When a lint fights a legitimate case —
  `allow_attributes_without_reason` is deny). The one sanctioned
  logically-impossible-`Err` escape is also there (Book ch9 shape:
  should-phrased message + inspection-proving reason).

## Documentation (missing_docs is enforced workspace-wide)

- Every public item carries a doc comment (rustc `missing_docs`; generated
  crates get theirs from the emitter — never hand-edit `// @generated`).
  Shape, sections, and summary-line rules: `comments.md` (RFC 1574).
- Intra-doc links (`[`Type`]`) resolve in the scope of the module where the
  item is DEFINED — which under the zero-re-exports rule is also where
  readers import it from. `rustdoc::broken_intra_doc_links` is deny; the CI
  doc job is the gate. Literal square brackets in prose are escaped `\[…\]`.
- `#[doc(alias = "EHR_STATUS")]`-style aliases on spec-named types are
  encouraged — rustdoc search then finds the Rust type from the openEHR
  spelling.

## Edition-2024 standing guidance (behaviour that compiles fine and differs)

- **`if let` scrutinee temporaries drop before `else`** — the guard rule
  lives in reliability.md; rewrite as `match` when a guard must span arms.
- **Never-type fallback**: `f()?;` on a fn generic over the `Ok` type can
  now infer `!` — annotate the turbofish/binding type at such call sites
  instead of leaning on inference
  (https://doc.rust-lang.org/edition-guide/rust-2024/never-type-fallback.html).
- **RPIT captures every in-scope lifetime by default** — when a returned
  `impl Trait` must NOT capture one, say so with precise capturing
  (`use<…>`); the old `Captures<..>` trick is obsolete
  (https://doc.rust-lang.org/edition-guide/rust-2024/rpit-lifetime-capture.html).
- **`Future`/`IntoFuture` are in the prelude** — a collision with a local
  trait method is resolved with fully-qualified syntax
  (`<T as MyTrait>::poll(…)`), never an import rename (the no-alias rule
  stands).
- The `unsafe_*` 2024 items are all moot under `unsafe_code = "forbid"`;
  `static_mut_refs` is compiler-enforced and the `LazyLock` convention
  already complies. Do not re-litigate them.

## What not to do

- Do not port JVM plumbing (classloaders, Spring context internals, PF4J) —
  design the idiomatic Rust equivalent (tower middleware, axum state, a Rust
  plugin model) or register it as its own tracker issue with a `// TODO(#NNNN):`.
- Do not hand-edit generated code; do not re-model what `openehr-*` provides.
