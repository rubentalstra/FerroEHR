# ehrbase-rs Conformance Report (generated)

> Generated from a conformance run — never hand-asserted. Scoped and
> honest: the deviations section lists every skip with its reason.

## 1. SUT identity

- SUT: `http://localhost:8080/ehrbase/rest/openehr/v1`
- Spec versions: RM 1.2.0 · ITS-REST 1.0.3 · AQL 1.1.0 · TERM 3.1.0
- Auth mode: basic
- Started: 2026-07-09T05:59:31.703565Z

**318 case×format executions · 211 passed · 106 failed.**

### Per-area matrix

| Area | Catalogue (active) | Passed | Failed | Errored | Skipped |
|---|--:|--:|--:|--:|--:|
| EHR — EHR service | 12 | 12 | 0 | 0 | 0 |
| STA — EHR_STATUS | 10 | 10 | 0 | 0 | 0 |
| COM — COMPOSITION | 31 | 37 | 1 | 0 | 0 |
| CTB — CONTRIBUTION (change sets) | 31 | 26 | 5 | 0 | 0 |
| DIR — DIRECTORY (FOLDER) | 37 | 36 | 1 | 0 | 0 |
| TPL — Template / OPT provisioning | 16 | 14 | 2 | 0 | 0 |
| SQR — Stored-query provisioning | 7 | 3 | 4 | 0 | 0 |
| QRY — AQL execution | 13 | 8 | 5 | 0 | 0 |
| VAL — Content / archetype validation | 118 | 37 | 81 | 0 | 0 |
| DEM — Demographic service | 24 | 18 | 6 | 0 | 0 |
| ADM — Admin service | 6 | 6 | 0 | 0 | 0 |
| SIG — Version signing | 5 | 4 | 1 | 0 | 1 |

### Failures

Each failure must become a finding (`F-AA-NN`) before/with the fix — never an exclusion.

- **ECC-COM-022** Get versioned composition (`com/get-versioned-composition`, xml): expected status 200, got 406 (body: {"error":"Not Acceptable","message":"not acceptable: canonical XML for this response is available once typed payloads land (P12); request application/json"})
- **ECC-CTB-027** List contributions — empty (`ctb/list-contributions-empty`, json): expected status 200, got 405 (body: )
- **ECC-CTB-028** List contributions — non existing EHR (`ctb/list-contributions-non-existing-ehr`, json): expected status 404, got 405 (body: )
- **ECC-CTB-029** List contributions — post commit (`ctb/list-contributions-post-commit`, json): expected status 200, got 405 (body: )
- **ECC-CTB-030** List contributions — EHR containing directory (`ctb/list-contributions-ehr-containing-directory`, json): expected status 200, got 405 (body: )
- **ECC-CTB-031** List contributions — EHR containing EHR status (`ctb/list-contributions-ehr-containing-ehr-status`, json): expected status 200, got 405 (body: )
- **ECC-DIR-034** Get versioned directory — directory with two versions (`dir/get-versioned-directory-directory-with-two-versions`, json): expected status 200, got 404 (body: )
- **ECC-TPL-014** Delete OPT — delete existing (`tpl/delete-opt-delete-existing`, json): expected one of [200, 204], got 405
- **ECC-TPL-015** Delete OPT — delete latest version (`tpl/delete-opt-delete-latest-version`, json): expected one of [200, 204], got 405
- **ECC-SQR-004** List stored queries — empty (`sqr/list-queries-empty`, json): expected status 200, got 404 (body: )
- **ECC-SQR-005** List stored queries — select items (`sqr/list-queries-select-items`, json): expected status 200, got 404 (body: )
- **ECC-SQR-006** Store stored query — bad formalism (`sqr/valid-query-bad-formalism`, json): expected 400/422 for a non-AQL query, got 200
- **ECC-SQR-007** Store stored query — invalid (`sqr/valid-query-invalid`, json): expected 400/422 for malformed AQL, got 200
- **ECC-QRY-006** AQL corpus — A empty db (`qry/corpus-a-empty-db`, json): 24/27 A/empty_db goldens matched (0 skipped); first divergence: A/106_get_ehrs.json: valid query rejected with status 400 (body: {"error":"Bad Request","message":"bad request: attribute `ehr_status` is not defined on EHR (RM model)"})
- **ECC-QRY-009** AQL corpus — D empty db (`qry/corpus-d-empty-db`, json): 16/18 D/empty_db goldens matched (8 skipped); first divergence: D/312_select_data_values_from_all_ehrs_contains_composition_with_archetype_top_5.json: valid query rejected with status 400 (body: {"error":"Bad Request","message":"bad request: invalid AQL: found 'Order' at 29..30"})
- **ECC-QRY-010** AQL corpus — A loaded db (`qry/corpus-a-loaded-db`, json): 20/23 A/loaded_db goldens matched (4 skipped); first divergence: A/106_get_ehrs.json: valid query rejected with status 400 (body: {"error":"Bad Request","message":"bad request: attribute `ehr_status` is not defined on EHR (RM model)"})
- **ECC-QRY-011** AQL corpus — B loaded db (`qry/corpus-b-loaded-db`, json): 15/18 B/loaded_db goldens matched (6 skipped); first divergence: B/104_get_compositions_top_5_ordered_by_starttime_asc.json: valid query rejected with status 400 (body: {"error":"Bad Request","message":"bad request: invalid AQL: found 'Order' at 7..8"})
- **ECC-QRY-013** AQL corpus — D loaded db (`qry/corpus-d-loaded-db`, json): 7/9 D/loaded_db goldens matched (17 skipped); first divergence: D/312_select_data_values_from_all_ehrs_contains_composition_with_archetype_top_5.json: valid query rejected with status 400 (body: {"error":"Bad Request","message":"bad request: invalid AQL: found 'Order' at 29..30"})
- **ECC-DEM-005** Demographic person delete (`dem/person-delete`, json): expected status in [200, 204], got 400
- **ECC-DEM-006** Demographic person get deleted (`dem/person-get-deleted`, json): expected status in [204, 404], got 200
- **ECC-DEM-011** Demographic agent delete (`dem/agent-delete`, json): expected status in [200, 204], got 400
- **ECC-DEM-014** Demographic group delete (`dem/group-delete`, json): expected status in [200, 204], got 400
- **ECC-DEM-017** Demographic organisation delete (`dem/organisation-delete`, json): expected status in [200, 204], got 400
- **ECC-DEM-020** Demographic role delete (`dem/role-delete`, json): expected status in [200, 204], got 400
- **ECC-VAL-036** Validate ITEM_STRUCTURE — type item list (`val/item-str-type-item-list`, json): ITEM_STRUCTURE ITEM_LIST slot filled with ITEM_TREE (Class not allowed): expected rejected (ITS-REST validation composition_create.yaml 422), got 201
- **ECC-VAL-037** Validate ITEM_STRUCTURE — type item table (`val/item-str-type-item-table`, json): ITEM_STRUCTURE ITEM_TABLE slot filled with ITEM_TREE (Class not allowed): expected rejected (ITS-REST validation composition_create.yaml 422), got 201
- **ECC-VAL-038** Validate ITEM_STRUCTURE — type item single (`val/item-str-type-item-single`, json): ITEM_STRUCTURE ITEM_SINGLE slot filled with ITEM_TREE (Class not allowed): expected rejected (ITS-REST validation composition_create.yaml 422), got 201
- **ECC-VAL-039** Validate DV_BOOLEAN — anything allowed (`val/dv-boolean-anything-allowed`, json): DV_BOOLEAN with value (RM present): expected accepted (composition_create.yaml 201), got 422 ({"message":"1 validation error(s)","validationErrors":["/content[openEHR-EHR-SECTION.test_all_types.v1]/items[at0001]/items[at0002]/items[openEHR-EHR-INSTRUCTION.test_all_types.v1]/activities[at0001]/)
- **ECC-VAL-040** Validate DV_BOOLEAN — only true allowed (`val/dv-boolean-only-true-allowed`, json): value true allowed (C_BOOLEAN true-only): expected accepted (composition_create.yaml 201), got 422 ({"message":"1 validation error(s)","validationErrors":["/content[openEHR-EHR-SECTION.test_all_types.v1]/items[at0001]/items[at0002]/items[openEHR-EHR-INSTRUCTION.test_all_types.v1]/activities[at0001]/)
- **ECC-VAL-041** Validate DV_BOOLEAN — only false allowed (`val/dv-boolean-only-false-allowed`, json): value false allowed (C_BOOLEAN false-only): expected accepted (composition_create.yaml 201), got 422 ({"message":"1 validation error(s)","validationErrors":["/content[openEHR-EHR-SECTION.test_all_types.v1]/items[at0001]/items[at0002]/items[openEHR-EHR-INSTRUCTION.test_all_types.v1]/activities[at0001]/)
- **ECC-VAL-042** Validate DV_IDENTIFIER — all pattern (`val/dv-identifier-all-pattern`, json): id 54480987 matches [0-9]+ (accepted): expected accepted (composition_create.yaml 201), got 422 ({"message":"1 validation error(s)","validationErrors":["/content[openEHR-EHR-SECTION.test_all_types.v1]/items[at0001]/items[at0002]/items[openEHR-EHR-INSTRUCTION.test_all_types.v1]/activities[at0001]/)
- **ECC-VAL-043** Validate DV_IDENTIFIER — all list (`val/dv-identifier-all-list`, json): id 54480987 in list (accepted): expected accepted (composition_create.yaml 201), got 422 ({"message":"1 validation error(s)","validationErrors":["/content[openEHR-EHR-SECTION.test_all_types.v1]/items[at0001]/items[at0002]/items[openEHR-EHR-INSTRUCTION.test_all_types.v1]/activities[at0001]/)
- **ECC-VAL-044** Validate DV_TEXT — open (`val/dv-text-open`, json): DV_TEXT with value (RM present): expected accepted (composition_create.yaml 201), got 422 ({"message":"1 validation error(s)","validationErrors":["/content[openEHR-EHR-SECTION.test_all_types.v1]/items[at0001]/items[at0002]/items[openEHR-EHR-INSTRUCTION.test_all_types.v1]/activities[at0001]/)
- **ECC-VAL-045** Validate DV_TEXT — list (`val/dv-text-list`, json): DV_TEXT value in the C_STRING list (accepted): expected accepted (composition_create.yaml 201), got 422 ({"message":"1 validation error(s)","validationErrors":["/content[openEHR-EHR-SECTION.test_all_types.v1]/items[at0001]/items[at0002]/items[openEHR-EHR-INSTRUCTION.test_all_types.v1]/activities[at0001]/)
- **ECC-VAL-046** Validate DV_CODED_TEXT — open (`val/dv-coded-text-open`, json): DV_CODED_TEXT with defining_code (RM present): expected accepted (composition_create.yaml 201), got 422 ({"message":"1 validation error(s)","validationErrors":["/content[openEHR-EHR-SECTION.test_all_types.v1]/items[at0001]/items[at0002]/items[openEHR-EHR-INSTRUCTION.test_all_types.v1]/activities[at0001]/)
- **ECC-VAL-047** Validate DV_CODED_TEXT — local codes (`val/dv-coded-text-local-codes`, json): DV_CODED_TEXT local::at0023 in code_list (accepted): expected accepted (composition_create.yaml 201), got 422 ({"message":"1 validation error(s)","validationErrors":["/content[openEHR-EHR-SECTION.test_all_types.v1]/items[at0001]/items[at0002]/items[openEHR-EHR-INSTRUCTION.test_all_types.v1]/activities[at0001]/)
- **ECC-VAL-048** Validate DV_CODED_TEXT — ext term (`val/dv-coded-text-ext-term`, json): SNOMED-CT 73211009 in the external code_list (accepted): expected accepted (composition_create.yaml 201), got 422 ({"message":"1 validation error(s)","validationErrors":["/content[openEHR-EHR-SECTION.test_all_types.v1]/items[at0001]/items[at0002]/items[openEHR-EHR-INSTRUCTION.test_all_types.v1]/activities[at0001]/)
- **ECC-VAL-049** Validate DV_ORDINAL — open (`val/dv-ordinal-open`, json): DV_ORDINAL with value (RM present): expected accepted (composition_create.yaml 201), got 422 ({"message":"1 validation error(s)","validationErrors":["/content[openEHR-EHR-SECTION.test_all_types.v1]/items[at0001]/items[at0002]/items[openEHR-EHR-INSTRUCTION.test_all_types.v1]/activities[at0001]/)
- **ECC-VAL-050** Validate DV_ORDINAL — constraint (`val/dv-ordinal-constraint`, json): DV_ORDINAL symbol local::at0014 in list (accepted): expected accepted (composition_create.yaml 201), got 422 ({"message":"1 validation error(s)","validationErrors":["/content[openEHR-EHR-SECTION.test_all_types.v1]/items[at0001]/items[at0002]/items[openEHR-EHR-INSTRUCTION.test_all_types.v1]/activities[at0001]/)
- **ECC-VAL-051** Validate DV_SCALE — open (`val/dv-scale-open`, json): DV_SCALE with value+symbol (accepted): expected accepted (composition_create.yaml 201), got 422 ({"message":"1 validation error(s)","validationErrors":["/content[openEHR-EHR-SECTION.test_all_types.v1]/items[at0001]/items[at0002]/items[openEHR-EHR-INSTRUCTION.test_all_types.v1]/activities[at0001]/)
- **ECC-VAL-052** Validate DV_SCALE — constraint (`val/dv-scale-constraint`, json): DV_SCALE value 1.0 in list {1.0} (accepted): expected accepted (composition_create.yaml 201), got 422 ({"message":"1 validation error(s)","validationErrors":["/content[openEHR-EHR-SECTION.test_all_types.v1]/items[at0001]/items[at0002]/items[openEHR-EHR-INSTRUCTION.test_all_types.v1]/activities[at0001]/)
- **ECC-VAL-053** Validate DV_COUNT — open (`val/dv-count-open`, json): DV_COUNT with magnitude (RM present): expected accepted (composition_create.yaml 201), got 422 ({"message":"1 validation error(s)","validationErrors":["/content[openEHR-EHR-SECTION.test_all_types.v1]/items[at0001]/items[at0002]/items[openEHR-EHR-INSTRUCTION.test_all_types.v1]/activities[at0001]/)
- **ECC-VAL-054** Validate DV_COUNT — range (`val/dv-count-range`, json): DV_COUNT magnitude 3 in range [0,10] (accepted): expected accepted (composition_create.yaml 201), got 422 ({"message":"1 validation error(s)","validationErrors":["/content[openEHR-EHR-SECTION.test_all_types.v1]/items[at0001]/items[at0002]/items[openEHR-EHR-INSTRUCTION.test_all_types.v1]/activities[at0001]/)
- **ECC-VAL-055** Validate DV_COUNT — list (`val/dv-count-list`, json): DV_COUNT magnitude 3 in the C_INTEGER list {3} (accepted): expected accepted (composition_create.yaml 201), got 422 ({"message":"1 validation error(s)","validationErrors":["/content[openEHR-EHR-SECTION.test_all_types.v1]/items[at0001]/items[at0002]/items[openEHR-EHR-INSTRUCTION.test_all_types.v1]/activities[at0001]/)
- **ECC-VAL-056** Validate DV_QUANTITY — open (`val/dv-quantity-open`, json): DV_QUANTITY with magnitude (RM present): expected accepted (composition_create.yaml 201), got 422 ({"message":"1 validation error(s)","validationErrors":["/content[openEHR-EHR-SECTION.test_all_types.v1]/items[at0001]/items[at0002]/items[openEHR-EHR-INSTRUCTION.test_all_types.v1]/activities[at0001]/)
- **ECC-VAL-057** Validate DV_QUANTITY — property (`val/dv-quantity-property`, json): units mg matches property mass openehr::124 (accepted): expected accepted (composition_create.yaml 201), got 422 ({"message":"1 validation error(s)","validationErrors":["/content[openEHR-EHR-SECTION.test_all_types.v1]/items[at0001]/items[at0002]/items[openEHR-EHR-INSTRUCTION.test_all_types.v1]/activities[at0001]/)
- **ECC-VAL-058** Validate DV_QUANTITY — property units (`val/dv-quantity-property-units`, json): DV_QUANTITY units 'mg' in [mg,kg] (accepted): expected accepted (composition_create.yaml 201), got 422 ({"message":"1 validation error(s)","validationErrors":["/content[openEHR-EHR-SECTION.test_all_types.v1]/items[at0001]/items[at0002]/items[openEHR-EHR-INSTRUCTION.test_all_types.v1]/activities[at0001]/)
- **ECC-VAL-060** Validate DV_PROPORTION — open (`val/dv-proportion-open`, json): DV_PROPORTION with numerator (RM present): expected accepted (composition_create.yaml 201), got 422 ({"message":"1 validation error(s)","validationErrors":["/content[openEHR-EHR-SECTION.test_all_types.v1]/items[at0001]/items[at0002]/items[openEHR-EHR-INSTRUCTION.test_all_types.v1]/activities[at0001]/)
- **ECC-VAL-061** Validate DV_PROPORTION — ratio (`val/dv-proportion-ratio`, json): type 0 in list {0} with RM-valid num/den (accepted): expected accepted (composition_create.yaml 201), got 422 ({"message":"1 validation error(s)","validationErrors":["/content[openEHR-EHR-SECTION.test_all_types.v1]/items[at0001]/items[at0002]/items[openEHR-EHR-INSTRUCTION.test_all_types.v1]/activities[at0001]/)
- **ECC-VAL-062** Validate DV_PROPORTION — unitary (`val/dv-proportion-unitary`, json): type 1 in list {1} with RM-valid num/den (accepted): expected accepted (composition_create.yaml 201), got 422 ({"message":"1 validation error(s)","validationErrors":["/content[openEHR-EHR-SECTION.test_all_types.v1]/items[at0001]/items[at0002]/items[openEHR-EHR-INSTRUCTION.test_all_types.v1]/activities[at0001]/)
- **ECC-VAL-063** Validate DV_PROPORTION — percent (`val/dv-proportion-percent`, json): type 2 in list {2} with RM-valid num/den (accepted): expected accepted (composition_create.yaml 201), got 422 ({"message":"1 validation error(s)","validationErrors":["/content[openEHR-EHR-SECTION.test_all_types.v1]/items[at0001]/items[at0002]/items[openEHR-EHR-INSTRUCTION.test_all_types.v1]/activities[at0001]/)
- **ECC-VAL-064** Validate DV_PROPORTION — fraction (`val/dv-proportion-fraction`, json): type 3 in list {3} with RM-valid num/den (accepted): expected accepted (composition_create.yaml 201), got 422 ({"message":"1 validation error(s)","validationErrors":["/content[openEHR-EHR-SECTION.test_all_types.v1]/items[at0001]/items[at0002]/items[openEHR-EHR-INSTRUCTION.test_all_types.v1]/activities[at0001]/)
- **ECC-VAL-065** Validate DV_PROPORTION — integer fraction (`val/dv-proportion-integer-fraction`, json): type 4 in list {4} with RM-valid num/den (accepted): expected accepted (composition_create.yaml 201), got 422 ({"message":"1 validation error(s)","validationErrors":["/content[openEHR-EHR-SECTION.test_all_types.v1]/items[at0001]/items[at0002]/items[openEHR-EHR-INSTRUCTION.test_all_types.v1]/activities[at0001]/)
- **ECC-VAL-067** Validate DV_PROPORTION — ratio range (`val/dv-proportion-ratio-range`, json): numerator 398.5 in range [0,1000] (accepted): expected accepted (composition_create.yaml 201), got 422 ({"message":"1 validation error(s)","validationErrors":["/content[openEHR-EHR-SECTION.test_all_types.v1]/items[at0001]/items[at0002]/items[openEHR-EHR-INSTRUCTION.test_all_types.v1]/activities[at0001]/)
- **ECC-VAL-068** Validate DV_INTERVAL<DV_COUNT> — open (`val/dv-interval-dv-count-open`, json): valid DV_INTERVAL lower<=upper (accepted): expected accepted (composition_create.yaml 201), got 422 ({"message":"1 validation error(s)","validationErrors":["/content[openEHR-EHR-SECTION.test_all_types.v1]/items[at0001]/items[at0002]/items[openEHR-EHR-INSTRUCTION.test_all_types.v1]/activities[at0001]/)
- **ECC-VAL-069** Validate DV_INTERVAL<DV_COUNT> — lower upper (`val/dv-interval-dv-count-lower-upper`, json): valid DV_INTERVAL lower<=upper (accepted): expected accepted (composition_create.yaml 201), got 422 ({"message":"1 validation error(s)","validationErrors":["/content[openEHR-EHR-SECTION.test_all_types.v1]/items[at0001]/items[at0002]/items[openEHR-EHR-INSTRUCTION.test_all_types.v1]/activities[at0001]/)
- **ECC-VAL-070** Validate DV_INTERVAL<DV_COUNT> — lower upper list (`val/dv-interval-dv-count-lower-upper-list`, json): valid DV_INTERVAL lower<=upper (accepted): expected accepted (composition_create.yaml 201), got 422 ({"message":"1 validation error(s)","validationErrors":["/content[openEHR-EHR-SECTION.test_all_types.v1]/items[at0001]/items[at0002]/items[openEHR-EHR-INSTRUCTION.test_all_types.v1]/activities[at0001]/)
- **ECC-VAL-071** Validate DV_INTERVAL<DV_QUANTITY> — open (`val/dv-interval-dv-quantity-open`, json): valid DV_INTERVAL lower<=upper (accepted): expected accepted (composition_create.yaml 201), got 422 ({"message":"1 validation error(s)","validationErrors":["/content[openEHR-EHR-SECTION.test_all_types.v1]/items[at0001]/items[at0002]/items[openEHR-EHR-INSTRUCTION.test_all_types.v1]/activities[at0001]/)
- **ECC-VAL-072** Validate DV_INTERVAL<DV_QUANTITY> — upper lower (`val/dv-interval-dv-quantity-upper-lower`, json): valid DV_INTERVAL lower<=upper (accepted): expected accepted (composition_create.yaml 201), got 422 ({"message":"1 validation error(s)","validationErrors":["/content[openEHR-EHR-SECTION.test_all_types.v1]/items[at0001]/items[at0002]/items[openEHR-EHR-INSTRUCTION.test_all_types.v1]/activities[at0001]/)
- **ECC-VAL-073** Validate DV_INTERVAL<DV_DATE_TIME> — open (`val/dv-interval-dv-date-time-open`, json): valid DV_INTERVAL lower<=upper (accepted): expected accepted (composition_create.yaml 201), got 422 ({"message":"1 validation error(s)","validationErrors":["/content[openEHR-EHR-SECTION.test_all_types.v1]/items[at0001]/items[at0002]/items[openEHR-EHR-INSTRUCTION.test_all_types.v1]/activities[at0001]/)
- **ECC-VAL-074** Validate DV_INTERVAL<DV_DATE_TIME> — lower upper constraint (`val/dv-interval-dv-date-time-lower-upper-constraint`, json): valid DV_INTERVAL lower<=upper (accepted): expected accepted (composition_create.yaml 201), got 422 ({"message":"1 validation error(s)","validationErrors":["/content[openEHR-EHR-SECTION.test_all_types.v1]/items[at0001]/items[at0002]/items[openEHR-EHR-INSTRUCTION.test_all_types.v1]/activities[at0001]/)
- **ECC-VAL-075** Validate DV_INTERVAL<DV_DATE_TIME> — lower upper range (`val/dv-interval-dv-date-time-lower-upper-range`, json): valid DV_INTERVAL lower<=upper (accepted): expected accepted (composition_create.yaml 201), got 422 ({"message":"1 validation error(s)","validationErrors":["/content[openEHR-EHR-SECTION.test_all_types.v1]/items[at0001]/items[at0002]/items[openEHR-EHR-INSTRUCTION.test_all_types.v1]/activities[at0001]/)
- **ECC-VAL-076** Validate DV_INTERVAL<DV_DATE> — open (`val/dv-interval-dv-date-open`, json): valid DV_INTERVAL lower<=upper (accepted): expected accepted (composition_create.yaml 201), got 422 ({"message":"1 validation error(s)","validationErrors":["/content[openEHR-EHR-SECTION.test_all_types.v1]/items[at0001]/items[at0002]/items[openEHR-EHR-INSTRUCTION.test_all_types.v1]/activities[at0001]/)
- **ECC-VAL-077** Validate DV_INTERVAL<DV_DATE> — lower upper constraint (`val/dv-interval-dv-date-lower-upper-constraint`, json): valid DV_INTERVAL lower<=upper (accepted): expected accepted (composition_create.yaml 201), got 422 ({"message":"1 validation error(s)","validationErrors":["/content[openEHR-EHR-SECTION.test_all_types.v1]/items[at0001]/items[at0002]/items[openEHR-EHR-INSTRUCTION.test_all_types.v1]/activities[at0001]/)
- **ECC-VAL-078** Validate DV_INTERVAL<DV_DATE> — lower upper range (`val/dv-interval-dv-date-lower-upper-range`, json): valid DV_INTERVAL lower<=upper (accepted): expected accepted (composition_create.yaml 201), got 422 ({"message":"1 validation error(s)","validationErrors":["/content[openEHR-EHR-SECTION.test_all_types.v1]/items[at0001]/items[at0002]/items[openEHR-EHR-INSTRUCTION.test_all_types.v1]/activities[at0001]/)
- **ECC-VAL-079** Validate DV_INTERVAL<DV_TIME> — open (`val/dv-interval-dv-time-open`, json): valid DV_INTERVAL lower<=upper (accepted): expected accepted (composition_create.yaml 201), got 422 ({"message":"1 validation error(s)","validationErrors":["/content[openEHR-EHR-SECTION.test_all_types.v1]/items[at0001]/items[at0002]/items[openEHR-EHR-INSTRUCTION.test_all_types.v1]/activities[at0001]/)
- **ECC-VAL-080** Validate DV_INTERVAL<DV_TIME> — lower upper constraint (`val/dv-interval-dv-time-lower-upper-constraint`, json): valid DV_INTERVAL lower<=upper (accepted): expected accepted (composition_create.yaml 201), got 422 ({"message":"1 validation error(s)","validationErrors":["/content[openEHR-EHR-SECTION.test_all_types.v1]/items[at0001]/items[at0002]/items[openEHR-EHR-INSTRUCTION.test_all_types.v1]/activities[at0001]/)
- **ECC-VAL-081** Validate DV_INTERVAL<DV_TIME> — lower upper range (`val/dv-interval-dv-time-lower-upper-range`, json): valid DV_INTERVAL lower<=upper (accepted): expected accepted (composition_create.yaml 201), got 422 ({"message":"1 validation error(s)","validationErrors":["/content[openEHR-EHR-SECTION.test_all_types.v1]/items[at0001]/items[at0002]/items[openEHR-EHR-INSTRUCTION.test_all_types.v1]/activities[at0001]/)
- **ECC-VAL-082** Validate DV_INTERVAL<DV_DURATION> — open (`val/dv-interval-dv-duration-open`, json): valid DV_INTERVAL lower<=upper (accepted): expected accepted (composition_create.yaml 201), got 422 ({"message":"1 validation error(s)","validationErrors":["/content[openEHR-EHR-SECTION.test_all_types.v1]/items[at0001]/items[at0002]/items[openEHR-EHR-INSTRUCTION.test_all_types.v1]/activities[at0001]/)
- **ECC-VAL-083** Validate DV_INTERVAL<DV_DURATION> — constraint (`val/dv-interval-dv-duration-constraint`, json): valid DV_INTERVAL lower<=upper (accepted): expected accepted (composition_create.yaml 201), got 422 ({"message":"1 validation error(s)","validationErrors":["/content[openEHR-EHR-SECTION.test_all_types.v1]/items[at0001]/items[at0002]/items[openEHR-EHR-INSTRUCTION.test_all_types.v1]/activities[at0001]/)
- **ECC-VAL-084** Validate DV_INTERVAL<DV_DURATION> — range (`val/dv-interval-dv-duration-range`, json): valid DV_INTERVAL lower<=upper (accepted): expected accepted (composition_create.yaml 201), got 422 ({"message":"1 validation error(s)","validationErrors":["/content[openEHR-EHR-SECTION.test_all_types.v1]/items[at0001]/items[at0002]/items[openEHR-EHR-INSTRUCTION.test_all_types.v1]/activities[at0001]/)
- **ECC-VAL-085** Validate DV_INTERVAL<DV_ORDINAL> — open (`val/dv-interval-dv-ordinal-open`, json): valid DV_INTERVAL lower<=upper (accepted): expected accepted (composition_create.yaml 201), got 422 ({"message":"1 validation error(s)","validationErrors":["/content[openEHR-EHR-SECTION.test_all_types.v1]/items[at0001]/items[at0002]/items[openEHR-EHR-INSTRUCTION.test_all_types.v1]/activities[at0001]/)
- **ECC-VAL-086** Validate DV_INTERVAL<DV_ORDINAL> — constraint (`val/dv-interval-dv-ordinal-constraint`, json): valid DV_INTERVAL lower<=upper (accepted): expected accepted (composition_create.yaml 201), got 422 ({"message":"1 validation error(s)","validationErrors":["/content[openEHR-EHR-SECTION.test_all_types.v1]/items[at0001]/items[at0002]/items[openEHR-EHR-INSTRUCTION.test_all_types.v1]/activities[at0001]/)
- **ECC-VAL-087** Validate DV_INTERVAL<DV_SCALE> — open (`val/dv-interval-dv-scale-open`, json): valid DV_INTERVAL lower<=upper (accepted): expected accepted (composition_create.yaml 201), got 422 ({"message":"1 validation error(s)","validationErrors":["/content[openEHR-EHR-SECTION.test_all_types.v1]/items[at0001]/items[at0002]/items[openEHR-EHR-INSTRUCTION.test_all_types.v1]/activities[at0001]/)
- **ECC-VAL-088** Validate DV_INTERVAL<DV_SCALE> — constraint (`val/dv-interval-dv-scale-constraint`, json): valid DV_INTERVAL lower<=upper (accepted): expected accepted (composition_create.yaml 201), got 422 ({"message":"1 validation error(s)","validationErrors":["/content[openEHR-EHR-SECTION.test_all_types.v1]/items[at0001]/items[at0002]/items[openEHR-EHR-INSTRUCTION.test_all_types.v1]/activities[at0001]/)
- **ECC-VAL-089** Validate DV_INTERVAL<DV_PROPORTION> — open (`val/dv-interval-dv-proportion-open`, json): valid DV_INTERVAL lower<=upper (accepted): expected accepted (composition_create.yaml 201), got 422 ({"message":"1 validation error(s)","validationErrors":["/content[openEHR-EHR-SECTION.test_all_types.v1]/items[at0001]/items[at0002]/items[openEHR-EHR-INSTRUCTION.test_all_types.v1]/activities[at0001]/)
- **ECC-VAL-090** Validate DV_INTERVAL<DV_PROPORTION> — ratio (`val/dv-interval-dv-proportion-ratio`, json): valid DV_INTERVAL lower<=upper (accepted): expected accepted (composition_create.yaml 201), got 422 ({"message":"1 validation error(s)","validationErrors":["/content[openEHR-EHR-SECTION.test_all_types.v1]/items[at0001]/items[at0002]/items[openEHR-EHR-INSTRUCTION.test_all_types.v1]/activities[at0001]/)
- **ECC-VAL-091** Validate DV_INTERVAL<DV_PROPORTION> — unitary (`val/dv-interval-dv-proportion-unitary`, json): valid DV_INTERVAL lower<=upper (accepted): expected accepted (composition_create.yaml 201), got 422 ({"message":"1 validation error(s)","validationErrors":["/content[openEHR-EHR-SECTION.test_all_types.v1]/items[at0001]/items[at0002]/items[openEHR-EHR-INSTRUCTION.test_all_types.v1]/activities[at0001]/)
- **ECC-VAL-092** Validate DV_INTERVAL<DV_PROPORTION> — percentage (`val/dv-interval-dv-proportion-percentage`, json): valid DV_INTERVAL lower<=upper (accepted): expected accepted (composition_create.yaml 201), got 422 ({"message":"1 validation error(s)","validationErrors":["/content[openEHR-EHR-SECTION.test_all_types.v1]/items[at0001]/items[at0002]/items[openEHR-EHR-INSTRUCTION.test_all_types.v1]/activities[at0001]/)
- **ECC-VAL-093** Validate DV_INTERVAL<DV_PROPORTION> — fraction (`val/dv-interval-dv-proportion-fraction`, json): valid DV_INTERVAL lower<=upper (accepted): expected accepted (composition_create.yaml 201), got 422 ({"message":"1 validation error(s)","validationErrors":["/content[openEHR-EHR-SECTION.test_all_types.v1]/items[at0001]/items[at0002]/items[openEHR-EHR-INSTRUCTION.test_all_types.v1]/activities[at0001]/)
- **ECC-VAL-094** Validate DV_INTERVAL<DV_PROPORTION> — integer fraction (`val/dv-interval-dv-proportion-integer-fraction`, json): valid DV_INTERVAL lower<=upper (accepted): expected accepted (composition_create.yaml 201), got 422 ({"message":"1 validation error(s)","validationErrors":["/content[openEHR-EHR-SECTION.test_all_types.v1]/items[at0001]/items[at0002]/items[openEHR-EHR-INSTRUCTION.test_all_types.v1]/activities[at0001]/)
- **ECC-VAL-095** Validate DV_INTERVAL<DV_PROPORTION> — ratio range (`val/dv-interval-dv-proportion-ratio-range`, json): valid DV_INTERVAL lower<=upper (accepted): expected accepted (composition_create.yaml 201), got 422 ({"message":"1 validation error(s)","validationErrors":["/content[openEHR-EHR-SECTION.test_all_types.v1]/items[at0001]/items[at0002]/items[openEHR-EHR-INSTRUCTION.test_all_types.v1]/activities[at0001]/)
- **ECC-VAL-096** Validate DV_DURATION — open (`val/dv-duration-open`, json): DV_DURATION with value (RM present): expected accepted (composition_create.yaml 201), got 422 ({"message":"1 validation error(s)","validationErrors":["/content[openEHR-EHR-SECTION.test_all_types.v1]/items[at0001]/items[at0002]/items[openEHR-EHR-INSTRUCTION.test_all_types.v1]/activities[at0001]/)
- **ECC-VAL-097** Validate DV_DURATION — fields (`val/dv-duration-fields`, json): DV_DURATION base value satisfies the constraint (accepted): expected accepted (composition_create.yaml 201), got 422 ({"message":"1 validation error(s)","validationErrors":["/content[openEHR-EHR-SECTION.test_all_types.v1]/items[at0001]/items[at0002]/items[openEHR-EHR-INSTRUCTION.test_all_types.v1]/activities[at0001]/)
- **ECC-VAL-098** Validate DV_DURATION — range (`val/dv-duration-range`, json): DV_DURATION base value satisfies the constraint (accepted): expected accepted (composition_create.yaml 201), got 422 ({"message":"1 validation error(s)","validationErrors":["/content[openEHR-EHR-SECTION.test_all_types.v1]/items[at0001]/items[at0002]/items[openEHR-EHR-INSTRUCTION.test_all_types.v1]/activities[at0001]/)
- **ECC-VAL-099** Validate DV_DURATION — fields range (`val/dv-duration-fields-range`, json): DV_DURATION base value satisfies the constraint (accepted): expected accepted (composition_create.yaml 201), got 422 ({"message":"1 validation error(s)","validationErrors":["/content[openEHR-EHR-SECTION.test_all_types.v1]/items[at0001]/items[at0002]/items[openEHR-EHR-INSTRUCTION.test_all_types.v1]/activities[at0001]/)
- **ECC-VAL-100** Validate DV_TIME — open (`val/dv-time-open`, json): DV_TIME with value (RM present): expected accepted (composition_create.yaml 201), got 422 ({"message":"1 validation error(s)","validationErrors":["/content[openEHR-EHR-SECTION.test_all_types.v1]/items[at0001]/items[at0002]/items[openEHR-EHR-INSTRUCTION.test_all_types.v1]/activities[at0001]/)
- **ECC-VAL-101** Validate DV_TIME — constraint (`val/dv-time-constraint`, json): DV_TIME base value satisfies the constraint (accepted): expected accepted (composition_create.yaml 201), got 422 ({"message":"1 validation error(s)","validationErrors":["/content[openEHR-EHR-SECTION.test_all_types.v1]/items[at0001]/items[at0002]/items[openEHR-EHR-INSTRUCTION.test_all_types.v1]/activities[at0001]/)
- **ECC-VAL-102** Validate DV_TIME — range (`val/dv-time-range`, json): DV_TIME base value satisfies the constraint (accepted): expected accepted (composition_create.yaml 201), got 422 ({"message":"1 validation error(s)","validationErrors":["/content[openEHR-EHR-SECTION.test_all_types.v1]/items[at0001]/items[at0002]/items[openEHR-EHR-INSTRUCTION.test_all_types.v1]/activities[at0001]/)
- **ECC-VAL-103** Validate DV_DATE — open (`val/dv-date-open`, json): DV_DATE with value (RM present): expected accepted (composition_create.yaml 201), got 422 ({"message":"1 validation error(s)","validationErrors":["/content[openEHR-EHR-SECTION.test_all_types.v1]/items[at0001]/items[at0002]/items[openEHR-EHR-INSTRUCTION.test_all_types.v1]/activities[at0001]/)
- **ECC-VAL-104** Validate DV_DATE — constraint (`val/dv-date-constraint`, json): DV_DATE base value satisfies the constraint (accepted): expected accepted (composition_create.yaml 201), got 422 ({"message":"1 validation error(s)","validationErrors":["/content[openEHR-EHR-SECTION.test_all_types.v1]/items[at0001]/items[at0002]/items[openEHR-EHR-INSTRUCTION.test_all_types.v1]/activities[at0001]/)
- **ECC-VAL-105** Validate DV_DATE — range (`val/dv-date-range`, json): DV_DATE base value satisfies the constraint (accepted): expected accepted (composition_create.yaml 201), got 422 ({"message":"1 validation error(s)","validationErrors":["/content[openEHR-EHR-SECTION.test_all_types.v1]/items[at0001]/items[at0002]/items[openEHR-EHR-INSTRUCTION.test_all_types.v1]/activities[at0001]/)
- **ECC-VAL-106** Validate DV_DATE_TIME — open (`val/dv-date-time-open`, json): DV_DATE_TIME with value (RM present): expected accepted (composition_create.yaml 201), got 422 ({"message":"1 validation error(s)","validationErrors":["/content[openEHR-EHR-SECTION.test_all_types.v1]/items[at0001]/items[at0002]/items[openEHR-EHR-INSTRUCTION.test_all_types.v1]/activities[at0001]/)
- **ECC-VAL-107** Validate DV_DATE_TIME — constraint (`val/dv-date-time-constraint`, json): DV_DATE_TIME full timestamp matches yyyy-mm-ddTHH:MM:SS (accepted): expected accepted (composition_create.yaml 201), got 422 ({"message":"1 validation error(s)","validationErrors":["/content[openEHR-EHR-SECTION.test_all_types.v1]/items[at0001]/items[at0002]/items[openEHR-EHR-INSTRUCTION.test_all_types.v1]/activities[at0001]/)
- **ECC-VAL-108** Validate DV_DATE_TIME — range (`val/dv-date-time-range`, json): DV_DATE_TIME base value satisfies the constraint (accepted): expected accepted (composition_create.yaml 201), got 422 ({"message":"1 validation error(s)","validationErrors":["/content[openEHR-EHR-SECTION.test_all_types.v1]/items[at0001]/items[at0002]/items[openEHR-EHR-INSTRUCTION.test_all_types.v1]/activities[at0001]/)
- **ECC-VAL-109** Validate DV_PARSABLE — open (`val/dv-parsable-open`, json): DV_PARSABLE with value (RM present): expected accepted (composition_create.yaml 201), got 422 ({"message":"1 validation error(s)","validationErrors":["/content[openEHR-EHR-SECTION.test_all_types.v1]/items[at0001]/items[at0002]/items[openEHR-EHR-INSTRUCTION.test_all_types.v1]/activities[at0001]/)
- **ECC-VAL-110** Validate DV_PARSABLE — value formalism (`val/dv-parsable-value-formalism`, json): formalism ISO8601 in list (accepted): expected accepted (composition_create.yaml 201), got 422 ({"message":"1 validation error(s)","validationErrors":["/content[openEHR-EHR-SECTION.test_all_types.v1]/items[at0001]/items[at0002]/items[openEHR-EHR-INSTRUCTION.test_all_types.v1]/activities[at0001]/)
- **ECC-VAL-111** Validate DV_MULTIMEDIA — open (`val/dv-multimedia-open`, json): DV_MULTIMEDIA with media_type (RM present): expected accepted (composition_create.yaml 201), got 422 ({"message":"1 validation error(s)","validationErrors":["/content[openEHR-EHR-SECTION.test_all_types.v1]/items[at0001]/items[at0002]/items[openEHR-EHR-INSTRUCTION.test_all_types.v1]/activities[at0001]/)
- **ECC-VAL-112** Validate DV_MULTIMEDIA — media type (`val/dv-multimedia-media-type`, json): media_type image/png in list (accepted): expected accepted (composition_create.yaml 201), got 422 ({"message":"1 validation error(s)","validationErrors":["/content[openEHR-EHR-SECTION.test_all_types.v1]/items[at0001]/items[at0002]/items[openEHR-EHR-INSTRUCTION.test_all_types.v1]/activities[at0001]/)
- **ECC-VAL-113** Validate DV_URI — open (`val/dv-uri-open`, json): DV_URI with value (RM present): expected accepted (composition_create.yaml 201), got 422 ({"message":"1 validation error(s)","validationErrors":["/content[openEHR-EHR-SECTION.test_all_types.v1]/items[at0001]/items[at0002]/items[openEHR-EHR-INSTRUCTION.test_all_types.v1]/activities[at0001]/)
- **ECC-VAL-114** Validate DV_URI — pattern (`val/dv-uri-pattern`, json): URI http://ok matches pattern (accepted): expected accepted (composition_create.yaml 201), got 422 ({"message":"1 validation error(s)","validationErrors":["/content[openEHR-EHR-SECTION.test_all_types.v1]/items[at0001]/items[at0002]/items[openEHR-EHR-INSTRUCTION.test_all_types.v1]/activities[at0001]/)
- **ECC-VAL-115** Validate DV_URI — list (`val/dv-uri-list`, json): URI http://ok in list (accepted): expected accepted (composition_create.yaml 201), got 422 ({"message":"1 validation error(s)","validationErrors":["/content[openEHR-EHR-SECTION.test_all_types.v1]/items[at0001]/items[at0002]/items[openEHR-EHR-INSTRUCTION.test_all_types.v1]/activities[at0001]/)
- **ECC-VAL-116** Validate DV_EHR_URI — open (`val/dv-ehr-uri-open`, json): DV_EHR_URI with value (accepted): expected accepted (composition_create.yaml 201), got 422 ({"message":"1 validation error(s)","validationErrors":["/content[openEHR-EHR-SECTION.test_all_types.v1]/items[at0001]/items[at0002]/items[openEHR-EHR-INSTRUCTION.test_all_types.v1]/activities[at0001]/)
- **ECC-VAL-117** Validate DV_EHR_URI — pattern (`val/dv-ehr-uri-pattern`, json): ehr://x matches pattern (accepted): expected accepted (composition_create.yaml 201), got 422 ({"message":"1 validation error(s)","validationErrors":["/content[openEHR-EHR-SECTION.test_all_types.v1]/items[at0001]/items[at0002]/items[openEHR-EHR-INSTRUCTION.test_all_types.v1]/activities[at0001]/)
- **ECC-VAL-118** Validate DV_EHR_URI — list (`val/dv-ehr-uri-list`, json): ehr://ok in list (accepted): expected accepted (composition_create.yaml 201), got 422 ({"message":"1 validation error(s)","validationErrors":["/content[openEHR-EHR-SECTION.test_all_types.v1]/items[at0001]/items[at0002]/items[openEHR-EHR-INSTRUCTION.test_all_types.v1]/activities[at0001]/)
- **ECC-SIG-001** Version signing — digest present (`sig/digest-present`, xml): expected status 200, got 406 (body: {"error":"Not Acceptable","message":"not acceptable: canonical XML for this response is available once typed payloads land (P12); request application/json"})

## 2. Scope of test

| Field | Value |
|---|---|
| Profiles requested | all |
| Data formats | json, xml |
| Catalogue (active cases) | 310 |
| Executed | 318 |
| Passed | 211 |
| Failed | 106 |

## 3. Detailed test report

| ECC id | Capability | Format | Data sets | Result |
|---|---|---|--:|---|
| ECC-EHR-001 | EhrOperations | json | 1/1 | PASS |
| ECC-EHR-002 | EhrOperations | json | 1/1 | PASS |
| ECC-EHR-003 | EhrOperations | json | 1/1 | PASS |
| ECC-EHR-004 | EhrOperations | json | 1/1 | PASS |
| ECC-EHR-005 | EhrOperations | json | 16/16 | PASS |
| ECC-EHR-006 | EhrOperations | json | 1/1 | PASS |
| ECC-EHR-007 | EhrOperations | json | 1/1 | PASS |
| ECC-EHR-008 | EhrOperations | json | 1/1 | PASS |
| ECC-EHR-009 | EhrOperations | json | 1/1 | PASS |
| ECC-EHR-010 | EhrOperations | json | 1/1 | PASS |
| ECC-EHR-011 | EhrOperations | json | 1/1 | PASS |
| ECC-STA-001 | EhrStatus | json | 1/1 | PASS |
| ECC-STA-002 | EhrStatus | json | 1/1 | PASS |
| ECC-STA-003 | EhrStatus | json | 1/1 | PASS |
| ECC-STA-004 | EhrStatus | json | 1/1 | PASS |
| ECC-STA-005 | EhrStatus | json | 1/1 | PASS |
| ECC-STA-006 | EhrStatus | json | 1/1 | PASS |
| ECC-STA-007 | EhrStatus | json | 1/1 | PASS |
| ECC-STA-008 | EhrStatus | json | 1/1 | PASS |
| ECC-STA-009 | EhrStatus | json | 1/1 | PASS |
| ECC-STA-010 | EhrStatus | json | 1/1 | PASS |
| ECC-EHR-012 | EhrOperations | json | 11/11 | PASS |
| ECC-COM-001 | CompositionOps | json | 1/1 | PASS |
| ECC-COM-001 | CompositionOps | xml | 1/1 | PASS |
| ECC-COM-002 | CompositionOps | json | 1/1 | PASS |
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
| ECC-COM-022 | CompositionOps | json | 1/1 | PASS |
| ECC-COM-022 | CompositionOps | xml | 0/0 | **FAIL** |
| ECC-COM-023 | CompositionOps | json | 1/1 | PASS |
| ECC-COM-024 | CompositionOps | json | 1/1 | PASS |
| ECC-COM-025 | CompositionOps | json | 1/1 | PASS |
| ECC-COM-026 | CompositionOps | json | 1/1 | PASS |
| ECC-COM-027 | CompositionOps | json | 1/1 | PASS |
| ECC-COM-028 | CompositionOps | json | 1/1 | PASS |
| ECC-COM-029 | CompositionOps | json | 1/1 | PASS |
| ECC-COM-030 | CompositionOps | json | 1/1 | PASS |
| ECC-COM-031 | CompositionOps | json | 1/1 | PASS |
| ECC-CTB-001 | ChangeSets | json | 1/1 | PASS |
| ECC-CTB-002 | ChangeSets | json | 1/1 | PASS |
| ECC-CTB-003 | ChangeSets | json | 1/1 | PASS |
| ECC-CTB-004 | ChangeSets | json | 1/1 | PASS |
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
| ECC-CTB-027 | ChangeSets | json | 0/0 | **FAIL** |
| ECC-CTB-028 | ChangeSets | json | 0/0 | **FAIL** |
| ECC-CTB-029 | ChangeSets | json | 0/0 | **FAIL** |
| ECC-CTB-030 | ChangeSets | json | 0/0 | **FAIL** |
| ECC-CTB-031 | ChangeSets | json | 0/0 | **FAIL** |
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
| ECC-DIR-033 | DirectoryOps | json | 1/1 | PASS |
| ECC-DIR-034 | DirectoryOps | json | 0/0 | **FAIL** |
| ECC-DIR-035 | DirectoryOps | json | 1/1 | PASS |
| ECC-DIR-036 | DirectoryOps | json | 1/1 | PASS |
| ECC-DIR-037 | DirectoryOps | json | 1/1 | PASS |
| ECC-TPL-001 | Adl14OptProvisioning | json | 1/1 | PASS |
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
| ECC-TPL-013 | Adl14OptProvisioning | json | 1/1 | PASS |
| ECC-TPL-014 | Adl14OptProvisioning | json | 0/0 | **FAIL** |
| ECC-TPL-015 | Adl14OptProvisioning | json | 0/0 | **FAIL** |
| ECC-TPL-016 | Adl14OptProvisioning | json | 1/1 | PASS |
| ECC-SQR-001 | QueryProvisioning | json | 1/1 | PASS |
| ECC-SQR-002 | QueryProvisioning | json | 1/1 | PASS |
| ECC-SQR-003 | QueryProvisioning | json | 1/1 | PASS |
| ECC-SQR-004 | QueryProvisioning | json | 0/0 | **FAIL** |
| ECC-SQR-005 | QueryProvisioning | json | 0/0 | **FAIL** |
| ECC-SQR-006 | QueryProvisioning | json | 0/0 | **FAIL** |
| ECC-SQR-007 | QueryProvisioning | json | 0/0 | **FAIL** |
| ECC-QRY-001 | AqlBasic | json | 1/1 | PASS |
| ECC-QRY-002 | AqlBasic | json | 1/1 | PASS |
| ECC-QRY-003 | AqlBasic | json | 1/1 | PASS |
| ECC-QRY-004 | AqlBasic | json | 1/1 | PASS |
| ECC-QRY-005 | AqlBasic | json | 2/2 | PASS |
| ECC-QRY-006 | AqlBasic | json | 0/0 | **FAIL** |
| ECC-QRY-007 | AqlBasic | json | 18/18 | PASS |
| ECC-QRY-008 | AqlBasic | json | 11/11 | PASS |
| ECC-QRY-009 | AqlBasic | json | 0/0 | **FAIL** |
| ECC-QRY-010 | AqlBasic | json | 0/0 | **FAIL** |
| ECC-QRY-011 | AqlBasic | json | 0/0 | **FAIL** |
| ECC-QRY-012 | AqlBasic | json | 1/1 | PASS |
| ECC-QRY-013 | AqlBasic | json | 0/0 | **FAIL** |
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
| ECC-DEM-005 | DemographicApi | json | 0/0 | **FAIL** |
| ECC-DEM-006 | DemographicApi | json | 0/0 | **FAIL** |
| ECC-DEM-007 | DemographicApi | json | 1/1 | PASS |
| ECC-DEM-008 | DemographicApi | json | 1/1 | PASS |
| ECC-DEM-009 | DemographicApi | json | 1/1 | PASS |
| ECC-DEM-010 | DemographicApi | json | 1/1 | PASS |
| ECC-DEM-011 | DemographicApi | json | 0/0 | **FAIL** |
| ECC-DEM-012 | DemographicApi | json | 1/1 | PASS |
| ECC-DEM-013 | DemographicApi | json | 1/1 | PASS |
| ECC-DEM-014 | DemographicApi | json | 0/0 | **FAIL** |
| ECC-DEM-015 | DemographicApi | json | 1/1 | PASS |
| ECC-DEM-016 | DemographicApi | json | 1/1 | PASS |
| ECC-DEM-017 | DemographicApi | json | 0/0 | **FAIL** |
| ECC-DEM-018 | DemographicApi | json | 1/1 | PASS |
| ECC-DEM-019 | DemographicApi | json | 1/1 | PASS |
| ECC-DEM-020 | DemographicApi | json | 0/0 | **FAIL** |
| ECC-DEM-021 | DemographicApi | json | 1/1 | PASS |
| ECC-DEM-022 | DemographicApi | json | 1/1 | PASS |
| ECC-DEM-023 | DemographicApi | json | 1/1 | PASS |
| ECC-DEM-024 | DemographicApi | json | 1/1 | PASS |
| ECC-VAL-001 | ArchetypeValidation | json | 6/6 | PASS |
| ECC-VAL-002 | ArchetypeValidation | json | 6/6 | PASS |
| ECC-VAL-003 | ArchetypeValidation | json | 6/6 | PASS |
| ECC-VAL-004 | ArchetypeValidation | json | 6/6 | PASS |
| ECC-VAL-005 | ArchetypeValidation | json | 6/6 | PASS |
| ECC-VAL-006 | ArchetypeValidation | json | 6/6 | PASS |
| ECC-VAL-007 | ArchetypeValidation | json | 6/6 | PASS |
| ECC-VAL-008 | ArchetypeValidation | json | 6/6 | PASS |
| ECC-VAL-009 | ArchetypeValidation | json | 6/6 | PASS |
| ECC-VAL-010 | ArchetypeValidation | json | 6/6 | PASS |
| ECC-VAL-011 | ArchetypeValidation | json | 6/6 | PASS |
| ECC-VAL-012 | ArchetypeValidation | json | 6/6 | PASS |
| ECC-VAL-013 | ArchetypeValidation | json | 2/2 | PASS |
| ECC-VAL-014 | ArchetypeValidation | json | 2/2 | PASS |
| ECC-VAL-015 | ArchetypeValidation | json | 2/2 | PASS |
| ECC-VAL-016 | ArchetypeValidation | json | 2/2 | PASS |
| ECC-VAL-017 | ArchetypeValidation | json | 3/3 | PASS |
| ECC-VAL-018 | ArchetypeValidation | json | 3/3 | PASS |
| ECC-VAL-019 | ArchetypeValidation | json | 3/3 | PASS |
| ECC-VAL-020 | ArchetypeValidation | json | 3/3 | PASS |
| ECC-VAL-021 | ArchetypeValidation | json | 3/3 | PASS |
| ECC-VAL-022 | ArchetypeValidation | json | 3/3 | PASS |
| ECC-VAL-023 | ArchetypeValidation | json | 3/3 | PASS |
| ECC-VAL-024 | ArchetypeValidation | json | 3/3 | PASS |
| ECC-VAL-025 | ArchetypeValidation | json | 3/3 | PASS |
| ECC-VAL-026 | ArchetypeValidation | json | 3/3 | PASS |
| ECC-VAL-027 | ArchetypeValidation | json | 3/3 | PASS |
| ECC-VAL-028 | ArchetypeValidation | json | 3/3 | PASS |
| ECC-VAL-029 | ArchetypeValidation | json | 2/2 | PASS |
| ECC-VAL-030 | ArchetypeValidation | json | 2/2 | PASS |
| ECC-VAL-031 | ArchetypeValidation | json | 1/1 | PASS |
| ECC-VAL-032 | ArchetypeValidation | json | 2/2 | PASS |
| ECC-VAL-033 | ArchetypeValidation | json | 2/2 | PASS |
| ECC-VAL-034 | ArchetypeValidation | json | 2/2 | PASS |
| ECC-VAL-035 | ArchetypeValidation | json | 2/2 | PASS |
| ECC-VAL-036 | ArchetypeValidation | json | 0/0 | **FAIL** |
| ECC-VAL-037 | ArchetypeValidation | json | 0/0 | **FAIL** |
| ECC-VAL-038 | ArchetypeValidation | json | 0/0 | **FAIL** |
| ECC-VAL-039 | ArchetypeValidation | json | 0/0 | **FAIL** |
| ECC-VAL-040 | ArchetypeValidation | json | 0/0 | **FAIL** |
| ECC-VAL-041 | ArchetypeValidation | json | 0/0 | **FAIL** |
| ECC-VAL-042 | ArchetypeValidation | json | 0/0 | **FAIL** |
| ECC-VAL-043 | ArchetypeValidation | json | 0/0 | **FAIL** |
| ECC-VAL-044 | ArchetypeValidation | json | 0/0 | **FAIL** |
| ECC-VAL-045 | ArchetypeValidation | json | 0/0 | **FAIL** |
| ECC-VAL-046 | ArchetypeValidation | json | 0/0 | **FAIL** |
| ECC-VAL-047 | ArchetypeValidation | json | 0/0 | **FAIL** |
| ECC-VAL-048 | ArchetypeValidation | json | 0/0 | **FAIL** |
| ECC-VAL-049 | ArchetypeValidation | json | 0/0 | **FAIL** |
| ECC-VAL-050 | ArchetypeValidation | json | 0/0 | **FAIL** |
| ECC-VAL-051 | ArchetypeValidation | json | 0/0 | **FAIL** |
| ECC-VAL-052 | ArchetypeValidation | json | 0/0 | **FAIL** |
| ECC-VAL-053 | ArchetypeValidation | json | 0/0 | **FAIL** |
| ECC-VAL-054 | ArchetypeValidation | json | 0/0 | **FAIL** |
| ECC-VAL-055 | ArchetypeValidation | json | 0/0 | **FAIL** |
| ECC-VAL-056 | ArchetypeValidation | json | 0/0 | **FAIL** |
| ECC-VAL-057 | ArchetypeValidation | json | 0/0 | **FAIL** |
| ECC-VAL-058 | ArchetypeValidation | json | 0/0 | **FAIL** |
| ECC-VAL-059 | ArchetypeValidation | json | 3/3 | PASS |
| ECC-VAL-060 | ArchetypeValidation | json | 0/0 | **FAIL** |
| ECC-VAL-061 | ArchetypeValidation | json | 0/0 | **FAIL** |
| ECC-VAL-062 | ArchetypeValidation | json | 0/0 | **FAIL** |
| ECC-VAL-063 | ArchetypeValidation | json | 0/0 | **FAIL** |
| ECC-VAL-064 | ArchetypeValidation | json | 0/0 | **FAIL** |
| ECC-VAL-065 | ArchetypeValidation | json | 0/0 | **FAIL** |
| ECC-VAL-066 | ArchetypeValidation | json | 2/2 | PASS |
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
| ECC-VAL-096 | ArchetypeValidation | json | 0/0 | **FAIL** |
| ECC-VAL-097 | ArchetypeValidation | json | 0/0 | **FAIL** |
| ECC-VAL-098 | ArchetypeValidation | json | 0/0 | **FAIL** |
| ECC-VAL-099 | ArchetypeValidation | json | 0/0 | **FAIL** |
| ECC-VAL-100 | ArchetypeValidation | json | 0/0 | **FAIL** |
| ECC-VAL-101 | ArchetypeValidation | json | 0/0 | **FAIL** |
| ECC-VAL-102 | ArchetypeValidation | json | 0/0 | **FAIL** |
| ECC-VAL-103 | ArchetypeValidation | json | 0/0 | **FAIL** |
| ECC-VAL-104 | ArchetypeValidation | json | 0/0 | **FAIL** |
| ECC-VAL-105 | ArchetypeValidation | json | 0/0 | **FAIL** |
| ECC-VAL-106 | ArchetypeValidation | json | 0/0 | **FAIL** |
| ECC-VAL-107 | ArchetypeValidation | json | 0/0 | **FAIL** |
| ECC-VAL-108 | ArchetypeValidation | json | 0/0 | **FAIL** |
| ECC-VAL-109 | ArchetypeValidation | json | 0/0 | **FAIL** |
| ECC-VAL-110 | ArchetypeValidation | json | 0/0 | **FAIL** |
| ECC-VAL-111 | ArchetypeValidation | json | 0/0 | **FAIL** |
| ECC-VAL-112 | ArchetypeValidation | json | 0/0 | **FAIL** |
| ECC-VAL-113 | ArchetypeValidation | json | 0/0 | **FAIL** |
| ECC-VAL-114 | ArchetypeValidation | json | 0/0 | **FAIL** |
| ECC-VAL-115 | ArchetypeValidation | json | 0/0 | **FAIL** |
| ECC-VAL-116 | ArchetypeValidation | json | 0/0 | **FAIL** |
| ECC-VAL-117 | ArchetypeValidation | json | 0/0 | **FAIL** |
| ECC-VAL-118 | ArchetypeValidation | json | 0/0 | **FAIL** |
| ECC-SIG-001 | Signing | json | 1/1 | PASS |
| ECC-SIG-001 | Signing | xml | 0/0 | **FAIL** |
| ECC-SIG-002 | Signing | json | 1/1 | PASS |
| ECC-SIG-003 | Signing | json | 4/4 | PASS |
| ECC-SIG-004 | Signing | json | 1/1 | PASS |
| ECC-SIG-005 | Signing | json | 0/0 | skipped |

## 4. Profile verdict (machine-computed, all-or-nothing)

### Core — not claimable

| Capability | Passed | Failed | Errored | Skipped | Verdict |
|---|--:|--:|--:|--:|---|
| Adl14ArchetypeProvisioning | 0 | 0 | 0 | 0 | fail |
| Adl14OptProvisioning | 14 | 2 | 0 | 0 | fail |
| EhrOperations | 12 | 0 | 0 | 0 | pass |
| EhrStatus | 10 | 0 | 0 | 0 | pass |
| CompositionOps | 37 | 1 | 0 | 0 | fail |
| ChangeSets | 26 | 5 | 0 | 0 | fail |
| Versioning | 0 | 0 | 0 | 0 | fail |
| ArchetypeValidation | 37 | 81 | 0 | 0 | fail |
| AnonymousEhrs | 0 | 0 | 0 | 0 | fail |

### Standard — not claimable

| Capability | Passed | Failed | Errored | Skipped | Verdict |
|---|--:|--:|--:|--:|---|
| Adl14ArchetypeProvisioning | 0 | 0 | 0 | 0 | fail |
| Adl14OptProvisioning | 14 | 2 | 0 | 0 | fail |
| EhrOperations | 12 | 0 | 0 | 0 | pass |
| EhrStatus | 10 | 0 | 0 | 0 | pass |
| CompositionOps | 37 | 1 | 0 | 0 | fail |
| ChangeSets | 26 | 5 | 0 | 0 | fail |
| Versioning | 0 | 0 | 0 | 0 | fail |
| ArchetypeValidation | 37 | 81 | 0 | 0 | fail |
| AnonymousEhrs | 0 | 0 | 0 | 0 | fail |
| DirectoryOps | 36 | 1 | 0 | 0 | fail |
| QueryProvisioning | 3 | 4 | 0 | 0 | fail |
| AqlBasic | 8 | 5 | 0 | 0 | fail |
| Signing | 4 | 1 | 0 | 1 | fail |

### Options — not claimable

| Capability | Passed | Failed | Errored | Skipped | Verdict |
|---|--:|--:|--:|--:|---|
| AdminApi | 6 | 0 | 0 | 0 | pass |
| DemographicApi | 18 | 6 | 0 | 0 | fail |

## 5. Deviations (skips), by reason

| Reason | Cases |
|---|--:|
| SutConfig: server not in `pgp` mode (needs a configured OpenPGP key); pgp-keyed self-host SUT wiring is a follow-up — digest cases prove the capability | 1 |
