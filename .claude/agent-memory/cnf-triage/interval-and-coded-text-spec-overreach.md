---
name: interval-and-coded-text-spec-overreach
description: Two confirmed app-side validation overreaches — DV_INTERVAL bound-presence + DV_CODED_TEXT value==rubric
metadata:
  type: project
---

Two validation additions (build d09d9604c, 2026-07-23) enforce invariants the
CLOSED openEHR invariant sets do not define — both are APPLICATION defects
(spec overreach), both reproduced on the wire. The vendored spec is the oracle;
"closing an acceptance hole" is not license to invent an invariant.

1. **DV_INTERVAL bound-presence** (commit e9d2ec11f) —
   `crates/openehr-rm/src/data_types/quantity/dv_interval_impl.rs`
   `DvInterval::invariants` added `!lower_unbounded && lower.is_none()` (+ upper)
   → 422 "lower/upper bound absent while *_unbounded is false on type
   DV_INTERVAL". BASE `...foundation_types.interval.adoc`: `lower/upper` are
   `0..1`; §Invariants is a closed 4-set (Lower/Upper_included_valid,
   Limits_consistent, Limits_comparable) — NONE require a bound when the flag is
   false. The repo's own register AMB-43 dispositions this ACCEPTED. The commit
   also edited COUNT+QUANTITY validate_open case cores to expect `rejected`
   (forbidden expectation-to-match-behaviour edit; masked green). Fix: revert
   both blocks + both case-core edits.

2. **DV_CODED_TEXT value==rubric** (commit e56245034, fed by WT-builder
   43f9c5601) — `crates/openehr-flat/src/validation/leaf.rs`
   `check_coded_text_rubric` → 422 "coded value 'X': value 'v' is not the bound
   rubric". RM `...dv_text.adoc` §Invariants: the ONLY value invariant is
   `Valid_value: not value.is_empty`; DV_CODED_TEXT adds only `defining_code
   1..1`. "value must be the rubric" is prose and names the TERMINOLOGY SERVICE
   (authoring language) as authority, NOT the template. CNF
   CONT-DV_CODED_TEXT-validate_local_codes checks code_string+terminology_id vs
   C_CODE_PHRASE, never value. When the OPT binds no term_definition rubric for
   an external code, the WT fallback uses the code string as the label, so the
   check degenerates to value==code. Fix: remove the check + its call site.

**How to apply:** these caused all 8 red rows in the 2026-07-23 run
(DV_CODED_TEXT-validate_local_codes + 6 DV_INTERVAL_*-validate_open +
SF-MAP-interval_reference_range). ORDINAL/SCALE hit BOTH (row1 = bound-presence,
rows4/5 = coded symbol value=code). Verify the two code paths still exist before
re-attributing (an implementer may have reverted). Reproduction recipe in
[[sut-reproduction-setup]]; the credential note there is correct
(ferroehr-admin:ferroehr works — POST /ehr → 201).
