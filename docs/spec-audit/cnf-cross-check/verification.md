# A1 Spec Audit — Verify + Fix — chapter `cnf-cross-check`

- **Chapter:** CNF Robot-suite behaviours NOT covered by ECC cases
- **Date:** 2026-07-12
- **Scope:** all 23 requirements `cnf-cross-check-R1 … R23`
- **Result (defer-nothing pass):** zero code defects — every row has
  implementation + non-ECC test evidence (the point of this chapter was to
  prove the ECC gaps are covered elsewhere). Zero deferrals.

## Verdict table (condensed)

| ids | classification | evidence |
|---|---|---|
| R1–R6 | verified | admin physical deletes (composition/contribution/directory/template/party incl. relationship cleanup) + cache invalidation — `service_admin.rs` suites (B3/SM-4); moka template-cache eviction on template delete |
| R7–R10 | verified | `versioned_ehr_status` container + revision history + at-time + by-version reads — `service_ehr.rs` (incl. the F-01-05/F-02-04 at-time cases); container `uid` = the EHR_STATUS VERSIONED_OBJECT uid (ch1) |
| R11, R12 | verified | versioned-composition revision history in commit order with `::N` tree-id suffixes (ch1 `revision_history` over the tree columns; branching work kept ordering by ordinal) |
| R13–R16 | verified | timezone-offset preservation is structural: the node codec stores the canonical JSON fragment **verbatim** (no temporal re-encoding anywhere on the path), reads reassemble the stored bytes, and AQL leaf extraction returns the stored text (`#>>`); evidence: `persistence.rs` `+01:00` jsonb round-trip, the corpus gates (fixtures carry `+00:00`, comma-fraction, offset-less and `Z` forms verbatim), ECC AQL datetime goldens |
| R17–R22 | verified | demographic PARTY_RELATIONSHIP create/get/update/delete + at-time/at-version + party-at-time — `service_demographic.rs` (SM-3 + ch8 audit: container-ref shape checks, source==uid, existence errors) |
| R23 | verified | admin reporting counts (`admin_contribution_count` per platform service + version counts) — `service_admin.rs` |

## Fixes applied

None required.

## Deferred

None.

## Uncertain / runtime probes

None remaining.
