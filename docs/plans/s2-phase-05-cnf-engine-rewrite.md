# Phase S2-05 — CNF conformance engine full redesign (multi-source, spec-exhaustive)

- Status: in-progress (design)
- Started: 2026-07-08   Owner: —
- Branch: `claude/cnf-hardening` (owner instruction: stay on this branch)
- Consumes: the vendored spec corpus at `docs/specs/openehr/` (the oracle —
  CNF schedule + Robot suite + ITS-REST OAS + QUERY/AQL + RM/BASE + ITS-JSON),
  the existing `crates/ehrbase-conformance` (v1, PR #27) as prior art.
- Compile required: yes — compiling, clippy-clean, tested increments.

## Why this phase (owner directive, 2026-07-08)

The v1 framework enforces the 322 identified schedule cases — but the schedule
alone is a thin instrument: master11 (QUERY) is TBD stubs upstream, master17.5
is empty, data-set variants are folded into single cases, and the Robot suite's
hundreds of concrete executable cases + golden expected results are used only
as fixtures. The owner wants a **complete redesign**: an official-grade testing
engine whose case base is derived *exhaustively* from the vendored official
specs — thousands of executable cases, every one citing its spec file+section —
usable as the real acceptance instrument for conformance claims.

Note (verified 2026-07-07, re-verified this phase): the vendored CNF **is**
identical to upstream HEAD (`specifications-CNF` @ `33251d2a`; upstream dormant
since 2024-08). "Up to date with the latest official specs" therefore means
**deriving the missing depth from the other pinned official artifacts**, not
re-vendoring.

## Design thesis (v3 — supersedes the registry-only model of
`docs/design/conformance-framework.md` §4 where they conflict)

Keep the **schedule as the normative spine** (profile/certificate reporting
keys on official case ids), but replace "one hand-registered case per schedule
heading" with a **multi-source case engine**:

1. **Schedule source** — every masterNN case, with every normative data-set /
   flow variant expanded to a distinct executable case (not "16/16 data sets
   inside one case").
2. **Robot source** — full transcription of the vendored Robot suite's
   concrete cases (native Rust, no Python), with its expected-result goldens;
   provenance-tagged, mapped back to schedule ids where tags allow.
3. **ITS-REST source** — an endpoint × documented-status-code matrix generated
   from the vendored OpenAPI (the same corpus `emit-rest` consumes): every
   operation, every documented response, both formats.
4. **AQL source** — the fixture corpus (valid groups A–D + invalid + golden
   result sets, empty + loaded DB) as first-class cases with golden diffing,
   plus AQL 1.1 spec-feature cases for constructs the corpus misses.
5. **Content source** — exhaustive truth tables for master15–17.x, including
   engine-defined tables for the chapters upstream left empty (17.5),
   grounded in the RM data_types spec text.
6. **Runner-defined** — SIGN-* (unchanged from v1 §4.6) and any capability
   with zero upstream material.

Per-source **coverage gates** (the house pattern): each source has a parser
over the vendored artifact and a guard asserting every extracted id is
implemented or explicitly excluded with a reason enum. Failing cases are
findings, never exclusions (v1 §4.5 discipline unchanged).

## Tasks

- [ ] Recon: exhaustive inventories (schedule chapters + variants, Robot suite
      counts + goldens, current-crate map, upstream freshness) — in flight.
- [ ] Author the v3 design (`docs/design/conformance-framework.md` rewrite):
      multi-source case model, id scheme, coverage gates, report schema.
- [ ] Engine core rewrite: case model + source registries + per-source
      coverage guards.
- [ ] Schedule source: variant-expanded transcription (all masterNN).
- [ ] Robot source: transcribe suites service-by-service with goldens.
- [ ] ITS-REST source: OAS-derived endpoint/status matrix generator + cases.
- [ ] AQL source: corpus cases + golden diffing + spec-feature cases.
- [ ] Content source: full truth tables incl. 17.5 fill.
- [ ] Reports: results.json / RESULTS.md / CONFORMANCE_STATEMENT.md updated to
      per-source provenance; badge.
- [ ] CI: smoke tier + full tier updated; `scripts/conformance.sh` contract
      preserved.

## Exit criteria

- [ ] Every vendored-artifact-extractable case id is implemented or
      reason-excluded, enforced by per-source guards (build-breaking).
- [ ] Total executable case count ≥ 4× the v1 322, each case carrying spec
      file + section citation.
- [ ] Full run against the compose stack produces the regenerated
      `docs/conformance/` artifact set; failures tracked as findings.

## Decisions made this phase

- (pending v3 design doc)

## Handoff for next session

Recon agents dispatched (schedule inventory, Robot inventory, current-crate
map, upstream freshness). Next: fold their results into the v3 design doc,
then implement engine core.
