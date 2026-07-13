# Conformance register — DIRECTORY (`I_EHR_DIRECTORY`): spec-first audit

W-10 area audit (read-only, 2026-07-13) of the ECC **DIRECTORY** area
(`tools/conformance/src/suites/directory.rs`, catalogue prefix `DIR`). Method is
spec-first: the spine below is built **from the CNF platform test schedule** —
every test case master09 defines, in the schedule's own id form — and each
existing ECC case is mapped **onto** its schedule home with a `file:line`
verdict. ECC cases with no schedule home are flagged (§3); §4 is the G-row
ledger for the new version-ladder framework.

**Spec oracles** (read before any change):

- `docs/specs/openehr/CNF/docs/platform_test_schedule/master09-func_tc_ehr_directory.adoc`
  — the DIRECTORY test suite (data sets + 37 test cases across 10 operations).
  Read whole.
- `docs/specs/openehr/CNF/docs/platform_test_schedule/master03-overview.adoc`
  — the test-case form and "a test = a case × a data set" rule.
- `docs/specs/openehr/CNF/docs/profiles/master03-profiles.adoc` — **Directory
  Operations** is a distinct capability, **STANDARD only** (not CORE);
  **Versioning** is CORE + STANDARD (EHR Persistence component).
- RM ehr `master04` §Folders / common `master06` §Change control (directory =
  a versioned root FOLDER; delete = a `lifecycle_state=deleted` version).

**Fixed contract:** the ECC catalogue numbering (`ECC-DIR-001..037`) and the SM
native trait `I_EHR_DIRECTORY`. This register audits the *runner cases*.

---

## 1. Verdict

The DIRECTORY suite is **structurally complete**: all 37 master09 test cases
across the 10 `I_EHR_DIRECTORY` operations have an ECC entry, all executed (no
skips), and the CRUD shape (create 201 + ETag + Location, get 200, update
200/204, delete 200/204, absent → 404, create-when-present → 409) is faithful to
the ITS-REST DIRECTORY contract. Three fidelity gaps keep it short of the
schedule, all for the rewrite:

1. **Time-travel is not actually exercised.** master09 §G defines
   `get_directory_at_time` cases whose *point* is temporal selection — a time
   between v1 and v2 must return **v1** (G.5, G.8); a time before creation must
   return **empty** (G.5). The ECC queries a far-future instant and asserts only
   `200` (= current), so the version-selection logic the schedule mandates is
   never verified over the wire; a comment concedes "precise first-version
   selection is exercised by the service-layer tests" (`directory.rs:781`). This
   is instrument-encodes-server-behaviour (§2, §4 G-1).
2. **`has_path` never asserts the false branch.** master09 D.2/D.3 attach
   data-set tables mixing existing paths (→true) and random paths (→false); the
   ECC asserts only the *present* branch (`200/204`), never a random path →
   `404` (§4 G-2).
3. **`get_versioned_directory` is rebound to at-version get.** master09 §L wants
   a VERSIONED_OBJECT that *references* every version (L.2: "should reference the
   two existing versions"); the ECC (D2 rebind) issues `GET
   /directory/{version_uid}` returning one FOLDER version — the container
   semantics are not tested (§2, §4 G-3).

No DIRECTORY case is a fabricated pass, and the abstract-op realizations follow
the CNF guide's element-2 rule. Version-uid scraping uses `support::version_uid`
(ad-hoc ETag parsing, W-3f lesson) — centralize in the rewrite (§4 G-4).

---

## 2. The spine (spec-first)

Every master09 test case, in schedule order, by its `I_EHR_DIRECTORY`
operation. Per case: citation; normative condition; data-set classes;
capability/profile; ECC mapping + verdict. Unless noted, every case carries
capability **Directory Operations** (`Capability::DirectoryOps`), profile
**STANDARD** (master03-profiles §EHR Persistence / Directory Operations), format
**JSON** (`directory.rs:291` `entry_cap`). Runner file is `directory.rs`.

**Data-set classes (master09 §Test Data Sets), attached to every create/get/
update case:** (1) FOLDER; (2) FOLDER+items; (3) FOLDER+subfolders; (4)
FOLDER+subfolders+items on all folders; (5) FOLDER with *n* levels (to detect
implementation limits) + the reference structure
`/emergency/{episode-x,episode-y}`, `/hospitalization`. The ECC uses **one
fixture** (`directory/subfolders_in_directory.json`, which realizes the
reference structure) for all shape-bearing cases — the *n*-level and
items-on-all-folders classes are not separately swept (§4 G-2).

### 2.1 `I_EHR_DIRECTORY.has_directory()` (master09 §C)

Realized as `GET /directory` → 200 (has) / 404 (not).
- `has_directory-empty_ehr` (C.1, →false) → ECC-DIR-012 (`directory.rs:549`) —
  **divergent (protocol collapse)**: asserts 404 (collapses false/error, §4 G-5).
- `has_directory-ehr_with_directory` (C.2, →true) → ECC-DIR-013 (`:562`) —
  **conformant** (200).
- `has_directory-bad_ehr` (C.3, →error) → ECC-DIR-014 (`:575`) — **conformant**
  (404).

### 2.2 `I_EHR_DIRECTORY.has_path()` (master09 §D)

Realized as `GET /directory?path=` → 200/204 (present) / 404 (absent).
- `has_path-empty_ehr` (D.1, →false) → ECC-DIR-017 (`:615`) — **conformant**
  (404).
- `has_path-ehr_root_directory` (D.2) — **data set:** `{'/'→true, random→false}`.
  → ECC-DIR-015 (`:589`) — **divergent (data-set collapse)**: asserts only
  `path=/` → 200/204; the `random→false` row is not run (§4 G-2).
- `has_path-folder_structure` (D.3) — **data set:** the 12-row path table over
  the reference tree (8 true, 4 false incl. partial-path randoms). → ECC-DIR-016
  (`:602`) — **divergent (data-set collapse)**: asserts only `path=/emergency` →
  200/204 (the fixture does carry `emergency`); the other 11 rows — crucially
  the four `→false` rows — are not exercised (§4 G-2).
- `has_path-bad_ehr` (D.4, →error) → ECC-DIR-018 (`:628`) — **conformant** (404).

### 2.3 `I_EHR_DIRECTORY.create_directory()` (master09 §E)

Realized as `POST /directory`.
- `create_directory-empty_ehr` (E.1) → ECC-DIR-001 (`:342`) — **conformant**
  (201 + `etag` + `location` present; one folder shape).
- `create_directory-ehr_with_directory` (E.2, →error already-exists) →
  ECC-DIR-002 (`:360`) — **conformant** (409).
- `create_directory-bad_ehr` (E.3, →error) → ECC-DIR-003 (`:377`) —
  **conformant** (404).

### 2.4 `I_EHR_DIRECTORY.get_directory()` (master09 §F)

Realized as `GET /directory`.
- `get_directory-empty_ehr` (F.1, →empty; NOTE: REST may 4xx) → ECC-DIR-022
  (`:683`) — **conformant** (404, per the schedule NOTE).
- `get_directory-ehr_root_directory` (F.2) → ECC-DIR-004 (`:391`) —
  **conformant** (200; asserts status only, not the returned empty-FOLDER shape).
- `get_directory-directory_with_structure` (F.3) → ECC-DIR-023 (`:696`) —
  **divergent (weak assertion)**: asserts 200 but does **not** compare the
  returned tree to the committed structure (the schedule wants "the full
  structure"); body fidelity unverified (§4 G-6).
- `get_directory-bad_ehr` (F.4, →error) → ECC-DIR-005 (`:405`) — **conformant**
  (404).

### 2.5 `I_EHR_DIRECTORY.get_directory_at_time()` (master09 §G)

Realized as `GET /directory?version_at_time=`.
- `get_directory_at_time-empty_ehr` (G.1, →empty) → ECC-DIR-027 (`:752`) —
  **conformant** (404, per NOTE).
- `get_directory_at_time-empty_ehr_empty_time` (G.2, →empty) → ECC-DIR-028
  (`:767`) — **conformant** (404; empty time = plain `GET /directory`).
- `get_directory_at_time-ehr_with_directory` (G.3, current time →current) →
  ECC-DIR-006 (`:418`) — **conformant** (far-future time → 200).
- `get_directory_at_time-ehr_with_directory_empty_time` (G.4, →current) →
  ECC-DIR-024 (`:711`) — **conformant** (200).
- `get_directory_at_time-ehr_with_directory_versions` (G.5) — **normative flow:**
  time before creation →empty; time between v1 and v2 →**v1**; current →v2. →
  ECC-DIR-025 (`:724`) — **instrument-encodes-server-behaviour**: queries a
  single far-future instant → asserts 200 only. The three-point temporal
  selection (the entire point of the case) is not exercised (§4 G-1).
- `get_directory_at_time-ehr_with_directory_versions_empty_time` (G.6, →current
  latest) → ECC-DIR-026 (`:739`) — **conformant** (200).
- `get_directory_at_time-bad_ehr` (G.7, →error) → ECC-DIR-007 (`:434`) —
  **conformant** (404).
- `get_directory_at_time-multiple_versions_first` (G.8) — **normative:** time
  AFTER v1 but BEFORE v2 must return **v1**. → ECC-DIR-029 (`:780`) —
  **instrument-encodes-server-behaviour**: queries far-future → asserts 200 (=
  current v2, the *opposite* of what G.8 selects). The case comment concedes the
  real selection is only in service-layer tests. This case does not distinguish a
  correct from an incorrect at-time selection (§4 G-1). **Highest-priority fix.**

### 2.6 `I_EHR_DIRECTORY.update_directory()` (master09 §H)

Realized as `PUT /directory` with `If-Match`.
- `update_directory-ehr_with_directory` (H.1) → ECC-DIR-008 (`:450`) —
  **conformant** (If-Match = directory uid → 200/204; renames root).
- `update_directory-empty_ehr` (H.2, →error no directory) → ECC-DIR-036 (`:892`)
  — **divergent (tolerance)**: accepts `{400,404,412}` (`status_in`). The set is
  edition-tolerant; the normative "non-existent directory" maps to a specific
  code the rewrite should pin (§4 G-7).
- `update_directory-bad_ehr` (H.3, →error) → ECC-DIR-009 (`:469`) — **divergent
  (tolerance)**: accepts `{400,404,412}` (§4 G-7).

### 2.7 `I_EHR_DIRECTORY.delete_directory()` (master09 §I)

Realized as `DELETE /directory` with `If-Match`.
- `delete_directory-empty_ehr` (I.1, →error) → ECC-DIR-037 (`:907`) —
  **divergent (tolerance)**: `{400,404,412}` (§4 G-7).
- `delete_directory-ehr_with_directory` (I.2) → ECC-DIR-010 (`:485`) —
  **divergent (post-condition unverified)**: asserts 200/204 but does **not**
  re-read to confirm the directory now shows a `lifecycle_state=deleted` version
  (schedule NOTE: "the directory exists as a new deleted version"). Logical-
  delete post-condition unproven (§4 G-8).
- `delete_directory-bad_ehr` (I.3, →error) → ECC-DIR-011 (`:502`) — **divergent
  (tolerance)**: `{400,404,412}` (§4 G-7).

### 2.8 `I_EHR_DIRECTORY.has_directory_version()` (master09 §J)

Realized as `GET /directory/{version_uid}` → 200 / 404.
- `has_directory_version-empty_ehr` (J.1, →false) → ECC-DIR-019 (`:642`) —
  **conformant** (404 for a fake version uid).
- `has_directory_version-directory_with_two_versions` (J.2, both →true) →
  ECC-DIR-020 (`:655`) — **divergent (partial)**: asserts the **v1** uid → 200;
  the v2 uid (the case's second assertion) is not separately checked (§4 G-2).
- `has_directory_version-bad_ehr` (J.3, →error) → ECC-DIR-021 (`:668`) —
  **conformant** (404).

### 2.9 `I_EHR_DIRECTORY.get_directory_at_version()` (master09 §K)

Realized as `GET /directory/{version_uid}`.
- `get_directory_at_version-empty_ehr` (K.1, →error) → ECC-DIR-032 (`:825`) —
  **conformant** (404).
- `get_directory_at_version-directory_with_two_versions` (K.2) — **normative:**
  v1 uid →v1; v2 uid →v2. → ECC-DIR-031 (`:812`) — **divergent (partial)**:
  fetches the **v1** uid → 200 only; does not confirm it returns *v1* content
  nor fetch v2 (§4 G-2/G-6).
- `get_directory_at_version-bad_ehr` (K.3, →error) → ECC-DIR-030 (`:799`) —
  **conformant** (404).

### 2.10 `I_EHR_DIRECTORY.get_versioned_directory()` (master09 §L)

master09 §L wants the **VERSIONED_OBJECT** (versioned FOLDER container). ITS-REST
(the tested development@e8a093e OAS and Release-1.0.3) exposes no
`versioned_directory` resource, so the ECC (D2 adjudication) **rebinds** these to
`GET /ehr/{id}/directory/{version_uid}` and tags them **`Capability::
Versioning`** (CORE + STANDARD) so they evidence CORE claimability (D5).
- `get_versioned_directory-empty_ehr` (L.1, →error/empty) → ECC-DIR-033
  (`:845`) — **conformant** (404 at a fake version uid).
- `get_versioned_directory-directory_with_two_versions` (L.2) — **normative:**
  returns the versioned FOLDER "referencing the two existing versions". →
  ECC-DIR-034 (`:860`) — **divergent (rebind loses container semantics)**:
  fetches the v2 FOLDER → 200; the VERSIONED_OBJECT with its two `versions`
  references is **not** returned or asserted. The D2 rebind is honestly recorded
  (`schedule_ref = "…get_versioned_directory (CNF master09:670)"`), but the case
  no longer tests what L.2 specifies (§4 G-3).
- `get_versioned_directory-bad_ehr` (L.3, →error) → ECC-DIR-035 (`:875`) —
  **conformant** (404).

---

## 3. ECC cases with no master09 schedule home

None. Every `ECC-DIR-*` case maps to a master09 test case, and master09 has no
missing test case — all 37 have an executed ECC entry. The suite invents no
DIRECTORY extension case. The only spec↔wire mismatch is the
`get_versioned_directory` rebind (§2.10), which is a realization/adjudication
choice recorded with a `schedule_ref`, not an extension.

---

## 4. G-rows — rulings for the version-ladder rewrite

The new framework runs a **highest-first spec-edition ladder** (RM 1.2.0 first).
Each assertion is tagged **normative-invariant** or **edition-specific**.

- **G-1 — actually exercise `get_directory_at_time` selection (normative-
  invariant). Highest priority.** G.5 and G.8 (ECC-DIR-025/029) must issue the
  three schedule-mandated queries: (a) `version_at_time` before EHR creation →
  empty/404; (b) between v1 and v2 → **v1** (assert the returned FOLDER is v1, by
  name/uid); (c) current/empty-time → **v2**. The current far-future single-shot
  200-only assertion cannot distinguish a correct engine from a broken one. The
  *selection semantics* are normative-invariant; the timestamp **format**
  (RFC3339/ISO-8601 with/without offset) is edition-specific → edition-tag the
  `version_at_time` literal.

- **G-2 — data-set sweeps, not single fixtures / single rows (normative-
  invariant).** master09 attaches data-set tables the ECC collapses: `has_path`
  D.2/D.3 (assert the `→false` random-path rows, not only the present branch —
  §2.2); the 5 folder shapes incl. *n*-level nesting on create/get (§2 preamble);
  and the **both-version** assertions in J.2 (§2.8), K.2 (§2.9) that today check
  only v1. Realize each row as a distinct `case × data set` per master03 §API.

- **G-3 — `get_versioned_directory` container semantics (normative-invariant),
  re-verify the ITS-REST binding (edition-specific).** L.2 requires a
  VERSIONED_OBJECT referencing every version. First re-verify against the
  vendored OAS whether a `versioned_directory` resource exists on any edition of
  the ladder (the ECC assumes none); if it does on RM 1.2.0/development, drive it
  and assert the two `versions` references. If it genuinely does not, keep the
  D2 rebind **but** additionally assert version *count/reachability* so L.2's
  intent is approximated, and keep the `schedule_ref`. The binding's existence is
  edition-specific; the "references all versions" requirement is normative-
  invariant.

- **G-4 — centralize version-uid / ETag extraction (W-3f lesson).**
  `support::version_uid` (`support.rs:116`) does ad-hoc weak/bare-ETag stripping
  and `uid.value` fallback; `ehr_with_two_versions` (`directory.rs:529`) then
  swallows its error (`unwrap_or_else(|_| v1.clone())`, `:544`) — a silent
  fallback that can mask a missing v2 uid and make a version case pass on the
  wrong id. Route all header/ETag/id scraping through one helper; never fall back
  silently. **Weak `W/"…"` vs bare-quoted ETag is edition-specific** (ITS-REST
  overview §"ETag and Last-Modified") — the helper owns the edition flag.

- **G-5 — boolean-op REST collapse is edition-specific.** `has_directory` C.1
  (`false`) and C.3 (`error`) both become 404 (§2.1); same collapse as the
  CONTRIBUTION `has_contribution` (register 05 §G-7). Record that the SM boolean
  op's false/error trichotomy is realized as 200/404 by element-2 mapping (an
  edition/binding property); the native `ehrbase-sm` surface preserves the
  distinction — the rewrite must not "fix" a 404 into a 200-false.

- **G-6 — assert returned body fidelity, not just status (normative-invariant).**
  F.3 (ECC-DIR-023), K.2 (ECC-DIR-031) assert only the status; the schedule wants
  "the full structure" / "the first version". Add a body comparison of the
  returned FOLDER tree / version to the committed data (canonical-JSON compare).
  Structure fidelity is normative-invariant; DV/leaf wire shapes inside the
  FOLDER are RM-version-sensitive → compare structurally, edition-tag leaf shapes.

- **G-7 — pin update/delete error codes (edition-specific).** H.2/H.3 (update)
  and I.1/I.3 (delete) use `status_in(&[400,404,412])`. The rewrite pins per
  cause: non-existent EHR/directory → **404**; failed/absent `If-Match`
  precondition → **412**; malformed → **400**. The 4xx *class* is normative-
  invariant; the exact code (and whether `If-Match` is even required on
  DELETE) is edition-specific → assert the code on the matched edition, fall
  back to the class otherwise. Note `If-Match` on DIRECTORY carries the
  **OBJECT_VERSION_ID** (not a weak ETag) — a strong-validator, edition-checked
  behaviour.

- **G-8 — assert the logical-delete post-condition (normative-invariant).** I.2
  (ECC-DIR-010) must, after the 200/204, re-read the directory and confirm the
  schedule NOTE: "the directory exists as a new deleted version
  (`VERSION.lifecycle_state=deleted`)" — i.e. a follow-up `GET /directory` shows
  the deleted lifecycle (or 404 per the chosen realization, recorded as
  edition-specific). Logical-delete truth (RM common master06 §Change control)
  is normative-invariant.
