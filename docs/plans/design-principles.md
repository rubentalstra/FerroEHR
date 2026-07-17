# Design principles + machine enforcement (owner directive 2026-07-17)

Owner: "we need to be safe and also optimized because our system is a database
for the clinical world — stability and reliability are very important", and:
hard rules live in `.claude/` AND are **enforced by the linter so violations
fail as errors** — never trusted to be followed.

Sources (read 2026-07-17): the official [Rust API Guidelines
checklist](https://rust-lang.github.io/api-guidelines/checklist.html)
(C-CASE … C-PERMISSIVE), the Rust Book's reliability chapters (error
handling: panic vs `Result`; `unsafe`), and the clippy lint tiers
(`pedantic`, `restriction`).

## The enforcement principle

A rule counts as HARD only when one of these fails the build on violation:

1. a `[workspace.lints]` entry at `deny`/`forbid` (fails `cargo clippy`
   locally AND in CI),
2. a `pedantic`/`all` warn + CI's `-D warnings` (fails CI; local warn),
3. a CI job (drift check, machete, attribution guard, changelog guard),
4. a compile property (the type system — newtypes, `#[must_use]`, sealed
   traits).

Prose-only rules in `.claude/rules/` describe intent; each names its
enforcement or is explicitly marked "review-enforced" (the weakest tier,
minimized).

## Where the workspace already stands (verified 2026-07-17)

- `clippy::all` + `clippy::pedantic` = warn, CI runs `-D warnings` → both
  tiers already FAIL CI (tier 2). Includes every `cast_*` truncation lint.
- `unsafe_code = "forbid"` (tier 1) — the strongest guarantee in the file.
- `dead_code = "deny"`, `clippy::todo = "deny"` (tier 1).
- `unwrap_used`/`expect_used` = warn → CI-deny (tier 2).
- CI jobs: codegen drift, machete, audit/deny, changelog, attribution,
  rustfmt (tier 3).

## The gap register

| # | Finding | Guideline / source | Enforcement fix | Status |
|---|---|---|---|---|
| D-1 | **Release builds run with `overflow-checks = false`** (no `[profile.release]`) — integer overflow silently WRAPS in production. The clinical "silent wrong answer" class. | Rust Book §integer overflow: release wrapping is explicitly a footgun to opt out of; safety-critical practice is checks on | `[profile.release] overflow-checks = true` (+ the same for the bench profile so measurements carry the production cost honestly) | ☐ |
| D-2 | `unwrap_used`/`expect_used` at warn — locally ignorable; the hard "no unwrap outside tests" rule is CI-only | C-FAILURE / Book ch9 (panic is for unrecoverable states only) | raise both to `deny` in `[workspace.lints]`; keep the documented `#[allow]` idiom in `#[cfg(test)]` | ☐ |
| D-3 | Panicking indexing/slicing (`x[i]`, `&s[a..b]`) is un-linted (restriction tier off) — an out-of-bounds is a 500 on a clinical endpoint | C-VALIDATE; Book ch9 | measure `clippy::indexing_slicing` + `clippy::string_slice` violation counts; deny if the count is small, else warn + ratchet row | ☐ (measure after the knee run frees the host) |
| D-4 | Arithmetic side effects (`+`/`-` on untyped ints) un-linted; with D-1 they become panics instead of wraps — still prefer explicit `checked_`/`saturating_` on hot invariants (version ordinals, nested-set nums) | Book §overflow; C-VALIDATE | measure `clippy::arithmetic_side_effects`; likely too wide to deny — apply targeted `checked_*` on the versioning/nested-set math + keep the lint at warn on the two storage/versioning modules if scoping is practical | ☐ |
| D-5 | `panic!`/`unimplemented!`/`dbg!`/`print{ln}!` un-linted in lib code (todo! already denied) | C-FAILURE | deny `clippy::panic`, `clippy::unimplemented`, `clippy::dbg_macro`, `clippy::print_stdout`, `clippy::print_stderr` workspace-wide; per-crate `[lints]` relaxation ONLY for `tools/*` binaries + the server binary (stdout is their UI) | ☐ |
| D-6 | `let _ = guard;` can silently drop a lock/tx guard | C-DTOR-FAIL adjacent | deny `let_underscore_drop` (rust) + `clippy::let_underscore_must_use` | ☐ |
| D-7 | `unreachable_pub` off — items `pub` beyond their real reach defeat the zero-re-export/visibility discipline the rewrite just established | C-STRUCT-PRIVATE / C-NEWTYPE-HIDE | `unreachable_pub = "warn"` (tier 2 via CI) — measure first; the fleet set deliberate visibility so the count should be near zero | ☐ |
| D-8 | **Id-type confusion is compilable**: `ehr_id: Uuid`, `vo_id: Uuid`, `contribution_id: Uuid`, `audit_id: Uuid` — swapping two arguments type-checks. The highest-value *type-system* enforcement available (C-NEWTYPE "newtypes provide static distinctions") | C-NEWTYPE, C-CUSTOM-TYPE | design pass: `EhrId(Uuid)` / `VoId(Uuid)` newtypes through the platform seams (storage fn signatures first — they are the mixup site); measure the churn, land as its own wave with zero-drift gates | ☐ registered (big) |
| D-9 | `# Panics` doc sections absent where indexing/expect-adjacent code remains | C-FAILURE | covered by `clippy::missing_panics_doc` (pedantic — already CI-deny); verify no blanket allows exist | ☐ verify |
| D-10 | Public types' `Debug` coverage (C-DEBUG) — `missing_debug_implementations` is warn (CI-deny). PHI note: `Debug` on payload-bearing types must not leak clinical content into logs — the tracing layer never logs bodies; keep it that way (review-enforced, noted in the rule file) | C-DEBUG + our own PHI posture | keep; add the PHI-in-Debug caveat to the rule file | ☐ |

## Execution order

1. Write `.claude/rules/reliability.md` (the hard rules, each naming its
   enforcement tier) + slim `rust-style.md` overlaps. [no build needed]
2. Land D-1 (profile) + the cheap denies (D-2, D-5, D-6) + D-7 warn; run the
   full workspace clippy to enumerate fallout; fix all of it. [needs the host]
3. Measure D-3/D-4 counts; deny or ratchet per the register.
4. D-8 newtype wave: its own branch + register row, gated like every wave
   (workspace green + ECC zero drift).
5. CLAUDE.md points at the rule file; WORKLIST row tracks the remainder.
