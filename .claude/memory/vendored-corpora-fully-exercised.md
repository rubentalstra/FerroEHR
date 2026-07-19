---
name: vendored-corpora-fully-exercised
description: Owner hard rule — any vendored test corpus must be 100% exercised with expected outcomes + a coverage gate; no dead fixtures
metadata: 
  node_type: memory
  type: feedback
  originSessionId: 9491391b-2993-4fd4-a58b-26e1d0724da8
  modified: 2026-07-19T09:32:09.609Z
---

Owner ruling (2026-07-19, during the ADL2 row): when a test corpus is
vendored (e.g. `crates/openehr-adl/tests/corpus/` — the openEHR
ADL2-reference library), implementing against ALL of it is a HARD
requirement — every file exercised with an asserted expected outcome
(rule-code filenames must raise exactly that code), enforced by a
coverage gate test that fails on any unclaimed file. Skips only via a
documented adjudication entry with spec citation.

**Why:** the corpora are the use-case library — archie's validation depth
comes from exercising its whole corpus; partial coverage silently narrows
conformance.

**How to apply:** when vendoring any fixture corpus, add the coverage
gate + per-category harnesses in the same phase as the first consumer;
never land a corpus as passive files. See [[ecc-own-conformance-framework]]
for the ECC-side analogue.
