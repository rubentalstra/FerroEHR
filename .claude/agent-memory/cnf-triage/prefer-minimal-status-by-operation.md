---
name: prefer-minimal-status-by-operation
description: Prefer return=minimal (and absent Prefer) answers 204 on UPDATE, 201 on CREATE — the docs text is silent, the released OAS fills it per operation
metadata:
  type: project
---

CONFIRMED first-hand 2026-07-28 (register AMB-62's ground re-verified):

- DOCS TEXT is SILENT on the status of the minimal branch — overview
  `Requests_and_responses.md` §"Prefer minimal, identifier or full
  representation response" only offers "The HTTP status is typically `201
  Created`" (descriptive) + "If no response body is returned, the service
  SHOULD use `204 No Content`" (conditional).
- The RELEASED OAS fills it PER OPERATION FAMILY:
  `responses/204_version_updated.yaml` — "`204 No Content` is returned when the
  update operation was successful and the `Prefer` header is missing or is set
  to `return=minimal`"; `responses/200_COMPOSITION_updated.yaml` is scoped to
  "`Prefer` header is `return=representation`, or only its identifiers when …
  `return=identifier`". `operations/composition_create.yaml` declares **201
  only** (no 204), so CREATE-minimal is 201-with-no-body.

So: **UPDATE-minimal = 204, CREATE-minimal = 201.** A 200-with-empty-body on an
update matches no released response declaration and violates the docs-text
SHOULD.

App seam: `ferroehr-rest::overview::negotiate::write_negotiated(headers,
minimal_status, repr_status, …)` — `AppliedPreference::Minimal =>
empty(minimal_status)`. EHR_STATUS + DIRECTORY updates pass `no_content`;
`api/ehr/composition.rs` composition_update passed `ok, ok` (the 2026-07-28 red
row). `identifier_status()` already re-routes 204→repr status for the
identifier variant, so setting minimal=204 does not break `-return_identifier`.
