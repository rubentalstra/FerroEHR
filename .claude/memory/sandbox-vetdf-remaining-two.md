---
name: sandbox-vetdf-remaining-two
description: Resolved 2026-09-12 — the sandbox seed passes at 231/91 beside FerroTERM (v4.2.5); the two remaining VETDF refusals are 2013 CKM source defects, pinned, never code
metadata:
  type: project
---

Resolved state as of 2026-09-12 (v4.2.5 tagged and published, release run 34711567715 fully green): the sandbox runs 4.2.5 with FerroTERM 0.1.3 beside the CDR serving SNOMED CT International 20260901 + LOINC 2.83, and the reseed leg seeds the whole demo dataset (8 EHRs, 183 compositions, 8 directories, 11 parties, 5 stored queries). The ADL 2 archetype-library pin is 231 accepted / 91 refused for this terminology posture: 89 AOM2 refusals plus two VETDF refusals against the served LOINC (`apgar` binds `LA6713`…`LA6727` without check digits, `braden_scale` binds the mistyped `LA9605-4`). Both are defects of the vendored 2013 CKM sources, kept verbatim; a CDR with no LOINC-serving server accepts them (233 / 89). Issues #3311 and #3315 are closed with every criterion ticked.

**Why:** the pin depends on which terminologies the deployment serves, so a seed count that moves after a terminology change is expected, not a regression, until the refusals are read.

**How to apply:** if the sandbox seed count moves again, read the 422 bodies first (replay `POST …/definition/template/adl2` per `.adls`) and adjudicate each VETDF term before touching the manifest pin; a served-edition change (SNOMED or LOINC release) re-opens exactly this question. The v4.3.0 items stay untouched until the owner starts them in a fresh session ([[next-milestone-in-fresh-session]]).
