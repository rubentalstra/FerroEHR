# Conformance register 03 — EHR component (`suites/ehr.rs`): spec-first audit

W-10 area audit (read-only, 2026-07-13) of the **EHR component** of
`tools/conformance`. Method is spec-first (README + owner ruling): the spine
below is the governing CNF schedule chapter enumerated test-case-by-test-case;
the existing ECC cases are mapped **onto** each schedule item with a
`file:line` verdict (conformant / divergent / missing /
instrument-encodes-server-behaviour). §3 lists ECC cases with no schedule home;
§4 carries the G-rows for the rewrite, marking every edition-/version-specific
assertion the version-ladder runner must know about.

**Spec oracles** (read before any change):

- `docs/specs/openehr/CNF/docs/platform_test_schedule/master06-func_tc_ehr.adoc`
  — the EHR test suite (`I_EHR_SERVICE`, `I_EHR`, `I_EHR_STATUS`); read whole.
  Its §Test Data Sets defines the VALID 16-row table (1.a) + default (1.b) +
  INVALID class (2).
- `docs/specs/openehr/CNF/docs/platform_test_schedule/master03-overview.adoc`
  — the test-case form (`<SERVICE_COMPONENT>.<operation>-<id>`) and the
  RM-version note (§API Conformance: "minimum required version is RM 1.0.2";
  supported versions stated in the Conformance Statement).
- `docs/specs/openehr/CNF/docs/profiles/master03-profiles.adoc` — the
  capability × CORE/STANDARD/OPTIONS matrix. EHR Operations, EHR Status,
  Versioning, Change sets = **CORE + STANDARD**; the EHR REST API =
  **CORE + STANDARD**; Anonymous EHRs (non-functional) = **CORE + STANDARD**.

**Mapped suite:** `tools/conformance/src/suites/ehr.rs` (21 schedule-provenance
entries + 2 own extensions) and the shared `suites/support.rs` helpers.

---

## 1. Verdict

The EHR suite is **operation-complete and faithful in coverage**: every one of
the 21 master06 test cases has a 1:1 ECC entry with the correct HTTP status
contract (201+ETag+Location on create, 200 on read, 404 for absent
resources, 409 for duplicate id/subject). The gaps are not missing cases but
**shallow assertions and hardcoded wire assumptions**: the create-EHR data-set
matrix is re-encoded as a Rust literal rather than sourced from the shared
data-set strategy (register 80); the EHR_STATUS mutators scrape the `uid`/ETag
and construct `OBJECT_VERSION_ID` strings inline (`support.rs::version_uid`,
`ehr.rs` `update_status_field`/`update_status_bad_ehr`), the exact
W-3f ETag lesson; and one case (`create-ehr-invalid-status`) bakes a
corpus-vs-spec adjudication into the case body with a hardcoded filename
instead of routing it through the runner's adjudication register. RM-1.2.0 wire
shapes are pinned throughout (`from_canonical_json::<Ehr>`), with no version
ladder. The rewrite is therefore about **centralizing extraction, sourcing data
sets, and moving adjudication out of case bodies** — not about adding coverage.

---

## 2. The spine (master06 test cases → ECC map)

Schedule ids use the chapter's own form. Data-set classes are from master06
§Test Data Sets. Capability/profile from master03-profiles. ECC file:line is in
`suites/ehr.rs` unless noted.

### `I_EHR_SERVICE.has_ehr()` — EHR Operations · CORE+STANDARD

| Schedule case | Normative condition | Data sets | ECC map — verdict |
|---|---|---|---|
| `I_EHR_SERVICE.has_ehr-existing_ehr_id` | EHR with known `ehr_id` exists → positive result | pre: 1 EHR present | `ECC-EHR-001` `ehr/has-ehr-existing-ehr-id` (`ehr.rs:351`) — **conformant** (SM boolean realized as `GET /ehr/{id}` → 200 per master03-overview §API Conformance abstract-call→REST; PUT-create then GET). |
| `I_EHR_SERVICE.has_ehr-existing_subject_id` | EHR with known `subject_id` exists → positive | pre: 1 EHR w/ subject | `ECC-EHR-002` `ehr/has-ehr-existing-subject-id` (`ehr.rs:364`) — **conformant** (`GET /ehr?subject_id=&subject_namespace=conformance` → 200). |
| `I_EHR_SERVICE.has_ehr-non_existing_ehr_id` | Empty server, random `ehr_id` → negative | pre: empty | `ECC-EHR-003` `ehr/has-ehr-non-existing-ehr-id` (`ehr.rs:381`) — **conformant** (404). |
| `I_EHR_SERVICE.has_ehr-non_existing_subject_id` | Empty server, random `subject_id` → negative | pre: empty | `ECC-EHR-004` `ehr/has-ehr-non-existing-subject-id` (`ehr.rs:394`) — **conformant** (404). |

Note: "server should be empty" is a schedule precondition the ECC harness cannot
guarantee on a shared SUT (it uses random UUIDs to stand in for absence). Sound
but a documented divergence from the literal precondition — G-4.

### `I_EHR_SERVICE.create_ehr()` — EHR Operations · CORE+STANDARD

| Schedule case | Normative condition | Data sets | ECC map — verdict |
|---|---|---|---|
| `I_EHR_SERVICE.create_ehr-main` | Create over each VALID data-set class; when `ehr_id` provided it must be unique per call → positive creation | VALID 1.a (16 rows), 1.b (default) | `ECC-EHR-005` `ehr/create-ehr-main` (`ehr.rs:472`) — **divergent**: iterates all 16 rows (201+ETag+Location, re-GET), but the 16-row matrix is a Rust literal `DATA_SETS` (`ehr.rs:414`) with an inline `ehr_status_row` builder (`ehr.rs:434`), not sourced from the vendored corpus / register 80. Does not exercise class 1.b (empty-body default) as a distinct row. |
| `I_EHR_SERVICE.create_ehr-same_ehr_twice` | Same `ehr_id` twice → second is negative (already exists) | VALID (id-providing) | `ECC-EHR-006` `ehr/create-ehr-same-ehr-twice` (`ehr.rs:527`) — **conformant** (PUT 201 then PUT 409). |
| `I_EHR_SERVICE.create_ehr-two_ehrs_same_patient` | Same subject twice → second is negative (EHR already exists for subject) | VALID (subject, no id) | `ECC-EHR-007` `ehr/create-ehr-two-ehrs-same-patient` (`ehr.rs:543`) — **conformant** (409). Encodes the one-EHR-per-subject ruling (platform G-4); the schedule expects a negative here, so consistent. |

### `I_EHR_SERVICE.get_ehr()` — EHR Operations · CORE+STANDARD

| Schedule case | Normative condition | Data sets | ECC map — verdict |
|---|---|---|---|
| `I_EHR_SERVICE.get_ehr-existing_ehr_by_ehr_id` | Known `ehr_id` → retrieve EHR | pre: 1 EHR | `ECC-EHR-008` `ehr/get-ehr-existing-ehr-by-ehr-id` (`ehr.rs:564`) — **conformant** (200 + `from_canonical_json::<Ehr>` validity). |
| `I_EHR_SERVICE.get_ehr-existing_ehr_by_subject_id` | Known `subject_id` → retrieve EHR | pre: 1 EHR w/ subject | `ECC-EHR-009` `ehr/get-ehr-existing-ehr-by-subject-id` (`ehr.rs:577`) — **conformant** (200). Does not assert the returned EHR is the one whose subject matches (identity check absent). |
| `I_EHR_SERVICE.get_ehr-get_ehr_by_invalid_ehr_id` | Empty server, random `ehr_id` → negative | pre: empty | `ECC-EHR-010` `ehr/get-ehr-get-ehr-by-invalid-ehr-id` (`ehr.rs:594`) — **conformant** (404). |
| `I_EHR_SERVICE.get_ehr-get_ehr_by_invalid_subject_id` | Empty server, random `subject_id` → negative | pre: empty | `ECC-EHR-011` `ehr/get-ehr-get-ehr-by-invalid-subject-id` (`ehr.rs:607`) — **conformant** (404). |

### `I_EHR_STATUS.get_ehr_status()` — EHR Status · CORE+STANDARD

| Schedule case | Normative condition | Data sets | ECC map — verdict |
|---|---|---|---|
| `I_EHR_STATUS.get_ehr_status-get_by_ehr_id` | Known `ehr_id` → `EHR_STATUS` matching the create-time rules (subject presence, `is_modifiable`, `is_queryable`) | pre: 1 EHR | `ECC-STA-001` `sta/get-ehr-status-get-by-ehr-id` (`ehr.rs:625`) — **divergent (shallow)**: asserts only `_type == "EHR_STATUS"`; the schedule requires verifying the three flag/subject rules against the create parameters. Not cross-checked. |
| `I_EHR_STATUS.get_ehr_status-bad_ehr` | Empty server, random `ehr_id` → negative | pre: empty | `ECC-STA-002` `sta/get-ehr-status-bad-ehr` (`ehr.rs:640`) — **conformant** (404). |

### `I_EHR_STATUS.set_ehr_queryable()` / `set_ehr_modifiable()` — EHR Status · CORE+STANDARD

| Schedule case | Normative condition | Data sets | ECC map — verdict |
|---|---|---|---|
| `I_EHR_STATUS.set_ehr_queryable-existing_ehr` | After set → `is_queryable == true` | pre: 1 EHR | `ECC-STA-003` `sta/set-ehr-queryable-existing-ehr` (`ehr.rs:655` → `update_status_field`, `ehr.rs:291`) — **conformant** (GET status, flip field, PUT with `If-Match`, re-GET asserts flag). One SM op realized via the whole-object `PUT /ehr_status` (ITS-REST has no discrete verb). |
| `I_EHR_STATUS.set_ehr_queryable-bad_ehr` | Random `ehr_id` → negative | pre: empty | `ECC-STA-004` `sta/set-ehr-queryable-bad-ehr` (`ehr.rs:658` → `update_status_bad_ehr`, `ehr.rs:323`) — **instrument-encodes-server-behaviour**: constructs `If-Match: {ehr_id}::conformance::1` inline (`ehr.rs:334`) assuming our `::conformance::` system id, and accepts `[400,404,412]` — a widened set that tolerates the precondition being evaluated before existence. |
| `I_EHR_STATUS.set_ehr_modifiable-existing_ehr` | After set → `is_modifiable == true` | pre: 1 EHR | `ECC-STA-005` `sta/set-ehr-modifiable-existing-ehr` (`ehr.rs:661`) — **conformant** (as STA-003). |
| `I_EHR_STATUS.set_ehr_modifiable-bad_ehr` | Random `ehr_id` → negative | pre: empty | `ECC-STA-006` `sta/set-ehr-modifiable-bad-ehr` (`ehr.rs:664`) — **instrument-encodes-server-behaviour** (as STA-004). |

### `I_EHR_STATUS.clear_ehr_queryable()` / `clear_ehr_modifiable()` — EHR Status · CORE+STANDARD

| Schedule case | Normative condition | Data sets | ECC map — verdict |
|---|---|---|---|
| `I_EHR_STATUS.clear_ehr_queryable-existing_ehr` | After clear → `is_queryable == false` | pre: 1 EHR | `ECC-STA-007` `sta/clear-ehr-queryable-existing-ehr` (`ehr.rs:667`) — **conformant** (target=false). |
| `I_EHR_STATUS.clear_ehr_queryable-bad_ehr` | Random `ehr_id` → negative | pre: empty | `ECC-STA-008` `sta/clear-ehr-queryable-bad-ehr` (`ehr.rs:670`) — **instrument-encodes-server-behaviour** (as STA-004). |
| `I_EHR_STATUS.clear_ehr_modifiable-existing_ehr` | After clear → `is_modifiable == false` | pre: 1 EHR | `ECC-STA-009` `sta/clear-ehr-modifiable-existing-ehr` (`ehr.rs:673`) — **conformant**. |
| `I_EHR_STATUS.clear_ehr_modifiable-bad_ehr` | Random `ehr_id` → negative | pre: empty | `ECC-STA-010` `sta/clear-ehr-modifiable-bad-ehr` (`ehr.rs:676`) — **instrument-encodes-server-behaviour** (as STA-004). |

**Schedule coverage:** 21/21 master06 test cases mapped; **0 missing**.

---

## 3. Existing ECC cases with no schedule home

| ECC | Suite | Nature | Flag |
|---|---|---|---|
| `ECC-EHR-012` `ehr/create-ehr-invalid-status` (`ehr.rs:176`, run `ehr.rs:682`) | EHR | Fixture-derived negative over the vendored INVALID `EHR_STATUS` data sets (master06 §Test Data Sets **class 2**). No single master06 *test case* covers class 2 (it names invalid data-set *shapes*, not a test case), so this is a data-set-class case, correctly anchored to the schedule's data-set section. | **Keep, re-home**: the case bakes a corpus-vs-spec adjudication (`001_ehr_status_subject_empty.json` accepted as spec-valid anonymous PARTY_SELF) with a hardcoded filename + a `TRIAGE.md` cross-ref in the body — this belongs in the runner's adjudication register (register 80 / §4 G-2), not in the case. |
| `ECC-EHR-013` `ehr/create-anonymous-ehr` (`ehr.rs:194`, run `ehr.rs:512`) | EHR | Evidences the **Anonymous EHRs** non-functional capability (master03-profiles §Non-Functional, CORE+STANDARD). No master06 functional test case exists for it. | **Keep**: valid capability-evidencing case (D5-style). Asserts default `EHR_STATUS.subject` carries no `external_ref` — this is a spec-derived check (RM ehr master04 §EHR Status / common §PARTY_SELF), not server-specific, but the exact JSON shape is RM-1.2.0 (§4 G-3). |

---

## 4. G-rows — gaps + rulings for the rewrite

- **G-1 (extraction centralization — the W-3f ETag lesson). EDITION-SPECIFIC.**
  `support.rs::version_uid` (`support.rs:116`) locally parses the `ETag`,
  stripping `W/`/`w/` and quotes, then falls back to body `uid.value`; the
  status mutators scrape `uid.value` inline (`ehr.rs:298`) and hand-build
  `OBJECT_VERSION_ID` as `{ehr_id}::conformance::1` (`ehr.rs:334`). Weak
  `W/"…"` vs bare-quoted ETag is an **edition-specific** form (ITS-REST overview
  §"ETag and Last-Modified" makes it weak; the bare form is deprecated but MAY
  appear). The rewrite must centralize all ETag / `OBJECT_VERSION_ID` / id
  extraction in one wire-adapter that records which edition form matched
  (version ladder), never per-case ad-hoc string work.

- **G-2 (adjudication out of case bodies).** `create-ehr-invalid-status`
  (`ehr.rs:690`) hardcodes `SPEC_VALID_ANONYMOUS` and a per-fixture
  accept/reject decision inside the case. Corpus-vs-spec adjudications are an
  owner-mandated runner concern (adjudications register, cf.
  `adjudications/README.md`), not case logic. The rewrite drives the INVALID
  class 2 data sets from register 80 with per-fixture expected-outcome metadata,
  so the case body only asserts "outcome matches the data set's declared
  validity".

- **G-3 (RM wire version pinning — no ladder). VERSION-SPECIFIC.** Every EHR/
  status validity check goes through `from_canonical_json::<Ehr>` /
  RM-1.2.0-shaped literals (`ehr_status_row`, `ehr.rs:434`; the anonymous-subject
  shape, `ehr.rs:518`). master03-overview §API Conformance sets the **minimum at
  RM 1.0.2** and requires the supported version to come from the Conformance
  Statement. The rewrite must express EHR_STATUS / EHR payloads at each
  supported RM edition, try highest-first, and record the satisfied level —
  today RM 1.1.0-era SUTs (e.g. upstream EHRbase) are structurally excluded.

- **G-4 (empty-server precondition + data-set sourcing).** Several cases carry
  the schedule precondition "the server should be empty"; the ECC harness cannot
  enforce this on a shared SUT and substitutes random UUIDs for absence
  (sound, but a documented divergence). Separately, the create-EHR 16-row matrix
  is a Rust literal (`DATA_SETS`, `ehr.rs:414`) that must move to register 80's
  data-set strategy, and class 1.b (empty-body default `EHR_STATUS`) needs a
  distinct row asserting the server-defaulted `is_modifiable=is_queryable=true`
  + `PARTY_SELF` subject (master06 §Test Data Sets note 3).

- **G-5 (shallow read assertions).** `get_ehr_status-get_by_ehr_id`
  (`ECC-STA-001`, `ehr.rs:625`) checks only `_type`; the schedule requires the
  served `EHR_STATUS` to match the create-time subject presence + `is_modifiable`
  + `is_queryable`. `get_ehr-existing_ehr_by_subject_id` (`ECC-EHR-009`) does not
  assert the returned EHR's subject identity. The rewrite adds the create→read
  round-trip equality the schedule mandates.

- **G-6 (bad_ehr negative-code width). EDITION-SPECIFIC tolerance.** The four
  `set/clear …-bad_ehr` cases accept `[400,404,412]` (`ehr.rs:338`). The schedule
  wants a negative for a non-existent EHR; 404 (resource absent) vs 412
  (precondition evaluated first) is an implementation-order distinction. The
  rewrite records which code the SUT returns as an edition finding rather than
  masking three codes behind one assertion.

---

*Register 80 owns the data-set strategy referenced by G-2/G-4; register 90 owns
the wire-adapter/version-ladder architecture referenced by G-1/G-3/G-6.*
