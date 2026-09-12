---
name: sandbox-vetdf-remaining-two
description: Where the FerroTERM/VETDF sandbox work stopped on 2026-09-12 (paused by the owner): v4.2.4 tagged, seed at 231/91, two archetypes still refused, issue #3315
metadata:
  type: project
---

State when the owner paused the session on 2026-09-12: v4.2.2, v4.2.3 and v4.2.4 are tagged and published; the sandbox runs 4.2.4 with FerroTERM 0.1.3 beside the CDR (`.env` has `FERROTERM_INDEX=/data/index/int:/data/index/loinc`, indexes under `/opt/ferroehr-sandbox/ferroterm-index/{int,loinc}`, SNOMED CT International 20260901 + LOINC 2.83, both SHA-256 verified). The v4.2.4 reseed leg failed: the ADL 2 library loaded 231 / 91 against the pinned 233 / 89, so the sandbox is half-seeded (no EHRs/compositions/parties/queries). Issue #3315 (P0, milestone v4.2.5) carries the two remaining refusals.

**Why:** the owner asked to pause mid-diagnosis; the next step is the library replay against the sandbox (`POST …/definition/template/adl2` per `.adls`, collect the 422 bodies, diff with the earlier 95-list in the scratchpad) to name the two archetypes and their exact VETDF terms.

**How to apply:** resume at #3315: replay, adjudicate each term (retired SNOMED concept vs LOINC answer-list code the index lacks vs a real archetype defect), fix or re-pin with the adjudication recorded, then `gh run rerun --failed` on the release run (`sandbox reseed` + `announce`) or dispatch `sandbox-reseed.yml`, and verify the seed passes. Related: [[compliance-corpus-direction]], the v4.3.0 items stay untouched until the owner says so ([[next-milestone-in-fresh-session]]).
