# Current phase

**Open items live in [`WORKLIST.md`](WORKLIST.md)** — one row per item,
owner-mandated single tracker. The forward product roadmap is the root
`ROADMAP.md`; the spec oracle is `docs/specs/openehr/` (never memory, never
prior art alone). The historical build record is `docs/PROGRESS.md`.
*(The former blueprint/design-doc layer was deleted 2026-07-16 (owner):
implemented or stale — the specs + this pointer + the worklist are the
navigation surface.)*

## Active work — W-14 full audit + platform rewrite

Branch `claude/w14-audit`. The full endpoint/path audit (155 endpoint ops
probed) and the per-folder platform rewrite it drove are both done; the
audit/rewrite register and tracker files have been pruned (their content is
in git history and `docs/PROGRESS.md`).

State (2026-07-16): the **full per-folder fresh rewrite of the platform is
landed** — 13 folders re-authored from the governing spec sections (fresh
files, old files deleted), the service error module split out, **zero
re-exports across both app crates**, single convergence pass done: the
whole workspace compiles all targets. Fix waves 1–2 (error-track defects +
the write-path redesign) are done; wave 3/4 leftovers and the fleet-found
defects (§4k) are the open fix surface.

**Open before W-14 closes** (see the WORKLIST rows):

1. S6 gates — workspace clippy + nextest, ECC **zero-drift vs 370·335·0**,
   fresh benchmark pair.
2. W-18 — the tracker-ID comment scrub (owner hard rule: no internal task
   IDs in code; only `docs/specs/openehr/` citations).
3. W-19 — stale doc references (code + instruction files citing the deleted
   design docs).

## Then, in order

1. **W-15** — the endpoint → function-chain map (`docs/endpoint-map.md`),
   agent-fleet authored.
2. **W-16 / W-17** — issue #95 (Accept-header format support) and issue #94
   (full example generation).
3. **W-2 / W-3 / W-4 / W-3d / FLAT** — per the worklist.

Every phase still ends with an ECC run showing zero drift; the baseline only
ratchets upward.
