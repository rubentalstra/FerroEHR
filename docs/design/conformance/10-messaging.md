# Conformance register 10 — Messaging component (`suites/message.rs`): spec-first audit

W-10 area audit (read-only, 2026-07-13) of the **Messaging** component of
`tools/conformance`. Method is spec-first (README + owner ruling): the spine
below is the governing CNF schedule chapter enumerated operation-by-operation;
the existing ECC cases are mapped **onto** each schedule item with a `file:line`
verdict. §3 lists ECC cases with no schedule home; §4 carries the G-rows.

**The governing chapter is a stub.** `master13-func_tc_messaging.adoc` ships
**no concrete test cases** — every one of its SM-operation subsections carries
only placeholder `==== Test Case aaaa` / `bbbb` bodies reading `TBD`, and
`== Dependencies` + `== Test Environment` + `== Test Data Sets` are `TBD` too
(17 `TBD` markers total; blueprint `07-cnf.md` §master13). The chapter also
carries a **schedule defect**: `I_EHR_EXTRACT.export_ehr()` appears **twice**
(lines 51 and 77 — an authoring duplicate), so the file lists 7 subsection
headings for 6 distinct operations. The spine records each stub verbatim (cited)
and derives the honest intent from the operation headings crossed with the RM
EHR Extract IM + the profiles *Messaging* capability rows. Every row is
therefore **ECC-original (schedule stub)**.

**Messaging is native-API-only in our SUT — how it is handled today.** openEHR
Messaging is an OPTIONS-profile capability with **no ITS-REST 1.0.3 binding**:
EHR Extract / TDD are realized on the `ehrbase-sm` native API only
(`I_EHR_EXTRACT_SERVICE` / `I_TDD_SERVICE`), and there is **no REST route** in
`ehrbase-rest` that reaches export/import/TDD (`message.rs` module docs). The ECC
drives SUTs over HTTP only, so **no part of Messaging is wire-exercisable**. The
current instrument's ruling (carried forward): every MSG case reports
`SKIPPED(NativeApiOnly)` and cites the real `app/ehrbase` testcontainer
integration test that proves the operation, so the capability's evidence is
**traceable off the wire, never fabricated**. This is the precedent registers 09
(admin native ops) and 11 (signing/terminology faults) reuse.

**Spec oracles** (read before any change):

- `docs/specs/openehr/CNF/docs/platform_test_schedule/master13-func_tc_messaging.adoc`
  — the MESSAGE_SERVICE suite; read whole. §Normative Reference names the
  abstract interfaces `I_EHR_EXTRACT`, `I_TDD` and the RM EHR Extract / EHR /
  Demographic / Common / Data-Structures / Data-Types / Support IMs; §Test Cases
  enumerates the export + import operations (all bodies `TBD`).
- `docs/specs/openehr/CNF/docs/platform_test_schedule/master03-overview.adoc`
  — the test-case form (§API Conformance Test Design).
- `docs/specs/openehr/CNF/docs/profiles/master03-profiles.adoc` §Functional —
  *Messaging* (EHR Extract, TDS) = **OPTIONS**; §REST APIs — MESSAGE API =
  **OPTIONS**.

**Mapped suite:** `tools/conformance/src/suites/message.rs` (10 ECC entries,
`ECC-MSG-001..010`, all `SKIPPED(NativeApiOnly)`).

---

## 1. Verdict

The Messaging suite is **correct in disposition and complete in intent**: it
enumerates the EHR-Extract export family (whole-EHR, spec-driven, unknown-EHR
negative), the EHR-Extract import family (clone reusing source id, fixed-id,
duplicate-target negative, extract-into-existing), and the TDD import family
(commit, typed rejections, batch) — the full SM-5 surface — and reports each as
`SKIPPED(NativeApiOnly)` with a named `app/ehrbase` integration test as evidence
(`message.rs:152–212`). This is the *right* answer to a capability with no
ITS-REST binding: neither a fabricated pass nor a silent gap. Every case already
threads a `schedule_ref` to its master13 operation with the `(TBD)` marker
(`message.rs:46` etc.), which registers 08/09 do **not** — MSG is the model here.

The only substantive observation is structural, not behavioural: the schedule
duplicate (`export_ehr` twice) and the schedule/SM naming skew (the chapter says
`I_EHR_EXTRACT`/`I_TDD`; the SM traits our cases cite are
`I_EHR_EXTRACT_SERVICE`/`I_TDD_SERVICE`) must be recorded so the rewrite's
`schedule_ref` values are honest about what the chapter literally contains. And
because Messaging is entirely off the wire, its coverage is **evidence-by-citation
only** — the rewrite must keep the cited integration-test names in lockstep with
`app/ehrbase/tests/` (a stale citation would silently break the traceability that
is the whole point).

---

## 2. The spine (master13 operations → ECC map)

Schedule ids use the overview form `I_EHR_EXTRACT.<operation>-<id>` /
`I_TDD.<operation>-<id>` (the chapter's literal interface names). The concrete
`<id>` is **TBD** in every subsection. Data-set classes: master13 §Test Data
Sets is `TBD` → **derived** (RM EHR Extract IM `X_VERSIONED_*` shapes). Capability
/ profile from `master03-profiles.adoc` §Functional — *Messaging* (OPTIONS).
Every case is `SKIPPED(NativeApiOnly)`. ECC file:line is in `suites/message.rs`.

### `I_EHR_EXTRACT.export_ehr()` — EHR Extract · OPTIONS · **(schedule duplicate)**

Schedule stub: `aaaa`/`bbbb` **`TBD`**, appearing **twice** in the chapter
(master13 §Test Cases, lines 51 and 77 — an authoring duplicate). Derived: export
a whole EHR (single).

| Schedule case (id TBD) | Derived condition | Data sets | ECC map — verdict |
|---|---|---|---|
| `export_ehr-<TBD>` (whole EHR) | export one EHR → EHR_EXTRACT carrying its versioned objects | pre: 1 EHR w/ content | `ECC-MSG-001` `msg/export-ehrs` (`message.rs:39`, cites `service_extract.rs::export_ehrs_carries_every_versioned_object_latest_only`) — **conformant-disposition**: SKIPPED(NativeApiOnly), evidence cited. Threads `schedule_ref` (`message.rs:46`). |
| `export_ehr-<TBD>` (unknown EHR) | export a non-existent EHR → `ehr_id_does_not_exist` | pre: empty | `ECC-MSG-003` `msg/export-unknown-ehr` (`message.rs:55`, cites `export_ehrs_unknown_ehr_is_ehr_id_does_not_exist`) — **conformant-disposition** (SKIPPED). |

### `I_EHR_EXTRACT.export_ehr_extract()` — EHR Extract · OPTIONS

Schedule stub: **`TBD`**. Derived: export a spec-driven extract (manifest +
version spec) for one EHR.

| Schedule case (id TBD) | Derived condition | Data sets | ECC map — verdict |
|---|---|---|---|
| `export_ehr_extract-<TBD>` | manifest/version-spec-driven extract | pre: 1 EHR | mapped by `ECC-MSG-002` `msg/export-ehr-extracts` (see below — the plural op; the singular is subsumed). Distinct singular case **missing** but subsumed by the plural spec-driven case. |

### `I_EHR_EXTRACT.export_ehrs()` — EHR Extract · OPTIONS

Schedule stub: **`TBD`**. Derived: export multiple EHRs. Mapped by
`ECC-MSG-001` `msg/export-ehrs` (whole-EHR export; the plural is the same native
op). **conformant-disposition**.

### `I_EHR_EXTRACT.export_ehr_extracts()` — EHR Extract · OPTIONS

Schedule stub: **`TBD`**. Derived: spec-driven multi-EHR extract
(`EXTRACT_ENTITY_MANIFEST` + `EXTRACT_VERSION_SPEC`).

| Schedule case (id TBD) | Derived condition | Data sets | ECC map — verdict |
|---|---|---|---|
| `export_ehr_extracts-<TBD>` | item-list + all-versions honoured | pre: multi-version EHR | `ECC-MSG-002` `msg/export-ehr-extracts` (`message.rs:47`, cites `export_ehr_extracts_honours_item_list_and_all_versions`) — **conformant-disposition** (SKIPPED). |

### Import family — under `I_EHR_EXTRACT` (SM `I_EHR_EXTRACT_SERVICE.import_*`)

The chapter's §Test Cases lists only export subsections + TDD; the **import**
operations (`import_ehr`, `import_ehr_extract`) are not given their own master13
subsection, but the RM Common master06 §Copying (IMPORTED_VERSION Cases 1/2/3)
governs them and the ECC covers them under the EHR Extract capability. These
rows are **ECC-original (schedule silent on import subsections)**.

| Schedule case (derived) | Derived condition (RM master06 §Copying) | ECC map — verdict |
|---|---|---|
| `import_ehr` clone (Case 1) | import whole-EHR clone **reusing** source id | `ECC-MSG-004` `msg/import-ehr-clone` (`message.rs:64`, cites `import_ehr_clone_into_fresh_target_reuses_source_id`) — **conformant-disposition** (SKIPPED). |
| `import_ehr` fixed id | import whole EHR into a caller-fixed id | `ECC-MSG-005` `msg/import-ehr-fixed-id` (`message.rs:72`, cites `import_ehr_into_fixed_fresh_id`) — **conformant-disposition**. |
| `import_ehr` duplicate (negative) | import into a duplicate target id → fail | `ECC-MSG-006` `msg/import-ehr-duplicate` (`message.rs:80`, cites `import_ehr_duplicate_target_is_rejected`) — **conformant-disposition**. |
| `import_ehr_extract` (Case 2) | import extract into existing EHR; re-import is a conflict | `ECC-MSG-007` `msg/import-ehr-extract` (`message.rs:87`, cites `import_ehr_extract_adds_a_versioned_object_and_rejects_re_import`) — **conformant-disposition**. |

### `I_TDD.import_tdd()` — TDS · OPTIONS

Schedule stub: `aaaa`/`bbbb` **`TBD`** (master13 §Test Cases). Derived: import a
TDD as a committed COMPOSITION over OPT/WebTemplate.

| Schedule case (id TBD) | Derived condition | ECC map — verdict |
|---|---|---|
| `import_tdd-<TBD>` (commit) | valid TDD → COMPOSITION committed | `ECC-MSG-008` `msg/tdd-import-commits` (`message.rs:96`, cites `service_tdd.rs::tdd_import_commits_composition`) — **conformant-disposition** (SKIPPED). |
| `import_tdd-<TBD>` (reject) | malformed / non-TDD / unknown EHR / unknown template → typed reject | `ECC-MSG-009` `msg/tdd-import-rejects` (`message.rs:104`, cites the 4 `tdd_import_rejects_*` tests) — **conformant-disposition**. |

### `I_TDD.import_tdds()` — TDS · OPTIONS

Schedule stub: **`TBD`**. Derived: batch import — commit all, fail-fast on error.

| Schedule case (id TBD) | Derived condition | ECC map — verdict |
|---|---|---|
| `import_tdds-<TBD>` (batch) | batch commits all; fail-fast | `ECC-MSG-010` `msg/tdd-import-tdds-batch` (`message.rs:112`, cites `tdd_import_tdds_batch_commits_all` + `_fail_fast`) — **conformant-disposition** (SKIPPED). |

**Schedule coverage:** master13 lists **7 subsection headings for 6 distinct SM
operations** (`export_ehr` duplicated) + the RM-governed import operations, all
`TBD`. All are represented by the 10 MSG cases as `SKIPPED(NativeApiOnly)` with
cited native evidence. **0 export/TDD operations unmapped**; the singular
`export_ehr_extract` is subsumed by the plural case (G-3). All rows are
ECC-original (schedule stub).

---

## 3. Existing ECC cases with no schedule home

None outside the schedule's operation set. The four **import** cases
(`ECC-MSG-004..007`) have no dedicated master13 *subsection* (the chapter lists
export + TDD only), but they are governed by RM Common master06 §Copying and sit
under the EHR Extract capability — flagged **ECC-original (schedule silent on
import; RM-backed)** in §2, not homeless.

---

## 4. G-rows — gaps + rulings for the rewrite

- **G-1 (native-API-only is the correct disposition — keep it, keep it honest).**
  Messaging has no ITS-REST binding; the ECC drives HTTP only. The rewrite keeps
  every MSG case `SKIPPED(NativeApiOnly)` and must keep the cited `app/ehrbase`
  integration-test names in lockstep with `app/ehrbase/tests/` — a stale citation
  silently breaks the off-wire traceability that is the entire evidentiary basis.
  A CI check that every cited test symbol still exists would harden this.

- **G-2 (schedule/SM naming skew — record it).** master13 §Test Cases names the
  interfaces `I_EHR_EXTRACT` / `I_TDD`; our cases cite the SM traits
  `I_EHR_EXTRACT_SERVICE` / `I_TDD_SERVICE` (`message.rs:42` etc.). The rewrite's
  `schedule_ref` must reproduce the chapter's literal name
  (`I_EHR_EXTRACT.export_ehrs (CNF master13, TBD)` — already done,
  `message.rs:46`) while the `citation` field keeps the SM-trait name; both are
  correct at their own layer and the divergence is a schedule authoring quirk.

- **G-3 (schedule duplicate + subsumed singular).** `export_ehr()` appears twice
  in master13 (lines 51, 77) and `export_ehr_extract()` (singular) has no distinct
  ECC case (subsumed by the plural `export_ehr_extracts`). The rewrite records the
  duplicate as a schedule defect (not two tests) and either adds a distinct
  singular-extract case or documents the subsumption explicitly, so coverage
  accounting is exact rather than approximate.

- **G-4 (evidence-only coverage does not count toward a wire verdict).** Because
  no MSG case touches the wire, the Messaging capability is **OPTIONS** and its
  evidence is off-transport. The rewrite must ensure the profile math treats every
  MSG SKIPPED as "not obtained over the wire" (never a silent OBTAINED), while the
  report surfaces the cited native evidence so a reader sees the capability *is*
  implemented — just not ECC-wire-tested. For a foreign SUT with an actual MESSAGE
  REST API, these become live cases (the ITS-REST MESSAGE API is OPTIONS/
  DEVELOPMENT) — the rewrite should leave that door open rather than hardcoding the
  skip.

- **G-5 (no data sets — derive from register 80).** master13 §Test Data Sets is
  `TBD`; the native tests carry their own fixtures. If any MSG case ever gains a
  wire binding, its export/import/TDD fixtures must come from register 80, not
  inline literals.

---

*Register 80 owns any future MSG wire fixtures (G-5); register 90 owns the
native-API-only skip pattern shared with register 09 and the profile-math ruling
referenced by G-4.*
