# A1 Spec Audit — Verify + Fix — chapter `its-rest-query-definition-admin`

- **Chapter:** ITS-REST Query / Definition (stored query, ADL 1.4, ADL2) /
  Admin operation semantics
- **Date:** 2026-07-11
- **Scope:** all 45 requirements `its-rest-query-definition-admin-R1 … R45`
- **Result (defer-nothing pass):** 1 fix — the REST ADL2 template upload
  **replaced** an existing template where the vendored contract declares
  `409_template_already_exists`; the adapter now 409s on a duplicate HRID
  (and 400s invalid source) while the SM-native `upload_artefact` keeps its
  spec-mandated replace semantics — the surface divergence is documented at
  the seam. Zero deferrals.

## Verdict table (condensed)

| ids | classification | evidence |
|---|---|---|
| R1, R2, R10 | verified | `q` required on ad-hoc GET/POST (params::build 400s); ECC QueryProvisioning |
| R3–R9 | verified | fetch/offset int32 handling; `fetch`+LIMIT/TOP conflict → 400 (`compose_paging`; TOP lowers to LIMIT); query-parameter substitution typed (`build_params`) |
| R11–R17 | verified | stored-query execute: name resolution incl. `misc` default namespace, SEMVER **prefix** resolution to latest-matching (stored_query.rs), 404 unknown, RESULT_SET shape + ETag (B6 query wire tail; ECC QueryProvisioning + golden corpus) |
| R18 | verified | PUT stored query exact-version duplicate → 409 (immutable pairs, `409_StoredQuery_version.yaml` cited at the store path) |
| R19–R26 | verified | store 400 on unparseable/wrong formalism (`valid_query_text` parse gate); list/get semantics; OPT 1.4 upload/list/get/delete + `Accept` enum 406 arms (ECC Adl14OptProvisioning) |
| R27, R28 | verified | OPT 1.4 duplicate → 409 (insert-only `DO NOTHING`, CNF `upload_opt-valid_opt_twice_conflict`); invalid OPT → 400 with the AOM rule code (B2 + ch13) |
| R29–R35 | verified | admin surface (ECC + B3 dump/load suites; physical deletes) |
| R36 | fixed | REST ADL2 template upload: duplicate HRID → 409 (`definition-codegen.openapi.yaml` declares `409_template_already_exists`); the SM-native replace stays (SM master04 "replace it") — divergence-by-surface documented at the adapter; test `adl2_template_upload_wire_conflicts_on_duplicate` (asserts both behaviours) |
| R37 | fixed-via-adapter | invalid ADL2 source at the REST surface → 400 (the ch14 registration validator raises precondition before the 422 path); native `upload_artefact` keeps `invalid_artefact` per SM |
| R38–R45 | verified | ADL2 list/get text/plain service; list_matching invalid pattern → `invalid_id_pattern` 400; remaining wire details per the generated contract + ECC |

## Fixes applied

- `service/api/definition.rs::template_adl2_upload` — pre-validate +
  existence check → 409/400 at the REST seam (SM-native semantics
  unchanged); `adl2_validation` visibility widened to `pub(crate)`.

## Deferred

None.

## Uncertain / runtime probes

None remaining.
