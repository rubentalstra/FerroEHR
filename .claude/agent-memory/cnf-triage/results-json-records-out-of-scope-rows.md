---
name: results-json-records-out-of-scope-rows
description: results.json status counts include cases the verdict pipeline never selects (unclaimed capability / applies version) — read headline counts against verdicts.coverage.selected
metadata:
  type: project
---

`run.rs` drives EVERY active case; `verdict.rs::select` filters afterwards on
(a) `intersects(case.capabilities, statement.claims.capabilities)` and
(b) `applies_satisfied(case, statement.spec_versions)`. So a case the party
never claimed still lands in `results.json` as `failed`.

Measured on the 2026-07-28 ehrbase-java record: 752 outcomes, 444 failed + 98
errored = 542 red — but only **227 of those 542 are in verdict scope**
(151 excluded as unclaimed-capability, 164 as version-excluded). `verdicts.json`
`coverage.selected` was 313, `driven` 291.

Corollary for a public comparison record: the headline "444 failed" is not a
conformance claim about the product; the defensible numbers are the in-scope
subset and the per-capability verdicts.

**Also confirmed:** `binding.applies` (`model/binding.rs:548`) is deserialized
and read by NOTHING — only `case.applies` is enforced. A binding declaring
`applies: { its_rest: ">=1.0.0" }` while carrying a 1.1.0-only matcher is
silently misleading metadata. See [[etag-weak-indicator-is-1-1-0-scoped]].

**How to apply:** before calling a comparison record "mass divergence", split
red rows by verdict scope; and never read a binding's `applies` as if it gated
anything.
