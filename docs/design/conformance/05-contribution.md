# Conformance register — CONTRIBUTION (`I_EHR_CONTRIBUTION`): spec-first audit

W-10 area audit (read-only, 2026-07-13) of the ECC **CONTRIBUTION** area
(`tools/conformance/src/suites/contribution.rs`, catalogue prefix `CTB`).
Method is spec-first: the spine below is built **from the CNF platform test
schedule** — every test case master08 defines, in the schedule's own id form —
and each existing ECC case is mapped **onto** its schedule home with a
`file:line` verdict. ECC cases with no schedule home are flagged (§3); §4 is the
G-row ledger for the new version-ladder framework.

**Spec oracles** (read before any change):

- `docs/specs/openehr/CNF/docs/platform_test_schedule/master08-func_tc_ehr_contribution.adoc`
  — the CONTRIBUTION test suite (data sets + 31 test cases). Read whole.
- `docs/specs/openehr/CNF/docs/platform_test_schedule/master03-overview.adoc`
  — the test-case form (`<COMPONENT>.<operation>-<id>`; a "test" = a case ×
  a data set; minimum RM 1.0.2, SUT states its RM version).
- `docs/specs/openehr/CNF/docs/profiles/master03-profiles.adoc` — the
  capability→profile matrix. **Change sets** is a distinct capability, CORE +
  STANDARD (EHR Persistence component).
- RM common `master06` §Change control package (CONTRIBUTION atomicity,
  change_type / lifecycle_state semantics, one-create rule).

**Fixed contract:** the ECC catalogue numbering (`ECC-CTB-001..031`,
`tools/conformance/inventory/ecc-catalog.tsv`) and the SM native trait
`I_EHR_CONTRIBUTION`. This register audits the *runner cases*, not the server.

---

## 1. Verdict

The CONTRIBUTION suite is **structurally complete against the schedule's test
cases** (all 31 master08 cases have an ECC entry: 26 executed, 5
skip-with-reason for `list_contributions`), and the executed cases are faithful
in **shape** — commit → 201 + ETag, get → 200/404, the atomic-rollback and
one-create semantics are asserted. Two systematic weaknesses keep it short of
schedule fidelity, both to be fixed in the rewrite:

1. **Data-set collapse.** master08 attaches rich data-set *classes* to each
   operation (the COMPOSITION validity matrix, the multi-version A/B/C/D commit
   tables, the 15-row `EHR_STATUS` `is_modifiable × is_queryable ×
   subject.external_ref` matrix). The ECC exercises **one minimal fixture per
   case**, so a "test" (case × data set, master03 §API) is realized as a single
   data point. The distinct-precondition cases (`full_ehr_status` D.2) reuse the
   `minimal` runner verbatim — the data-set distinction the schedule draws is
   not exercised (§2, §4 G-2).
2. **Negative assertions are status-class only.** Every reject case asserts
   `support::assert_negative` = *any* `4xx` (`support.rs:151`), never the
   normative status the ITS-REST contribution contract binds (400 malformed /
   422 semantic-invalid / 409 conflict). This is edition-tolerant by design but
   erases the wire distinction the rewrite's version ladder needs (§4 G-1).

No ECC CONTRIBUTION case is a fabricated pass; the `list_contributions` skips
are honestly bookmarked (D2). ETag/uid scraping is done by two ad-hoc local
helpers that the rewrite must centralize (§4 G-4).

---

## 2. The spine (spec-first)

Every master08 test case, in schedule order, by its `I_EHR_CONTRIBUTION`
operation. Per case: (a) citation; (b) normative condition; (c) data-set
classes; (d) capability/profile; (e) ECC mapping + verdict. All CONTRIBUTION
cases carry capability **Change sets** (`Capability::ChangeSets`), profiles
**CORE + STANDARD** (master03-profiles §EHR Persistence / Change sets), format
**JSON only** (a CONTRIBUTION commit is a version-set + audit wrapper — no
canonical-XML wire shape; §4 G-6). Runner file is `contribution.rs`.

### 2.1 `I_EHR_CONTRIBUTION.commit_contribution()` (master08 §Test Cases, C/D/E)

#### `commit_contribution-valid_composition` (C.1)
- **Citation:** master08 §"Test Case …-valid_composition".
- **Condition:** commit a CONTRIBUTION of a valid COMPOSITION → positive; the
  created VERSION id is retrievable and its version number matches the
  change_type (creation ⇒ `::1`).
- **Data sets:** General B.1/COMPOSITION B.1.a (minimal, one per ENTRY type).
- **ECC:** ECC-CTB-001 (`contribution.rs:394`) — **conformant**. Asserts 201 +
  `etag` present + `versions[0].id` ends `::1`. Uses the single `minimal_
  evaluation` fixture; the "one for each entry type" class is not enumerated
  (§4 G-2).

#### `commit_contribution-invalid_composition` (C.2)
- **Citation:** master08 §"…-invalid_composition".
- **Condition:** commit an invalid COMPOSITION → negative with error info.
- **Data sets:** COMPOSITION B.2.a (mandatory item missing / wrong type / extra
  item / invalid value).
- **ECC:** ECC-CTB-002 (`contribution.rs:415`) — **divergent (tolerance)**.
  Removes `composer` (mandatory) then asserts `assert_negative` (any 4xx). The
  reject is correct, but the exact status (422 semantic) is unpinned
  (`support.rs:151`); only one of the four B.2.a invalidity classes is
  exercised (§4 G-1/G-2).

#### `commit_contribution-empty` (C.3)
- **Citation:** master08 §"…-empty".
- **Condition:** commit with no VERSIONs → negative, error names the empty
  VERSION list.
- **Data sets:** General B.4 (empty CONTRIBUTION).
- **ECC:** ECC-CTB-003 (`contribution.rs:433`) — **conformant** (any-4xx
  tolerance; `no_versions.json` fixture).

#### `commit_contribution-valid_invalid_compositions` (C.4)
- **Citation:** master08 §"…-valid_invalid_compositions" + §Combinations D
  ("if any COMPOSITION is invalid, the whole commit fails").
- **Condition:** mixed valid + invalid VERSIONs → the whole commit is rejected
  transactionally; **no** CONTRIBUTION/VERSION persists.
- **Data sets:** General B.6 / COMPOSITION combination table D (four
  valid+invalid mixes).
- **ECC:** ECC-CTB-004 (`contribution.rs:443`) — **divergent (post-condition
  unverified)**. Asserts the request is a 4xx, but never re-reads the EHR to
  confirm the atomic-rollback post-condition (schedule NOTE: "no CONTRIBUTIONs
  or VERSIONs should be created"). Only one of table D's four mixes is run
  (§4 G-3/G-2).

#### `commit_contribution-event_composition` (C.5)
- **Citation:** master08 §"…-event_composition".
- **Condition:** create an event COMPOSITION (v1), then modify it in a second
  commit (change_type modification, `preceding_version_uid` = v1) → v2 with the
  same `OBJECT_VERSION_ID` object, version 2.
- **Data sets:** COMPOSITION B.1.a event category; multi-commit flow.
- **ECC:** ECC-CTB-006 (`contribution.rs:470`) — **conformant**. Asserts v2 ends
  `::2`. Patches `preceding_version_uid` with the real v1 (fixture adaptation
  §6, additive). `minimal_admin` is category=event.

#### `commit_contribution-persistent_composition` (C.6)
- **Citation:** master08 §"…-persistent_composition".
- **Condition:** create a persistent COMPOSITION then modify → v2, same object.
- **Data sets:** COMPOSITION B.1.c persistent; multi-commit flow.
- **ECC:** ECC-CTB-007 (`contribution.rs:502`) — **conformant** (asserts `::2`;
  `minimal_persistent` fixtures).

#### `commit_contribution-delete` (C.7)
- **Citation:** master08 §"…-delete".
- **Condition:** create then delete a COMPOSITION (change_type deleted,
  preceding = v1) → v2, VERSIONED_OBJECT logically deleted.
- **Data sets:** COMPOSITION change-type flow creation→…→deleted.
- **ECC:** ECC-CTB-008 (`contribution.rs:534`) — **divergent (post-condition
  unverified)**. Asserts the delete commit returns 201; nulls the deleted
  VERSION's `data` (RM master06: a deleted VERSION's data is Void — additive
  §6). Does **not** assert the object is subsequently logically deleted (a
  follow-up GET would 404/return a deleted lifecycle) — the schedule
  post-condition "VERSIONED_OBJECT should be logically deleted" is not checked.
  Note the schedule's own NOTE flags delete effect as under-specified in SM.

#### `commit_contribution-two_commits_second_invalid` (C.8)
- **Citation:** master08 §"…-two_commits_second_invalid".
- **Condition:** valid create, then a modify with invalid content → negative;
  exactly one VERSION remains.
- **Data sets:** COMPOSITION B.2.a invalid on the second commit.
- **ECC:** ECC-CTB-009 (`contribution.rs:562`) — **divergent (post-condition
  unverified)**. Asserts the 2nd commit is 4xx; does not re-read to confirm only
  v1 survives (§4 G-3).

#### `commit_contribution-two_commits_second_creation` (C.9)
- **Citation:** master08 §"…-two_commits_second_creation" + NOTE (only one
  'create' per object; subsequent ops must be modification).
- **Condition:** valid create, then a second commit with change_type=creation on
  the existing object → negative (wrong change_type).
- **Data sets:** change-type misuse.
- **ECC:** ECC-CTB-010 (`contribution.rs:590`) — **conformant** (any-4xx). Sets
  the 2nd commit's change_type to code `249` (creation).

#### `commit_contribution-non_exiting_opt` (C.10)
- **Citation:** master08 §"…-non_exiting_opt".
- **Condition:** COMPOSITION references an OPT never loaded → negative, error
  names the missing OPT.
- **Data sets:** COMPOSITION B.2.b (referenced OPT not loaded).
- **ECC:** ECC-CTB-005 (`contribution.rs:458`) — **conformant (tolerance)**.
  `ref_to_non_existent_OPT.json`; any-4xx (module comment expects 422). The
  "error names the missing OPT" body assertion is not made (§4 G-1).

#### `commit_contribution-minimal_ehr_status` (D.1)
- **Citation:** master08 §"…-minimal_ehr_status" + §EHR_STATUS data sets B.3.
- **Condition:** commit a valid `EHR_STATUS` modification (change_type is always
  modification/amendment for STATUS) → positive; a new CONTRIBUTION + STATUS
  VERSION exist; CONTRIBUTION count verified.
- **Data sets:** EHR_STATUS accepted matrix (15 rows: `is_modifiable ×
  is_queryable × subject.external_ref{HIER_OBJECT_ID,GENERIC_ID,NULL}`).
- **ECC:** ECC-CTB-011 (`contribution.rs:619`→`632`) — **divergent (data-set
  collapse)**. Commits one `status.contribution.modification` fixture against the
  current STATUS uid → 201. The 15-row matrix is not swept, and the "verify
  CONTRIBUTION uids + count" post-condition is not checked (§4 G-2).

#### `commit_contribution-full_ehr_status` (D.2)
- **Citation:** master08 §"…-full_ehr_status" (differs from D.1 only in
  precondition: EHR created *with* a full EHR_STATUS incl. `subject.external_
  ref`).
- **Condition:** as D.1 but the pre-existing STATUS is fully populated.
- **Data sets:** EHR_STATUS full-population precondition.
- **ECC:** ECC-CTB-012 (`contribution.rs:623`) — **instrument-encodes-server-
  behaviour**. `run_full_ehr_status` **calls the same `commit_status_
  modification` runner as D.1** (`:627`) — the "full EHR_STATUS precondition" is
  never established (the per-case EHR is created default). The case cannot fail
  differently from D.1; it re-measures D.1 under a new id (§4 G-2).

#### `commit_contribution-ehr_status_invalid_change_type` (D.3)
- **Citation:** master08 §"…-ehr_status_invalid_change_type" + §EHR_STATUS
  reject 1 (change_type ∈ {creation, deleted} rejected — default STATUS already
  exists, STATUS can't be deleted).
- **Condition:** change_type=creation (or delete) on an EHR that already has a
  STATUS → negative.
- **Data sets:** EHR_STATUS reject matrix row 1.
- **ECC:** ECC-CTB-013 (`contribution.rs:644`) — **divergent (partial)**. Tests
  only change_type=creation (code 249) with the preceding removed → 4xx. The
  `deleted` half of the reject rule is not exercised (§4 G-2).

#### `commit_contribution-invalid_ehr_status` (D.4)
- **Citation:** master08 §"…-invalid_ehr_status" + §EHR_STATUS reject 3.
- **Condition:** change_type=modification with an invalid STATUS → negative.
- **Data sets:** invalid EHR_STATUS.
- **ECC:** ECC-CTB-014 (`contribution.rs:661`) — **conformant (tolerance)**.
  Removes mandatory `is_queryable`, modifies against current uid → any-4xx.

#### `commit_contribution-valid_directory` (E.1)
- **Citation:** master08 §"…-valid_directory" + §FOLDER data sets.
- **Condition:** commit a valid FOLDER (change_type creation) to an EHR with no
  directory → positive; a CONTRIBUTION + directory now exist.
- **Data sets:** FOLDER valid combos (minimal / +items / +subfolders / …).
- **ECC:** ECC-CTB-015 (`contribution.rs:682`) — **conformant** (201;
  `folder.contribution.creation` fixture; one of the 5 folder shapes).

#### `commit_contribution-fail_create_existing_directory` (E.2)
- **Citation:** master08 §"…-fail_create_existing_directory".
- **Condition:** creation of a directory when one already exists → negative
  (wrong change_type, root FOLDER already present).
- **ECC:** ECC-CTB-016 (`contribution.rs:692`) — **conformant** (create twice;
  2nd is any-4xx; the DIRECTORY-API twin ECC-DIR-002 pins 409, §06 register).

#### `commit_contribution-fail_modify_non_existing_directory` (E.3)
- **Citation:** master08 §"…-fail_modify_non_existing_directory".
- **Condition:** modify a directory that doesn't exist (random preceding uid) →
  negative (wrong change_type).
- **ECC:** ECC-CTB-017 (`contribution.rs:705`) — **conformant** (any-4xx;
  normalizes change_type to 251, random preceding uid).

#### `commit_contribution-update_existing_directory` (E.4)
- **Citation:** master08 §"…-update_existing_directory".
- **Condition:** modify/amend an existing directory → positive; new
  CONTRIBUTION + FOLDER VERSION.
- **ECC:** ECC-CTB-018 (`contribution.rs:719`) — **conformant** (create → modify
  with real preceding → 201).

### 2.2 `I_EHR_CONTRIBUTION.list_contributions()` (master08 §F)

master08 defines 5 cases (F.1 post_commit, F.2 empty, F.3 non_existing_ehr,
F.4 ehr_containing_ehr_status, F.5 ehr_containing_directory), each returning a
list (or an EHR-not-found error).

- **ECC:** ECC-CTB-027..031 (`contribution.rs:842..860`, all `skip_list`) —
  **skip-with-reason (D2)**. The SM `list_contributions()` operation has **no
  ITS-REST binding**: ITS-REST (Release-1.0.3 and the tested development@e8a093e
  OAS) binds **POST only** on `/ehr/{ehr_id}/contribution`; there is no GET
  collection resource. Each case reports `SKIPPED` with `schedule_ref =
  "I_EHR_CONTRIBUTION.list_contributions (CNF master08:595)"` rather than a
  fabricated 405 failure. **Verdict: honest skip.** The rewrite keeps the skip
  but should attach the native-API integration test that *does* exercise
  `list_contributions` as the evidence pointer (§4 G-5).

### 2.3 `I_EHR_CONTRIBUTION.has_contribution()` (master08 §G)

Schedule: G.1 existing→true, G.2 empty_ehr→false, G.3 bad_ehr→error, G.4
bad_contribution→false. Realized (CNF guide §"From Specifications to Runnable
Tests", element 2) as `GET /contribution/{uid}` where 200 = true, 404 = false.

- `has_contribution-existing` (G.1) → ECC-CTB-023 (reuses `run_get_existing`,
  `:186/741`) — **conformant** (200 + body `_type == CONTRIBUTION`).
- `has_contribution-empty_ehr` (G.2, expects **false**) → ECC-CTB-026 (reuses
  `run_get_empty_ehr`, `:204/772`) — **divergent (protocol collapse)**. Asserts
  404. The REST realization collapses the schedule's `false` (G.2) and `error`
  (G.3) into one 404; acceptable under element-2 mapping but the true/error
  distinction the boolean-returning SM op draws is lost (§4 G-7).
- `has_contribution-bad_ehr` (G.3, expects **error**) → ECC-CTB-025 (reuses
  `run_get_bad_ehr`, `:198/786`) — **conformant** (404).
- `has_contribution-bad_contribution` (G.4, expects **false**) → ECC-CTB-024
  (reuses `run_get_bad_contribution`, `:192/803`) — **conformant** (404 for a
  random uid on an EHR that has a real CONTRIBUTION).

### 2.4 `I_EHR_CONTRIBUTION.get_contribution()` (master08 §H)

Schedule: H.1 existing, H.2 empty_ehr (error), H.3 bad_ehr (error), H.4
bad_contribution (error). Realized as `GET /ehr/{id}/contribution/{uid}`.

- `get_contribution-existing` (H.1) → ECC-CTB-019 (`:741`) — **conformant** (200
  + `_type == CONTRIBUTION`; ETag/uid via `contribution_uid`, §4 G-4).
- `get_contribution-empty_ehr` (H.2) → ECC-CTB-020 (`:772`) — **conformant**
  (404 for a random uid on a fresh EHR).
- `get_contribution-bad_ehr` (H.3) → ECC-CTB-021 (`:786`) — **conformant** (404).
- `get_contribution-bad_contribution` (H.4) → ECC-CTB-022 (`:803`) —
  **conformant** (404).

---

## 3. ECC cases with no master08 schedule home

None. Every `ECC-CTB-*` case maps to a master08 test case (the
`has_contribution` cases reuse the `get_contribution` runners — a realization
choice, not an extension). The suite invents no CONTRIBUTION extension case.
Conversely, master08 has **no missing test case** — all 31 have an ECC entry
(26 executed + 5 honest `list_contributions` skips).

---

## 4. G-rows — rulings for the version-ladder rewrite

The new framework runs a **highest-first spec-edition ladder** (RM 1.2.0 first,
falling back through prior editions for a multi-SUT run). Each assertion must be
tagged **normative-invariant** (holds across every edition) or **edition-
specific** (holds only for the edition under test). The rows below fix that
tagging and the data-set / centralization debt.

- **G-1 — pin negative status codes (edition-specific).** Replace the blanket
  `assert_negative` (any 4xx, `support.rs:151`) with the ITS-REST-bound status
  per reject class: malformed body → **400**, schema/OPT-absent/semantic-invalid
  → **422**, change_type/existence conflict → **409**. The *class* (4xx) is
  normative-invariant; the *specific code* is edition-specific (the
  development@e8a093e OAS vs Release-1.0.3 may differ) → assert the code only on
  the matched edition, fall back to the 4xx class on older editions. Affects C.2,
  C.4, C.8, C.9, C.10, D.3, D.4, E.2, E.3.

- **G-2 — data-set sweeps, not single fixtures (normative-invariant
  structure).** master08 attaches data-set *classes*; the rewrite must iterate
  them as `case × data set` (master03 §API "a test = a case with a data set"):
  the COMPOSITION validity matrix + B.1 per-ENTRY-type sweep (C.1/C.2), the
  multi-version A/B/C/D commit tables (currently only D partially, via C.4), and
  the **15-row EHR_STATUS matrix** `is_modifiable × is_queryable ×
  subject.external_ref` (D.1). The matrix *rows* are normative-invariant; the
  wire encoding of `subject.external_ref` (`HIER_OBJECT_ID`/`GENERIC_ID` shapes)
  is RM-version-sensitive → edition-tag the payload builder.

- **G-3 — assert the transactional post-conditions.** C.4/C.8/D.4 carry
  post-conditions the ECC never verifies ("no CONTRIBUTIONs/VERSIONs created";
  "only one VERSION remains"; C.7 "VERSIONED_OBJECT logically deleted"). Add a
  follow-up read (contribution list count / version count / lifecycle state) so
  atomic rollback (RM master06 §Contributions) is *proven*, not assumed.
  Post-condition truth is **normative-invariant**.

- **G-4 — centralize ETag/uid extraction (W-3f lesson).** `contribution.rs::
  contribution_uid` (`:319`) and `version_uid_at` (`:305`), plus `support.rs::
  version_uid` (`:116`), each re-implement weak/bare-ETag stripping (`W/"…"` →
  bare, `trim_matches('"')`) and body-`uid.value` fallback locally. The rewrite
  must route all header/ETag/id scraping through one helper. **The weak-vs-bare
  ETag form is edition-specific** (ITS-REST overview §"ETag and Last-Modified"
  makes it weak `W/"…"`; the bare quoted form is deprecated-but-tolerated) — the
  centralized helper carries the edition flag, individual cases must not.

- **G-5 — `list_contributions` skip keeps its evidence pointer (D2).** The
  no-ITS-REST-binding skip (§2.2) is correct and must survive the rewrite
  unchanged, but attach the native-API integration test as the `schedule_ref`
  evidence so the skip is *documented coverage*, not a hole. Skip reason is
  edition-checked: if a future ITS-REST edition adds a GET collection resource,
  the ladder must promote these to executed cases.

- **G-6 — CONTRIBUTION is JSON-only on the wire (normative-invariant).** A
  CONTRIBUTION commit is a version-set + audit wrapper with no canonical-XML wire
  shape in the tested OAS; keep `formats = [Json]`. (Blueprint ch5 F-05-06 tracks
  version-family/CONTRIBUTION XML as a separate open item — do not silently add
  XML here.)

- **G-7 — boolean-op REST collapse is edition-specific.** `has_contribution`
  G.2 (`false`) and G.3 (`error`) both become 404 over REST (§2.3). Record that
  the true/false/error trichotomy of the SM boolean op is realized as 200/404 by
  element-2 mapping; the collapse is a REST-binding property (edition-specific),
  and the native-API surface (`ehrbase-sm`) preserves the distinction — the
  rewrite must not "fix" the 404 into a 200-false.

- **G-8 — change_type codes are terminology-pinned (normative-invariant).** The
  suite hardcodes `openehr` group codes 249=creation, 251=modification,
  253=deleted (`contribution.rs:361/383/610`). These are openEHR Terminology
  `audit change type` codes (master08 §Data Set Considerations links the list) —
  edition-invariant. Keep them, but source them from `openehr-term`, not string
  literals, so a terminology bump can't drift them silently. The
  `normalize_modification` fixture patch (249→251, `:373`) is a corpus-fixture
  defect workaround (RM-1.0.x-era inconsistent fixture) — keep it a documented
  additive adaptation, never a case edit (standing rule 3).
