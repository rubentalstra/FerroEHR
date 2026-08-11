---
name: rm-data-types-impl-templates
description: Confirmed defect patterns in the openehr-rm data_types spec-behaviour templates (DV_ORDERED family) — exact-vs-float split, is_integral double reading, invented DV_URI invariant, REFERENCE_RANGE/DV_INTERVAL absent-bound disagreement
metadata:
  type: feedback
---

Verified 2026-08-11 against `docs/specs/openehr/RM/docs/UML/classes/*` while
auditing `tools/openehr-codegen/templates/openehr-rm/data_types/` (issue #2253).
Re-verify before acting; these were first-hand at that date.

- **Exact-vs-float split inside one class is the recurring silent-wrong-answer
  shape.** `DvProportion::is_equal` cross-multiplies in `rust_decimal` while the
  `ordered_limit!` key and `magnitude()` divide in `f64`
  (`dv_ordered_impl.rs:410,487`) — proven: `1.0/3.0 < 0.1/0.3` is `true` in
  IEEE-754 while `is_equal` says equal. BASE `Ordered` §Functions makes
  `less_than` + `=` jointly define `greater_than`, so the pair must agree.
  When auditing any DV_ORDERED subtype, diff the ordering key's arithmetic
  against the class's own equality function.
- **`DV_PROPORTION.is_integral` has TWO live readings in-tree.** §Functions says
  "True if the numerator and denominator values are integers, i.e. if precision
  is 0". `DvProportion::is_integral()` (dv_ordered_impl.rs:499) uses the FLOOR
  test; the generated `dv_proportion_core` uses "precision is 0" for
  Fraction_validity and the floor test for Precision_validity, under a NOTE that
  contradicts itself. `(10, 500, pk_fraction, precision=1)` → function true,
  validator rejects.
- **`Scheme_valid` is emitted on DV_URI, which does not declare it** (that is
  DV_EHR_URI's invariant). The behaviour is groundable in the DV_URI
  §Description RFC-3986 clause (AMB-209 already cites it), but the in-code
  justification calls the CNF schedule "the conformance oracle", which
  `spec-adherence.md` forbids outright. `InvariantViolation.message` reaches the
  client via `ServiceError::ValidationFailed`, so a fabricated invariant NAME is
  wire-visible.
- **REFERENCE_RANGE `Range_is_simple` contradicts AMB-43.** `limit_ok`
  (reference_range_impl.rs:19) requires `unbounded || limit.is_some()`, so an
  absent bound with `*_unbounded = false` — which `DvInterval` accepts by name,
  per AMB-43 — is rejected one level up.
- **`//!` module docs escape `comment-style.sh` entirely** (the NOTE ≤3-line and
  8-line run budgets are gated on `is_line`, i.e. plain `//` only). Multi-line
  `NOTE:` adjudications inside `//!` are review-enforced only.
- Completeness is machine-guarded by
  `tools/openehr-codegen/tests/it/unrealized_bmm_functions.txt` + the
  `unrealized_bmm_functions_match_the_ratchet` test — it checks NAME existence,
  never behaviour, so question 2 is cheap but question 1 still needs the
  ancestor-class table read (DV_ABSOLUTE_QUANTITY/DV_AMOUNT state accuracy rules
  the subtype tables omit).
