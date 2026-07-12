# ITS-REST Admin API — spec-conformance audit

Read-only audit of the **ITS-REST Admin API** (development edition,
`DEVELOPMENT` maturity — newly vendored) against the implementation. Structure
mirrors `docs/design/sm-platform/10-subject-proxy.md`: spec oracle → verified
current state (file:line) → gap register (G-n rows with exact citations) →
target design → PORT-NOTE residue.

**Verdict up front:** the vendored dev-edition Admin API is **far narrower than
the historic EHRbase admin surface** — it defines *exactly two* operations, both
physical EHR delete, and **both are already wired** in `ehrbase-rest`. So wire
*coverage* of the vendored spec is complete. The honest residue is (a) the
`admin_ehr_delete_all` **response shape and empty-list semantics diverge** from
the now-vendored operation (we return `200 {"deleted": n}` and refuse an
empty list with `400`; the spec mandates `204`/`202` and treats an absent
`ehr_id` as "delete all"), and (b) **stale citations** in code and in
`docs/design/sm-platform/15-admin.md` that still say `admin_ehr_delete_all` is
"not in any OAS" — the newly-vendored admin OAS now defines it.

The premise that "the new spec binds more of the admin surface" holds only for
`admin_ehr_delete_all`: the dev-edition admin API binds **no**
composition/directory/contribution physical-delete and **no** template-delete
operation — those simply do not exist in it. The eight other SM admin
operations (`physical_party_delete`, the four statistics calls,
`archive_ehrs`/`archive_parties`, `export_ehrs`/`load_ehrs`) remain
correctly native-API-only; there is no ITS-REST binding to give them.

---

## Spec oracle (read before any change)

The vendored dev-edition Admin API is small enough to enumerate completely:

- `docs/specs/openehr/ITS-REST/specifications/admin.openapi.yaml` — the API
  root. `info.version: development`, `x-status: DEVELOPMENT`, `security: []`
  (auth is out of band — SM master02 §Overview). It mounts exactly two paths
  (`admin.openapi.yaml:24-30`):
  - `DELETE /admin/ehr/{ehr_id}` → `./operations/admin_ehr_delete.yaml`
  - `DELETE /admin/ehr/all{?ehr_id*}` → `./operations/admin_ehr_delete_all.yaml`
- `docs/specs/openehr/ITS-REST/specifications/operations/admin_ehr_delete.yaml`
  — `admin_ehr_delete`. "Deletes the EHR identified by `ehr_id`. All resources
  associated with or owned by the specified EHR (COMPOSITION, EHR_STATUS,
  ITEM_TAG, CONTRIBUTION, and their historical versions) will also be
  **permanently and physically deleted** … (e.g., the GDPR)" (:3-6). Responses:
  **`202`** (async accepted), **`204`** (sync hard-delete complete), **`404`**
  (`404_unknown_ehr_id` — "an EHR with `ehr_id` does not exist") (:15-21). Path
  param `ehr_id` is a required `uuid` (`parameters/path/ehr_id.yaml`).
- `docs/specs/openehr/ITS-REST/specifications/operations/admin_ehr_delete_all.yaml`
  — `admin_ehr_delete_all`. "Deletes **all or multiple** EHRs, or a specified
  subset … identified using the `ehr_id` query parameter" (:5). "Intended
  primarily for **development or testing** … may be disabled in **production**,
  in which case server may respond with **`405`** Method Not Allowed" (:7).
  Same GDPR physical-cascade wording (:9); async → `202`, sync → `204` (:11-12).
  Responses: `202`, `204`, `404`, `405` (:18-26).
- `docs/specs/openehr/ITS-REST/specifications/parameters/query/ehr_id_Admin.yaml`
  — the `ehr_id` query param: **`in: query`, `style: form`, `explode: true`,
  `type: string`/`format: uuid`, and crucially "An **optional** parameter to
  perform the operation on a **subset** of EHRs" (:1-9). Examples show a single
  value and a repeated/array form (:10-18). An absent `ehr_id` therefore means
  the full set ("all EHRs").
- Response bodies: `responses/204_deleted_hard.yaml` ("physically deleted, i.e.
  hard-delete"), `responses/202.yaml`, `responses/404_unknown_ehr_id.yaml`,
  `responses/404.yaml`, `responses/405.yaml` — **none define a response body**
  (all are description-only; `204` carries no content).
- Computable mirror (byte-identical operation set, verified):
  `docs/specs/openehr/ITS-REST/computable/OAS/admin-codegen.openapi.yaml`
  (`/admin/ehr/{ehr_id}:58`, `/admin/ehr/all{?ehr_id*}:80`); the codegen input
  actually consumed is `crates/openehr-its/vendor/rest-oas/admin-codegen.openapi.yaml`
  (same two ops; ITS-REST OAS pinned `e8a093e…` master,
  `crates/openehr-its/vendor/PROVENANCE.md:14`).
- Prior art (behaviour, not wire): CNF Robot
  `docs/specs/openehr/CNF/tests/platform/robot/I_ADMIN_SERVICE/001-EHR.robot`
  (physical EHR delete → `204`, full backing-table cascade). The CNF admin
  schedule `CNF/docs/platform_test_schedule/master12-func_tc_admin.adoc` is TBD.
- Adjacent SM authority: `SM/docs/UML/classes/i_admin_service.adoc`
  (`physical_ehr_delete`, precondition `Pre_has_ehr`, error
  `ehr_id_does_not_exist`); `SM/docs/openehr_platform/master02-overview.adoc:40`
  (the ADMIN component role). Note: `i_admin_service.adoc` defines only the
  **single**-EHR `physical_ehr_delete`; the bulk `admin_ehr_delete_all` exists
  in the ITS-REST OAS but **not** in the abstract SM interface.

---

## Verified current state (file:line evidence)

**Generated ITS-REST contract** —
`crates/openehr-its/src/rest/generated/admin.rs`: `AdminEhrDeleteParams`
(`ehr_id: String`, :17-21), `AdminEhrDeleteAllParams` (`ehr_id: Option<String>`,
:24-27), the `AdminApi` server trait with `admin_ehr_delete` / `admin_ehr_delete_all`
(both default → `NotImplemented`), and the route table
`ROUTES = [("DELETE","/admin/ehr/{ehr_id}",…), ("DELETE","/admin/ehr/all{?ehr_id*}",…)]`
(:52-54). The generated surface is **exactly** the two vendored operations —
nothing to add or drop.

**Wire dispatch** — `app/ehrbase-rest/src/dispatch/admin.rs`:
- Config gate: `admin.enabled` default off → every admin route answers `404`
  (`:52-56`).
- `admin_ehr_delete` → `AdminService::admin_ehr_delete(ehr_id)` → **`204`
  No Content**; unknown EHR → `404` via the SM `ehr_id_does_not_exist` → NotFound
  mapping (`:62-68`).
- `admin_ehr_delete_all` → reads the `ehr_id` list from the raw query
  (`ehr_id_list`, accepts both `?ehr_id=a&ehr_id=b` and `?ehr_id=a,b`, `:108-120`);
  **empty list → `400`** (`:78-85`); otherwise `admin_ehr_delete_all(ids)` →
  **`200` with body `{"deleted": <n>}`** (`:86-95`).

**SM native seam** — `app/ehrbase-sm/src/services/admin/service.rs`:
`AdminService` trait (`:38`) with `admin_ehr_delete` (:41, "`204`; unknown EHR
→ 404"), `admin_ehr_delete_all` (:56), and the eight native-only operations
`admin_list_contributions` (:72), `admin_contribution_count` (:86),
`versioned_composition_count` (:104), `composition_version_count` (:118),
`physical_party_delete` (:136), plus `AdminArchive::archive_ehrs`/`archive_parties`
and `AdminDumpLoad::export_ehrs`/`load_ehrs`. Service impl + cascade machinery
live in `app/ehrbase/src/service/admin.rs` and `dump_load.rs` (audited in
`docs/design/sm-platform/15-admin.md`).

**ECC coverage** — `tools/conformance/src/suites/admin.rs`: single-delete →
`204` (`:98-111`), unknown → `404` (`:113-118`), delete-then-redelete →
`404` (`:145`), `run_delete_all` asserts **`200 {"deleted": 2}`** (`:150-174`),
`run_delete_all_partial` → **`200 {"deleted": 1}`** (`:176-200`),
`run_delete_all_empty` → **`400`** (`:202-212`). Auth is exercised by
`tools/conformance/src/suites/security.rs:112-120` (non-admin credential on
`DELETE /admin/ehr/{id}` must be rejected).

### Faithful realizations (not gaps)

- **100 % wire coverage of the vendored operation set** — both admin operations
  are mounted and functional; the generated route table needs no change.
- **`admin_ehr_delete` is exactly spec-conformant**: `DELETE /admin/ehr/{uuid}`
  → `204` on success, `404` on unknown id, full physical cascade
  (`service/admin.rs` `physical_ehr_delete`, matching the CNF Robot expectation).
  We do not emit the optional async `202` — the spec offers it as a "may", so a
  purely synchronous `204` is conformant.
- **Auth beyond the spec is legitimate**: the OAS sets `security: []`, deferring
  auth out of band (SM master02); requiring the admin role at our wire is an
  in-band realization of that out-of-band contract, not a divergence.

---

## Gap register

Every gap cites the governing spec text. None affects `admin_ehr_delete`; the
register is the `admin_ehr_delete_all` wire contract and citation hygiene now
that the operation is vendored.

| # | Gap | Spec citation | Today |
|---|-----|---------------|-------|
| G-1 | **`admin_ehr_delete_all` returns `200` with a JSON body; the spec mandates `204`/`202` and no body.** The vendored operation declares only `202`/`204`/`404`/`405` responses, all description-only (`204_deleted_hard.yaml` is No Content). Our wire returns `200 {"deleted": n}`. This is a real wire divergence from a now-vendored operation (before this vendoring it was PORT-NOTEd as "unspecified, so shape is our choice"; the operation is no longer unspecified). | `operations/admin_ehr_delete_all.yaml:18-26`; `responses/204_deleted_hard.yaml`; `responses/202.yaml` | `dispatch/admin.rs:86-95` returns `200 {"deleted": n}`; ECC asserts it (`admin.rs:150-200`). |
| G-2 | **Empty/absent `ehr_id` is refused with `400`; the spec makes `ehr_id` optional and an absent list means "delete all EHRs".** "Deletes **all** or multiple EHRs, or a specified subset" + `ehr_id` is "An **optional** parameter to perform the operation on a **subset**". We deliberately refuse an implicit delete-everything. Defensible as a safety posture, but it now contradicts an explicit spec sentence and must be recorded as a deliberate deviation (or gated so the "all" form is reachable in a dev/test profile). | `operations/admin_ehr_delete_all.yaml:5`; `parameters/query/ehr_id_Admin.yaml:1-9` | `dispatch/admin.rs:78-85` → `400`; ECC `run_delete_all_empty` asserts `400` (`admin.rs:202-212`). |
| G-3 | **`405 Method Not Allowed` for a production-disabled bulk delete is never emitted.** The spec's `405` is the specific "delete-all disabled in production" signal (distinct from the whole admin API being off). Our only gate is `admin.enabled` → `404` for the entire group; there is no per-operation `405` for delete-all-in-production. | `operations/admin_ehr_delete_all.yaml:7,25`; `responses/405.yaml` | Group-level `404` when `admin.enabled=false` (`dispatch/admin.rs:52-56`); no `405` path. |
| G-4 | **Async `202` never emitted (informational).** The spec permits `202` when the delete is processed asynchronously/in batches. We are always synchronous → `204`/`200`. Conformant (`202` is a "may"), but worth noting for large-EHR/batch behaviour. | `operations/admin_ehr_delete.yaml:8-9`; `admin_ehr_delete_all.yaml:11-12`; `responses/202.yaml` | Always synchronous; no `202` path. |
| G-5 | **Stale "not in any OAS" citations — citation-hygiene violation.** Now that the dev-edition admin OAS vendors `admin_ehr_delete_all`, three code comments are false: `dispatch/admin.rs:79-81` ("delete-all is unspecified (not in the SM, not in any OAS)"), `dispatch/admin.rs:9-11` ("The ADMIN API is dev-branch only in ITS-REST (no vendored OAS)"), and `ehrbase-sm/.../admin/service.rs:50-51` ("this operation has no spec at all … not in any OAS"). `docs/design/sm-platform/15-admin.md` G-2 makes the same now-outdated claim. | `.claude/rules/spec-adherence.md` (keep citations findable/true); `admin.openapi.yaml:28-30`; `crates/openehr-its/vendor/rest-oas/admin-codegen.openapi.yaml:80` | Four dead/false references across three files + the 15-admin doc. The operation **is** in the SM-adjacent ITS-REST OAS (though still not in the abstract `i_admin_service.adoc`). |
| G-6 | **No wire exists — nor is one defined by this spec — for the other admin surfaces the task set implied.** The dev-edition admin API defines **no** composition/directory/contribution physical-delete and **no** template/OPT delete. WORKLIST W-2(a)'s `list_contributions ×5` cannot gain an admin-API binding (undefined here → native-only / N/A), and `delete_opt ×4` belongs to the **Definition** API, not admin. The eight SM admin ops (party delete, statistics, archive, dump/load) stay native-API-only — spec-consistent, not a defect. | `admin.openapi.yaml:24-30` (only two paths); `SM/docs/UML/classes/i_admin_service.adoc`; `docs/plans/WORKLIST.md` W-2 | Native seam complete; no additional admin routes exist because the spec defines none. Recorded in `15-admin.md` G-1. |

---

## Target design (to close the register)

The generated contract and `admin_ehr_delete` need no change. The work is the
`admin_ehr_delete_all` contract alignment and a citation scrub.

### 1. Citation scrub (G-5) — do first, cheap, hygiene-blocking

- `dispatch/admin.rs:9-11`: drop "dev-branch only … no vendored OAS" — the OAS
  **is** vendored (`crates/openehr-its/vendor/rest-oas/admin-*.openapi.yaml`,
  provenance `e8a093e`). Cite the operation YAMLs instead.
- `dispatch/admin.rs:79-81` and `ehrbase-sm/.../admin/service.rs:50-51`: the
  delete-all "unspecified / not in any OAS" claim is false; restate as
  "the ITS-REST OAS defines `admin_ehr_delete_all` (`operations/admin_ehr_delete_all.yaml`)
  but the abstract SM `i_admin_service.adoc` does not — the *response body* and
  *empty-list refusal* are our deliberate deviations (G-1/G-2)".
- `docs/design/sm-platform/15-admin.md` G-2: update from "no spec at all" to
  "OAS-defined; response-shape/empty-list divergence" and cross-link this doc.

### 2. Align the `admin_ehr_delete_all` wire contract (G-1/G-2/G-3)

Decide and record one of two postures, and make the ECC cases match it (the ECC
cases at `admin.rs:150-212` currently encode the divergent `200`/`400`
behaviour and would move with the decision — never edit a case to route around
a defect; move it because the spec contract changed):

- **Spec-strict**: return **`204` No Content** (no body) for a successful bulk
  delete; treat an absent `ehr_id` as "delete all" behind a dev/test profile
  flag, and emit **`405`** when that profile is disabled in production (the
  spec's exact "disabled in production" signal). Drop the `{"deleted": n}` body
  or move the count to a header/`202` batch report.
- **Deviation-with-note**: keep `200 {"deleted": n}` and the empty-list `400`
  as a deliberate, safer-by-default realization, but carry a `// PORT NOTE:`
  citing `admin_ehr_delete_all.yaml` that (a) the spec response is `204`/`202`
  and (b) the spec's absent-`ehr_id`-means-all semantics is intentionally not
  honoured for safety. This keeps the divergence honest and findable.

Either way, the async `202` (G-4) stays an optional future for batched deletes;
record it as a PERF/scale item, not a conformance gap.

### 3. Confirm the rest of the admin surface stays native-only (G-6)

No action beyond documentation: the vendored admin API defines no other route,
so `physical_party_delete`, statistics, archive, and dump/load remain SM-native
(their exposure question is tracked in `docs/design/sm-platform/15-admin.md`
G-1 and WORKLIST W-3d, as an **extension** surface, not an ITS-REST-defined one).
For W-2 skip elimination: `list_contributions` has no admin-API binding to
expose → N/A with a citation to `admin.openapi.yaml` (only two paths);
`delete_opt` is a Definition-API concern, out of this audit's scope.

---

## Standing PORT-NOTE residue (the honest set after closure)

- `admin_ehr_delete_all` is defined in the ITS-REST OAS but **not** in the
  abstract SM `i_admin_service.adoc` — a spec-internal inconsistency to record,
  not resolve.
- If the deviation-with-note posture is chosen: the `200 {"deleted": n}` body
  and the empty-list `400` refusal are deliberate deviations from the vendored
  `204`/`202` + absent-means-all contract (G-1/G-2), safer-by-default.
- Async `202` is not emitted; all admin deletes are synchronous (G-4).
- The whole admin group is behind `admin.enabled` and answers `404` when off
  (prior art: EHRbase `ADMINAPI_ACTIVE`); the spec's per-operation `405` for a
  production-disabled bulk delete is not modelled (G-3).
- The ADMIN API's out-of-band `security: []` is realized in-band as an
  admin-role requirement — an addition permitted by SM master02, not a
  divergence.
- All non-delete admin capabilities (party delete, statistics, archive,
  dump/load) are native-API-only because the dev-edition ITS-REST Admin API
  defines no wire for them (G-6) — tracked as an extension surface in
  `docs/design/sm-platform/15-admin.md`.
