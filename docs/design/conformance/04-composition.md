# Conformance register 04 — COMPOSITION (`suites/composition.rs`): spec-first audit

W-10 area audit (read-only, 2026-07-13) of the **COMPOSITION component** of
`tools/conformance`. Method is spec-first (README + owner ruling): the spine
below is the governing CNF schedule chapter enumerated test-case-by-test-case;
the existing ECC cases are mapped **onto** each schedule item with a
`file:line` verdict (conformant / divergent / missing /
instrument-encodes-server-behaviour). §3 lists ECC cases with no schedule home;
§4 carries the G-rows for the rewrite, marking every edition-/version-specific
assertion the version-ladder runner must know about.

**Spec oracles** (read before any change):

- `docs/specs/openehr/CNF/docs/platform_test_schedule/master07-func_tc_ehr_composition.adoc`
  — the `I_EHR_COMPOSITION` test suite; read whole. Depends on the Definitions
  (OPT) and EHR suites for preconditions. Key normative demands: **content
  check** ("the retrieved format should contain all the exact same data as the
  format used when committing"), version-number checks, the update
  `preceding_version_uid` semantics + change_type CREATE/MODIFY postconditions,
  and the delete = **logical delete** (`VERSION.lifecycle_state = openehr::523|deleted|`).
- `docs/specs/openehr/CNF/docs/platform_test_schedule/master03-overview.adoc`
  — the test-case form and the RM-version note (minimum RM 1.0.2; supported
  version from the Conformance Statement).
- `docs/specs/openehr/CNF/docs/profiles/master03-profiles.adoc` — Composition
  Operations, Versioning, Change sets = **CORE + STANDARD**; EHR REST API =
  **CORE + STANDARD**. (`get_versioned_composition` reads are tagged as the
  Versioning capability so CORE is claimable.)

**Mapped suite:** `tools/conformance/src/suites/composition.rs` (31
schedule-provenance entries) + shared `suites/support.rs`.

---

## 1. Verdict

The COMPOSITION suite is **broad but assertion-shallow**. All 8 SM operations
are exercised across event/persistent kinds and both JSON+XML on the
round-trip cases, and the status contract is right (201+ETag+Location, 200 read,
404 absent, 4xx negative, version-number checks). But three schedule
postconditions that are the *whole point* of the respective cases are **not
asserted**: the **content check** the schedule attaches to every `get_*` case
(retrieved == committed) is downgraded to a structural `from_canonical_json`
validity check (JSON only; XML reads assert only non-empty body); the
`update_composition-event` change_type CREATE→MODIFY postcondition is
unverified; and the `delete_composition` logical-delete postcondition
(`lifecycle_state = openehr::523|deleted|`) is unverified — the case asserts
only 200/204. The suite also inherits the ad-hoc ETag/uid scraping
(`support.rs::version_uid`, `object_uid`, `assert_version_number`) that the
W-3f review flagged, and pins RM 1.2.0 wire shapes with no version ladder. One
schedule case — the **positive `has_composition`** — has no ECC entry.

---

## 2. The spine (master07 test cases → ECC map)

Schedule ids use the chapter's own form. All cases share the OPT+EHR
preconditions (event OPT `nested.en.v1`; persistent OPT
`persistent_minimal.en.v1`). Capability = Composition Operations (CORE+STANDARD)
unless noted Versioning. ECC file:line in `suites/composition.rs`.

### `I_EHR_COMPOSITION.has_composition()`

| Schedule case | Normative condition | ECC map — verdict |
|---|---|---|
| `I_EHR_COMPOSITION.has_composition` | EHR + CONTRIBUTION + VERSION of known uid exist → result **TRUE** | **MISSING** — no positive `has_composition` ECC case. Coverage leaks into `com/get-composition-latest` (200) but no case realizes the SM boolean-TRUE via `GET /composition/{uid}` on an existing composition. |
| `I_EHR_COMPOSITION.has_composition-bad_composition` | EHR exists, no CONTRIBUTIONs, random `VERSION.uid` → **FALSE** | `ECC-COM-011` `com/has-composition-bad-composition` (`composition.rs:995`) — **conformant** (404 = SM FALSE, per the CNF abstract-call→REST mapping documented at `composition.rs:22`). |
| `I_EHR_COMPOSITION.has_composition-bad_ehr` | No EHRs, random `ehr_id` → error (EHR non-existent) | `ECC-COM-012` `com/has-composition-bad-ehr` (`composition.rs:1009`) — **conformant** (404). |

### `I_EHR_COMPOSITION.get_composition_latest()`

| Schedule case | Normative condition | ECC map — verdict |
|---|---|---|
| `I_EHR_COMPOSITION.get_composition_latest` | VERSIONED_COMPOSITION with 2 VERSIONs → return latest; **content check** vs committed | `ECC-COM-008` `com/get-composition-latest` (`composition.rs:573`), JSON+XML — **divergent**: sets up only 1 version (not the schedule's 2, so "is latest" is untested), and does a structural-validity check (`check_composition`, `composition.rs:437`), **not** the mandated content-equality check. XML path asserts non-empty only. |
| `I_EHR_COMPOSITION.get_composition_latest-bad_composition` | EHR, no CONTRIBUTIONs, random VC uid → empty/error | `ECC-COM-009` `com/get-composition-latest-bad-composition` (`composition.rs:589`) — **conformant** (404). |
| `I_EHR_COMPOSITION.get_composition_latest-bad_ehr` | No EHRs → error | `ECC-COM-010` `com/get-composition-latest-bad-ehr` (`composition.rs:603`) — **conformant** (404). |

### `I_EHR_COMPOSITION.get_composition_at_time()`

| Schedule case | Normative condition | ECC map — verdict |
|---|---|---|
| `I_EHR_COMPOSITION.get_composition_at_time` | At **current** time → latest version + content check | `ECC-COM-013` `com/get-composition-at-time` (`composition.rs:628`), JSON+XML — **divergent** (structural check only, no content-equality; see G-1). |
| `I_EHR_COMPOSITION.get_composition_at_time-no_time_arg` | No time → latest version + content check | `ECC-COM-014` `com/get-composition-at-time-no-time-arg` (`composition.rs:647`), JSON+XML — **divergent** (same). |
| `I_EHR_COMPOSITION.get_composition_at_time-bad_composition` | EHR, no CONTRIBUTIONs → empty/error | `ECC-COM-015` `com/get-composition-at-time-bad-composition` (`composition.rs:664`) — **conformant** (404). |
| `I_EHR_COMPOSITION.get_composition_at_time-bad_ehr` | No EHRs → error | `ECC-COM-016` `com/get-composition-at-time-bad-ehr` (`composition.rs:681`) — **conformant** (404). |
| `I_EHR_COMPOSITION.get_composition_at_times` | 2 VERSIONs at t0<t1: time<t0 → negative; t0<t<t1 → v1; t>t1 → v2; content check each | `ECC-COM-017` `com/get-composition-at-times` (`composition.rs:698`) — **divergent/encodes-behaviour**: the three time points + version-uid resolution are correct, but "before t0" accepts `[204,400,404]` (`composition.rs:730`) where the schedule frames it as a negative/error (204 is a server-choice tolerance), and it resolves versions by uid equality rather than content check. |

### `I_EHR_COMPOSITION.get_composition_version()`

| Schedule case | Normative condition | ECC map — verdict |
|---|---|---|
| `I_EHR_COMPOSITION.get_composition_version` | Known version id → the VERSION's COMPOSITION + content check | `ECC-COM-018` `com/get-composition-version` (`composition.rs:772`), JSON+XML — **divergent** (structural check only; see G-1). |
| `I_EHR_COMPOSITION.get_composition_version-bad_version` | EHR, no commits, random version id → negative | `ECC-COM-019` `com/get-composition-version-bad-version` (`composition.rs:787`) — **instrument-encodes-server-behaviour**: builds the bogus id as `{uuid}::conformance::1` (`composition.rs:790`), assuming our `::conformance::` system id → 404. |
| `I_EHR_COMPOSITION.get_composition_version-bad_ehr` | No EHRs → negative | `ECC-COM-020` `com/get-composition-version-bad-ehr` (`composition.rs:802`) — **instrument-encodes-server-behaviour** (same `::conformance::` construction). |
| `I_EHR_COMPOSITION.get_composition_versions` | 2 VERSIONs v1,v2: each id retrieves its own + content check | `ECC-COM-021` `com/get-composition-versions` (`composition.rs:816`) — **divergent**: correctly retrieves both by uid, but matches on returned version-uid, not the mandated per-version content check. |

### `I_EHR_COMPOSITION.get_versioned_composition()` — capability **Versioning**

| Schedule case | Normative condition | ECC map — verdict |
|---|---|---|
| `I_EHR_COMPOSITION.get_versioned_composition` | Known VC uid → valid VERSIONED_COMPOSITION referencing its VERSION(s) | `ECC-COM-022` `com/get-versioned-composition` (`composition.rs:853`), JSON+XML, cap `Versioning` — **divergent (shallow)**: asserts 200 + non-empty body only; does not validate the VERSIONED_COMPOSITION shape or that it references the committed VERSION. (F-05-06 note: version-family XML shape.) |
| `I_EHR_COMPOSITION.get_versioned_composition-non_existent` | EHR, no commits, random VC uid → negative | `ECC-COM-023` `com/get-versioned-composition-non-existent` (`composition.rs:873`), cap `Versioning` — **conformant** (404). |
| `I_EHR_COMPOSITION.get_versioned_composition-bad_ehr` | No EHRs → negative | `ECC-COM-024` `com/get-versioned-composition-bad-ehr` (`composition.rs:890`), cap `Versioning` — **conformant** (404). |

### `I_EHR_COMPOSITION.create_composition()`

| Schedule case | Normative condition | ECC map — verdict |
|---|---|---|
| `I_EHR_COMPOSITION.create_composition-event` | Valid event COMPOSITION vs existing OPT → positive, **version number 1** | `ECC-COM-001` `com/create-composition-event` (`composition.rs:474`), JSON+XML — **conformant** (201+ETag+Location, `assert_version_number(…,1)`). |
| `I_EHR_COMPOSITION.create_composition-persistent` | Valid persistent COMPOSITION → positive, version 1 | `ECC-COM-002` `com/create-composition-persistent` (`composition.rs:478`), JSON+XML — **conformant**. |
| `I_EHR_COMPOSITION.create_composition-same_opt_twice` | Second persistent create for same OPT → negative (only one create allowed) | `ECC-COM-003` `com/create-composition-same-opt-twice` (`composition.rs:494`) — **conformant** (201 then `assert_negative` 4xx). Note: schedule §Notes flags this as under-debate in the openEHR SEC. |
| `I_EHR_COMPOSITION.create_composition-invalid_event` | Invalid event COMPOSITION → negative w/ error info | `ECC-COM-004` `com/create-composition-invalid-event` (`composition.rs:508`) — **conformant** (posts vendored `__invalid_wrong_structure`, `assert_negative`). |
| `I_EHR_COMPOSITION.create_composition-invalid_persistent` | Invalid persistent COMPOSITION → negative | `ECC-COM-005` `com/create-composition-invalid-persistent` (`composition.rs:512`) — **conformant**. |
| `I_EHR_COMPOSITION.create_composition-event_bad_opt` | COMPOSITION references a missing OPT → negative w/ non-existent-OPT info | `ECC-COM-006` `com/create-composition-event-bad-opt` (`composition.rs:533`) — **conformant** (`[404,422]`; our validation returns 422). |
| `I_EHR_COMPOSITION.create_composition-event_bad_ehr` | Valid COMPOSITION, random `ehr_id` → negative (non-existent EHR) | `ECC-COM-007` `com/create-composition-event-bad-ehr` (`composition.rs:555`) — **conformant** (404). |

### `I_EHR_COMPOSITION.update_composition()`

| Schedule case | Normative condition | ECC map — verdict |
|---|---|---|
| `I_EHR_COMPOSITION.update_composition-event` | Create then update (via `preceding_version_uid`) → 2 VERSIONs; **change_type CREATE then MODIFY** | `ECC-COM-025` `com/update-composition-event` (`composition.rs:909` → `run_update_ok`, `composition.rs:918`) — **divergent**: asserts 200/204 + version number 2, but **not** the change_type CREATE/MODIFY postcondition the case exists to prove (see G-2). |
| `I_EHR_COMPOSITION.update_composition-persistent` | Persistent create+update → 2 VERSIONs | `ECC-COM-026` `com/update-composition-persistent` (`composition.rs:913`) — **divergent** (version number 2 only; no change_type check). |
| `I_EHR_COMPOSITION.update_composition-non_existent` | Random `preceding_version_uid` → negative (non-existent preceding version) | `ECC-COM-027` `com/update-composition-non-existent` (`composition.rs:928`) — **instrument-encodes-server-behaviour**: builds `preceding_version_uid` as `{uuid}::conformance::1` (`composition.rs:933`), accepts `[404,412]`. |
| `I_EHR_COMPOSITION.update_composition-wrong_template` | Update body referencing a different template → negative (template_id mismatch) | `ECC-COM-028` `com/update-composition-wrong-template` (`composition.rs:940`) — **conformant** (event created, updated with persistent body → `assert_negative`). Does not assert the error is specifically a `template_id` mismatch. |

### `I_EHR_COMPOSITION.delete_composition()`

| Schedule case | Normative condition | ECC map — verdict |
|---|---|---|
| `I_EHR_COMPOSITION.delete_composition-event` | Create then delete → 2 VERSIONs; **2nd `VERSION.lifecycle_state = openehr::523|deleted|`** (logical delete) | `ECC-COM-029` `com/delete-composition-event` (`composition.rs:955` → `run_delete_ok`, `composition.rs:966`) — **divergent**: asserts only 200/204; the logical-delete postcondition (a deleted VERSION with `lifecycle_state`/`change_type` = `openehr::523`) is unverified (see G-2). |
| `I_EHR_COMPOSITION.delete_composition-persistent` | Persistent create+delete → deleted VERSION | `ECC-COM-030` `com/delete-composition-persistent` (`composition.rs:959`) — **divergent** (200/204 only). |
| `I_EHR_COMPOSITION.delete_composition-non_existent` | Random `preceding_version_uid` → negative (non-existent COMPOSITION) | `ECC-COM-031` `com/delete-composition-non-existent` (`composition.rs:978`) — **instrument-encodes-server-behaviour**: `{uuid}::conformance::1` version segment, accepts `[404,409,412]`. |

**Schedule coverage:** 31/32 master07 test cases mapped; **1 missing**
(`I_EHR_COMPOSITION.has_composition`, positive).

---

## 3. Existing ECC cases with no schedule home

None. All 31 `ECC-COM-*` entries trace to a master07 test case. (Registers 05
Contribution / 06 Directory cover the sibling versioned-object surfaces; the
COMPOSITION register is clean of extensions.)

---

## 4. G-rows — gaps + rulings for the rewrite

- **G-1 (the content check is not implemented — the biggest gap).** master07
  attaches "the retrieved format should contain all the exact same data as the
  format used when committing the COMPOSITION (content check)" to **every**
  `get_composition_*` case. `check_composition` (`composition.rs:437`) instead
  does a structural `from_canonical_json::<Composition>` validity check for JSON
  and **only a non-empty-body check for XML** (`composition.rs:446`). The rewrite
  must implement retrieved-vs-committed content equality (canonical-form
  comparison, tolerant of server-populated committal metadata), and validate XML
  reads against the openEHR XSD as the schedule §Test Environment requires.
  Without this, every `get_*` case is a liveness probe, not a conformance check.

- **G-2 (unverified versioning postconditions).** The `update_composition-event`
  change_type **CREATE→MODIFY** postcondition and the `delete_composition`
  **logical-delete** postcondition (`VERSION.lifecycle_state = openehr::523|deleted|`,
  RM common change-control) are the reason those cases exist, and neither is
  asserted (`run_update_ok`/`run_delete_ok` check only status + version number).
  The rewrite reads back the VERSIONED_COMPOSITION / VERSION and asserts the
  audit `change_type` and `lifecycle_state` codes. The `openehr::523` code string
  is a **terminology-version-sensitive** literal — the version ladder must carry
  the code per openEHR terminology edition.

- **G-3 (`OBJECT_VERSION_ID` construction + ETag scraping — the W-3f lesson).
  EDITION-SPECIFIC.** Negatives fabricate ids as `{uuid}::conformance::1`
  (`composition.rs:790,806,933,982`), hardcoding our `::conformance::` system id;
  reads/writes recover the uid via `support.rs::version_uid` (`support.rs:116`,
  weak-vs-bare ETag stripping) + `object_uid` (`support.rs:140`, split on `::`)
  + `assert_version_number` (`composition.rs:455`, `::`-suffix parse). The
  three-part `OBJECT_VERSION_ID` grammar is RM-invariant, but the **creating-system
  id** segment and the **ETag weak `W/"…"` vs deprecated bare form** are
  edition/server-specific. The rewrite centralizes id/ETag parsing and synthetic-id
  construction in one wire-adapter that records the matched edition form, so no
  case string-builds an OVID or strips an ETag by hand.

- **G-4 (missing positive `has_composition`).** Add the positive
  `I_EHR_COMPOSITION.has_composition` case (existing composition →
  `GET /composition/{uid}` 200 = SM TRUE) to close the 31/32 gap; realize the SM
  boolean via the documented abstract-call→REST mapping (`composition.rs:22`).

- **G-5 (multi-version setup depth).** `get_composition_latest` and
  `get_composition_version` (base cases) set up a single version, so "is the
  **latest**"/"is **that** version" is not actually distinguished; only the
  `_at_times` / `_versions` cases commit two. The rewrite gives the latest/version
  base cases the schedule's 2-VERSION precondition so "latest" is a real
  assertion, and sources the second commit through the shared update helper.

- **G-6 (RM wire version pinning — no ladder). VERSION-SPECIFIC.** Positive
  bodies are the vendored RM-1.2.0-adapted canonical fixtures
  (`Kind::json_fixture`/`xml_fixture`, `composition.rs:342`) and validity uses
  `from_canonical_json::<Composition>`; master03-overview sets the minimum at
  **RM 1.0.2**. The rewrite must offer per-edition COMPOSITION payloads
  (highest-first ladder), recording the satisfied RM level, so RM-1.1.0-era SUTs
  are testable instead of structurally excluded.

---

*Register 80 owns the data-set/fixture strategy referenced by G-5/G-6; register
90 owns the wire-adapter/content-check/version-ladder architecture referenced by
G-1/G-2/G-3/G-6.*
