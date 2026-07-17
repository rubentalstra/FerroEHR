# Worklist — the single open-items tracker

Owner-requested (2026-07-12): every open work item lives HERE, one row each,
with status + the governing plan/branch. **This file is the single entry
point** (the former `current-phase.md` pointer was retired 2026-07-17);
close a row by linking the merged PR, and delete the row's plan file in the
PR that implements it. Never track work only in chat.
Fully refreshed 2026-07-16 (post platform-rewrite; stale rows pruned to the
closed table at the bottom).

## Open

| # | Item | Status | Where |
|---|------|--------|-------|
| W-14 | **Full code audit + platform rewrite** (owner 2026-07-16, expanded to every endpoint/path): register complete (155 endpoint ops probed, findings F-1..F-54), the fix waves 1–2 done, the **full per-folder fresh rewrite of the platform executed** (13 folders, fresh files from the spec, zero re-exports, single convergence — workspace compiles all targets). OPEN: S6 gates (workspace clippy + nextest, ECC zero-drift vs 370·335·0, fresh benchmark pair), the remaining §5 wave-3/4 rows (F-8/9/10/21/25/26/27/28/33/39/23/17), the §4k fleet-found defects (F-45..F-53). | in progress | branch `claude/w14-audit` |
| W-15 | **Endpoint → function-chain map** (owner 2026-07-16): one clean document mapping EVERY endpoint through its whole call chain down to the SQL, as the standing navigation + optimization instrument. Tooling researched: no crates.io tool does this for a large async workspace (cargo-call-stack = embedded/nightly; rust-callgraph = research nightlies; cargo-modules = module graphs only) — agent-fleet authored instead. **Delivered 2026-07-16: `docs/endpoint-map.md`** (129 traced sections: EHR 33 ops + QUERY/DEFINITION 19 + DEMOGRAPHIC 50 as 22 shape rows + admin/extensions/public + the 5 background loops; round-trip counts + every N+1 named). The trace itself found 2 new register rows (the demographic wire committal-header drop; the versioned-composition full-body ownership gate). Regenerate after structural changes. | done | `docs/endpoint-map.md` |
| W-17 | **Issue #94 — template example generator emits only the skeleton** (registered as F-44): rewrite the example generator to walk the full WebTemplate tree and synthesize spec-valid values for every field (multi-archetype templates covered); the generated example must pass our own composition validation. Proper scoping first (generator design vs the WebTemplate builder), then fix. | queued | [issue #94](https://github.com/rubentalstra/ehrbase-rs/issues/94) |
| W-18 | **Tracker-ID comment scrub** (owner hard rule 2026-07-16: no F-nn/S-nn/G-nn/W-nn/wave/phase markers anywhere in code — only `docs/specs/openehr/` citations): ~700 occurrences across ~170 files; partial slice landed; the six scrub agents relaunch when the session limit resets (file lists staged in the session scratchpad). | parked (rate limit) | this worklist |
| W-19 | **Stale doc-reference cleanup** after the 2026-07-16 docs purge (blueprint + design docs deleted): source comments still cite deleted `docs/design/*` files (~20 sites: rest api mods, conformance tools, benchmark reports, overview/error.rs); root `CLAUDE.md`, `docs/architecture.md`, `docs/plans/README.md`, and `.claude/rules/{auth,configuration,docs-website}.md` still point at deleted files. Rewrite each reference to the vendored spec path or plain prose. | queued | this worklist |
| W-20 | **Design principles + machine enforcement** (owner 2026-07-17: hard rules in `.claude/` + linter enforcement that FAILS; safety/stability for the clinical datastore): the hard rules + their enforcement now live in `.claude/rules/reliability.md` (release overflow-checks, deny-tier lints, measured ratchets, `unreachable_pub`, id newtypes, doc + `Debug` requirements). **Enforcement landed + merged ([PR #106](https://github.com/rubentalstra/ehrbase-rs/pull/106), 2026-07-17): release overflow-checks, deny-tier lints, unreachable_pub at CI-deny, test files out of `src/`, ratchets measured (270 indexing / 194 arithmetic).** Open: the `EhrId`/`VoId` newtype wave (branch `claude/w20-d8-id-newtypes`, converging). | in progress | `.claude/rules/reliability.md` |
| W-2 | **ECC skip elimination** (owner ruling: a case passes, fails, errors, or is N/A — never "skipped"): wire the remaining native-API-only surfaces or adjudicate N/A with citations; zero skipped outcomes in the final report. | queued | this worklist |
| W-3 | **X1 — honest EHRbase vs ehrbase-rs comparison page**: ECC matrix (ours vs upstream 2.34.0), benchmark ladder + overlay curves, per-case upstream failure triage. Measured numbers only. (Plan file pruned 2026-07-16 — re-author at start.) | queued | this worklist |
| W-4 | **ADL2 — full implementation, spec-exact**: ADL2/cADL2/ODIN source parser, the complete AOM2 semantic-validation catalogue, specialisation flattening, OPT2, template semantics. Oracle: `docs/specs/openehr/AM/docs/{ADL2,AOM2,OPT2}/`. | queued | plan to be authored |
| W-3d | **SM platform chapter-register gap closure**: the remaining G-rows across the platform-service audit (register files pruned 2026-07-16 — re-derive open rows from the SM spec text at execution). | queued | `docs/specs/openehr/SM/` |
| W-21 | **Release v3.1.0** (owner 2026-07-17: minor bump — the Simplified Formats rewrite + governance overhauls): cut prepared on `claude/release-v3.1.0` ([PR #109](https://github.com/rubentalstra/ehrbase-rs/pull/109) — changelog section, workspace 3.0.3→3.1.0, Helm chart + goldens). Blocked on develop CI green: the post-#108 `tenant_isolation` CI failure was diagnosed as a REAL multi-tenancy defect (sqlx `before_acquire` never fires for freshly opened connections → a pool-growth acquire ran as the reserved default tenant) and fixed with a deterministic regression test in [PR #110](https://github.com/rubentalstra/ehrbase-rs/pull/110). v3.1.0 tagged 2026-07-17 (pre-release; notes published from the changelog) but its binary-asset build died on the same crate-consolidation defect already fixed in containers.yml (`-p ehrbase` builds the library, no binary) — all five workflows audited, release.yml was the only remaining hit; **v3.1.1** cut with the release.yml fix so the tarball assets attach (owner: tag without waiting on CI — the pipeline is proven green). | in progress | [PR #109](https://github.com/rubentalstra/ehrbase-rs/pull/109), [PR #110](https://github.com/rubentalstra/ehrbase-rs/pull/110) |

## Closed (recent; full history in `docs/PROGRESS.md` + git)

| # | Item | Closed |
|---|------|--------|
| FLAT | Simplified Formats spec-exact greenfield rewrite (openehr-flat + REST negotiation from the STABLE ITS-REST spec; ECC-SF area added; plan file deleted per the lifecycle) | [PR #108](https://github.com/rubentalstra/ehrbase-rs/pull/108), 2026-07-17 |
| W-16 | Issue #95 — format support via Accept header (subsumed by the FLAT rewrite: strict RFC 9110 negotiation on every relevant endpoint, 406/415 correctness, book docs updated) | [PR #108](https://github.com/rubentalstra/ehrbase-rs/pull/108), 2026-07-17 |
| W-13 | Configuration redesign — one TOML file | merged [PR #96](https://github.com/rubentalstra/ehrbase-rs/pull/96), v3.0.2 |
| W-5 | P20 profile-driven optimization (v3.0.3 measured pair: 631.6 vs 316.1 req/s, 2.0×; all 14 classes lower at p50/p99) | released v3.0.3, 2026-07-16; the two open leftovers (group-commit A/B, knee profiler) fold into W-14's close |
| W-10/W-11/W-12 | Conformance instrument rewrite · hospital-day benchmark · overload shed layer | 2026-07-13/14 |
| W-1/W-3a/W-3b/W-3c/W-3e/W-3f/W-6..W-9 | citation-discipline sweep · Architecture-Overview study + gap closure · SPS redesign · rest-crate rewrite · platform redesign · folders/AQL-subsumption/paths/EHR_ACCESS rows | merged PRs #72/#74/#76 + `claude/w3f-platform-redesign`, 2026-07-12/13 |

## Standing rules picked up along the way (enforced, not tracked)

- CI must be green on develop at all times (new advisories get dated,
  documented ignores only when no fix is reachable).
- Citable references are ONLY the vendored specs + official external docs —
  **never an internal markdown file** (the ADR layer is deleted; internal
  plan/design files are deleted in the PR that implements them), **and never
  internal tracker IDs** (F-nn/S-nn/G-nn/W-nn/phase markers) in code (owner
  hard rules 2026-07-11 + 2026-07-16 + 2026-07-17); scrub on touch.
- No `use X as Y` import renaming; **zero re-exports** (every import names
  its defining module); `urlencoding` for all percent codecs.
- Every phase ends with a zero-drift ECC run; the baseline only ratchets up.
