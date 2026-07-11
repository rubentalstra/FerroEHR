# A1 Spec Audit — Verify + Fix — chapter `sm-platform`

- **Chapter:** SM Platform Service Model (master02–15 + UML class files)
- **Date:** 2026-07-11
- **Scope:** all 56 requirements `sm-platform-R1 … R56`
- **Result (defer-nothing pass):** 2 defects fixed — the bundle `subsumes`
  returned identity-true (strict subsumption excludes the code itself; a
  flat vocabulary strictly subsumes nothing), and subject-variable
  canonical names were stored without the master10 naming validity check.
  Everything else verifies through the SM-1..SM-6 build + the PR #33
  change-control audit + the 341-execution ECC baseline. Zero deferrals.

## Verdict table (condensed)

| ids | classification | evidence |
|---|---|---|
| R1, R2 | verified | one `sqlx::Transaction` per service write; every versioned write emits CONTRIBUTION + AUDIT + version atomically (PR #33 audit; vobject/contribution) |
| R3 | verified | `time_committed`/`system_id` are server-computed (`now()` default + service system id; `ck_audit_system_id_nonempty`) — client values never read |
| R4 | verified | `parse_preceding`/`expected_from_if_match` (ch1 TreeId): missing/mismatched preceding on a non-first update → `version_mismatch` (ECC ChangeSets) |
| R5, R7, R8 | verified-model | `UpdateVersion` carries `lifecycle_state`/`data`/`audit` non-Option (typed 1..1); `UpdateAudit.change_type`/`committer` mandatory; wire adapter synthesizes `532\|complete\|` for plain PUT/POST per ITS-REST practice |
| R6 | verified | `validate_commit_audit` + `service/codes.rs` audit-change-type group check + the DB CHECK |
| R9 | verified | kind-discriminated typed deserialize per container (COMPOSITION/FOLDER/PARTY/PARTY_RELATIONSHIP payloads; foreign `_type` → 422) |
| R10, R13, R14 | verified | `CallStatusType` carries the full generic set + the EHR and Definition descendant codes with the exact snake_case renderings (`error.rs`) |
| R11, R12 | verified-equivalent | Rust `Result<_, SmError>` realizes `last_call_failed`/`last_call_status` (master02 formal equivalence — the failing call carries its `CALL_STATUS` fields on `SmError`) |
| R15 | verified-policy | the native `create_ehr` takes an optional EHR_STATUS; a subject-bearing status routes as the `create_ehr_for_subject` semantics (the ITS-REST/CNF flow — PUT /ehr with subject-bearing status is the standard subject-EHR creation; CNF outranks the subject-less-only prose reading) with the one-EHR-per-subject guard |
| R16–R19 | verified | duplicate-id 409 (`ehr_create_fail_duplicate_id`), default EHR_STATUS (modifiable+queryable+PARTY_SELF), duplicate-subject 409, `ehr_id_does_not_exist` on all EHR-scoped reads — ECC EhrService/EhrStatus cases |
| R20–R23 | verified | create/update composition validation gates (B2 walker + `definitions_valid`), logical delete to `523\|deleted\|` (PR #33), OBJECT_VERSION_ID vs UUID parameter split (ch1 `version_id.rs`) |
| R24–R26 | verified | directory Pre_no_directory 409, preceding-version discipline, logical delete — ECC DirectoryOps |
| R27 | verified | is_queryable/is_modifiable set/clear post-conditions (+ ch2's `is_modifiable = False` write guard) |
| R28, R29 | verified | multi-version contribution commit under one audit; `contribution_does_not_exist` |
| R30–R32 | verified | demographic create/update/delete versioning + status codes (SM-3, ch8) |
| R33–R36 | verified | ADL2 upload now Pre_valid-gated by the ch14 registration validator; ADL 1.4 archetype/OPT upload validity (B2 + ch13); delete pre/post |
| R37 | verified | uncompilable id patterns → `invalid_id_pattern` (definition.rs) |
| R38, R39 | verified | formalism case/version equivalence (`is_aql_v1`); `misc` default namespace (`qualify`) — unit-tested |
| R40–R42 | verified | store_query parse-gate (`invalid_query`); QUERY_DESCRIPTOR fields; delete pre/post |
| R43 | verified | population queries filter `is_queryable` (service_aql `set_not_queryable` case) |
| R44–R46 | verified-equivalent | the native API is wire-shaped (ADR-011): `fetch`/`offset` are `Option`-typed (explicit absence instead of the SM's ≤0 sentinels) with the ITS-REST composition rules — semantically equivalent paging capability (master02 formal equivalence); `Page::all()` covers 'all' |
| R47 | verified | EHR-index existence errors (SM-3) |
| R48, R50 | verified | terminology preconditions (`Pre_has_terminology`/`Pre_has_term`/`Pre_has_value_set` in `service/terminology.rs`); value-set membership semantics |
| R49 | fixed | bundle `subsumes` returned TRUE for identical codes — strict subsumption excludes identity and flat vocabularies have no hierarchy, so it is now uniformly False (hierarchical subsumption via the FHIR provider's `$subsumes`, which already excluded `equivalent`); test corrected with the citation |
| R51–R53 | verified | dump/load duplicate-id failure, `file_not_writable`, physical deletes incl. relationship cleanup (B3 suites) |
| R54 | verified | extract import into fixed/existing EHR (ch9 + B3) |
| R55 | fixed | subject-variable canonical names are now validity-checked at storage (no whitespace/unprintable, non-empty; namespace included) — `SubjectVariable::name_valid()` + the service reject; tests in both crates |
| R56 | verified | `definitions_valid`/`content_valid` gates on every commit path (template lookup + walker + typed deserialize) |

## Fixes applied

- `app/ehrbase/src/service/terminology.rs` — `subsumes` strictness (R49).
- `app/ehrbase-sm/src/services/subject_proxy.rs` + `app/ehrbase/src/service/subject_proxy.rs`
  — `name_valid()` + the storage-time reject (R55).

## Deferred

None.

## Uncertain / runtime probes

None remaining.
