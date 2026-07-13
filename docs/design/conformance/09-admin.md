# Conformance register 09 — Admin component (`suites/admin.rs`): spec-first audit

W-10 area audit (read-only, 2026-07-13) of the **Admin** component of
`tools/conformance`. Method is spec-first (README + owner ruling): the spine
below is the governing CNF schedule chapter enumerated operation-by-operation;
the existing ECC cases are mapped **onto** each schedule item with a `file:line`
verdict (conformant / divergent / missing / instrument-encodes-server-behaviour).
§3 lists ECC cases with no schedule home; §4 carries the G-rows for the rewrite.

**The governing chapter is a stub.** `master12-func_tc_admin.adoc` ships **no
concrete test cases** — every one of its 9 SM-operation subsections carries only
placeholder `==== Test Case aaaa` / `bbbb` bodies reading `TBD`, and
`== Dependencies` + `== Test Environment` + `== Test Data Sets` are `TBD` too
(21 `TBD` markers total; blueprint `07-cnf.md` §master12). So the spine records
each schedule stub **verbatim (cited)** and derives the honest spine from the
chapter's operation headings (§Test Cases subsection titles) crossed with the
profiles *Admin* capability rows (`master03-profiles.adoc` §Functional) and the
existing ECC case universe. Every spine row backed only by a TBD heading is
flagged **ECC-original (schedule stub)**.

**Spec oracles** (read before any change):

- `docs/specs/openehr/CNF/docs/platform_test_schedule/master12-func_tc_admin.adoc`
  — the ADMIN_SERVICE suite; read whole. §Normative Reference names the abstract
  interfaces `I_ADMIN_SERVICE`, `I_ADMIN_DUMP_LOAD`, `I_ADMIN_ARCHIVE`; §Test
  Cases enumerates 9 SM operations (all bodies `TBD`).
- `docs/specs/openehr/CNF/docs/platform_test_schedule/master03-overview.adoc`
  — the test-case form `<SERVICE_COMPONENT>.<operation>-<test-specific id>`
  (§API Conformance Test Design).
- `docs/specs/openehr/CNF/docs/profiles/master03-profiles.adoc` §Functional —
  *Admin* (Activity Report, Physical Deletion, EHR Dump/Load, Bulk EHR load, EHR
  Archive, Demographic Archive) = **all OPTIONS**; §REST APIs — ADMIN API =
  **OPTIONS**.

**Mapped suite:** `tools/conformance/src/suites/admin.rs` (6 ECC entries,
`ECC-ADM-001..006`) and the shared `suites/support.rs` helpers.

---

## 1. Verdict

The Admin suite is **narrow by construction and honest about it**: master12 has
no concrete cases, and the *implemented* ITS-REST admin wire is exactly two
routes — `DELETE /admin/ehr/{ehr_id}` and `DELETE /admin/ehr/all{?ehr_id*}`
(`admin.rs` module docs) — realizing one SM operation, `physical_ehr_delete`.
The six ECC cases cover that operation thoroughly (single delete 204, absent 404,
idempotent re-delete 404, bulk-all 204, partial-with-missing 204, empty-selector
deletes-all 204), each grounded in the vendored ADMIN OAS operation files.

The gap is that the schedule names **9 SM operations** and the suite touches
**one**. The other eight — `list_contributions`, `contribution_count`,
`versioned_composition_count`, `composition_version_count`,
`physical_party_delete`, `export_ehrs` (`I_ADMIN_DUMP_LOAD`), `archive_ehrs`,
`archive_parties` (`I_ADMIN_ARCHIVE`) — are **missing from the ECC**. Several are
implemented natively (SM-4 wave 3: dump/load, archive, activity-report counts —
blueprint `07-cnf.md` §master12) but have no ITS-REST route the HTTP-only ECC can
reach, so they are the **Messaging precedent**: candidate native-API-only
skip-with-reason cases, not silent omissions. Every ADM entry carries
`schedule_ref: None` (`admin.rs:88`). And the whole surface is OPTIONS: for a
foreign SUT the fairness register decides per-case (admin routes are
implementation-shaped extensions), and its absence never dents CORE/STANDARD.

---

## 2. The spine (master12 operations → ECC map)

Schedule ids use the overview form `I_ADMIN_SERVICE.<operation>-<id>` (or
`I_ADMIN_DUMP_LOAD.`/`I_ADMIN_ARCHIVE.`). The concrete `<id>` is **TBD** in every
subsection, recorded verbatim. Data-set classes: master12 §Test Data Sets is
`TBD` → **derived**. Capability / profile from `master03-profiles.adoc`
§Functional — *Admin* (all OPTIONS). ECC file:line is in `suites/admin.rs`.

### `I_ADMIN_SERVICE.list_contributions()` — Activity Report · OPTIONS

Schedule stub: `aaaa`/`bbbb` **`TBD`** (master12 §Test Cases). Derived intent:
enumerate an EHR's CONTRIBUTIONs.

| Schedule case (id TBD) | Derived condition | Data sets | ECC map — verdict |
|---|---|---|---|
| `list_contributions-<TBD>` | list an EHR's contributions | pre: EHR + contributions | **missing** — no ECC case; the SM op is native (SM-4) but has no ITS-REST admin route reachable over the wire. G-2 (native-API-only candidate). |

### `I_ADMIN_SERVICE.contribution_count()` — Activity Report · OPTIONS

Schedule stub: **`TBD`**. Derived: count CONTRIBUTIONs for an EHR. → **missing**
(G-2).

### `I_ADMIN_SERVICE.versioned_composition_count()` — Activity Report · OPTIONS

Schedule stub: **`TBD`**. Derived: count versioned compositions for an EHR. →
**missing** (G-2).

### `I_ADMIN_SERVICE.composition_version_count()` — Activity Report · OPTIONS

Schedule stub: **`TBD`**. Derived: count versions of a composition. → **missing**
(G-2).

### `I_ADMIN_SERVICE.physical_ehr_delete()` — Physical Deletion · OPTIONS

Schedule stub: `aaaa`/`bbbb` **`TBD`**. Derived intent (SM §I_ADMIN_SERVICE +
ADMIN OAS `operations/admin_ehr_delete.yaml` / `admin_ehr_delete_all.yaml`):
physically remove an EHR and its full version history; a synchronous cascade.
**This is the one operation the ITS-REST admin wire exposes and the ECC covers.**

| Schedule case (id TBD) | Derived condition | Data sets | ECC map — verdict |
|---|---|---|---|
| `physical_ehr_delete-<TBD>` (single) | existing EHR → 204 physical delete | pre: 1 EHR | `ECC-ADM-001` `adm/ehr-delete` (`admin.rs:101`) — **conformant** (`DELETE /admin/ehr/{id}` with `AuthSlot::Admin` → 204). ECC-original (schedule stub). |
| `physical_ehr_delete-<TBD>` (absent) | unknown EHR → 404 | pre: empty | `ECC-ADM-002` `adm/ehr-delete-absent` (`admin.rs:116`) — **conformant** (404 on random UUID). |
| `physical_ehr_delete-<TBD>` (idempotent) | re-delete of a gone EHR → 404 | pre: 1 EHR then deleted | `ECC-ADM-003` `adm/ehr-delete-idempotent` (`admin.rs:131`) — **conformant** (204 then 404 — physical delete leaves no trace, correctly non-idempotent-observable). |
| `physical_ehr_delete-<TBD>` (bulk) | `DELETE /admin/ehr/all?ehr_id=a&ehr_id=b` → 204 bodyless | pre: 2 EHRs | `ECC-ADM-004` `adm/ehr-delete-all` (`admin.rs:157`) — **conformant** (204; OAS `admin_ehr_delete_all.yaml` → `responses/204_deleted_hard.yaml`, not a `200 {"deleted": n}` body). |
| `physical_ehr_delete-<TBD>` (partial) | bulk with a missing id → still 204 (skipped) | pre: 1 EHR + 1 absent id | `ECC-ADM-005` `adm/ehr-delete-all-partial` (`admin.rs:178`) — **instrument-encodes-server-behaviour**: asserts 204 for a set including a non-existent id, encoding "no per-id failure for the bulk op" — a reading of the OAS, not a normative schedule case (master12 TBD). |
| `physical_ehr_delete-<TBD>` (empty selector) | `DELETE /admin/ehr/all` (no `ehr_id`) → delete all, 204 | pre: any | `ECC-ADM-006` `adm/ehr-delete-all-empty` (`admin.rs:200`) — **instrument-encodes-server-behaviour**: encodes "absent `ehr_id` deletes **all** EHRs" (OAS `parameters/query/ehr_id_Admin.yaml` optional selector) — a destructive default that is a design reading, not schedule-mandated. |

### `I_ADMIN_SERVICE.physical_party_delete()` — Physical Deletion · OPTIONS

Schedule stub: **`TBD`**. Derived: physically remove a demographic PARTY. →
**missing** — no ECC case; depends on the demographic wire (register 08, an
extension). G-3.

### `I_ADMIN_DUMP_LOAD.export_ehrs()` — EHR Dump/Load · OPTIONS

Schedule stub: `aaaa`/`bbbb` **`TBD`** (master12 §Test Cases). Derived: export
EHRs for dump; the load side (`import`/`load_ehrs`) round-trips them.

| Schedule case (id TBD) | Derived condition | Data sets | ECC map — verdict |
|---|---|---|---|
| `export_ehrs-<TBD>` | dump EHRs → segmented export; reload → equality | pre: EHRs present | **missing** — native only (SM-4 wave 3 `export_ehrs`/`load_ehrs`, `DUMP_LOAD_FAIL_REPORT`; blueprint §B3); no ITS-REST route. G-2 (native-API-only candidate; also Bulk EHR load capability). |

### `I_ADMIN_ARCHIVE.archive_ehrs()` — EHR Archive · OPTIONS

Schedule stub: **`TBD`**. Derived: move EHRs to archival storage. → **missing** —
native only, no wire route; the archive storage-movement PERF item is blueprint
tail (§2.1). G-2.

### `I_ADMIN_ARCHIVE.archive_parties()` — Demographic Archive · OPTIONS

Schedule stub: **`TBD`**. Derived: archive demographic parties. → **missing** —
native only + demographic-dependent. G-2/G-3.

**Schedule coverage:** master12 defines **9 SM operations × 2 TBD stubs = 18
placeholder test cases** (no concrete case). Of the 9 operations: **1 mapped**
(`physical_ehr_delete`, 6 ECC cases), **8 missing** (4 Activity-Report counts,
`physical_party_delete`, `export_ehrs`, `archive_ehrs`, `archive_parties`). The
one mapped operation is **ECC-original (schedule stub)** — the ECC is the source
of admin test substance. The 8 missing operations split: 6 are native-API-only
(no ITS-REST binding — the Messaging precedent, G-2) and 2 additionally depend on
the demographic extension (G-3).

---

## 3. Existing ECC cases with no schedule home

None. All 6 ADM cases map to `I_ADMIN_SERVICE.physical_ehr_delete` (via the two
implemented admin routes). No ADM case exercises a surface outside the schedule's
operation set — the suite is a subset of the spine, not a superset.

---

## 4. G-rows — gaps + rulings for the rewrite

- **G-1 (destructive-default + widened-code cases encode a design reading, not
  a schedule case).** `adm/ehr-delete-all-empty` (`admin.rs:200`) makes an absent
  `ehr_id` delete **all** EHRs, and `adm/ehr-delete-all-partial` (`admin.rs:178`)
  swallows a missing id as 204. Both are readings of the ADMIN OAS (which
  master12 does not concretise). The rewrite keeps them but flags each
  **instrument-encodes-server-behaviour**, and — because the empty-selector case
  is globally destructive on a shared SUT — must gate it behind a dedicated
  isolated-SUT precondition (never run it against a bring-your-own endpoint whose
  data must survive).

- **G-2 (8 SM operations missing; 6 are native-API-only — the Messaging
  precedent).** `list_contributions`, `contribution_count`,
  `versioned_composition_count`, `composition_version_count`, `export_ehrs`
  (dump/load), `archive_ehrs`, `archive_parties` are implemented natively (SM-4
  wave 3) but have **no ITS-REST admin route** the HTTP-only ECC can reach. The
  rewrite must add `SKIPPED(NativeApiOnly)` cases for each — exactly as
  `suites/message.rs` does — citing the `app/ehrbase` integration test that
  proves the native operation, so the capability evidence is traceable off the
  wire rather than silently absent. Each carries a `schedule_ref` to its master12
  operation (TBD).

- **G-3 (`physical_party_delete` + `archive_parties` are demographic-dependent).**
  These two act on demographic PARTYs, which live behind the ehrbase-rs
  demographic extension (register 08). The rewrite treats them as native-API-only
  and, for foreign SUTs, fairness-register N/A alongside the DEM area.

- **G-4 (`schedule_ref` not threaded).** Every ADM entry sets
  `schedule_ref: None` (`admin.rs:88`). The rewrite threads the SM-operation ref
  per case (`I_ADMIN_SERVICE.physical_ehr_delete (CNF master12, TBD)`) so the
  report shows derived provenance + schedule-stub status.

- **G-5 (Admin is wholly OPTIONS — fairness N/A for foreign SUTs).
  EXTENSION-SPECIFIC.** `master03-profiles.adoc` §Functional puts every Admin
  capability under OPTIONS and §REST APIs puts the ADMIN API under OPTIONS; the
  admin group is config-gated (`RestConfig::admin.enabled`, `admin.rs` module
  docs). A SUT with no admin API (or a differently-shaped one) must be a
  per-area / per-case fairness-register decision (no seeded rule exists yet, unlike
  DEM/SIG — the rewrite should note admin routes are implementation-shaped and
  triage per SUT). Admin's absence never dents a CORE/STANDARD verdict.

---

*Register 80 owns the EHR/contribution fixtures the missing counts + dump/load
cases would need; register 90 owns the `schedule_ref` threading + the
native-API-only skip pattern (shared with register 10) referenced by G-2/G-4.*
