---
name: runner-ignores-case-level-applies
description: RUNNER selection defect — run.rs::execute never consults case.applies, so version-floored cases are driven against a party that declares a lower spec version
metadata:
  type: project
---

`run.rs::execute` applies FIVE drive-time selection arms — unrealized binding,
extension-family × unclaimed capability, `case.option` vs `statement.options`,
`OperationBinding.applies` (`unmet_binding_floors`, run.rs:384-406), SMART lane
/ instance / ixit facts / exclusive-server — but **never `case.applies`**. Only
`verdict.rs::select` (`applies_satisfied`, verdict.rs:509) filters on it.

**Why it matters:** the #635 floor triage put the ITS-REST 1.1.0 floors on the
CASE, not the binding. On the 2026-07-28 ehrbase-java record (party declares
`its_rest: 1.0.3`) **127 red rows** sat on cases carrying
`applies: { its_rest: ">=1.1.0" }` — 57 SF, 27 item tags, 16 demographic,
8 composition, 7 admin, … They are excluded from the verdict, but they are
published in `results.json` as failed/errored, i.e. as divergences on surfaces
the release dated after the party's declared version. Only **4 binding-level**
floors exist in the whole catalogue (3 × `I_EHR_SERVICE.create_ehr*`,
`I_QUERY_SERVICE.execute_stored_query`), so the binding arm catches almost
nothing.

**How to apply:** the fix is one more arm in `run.rs::execute` mirroring the
binding-floor arm (same not-applicable-with-citation shape). Until then, never
read a red-row count off `results.json` for a party whose `spec_versions` are
below the catalogue's realized ITS-REST — split by `case.applies` first.
See [[results-json-records-out-of-scope-rows]], [[its-rest-1-1-0-dated-surfaces]].
