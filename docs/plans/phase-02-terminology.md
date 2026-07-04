# Phase 02 — Terminology (TERM) bundle + service API

- Status: **done**
- Build order: complete (spec foundation)
- Decisions: ADR-004 (TERM data classes generated; bundle/assets hand-written)

## Outcome

`openehr-term` holds the TERM 3.1.0 data classes (**generated** from BMM) plus
the hand-written terminology **bundle + XML assets + access logic** (which BMM
cannot produce — it only has the ~6 service-interface classes). The vendored
terminology XML lives in `assets/` (data, outside `src/`, survives regen). The
SPECPR-51 `id=532` dual-rubric quirk is preserved and pinned by a regression
test.

A thin XML→model loader is added when composition validation (P15) needs it.

## Verification

`openehr-term` builds clean; the `id=532` regression test passes.
