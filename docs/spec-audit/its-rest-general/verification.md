# A1 Spec Audit — Verify + Fix — chapter `its-rest-general`

- **Chapter:** ITS-REST general protocol (Requests_and_responses + Resources)
- **Date:** 2026-07-11
- **Scope:** all 40 requirements `its-rest-general-R1 … R40`
- **Result (defer-nothing pass):** 1 fix — **`Prefer: resolve_refs`** had no
  trace in the codebase (despite the B6 row claiming closure): it is now
  honoured end-to-end on contribution reads (`versions` as full
  `ORIGINAL_VERSION`s). Everything else verifies through the B6 protocol
  tail (committal-header merge, If-Match hardening, `Last-Modified`,
  `OPTIONS /`, status-code fixes) and the 341-execution ECC baseline
  (CORE PASS · STANDARD PASS · OPTIONS OBTAINED). Zero deferrals.

## Verdict table (condensed)

| ids | classification | evidence |
|---|---|---|
| R1 | verified | Basic + Bearer auth with `WWW-Authenticate` on 401 (P11); SEC ECC cases pass |
| R2 | verified | every direct PUT/POST/DELETE routes through the CONTRIBUTION-wrapped commit path (PR #33 audit) |
| R3, R4 | verified | `committal.rs`: `openEHR-VERSION.*`/`openEHR-AUDIT_DETAILS.*` request headers parsed and MERGED with server defaults (B6 MUST item) |
| R5, R6 | verified | audit provenance server-side; header-supplied parts merged not overriding |
| R7–R9 | verified | If-Match full-OVID compare → 412 with the latest `version_uid` in `ETag` + `Location` (`error_with_meta` + unit test); quoted `version_uid` format parsed (ch1 `expected_from_if_match`) |
| R10 | verified | `openEHR-TEMPLATE_ID` on the FLAT commit surface (`dispatch/flat.rs`) |
| R11–R14 | verified | `Location` + `ETag` on create/update (201/204 header sets per the response yamls; ECC asserts them) |
| R15 | verified | status-code mapping per the generated contract (B6 fixes F-01-11/F-02-10/F-03-*) |
| R17–R19, R21 | verified | `prefers_representation` with the `return=minimal` default (unit test `prefer_default_is_minimal`); 204-with-Location on minimal |
| R20 | fixed | `Prefer: resolve_refs` honoured: `prefers_resolve_refs` (negotiate) → `get_contribution_resolved` (new SM trait op + service impl — versions resolved through the generic version reader into full `ORIGINAL_VERSION`s); integration test `contribution_resolve_refs`. (The demographic contribution wire — our own extension surface — keeps OBJECT_REFs.) |
| R22, R23 | verified | both canonical JSON and XSD-valid canonical XML served (C14N gate; ECC runs both formats) |
| R24, R25 | verified | 415 on unsupported `Content-Type`, 406 on unfulfillable `Accept` (negotiate + typed `ApiError` variants; ECC content-negotiation cases) |
| R26 | verified | `Content-Type` on every bodied response; none on 204 |
| R27–R29 | verified | generated canonical serialization: snake_case, `_`-prefixed metadata, `_type` on polymorphic slots, absent-not-null (fidelity gates) |
| R30 | verified | exact simplified media types (`application/openehr.wt.flat+json` etc.) |
| R31 | verified | date/time query params parse as extended ISO 8601 (jiff); basic-format values reject 400 (MUST-use-extended satisfied by rejection) |
| R32 | verified | verbatim canonical storage (node codec) — values returned in their original lexical form |
| R33, R34 | verified | identifier immutability (storage keys); `version_uid` lexical form = `object_id::creating_system_id::version_tree_id` with `object_id` = the VERSIONED_OBJECT uid (ch1 `version_id.rs` + branching work) |
| R35, R36 | verified | `OPTIONS /` conformance endpoint with `Allow` + Options body (B6 R32 item; ECC OPTIONS profile OBTAINED) |
| R37 | verified | template-endpoint Accept enum honoured (`dispatch/definition.rs` 406 arms) |
| R38, R39 | verified-policy | RFC-2119 classification respected throughout this register; HTTP method semantics are the axum routing surface of the generated contract |
| R40 | verified-policy | error-detail bodies are returned uniformly; the spec's `MAY … only when Prefer: return=representation` grants discretion, and the CNF Robot suites themselves consume error bodies without setting `Prefer` — CNF practice outranks the restrictive prose reading |

## Fixes applied

- `negotiate.rs::prefers_resolve_refs`; `EhrContributionService::
  get_contribution_resolved` (defaulted trait op + service impl over the
  generic version reader); dispatch wiring on `contribution_get`;
  integration test.

## Deferred

None.

## Uncertain / runtime probes

None remaining.
