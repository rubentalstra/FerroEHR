# OAPI — served-OpenAPI completeness sweep, every endpoint spec-exact

Owner (2026-07-18): "we are missing many many of these utoipa responses and
params and docs in all our endpoints", following the official openEHR docs —
and explicitly BOTH halves of the vendored ITS-REST spec: the
operation/response/parameter/header YAMLs
(`docs/specs/openehr/ITS-REST/specifications/{operations,responses,
parameters,headers,schemas}/`) AND the normative prose
(`docs/specs/openehr/ITS-REST/specifications/docs/` — the
`overview/Requests_and_responses.md` header/status/Prefer/If-Match rules and
the per-group chapters); prose-vs-yaml disagreements reconciled and
recorded. The served document is the only OpenAPI we publish (owner hard
rule). Baseline precedent: the DIRECTORY group (5 declarations), enriched in
the DIR row (PR #128). This file is deleted in the PR that lands the sweep.

## Inventory (162 `#[utoipa::path]` declarations, 15 files)

- [ ] **Wave 1a — EHR group** (`api/ehr/openapi_routes.rs`, 35 incl. the 5
      done DIRECTORY ops): EHR create/get/by-subject, EHR_STATUS get/at-time/
      version/update, VERSIONED_EHR_STATUS, COMPOSITION CRUD + versioned,
      CONTRIBUTION, item tags. Oracle: the matching
      `operations/*.yaml` + responses/parameters + overview prose.
- [ ] **Wave 1b — DEMOGRAPHIC group** (`api/demographic/openapi_routes.rs`
      43 + `relationship.rs` 8): PARTY/RELATIONSHIP CRUD + versioned +
      contributions + tags. Oracle: `operations/demographic_*.yaml` etc.
      (DEVELOPMENT-status API — note where the OAS itself is thin and the
      prose rules fill in).
- [ ] **Wave 2a — DEFINITION + QUERY + ADMIN + status** (`api/definition/
      openapi_routes.rs` 14, `api/query/openapi_routes.rs` 6,
      `api/admin/openapi_routes.rs` 5, `api/mod.rs` 3, `overview/status.rs`
      3): templates ADL1.4/2, stored queries, /query execute, admin EHR
      delete; the OPTIONS/system rows. Oracles: the matching op YAMLs;
      ADMIN is DEVELOPMENT-status.
- [ ] **Wave 2b — extension surfaces** (`extensions/management` 12,
      `fhir.rs` 8, `terminology.rs` 7, `openapi.rs` 6, `tenant_routes.rs` 5,
      `event_subscription.rs` 5, `smart/discovery.rs` 2): no openEHR spec
      governs these (our own design/extension — flag per declaration where
      relevant); the oracle is the HANDLER's real wire (read the code:
      params, headers, every reachable status) + the overview conventions
      for header naming.
- [ ] **The completeness gate**: a structural test over the SERVED document
      (every `{param}` in a path template has a matching documented Path
      parameter; every operation carries a description and ≥1 success + ≥1
      error response unless a documented discovery exception; every
      PUT/DELETE on a versioned resource documents `If-Match` + a 412;
      every RM-resource POST/PUT documents `Prefer`) — the ratchet that
      keeps the document complete.
- [ ] Gates: workspace clippy/nextest, the served==fresh + route-coverage
      openapi tests, changelog entry, ECC zero-drift (wire-doc only — no
      behaviour change expected; verify), `/phase-done` (row closed, this
      file deleted).

## Standard (set by the DIRECTORY precedent)

Every declaration documents: all path/query params with real descriptions;
request headers (`Prefer` with its three values, `If-Match` with the quoted
form note, committal headers where accepted); request body description;
EVERY reachable status code with its exact trigger (per the op YAML +
handler); response headers in prose where load-bearing (`ETag` weak form,
`Location`, `Last-Modified`); a doc comment naming the operation and its
spec anchor. Where prose and YAML disagree, the declaration follows the
reconciled reading and the PR description records it.
