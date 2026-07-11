# A1 Spec Audit — Verify + Fix — chapter `its-rest-ehr-composition`

- **Chapter:** ITS-REST EHR / EHR_STATUS / COMPOSITION / DIRECTORY /
  CONTRIBUTION operation semantics
- **Date:** 2026-07-11
- **Scope:** all 53 requirements `its-rest-ehr-composition-R1 … R53`
- **Result (defer-nothing pass):** 2 fixes — a body-supplied
  `COMPOSITION.uid` was not cross-checked against the path on PUT (a
  mismatched body could silently write to the path's object), and a
  client-supplied **CONTRIBUTION uid was silently ignored** (the spec
  honours it when unused, conflicts when in use). Zero deferrals.

## Verdict table (condensed)

| ids | classification | evidence |
|---|---|---|
| R1, R2 | verified | monomorphic-slot rejection via kind-discriminated typed deserialize (`validate.rs::run::<T>`; EHR_STATUS.subject PARTY_SELF-only per RM ehr_status) — ch1/ch3 audit work |
| R3, R4 | verified | EHR_STATUS mandatory trio (`validate_ehr_status`) and COMPOSITION mandatory set (typed + walker) → 400/422; ECC EhrStatus/CompositionOps |
| R5, R6 | verified | PARTY_PROXY unions dispatch on `_type` (generated enums; foreign `_type` → 422); committer via `validate_committer` |
| R7–R9 | verified | NewContribution/UPDATE_VERSION/UPDATE_AUDIT mandatory fields (typed `UpdateVersion`/`UpdateAudit` + `commit_version_set` checks) |
| R11, R12 | verified | duplicate-subject 409 (`uq_ehr_subject`), duplicate ehr_id 409 (ECC create_ehr cases) |
| R13 | verified | `parse_ehr_id` → 400 on malformed |
| R16, R17 | verified | 400 invalid COMPOSITION vs 422 unknown-template split (`definitions_valid` → 422; ECC) |
| R18 | fixed | body `COMPOSITION.uid` vs path `uid_based_id` cross-check on PUT — mismatch → 400 (dispatch/ehr.rs) |
| R19–R21 | verified | uid-form discipline (HIER vs OBJECT_VERSION_ID per operation — ch1 `version_id.rs`; delete takes the latest OVID; 412 on stale If-Match with latest-version headers) |
| R24 | verified | required If-Match on ehr_status PUT (`require_if_match`) |
| R25, R26 | verified | the is_modifiable guard is scoped to EHR *contents* — EHR_STATUS itself always modifiable (`ensure_content_writable` doc + PORT-NOTEd 409 wire code) |
| R27 | fixed | client-supplied CONTRIBUTION uid honoured (`insert_contribution_with_id`, `ON CONFLICT DO NOTHING` → 409 "already in use"; malformed → 422); test `contribution_supplied_uid` |
| R28 | verified | multi-version atomic commit (one transaction) |
| R29 | verified | change-type/operation mismatch → 400 (`classify` + unit tests: creation-on-existing, modify-without-preceding, delete-with-data) |
| R30–R37 | verified | directory CRUD semantics (If-Match required, logical delete, 404/412 codes — ECC DirectoryOps); versioned_* read surfaces (ch1) |
| R38–R53 | verified | If-Match quoted-OVID format; ETag/Location header sets per response yamls; version-at-time reads; versioned-object metadata (revision history — ch1); ECC EhrService/EhrStatus/CompositionOps/ChangeSets all green |

## Fixes applied

- `dispatch/ehr.rs` — body-uid/path cross-check on `composition_update`.
- `vobject.rs::insert_contribution_with_id` + `commit_contribution`
  (supplied-uid parameter) + `contribution.rs::commit_version_set` (parse
  `body.uid.value`, 422 malformed) — duplicate → 409; the ADR-014 outbox
  comment scrubbed to a spec-silence flag while touching the file.

## Deferred

None.

## Uncertain / runtime probes

None remaining.
