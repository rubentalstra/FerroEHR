# Conformance register 01 — DEFINITION / ADL component (`suites/definition_adl14.rs`): spec-first audit

W-10 area audit (read-only, 2026-07-13) of the **DEFINITION / ADL (OPT 1.4/2)
provisioning** component of `tools/conformance`. Method is spec-first (README +
owner ruling): the spine below is the governing CNF schedule chapter enumerated
test-case-by-test-case; the existing ECC cases are mapped **onto** each schedule
item with a `file:line` verdict (conformant / divergent / missing /
instrument-encodes-server-behaviour). §3 lists ECC cases with no schedule home;
§4 carries the G-rows for the rewrite, marking every edition-/version-specific
assertion the version-ladder runner must know about.

**Spec oracles** (read before any change):

- `docs/specs/openehr/CNF/docs/platform_test_schedule/master04-func_tc_definition_adl.adoc`
  — the DEFINITION/ADL test suite. Titled "**I_DEFINITION_ADL2 and
  I_DEFINITION_ADL14 Interfaces**", but its body defines test cases for
  **`I_DEFINITION_ADL14` only** (§OPT 1.4/2 Test cases). Read whole. Its
  §Test Environment states template-id formats are server-specific ("openEHR not
  yet defining a format for the template IDs") and that OPT 1.4/2 cases are the
  same but written with separate data sets.
- `docs/specs/openehr/CNF/docs/platform_test_schedule/master03-overview.adoc`
  — the test-case form (`<SERVICE_COMPONENT>.<operation>-<id>`) and the
  RM-version note (§API Conformance: "minimum required version is RM 1.0.2").
- `docs/specs/openehr/CNF/docs/profiles/master03-profiles.adoc` — the
  capability × CORE/STANDARD/OPTIONS matrix. §Functional/Definitions:
  **ADL 1.4 Archetype provisioning = CORE+STANDARD**; **ADL 1.4 OPT
  provisioning = CORE+STANDARD**; **ADL 2 Archetype provisioning = OPTIONS**;
  **ADL 2 OPT provisioning = OPTIONS**. §REST APIs: **DEFINITION API =
  CORE+STANDARD**.

**Mapped suite:** `tools/conformance/src/suites/definition_adl14.rs` (16 ECC-TPL
entries, area `Tpl`) + the shared `fixtures` OPT loaders.

---

## 1. Verdict

The ADL 1.4 suite is **operation-complete against the schedule's realizable
surface**: all 16 master04 `I_DEFINITION_ADL14` test cases have a 1:1 ECC-TPL
entry, with the four `delete_opt` cases honestly adjudicated as
skip-with-reason (the SM `delete_opt()` has no ITS-REST ADL 1.4 DELETE binding —
deletion is ADMIN-API-only; module docs `definition_adl14.rs:16-21`, D2). The
gaps are not missing cases but **shallow post-condition assertions and
version-semantics the ADL 1.4 wire cannot express**: the "retrieved OPT equals
the uploaded one" equality (master04 §get_opt-retrieve_single NOTE) is never
checked (status-only); the two-version cases (`upload_opt-valid_opt_twice_no_conflict`,
`get_opt-retrieve_latest_version`/`-retrieve_specific_version`) tolerate a
widened status set because ITS-REST ADL 1.4 has no version parameter, so they
encode our server's unversioned behaviour rather than asserting the schedule's
two-coexisting-versions post-condition; the empty-server precondition
(`get_opts-retrieve_all_no_opts`) cannot hold on a shared SUT and is degraded to
a status check. The `template_id` is a hardcoded literal (`TID`,
`definition_adl14.rs:173`) despite the schedule declaring the format
server-specific. Separately, the **ADL 2 half of the chapter title is
unrealized** — the schedule defines no ADL 2 test cases and the suite has none,
though the model carries an `Adl2Provisioning` capability. The rewrite is about
**round-trip equality, sourcing the OPT data-set classes, edition-aware
version/template-id handling, and deciding the ADL 2 gap** — not adding raw
coverage.

---

## 2. The spine (master04 test cases → ECC map)

Schedule ids use the chapter's own form (`I_DEFINITION_ADL14.<op>-<id>`).
Data-set classes are from each operation's §Data set(s). Capability/profile from
master03-profiles. ECC file:line is in `suites/definition_adl14.rs`.

### `I_DEFINITION_ADL14.validate_opt()` — ADL 1.4 OPT provisioning · CORE+STANDARD

master04 §validate_opt data sets: minimal-valid (each entry type), maximal-valid
(all RM types), + four invalid classes (empty file, empty `template_id`, removed
mandatory elements, added-over-upper-bound-1). The chapter §validate_opt NOTE
explicitly sanctions realizing validation via the upload endpoint when a server
has no standalone validate service.

| Schedule case | Normative condition | Data sets | ECC map — verdict |
|---|---|---|---|
| `I_DEFINITION_ADL14.validate_opt-valid_opt` | Each valid OPT → positive ("OPT is valid"); no state change | valid: minimal, maximal | `ECC-TPL-011` `tpl/validate-opt-valid-opt` (`definition_adl14.rs:283`) — **conformant (note)**: realized via upload (2xx/409 both prove validation passed), which the schedule NOTE permits. Uses only `minimal_evaluation.opt`, not the maximal class (G-3). |
| `I_DEFINITION_ADL14.validate_opt-invalid_opt` | Each invalid OPT → negative ("OPT is invalid") | invalid: 4 classes | `ECC-TPL-012` `tpl/validate-opt-invalid-opt` (`definition_adl14.rs:300`) — **conformant**: uploads the first invalid `.opt` fixture, asserts `4xx`. Exercises one fixture, not each invalid class distinctly (G-3). |

### `I_DEFINITION_ADL14.upload_opt()` — ADL 1.4 OPT provisioning · CORE+STANDARD

master04 §upload_opt data sets add "minimal valid OPT, two versions". The
two-versions cases hinge on a version parameter the chapter admits is
non-standard (§upload_opt-valid_opt_twice NOTE cites SPECBASE-30 / SPECITS-42).

| Schedule case | Normative condition | Data sets | ECC map — verdict |
|---|---|---|---|
| `I_DEFINITION_ADL14.upload_opt-valid_opt` | Valid OPT accepted, stored as uploaded | valid: minimal, maximal | `ECC-TPL-001` `tpl/upload-opt-valid-opt` (`definition_adl14.rs:416`) — **conformant**: retargets the OPT `template_id` to a fresh UUID via `openehr_its::opt14` so a shared-SUT re-run genuinely asserts a *fresh* 201 (order-independent). Also the D5 CORE `Adl14ArchetypeProvisioning` evidence (archetypes provisioned inside the OPT; module docs `definition_adl14.rs:23-33`). |
| `I_DEFINITION_ADL14.upload_opt-invalid_opt` | Invalid OPT rejected; no state change | invalid: 4 classes | `ECC-TPL-002` `tpl/upload-opt-invalid-opt` (`definition_adl14.rs:444` → `upload_invalid_set`, `:449`) — **conformant**: iterates every invalid `.opt` fixture, requires each `4xx`; reports `passed/total`. The strongest case in the suite. |
| `I_DEFINITION_ADL14.upload_opt-valid_opt_twice_conflict` | Same `template_id` twice, no version → second negative (conflict) | valid, id-providing | `ECC-TPL-004` `tpl/upload-opt-valid-opt-twice-conflict` (`definition_adl14.rs:190`) — **conformant**: ensure-present then re-upload same id → asserts `409`. |
| `I_DEFINITION_ADL14.upload_opt-valid_opt_twice_no_conflict` | Same `template_id`, **different version param** → both positive, two versions coexist | valid, two versions | `ECC-TPL-005` `tpl/upload-opt-valid-opt-twice-no-conflict` (`definition_adl14.rs:207`) — **instrument-encodes-server-behaviour**: ITS-REST ADL 1.4 has no version parameter, so the case re-uploads the *identical* OPT and accepts `[200,204,409]` (`:215`), never producing two coexisting versions. It cannot assert the schedule post-condition ("two new OPTs … different versions"); it encodes our unversioned semantics (G-2). |

### `I_DEFINITION_ADL14.get_opt()` — ADL 1.4 OPT provisioning · CORE+STANDARD

master04 §get_opt NOTE (retrieve_single): "the retrieved OPT should be exactly
the same as the uploaded one"; retrieve_latest/specific require versioned OPTs
loaded and a "small modification" to prove which version returned.

| Schedule case | Normative condition | Data sets | ECC map — verdict |
|---|---|---|---|
| `I_DEFINITION_ADL14.get_opt-retrieve_single` | Existing `template_id` → the correct OPT, **byte/semantic-identical to uploaded** | single-version OPTs loaded | `ECC-TPL-006` `tpl/get-opt-retrieve-single` (`definition_adl14.rs:232`) — **divergent (shallow)**: asserts `200` only; the NOTE's uploaded==retrieved equality is never checked (G-3). |
| `I_DEFINITION_ADL14.get_opt-retrieve_fail` | Empty server, random `template_id` → error (non-existence) | none loaded | `ECC-TPL-009` `tpl/get-opt-retrieve-fail` (`definition_adl14.rs:258`) — **conformant**: `GET …/does.not.exist.v1` → `404`. |
| `I_DEFINITION_ADL14.get_opt-retrieve_latest_version` | Versioned OPT → the **latest** version returned | two-version OPTs loaded | `ECC-TPL-007` `tpl/get-opt-retrieve-latest-version` (`definition_adl14.rs:240`) — **instrument-encodes-server-behaviour**: identical body to retrieve_single (`200` only); no versioned OPT is loaded and no modification proves latest-ness — the concept has no ADL 1.4 wire (G-2). |
| `I_DEFINITION_ADL14.get_opt-retrieve_specific_version` | Versioned OPT + non-latest version param → that specific version | two-version OPTs loaded | `ECC-TPL-008` `tpl/get-opt-retrieve-specific-version` (`definition_adl14.rs:248`) — **instrument-encodes-server-behaviour**: `GET …/{tid}/1.0.0` accepts `[200,404]` (`:254`) because ADL 1.4 OPTs are not version-addressed in ITS-REST; tolerant by construction (G-2). |

### `I_DEFINITION_ADL14.get_opts()` — ADL 1.4 OPT provisioning · CORE+STANDARD

| Schedule case | Normative condition | Data sets | ECC map — verdict |
|---|---|---|---|
| `I_DEFINITION_ADL14.get_opts-retrieve_all` | All loaded OPTs returned; only latest version of each | all valid OPTs loaded | `ECC-TPL-010` `tpl/get-opts-retrieve-all` (`definition_adl14.rs:269`) — **divergent (shallow)**: ensure-present then `GET /definition/template/adl1.4` → asserts `200` only; does not verify the loaded OPT is *in* the list or the latest-only rule (G-3). |
| `I_DEFINITION_ADL14.get_opts-retrieve_all_no_opts` | Empty server → empty set, no failure | none loaded | `ECC-TPL-003` `tpl/get-opts-retrieve-all-no-opts` (`definition_adl14.rs:478`) — **divergent (precondition)**: asserts `200` only, on a shared SUT where "no OPTs loaded" cannot be guaranteed and the empty-set body is not asserted (G-4). |

### `I_DEFINITION_ADL14.delete_opt()` — ADL 1.4 OPT provisioning · CORE+STANDARD

master04 §delete_opt defines four cases (delete_existing, delete_latest_version,
delete_specific_version, delete_non_existing) with a versioned-OPT-cascade note.

| Schedule case | Normative condition | Data sets | ECC map — verdict |
|---|---|---|---|
| `I_DEFINITION_ADL14.delete_opt-delete_existing` | Delete each existing `template_id`; verify gone via get_opts | all valid OPTs loaded | `ECC-TPL-014` `tpl/delete-opt-delete-existing` (`definition_adl14.rs:143`) — **skip-with-reason (D2, conformant handling)**: no ITS-REST ADL 1.4 DELETE verb; `.with_schedule_ref(DELETE_OPT_SCHEDULE_REF)` traces the SM op. |
| `I_DEFINITION_ADL14.delete_opt-delete_latest_version` | Delete all versions when no version param | versioned OPT loaded | `ECC-TPL-015` `tpl/delete-opt-delete-latest-version` (`definition_adl14.rs:150`) — **skip-with-reason (D2)**. |
| `I_DEFINITION_ADL14.delete_opt-delete_specific_version` | Delete a non-latest version; latest still retrievable | versioned OPT loaded | `ECC-TPL-016` `tpl/delete-opt-delete-specific-version` (`definition_adl14.rs:157`) — **skip-with-reason (D2)**. |
| `I_DEFINITION_ADL14.delete_opt-delete_non_existing` | Delete a non-existent `template_id` → error | none loaded | `ECC-TPL-013` `tpl/delete-opt-delete-non-existing` (`definition_adl14.rs:136`) — **skip-with-reason (D2)**. |

**Schedule coverage:** 16/16 master04 `I_DEFINITION_ADL14` test cases mapped;
**0 missing**. Verdicts: 5 conformant (TPL-001, -002, -004, -009, -011,
-012 — six counting the sanctioned validate-via-upload), 3 divergent-shallow
(TPL-006, -010, -003), 3 instrument-encodes-server-behaviour (TPL-005, -007,
-008), 4 skip-with-reason (TPL-013..016). **ADL 2 (I_DEFINITION_ADL2):
schedule defines 0 test cases → 0 ECC cases** (G-1).

---

## 3. Existing ECC cases with no schedule home

None. Every ECC-TPL entry maps to a master04 `I_DEFINITION_ADL14` test case.
The D5 archetype-provisioning evidencing rides on `ECC-TPL-001` (a real
`upload_opt-valid_opt` case tagged `Capability::Adl14ArchetypeProvisioning`
rather than a separate no-home case), so it is not an orphan — see §4 G-5.

---

## 4. G-rows — gaps + rulings for the rewrite

- **G-1 (ADL 2 is unrealized — schedule + suite gap). CAPABILITY-CLAIMABILITY.**
  master04 is titled for both `I_DEFINITION_ADL2` and `I_DEFINITION_ADL14` but
  defines test cases for **ADL 1.4 only** (§OPT 1.4/2 Test cases contains only
  `I_DEFINITION_ADL14.*`). The profiles matrix lists **ADL 2 Archetype
  provisioning** and **ADL 2 OPT provisioning** as OPTIONS capabilities, and the
  model carries `Capability::Adl2Provisioning` (`model/case.rs:110`), but no ECC
  case exercises it. The rewrite must either add ADL 2 OPT/archetype provisioning
  cases (against the ITS-REST `/definition/template/adl2` surface, OPTIONS) so
  the capability is claimable from real passing cases, or record a PORT NOTE that
  the schedule itself omits ADL 2 and the OPTIONS capability is therefore
  unevidenceable until upstream fills the chapter.

- **G-2 (OPT versioning has no ADL 1.4 wire — three cases encode our behaviour).
  EDITION-SPECIFIC.** `upload_opt-valid_opt_twice_no_conflict` (`:207`),
  `get_opt-retrieve_latest_version` (`:240`), and `get_opt-retrieve_specific_version`
  (`:248`) all target a version parameter master04 admits is non-standard
  (§upload_opt-valid_opt_twice NOTE, SPECBASE-30/SPECITS-42); ITS-REST ADL 1.4
  exposes no version-addressed template resource, so the cases widen their status
  sets (`[200,204,409]`, `[200,404]`) and never assert the schedule's
  "two coexisting versions" / "latest returned" / "specific returned"
  post-conditions. The rewrite must express the version-param form per edition
  (some editions encode version in the `template_id` or `other_details`), try
  highest-first, and record the satisfied form as an edition finding — or, where
  no edition offers it, PORT-NOTE the schedule case as structurally
  unrealizable rather than passing a tolerant status check.

- **G-3 (shallow post-conditions — no round-trip equality).** `get_opt-retrieve_single`
  (`ECC-TPL-006`, `:232`) asserts `200` only; master04 §get_opt-retrieve_single
  NOTE requires "the retrieved OPT should be exactly the same as the uploaded
  one". `get_opts-retrieve_all` (`ECC-TPL-010`) does not assert the uploaded OPT
  is present or the latest-only rule. `validate_opt-*` / `upload_opt-*` use only
  `minimal_evaluation.opt`, not the maximal-valid ("all RM types") class nor the
  four distinct invalid classes as separate data sets. The rewrite adds the
  upload→retrieve semantic-equality check the schedule mandates and drives the
  full data-set matrix (minimal + maximal + each invalid class) from register 80.

- **G-4 (empty-server precondition on a shared SUT + data-set sourcing).**
  `get_opts-retrieve_all_no_opts` (`ECC-TPL-003`) carries the schedule
  precondition "no OPTs should be loaded" which the ECC harness cannot enforce on
  a shared SUT; it degrades to a `200` check and asserts nothing about the empty
  set. Same class of divergence as register 03 G-4 — document it, and where the
  runner can use a scratch tenant/clean SUT mode, assert the empty body. The
  vendored `valid_templates`/`invalid_templates` OPT sets must move to register
  80's data-set strategy.

- **G-5 (delete_opt D2 skip is correct — but the capability needs evidence
  elsewhere).** The four `delete_opt` skips (`:319-354`) are the honest reading
  (no ITS-REST ADL 1.4 DELETE binding; deletion is ADMIN-API-only). Keep them,
  and ensure the ADMIN template-deletion path is exercised in register 09 so OPT
  lifecycle is not wholly unevidenced. The `validate_opt` realization via upload
  (schedule NOTE-sanctioned) should stay recorded as a deliberate mapping, not a
  divergence.

- **G-6 (server-specific `template_id` format hardcoded). EDITION-SPECIFIC.**
  `TID = "minimal_evaluation.en.v1"` (`definition_adl14.rs:173`) is a fixed
  literal, yet master04 §Test Environment note 3 states "openEHR not yet defining
  a format for the template IDs … the template IDs should be adapted to prevent
  failures for wrong format on the template ID" per server. For the
  bring-your-own-endpoint / upstream-EHRbase SUTs (README ruling 1) the `template_id`
  and the ADL 1.4 vs ADL 2 endpoint form must be a per-SUT/per-edition
  configuration input on the version ladder, not baked into the case.

---

*Register 80 owns the OPT data-set strategy referenced by G-3/G-4; register 90
owns the wire-adapter / version-ladder / template-id-config architecture
referenced by G-2/G-6.*
