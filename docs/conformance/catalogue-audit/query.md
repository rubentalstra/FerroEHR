# Catalogue audit — QUERY chapter

Issue #231 · audited 2026-07-24 · 15 cases · verdicts: 14 ok / 1 DEFECT (fixed in this audit) / 0 ambiguities

Chapter context: CNF `master11-func_tc_querying.adoc` carries the official case
inventory but every table's cells are xx/TBD (verified for smoke_test,
execute_stored_query-empty_db, execute_ad_hoc_query-empty_db/-loaded_db); flows
are SM/AQL/ITS-REST-derived per the stub-chapter posture. Several cases are
explicit catalogue additions re-adjudicated from the retired ECC corpus, each
flagged in its authoring comment. A runner-mechanics fact verified during this
audit and relevant to several cases: the `execute_ad_hoc_query` binding's
`ehr_id: "${ehr_id?}"` query parameter resolves from the case VarStore
(`exec/driver.rs::build_url` renders binding templates against vars), so the
minted `${ehr_id}` auto-scopes every query step in a case with `requires.ehr` —
the unscoped-looking `where_magnitude` is therefore collision-safe (dim 4/5).

| case | verdict | evidence | resolution |
|---|---|---|---|
| smoke_test | ok | master11 stub verified; simplest SELECT/FROM per QUERY master03; RESULT_SET per ITS-REST query 200_Query; AMB-6 fetch-explicit | none |
| execute_ad_hoc_query-empty_db | ok | Zero-rows ground scoped to a fresh minted EHR (the shared-SUT realization); count 0 derivable from AQL semantics + 200_Query | none |
| execute_ad_hoc_query-empty_db_bare_ehr | ok | Single-EHR scope (Request.md ehr_id param) makes the bare `FROM EHR e` projection return exactly the scoped EHR — the 2026-07-22 split's one-row ground is sound | none |
| execute_ad_hoc_query-empty_db_shapes | ok | Both fixture shapes (CONTAINS chain, WHERE predicate) are grammar-valid per master03; zero rows under the fresh-EHR scope; fixtures exist in MANIFEST | none |
| execute_ad_hoc_query-loaded_db | ok | master11 stub verified; bag equality (`match: set`) correct — no ORDER BY means unspecified row order (master03 §ORDER BY); `cnf.set.query-bp-10#all_uids_asc` exists | none |
| execute_ad_hoc_query-uid_projection | ok | master03 §Identified Paths (uid/value projection); ORDER BY totally orders distinct version ids → `match: ordered` sound | none |
| execute_ad_hoc_query-where_magnitude | ok | WHERE + query_parameters per master03; set member `magnitude_ge_140_by_uid` exists; auto-injected EHR scope prevents cross-case pollution (see header note) | none |
| execute_ad_hoc_query-distinct | ok | master03 §DISTINCT (lines 1055–1061: "removal of all such duplicate rows") — ten identical projected names collapse to one | none |
| execute_ad_hoc_query-order_by_limit | ok | master03 §ORDER BY + §LIMIT; distinct systolic magnitudes totally order the rows; AqlAdvanced (OPTIONS) tiering consistent with the capability matrix | none |
| execute_ad_hoc_query-fetch_with_top | ok | ITS-REST `query/Request.md`:25 verified verbatim: fetch "cannot be combined with AQL-top" — the one spec-MANDATED rejection in this family; AMB-6 carried | none |
| execute_ad_hoc_query-malformed_queries | ok | Grammar-unparseable text (missing FROM) → invalid_query per ITS-REST 400_Query; fixture exists | none |
| execute_ad_hoc_query-dialect_extensions | ok | TIMEWINDOW removal verified (QUERY master00 amendment record: SPECQUERY-20 "remove TIMEWINDOW"); clause-order fixture matches the grammar; correctly report_only under AMB-30 (rejection of dialect supersets is spec-silent) | none |
| execute_ad_hoc_query-terminology_expand_matches | ok | master03 §TERMINOLOGY usage (a) — expand as matches operand — verified; value-set CONTENT correctly flagged as engine extension (no openEHR spec governs the bundle); AqlTerminology (OPTIONS) guard | none |
| execute_ad_hoc_query-terminology_unsupported_forms | DEFECT — fixed | master03 §TERMINOLOGY read in full: the `operation` argument "is not restricted to any particular value" and `lookup`/`map` are LISTED example operations; the case (and two fixture manifests) claimed only expand/validate/subsumes are defined forms and expected rejection of 'lookup' usages — a restriction the spec disclaims. The genuinely groundable "outside the defined usage forms" ground is position-based: the three §TERMINOLOGY usage positions are all WHERE-clause forms, so TERMINOLOGY() in a SELECT column is outside them | The two operation-strictness rows removed (case + MANIFEST entries + fixture files); the SELECT-position row kept with corrected rationale; still report_only under AMB-30 — FIXED |
| execute_stored_query-empty_db | ok | master11 stub verified; cross-interface store (SM Pre_valid_query) then execute-by-name; zero rows under the fresh-EHR scope; `stored` = wire 200 per the store_query binding | none |

Checks common to the chapter:
- **Expectations (dim 2):** every expect/assert recomputed from QUERY master03/master00, ITS-REST query Request/200_Query/400_Query, and the SM query operations; the one non-derivable expectation found is the fixed defect above.
- **Fixtures (dim 4):** all `cnf.aql.*`, `cnf.set.query-bp-10`, `cnf.set.bp-10`, `cnf.opt.blood_pressure` keys verified in `corpus/MANIFEST.yaml` (incl. the `#member` projections used by ordered/set matches).
- **Ambiguity tags (dim 6):** AMB-6 (fetch explicit; TOP+fetch) and AMB-30 (dialect-superset rejection spec-silent, report_only) read and cover their tagged cases.
- Systemic note: the `terminology_unsupported_forms` comment referenced the nonexistent `REQUIREMENTS.md` (removed with the fix); the chapter-wide sweep of such references is tracked in the audit summary.

Post-fix machine floor: `cnf-runner validate` — 393 cases, 88 bindings, 0 findings.
