# Reliability & safety hard rules (clinical-grade Rust)

This system is a clinical data repository: silent wrong answers are worse
than loud failures, and every hard rule below is **machine-enforced** — a
violation fails the build or CI, never just review. When you add a rule
here, add its enforcement in the same change; a rule without a failing
check is a wish, not a rule. (Owner directive 2026-07-17; hardened to the
full official-books baseline per issue #1311, 2026-07-30. Principles from
the Rust API Guidelines checklist, the Rust Book's error-handling/overflow
chapters, the Clippy book, and the Cargo/rustdoc books.)

## Enforcement tiers (strongest first)

1. **Compile property** — the type system makes the violation
   unrepresentable (newtypes, `#[must_use]`, sealed traits, `forbid`,
   `compile_error!` feature guards).
2. **Workspace lint at `deny`/`forbid`** — fails every `cargo clippy`,
   local and CI (`Cargo.toml [workspace.lints]`). NOTE on `forbid`: it
   cannot be relaxed by ANY attribute — `#[allow]` under a forbid is itself
   a compile error (rustc book, lint levels); a crate that ever needed the
   forbidden thing would have to stop inheriting the workspace table, which
   is an owner decision, not an allow.
3. **Warn + CI `-D warnings`** — fails CI (`clippy::all` + `clippy::pedantic`
   both live here, so every pedantic lint is effectively a hard rule —
   including `missing_errors_doc`/`missing_panics_doc`, which are pedantic
   members, not explicit table entries).
4. **CI job** — codegen drift, machete, cargo-deny (which subsumes
   cargo-audit: same RustSec DB, plus yanked/licenses/bans/sources),
   changelog guard,
   attribution guard, rustfmt, the rustdoc job (`cargo doc` with
   `RUSTDOCFLAGS=-D warnings` — the `[workspace.lints.rustdoc]` table is
   inert without a doc run), the MSRV job (`cargo hack check
   --rust-version`), and the scheduled latest-deps workflow.
5. **Review-enforced** (weakest; minimize): only for properties no tool can
   check — each is explicitly marked below.

## The rules

- **No `unsafe`, ever** (`unsafe_code = "forbid"`, tier 2). No exceptions;
  a need for `unsafe` is a design defect to solve differently. The SAFETY
  vocabulary is machine-guarded from both sides:
  `undocumented_unsafe_blocks`/`multiple_unsafe_ops_per_block` (moot but
  free) and `unnecessary_safety_comment`/`unnecessary_safety_doc` deny a
  `// SAFETY:` comment or `# Safety` doc section on SAFE code.
- **Fail loud, never wrap**: release builds run with
  `overflow-checks = true` — integer overflow panics (→ the catch-panic
  layer's clean 500 + log) instead of silently wrapping into a wrong
  clinical answer. On load-bearing arithmetic (version ordinals,
  nested-set `num`/`num_cap` math, pagination offsets) prefer explicit
  `checked_*`/`saturating_*` with a typed error — a panic is the backstop,
  not the design. Numeric-honesty lints back this at deny tier:
  `float_cmp_const`, `lossy_float_literal`, `as_underscore`,
  `fn_to_numeric_cast_any`, `precedence_bits`, `suspicious_xor_used_as_pow`,
  `ambiguous_negative_literals` (+ `integer_division` at warn).
- **The panic strategy is `unwind`, pinned in `[profile.release]`, and must
  never become `abort`** (tier 1 by profile + comment): the clean-500
  contract is `tower-http` CatchPanic = `std::panic::catch_unwind`, which
  "only catches unwinding panics" — and Cargo documents that tests IGNORE
  the `panic` setting, so an abort regression is untestable by construction
  (https://doc.rust-lang.org/cargo/reference/profiles.html#panic). Release
  builds carry `debug = "line-tables-only"` so a production panic names its
  file:line; `strip` stays `"none"` (rustc: stripping symbols makes traces
  "incomprehensible").
- **No `unwrap`/`expect`/`panic!`/`unreachable!`/`unimplemented!` in
  application code** (deny-tier lints; `todo!` was already denied). Tests
  keep the `clippy.toml` `allow-*-in-tests` scoping (unwrap/expect/panic/
  print/indexing — the Book ch11 doctrine: a panicking assertion is exactly
  how a test fails). Recoverable failures return typed errors (`thiserror`
  in libraries, `anyhow` only in binaries) — the Rust Book's ch9 split:
  panic is for states that cannot happen, `Result` for everything that can.
- **The ONE sanctioned escape for a logically-impossible `Err`/`None`**
  (Book ch9: "perfectly acceptable to call `expect` … and document the
  reason you think you'll never have an `Err` variant"): a narrowly-scoped
  `#[expect(clippy::expect_used, reason = "…")]` on the smallest item, whose
  reason states the inspection proving unreachability, plus a
  *should*-phrased message (`.expect("hardcoded IP address should be
  valid")`). Dodging the lint with `unwrap_or_default()`/`unwrap_or_else`
  instead is FORBIDDEN — that converts a loud impossible state into a
  silent wrong clinical value, the exact failure class this file exists to
  prevent.
- **No panicking indexing on request paths** (deny tier:
  `indexing_slicing`, `string_slice`): `.get(..)`/pattern matching over
  `x[i]` and `&s[a..b]` — `string_slice` panics on a UTF-8 boundary and
  clinical text is full of multi-byte content. Tests are scoped out via
  `clippy.toml`; a hot-path site PROVEN in-bounds uses the `#[expect]`
  escape above, never a bare index.
- **Guards are never silently dropped**: `let _ = lock/tx/handle;` is
  denied (`let_underscore_drop` + `let_underscore_lock`) — bind guards to
  named variables that live to scope end. `unused_result_ok` (deny) closes
  the `.ok();` variant — it looks like a check but only silences
  `#[must_use]`. Edition-2024 corollary (review-enforced; from the Edition
  Guide's own deadlock example): **a guard/transaction/borrow produced in an
  `if let` scrutinee is dropped BEFORE the `else` branch runs**
  (https://doc.rust-lang.org/edition-guide/rust-2024/temporary-if-let-scope.html)
  — never rely on it inside `else`; rewrite as `match` when the guard must
  span both arms. Tail-expression temporaries drop at end of block, before
  locals (temporary-tail-expr-scope.html) — same discipline.
- **`Result → Option` inside a chain is a DECISION, and it carries NO
  automated guard** (review-enforced; the honest no-guard record, issue
  #1733). `.filter_map(|x| f(x).ok())`, `.and_then(|x| f(x).ok())` and
  `f(x).ok()?` turn an error into a missing element with no trace — the
  silent-data-loss shape that dropped a client-supplied attestation from a
  commit and a non-decoding version uid from an admin merge list. The rule:
  **a fallible conversion whose failure means "the input is DEFECTIVE"
  propagates a typed error; only a fallible conversion whose failure means
  "this input is legitimately ABSENT / not of this form" may become
  `Option`, and it carries a `// NOTE:` saying so.** Enforcement is honest
  about its own limits: this is the one rule in this file with no failing
  check, because there is no lint (the Clippy book lists none —
  https://rust-lang.github.io/rust-clippy/master/) and a grep gate cannot
  make the distinction the rule turns on. The two shapes are
  *textually identical* — a grammar probe where a parse failure IS the
  answer (`CaptureName::parse(a).ok()?` in the reference-grammar reader)
  reads exactly like a defect swallowed in a codec — and the repo carries
  ~300 `.ok()` sites, the vast majority legitimate, so a pattern gate would
  be ~99% false positives and would be blanket-suppressed within a release.
  A wish honestly labelled beats a check that trains people to ignore it.
- **Determinism is lint-backed** (deny tier: `iter_over_hash_type`):
  HashMap/HashSet iteration order is undefined; anything that feeds
  canonical JSON/XML, SQL generation, or emitted code iterates ordered
  structures (`BTreeMap`/sorted vecs) — byte-determinism is a codegen
  emitter invariant and a wire-parity requirement.
- **No debug/print output from libraries** (`dbg_macro`, `print_stdout`,
  `print_stderr` denied): libraries speak `tracing`; only the binaries and
  `tools/*` write to stdio (crate-root `#![allow]`/`#![expect]` relaxations
  there, each with a reason). `pointer_format` (deny) keeps addresses out
  of logs/Debug output.
- **Banned APIs are compile-time bans, not review notes** (`clippy.toml`
  `disallowed-methods`/`disallowed-types`, owner rulings 2026-07-30):
  `std::time::SystemTime::now` (wall-clock comes from jiff; `Instant` stays
  fine for latency), `std::env::var`/`var_os` (config flows through the
  config tree), `uuid::Uuid::new_v4` (DB keys are uuidv7), the
  `chrono::*` types (jiff is the one time library), and
  `Option::as_slice`/`as_mut_slice` (on an `Option<Vec<T>>` receiver they
  yield `&[Vec<T>]` — a slice of 0-or-1 *vectors*, not `&[T]` — and keep
  compiling after a field's shape flips between `Vec<T>` and
  `Option<Vec<T>>`; spell it `.as_deref().unwrap_or_default()` or match on
  the `Option` — added 2026-08-02, issue #1718; `Vec::as_slice` stays
  fine). A legitimate exception
  site carries a scoped `#[expect(clippy::disallowed_methods, reason)]`.
- **Errors are types, not strings, at every boundary that branches**
  (C-GOOD-ERR): a caller that needs to distinguish outcomes gets an enum
  variant, not a substring match. String context belongs in the display
  text, not the discriminant. (Review-enforced at the seam; the
  status-mapping tests pin the wire outcome.)
- **Ids are distinct types where confusion is fatal** (C-NEWTYPE): distinct
  id newtypes (`EhrId`, `VoId`, …) are used throughout, so the type system
  rejects a swapped-argument mistake at compile time (tier 1). Never pass a
  bare `Uuid` where a typed id belongs, and never add a function that takes
  two adjacent bare `Uuid` parameters. (This is the API Guidelines
  C-VALIDATE preference order made structural: static beats dynamic beats
  assertion beats opt-out.)
- **Every public item: documented, `Debug`, with concrete
  `# Errors`/`# Panics` sections** (C-DOC, C-DEBUG, C-FAILURE):
  `missing_docs` (rustc, tier 3) requires the doc comment;
  `missing_debug_implementations` requires Debug;
  `missing_errors_doc`/`missing_panics_doc` (pedantic members, tier 3)
  require the sections. Generated crates get their docs FROM THE EMITTER
  (BMM `documentation` propagation) — never hand-edit a `// @generated`
  file to document it. Doc quality is lint-backed too: the
  `[workspace.lints.rustdoc]` table (broken/private intra-doc links,
  invalid codeblock attributes, bare URLs at deny) + the CI doc job;
  doctests are copy-paste templates and deny warnings via
  `#![doc(test(attr(deny(warnings))))]` (C-QUESTION-MARK: `?`, never
  unwrap). **PHI caveat** (review-enforced): `Debug` impls and `tracing`
  fields must never carry clinical payload content — log identifiers and
  shapes, not bodies.
- **Visibility is deliberate** (C-STRUCT-PRIVATE): private by default,
  scoped visibility only at real module boundaries, zero re-exports
  (every import names its defining module), `unreachable_pub` watched at
  CI. Struct fields private unless the type IS a plain record.
- **Constructors and conversions follow the standard shapes** (C-CTOR,
  C-CONV/C-CONV-TRAITS, C-GETTER, C-BUILDER): `new`/`with_*` builders,
  `From`/`TryFrom` over ad-hoc `to_x()` where the conversion is total/
  fallible, getters without `get_` prefixes. (Tier 3 via pedantic +
  review; `avoid-breaking-exported-api = false` in `clippy.toml` keeps the
  API-shape lints live — every crate is `publish = false`, so there is no
  external semver surface to protect.)
- **Blocking never hides in async**: no `std::sync` locks held across
  `.await` (clippy `await_holding_lock`, tier 3 — active via `clippy::all`),
  no synchronous I/O on the runtime; `spawn_blocking` for the rare
  CPU-heavy transform.
- **Dependencies are pinned, locked, and vetted** (tier 4): workspace-table
  only, `cargo deny check` green at all times (its advisories check reads
  the same RustSec DB as cargo-audit and adds yanked/licenses/bans/sources,
  so CI runs deny alone), no new dependency for what
  the pinned set already provides. CI builds run `--locked` (the Cargo FAQ's
  determinism rationale: CI fails on new commits, never on registry drift);
  the scheduled `latest-deps` workflow is the official strategy for
  discovering in-range upstream breakage on schedule. Provenance, labeled
  honestly: `cargo audit`/RustSec = the Rust Secure Code WG (the closest
  thing to official); `cargo deny` = third-party (Embark) — our own tooling
  choice; `cargo vet` = Mozilla, not adopted (a standing-commitment decision
  for the tracker if ever). Feature discipline: mutually exclusive features
  carry a `compile_error!` guard (the Cargo book's prescription — the
  console's `hydrate`/`ssr` pair), and `--all-features` lanes exclude the
  guarded crate, testing it per-feature.
- **Comment style is machine-enforced** (owner rulings 2026-07-17,
  2026-08-01, 2026-08-04; the full guide is `comments.md` — RFC 505 +
  RFC 1574): line comments only, unfinished work is `// TODO(#NNNN):` (issue
  reference mandatory), a settled spec-silent decision is `// NOTE:` as a
  citation + one sentence (≤3 lines), plain `//` runs ≤8 lines (essays live
  on the PR/issue), `// SAFETY:` reserved for `unsafe`
  (`unnecessary_safety_comment` guards misuse), and no unsanctioned marker
  vocabulary (the guard's list is the authority — only `TODO`, `NOTE`, and
  `SAFETY` are sanctioned).
  Enforcement (tier 4): `scripts/check-comment-style.sh` — per-edit
  via the `rust_fmt_clippy.sh` hook, per-PR via the `comment-style` CI job
  (diff-scoped until the legacy essay sweep #1870 closes) — plus
  `clippy::too_long_first_doc_paragraph` (tier 3) for the RFC 1574 doc
  summary line. Change-narration and prose deferrals stay review-enforced
  (no tool can judge them).

## Recorded deviations from the API Guidelines (deliberate, owner-adjudicated)

- **C-SERDE — satisfied via emitted MANUAL impls** (#1702): the spec types implement
  `serde::Serialize`/`Deserialize` through explicit generated code
  (per-crate `json_serde.rs` from `emit-json`), never derives and never
  serde attributes — the manual form is what lets the strict reader,
  `_type` dispatch, and validated-constructor parsing live in auditable
  code (root `CLAUDE.md` §Code generation). The wire contract stays pinned
  by the canonical-output gates.
- **C-PERMISSIVE — MIT for the project's own code** (owner decision
  2026-07-31): the project's own code is MIT-licensed; vendored openEHR
  machine-readable
  artifacts and CKM-derived data keep their upstream Apache-2.0 terms
  (`LICENSE-APACHE-2.0`). Dependencies stay license-gated by `deny.toml`.
- **C-STABLE — latent, neutralized by `publish = false`**: the `openehr-*`
  crates carry ≥1.0 spec versions while exposing pre-1.0 public deps
  (`jiff 0.2`, `sqlx 0.9`, `chumsky 0.13`, `leptos 0.8`). Harmless while
  nothing publishes; a HARD BLOCKER for any future standalone publication —
  re-adjudicate then.

## When a lint fights a legitimate case

**`#[expect(lint, reason = "…")]` is the default suppression** — it
self-reports the moment the expectation stops being fulfilled
(`unfulfilled_lint_expectations`), so stale suppressions cannot accumulate.
`#[allow(lint, reason = "…")]` is reserved for cases where the lint fires
only in SOME configurations (cfg/feature-dependent code, macro expansions) —
an `#[expect]` there would itself warn in the quiet configuration. Both
forms MUST carry `reason = "…"` (`allow_attributes_without_reason` = deny;
`allow_attributes` = warn steers toward `#[expect]`). Scope every
suppression to the smallest item. A file-level or crate-level suppression
needs the owner's sign-off in the PR.
