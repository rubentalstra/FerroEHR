---
name: provisioning-silently-swallows-failures
description: requires.templates upload discards its HTTP result and `requires.server: empty` is a documented no-op — both let a mis-provisioned SUT produce fake behavioural failures
metadata:
  type: project
---

`exec/driver.rs::provision`, confirmed 2026-07-28:

- **Template upload result discarded**: `let _uploaded = self.send(request_spec
  .method, &url, &headers, Some(&payload), false)?;` (~L2241). Any non-2xx
  (406/415/422) on `POST /definition/template/adl1.4` is swallowed and the case
  proceeds as if the template exists — downstream commits then fail on
  "validation_failed"/"template_not_found" and read as SUT defects.
  The comment only sanctions tolerating **409 already-provisioned**.
- **Provisioning sends no `Accept`** (headers are built from the binding's
  `request.headers` + auth only), while the normal step path defaults
  `Accept: application/json` (L1477/L1628). So the SAME upload can succeed in
  provisioning and 406 when driven as a case — that is exactly the ehrbase-java
  `I_DEFINITION_ADL14.upload_opt-*` 406 class.
- **`requires: { server: empty }` is a NO-OP** by design ("isolation is the
  runner's tenancy concern … never destructive", ~L2197). Emptiness is achieved
  only by the freshly composed SUT plus per-case scoping — the query cases mint
  a fresh EHR and pass `ehr_id` as the single-EHR execution scope
  (`Request.md` §About the ehr_id parameter), so a "row count N != expected 0"
  means the SUT ignored `ehr_id`, NOT that provisioning leaked.

**Superseded:** `merge_with_vars` now promotes numbers and booleans, so
`fetch: 100` DOES reach the URL — the old non-string-drop bug is fixed
(see [[runner-nonstring-with-values-dropped]], now historical).

**How to apply:** on any mass "validation_failed"/"template_not_found" class,
check whether the provisioning upload could have failed silently before
attributing to the SUT.
