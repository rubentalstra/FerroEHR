# Reliability & safety hard rules (clinical-grade Rust)

This system is a clinical data repository: silent wrong answers are worse
than loud failures, and every hard rule below is **machine-enforced** — a
violation fails the build or CI, never just review. When you add a rule
here, add its enforcement in the same change; a rule without a failing
check is a wish, not a rule. (Owner directive 2026-07-17; principles from
the official Rust API Guidelines checklist and the Rust Book's
error-handling/overflow chapters.)

## Enforcement tiers (strongest first)

1. **Compile property** — the type system makes the violation
   unrepresentable (newtypes, `#[must_use]`, sealed traits, `forbid`).
2. **Workspace lint at `deny`/`forbid`** — fails every `cargo clippy`,
   local and CI (`Cargo.toml [workspace.lints]`).
3. **Warn + CI `-D warnings`** — fails CI (`clippy::all` + `clippy::pedantic`
   both live here, so every pedantic lint is effectively a hard rule).
4. **CI job** — codegen drift, machete, cargo-audit/deny, changelog guard,
   attribution guard, rustfmt.
5. **Review-enforced** (weakest; minimize): only for properties no tool can
   check — each is explicitly marked below.

## The rules

- **No `unsafe`, ever** (`unsafe_code = "forbid"`, tier 2). No exceptions;
  a need for `unsafe` is a design defect to solve differently.
- **Fail loud, never wrap**: release builds run with
  `overflow-checks = true` — integer overflow panics (→ the catch-panic
  layer's clean 500 + log) instead of silently wrapping into a wrong
  clinical answer. On load-bearing arithmetic (version ordinals,
  nested-set `num`/`num_cap` math, pagination offsets) prefer explicit
  `checked_*`/`saturating_*` with a typed error — a panic is the backstop,
  not the design.
- **No `unwrap`/`expect`/`panic!`/`unimplemented!` in application code**
  (deny-tier lints; `todo!` was already denied). Tests keep the documented
  `#[cfg(test)]`-scoped allows. Recoverable failures return typed errors
  (`thiserror` in libraries, `anyhow` only in binaries) — the Rust Book's
  ch9 split: panic is for states that cannot happen, `Result` for
  everything that can.
- **No panicking indexing on request paths**: prefer `.get(..)`/pattern
  matching over `x[i]` and `&s[a..b]` (`indexing_slicing`/`string_slice`
  lints; `string_slice` can panic on a UTF-8 boundary — clinical text is
  full of multi-byte content).
- **Guards are never silently dropped**: `let _ = lock/tx/handle;` is
  denied (`let_underscore_drop`) — bind guards to named variables that live
  to scope end.
- **No debug/print output from libraries** (`dbg_macro`, `print_stdout`,
  `print_stderr` denied): libraries speak `tracing`; only the binaries and
  `tools/*` write to stdio (per-crate `[lints]` relaxations there, each
  with a comment).
- **Errors are types, not strings, at every boundary that branches**
  (C-GOOD-ERR): a caller that needs to distinguish outcomes gets an enum
  variant, not a substring match. String context belongs in the display
  text, not the discriminant. (Review-enforced at the seam; the
  status-mapping tests pin the wire outcome.)
- **Ids are distinct types where confusion is fatal** (C-NEWTYPE): distinct
  id newtypes (`EhrId`, `VoId`, …) are used throughout, so the type system
  rejects a swapped-argument mistake at compile time (tier 1). Never pass a
  bare `Uuid` where a typed id belongs, and never add a function that takes
  two adjacent bare `Uuid` parameters.
- **Every public item: `Debug`, docs with concrete `# Errors`/`# Panics`**
  (C-DEBUG, C-FAILURE — `missing_debug_implementations` +
  `missing_errors_doc`/`missing_panics_doc` at CI-deny). **PHI caveat**
  (review-enforced): `Debug` impls and `tracing` fields must never carry
  clinical payload content — log identifiers and shapes, not bodies.
- **Visibility is deliberate** (C-STRUCT-PRIVATE): private by default,
  scoped visibility only at real module boundaries, zero re-exports
  (every import names its defining module), `unreachable_pub` watched at
  CI. Struct fields private unless the type IS a plain record.
- **Constructors and conversions follow the standard shapes** (C-CTOR,
  C-CONV/C-CONV-TRAITS, C-GETTER, C-BUILDER): `new`/`with_*` builders,
  `From`/`TryFrom` over ad-hoc `to_x()` where the conversion is total/
  fallible, getters without `get_` prefixes. (Tier 3 via pedantic +
  review.)
- **Blocking never hides in async**: no `std::sync` locks held across
  `.await` (clippy `await_holding_lock`, tier 3), no synchronous I/O on the
  runtime; `spawn_blocking` for the rare CPU-heavy transform.
- **Dependencies are pinned and vetted** (tier 4): workspace-table only,
  `cargo audit`/`deny` green at all times, no new dependency for what the
  pinned set already provides.
- **Only the official comment-annotation forms** (owner ruling 2026-07-17):
  unfinished work is `// TODO:` (or `// TODO(perf):` for a deferred
  performance optimization); a deliberate spec-silent design decision is a
  plain `// NOTE:` carrying the spec citation or the explicit "no openEHR
  spec governs this — our own design/extension" flag; `unsafe` (none
  expected) carries `// SAFETY:`. The bespoke `(port)` vocabulary
  (`// PORT NOTE:`, `TODO(port)`, `PERF(port)`, the "PORT STATUS" trailer)
  is deleted and must not reappear. **Explicit exception to this file's
  rule-needs-a-check pattern:** there is NO CI grep gate for this (the owner
  removed it) — the enforcement story is that the official `// TODO:` form is
  the one the Rust toolchain and the IDE surface, so it is self-reinforcing;
  the `(port)` forms simply have no tooling behind them and die of neglect.
  (Review-enforced.)

## When a lint fights a legitimate case

`#[allow]` is a *documented exception*, never a reflex: scope it to the
smallest item, and the same line carries the reason
(`#[allow(clippy::x)] // <why this case is sound>`). A file-level or
crate-level allow needs the owner's sign-off in the PR.
