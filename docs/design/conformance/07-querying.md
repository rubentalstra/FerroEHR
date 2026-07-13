# Conformance register 07 — QUERY (AQL) component (`suites/query.rs`, `suites/query_golden.rs`): spec-first audit

W-10 area audit (read-only, 2026-07-13) of the **QUERY / AQL execution**
component of `tools/conformance`. Method is spec-first (README + owner ruling):
the spine is the governing CNF schedule chapter enumerated
test-case-by-test-case; the existing ECC cases are mapped **onto** each item
with a `file:line` verdict. Because master11 is a **TBD stub**, the spine records
the stub verbatim (cited) and then continues with the ECC-original corpus rows
derived from the AQL 1.1 spec + the vendored golden corpus — each such row
flagged **ECC-original (schedule stub)**. §3 lists the no-schedule-home cases;
§4 carries the G-rows, marking every edition-/version-specific assertion and the
ad-hoc-wire-scraping (W-3f) lesson.

**Spec oracles** (read before any change):

- `docs/specs/openehr/CNF/docs/platform_test_schedule/master11-func_tc_querying.adoc`
  — the `QUERY_SERVICE` (`I_QUERY_SERVICE`) test suite. **This chapter is a
  stub**: §Test Environment and §Test Data Sets are the literal `TBD`; it
  contains four real case headings + one literal placeholder heading
  ("`==== Test Case bbbb`" / `TBD`, master11:74); every real case's Flow cell is
  `xx`. Read whole.
- `docs/specs/openehr/CNF/docs/platform_test_schedule/master03-overview.adoc`
  — the test-case form and the RM-version note (§API Conformance: minimum RM
  1.0.2; supported version from the Conformance Statement).
- `docs/specs/openehr/CNF/docs/profiles/master03-profiles.adoc` —
  §Functional/Querying: **AQL basic = STANDARD**; **AQL advanced = OPTIONS**;
  **AQL & terminology = OPTIONS**. §REST APIs: **QUERY API = STANDARD**.
- AQL 1.1 grammar/spec (`crates/openehr-query/vendor/grammar/AqlParser.g4`,
  QUERY `master03-syntax`, `master00-amendment_record` SPECQUERY-20) — the true
  oracle for the corpus rows, since the schedule provides none.

**Mapped suites:** `tools/conformance/src/suites/query.rs` (13 ECC-QRY entries,
area `Qry`) + `query_golden.rs` (the golden normalizer, 7 documented suppression
rules). Golden-dialect adjudications (TIMEWINDOW, LIMIT-before-ORDER-BY) are
inline predicates in `query.rs`.

---

## 1. Verdict

master11 is a **schedule stub** (TBD environment/data sets, `xx` flows, a
literal "`Test Case bbbb — TBD`" heading), so it cannot be the oracle for query
behaviour. The honest spine is: the four real master11 case headings, plus the
profiles capabilities (AQL basic STANDARD / advanced OPTIONS / terminology
OPTIONS), plus the AQL 1.1 spec + the vendored golden corpus — which is exactly
how the suite is built (`query.rs:1-41`). On that basis the suite is **the
strongest real-query evidence available and honestly constructed**: the four
schedule headings have live ECC cases (`smoke_test`, `execute_ad_hoc_query-empty_db`,
`execute_stored_query-empty_db`, `execute_ad_hoc_query-loaded_db`), the corpus is
driven through a **documented normalizer** whose every suppression names its rule
(`query_golden.rs`, design §6), invalid queries are load-bearing negatives, and
the two golden-dialect defects (TIMEWINDOW removed in AQL 1.1; LIMIT-before-ORDER-BY
2019-dialect) are handled spec-first with citations rather than by editing a
golden. Three structural gaps dominate the rewrite: **(a)** every QRY case is
tagged `AqlBasic` (STANDARD) even though `AqlAdvanced` and `Terminology`
(OPTIONS) capabilities exist in the model and go unexercised, so those OPTIONS
capabilities are unclaimable from real passing cases; **(b)** the golden
normalizer + dialect adjudications live in *code* rather than a committed
adjudication register (the W-3f "no ad-hoc wire scraping" lesson), and several
suppression rules are edition-specific; **(c)** RESULT_SET shape assertions are
pinned to RM 1.2.0 / the dev OAS with no version ladder.

---

## 2. The spine (master11 → ECC map, then ECC-original corpus rows)

Schedule ids use the chapter's own form. **Every real master11 case Flow is `xx`
and the environment/data sets are `TBD`** (stub, cited once). Capability =
`AqlBasic` (STANDARD) for every current entry; profile STANDARD; area `Qry`. ECC
file:line is in `suites/query.rs`.

### `I_QUERY_SERVICE.execute_stored_query()` — AQL basic · STANDARD

| Schedule case | Schedule content | ECC substitute condition | ECC map — verdict |
|---|---|---|---|
| `I_QUERY_SERVICE.smoke_test` (master11:48, flow `xx`) | STUB | minimal ad-hoc query returns a well-formed `RESULT_SET` | `ECC-QRY-001` `qry/smoke-test` (`query.rs:385`) — **conformant-over-stub**: `POST /query/aql "SELECT e/ehr_id/value FROM EHR e"` → `200`, asserts `meta._type == "RESULTSET"` + a `columns` array (concretized against ITS-REST `200_QUERY.yaml`). |
| `I_QUERY_SERVICE.execute_stored_query-empty_db` (master11:61, flow `xx`) | STUB | store then execute-by-name → empty golden `RESULT_SET` | `ECC-QRY-003` `qry/execute-stored-query-empty-db` (`query.rs:434`) — **conformant-over-stub**: `PUT /definition/query/{name}/1.0.0` then `GET /query/{name}`, full golden diff (`golden::Mode::Full`) of the fixed-non-existent-id query (`A/200_get_ehr_by_id_empty_db.json`). |
| `Test Case bbbb` (master11:74) | STUB — **literal placeholder** ("`bbbb`" / `TBD`); no operation, no flow | — | **no ECC case (correct)**: a content-free TBD heading; nothing to map. Recorded so coverage math is honest. |

### `I_QUERY_SERVICE.execute_ad_hoc_query()` — AQL basic · STANDARD

| Schedule case | Schedule content | ECC substitute condition | ECC map — verdict |
|---|---|---|---|
| `I_QUERY_SERVICE.execute_ad_hoc_query-empty_db` (master11:83, `A.1.z`, flow `xx`) | STUB | fixed-non-existent-id query → empty golden (DB-state-independent) | `ECC-QRY-002` `qry/execute-ad-hoc-query-empty-db` (`query.rs:407`) — **conformant-over-stub**: `POST /query/aql`, full golden diff of `A/200_…_empty_db`. |
| `I_QUERY_SERVICE.execute_ad_hoc_query-loaded_db` (master11:96, `A.1.a`, flow `xx`) | STUB | commit a real composition to a fresh EHR, then EHR-scoped `CONTAINS COMPOSITION` returns it | `ECC-QRY-004` `qry/execute-ad-hoc-query-loaded-db` (`query.rs:480`) — **conformant-over-stub**: creates EHR, ensures OPT, commits `nested.en.v1` composition (`201`), runs `SELECT c/uid/value FROM EHR e[ehr_id/value='…'] CONTAINS COMPOSITION c`, asserts `rows>0` + column path `/uid/value`. Self-contained through the API (shared-SUT-safe). |

**Schedule coverage:** 4/4 *real* master11 headings mapped (QRY-001..004); the
literal `bbbb` placeholder is content-free (0 mappable). **All 4 are
conformant-over-stub.**

### ECC-original corpus rows (schedule stub — no normative master11 backing)

Below the stub, the real query evidence is the vendored AQL corpus
(`query/aql_queries_{valid,invalid}` + `expected_results/{empty_db,loaded_db}/{A-D}`),
driven as ECC-original cases whose oracle is the **AQL 1.1 spec + the golden
`RESULT_SET`s**. Each row is flagged **ECC-original (schedule stub)**.

| ECC case | Oracle / condition | Diff mode | Verdict |
|---|---|---|---|
| `ECC-QRY-005` `qry/corpus-invalid` (`query.rs:524`) | every `aql_queries_invalid/**` query must be rejected (`400`/`422`, ITS-REST `400_QUERY.yaml`) | n/a (status) | **conformant** — load-bearing negatives; `passed/total` over all groups. |
| `ECC-QRY-006..009` `qry/corpus-{a,b,c,d}-empty-db` (`query.rs:566-577`, `run_golden_group`) | each `expected_results/empty_db/<grp>` golden vs served `RESULT_SET` | `Full` (empty_db) / `ColumnsOnly` otherwise | **conformant-with-normalizer** — full columns+rows for fixed-id empty_db queries; every suppression names its `query_golden::Rule` (design §6). |
| `ECC-QRY-010..013` `qry/corpus-{a,b,c,d}-loaded-db` (`query.rs:578-589`) | each `expected_results/loaded_db/<grp>` golden | `ColumnsOnly` (shared-SUT) | **conformant-with-normalizer** — most rows skipped as unrunnable (`__MODIFY_…__` tokens / `$`-binds), columns diffed for the rest; a golden with no paired query is skipped, never a silent pass. |

**Golden-dialect adjudications** (inline in `query.rs`, spec-first, cited):

- `uses_removed_timewindow` (`query.rs:235`): AQL 1.1 removed `TIMEWINDOW`
  (QUERY `master00-amendment_record`, SPECQUERY-20). The corpus classifies
  `A/109`, `B/103`, `C/103` as valid; the pinned spec inverts the expectation to
  a `4xx` rejection. **Spec-supersedes-corpus** — correct.
- `limit_before_order_by` (`query.rs:250`): the 2019-era corpus places `LIMIT`
  before `ORDER BY` (`A/107`, `A/110`, `B/104-106`, `D/312-313`); AQL 1.1 orders
  `orderByClause? limitClause?` (`AqlParser.g4:22-24`), so the SUT's rejection is
  spec-correct and the golden is defective → **skip-with-reason (D3)**, never a
  server failure. Distinct from a spec-valid query the engine wrongly rejects
  (`e/ehr_status` on `EHR`, `A/106`), which stays a real finding.

---

## 3. Existing ECC cases with no schedule home

Because master11 is a stub, the entire corpus block is ECC-original. This is
**expected and honest** (README: "for QRY this will be most of them; that is
fine").

| ECC | Nature | Flag |
|---|---|---|
| `ECC-QRY-005` `qry/corpus-invalid` | AQL 1.1 negative-rejection evidence (invalid corpus). No master11 case covers invalid queries. | **ECC-original (schedule stub)** — keep; oracle is AQL 1.1 + ITS-REST `400_QUERY.yaml`. |
| `ECC-QRY-006..009` `qry/corpus-{a-d}-empty-db` | Golden `RESULT_SET` diffs, empty-DB (DB-state-independent). | **ECC-original (schedule stub)** — keep; oracle is the vendored golden corpus + AQL 1.1. |
| `ECC-QRY-010..013` `qry/corpus-{a-d}-loaded-db` | Golden `RESULT_SET` diffs, loaded-DB (columns-only on a shared SUT). | **ECC-original (schedule stub)** — keep; see G-5 for the shared-SUT divergence. |

The four QRY-001..004 cases are nominally schedule-mapped (the ids match master11
headings) but their *assertions* are ECC-derived over `xx` stub flows — the same
provenance caveat as register 02 G-1; recorded there and in §4 G-1 rather than
re-flagged per case.

---

## 4. G-rows — gaps + rulings for the rewrite

- **G-1 (master11 is a TBD stub — the QRY spine is ECC-original by necessity).
  SCHEDULE STUB.** §Test Environment and §Test Data Sets are `TBD`; one heading
  is the literal placeholder "`Test Case bbbb`" (master11:74); all four real
  flows are `xx`. No QRY assertion traces to normative schedule content. The
  rewrite must state this and derive the spine from **profiles (AQL basic
  STANDARD / advanced OPTIONS / terminology OPTIONS) + AQL 1.1 + the golden
  corpus**, flagging every corpus case **ECC-original (schedule stub)** in the
  report provenance and never presenting a QRY case as schedule-conformant. The
  four `I_QUERY_SERVICE.*` ids may keep their `schedule_ref` trace, but the
  report must show they concretize `xx` flows.

- **G-2 (capability mis-tagging — AQL advanced / terminology OPTIONS are
  unclaimable). CAPABILITY-CLAIMABILITY — LOAD-BEARING.** Every QRY case is
  tagged `Capability::AqlBasic` (`query.rs:184`, STANDARD), yet the model carries
  `Capability::AqlAdvanced` (`model/case.rs:115`, OPTIONS) and
  `Capability::Terminology` (`model/case.rs:156`, OPTIONS) which **no ECC case
  uses**. The corpus's advanced constructs (aggregation, single-row functions,
  the D-group) and the `TERMINOLOGY()`/`matches {uri}` family belong to those
  OPTIONS capabilities per master03-profiles. As written, AQL advanced and AQL &
  terminology cannot be *obtained* from real passing cases (profiles §"OPTIONS is
  obtained if any optional capability is passed"). The rewrite must classify each
  corpus/spec query by AQL 1.1 feature → the correct capability, so the OPTIONS
  profile claim is earned, not absent.

- **G-3 (golden normalizer + dialect adjudications live in code, not a register).
  W-3f LESSON — LOAD-BEARING.** The 7 suppression rules
  (`query_golden.rs::Rule`) and the two dialect predicates (`uses_removed_timewindow`,
  `limit_before_order_by`, `query.rs:235`/`:250`) are hardcoded in the suite.
  Per the adjudication discipline (`adjudications/README.md`: "only reclassifies
  with a citation — never edits, weakens, or skips a case"; standing rule 3 "never
  edit a test to route around a bug; never edit a golden"), corpus-vs-spec and
  golden-dialect adjudications belong in a **committed golden-adjudication
  register** (parallel to `adjudications/*.toml`), keyed by golden id with a
  citation, not in case bodies. The rewrite externalizes them so the suppression
  set is auditable and a golden is never mutated. Several rules are
  **VERSION-SPECIFIC** and must be edition-ladder assertions, not blanket
  suppressions: `RmTypeIgnored` (our RM 1.2.0 `_type` vs RM-1.0.x goldens),
  `SignatureDefaultOn` (our default-on `VERSION.signature`), `NumberFormatInsensitive`
  (`120` vs `120.0` by RM version), and the `meta._schema_version` handling
  (`query_golden.rs:47-49`).

- **G-4 (ad-hoc RESULT_SET wire scraping — no version ladder). VERSION-SPECIFIC.**
  Assertions reach into the served body directly: `meta._type == "RESULTSET"`
  (`query.rs:390`), `columns[0].path == "/uid/value"` (`query.rs:510`),
  `actual["rows"].as_array()` counts (`query.rs:502`), `columns[0].path`
  (`query.rs:510`). The `RESULT_SET` shape (`meta`, `columns{name,path}`, `rows`,
  `_schema_version`) is RM-1.2.0 / dev-OAS-specific. Per the W-3f ETag lesson,
  the rewrite centralizes RESULT_SET extraction in one wire adapter that records
  which edition form matched (the `_schema_version`, the columns `path`-omission
  behaviour noted in `query_golden.rs:390`, any RESULT_SET `ETag`), tried
  highest-first, so RM-1.1.0-era SUTs (upstream EHRbase) and other CDRs are not
  structurally excluded (README ruling 2; master03-overview §API Conformance
  minimum RM 1.0.2).

- **G-5 (shared non-empty SUT → columns-only for loaded_db). Documented
  divergence + data-set sourcing.** Global/loaded queries see other chapters'
  EHRs, so `run_golden_group` diffs `ColumnsOnly` for non-empty-db goldens
  (`query.rs:333`) and full columns+rows only for fixed-non-existent-id empty_db
  queries — sound, but a documented weakening of the golden's row oracle. The
  rewrite routes the loaded-DB data load through register 80's data-set strategy
  (fresh EHR per case, self-contained through the API, as `run_adhoc_loaded_db`
  already does) so full-row diffs become possible where a case owns its data;
  where they cannot, the columns-only fallback is recorded as an explicit
  per-case divergence, not a silent mode switch.

- **G-6 (unrunnable-query skips are honest but under-covered).** Loaded-DB
  goldens carrying `__MODIFY_…__` substitution tokens or `$`-binds are skipped
  (`unrunnable`, `query.rs:222`) because the harness lacks the runtime ids —
  correct, never a silent pass. But this means most `loaded_db` result *values*
  go unasserted. The rewrite, with register 80's self-loading data sets, should
  substitute real ids (the placeholder-wildcard machinery in
  `query_golden.rs::is_placeholder` already matches them) so the `__MODIFY_…__`
  goldens become runnable full-diff cases rather than skips.

---

*Register 80 owns the AQL corpus + loaded-DB data-set strategy referenced by
G-2/G-5/G-6; register 90 owns the RESULT_SET wire adapter, the externalized
golden-adjudication register, and the version-ladder architecture referenced by
G-3/G-4. The schedule-stub provenance flag (G-1) is shared with register 02.*
