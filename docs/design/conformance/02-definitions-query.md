# Conformance register 02 — DEFINITION / QUERY component (`suites/definition_query.rs`): spec-first audit

W-10 area audit (read-only, 2026-07-13) of the **DEFINITION / stored-query
provisioning** component of `tools/conformance`. Method is spec-first (README +
owner ruling): the spine is the governing CNF schedule chapter enumerated
test-case-by-test-case; the existing ECC cases are mapped **onto** each schedule
item with a `file:line` verdict. §3 lists ECC cases with no schedule home; §4
carries the G-rows for the rewrite, marking every edition-/version-specific
assertion the version-ladder runner must know about.

**Spec oracles** (read before any change):

- `docs/specs/openehr/CNF/docs/platform_test_schedule/master05-func_tc_definition_query.adoc`
  — the DEFINITION/`I_DEFINITION_QUERY` test suite. **This chapter is a stub**:
  its §Test Environment and §Test Data Sets are both the literal `[.tbd] TBD`,
  and every test case's Description/Pre-conditions/Post-conditions/Flow cell is
  the placeholder `xx` (one case id is itself a placeholder, `has_query-xxx`).
  The only normative content is the SM operation names and the case ids. Read
  whole.
- `docs/specs/openehr/CNF/docs/platform_test_schedule/master03-overview.adoc`
  — the test-case form and the RM-version note.
- `docs/specs/openehr/CNF/docs/profiles/master03-profiles.adoc` —
  §Functional/Definitions: **Query provisioning = STANDARD**; §REST APIs:
  **QUERY API = STANDARD**. (Stored-query provisioning is not a CORE capability.)

**Mapped suite:** `tools/conformance/src/suites/definition_query.rs` (7 ECC-SQR
entries, area `Sqr`, all `Capability::QueryProvisioning`, profile STANDARD).

---

## 1. Verdict

master05 is a **schedule stub** — no test environment, no data sets, and `xx`
placeholder flows throughout (§Oracle above). There is therefore *no normative
condition to be faithful to at the flow level*; the honest spine is the SM
operation set (`has_query`, `valid_query`, `list_queries`) crossed with the
ITS-REST DEFINITION QUERY surface, and the ECC cases derive their assertions
from ITS-REST + AQL 1.1 (the case citations say exactly this) plus the
EHRbase-suite `A.3.*` reference ids the chapter carries as comments. On that
basis the suite is **operation-complete and honest**: all 7 master05 case
headings have a 1:1 ECC-SQR entry, the two bare-list cases are correctly
adjudicated skip-with-reason (the SM `list_queries()` bare-collection has no
ITS-REST binding — the list resource is `GET /definition/query/{qualified_query_name}`;
D2, module docs `definition_query.rs:14-20`), and the store-time negatives assert
real AQL validation (`400`/`422`). The gaps are that **every assertion is
ECC-derived over a stub** (§4 G-1), the placeholder `has_query-xxx` id/title is
carried verbatim, and the valid-AQL data set is a single hardcoded corpus file.
The rewrite is about **owning the derivation explicitly (the schedule cannot be
the oracle here), sourcing data sets, and edition-aware negative-code
handling** — not adding coverage the stub does not call for.

---

## 2. The spine (master05 test cases → ECC map)

Schedule ids use the chapter's own form (`I_DEFINITION_QUERY.<op>-<id>`).
**Every schedule row's Flow/Pre/Post is `xx`** (stub, cited once here rather than
per row). Data-set classes are the schedule's `[.tbd] TBD`, so the "Data sets"
column records the ECC-chosen substitute. Capability = `QueryProvisioning`
(STANDARD) throughout. ECC file:line is in `suites/definition_query.rs`.

### `I_DEFINITION_QUERY.has_query()` — Query provisioning · STANDARD

| Schedule case | Schedule content | ECC substitute data set | ECC map — verdict |
|---|---|---|---|
| `I_DEFINITION_QUERY.has_query-xxx` (master05:37, flow `xx`) | STUB — placeholder id and `xx` flow; no normative condition | valid AQL from `query/aql_queries_valid/A/101_get_ehrs.json` | `ECC-SQR-003` `sqr/has-query-xxx` (`definition_query.rs:156`) — **conformant-over-stub**: stores a query then `GET /definition/query/{name}/{version}` → asserts `200` (existence realized as the named GET). Carries the placeholder `xxx` id/title verbatim (G-3). |

### `I_DEFINITION_QUERY.valid_query()` — Query provisioning · STANDARD

| Schedule case | Schedule content | ECC substitute data set | ECC map — verdict |
|---|---|---|---|
| `I_DEFINITION_QUERY.valid_query-valid` (master05:54, `A.3.a`, flow `xx`) | STUB | valid corpus AQL | `ECC-SQR-001` `sqr/valid-query-valid` (`definition_query.rs:135` → `store_query`, `:114`) — **conformant-over-stub**: `PUT /definition/query/{name}/{version}` (text/plain AQL) → asserts `[200,201]`. |
| `I_DEFINITION_QUERY.valid_query-invalid` (master05:67, `A.3.b`, flow `xx`) | STUB | malformed AQL literal `"SELECT FROM WHERE {{{ not valid aql"` | `ECC-SQR-007` `sqr/valid-query-invalid` (`definition_query.rs:219` → `store_bad`, `:171`) — **conformant-over-stub**: store-time validation → asserts `[400,422]`. |
| `I_DEFINITION_QUERY.valid_query-bad_formalism` (master05:80, `A.3.c`, flow `xx`) | STUB | non-AQL body `"SELECT * FROM patients; -- SQL, not AQL"` | `ECC-SQR-006` `sqr/valid-query-bad-formalism` (`definition_query.rs:205`) — **conformant-over-stub**: asserts `[400,422]` for a non-AQL formalism. |

### `I_DEFINITION_QUERY.list_queries()` — Query provisioning · STANDARD

| Schedule case | Schedule content | ECC substitute data set | ECC map — verdict |
|---|---|---|---|
| `I_DEFINITION_QUERY.list_queries-non_empty` (master05:110, flow `xx`) | STUB | one stored query | `ECC-SQR-002` `sqr/list-queries-non-empty` (`definition_query.rs:142`) — **divergent-mapping (D2 rebind, conformant to the rebound resource)**: the SM bare list has no ITS-REST binding, so this is rebound to `GET /definition/query/{name}` (the named-query list resource) → asserts `200`. A defensible D2 rebind, but note it lists a single named query, not "all stored queries". |
| `I_DEFINITION_QUERY.list_queries-empty` (master05:97, flow `xx`) | STUB | none | `ECC-SQR-004` `sqr/list-queries-empty` (`definition_query.rs:54`, run `:192`) — **skip-with-reason (D2, conformant handling)**: bare `GET /definition/query` collection is absent from ITS-REST (Release-1.0.3 + development@e8a093e); `.with_schedule_ref("I_DEFINITION_QUERY.list_queries (CNF master05:93)")`. |
| `I_DEFINITION_QUERY.list_queries-select_items` (master05:123, flow `xx`) | STUB | none | `ECC-SQR-005` `sqr/list-queries-select-items` (`definition_query.rs:62`, run `:198`) — **skip-with-reason (D2)**, same reason. |

**Schedule coverage:** 7/7 master05 case headings mapped; **0 missing**.
Verdicts: 4 conformant-over-stub (SQR-001, -003, -006, -007), 1 divergent-mapping
(SQR-002, D2 rebind), 2 skip-with-reason (SQR-004, -005, D2). **All 7 are
ECC-derived over a schedule stub** — see G-1.

---

## 3. Existing ECC cases with no schedule home

None. All 7 ECC-SQR entries map to a master05 case heading. (Because the chapter
is a stub, the *mapping* is nominal — the ids match — but the *assertions* have
no schedule backing; this is recorded as the whole-suite condition in §4 G-1
rather than per-case here.)

---

## 4. G-rows — gaps + rulings for the rewrite

- **G-1 (the schedule is a stub — the ECC spine is ECC-original by necessity).
  SCHEDULE STUB.** master05 §Test Environment and §Test Data Sets are `[.tbd]
  TBD`, and every case Flow is `xx` (`definition_query.rs` module docs already
  note "design-time reading"). No assertion in this suite traces to normative
  schedule content — they trace to ITS-REST DEFINITION QUERY + AQL 1.1 (the case
  citations) and to the EHRbase-suite `A.3.*` reference ids the chapter carries
  as comments. The rewrite must state this plainly: the DEFINITION QUERY spine is
  **derived from the ITS-REST contract + AQL 1.1 + profiles (Query provisioning
  STANDARD)**, with the stub schedule providing only operation names and case
  ids. Every case is flagged **ECC-original (schedule stub)** in the report's
  provenance, exactly as the QRY area (register 07) is. Do not present any SQR
  case as schedule-conformant.

- **G-2 (list_queries D2 split — record it, and watch the ladder). EDITION-SPECIFIC.**
  ITS-REST binds the list resource as `GET /definition/query/{qualified_query_name}`
  (verbs `[get, put]`); a bare `GET /definition/query` collection is absent in
  Release-1.0.3 and development@e8a093e (`definition_query.rs:183-190`). So
  `list_queries-empty`/`-select_items` are skipped and evidence nothing, while
  `-non_empty` is rebound to the named resource (G row in §2). The rewrite keeps
  the skips honest but must (a) treat a bare-list endpoint as an edition-ladder
  probe — if a future/other-CDR edition exposes it, the skipped cases become
  live; (b) not let the rebind masquerade as "list all queries" — either add a
  distinct multi-store→list-all case if any edition supports it, or PORT-NOTE the
  select_items semantics as unrealizable.

- **G-3 (placeholder id + shallow store post-conditions). SCHEDULE STUB
  artifact.** `sqr/has-query-xxx` (`ECC-SQR-003`) carries the schedule's literal
  `xxx` placeholder as its id and title ("Stored query existence check — xxx");
  since the schedule gives no real name, the rewrite should rename it to a
  descriptive id (e.g. `sqr/has-query-existing`) and record the schedule id in
  `schedule_ref`, not in the case id. Separately, `valid_query-valid`/`has_query`
  assert only the store/GET status; they do not assert the `Location` header on
  store or that the retrieved AQL text round-trips to what was stored — add the
  store→retrieve equality the ITS-REST contract implies.

- **G-4 (single hardcoded data set → register 80).** The valid AQL is a single
  file `query/aql_queries_valid/A/101_get_ehrs.json` (`definition_query.rs:105`),
  and the negative bodies are inline string literals (`:207`, `:221`). The
  rewrite sources valid + invalid + bad-formalism AQL classes from register 80's
  data-set strategy (shared with the QRY corpus), so the DEFINITION QUERY and
  QUERY areas exercise the same, versioned corpus rather than divergent
  hand-picked strings.

- **G-5 (negative status-code width). EDITION-SPECIFIC tolerance.** Both
  store-time negatives accept `[400,422]` (`definition_query.rs:209`, `:223`).
  `400` (malformed request) vs `422` (semantically-invalid AQL) is an
  implementation choice ITS-REST does not pin for stored-query create; the
  rewrite records which code the SUT returns as an edition finding rather than
  masking both behind one assertion (mirrors register 03 G-6).

---

*Register 80 owns the AQL data-set strategy referenced by G-4; register 90 owns
the version-ladder / endpoint-probe architecture referenced by G-2/G-5. The
schedule-stub provenance flag (G-1) is a report-model concern shared with
register 07.*
