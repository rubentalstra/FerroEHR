---
name: provisioning-silently-swallows-failures
description: requires.templates upload discards its HTTP result and `requires.server: empty` is a documented no-op — both let a mis-provisioned SUT produce fake behavioural failures
metadata:
  type: project
---

`exec/driver.rs::provision`, confirmed 2026-07-28:

- **Template upload result discarded at TWO sites**: `let _uploaded =
  self.send(request_spec.method, &url, &headers, Some(&payload), false)?;` —
  `provision_synthesized_opt` **driver.rs:2122** (per-row synthesized OPT) and
  `provision` **driver.rs:2325-2326** (`requires.templates`). Any non-2xx
  (406/415/422) on `POST /definition/template/adl1.4` is swallowed and the case
  proceeds as if the template exists — downstream commits then fail on
  "validation_failed"/"template_not_found" and read as SUT defects.
  The comment only sanctions tolerating **409 already-provisioned**.
- **MEASURED BLAST RADIUS (ehrbase, 2026-07-28): 197 of 294 in-scope red
  rows** — the whole 123-row CONT battery plus most of composition/contribution
  — came from ONE swallowed 406. Reproduced: provisioning POSTs the OPT with
  `Accept: application/json` (the binding's declared header, now shared with the
  step path per #629), EHRbase answers `406 {"error":"Not Acceptable"}` and
  serves the upload only for `Accept: application/xml`; every following commit
  gets `422 "Template with template_id '…' does not exist"`, reported as
  "expected `created`, observed `validation_failed`".
- Provisioning now uses `compose_headers` (the one request-construction path),
  so it DOES send the binding's `Accept` — the earlier "provisioning sends no
  Accept" note is obsolete. The failure mode moved: the shared Accept means a
  server that refuses that representation breaks provisioning for every
  template-dependent case at once.
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
