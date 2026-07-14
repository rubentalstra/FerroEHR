# Platform crate — EHR service (`service/ehr/`): spec-first redesign

W-3f area audit (read-only, 2026-07-12) of the **EHR component** of the
`ehrbase` platform crate. The method is spec-first (owner ruling): the register
below is built **from the spec** — the Architecture Overview EHR-design chapter
crossed with the SM EHR component's six interfaces, operation-by-operation — and
existing code is then mapped **onto** each spec item with a `file:line` verdict.
Code that maps to no spec item is flagged (spec-silent / extension / quarantine
/ delete). §4 is the target decomposition of `app/ehrbase/src/service/ehr/`.

**Spec oracles** (read before any change):

- `docs/specs/openehr/BASE/docs/architecture_overview/master06-design_of_the_ehr.adoc`
  — the EHR design chapter (EHR / EHR_ACCESS / EHR_STATUS / Directory / Folders
  / Compositions / Contributions, system identity, language). Read whole.
- `docs/specs/openehr/SM/docs/openehr_platform/master05-ehr_service.adoc` and the
  nine class files it `include::`s under `docs/specs/openehr/SM/docs/UML/classes/`
  (`i_ehr_service`, `i_ehr`, `i_ehr_status`, `i_ehr_directory`,
  `i_ehr_composition`, `i_ehr_contribution`, `ehr_summary`, `uv_folder`,
  `uv_composition`).
- Adjacent RM: `RM/docs/ehr/master04-ehr_package.adoc` (EHR Creation, EHR Active
  Status, Folders), RM common `master06` (change control / versioning).

**Fixed contract** (do not change): the SM native traits at
`app/ehrbase-sm/src/services/ehr/` (`service.rs` = `EhrService`+`EhrSummary`;
`status.rs`; `directory.rs`; `composition.rs`; `contribution.rs`; `handle.rs`
= the `I_EHR` accessor). The platform crate **implements** these; the redesign
only reshapes the implementation side.

**Prior register absorbed:** `docs/design/sm-platform/05-ehr.md` (W-3c). Its
G-rows are carried below and re-verified against current code — G-1 and G-2 are
now **closed** (see §3).

---

## 1. Verdict

The EHR surface is **functionally complete and faithful**: all 42 SM operations
across the six interfaces are implemented with parameter-name / return-type
parity, and the change-control semantics behind them (implicit CONTRIBUTION
creation, `OBJECT_VERSION_ID` construction, logical delete = lifecycle `523`,
audit-copy rule, optimistic `If-Match`, immutable per-EHR `system_id`,
`is_modifiable` content-write guard) are correctly realized. The redesign is
therefore **structural, not behavioural**: today the EHR logic is spread across
flat sibling files (`ehr.rs`, `composition.rs`, `directory.rs`, `contribution.rs`)
with the SM trait impls split off into `service/api/ehr.rs`, and shared
version-metadata helpers parked in `ehr.rs`. W-3f folds these into a
`service/ehr/` folder that mirrors the SM interface set one-file-per-interface,
each file carrying its domain logic **and** its `impl <Interface>Service`, with
the cross-cutting versioning/storage/validation calls marked as integration
seams. Only three genuine behavioural gaps remain (G-5, G-6 and the G-3/G-4
cardinality decisions); the rest are already-correct or extension-quarantine.

---

## 2. Spec-item register (spec-first)

Each row: spec element + citation → code mapping (`file:line`) → verdict.
Files are under `app/ehrbase/src/service/` unless noted; `api/ehr.rs` = the SM
trait-impl adapter.

### 2.1 Architecture Overview — EHR design (`master06 §The EHR`)

| # | Spec model element | Citation | Code | Verdict |
|---|---|---|---|---|
| A1 | **EHR** root object, globally unique EHR id | master06 §The EHR (bullets) | `ehr.rs:117` `ehr_summary` builds RM `EHR` (`ehr_id` HIER_OBJECT_ID) | conformant |
| A2 | **system_id** recorded in each EHR, "assigns version identifiers"; not directly processable | master06 §System Identity | stored immutably at create `ehr.rs:42-52`, served from storage `ehr.rs:126,238` | conformant |
| A3 | **EHR_ACCESS** (versioned) — EHR-wide access-control object | master06 §Top-level Structures; §The EHR | created under the create-EHR CONTRIBUTION `ehr.rs:73-82`; `default_ehr_access` `ehr.rs:750`; `EHR.ehr_access` ref emitted `ehr.rs:179-186` | conformant |
| A4 | **EHR_STATUS** (versioned) — status + optional subject | master06 §The EHR | full `I_EHR_STATUS` surface (2.4) | conformant |
| A5 | **Directory** (versioned, optional) — Folder hierarchy organising Compositions | master06 §The EHR | `directory.rs`; `EHR.directory` = `folders.item(1)` `ehr.rs:517-546` | conformant |
| A6 | **Folders** (versioned, optional, additional hierarchies) | master06 §The EHR; RM ehr master04 §Folders | reads multi-hierarchy `ehr.rs:553-568`; write single-slot | partial → **G-3** |
| A7 | **Compositions** (versioned) — clinical/admin content | master06 §The EHR | `composition.rs`; full `I_EHR_COMPOSITION` (2.5) | conformant |
| A8 | **Contributions** — change-set audits, ≥1 Version each | master06 §The EHR | `contribution.rs`; full `I_EHR_CONTRIBUTION` (2.6) | conformant |
| A9 | **Language** — EHR default language, per-Composition/Entry | master06 §Language | consumer-driven (validated at composition validation); no EHR-level default store | spec-silent (no store mandated) — note |
| A10 | Entries / Instruction state machine / Time-in-EHR | master06 §Entries … §Time | content-model concerns owned by RM + validation, not the EHR service | out of area |

### 2.2 `I_EHR_SERVICE` (`i_ehr_service.adoc`) — 9 operations

| SM op | Citation | Code | Verdict |
|---|---|---|---|
| `has_ehr` | i_ehr_service §has_ehr | `api/ehr.rs:133` → `ensure_ehr_exists` `composition.rs:332` | conformant |
| `has_ehr_for_subject` | §has_ehr_for_subject | `api/ehr.rs:141` → `ehr_by_subject` `ehr.rs:97` | conformant (cardinality → G-4) |
| `create_ehr` (`Pre_no_subject`) | §create_ehr | `api/ehr.rs:152` → `create_ehr` `ehr.rs:27`; default status `ehr.rs:735` | precondition unenforced → **G-5** |
| `create_ehr_with_id` (`Pre_no_subject`, dup→`ehr_create_fail_duplicate_id`) | §create_ehr_with_id | `api/ehr.rs:159`; dup `ehr.rs:48-52` → 409 | conformant except `Pre_no_subject` → **G-5** |
| `create_ehr_for_subject` | §create_ehr_for_subject | `api/ehr.rs:169`; `status_for_subject` `api/ehr.rs:108` | conformant |
| `create_ehr_for_subject_with_id` | §…_with_id | `api/ehr.rs:183` | conformant |
| `get_ehr : EHR_SUMMARY` | §get_ehr; `ehr_summary.adoc` (6 attrs) | `api/ehr.rs:197` → `summarize_ehr` `ehr.rs:230` (all 6; `composition_count` = distinct `vo_id`) | conformant |
| `get_ehrs_for_subject : List<EHR_SUMMARY>` | §get_ehrs_for_subject | `api/ehr.rs:201` (list of ≤1) | conformant modulo cardinality → **G-4** |
| `i_ehr : I_EHR` | §i_ehr; `i_ehr.adoc` | `IEhr` handle (SM crate `handle.rs`) built by `EhrService::i_ehr` | conformant |

### 2.3 `I_EHR` accessor (`i_ehr.adoc`) — 4 attributes

`ehr_status` / `directory` / `compositions` / `contributions` — realized as the
generic `IEhr` handle in the **SM crate** (`handle.rs`), delegating to the flat
traits. No platform-crate code needed; the handle is the fixed contract.
Verdict: conformant (accessor is SM-crate sugar).

### 2.4 `I_EHR_STATUS` (`i_ehr_status.adoc`) — 10 operations

| SM op | Citation | Code | Verdict |
|---|---|---|---|
| `has_ehr_status_version` | §has_ehr_status_version | `api/ehr.rs:241` (one EHR_STATUS per EHR) | conformant |
| `get_ehr_status` | §get_ehr_status | `api/ehr.rs:254` → `status_at(None)` `ehr.rs:272` | conformant |
| `get_ehr_status_at_time` | §get_ehr_status_at_time | `api/ehr.rs:258`; `an_ehr_id` restored (spec-defect) | conformant + PORT NOTE |
| `set_ehr_queryable` (`Post_is_queryable_set`) | §set_ehr_queryable | `api/ehr.rs:285` → `status_mutate` `ehr.rs:369` | conformant (**G-1 closed**) |
| `clear_ehr_queryable` | §clear_ehr_queryable | `api/ehr.rs:296` | conformant |
| `set_ehr_modifiable` | §set_ehr_modifiable | `api/ehr.rs:307` | conformant |
| `clear_ehr_modifiable` | §clear_ehr_modifiable | `api/ehr.rs:318` (EHR_STATUS always writable) | conformant |
| `update_other_details (ITEM_TREE)` | §update_other_details | `api/ehr.rs:331` | conformant |
| `get_ehr_status_at_version` (bare EHR_STATUS) | §get_ehr_status_at_version | `api/ehr.rs:267` → `status_by_version` `ehr.rs:292` (F-01-03) | conformant |
| `get_versioned_ehr_status : VERSIONED_EHR_STATUS` | §get_versioned_ehr_status | `api/ehr.rs:281` → `versioned_status` `ehr.rs:391` | conformant |

Wire-adapter extras (not SM ops, kept as adapter seams): `replace_ehr_status`,
`ehr_status_revision_history`, `ehr_status_version_at_time`,
`ehr_status_original_version` — PORT-NOTEd in `status.rs` (SM crate).

### 2.5 `I_EHR_DIRECTORY` (`i_ehr_directory.adoc`) — 10 operations

| SM op | Citation | Code | Verdict |
|---|---|---|---|
| `has_directory` | §has_directory | `api/ehr.rs:529` → `directory_vo_opt` `ehr.rs:528` | conformant |
| `has_path` (slash-separated Folder names) | §has_path | `api/ehr.rs:537`; `select_subfolder` `directory.rs:300` | conformant |
| `create_directory` (VERSIONED_OBJECT+ORIGINAL_VERSION+CONTRIBUTION; `content_valid`) | §create_directory | `directory.rs:15`; `validate_folder` `directory.rs:239` | conformant (single-slot → G-3) |
| `get_directory` (= at_time None) | §get_directory | `directory.rs:55` `directory_at_time` | conformant |
| `get_directory_at_time` | §get_directory_at_time | `directory.rs:55` | conformant |
| `update_directory` (`has_directory`, optimistic lock) | §update_directory | `directory.rs:137`; `If-Match` `api/ehr.rs:571` | conformant |
| `delete_directory` (logical) | §delete_directory | `directory.rs:170` | conformant |
| `has_directory_version` | §has_directory_version | `directory.rs:120`; `api/ehr.rs:611` | conformant (**G-2 closed**) |
| `get_directory_at_version : FOLDER` | §get_directory_at_version | `directory.rs:89`; `api/ehr.rs:599` | conformant |
| `get_versioned_directory : VERSIONED_FOLDER` | §get_versioned_directory | `versioned_directory` `directory.rs:111`; `api/ehr.rs:624` | conformant (**G-2 closed**) |

### 2.6 `I_EHR_COMPOSITION` (`i_ehr_composition.adoc`) — 8 operations

| SM op | Citation | Code | Verdict |
|---|---|---|---|
| `has_composition` | §has_composition | `api/ehr.rs:391` | conformant |
| `get_composition_latest` (deleted → null/204) | §get_composition_latest | `api/ehr.rs:404` → `read_composition` `composition.rs:62` | conformant |
| `get_composition_at_time` | §get_composition_at_time | `api/ehr.rs:415`; `composition_at_time` `composition.rs:83` | conformant |
| `get_composition_at_version` | §get_composition_at_version | `api/ehr.rs:433` | conformant |
| `get_versioned_composition : VERSIONED_COMPOSITION` | §get_versioned_composition | `api/ehr.rs:445` → `versioned_composition` `composition.rs:100` | conformant |
| `create_composition` (`definitions_valid`,`valid_content`) | §create_composition | `composition.rs:16`; validation `composition.rs:423`; persistent-dup `composition.rs:356` | conformant |
| `update_composition` (optimistic lock; template check) | §update_composition | `composition.rs:159`; template-mismatch → 422 `composition.rs:184` | conformant |
| `delete_composition` (logical, `523\|deleted\|`; SM types `UUID`) | §delete_composition | `composition.rs:280`; uses `ObjectVersionId` | conformant, stronger typing → **G-7** |

### 2.7 `I_EHR_CONTRIBUTION` (`i_ehr_contribution.adoc`) — 5 operations

| SM op | Citation | Code | Verdict |
|---|---|---|---|
| `has_contribution` (`Pre_has_ehr`) | §has_contribution | `api/ehr.rs:633` | conformant |
| `get_contribution : CONTRIBUTION` | §get_contribution | `api/ehr.rs:641` (+ `resolve_refs` variant) | conformant |
| `commit_contribution` (atomic multi-UPDATE_VERSION; `Pre_has_ehr`) | §commit_contribution | `api/ehr.rs:660` → `commit_version_set` `contribution.rs:327` | `has_ehr` precheck missing → **G-6** |
| `list_contributions` (time_range+paging) | §list_contributions | `contribution.rs:~839` (`ensure_ehr_exists` first) | conformant |
| `contribution_count` | §contribution_count | `contribution.rs:~873` | conformant |

---

## 3. Code with no spec item (flags)

| File | Nature | Flag / disposition |
|---|---|---|
| `ehr_access_cache.rs` (68 L) | Per-EHR cache of parsed `EHR_ACCESS` scheme settings, consulted on every EHR-scoped request | **spec-silent** — "no openEHR spec governs this cache — our own design/extension" (already so-labelled, `ehr_access_cache.rs:12`). The *cache* is enterprise-access-control machinery (RBAC/ABAC is a Stage-2 concern per CLAUDE.md). **Quarantine candidate toward register 12** (enterprise access): keep the `EHR_ACCESS` object create+validate in `ehr/access.rs`, but the *scheme cache* moves to the access-control module when that lands. Do not design it here. |
| `ehr_uri.rs` (191 L) | Local `ehr:`-URI → node resolution over the versioned-object read surface | **spec-silent extension** — BASE master11 §"EHR URIs" defines the URI *grammar* but leaves *resolution* to an unspecified name-resolution service (already flagged, `ehr_uri.rs:4-14`). Keep as `ehr/uri.rs`; retain the flag. Foreign-system resolution stays out of scope. |
| `item_tag.rs` (192 L) | ITEM_TAG CRUD (ITS-REST **experimental** tags API) on the `item_tag` table | **extension** — RM `ITEM_TAG` is a real RM class (invariants enforced `item_tag.rs:106-118`) but the tags REST API is development-branch experimental. Keep as `ehr/tags.rs`, flagged as an ITS-REST extension. Not an SM-EHR interface. |
| shared version-meta helpers in `ehr.rs` (`object_version_id`, `version_meta`, `version_response`, `with_uid`, `current_vo`, `latest_version_meta`, `audit`, `committer`) | Cross-cutting versioning glue used by EHR_STATUS **and** COMPOSITION / DIRECTORY / demographic | not EHR-specific → **versioning/ seam** (G-9). |
| raw `ehr` / `ehr_folder` / `item_tag` / `subject_*` SQL inline in `ehr.rs`/`directory.rs`/`item_tag.rs` | Persistence | **storage/ seam** (G-10). No openEHR spec governs the SQL schema (blueprint §2.1) — flag, do not re-litigate table shape (DB schema settled). |

---

## 4. G-row register

| id | citation / flag | severity | disposition |
|---|---|---|---|
| G-1 | `i_ehr_status.adoc` §set/clear_ehr_* + §update_other_details | — | **already-correct** — five discrete mutators now implemented (`status_mutate` `ehr.rs:369`, `api/ehr.rs:285-345`); W-3c gap closed. `replace_ehr_status` kept as the wire aggregate. |
| G-2 | `i_ehr_directory.adoc` §get_versioned_directory / §has_directory_version | — | **already-correct** — both implemented (`directory.rs:111,120`, `api/ehr.rs:611,624`); W-3c gap closed. VERSIONED_FOLDER assembled via the shared `versioned_object` builder. |
| G-3 | RM ehr master04 §Folders; `EHR` `Folders_valid`/`Directory_in_folders`; arch A6 | MED | **quarantine / PORT NOTE** — read side is multi-hierarchy (`live_folder_hierarchies` `ehr.rs:553`); write side single-slot by design (`directory.rs:30`). Owned by **WORKLIST W-6** (closed — merged PR #74) — cross-ref only, do not re-plan. |
| G-4 | `i_ehr_service.adoc` §get_ehrs_for_subject (`List<EHR_SUMMARY>`), §has_ehr_for_subject | LOW | **PORT NOTE + decide** — one-EHR-per-subject (`ehr_subject_uq`) narrows the `List` to ≤1. CNF `create_ehr-two_ehrs_same_patient` expects **409**, which supports the constraint; verify + cite `master06`/CNF before either PORT-NOTEing or lifting. |
| G-5 | `i_ehr_service.adoc` §create_ehr / §create_ehr_with_id `Pre_no_subject` | MED/LOW | **fix-in-rewrite OR PORT NOTE (owner decision)** — precondition (`an_ehr_status.subject = Void`) unenforced; `POST /ehr` intentionally accepts a subject-bearing status (SM-vs-ITS-REST tension). Either reject a non-anonymous `external_ref` on the id-only create paths, or record a cited PORT NOTE (cite `i_ehr_service.adoc` + the ITS-REST `ehr` schema). |
| G-6 | `i_ehr_contribution.adoc` §commit_contribution `Pre_has_ehr` | LOW | **fix-in-rewrite** — `commit_version_set` (`contribution.rs:327`) has no `has_ehr` at entry (confirmed 2026-07-12); a create-only CONTRIBUTION to a missing EHR surfaces a storage FK error, not a clean `ehr_does_not_exist`/404. Add `ensure_ehr_exists(ehr_id)` at the top when `ehr_id = Some`. |
| G-7 | `i_ehr_composition.adoc` §delete_composition (`UUID`) vs §has_composition (`OBJECT_VERSION_ID`) | INFO | **already-correct (deliberate strengthening)** — impl uses `ObjectVersionId` throughout; the spec is internally inconsistent. Keep the PORT NOTE. |
| G-8 | redesign-structural (no spec — this phase's own goal) | — | **fix-in-rewrite** — EHR logic is flat siblings (`ehr.rs`/`composition.rs`/`directory.rs`/`contribution.rs`) + a split-off adapter (`api/ehr.rs`); does not mirror the SM interface set. Fold into `service/ehr/` per §5. |
| G-9 | redesign-structural — shared version-meta helpers | — | **fix-in-rewrite / TODO(w3f-integrate)** — move `object_version_id`/`version_meta`/`with_uid`/`version_response`/`current_vo`/`latest_version_meta` out of `ehr.rs` to a `versioning/` module (used by every versioned kind). |
| G-10 | redesign-structural — inline SQL (spec-silent, DB settled) | — | **TODO(w3f-integrate)** — push `ehr`/`ehr_folder`/`item_tag`/subject-column SQL behind a `storage/` EHR repository seam; do not alter the settled schema. |

---

## 5. Target design — `app/ehrbase/src/service/ehr/`

The folder mirrors `app/ehrbase-sm/src/services/ehr/` **one file per SM
interface**; each file owns its domain logic **and** its `impl
<Interface>Service for EhrbaseService` (collapsing today's `api/ehr.rs` split,
G-8). The spec's own decomposition (six interfaces + the EHR_ACCESS top-level
structure) drives the file set. Every file ≤ ~700 lines.

```
service/ehr/
├── mod.rs          module wiring; adapter helpers shared by all trait impls
│                     (ensure_if_match, parse_at_time, parse_time_range,
│                      version_uid, status_for_subject) + re-exports.
├── service.rs      I_EHR_SERVICE (§2.2): create_ehr(+3 variants), has_ehr,
│                     ehr_by_subject, ehr_summary, summarize_ehr(→EHR_SUMMARY),
│                     ehr_object(_for_subject) adapter seams; impl EhrService.
│                     (~350 L)
├── status.rs       I_EHR_STATUS (§2.4): status_at, status_by_version,
│                     status_update, status_mutate + the 5 mutators,
│                     versioned_status, status_revision_history, status_version,
│                     status_version_at_time, ehr_status_meta,
│                     ehr_is_modifiable + ensure_content_writable,
│                     validate_ehr_status (+ its tests); impl EhrStatusService.
│                     (~600 L; if over, split validate_ehr_status → status/validate.rs)
├── directory.rs    I_EHR_DIRECTORY (§2.5): create/update/delete_directory,
│                     directory_at_time, directory_version, versioned_directory,
│                     has_directory_version, directory_vo(_opt),
│                     live_folder_hierarchies, directory_meta, validate_folder,
│                     select_subfolder; impl EhrDirectoryService. (~560 L)
├── composition.rs  I_EHR_COMPOSITION (§2.6): create/read/update/delete,
│                     composition_at_time, versioned_composition,
│                     composition_version(_at_time), composition_current_meta,
│                     template_of_version, reject_duplicate_persistent,
│                     validate_composition_for_commit + validate_for_commit
│                     dispatch; impl EhrCompositionService.
│                     (~670 L; if over, split validation → composition/validate.rs)
├── contribution/   I_EHR_CONTRIBUTION (§2.7): the existing ~1600-line engine,
│                     itself decomposed (commit engine / read / list-count).
│                     commit_version_set is the SHARED version-commit engine →
│                     versioning/ seam (see below); this folder keeps only the
│                     I_EHR_CONTRIBUTION surface + has-ehr guard (G-6).
├── access.rs       EHR_ACCESS top-level structure (arch A3): default_ehr_access,
│                     validate_ehr_access (create-at-EHR-creation only; no SM
│                     interface, no direct REST write). The EhrAccessCache stays
│                     spec-silent/enterprise → quarantine (register 12), NOT here.
├── tags.rs         ITEM_TAG (ITS-REST experimental extension, §3) — from item_tag.rs.
└── uri.rs          ehr:-URI resolution (spec-silent extension, §3) — from ehr_uri.rs.
```

### Integration seams — `TODO(w3f-integrate)`

- **versioning/** — the `vobject` machinery (`create`/`update`/`delete`/
  `read_current`/`read_version`/`version_at`), `commit_version_set`, `version_id`,
  and `versioned.rs` (`revision_history`/`versioned_object`/`original_version`),
  plus the shared version-meta helpers (G-9). Every `ehr/` file calls into it;
  none should own version-tree mechanics. Governing spec: RM common `master06`
  (VERSION / VERSIONED_OBJECT / CONTRIBUTION / AUDIT_DETAILS).
- **storage/** — raw `ehr` / `ehr_folder` / `item_tag` / subject-column SQL (G-10)
  behind an EHR repository. Spec-silent (DB schema settled — blueprint §2.1);
  flag, don't re-shape.
- **validation/** — `validate_ehr_status` / `validate_ehr_access` /
  `validate_folder` / `validate_composition_for_commit` call
  `openehr_flat::{validate_rm_and_terminology, validate_archetype_conformance}`.
  Keep validators co-located with their type (per file), but the archetype/OPT
  pass is a `validation/` seam. Governing spec: RM ehr master04 + AM.

### Behavioural changes to make during the fold (not just moves)

- **G-6:** add `ensure_ehr_exists` at `commit_version_set` entry (`ehr_id = Some`).
- **G-5:** implement the owner's decision on `Pre_no_subject` (guard or PORT NOTE).
- **G-3 / G-4:** leave as-is (W-6 / pending CNF-cited decision); carry the PORT NOTEs.

---

## 6. PORT-NOTE residue (keep / re-verify / drop)

| PORT NOTE | Location | Action |
|---|---|---|
| `replace_ehr_status` = wire aggregate of the 5 EHR_STATUS mutators | SM `status.rs`; re-stated in `status_mutate` | **keep** — ITS-REST has only the whole-object PUT; the discrete SM calls now exist (G-1), so re-frame as "wire aggregate of these five calls". |
| `get_ehr_status_at_time` restores the `an_ehr_id` the SM signature omits | SM `status.rs:33` | **keep** — genuine spec defect. |
| `delete_composition` uses `OBJECT_VERSION_ID` not the SM's `UUID` (G-7) | `composition.rs` / SM `composition.rs` | **keep** — deliberate cited strengthening; spec internally inconsistent. |
| `get_ehr`/`EHR_SUMMARY` is native-only; `GET /ehr/{id}` returns RM `EHR`, not `EHR_SUMMARY` | SM `service.rs:120-151` | **keep** — ITS-REST extension seam (`ehr_object`). |
| One EHR per subject (G-4) | `ehr.rs:97` / `ehr_subject_uq` | **re-verify** against CNF `two_ehrs_same_patient` (expects 409), then keep with citation. |
| `EHR.folders` multi-hierarchy management is an extension beyond `/directory` (G-3) | `ehr.rs:526`, `directory.rs:28` | **keep** — owned by W-6. |
| `ensure_content_writable` returns 409 for a write to a non-modifiable EHR (wire code underdetermined) | `ehr.rs:603-621` | **keep** — ITS-REST does not enumerate the code; 409 is the closest RFC 9110 semantics, cited. |
| `is_persistent`/`reject_duplicate_persistent` CNF cardinality "under debate in openEHR SEC" | `composition.rs:344-355` | **keep** — CNF-sourced, not RM; cited verbatim. |
| `commit_contribution` typed→wire-JSON→re-parse round-trip glue | `api/ehr.rs:672-680` → moves to `contribution/` | **keep, re-verify** — flagged as glue; a native typed path is a future cleanup, not W-3f. |

---

## W-3f closure (2026-07-13)

The flat EHR siblings folded into `src/service/ehr/` (`service.rs`, `composition.rs`, `composition_validate.rs`, `directory.rs`, `contributions.rs`, `status.rs`, `status_validate.rs`, `access.rs`, `tags.rs`, `uri.rs`, `meta.rs`); version-meta helpers and SQL moved to `versioning/` and `storage/`.

| G | Disposition | Evidence |
|---|---|---|
| G-1 | already-correct | five discrete EHR_STATUS mutators — `service/ehr/status.rs` `status_mutate`; W-3c gap already closed |
| G-2 | already-correct | `get_versioned_directory`/`has_directory_version` — `service/ehr/directory.rs` |
| G-3 | PORT NOTE (quarantine) | read multi-hierarchy / write single-slot by design — `service/ehr/directory.rs`; owned by WORKLIST W-6 (cross-ref only) |
| G-4 | PORT NOTE | one-EHR-per-subject (`ehr_subject_uq`) narrows `List<EHR_SUMMARY>` to ≤1 — `service/ehr/service.rs`, cited CNF `two_ehrs_same_patient`=409 |
| G-5 | PORT NOTE | `Pre_no_subject` — id-only create paths accept subject-bearing status (SM-vs-ITS-REST tension), cited — `service/ehr/service.rs` |
| G-6 | FIXED in code | `versioning/contribution.rs:175` `ensure_ehr_exists` at commit entry when `ehr_id = Some` → clean `ehr_does_not_exist` |
| G-7 | already-correct | `OBJECT_VERSION_ID` used throughout `delete_composition` (deliberate strengthening; spec internally inconsistent) — `service/ehr/composition.rs` |
| G-8 | FIXED (structural) | EHR logic folded into `service/ehr/` per §5 (no longer flat siblings + split adapter) |
| G-9 | FIXED (relocated) | shared version-meta helpers moved to `versioning/` (`object_version_id`, revision-history builders) consumed by every versioned kind |
| G-10 | FIXED (seam) | `ehr`/`ehr_folder`/`item_tag`/subject-column SQL behind `storage/{ehr_repo,tag_repo}.rs`; schema unchanged |

Open residue: none — G-1/G-2/G-7 already-correct, G-3/G-4/G-5 kept PORT NOTE, G-6/G-8/G-9/G-10 fixed in the rewrite.
