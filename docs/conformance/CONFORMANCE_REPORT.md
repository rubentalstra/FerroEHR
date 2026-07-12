# ehrbase-rs Conformance Report (generated)

> Generated from a conformance run — never hand-asserted. Scoped and
> honest: the deviations section lists every skip with its reason.

## 1. SUT identity

- Product: ehrbase-rs 3.0.0
- SUT: `http://localhost:8080/ehrbase/rest/openehr/v1`
- Spec versions: RM 1.2.0 · ITS-REST development@e8a093e · AQL 1.1.0 · TERM 3.1.0
- Auth mode: basic
- Started: 2026-07-12T06:20:46.058532Z

**341 case×format executions · 211 passed · 104 failed · 0 not applicable.**

### Per-area matrix

| Area | Catalogue (active) | Passed | Failed | Errored | Skipped | N/A |
|---|--:|--:|--:|--:|--:|--:|
| EHR — EHR service | 13 | 10 | 3 | 0 | 0 | 0 |
| STA — EHR_STATUS | 10 | 6 | 4 | 0 | 0 | 0 |
| COM — COMPOSITION | 31 | 36 | 2 | 0 | 0 | 0 |
| CTB — CONTRIBUTION (change sets) | 31 | 24 | 2 | 0 | 5 | 0 |
| DIR — DIRECTORY (FOLDER) | 37 | 37 | 0 | 0 | 0 | 0 |
| TPL — Template / OPT provisioning | 16 | 11 | 1 | 0 | 4 | 0 |
| SQR — Stored-query provisioning | 7 | 5 | 0 | 0 | 2 | 0 |
| QRY — AQL execution | 13 | 13 | 0 | 0 | 0 | 0 |
| VAL — Content / archetype validation | 119 | 27 | 92 | 0 | 0 | 0 |
| DEM — Demographic service | 24 | 24 | 0 | 0 | 0 | 0 |
| ADM — Admin service | 6 | 6 | 0 | 0 | 0 | 0 |
| SEC — Security / authorization | 2 | 2 | 0 | 0 | 0 | 0 |
| SIG — Version signing | 5 | 5 | 0 | 0 | 1 | 0 |
| MSG — Messaging | 10 | 0 | 0 | 0 | 10 | 0 |
| TS — Terminology-server integration | 9 | 5 | 0 | 0 | 4 | 0 |

### Failures

Each failure must become a finding (`F-AA-NN`) before/with the fix — never an exclusion.

- **ECC-EHR-002** EHR existence check — existing subject id (`ehr/has-ehr-existing-subject-id`, json): expected status 201, got 422 (body: {"error":"Unprocessable Entity","message":"unprocessable entity: EHR_STATUS.subject must be a PARTY_SELF (RM ehr master04 §EHR Status): expected _type \"PARTY_SELF\", found \"PARTY_IDENTIFIED\""})
- **ECC-EHR-007** Create EHR — two EHRs same patient (`ehr/create-ehr-two-ehrs-same-patient`, json): expected status 201, got 422 (body: {"error":"Unprocessable Entity","message":"unprocessable entity: EHR_STATUS.subject must be a PARTY_SELF (RM ehr master04 §EHR Status): expected _type \"PARTY_SELF\", found \"PARTY_IDENTIFIED\""})
- **ECC-EHR-009** Get EHR — existing EHR by subject id (`ehr/get-ehr-existing-ehr-by-subject-id`, json): expected status 201, got 422 (body: {"error":"Unprocessable Entity","message":"unprocessable entity: EHR_STATUS.subject must be a PARTY_SELF (RM ehr master04 §EHR Status): expected _type \"PARTY_SELF\", found \"PARTY_IDENTIFIED\""})
- **ECC-STA-004** Set EHR_STATUS is_queryable — bad EHR (`sta/set-ehr-queryable-bad-ehr`, json): expected status in [400, 404, 412], got 422
- **ECC-STA-006** Set EHR_STATUS is_modifiable — bad EHR (`sta/set-ehr-modifiable-bad-ehr`, json): expected status in [400, 404, 412], got 422
- **ECC-STA-008** Clear EHR_STATUS is_queryable — bad EHR (`sta/clear-ehr-queryable-bad-ehr`, json): expected status in [400, 404, 412], got 422
- **ECC-STA-010** Clear EHR_STATUS is_modifiable — bad EHR (`sta/clear-ehr-modifiable-bad-ehr`, json): expected status in [400, 404, 412], got 422
- **ECC-COM-001** Create composition — event (`com/create-composition-event`, json): OPT nested/nested.opt upload returned 406 (expected 2xx or 409 already-present)
- **ECC-COM-002** Create composition — persistent (`com/create-composition-persistent`, json): OPT minimal_persistent/persistent_minimal.opt upload returned 406 (expected 2xx or 409 already-present)
- **ECC-CTB-001** Commit contribution — valid composition (`ctb/commit-contribution-valid-composition`, json): OPT minimal/minimal_evaluation.opt upload returned 406 (expected 2xx or 409 already-present)
- **ECC-CTB-004** Commit contribution — valid invalid compositions (`ctb/commit-contribution-valid-invalid-compositions`, json): OPT minimal/minimal_admin.opt upload returned 406 (expected 2xx or 409 already-present)
- **ECC-TPL-001** Upload OPT — valid OPT (provisions ADL 1.4 archetypes) (`tpl/upload-opt-valid-opt`, json): fresh valid OPT rejected with 406
- **ECC-VAL-001** Validate COMPOSITION — content card any context any (`val/comp-content-card-any-context-any`, json): authored OPT upload returned 406 (expected 2xx or 409 already-present)
- **ECC-VAL-002** Validate COMPOSITION — content card 1plus context any (`val/comp-content-card-1plus-context-any`, json): authored OPT upload returned 406 (expected 2xx or 409 already-present)
- **ECC-VAL-003** Validate COMPOSITION — content card 3plus context any (`val/comp-content-card-3plus-context-any`, json): authored OPT upload returned 406 (expected 2xx or 409 already-present)
- **ECC-VAL-004** Validate COMPOSITION — content card OPT context any (`val/comp-content-card-opt-context-any`, json): authored OPT upload returned 406 (expected 2xx or 409 already-present)
- **ECC-VAL-005** Validate COMPOSITION — content card mand context any (`val/comp-content-card-mand-context-any`, json): authored OPT upload returned 406 (expected 2xx or 409 already-present)
- **ECC-VAL-006** Validate COMPOSITION — content card 3to5 context any (`val/comp-content-card-3to5-context-any`, json): authored OPT upload returned 406 (expected 2xx or 409 already-present)
- **ECC-VAL-007** Validate COMPOSITION — content card any context mand (`val/comp-content-card-any-context-mand`, json): authored OPT upload returned 406 (expected 2xx or 409 already-present)
- **ECC-VAL-008** Validate COMPOSITION — content card 1plus context mand (`val/comp-content-card-1plus-context-mand`, json): authored OPT upload returned 406 (expected 2xx or 409 already-present)
- **ECC-VAL-009** Validate COMPOSITION — content card 3plus context mand (`val/comp-content-card-3plus-context-mand`, json): authored OPT upload returned 406 (expected 2xx or 409 already-present)
- **ECC-VAL-010** Validate COMPOSITION — content card OPT context mand (`val/comp-content-card-opt-context-mand`, json): authored OPT upload returned 406 (expected 2xx or 409 already-present)
- **ECC-VAL-011** Validate COMPOSITION — content card mand context mand (`val/comp-content-card-mand-context-mand`, json): authored OPT upload returned 406 (expected 2xx or 409 already-present)
- **ECC-VAL-012** Validate COMPOSITION — content card 3to5 context mand (`val/comp-content-card-3to5-context-mand`, json): authored OPT upload returned 406 (expected 2xx or 409 already-present)
- **ECC-VAL-017** Validate HISTORY — events card any summary ex OPT (`val/hist-events-card-any-summary-ex-opt`, json): authored OPT upload returned 406 (expected 2xx or 409 already-present)
- **ECC-VAL-018** Validate HISTORY — events card 1plus summary ex OPT (`val/hist-events-card-1plus-summary-ex-opt`, json): authored OPT upload returned 406 (expected 2xx or 409 already-present)
- **ECC-VAL-019** Validate HISTORY — events card 3plus summary ex OPT (`val/hist-events-card-3plus-summary-ex-opt`, json): authored OPT upload returned 406 (expected 2xx or 409 already-present)
- **ECC-VAL-020** Validate HISTORY — events card OPT summary ex OPT (`val/hist-events-card-opt-summary-ex-opt`, json): authored OPT upload returned 406 (expected 2xx or 409 already-present)
- **ECC-VAL-021** Validate HISTORY — events card mand summary ex OPT (`val/hist-events-card-mand-summary-ex-opt`, json): authored OPT upload returned 406 (expected 2xx or 409 already-present)
- **ECC-VAL-022** Validate HISTORY — events card 3to5 summary ex OPT (`val/hist-events-card-3to5-summary-ex-opt`, json): authored OPT upload returned 406 (expected 2xx or 409 already-present)
- **ECC-VAL-023** Validate HISTORY — events card any summary ex mand (`val/hist-events-card-any-summary-ex-mand`, json): authored OPT upload returned 406 (expected 2xx or 409 already-present)
- **ECC-VAL-024** Validate HISTORY — events card 1plus summary ex mand (`val/hist-events-card-1plus-summary-ex-mand`, json): authored OPT upload returned 406 (expected 2xx or 409 already-present)
- **ECC-VAL-025** Validate HISTORY — events card 3plus summary ex mand (`val/hist-events-card-3plus-summary-ex-mand`, json): authored OPT upload returned 406 (expected 2xx or 409 already-present)
- **ECC-VAL-026** Validate HISTORY — events card OPT summary ex mand (`val/hist-events-card-opt-summary-ex-mand`, json): authored OPT upload returned 406 (expected 2xx or 409 already-present)
- **ECC-VAL-027** Validate HISTORY — events card mand summary ex mand (`val/hist-events-card-mand-summary-ex-mand`, json): authored OPT upload returned 406 (expected 2xx or 409 already-present)
- **ECC-VAL-028** Validate HISTORY — events card 3to5 summary ex mand (`val/hist-events-card-3to5-summary-ex-mand`, json): authored OPT upload returned 406 (expected 2xx or 409 already-present)
- **ECC-VAL-032** Validate EVENT — type point event (`val/event-type-point-event`, json): authored OPT upload returned 406 (expected 2xx or 409 already-present)
- **ECC-VAL-033** Validate EVENT — type interval event (`val/event-type-interval-event`, json): authored OPT upload returned 406 (expected 2xx or 409 already-present)
- **ECC-VAL-034** Validate ITEM_STRUCTURE — type any (`val/item-str-type-any`, json): authored OPT upload returned 406 (expected 2xx or 409 already-present)
- **ECC-VAL-035** Validate ITEM_STRUCTURE — type item tree (`val/item-str-type-item-tree`, json): OPT validation/clinical_content_validation.opt upload returned 406 (expected 2xx or 409 already-present)
- **ECC-VAL-039** Validate DV_BOOLEAN — anything allowed (`val/dv-boolean-anything-allowed`, json): OPT all_types/Test_all_types.opt upload returned 406 (expected 2xx or 409 already-present)
- **ECC-VAL-040** Validate DV_BOOLEAN — only true allowed (`val/dv-boolean-only-true-allowed`, json): authored OPT upload returned 406 (expected 2xx or 409 already-present)
- **ECC-VAL-041** Validate DV_BOOLEAN — only false allowed (`val/dv-boolean-only-false-allowed`, json): authored OPT upload returned 406 (expected 2xx or 409 already-present)
- **ECC-VAL-042** Validate DV_IDENTIFIER — all pattern (`val/dv-identifier-all-pattern`, json): authored OPT upload returned 406 (expected 2xx or 409 already-present)
- **ECC-VAL-043** Validate DV_IDENTIFIER — all list (`val/dv-identifier-all-list`, json): authored OPT upload returned 406 (expected 2xx or 409 already-present)
- **ECC-VAL-045** Validate DV_TEXT — list (`val/dv-text-list`, json): authored OPT upload returned 406 (expected 2xx or 409 already-present)
- **ECC-VAL-047** Validate DV_CODED_TEXT — local codes (`val/dv-coded-text-local-codes`, json): OPT all_types/Test_all_types_v2.opt upload returned 406 (expected 2xx or 409 already-present)
- **ECC-VAL-048** Validate DV_CODED_TEXT — ext term (`val/dv-coded-text-ext-term`, json): authored OPT upload returned 406 (expected 2xx or 409 already-present)
- **ECC-VAL-051** Validate DV_SCALE — open (`val/dv-scale-open`, json): authored OPT upload returned 406 (expected 2xx or 409 already-present)
- **ECC-VAL-052** Validate DV_SCALE — constraint (`val/dv-scale-constraint`, json): authored OPT upload returned 406 (expected 2xx or 409 already-present)
- **ECC-VAL-054** Validate DV_COUNT — range (`val/dv-count-range`, json): authored OPT upload returned 406 (expected 2xx or 409 already-present)
- **ECC-VAL-055** Validate DV_COUNT — list (`val/dv-count-list`, json): authored OPT upload returned 406 (expected 2xx or 409 already-present)
- **ECC-VAL-057** Validate DV_QUANTITY — property (`val/dv-quantity-property`, json): authored OPT upload returned 406 (expected 2xx or 409 already-present)
- **ECC-VAL-059** Validate DV_QUANTITY — property units mag (`val/dv-quantity-property-units-mag`, json): OPT time_series/time_series.opt upload returned 406 (expected 2xx or 409 already-present)
- **ECC-VAL-061** Validate DV_PROPORTION — ratio (`val/dv-proportion-ratio`, json): authored OPT upload returned 406 (expected 2xx or 409 already-present)
- **ECC-VAL-062** Validate DV_PROPORTION — unitary (`val/dv-proportion-unitary`, json): authored OPT upload returned 406 (expected 2xx or 409 already-present)
- **ECC-VAL-063** Validate DV_PROPORTION — percent (`val/dv-proportion-percent`, json): authored OPT upload returned 406 (expected 2xx or 409 already-present)
- **ECC-VAL-064** Validate DV_PROPORTION — fraction (`val/dv-proportion-fraction`, json): authored OPT upload returned 406 (expected 2xx or 409 already-present)
- **ECC-VAL-065** Validate DV_PROPORTION — integer fraction (`val/dv-proportion-integer-fraction`, json): authored OPT upload returned 406 (expected 2xx or 409 already-present)
- **ECC-VAL-066** Validate DV_PROPORTION — any fraction (`val/dv-proportion-any-fraction`, json): OPT minimal/minimal_action_2.opt upload returned 406 (expected 2xx or 409 already-present)
- **ECC-VAL-067** Validate DV_PROPORTION — ratio range (`val/dv-proportion-ratio-range`, json): authored OPT upload returned 406 (expected 2xx or 409 already-present)
- **ECC-VAL-068** Validate DV_INTERVAL<DV_COUNT> — open (`val/dv-interval-dv-count-open`, json): authored OPT upload returned 406 (expected 2xx or 409 already-present)
- **ECC-VAL-069** Validate DV_INTERVAL<DV_COUNT> — lower upper (`val/dv-interval-dv-count-lower-upper`, json): authored OPT upload returned 406 (expected 2xx or 409 already-present)
- **ECC-VAL-070** Validate DV_INTERVAL<DV_COUNT> — lower upper list (`val/dv-interval-dv-count-lower-upper-list`, json): authored OPT upload returned 406 (expected 2xx or 409 already-present)
- **ECC-VAL-071** Validate DV_INTERVAL<DV_QUANTITY> — open (`val/dv-interval-dv-quantity-open`, json): authored OPT upload returned 406 (expected 2xx or 409 already-present)
- **ECC-VAL-072** Validate DV_INTERVAL<DV_QUANTITY> — upper lower (`val/dv-interval-dv-quantity-upper-lower`, json): authored OPT upload returned 406 (expected 2xx or 409 already-present)
- **ECC-VAL-073** Validate DV_INTERVAL<DV_DATE_TIME> — open (`val/dv-interval-dv-date-time-open`, json): authored OPT upload returned 406 (expected 2xx or 409 already-present)
- **ECC-VAL-074** Validate DV_INTERVAL<DV_DATE_TIME> — lower upper constraint (`val/dv-interval-dv-date-time-lower-upper-constraint`, json): authored OPT upload returned 406 (expected 2xx or 409 already-present)
- **ECC-VAL-075** Validate DV_INTERVAL<DV_DATE_TIME> — lower upper range (`val/dv-interval-dv-date-time-lower-upper-range`, json): authored OPT upload returned 406 (expected 2xx or 409 already-present)
- **ECC-VAL-076** Validate DV_INTERVAL<DV_DATE> — open (`val/dv-interval-dv-date-open`, json): authored OPT upload returned 406 (expected 2xx or 409 already-present)
- **ECC-VAL-077** Validate DV_INTERVAL<DV_DATE> — lower upper constraint (`val/dv-interval-dv-date-lower-upper-constraint`, json): authored OPT upload returned 406 (expected 2xx or 409 already-present)
- **ECC-VAL-078** Validate DV_INTERVAL<DV_DATE> — lower upper range (`val/dv-interval-dv-date-lower-upper-range`, json): authored OPT upload returned 406 (expected 2xx or 409 already-present)
- **ECC-VAL-079** Validate DV_INTERVAL<DV_TIME> — open (`val/dv-interval-dv-time-open`, json): authored OPT upload returned 406 (expected 2xx or 409 already-present)
- **ECC-VAL-080** Validate DV_INTERVAL<DV_TIME> — lower upper constraint (`val/dv-interval-dv-time-lower-upper-constraint`, json): authored OPT upload returned 406 (expected 2xx or 409 already-present)
- **ECC-VAL-081** Validate DV_INTERVAL<DV_TIME> — lower upper range (`val/dv-interval-dv-time-lower-upper-range`, json): authored OPT upload returned 406 (expected 2xx or 409 already-present)
- **ECC-VAL-082** Validate DV_INTERVAL<DV_DURATION> — open (`val/dv-interval-dv-duration-open`, json): authored OPT upload returned 406 (expected 2xx or 409 already-present)
- **ECC-VAL-083** Validate DV_INTERVAL<DV_DURATION> — constraint (`val/dv-interval-dv-duration-constraint`, json): authored OPT upload returned 406 (expected 2xx or 409 already-present)
- **ECC-VAL-084** Validate DV_INTERVAL<DV_DURATION> — range (`val/dv-interval-dv-duration-range`, json): authored OPT upload returned 406 (expected 2xx or 409 already-present)
- **ECC-VAL-085** Validate DV_INTERVAL<DV_ORDINAL> — open (`val/dv-interval-dv-ordinal-open`, json): authored OPT upload returned 406 (expected 2xx or 409 already-present)
- **ECC-VAL-086** Validate DV_INTERVAL<DV_ORDINAL> — constraint (`val/dv-interval-dv-ordinal-constraint`, json): authored OPT upload returned 406 (expected 2xx or 409 already-present)
- **ECC-VAL-087** Validate DV_INTERVAL<DV_SCALE> — open (`val/dv-interval-dv-scale-open`, json): authored OPT upload returned 406 (expected 2xx or 409 already-present)
- **ECC-VAL-088** Validate DV_INTERVAL<DV_SCALE> — constraint (`val/dv-interval-dv-scale-constraint`, json): authored OPT upload returned 406 (expected 2xx or 409 already-present)
- **ECC-VAL-089** Validate DV_INTERVAL<DV_PROPORTION> — open (`val/dv-interval-dv-proportion-open`, json): authored OPT upload returned 406 (expected 2xx or 409 already-present)
- **ECC-VAL-090** Validate DV_INTERVAL<DV_PROPORTION> — ratio (`val/dv-interval-dv-proportion-ratio`, json): authored OPT upload returned 406 (expected 2xx or 409 already-present)
- **ECC-VAL-091** Validate DV_INTERVAL<DV_PROPORTION> — unitary (`val/dv-interval-dv-proportion-unitary`, json): authored OPT upload returned 406 (expected 2xx or 409 already-present)
- **ECC-VAL-092** Validate DV_INTERVAL<DV_PROPORTION> — percentage (`val/dv-interval-dv-proportion-percentage`, json): authored OPT upload returned 406 (expected 2xx or 409 already-present)
- **ECC-VAL-093** Validate DV_INTERVAL<DV_PROPORTION> — fraction (`val/dv-interval-dv-proportion-fraction`, json): authored OPT upload returned 406 (expected 2xx or 409 already-present)
- **ECC-VAL-094** Validate DV_INTERVAL<DV_PROPORTION> — integer fraction (`val/dv-interval-dv-proportion-integer-fraction`, json): authored OPT upload returned 406 (expected 2xx or 409 already-present)
- **ECC-VAL-095** Validate DV_INTERVAL<DV_PROPORTION> — ratio range (`val/dv-interval-dv-proportion-ratio-range`, json): authored OPT upload returned 406 (expected 2xx or 409 already-present)
- **ECC-VAL-097** Validate DV_DURATION — fields (`val/dv-duration-fields`, json): authored OPT upload returned 406 (expected 2xx or 409 already-present)
- **ECC-VAL-098** Validate DV_DURATION — range (`val/dv-duration-range`, json): authored OPT upload returned 406 (expected 2xx or 409 already-present)
- **ECC-VAL-099** Validate DV_DURATION — fields range (`val/dv-duration-fields-range`, json): authored OPT upload returned 406 (expected 2xx or 409 already-present)
- **ECC-VAL-101** Validate DV_TIME — constraint (`val/dv-time-constraint`, json): authored OPT upload returned 406 (expected 2xx or 409 already-present)
- **ECC-VAL-102** Validate DV_TIME — range (`val/dv-time-range`, json): authored OPT upload returned 406 (expected 2xx or 409 already-present)
- **ECC-VAL-104** Validate DV_DATE — constraint (`val/dv-date-constraint`, json): authored OPT upload returned 406 (expected 2xx or 409 already-present)
- **ECC-VAL-105** Validate DV_DATE — range (`val/dv-date-range`, json): authored OPT upload returned 406 (expected 2xx or 409 already-present)
- **ECC-VAL-108** Validate DV_DATE_TIME — range (`val/dv-date-time-range`, json): authored OPT upload returned 406 (expected 2xx or 409 already-present)
- **ECC-VAL-110** Validate DV_PARSABLE — value formalism (`val/dv-parsable-value-formalism`, json): authored OPT upload returned 406 (expected 2xx or 409 already-present)
- **ECC-VAL-112** Validate DV_MULTIMEDIA — media type (`val/dv-multimedia-media-type`, json): authored OPT upload returned 406 (expected 2xx or 409 already-present)
- **ECC-VAL-114** Validate DV_URI — pattern (`val/dv-uri-pattern`, json): authored OPT upload returned 406 (expected 2xx or 409 already-present)
- **ECC-VAL-115** Validate DV_URI — list (`val/dv-uri-list`, json): authored OPT upload returned 406 (expected 2xx or 409 already-present)
- **ECC-VAL-116** Validate DV_EHR_URI — open (`val/dv-ehr-uri-open`, json): authored OPT upload returned 406 (expected 2xx or 409 already-present)
- **ECC-VAL-117** Validate DV_EHR_URI — pattern (`val/dv-ehr-uri-pattern`, json): authored OPT upload returned 406 (expected 2xx or 409 already-present)
- **ECC-VAL-118** Validate DV_EHR_URI — list (`val/dv-ehr-uri-list`, json): authored OPT upload returned 406 (expected 2xx or 409 already-present)

### Not applicable to this SUT (extensions / RM-version-sensitive)

_None — every catalogued case applies to this SUT._

## 2. Scope of test

| Field | Value |
|---|---|
| Profiles requested | all |
| Data formats | json, xml |
| Catalogue (active cases) | 333 |
| Executed | 341 |
| Passed | 211 |
| Failed | 104 |
| Not applicable | 0 |

## 3. Detailed test report

| ECC id | Capability | Format | Data sets | Result |
|---|---|---|--:|---|
| ECC-EHR-001 | EhrOperations | json | 1/1 | PASS |
| ECC-EHR-002 | EhrOperations | json | 0/0 | **FAIL** |
| ECC-EHR-003 | EhrOperations | json | 1/1 | PASS |
| ECC-EHR-004 | EhrOperations | json | 1/1 | PASS |
| ECC-EHR-005 | EhrOperations | json | 16/16 | PASS |
| ECC-EHR-006 | EhrOperations | json | 1/1 | PASS |
| ECC-EHR-007 | EhrOperations | json | 0/0 | **FAIL** |
| ECC-EHR-008 | EhrOperations | json | 1/1 | PASS |
| ECC-EHR-009 | EhrOperations | json | 0/0 | **FAIL** |
| ECC-EHR-010 | EhrOperations | json | 1/1 | PASS |
| ECC-EHR-011 | EhrOperations | json | 1/1 | PASS |
| ECC-STA-001 | EhrStatus | json | 1/1 | PASS |
| ECC-STA-002 | EhrStatus | json | 1/1 | PASS |
| ECC-STA-003 | EhrStatus | json | 1/1 | PASS |
| ECC-STA-004 | EhrStatus | json | 0/0 | **FAIL** |
| ECC-STA-005 | EhrStatus | json | 1/1 | PASS |
| ECC-STA-006 | EhrStatus | json | 0/0 | **FAIL** |
| ECC-STA-007 | EhrStatus | json | 1/1 | PASS |
| ECC-STA-008 | EhrStatus | json | 0/0 | **FAIL** |
| ECC-STA-009 | EhrStatus | json | 1/1 | PASS |
| ECC-STA-010 | EhrStatus | json | 0/0 | **FAIL** |
| ECC-EHR-012 | EhrOperations | json | 11/11 | PASS |
| ECC-EHR-013 | AnonymousEhrs | json | 1/1 | PASS |
| ECC-COM-001 | CompositionOps | json | 0/0 | **FAIL** |
| ECC-COM-001 | CompositionOps | xml | 1/1 | PASS |
| ECC-COM-002 | CompositionOps | json | 0/0 | **FAIL** |
| ECC-COM-002 | CompositionOps | xml | 1/1 | PASS |
| ECC-COM-003 | CompositionOps | json | 1/1 | PASS |
| ECC-COM-004 | CompositionOps | json | 1/1 | PASS |
| ECC-COM-005 | CompositionOps | json | 1/1 | PASS |
| ECC-COM-006 | CompositionOps | json | 1/1 | PASS |
| ECC-COM-007 | CompositionOps | json | 1/1 | PASS |
| ECC-COM-008 | CompositionOps | json | 1/1 | PASS |
| ECC-COM-008 | CompositionOps | xml | 1/1 | PASS |
| ECC-COM-009 | CompositionOps | json | 1/1 | PASS |
| ECC-COM-010 | CompositionOps | json | 1/1 | PASS |
| ECC-COM-011 | CompositionOps | json | 1/1 | PASS |
| ECC-COM-012 | CompositionOps | json | 1/1 | PASS |
| ECC-COM-013 | CompositionOps | json | 1/1 | PASS |
| ECC-COM-013 | CompositionOps | xml | 1/1 | PASS |
| ECC-COM-014 | CompositionOps | json | 1/1 | PASS |
| ECC-COM-014 | CompositionOps | xml | 1/1 | PASS |
| ECC-COM-015 | CompositionOps | json | 1/1 | PASS |
| ECC-COM-016 | CompositionOps | json | 1/1 | PASS |
| ECC-COM-017 | CompositionOps | json | 3/3 | PASS |
| ECC-COM-018 | CompositionOps | json | 1/1 | PASS |
| ECC-COM-018 | CompositionOps | xml | 1/1 | PASS |
| ECC-COM-019 | CompositionOps | json | 1/1 | PASS |
| ECC-COM-020 | CompositionOps | json | 1/1 | PASS |
| ECC-COM-021 | CompositionOps | json | 2/2 | PASS |
| ECC-COM-022 | Versioning | json | 1/1 | PASS |
| ECC-COM-022 | Versioning | xml | 1/1 | PASS |
| ECC-COM-023 | Versioning | json | 1/1 | PASS |
| ECC-COM-024 | Versioning | json | 1/1 | PASS |
| ECC-COM-025 | CompositionOps | json | 1/1 | PASS |
| ECC-COM-026 | CompositionOps | json | 1/1 | PASS |
| ECC-COM-027 | CompositionOps | json | 1/1 | PASS |
| ECC-COM-028 | CompositionOps | json | 1/1 | PASS |
| ECC-COM-029 | CompositionOps | json | 1/1 | PASS |
| ECC-COM-030 | CompositionOps | json | 1/1 | PASS |
| ECC-COM-031 | CompositionOps | json | 1/1 | PASS |
| ECC-CTB-001 | ChangeSets | json | 0/0 | **FAIL** |
| ECC-CTB-002 | ChangeSets | json | 1/1 | PASS |
| ECC-CTB-003 | ChangeSets | json | 1/1 | PASS |
| ECC-CTB-004 | ChangeSets | json | 0/0 | **FAIL** |
| ECC-CTB-005 | ChangeSets | json | 1/1 | PASS |
| ECC-CTB-006 | ChangeSets | json | 1/1 | PASS |
| ECC-CTB-007 | ChangeSets | json | 1/1 | PASS |
| ECC-CTB-008 | ChangeSets | json | 1/1 | PASS |
| ECC-CTB-009 | ChangeSets | json | 1/1 | PASS |
| ECC-CTB-010 | ChangeSets | json | 1/1 | PASS |
| ECC-CTB-011 | ChangeSets | json | 1/1 | PASS |
| ECC-CTB-012 | ChangeSets | json | 1/1 | PASS |
| ECC-CTB-013 | ChangeSets | json | 1/1 | PASS |
| ECC-CTB-014 | ChangeSets | json | 1/1 | PASS |
| ECC-CTB-015 | ChangeSets | json | 1/1 | PASS |
| ECC-CTB-016 | ChangeSets | json | 1/1 | PASS |
| ECC-CTB-017 | ChangeSets | json | 1/1 | PASS |
| ECC-CTB-018 | ChangeSets | json | 1/1 | PASS |
| ECC-CTB-019 | ChangeSets | json | 1/1 | PASS |
| ECC-CTB-020 | ChangeSets | json | 1/1 | PASS |
| ECC-CTB-021 | ChangeSets | json | 1/1 | PASS |
| ECC-CTB-022 | ChangeSets | json | 1/1 | PASS |
| ECC-CTB-023 | ChangeSets | json | 1/1 | PASS |
| ECC-CTB-024 | ChangeSets | json | 1/1 | PASS |
| ECC-CTB-025 | ChangeSets | json | 1/1 | PASS |
| ECC-CTB-026 | ChangeSets | json | 1/1 | PASS |
| ECC-CTB-027 | ChangeSets | json | 0/0 | skipped |
| ECC-CTB-028 | ChangeSets | json | 0/0 | skipped |
| ECC-CTB-029 | ChangeSets | json | 0/0 | skipped |
| ECC-CTB-030 | ChangeSets | json | 0/0 | skipped |
| ECC-CTB-031 | ChangeSets | json | 0/0 | skipped |
| ECC-DIR-001 | DirectoryOps | json | 1/1 | PASS |
| ECC-DIR-002 | DirectoryOps | json | 1/1 | PASS |
| ECC-DIR-003 | DirectoryOps | json | 1/1 | PASS |
| ECC-DIR-004 | DirectoryOps | json | 1/1 | PASS |
| ECC-DIR-005 | DirectoryOps | json | 1/1 | PASS |
| ECC-DIR-006 | DirectoryOps | json | 1/1 | PASS |
| ECC-DIR-007 | DirectoryOps | json | 1/1 | PASS |
| ECC-DIR-008 | DirectoryOps | json | 1/1 | PASS |
| ECC-DIR-009 | DirectoryOps | json | 1/1 | PASS |
| ECC-DIR-010 | DirectoryOps | json | 1/1 | PASS |
| ECC-DIR-011 | DirectoryOps | json | 1/1 | PASS |
| ECC-DIR-012 | DirectoryOps | json | 1/1 | PASS |
| ECC-DIR-013 | DirectoryOps | json | 1/1 | PASS |
| ECC-DIR-014 | DirectoryOps | json | 1/1 | PASS |
| ECC-DIR-015 | DirectoryOps | json | 1/1 | PASS |
| ECC-DIR-016 | DirectoryOps | json | 1/1 | PASS |
| ECC-DIR-017 | DirectoryOps | json | 1/1 | PASS |
| ECC-DIR-018 | DirectoryOps | json | 1/1 | PASS |
| ECC-DIR-019 | DirectoryOps | json | 1/1 | PASS |
| ECC-DIR-020 | DirectoryOps | json | 1/1 | PASS |
| ECC-DIR-021 | DirectoryOps | json | 1/1 | PASS |
| ECC-DIR-022 | DirectoryOps | json | 1/1 | PASS |
| ECC-DIR-023 | DirectoryOps | json | 1/1 | PASS |
| ECC-DIR-024 | DirectoryOps | json | 1/1 | PASS |
| ECC-DIR-025 | DirectoryOps | json | 1/1 | PASS |
| ECC-DIR-026 | DirectoryOps | json | 1/1 | PASS |
| ECC-DIR-027 | DirectoryOps | json | 1/1 | PASS |
| ECC-DIR-028 | DirectoryOps | json | 1/1 | PASS |
| ECC-DIR-029 | DirectoryOps | json | 1/1 | PASS |
| ECC-DIR-030 | DirectoryOps | json | 1/1 | PASS |
| ECC-DIR-031 | DirectoryOps | json | 1/1 | PASS |
| ECC-DIR-032 | DirectoryOps | json | 1/1 | PASS |
| ECC-DIR-033 | Versioning | json | 1/1 | PASS |
| ECC-DIR-034 | Versioning | json | 1/1 | PASS |
| ECC-DIR-035 | Versioning | json | 1/1 | PASS |
| ECC-DIR-036 | DirectoryOps | json | 1/1 | PASS |
| ECC-DIR-037 | DirectoryOps | json | 1/1 | PASS |
| ECC-TPL-001 | Adl14ArchetypeProvisioning | json | 0/0 | **FAIL** |
| ECC-TPL-002 | Adl14OptProvisioning | json | 18/18 | PASS |
| ECC-TPL-003 | Adl14OptProvisioning | json | 1/1 | PASS |
| ECC-TPL-004 | Adl14OptProvisioning | json | 1/1 | PASS |
| ECC-TPL-005 | Adl14OptProvisioning | json | 1/1 | PASS |
| ECC-TPL-006 | Adl14OptProvisioning | json | 1/1 | PASS |
| ECC-TPL-007 | Adl14OptProvisioning | json | 1/1 | PASS |
| ECC-TPL-008 | Adl14OptProvisioning | json | 1/1 | PASS |
| ECC-TPL-009 | Adl14OptProvisioning | json | 1/1 | PASS |
| ECC-TPL-010 | Adl14OptProvisioning | json | 1/1 | PASS |
| ECC-TPL-011 | Adl14OptProvisioning | json | 1/1 | PASS |
| ECC-TPL-012 | Adl14OptProvisioning | json | 1/1 | PASS |
| ECC-TPL-013 | Adl14OptProvisioning | json | 0/0 | skipped |
| ECC-TPL-014 | Adl14OptProvisioning | json | 0/0 | skipped |
| ECC-TPL-015 | Adl14OptProvisioning | json | 0/0 | skipped |
| ECC-TPL-016 | Adl14OptProvisioning | json | 0/0 | skipped |
| ECC-SQR-001 | QueryProvisioning | json | 1/1 | PASS |
| ECC-SQR-002 | QueryProvisioning | json | 1/1 | PASS |
| ECC-SQR-003 | QueryProvisioning | json | 1/1 | PASS |
| ECC-SQR-004 | QueryProvisioning | json | 0/0 | skipped |
| ECC-SQR-005 | QueryProvisioning | json | 0/0 | skipped |
| ECC-SQR-006 | QueryProvisioning | json | 1/1 | PASS |
| ECC-SQR-007 | QueryProvisioning | json | 1/1 | PASS |
| ECC-QRY-001 | AqlBasic | json | 1/1 | PASS |
| ECC-QRY-002 | AqlBasic | json | 1/1 | PASS |
| ECC-QRY-003 | AqlBasic | json | 1/1 | PASS |
| ECC-QRY-004 | AqlBasic | json | 1/1 | PASS |
| ECC-QRY-005 | AqlBasic | json | 2/2 | PASS |
| ECC-QRY-006 | AqlBasic | json | 25/25 | PASS |
| ECC-QRY-007 | AqlBasic | json | 18/18 | PASS |
| ECC-QRY-008 | AqlBasic | json | 11/11 | PASS |
| ECC-QRY-009 | AqlBasic | json | 16/16 | PASS |
| ECC-QRY-010 | AqlBasic | json | 21/21 | PASS |
| ECC-QRY-011 | AqlBasic | json | 15/15 | PASS |
| ECC-QRY-012 | AqlBasic | json | 1/1 | PASS |
| ECC-QRY-013 | AqlBasic | json | 7/7 | PASS |
| ECC-ADM-001 | AdminApi | json | 1/1 | PASS |
| ECC-ADM-002 | AdminApi | json | 1/1 | PASS |
| ECC-ADM-003 | AdminApi | json | 1/1 | PASS |
| ECC-ADM-004 | AdminApi | json | 1/1 | PASS |
| ECC-ADM-005 | AdminApi | json | 1/1 | PASS |
| ECC-ADM-006 | AdminApi | json | 1/1 | PASS |
| ECC-DEM-001 | DemographicApi | json | 1/1 | PASS |
| ECC-DEM-002 | DemographicApi | json | 1/1 | PASS |
| ECC-DEM-003 | DemographicApi | json | 1/1 | PASS |
| ECC-DEM-004 | DemographicApi | json | 1/1 | PASS |
| ECC-DEM-005 | DemographicApi | json | 1/1 | PASS |
| ECC-DEM-006 | DemographicApi | json | 1/1 | PASS |
| ECC-DEM-007 | DemographicApi | json | 1/1 | PASS |
| ECC-DEM-008 | DemographicApi | json | 1/1 | PASS |
| ECC-DEM-009 | DemographicApi | json | 1/1 | PASS |
| ECC-DEM-010 | DemographicApi | json | 1/1 | PASS |
| ECC-DEM-011 | DemographicApi | json | 1/1 | PASS |
| ECC-DEM-012 | DemographicApi | json | 1/1 | PASS |
| ECC-DEM-013 | DemographicApi | json | 1/1 | PASS |
| ECC-DEM-014 | DemographicApi | json | 1/1 | PASS |
| ECC-DEM-015 | DemographicApi | json | 1/1 | PASS |
| ECC-DEM-016 | DemographicApi | json | 1/1 | PASS |
| ECC-DEM-017 | DemographicApi | json | 1/1 | PASS |
| ECC-DEM-018 | DemographicApi | json | 1/1 | PASS |
| ECC-DEM-019 | DemographicApi | json | 1/1 | PASS |
| ECC-DEM-020 | DemographicApi | json | 1/1 | PASS |
| ECC-DEM-021 | DemographicApi | json | 1/1 | PASS |
| ECC-DEM-022 | DemographicApi | json | 1/1 | PASS |
| ECC-DEM-023 | DemographicApi | json | 1/1 | PASS |
| ECC-DEM-024 | DemographicApi | json | 1/1 | PASS |
| ECC-VAL-001 | ArchetypeValidation | json | 0/0 | **FAIL** |
| ECC-VAL-002 | ArchetypeValidation | json | 0/0 | **FAIL** |
| ECC-VAL-003 | ArchetypeValidation | json | 0/0 | **FAIL** |
| ECC-VAL-004 | ArchetypeValidation | json | 0/0 | **FAIL** |
| ECC-VAL-005 | ArchetypeValidation | json | 0/0 | **FAIL** |
| ECC-VAL-006 | ArchetypeValidation | json | 0/0 | **FAIL** |
| ECC-VAL-007 | ArchetypeValidation | json | 0/0 | **FAIL** |
| ECC-VAL-008 | ArchetypeValidation | json | 0/0 | **FAIL** |
| ECC-VAL-009 | ArchetypeValidation | json | 0/0 | **FAIL** |
| ECC-VAL-010 | ArchetypeValidation | json | 0/0 | **FAIL** |
| ECC-VAL-011 | ArchetypeValidation | json | 0/0 | **FAIL** |
| ECC-VAL-012 | ArchetypeValidation | json | 0/0 | **FAIL** |
| ECC-VAL-013 | ArchetypeValidation | json | 2/2 | PASS |
| ECC-VAL-014 | ArchetypeValidation | json | 2/2 | PASS |
| ECC-VAL-015 | ArchetypeValidation | json | 2/2 | PASS |
| ECC-VAL-016 | ArchetypeValidation | json | 2/2 | PASS |
| ECC-VAL-017 | ArchetypeValidation | json | 0/0 | **FAIL** |
| ECC-VAL-018 | ArchetypeValidation | json | 0/0 | **FAIL** |
| ECC-VAL-019 | ArchetypeValidation | json | 0/0 | **FAIL** |
| ECC-VAL-020 | ArchetypeValidation | json | 0/0 | **FAIL** |
| ECC-VAL-021 | ArchetypeValidation | json | 0/0 | **FAIL** |
| ECC-VAL-022 | ArchetypeValidation | json | 0/0 | **FAIL** |
| ECC-VAL-023 | ArchetypeValidation | json | 0/0 | **FAIL** |
| ECC-VAL-024 | ArchetypeValidation | json | 0/0 | **FAIL** |
| ECC-VAL-025 | ArchetypeValidation | json | 0/0 | **FAIL** |
| ECC-VAL-026 | ArchetypeValidation | json | 0/0 | **FAIL** |
| ECC-VAL-027 | ArchetypeValidation | json | 0/0 | **FAIL** |
| ECC-VAL-028 | ArchetypeValidation | json | 0/0 | **FAIL** |
| ECC-VAL-029 | ArchetypeValidation | json | 2/2 | PASS |
| ECC-VAL-030 | ArchetypeValidation | json | 2/2 | PASS |
| ECC-VAL-031 | ArchetypeValidation | json | 1/1 | PASS |
| ECC-VAL-032 | ArchetypeValidation | json | 0/0 | **FAIL** |
| ECC-VAL-033 | ArchetypeValidation | json | 0/0 | **FAIL** |
| ECC-VAL-034 | ArchetypeValidation | json | 0/0 | **FAIL** |
| ECC-VAL-035 | ArchetypeValidation | json | 0/0 | **FAIL** |
| ECC-VAL-036 | ArchetypeValidation | json | 2/2 | PASS |
| ECC-VAL-037 | ArchetypeValidation | json | 2/2 | PASS |
| ECC-VAL-038 | ArchetypeValidation | json | 2/2 | PASS |
| ECC-VAL-039 | ArchetypeValidation | json | 0/0 | **FAIL** |
| ECC-VAL-040 | ArchetypeValidation | json | 0/0 | **FAIL** |
| ECC-VAL-041 | ArchetypeValidation | json | 0/0 | **FAIL** |
| ECC-VAL-042 | ArchetypeValidation | json | 0/0 | **FAIL** |
| ECC-VAL-043 | ArchetypeValidation | json | 0/0 | **FAIL** |
| ECC-VAL-044 | ArchetypeValidation | json | 2/2 | PASS |
| ECC-VAL-045 | ArchetypeValidation | json | 0/0 | **FAIL** |
| ECC-VAL-046 | ArchetypeValidation | json | 2/2 | PASS |
| ECC-VAL-047 | ArchetypeValidation | json | 0/0 | **FAIL** |
| ECC-VAL-048 | ArchetypeValidation | json | 0/0 | **FAIL** |
| ECC-VAL-049 | ArchetypeValidation | json | 2/2 | PASS |
| ECC-VAL-050 | ArchetypeValidation | json | 2/2 | PASS |
| ECC-VAL-051 | ArchetypeValidation | json | 0/0 | **FAIL** |
| ECC-VAL-052 | ArchetypeValidation | json | 0/0 | **FAIL** |
| ECC-VAL-053 | ArchetypeValidation | json | 2/2 | PASS |
| ECC-VAL-054 | ArchetypeValidation | json | 0/0 | **FAIL** |
| ECC-VAL-055 | ArchetypeValidation | json | 0/0 | **FAIL** |
| ECC-VAL-056 | ArchetypeValidation | json | 2/2 | PASS |
| ECC-VAL-057 | ArchetypeValidation | json | 0/0 | **FAIL** |
| ECC-VAL-058 | ArchetypeValidation | json | 2/2 | PASS |
| ECC-VAL-059 | ArchetypeValidation | json | 0/0 | **FAIL** |
| ECC-VAL-060 | ArchetypeValidation | json | 2/2 | PASS |
| ECC-VAL-061 | ArchetypeValidation | json | 0/0 | **FAIL** |
| ECC-VAL-062 | ArchetypeValidation | json | 0/0 | **FAIL** |
| ECC-VAL-063 | ArchetypeValidation | json | 0/0 | **FAIL** |
| ECC-VAL-064 | ArchetypeValidation | json | 0/0 | **FAIL** |
| ECC-VAL-065 | ArchetypeValidation | json | 0/0 | **FAIL** |
| ECC-VAL-066 | ArchetypeValidation | json | 0/0 | **FAIL** |
| ECC-VAL-067 | ArchetypeValidation | json | 0/0 | **FAIL** |
| ECC-VAL-068 | ArchetypeValidation | json | 0/0 | **FAIL** |
| ECC-VAL-069 | ArchetypeValidation | json | 0/0 | **FAIL** |
| ECC-VAL-070 | ArchetypeValidation | json | 0/0 | **FAIL** |
| ECC-VAL-071 | ArchetypeValidation | json | 0/0 | **FAIL** |
| ECC-VAL-072 | ArchetypeValidation | json | 0/0 | **FAIL** |
| ECC-VAL-073 | ArchetypeValidation | json | 0/0 | **FAIL** |
| ECC-VAL-074 | ArchetypeValidation | json | 0/0 | **FAIL** |
| ECC-VAL-075 | ArchetypeValidation | json | 0/0 | **FAIL** |
| ECC-VAL-076 | ArchetypeValidation | json | 0/0 | **FAIL** |
| ECC-VAL-077 | ArchetypeValidation | json | 0/0 | **FAIL** |
| ECC-VAL-078 | ArchetypeValidation | json | 0/0 | **FAIL** |
| ECC-VAL-079 | ArchetypeValidation | json | 0/0 | **FAIL** |
| ECC-VAL-080 | ArchetypeValidation | json | 0/0 | **FAIL** |
| ECC-VAL-081 | ArchetypeValidation | json | 0/0 | **FAIL** |
| ECC-VAL-082 | ArchetypeValidation | json | 0/0 | **FAIL** |
| ECC-VAL-083 | ArchetypeValidation | json | 0/0 | **FAIL** |
| ECC-VAL-084 | ArchetypeValidation | json | 0/0 | **FAIL** |
| ECC-VAL-085 | ArchetypeValidation | json | 0/0 | **FAIL** |
| ECC-VAL-086 | ArchetypeValidation | json | 0/0 | **FAIL** |
| ECC-VAL-087 | ArchetypeValidation | json | 0/0 | **FAIL** |
| ECC-VAL-088 | ArchetypeValidation | json | 0/0 | **FAIL** |
| ECC-VAL-089 | ArchetypeValidation | json | 0/0 | **FAIL** |
| ECC-VAL-090 | ArchetypeValidation | json | 0/0 | **FAIL** |
| ECC-VAL-091 | ArchetypeValidation | json | 0/0 | **FAIL** |
| ECC-VAL-092 | ArchetypeValidation | json | 0/0 | **FAIL** |
| ECC-VAL-093 | ArchetypeValidation | json | 0/0 | **FAIL** |
| ECC-VAL-094 | ArchetypeValidation | json | 0/0 | **FAIL** |
| ECC-VAL-095 | ArchetypeValidation | json | 0/0 | **FAIL** |
| ECC-VAL-096 | ArchetypeValidation | json | 2/2 | PASS |
| ECC-VAL-097 | ArchetypeValidation | json | 0/0 | **FAIL** |
| ECC-VAL-098 | ArchetypeValidation | json | 0/0 | **FAIL** |
| ECC-VAL-099 | ArchetypeValidation | json | 0/0 | **FAIL** |
| ECC-VAL-100 | ArchetypeValidation | json | 2/2 | PASS |
| ECC-VAL-101 | ArchetypeValidation | json | 0/0 | **FAIL** |
| ECC-VAL-102 | ArchetypeValidation | json | 0/0 | **FAIL** |
| ECC-VAL-103 | ArchetypeValidation | json | 2/2 | PASS |
| ECC-VAL-104 | ArchetypeValidation | json | 0/0 | **FAIL** |
| ECC-VAL-105 | ArchetypeValidation | json | 0/0 | **FAIL** |
| ECC-VAL-119 | ArchetypeValidation | json | 1/1 | PASS |
| ECC-VAL-106 | ArchetypeValidation | json | 2/2 | PASS |
| ECC-VAL-107 | ArchetypeValidation | json | 2/2 | PASS |
| ECC-VAL-108 | ArchetypeValidation | json | 0/0 | **FAIL** |
| ECC-VAL-109 | ArchetypeValidation | json | 2/2 | PASS |
| ECC-VAL-110 | ArchetypeValidation | json | 0/0 | **FAIL** |
| ECC-VAL-111 | ArchetypeValidation | json | 2/2 | PASS |
| ECC-VAL-112 | ArchetypeValidation | json | 0/0 | **FAIL** |
| ECC-VAL-113 | ArchetypeValidation | json | 2/2 | PASS |
| ECC-VAL-114 | ArchetypeValidation | json | 0/0 | **FAIL** |
| ECC-VAL-115 | ArchetypeValidation | json | 0/0 | **FAIL** |
| ECC-VAL-116 | ArchetypeValidation | json | 0/0 | **FAIL** |
| ECC-VAL-117 | ArchetypeValidation | json | 0/0 | **FAIL** |
| ECC-VAL-118 | ArchetypeValidation | json | 0/0 | **FAIL** |
| ECC-SIG-001 | Signing | json | 1/1 | PASS |
| ECC-SIG-001 | Signing | xml | 1/1 | PASS |
| ECC-SIG-002 | Signing | json | 1/1 | PASS |
| ECC-SIG-003 | Signing | json | 4/4 | PASS |
| ECC-SIG-004 | Signing | json | 1/1 | PASS |
| ECC-SIG-005 | Signing | json | 0/0 | skipped |
| ECC-MSG-001 | Messaging | json | 0/0 | skipped |
| ECC-MSG-002 | Messaging | json | 0/0 | skipped |
| ECC-MSG-003 | Messaging | json | 0/0 | skipped |
| ECC-MSG-004 | Messaging | json | 0/0 | skipped |
| ECC-MSG-005 | Messaging | json | 0/0 | skipped |
| ECC-MSG-006 | Messaging | json | 0/0 | skipped |
| ECC-MSG-007 | Messaging | json | 0/0 | skipped |
| ECC-MSG-008 | Messaging | json | 0/0 | skipped |
| ECC-MSG-009 | Messaging | json | 0/0 | skipped |
| ECC-MSG-010 | Messaging | json | 0/0 | skipped |
| ECC-TS-001 | Terminology | json | 1/1 | PASS |
| ECC-TS-002 | Terminology | json | 2/2 | PASS |
| ECC-TS-003 | Terminology | json | 1/1 | PASS |
| ECC-TS-004 | Terminology | json | 1/1 | PASS |
| ECC-TS-005 | Terminology | json | 1/1 | PASS |
| ECC-TS-006 | Terminology | json | 0/0 | skipped |
| ECC-TS-007 | Terminology | json | 0/0 | skipped |
| ECC-TS-008 | Terminology | json | 0/0 | skipped |
| ECC-TS-009 | Terminology | json | 0/0 | skipped |
| ECC-SEC-001 | Authentication | json | 1/1 | PASS |
| ECC-SEC-002 | Authentication | json | 1/1 | PASS |

## 4. Profile verdict (machine-computed)

CORE/STANDARD are all-or-nothing (every capability must pass); OPTIONS is any-passes (obtained if ≥1 optional capability passes) — `master03-profiles.adoc`.

### Core — not claimable

| Capability | Passed | Failed | Errored | Skipped | N/A | Verdict |
|---|--:|--:|--:|--:|--:|---|
| Adl14ArchetypeProvisioning | 0 | 1 | 0 | 0 | 0 | fail |
| Adl14OptProvisioning | 11 | 0 | 0 | 4 | 0 | pass |
| EhrOperations | 9 | 3 | 0 | 0 | 0 | fail |
| EhrStatus | 6 | 4 | 0 | 0 | 0 | fail |
| CompositionOps | 32 | 2 | 0 | 0 | 0 | fail |
| ChangeSets | 24 | 2 | 0 | 5 | 0 | fail |
| Versioning | 7 | 0 | 0 | 0 | 0 | pass |
| ArchetypeValidation | 27 | 92 | 0 | 0 | 0 | fail |
| AnonymousEhrs | 1 | 0 | 0 | 0 | 0 | pass |

### Standard — not claimable

| Capability | Passed | Failed | Errored | Skipped | N/A | Verdict |
|---|--:|--:|--:|--:|--:|---|
| Adl14ArchetypeProvisioning | 0 | 1 | 0 | 0 | 0 | fail |
| Adl14OptProvisioning | 11 | 0 | 0 | 4 | 0 | pass |
| EhrOperations | 9 | 3 | 0 | 0 | 0 | fail |
| EhrStatus | 6 | 4 | 0 | 0 | 0 | fail |
| CompositionOps | 32 | 2 | 0 | 0 | 0 | fail |
| ChangeSets | 24 | 2 | 0 | 5 | 0 | fail |
| Versioning | 7 | 0 | 0 | 0 | 0 | pass |
| ArchetypeValidation | 27 | 92 | 0 | 0 | 0 | fail |
| AnonymousEhrs | 1 | 0 | 0 | 0 | 0 | pass |
| DirectoryOps | 34 | 0 | 0 | 0 | 0 | pass |
| QueryProvisioning | 5 | 0 | 0 | 2 | 0 | pass |
| AqlBasic | 13 | 0 | 0 | 0 | 0 | pass |
| Signing | 5 | 0 | 0 | 1 | 0 | pass |

### Options — **OBTAINED** (any-passes)

| Capability | Passed | Failed | Errored | Skipped | N/A | Verdict |
|---|--:|--:|--:|--:|--:|---|
| Adl2Provisioning | 0 | 0 | 0 | 0 | 0 | not evidenced |
| DemographicApi | 24 | 0 | 0 | 0 | 0 | pass |
| AqlAdvanced | 0 | 0 | 0 | 0 | 0 | not evidenced |
| Terminology | 5 | 0 | 0 | 4 | 0 | pass |
| AdminApi | 6 | 0 | 0 | 0 | 0 | pass |
| AdminActivityReport | 0 | 0 | 0 | 0 | 0 | not evidenced |
| AdminPhysicalDeletion | 0 | 0 | 0 | 0 | 0 | not evidenced |
| AdminEhrDumpLoad | 0 | 0 | 0 | 0 | 0 | not evidenced |
| AdminBulkEhrLoad | 0 | 0 | 0 | 0 | 0 | not evidenced |
| AdminEhrArchive | 0 | 0 | 0 | 0 | 0 | not evidenced |
| AdminDemographicArchive | 0 | 0 | 0 | 0 | 0 | not evidenced |
| Messaging | 0 | 0 | 0 | 10 | 0 | not evidenced |

## 5. Deviations (skips), by reason

| Reason | Cases |
|---|--:|
| NativeApiOnly: I_EHR_EXTRACT_SERVICE.export_ehr_extracts is exercised by app/ehrbase/tests/service_extract.rs::export_ehr_extracts_honours_item_list_and_all_versions — Messaging has no ITS-REST binding | 1 |
| NativeApiOnly: I_EHR_EXTRACT_SERVICE.export_ehrs (unknown EHR) is exercised by app/ehrbase/tests/service_extract.rs::export_ehrs_unknown_ehr_is_ehr_id_does_not_exist — Messaging has no ITS-REST binding | 1 |
| NativeApiOnly: I_EHR_EXTRACT_SERVICE.export_ehrs is exercised by app/ehrbase/tests/service_extract.rs::export_ehrs_carries_every_versioned_object_latest_only — Messaging has no ITS-REST binding | 1 |
| NativeApiOnly: I_EHR_EXTRACT_SERVICE.import_ehr (duplicate id) is exercised by app/ehrbase/tests/service_import.rs::import_ehr_duplicate_target_is_rejected — Messaging has no ITS-REST binding | 1 |
| NativeApiOnly: I_EHR_EXTRACT_SERVICE.import_ehr (fixed id) is exercised by app/ehrbase/tests/service_import.rs::import_ehr_into_fixed_fresh_id — Messaging has no ITS-REST binding | 1 |
| NativeApiOnly: I_EHR_EXTRACT_SERVICE.import_ehr is exercised by app/ehrbase/tests/service_import.rs::import_ehr_clone_into_fresh_target_reuses_source_id — Messaging has no ITS-REST binding | 1 |
| NativeApiOnly: I_EHR_EXTRACT_SERVICE.import_ehr_extract is exercised by app/ehrbase/tests/service_import.rs::import_ehr_extract_adds_a_versioned_object_and_rejects_re_import — Messaging has no ITS-REST binding | 1 |
| NativeApiOnly: I_TDD_SERVICE.import_tdd (typed rejections) is exercised by app/ehrbase/tests/service_tdd.rs::{tdd_import_rejects_malformed_payload, tdd_import_rejects_non_tdd_xml, tdd_import_rejects_unknown_ehr, tdd_import_rejects_unknown_template} — Messaging has no ITS-REST binding | 1 |
| NativeApiOnly: I_TDD_SERVICE.import_tdd is exercised by app/ehrbase/tests/service_tdd.rs::tdd_import_commits_composition — Messaging has no ITS-REST binding | 1 |
| NativeApiOnly: I_TDD_SERVICE.import_tdds is exercised by app/ehrbase/tests/service_tdd.rs::{tdd_import_tdds_batch_commits_all, tdd_import_tdds_batch_fail_fast} — Messaging has no ITS-REST binding | 1 |
| SM I_DEFINITION_ADL14.delete_opt() (CNF master04:319) has no ITS-REST ADL 1.4 binding — ITS-REST development@e8a093e (and Release-1.0.3) define no DELETE verb on /definition/template/adl1.4/{id}; OPT deletion lives in the ADMIN API only | 4 |
| SM I_DEFINITION_QUERY.list_queries() (CNF master05:93) has no ITS-REST binding — ITS-REST development@e8a093e (and Release-1.0.3) expose GET /definition/query/{qualified_query_name}, not a bare GET /definition/query collection | 2 |
| SM I_EHR_CONTRIBUTION.list_contributions() (CNF master08:595) has no ITS-REST binding — ITS-REST development@e8a093e (and Release-1.0.3) define POST only on /ehr/{ehr_id}/contribution, with no GET collection resource; the list is a native-API concern, not wire-exercisable | 5 |
| SutConfig: no FHIR terminology provider configured on the SUT (EHRBASE_VALIDATION_EXTERNAL_TERMINOLOGY_* unset) — a `hl7.org/fhir/4.0` expand is rejected as `UnknownTerminologyService`. harness terminology server: http://127.0.0.1:52910 (fixture). The bundle (`openehr`) expand cases prove the TERMINOLOGY family; wire this by pointing the SUT at a FHIR server (host.docker.internal for a runner-host fixture, docs/design/terminology-server-integration.md §5). | 1 |
| SutConfig: server not in `pgp` mode (needs a configured OpenPGP key); a pgp-keyed compose profile is a follow-up — digest cases prove the capability | 1 |
| SutConfig: the 5xx fault requires a fault-injecting terminology server wired to the SUT (--tx-server-url + an SUT FHIR provider pointed at it); the HTTP-only ECC cannot reconfigure an external SUT's provider per case. Harness tx server: http://127.0.0.1:52910 (fixture). The fault→500 mapping is proven by conformance ts::fixture::tests::fault_server_error_is_5xx + app/ehrbase/tests/terminology_fhir.rs::server_5xx_is_an_exception. | 1 |
| SutConfig: the malformed fault requires a fault-injecting terminology server wired to the SUT (--tx-server-url + an SUT FHIR provider pointed at it); the HTTP-only ECC cannot reconfigure an external SUT's provider per case. Harness tx server: http://127.0.0.1:52910 (fixture). The fault→500 mapping is proven by conformance ts::fixture::tests::fault_malformed_is_not_json + app/ehrbase/tests/terminology_fhir.rs::malformed_body_is_an_exception. | 1 |
| SutConfig: the timeout fault requires a fault-injecting terminology server wired to the SUT (--tx-server-url + an SUT FHIR provider pointed at it); the HTTP-only ECC cannot reconfigure an external SUT's provider per case. Harness tx server: http://127.0.0.1:52910 (fixture). The fault→500 mapping is proven by conformance ts::fixture::tests::fault_timeout_exceeds_a_short_client_deadline + app/ehrbase/tests/terminology_fhir.rs::timeout_is_an_exception. | 1 |

## 6. Terminology server (TS area)

- Server: `http://127.0.0.1:52910`
- Mode: fixture

Recorded FHIR-tx exchange (4 request(s)):

| # | Method | Path | Query |
|--:|---|---|---|
| 1 | GET | `/ValueSet/$expand` | url=http%3A%2F%2Fhl7.org%2Ffhir%2FValueSet%2Fsurface |
| 2 | GET | `/ValueSet/$validate-code` | url=http%3A%2F%2Fhl7.org%2Ffhir%2FValueSet%2Fsurface&code=B |
| 3 | GET | `/CodeSystem/$lookup` | code=B |
| 4 | GET | `/CodeSystem/$subsumes` | codeA=L&codeB=O |
