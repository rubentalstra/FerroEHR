# SM digest 2/6 — Platform Service Model: EHR service

Part of the SM-platform design set (`docs/design/sm-platform/README.md`).
Sources: `docs/specs/openehr/SM/docs/openehr_platform/master05-ehr_service.adoc`
+ the included UML class files (all read in full).

## 1. Component structure

Package `sm.platform.interface.ehr` — the service interface to the `EHR`
component. Shape:

- **`I_EHR_SERVICE : I_STATUS`** — the entry interface (the `: I_STATUS`
  inheritance is stated in the master02 IDL example, not in the class table).
- **`I_EHR`** — a per-EHR *accessor*, not an operation interface: four
  mandatory attributes `ehr_status: I_EHR_STATUS`, `directory:
  I_EHR_DIRECTORY`, `compositions: I_EHR_COMPOSITION`, `contributions:
  I_EHR_CONTRIBUTION`. Obtained via `I_EHR_SERVICE.i_ehr(ehr_id)`.
- Data classes: `EHR_SUMMARY`, `UV_COMPOSITION`, `UV_FOLDER`; error enum
  `EHR_CALL_STATUS_TYPE`.
- `I_EHR_EXTRACT_SERVICE` is **not** included by master05 (it is included by
  the Message chapter) — captured in digest 4.

## 2. `I_EHR_SERVICE` (`i_ehr_service.adoc`)

"Primary interface to `EHR_SERVICE` persistent repository." Plus inherited
`last_call_failed()` / `last_call_status()`.

| Call | Args → Returns | Pre / Post | Errors | Semantics |
|---|---|---|---|---|
| `has_ehr` | `(ehr_id: UUID): Boolean` | — | — | EHR exists. |
| `has_ehr_for_subject` | `(a_subject_id: PARTY_REF): Boolean` | — | `ehr_does_not_exist` | EHR(s) exist for subject. |
| `create_ehr` | `(an_ehr_status: EHR_STATUS [0..1]): UUID` | pre `an_ehr_status.subject = Void`; post `has_ehr(Result)` | — | System-generated id. Default `EHR_STATUS` if absent: `is_modifiable` + `is_queryable` True; default `subject` = `PARTY_SELF`. |
| `create_ehr_with_id` | `(an_ehr_id: UUID, an_ehr_status [0..1]): UUID` | pre no-subject + `not has_ehr(an_ehr_id)`; post `has_ehr(Result)` | `ehr_create_fail_duplicate_id` | Client-supplied id; id echoed as safety check. |
| `create_ehr_for_subject` | `(a_subject_id: PARTY_REF, an_ehr_status [0..1]): UUID` | — | `ehr_for_subject_already_exists` | `EHR_STATUS.subject` set to the subject id. |
| `create_ehr_for_subject_with_id` | `(an_ehr_id: UUID, a_subject_id: PARTY_REF, an_ehr_status [0..1]): UUID` | pre `not has_ehr(an_ehr_id)` | `ehr_create_fail_duplicate_id` | Both ids client-supplied. |
| `get_ehr` | `(an_ehr_id: UUID): EHR_SUMMARY` | pre `has_ehr` | `ehr_id_does_not_exist` | Summarised EHR root + EHR_STATUS. |
| `get_ehrs_for_subject` | `(a_subject_id: PARTY_REF): List<EHR_SUMMARY>` | — | `esubject_id_does_not_exist` (spec typo, verbatim) | All EHRs whose `ehr_status.subject_id` matches. |
| `i_ehr` | `(ehr_id: UUID): I_EHR` | — | `ehr_id_does_not_exist` | Access the per-EHR interfaces. |

## 3. `I_EHR_STATUS` (`i_ehr_status.adoc`)

"Interface to `EHR_STATUS` of an EHR, **with implicit Contribution
creation**" — every mutating call creates a new EHR_STATUS version +
CONTRIBUTION server-side.

| Call | Args → Returns | Pre / Post | Notes |
|---|---|---|---|
| `has_ehr_status_version` | `(an_ehr_id: UUID, a_version_uid: UUID): Boolean` | pre `has_ehr` | |
| `get_ehr_status` | `(an_ehr_id: UUID): EHR_STATUS` | pre `has_ehr` | current version |
| `get_ehr_status_at_time` | `(a_time: Iso8601_date_time [0..1]): EHR_STATUS` | pre `has_ehr(an_ehr_id)` | **spec defect: `an_ehr_id` missing from signature**; no time ⇒ latest |
| `set_ehr_queryable` / `set_ehr_modifiable` | `(an_ehr_id: UUID)` | pre `has_ehr`; post flag set | versioned flag mutation |
| `clear_ehr_queryable` / `clear_ehr_modifiable` | `(an_ehr_id: UUID)` | pre `has_ehr`; post flag cleared | |
| `update_other_details` | `(an_ehr_id: UUID, a_details: ITEM_TREE)` | pre `has_ehr` | new version of `other_details` |
| `get_ehr_status_at_version` | `(an_ehr_id: UUID, a_version_uid: UUID): EHR_STATUS` | pre `has_ehr` | |
| `get_versioned_ehr_status` | `(an_ehr_id: UUID): VERSIONED_EHR_STATUS` | pre `has_ehr` + `has_ehr_status_version(an_ehr_id, a_version_uid)` (**defect: references arg not in signature**) | |

## 4. `I_EHR_DIRECTORY` (`i_ehr_directory.adoc`)

"Operations on EHR directory, with implicit Contribution creation."

| Call | Args → Returns | Pre / Post | Errors |
|---|---|---|---|
| `has_directory` | `(ehr_id): Boolean` | pre `has_ehr` | — |
| `has_path` | `(ehr_id, a_path: String): Boolean` — path = slash-separated Folder `name`s | pre `has_ehr` | `ehr_id_does_not_exist` |
| `create_directory` | `(ehr_id, a_dir_struct: UV_FOLDER)` | pre `has_ehr` + `definitions_valid` + `not has_directory` + `valid_content` | `ehr_id_does_not_exist`, `definition_unknown`, `content_invalid`; creates VERSIONED_OBJECT + ORIGINAL_VERSION + CONTRIBUTION |
| `get_directory` | `(ehr_id): FOLDER` | pre `has_ehr` | current, else Void |
| `get_directory_at_time` | `(an_ehr_id, a_time [0..1]): FOLDER` | pre `has_ehr` | no time ⇒ latest |
| `update_directory` | `(ehr_id, a_dir_struct: UV_FOLDER)` | pre `has_ehr` + `definitions_valid` + `valid_content` + `has_directory`; **preceding version must be supplied and correct** (optimistic lock) | new ORIGINAL_VERSION + CONTRIBUTION |
| `delete_directory` | `(ehr_id)` | pre `has_ehr` + `has_directory` | logical delete = new version with contents removed |
| `has_directory_version` | `(an_ehr_id, a_version_uid: UUID): Boolean` | — | `ehr_id_does_not_exist` |
| `get_directory_at_version` | `(an_ehr_id, a_version_uid: UUID): FOLDER` | — | `version_does_not_exist` |
| `get_versioned_directory` | `(an_ehr_id): VERSIONED_FOLDER` | pre `has_ehr` | |

## 5. `I_EHR_COMPOSITION` (`i_ehr_composition.adoc`)

"Interface for commit and retrieve of Compositions, with implicit
Contribution creation."

| Call | Args → Returns | Pre / Post | Errors |
|---|---|---|---|
| `has_composition` | `(an_ehr_id, a_version_uid: OBJECT_VERSION_ID): Boolean` | pre `has_ehr` | `ehr_id_does_not_exist` |
| `get_composition_latest` | `(an_ehr_id, a_versioned_object_uid: UUID): COMPOSITION` | pre `has_ehr` + `has_composition` | `composition_does_not_exist` |
| `get_composition_at_time` | `(an_ehr_id, a_versioned_object_uid: UUID, a_time [0..1]): COMPOSITION` | pre `has_ehr` | no time ⇒ latest |
| `get_composition_at_version` | `(an_ehr_id, a_version_uid: OBJECT_VERSION_ID): COMPOSITION` | — | `ehr_does_not_exist`, `object_version_does_not_exist` |
| `get_versioned_composition` | `(an_ehr_id, a_versioned_object_uid: UUID): VERSIONED_COMPOSITION` | pre `has_ehr` | `versioned_composition_does_not_exist` |
| `create_composition` | `(an_ehr_id, a_comp: UV_COMPOSITION): UUID` | pre `has_ehr` + `definitions_valid` + `valid_content`; post `has_composition(an_ehr_id, Result)` | `composition_already_exists`, `definition_unknown`, `content_invalid`; creates VERSIONED_OBJECT + ORIGINAL_VERSION + CONTRIBUTION |
| `update_composition` | `(an_ehr_id, a_comp: UV_COMPOSITION): UUID` | pre `has_ehr` + `definitions_valid` + `valid_content` | `composition_does_not_exist`, …; new ORIGINAL_VERSION + CONTRIBUTION |
| `delete_composition` | `(an_ehr_id, a_version_uid: UUID)` | pre `has_ehr` | logical delete: new version, content removed, lifecycle `523\|deleted\|`; arg = current top version (type inconsistency vs OBJECT_VERSION_ID elsewhere) |

## 6. `I_EHR_CONTRIBUTION` (`i_ehr_contribution.adoc`)

"Interface for explicit Contribution level operations."

| Call | Args → Returns | Pre / Post | Errors |
|---|---|---|---|
| `has_contribution` | `(an_ehr_id, a_contrib_id: UUID)` (return type absent in source; Boolean implied) | pre `has_ehr` | `ehr_id_does_not_exist` |
| `get_contribution` | `(an_ehr_id, a_contrib_id: UUID): CONTRIBUTION` | pre `has_ehr` + `has_contribution` | `contribution_does_not_exist` |
| `commit_contribution` | `(an_ehr_id, versions: List<UPDATE_VERSION>, an_audit: UPDATE_AUDIT): UUID` | pre `has_ehr`; post `has_contribution` | **the explicit multi-version atomic commit**: "Commit a `CONTRIBUTION` containing any number of `UPDATE_VERSION` objects." |
| `list_contributions` | `(an_ehr_id, time_range: Interval<Iso8601_date_time> [0..1], item_offset, items_to_fetch): List<UUID>` | — | `ehr_does_not_exist` |
| `contribution_count` | `(ehr_id, time_range [0..1]): Integer` | — | `ehr_does_not_exist` |

## 7. Data classes

**`EHR_SUMMARY`** — "Summary form of `EHR` + `EHR_STATUS` … convenient for
use in service interface": `ehr_id: UUID [1]`, `system_id: String [1]`,
`ehr_status: EHR_STATUS [1]`, `time_created: Iso8601_date_time [1]`,
`contribution_count: Integer [1]`, `composition_count: Integer [1]`.

**`UV_COMPOSITION`** / **`UV_FOLDER`** — `UPDATE_VERSION<COMPOSITION>` /
`UPDATE_VERSION<FOLDER>`, no extra attributes (base class + `UPDATE_AUDIT`:
digest 1 §4.4).

**`EHR_CALL_STATUS_TYPE`** (extends `CALL_STATUS_TYPE`):
`composition_does_not_exist`, `contribution_does_not_exist`,
`composition_archetype_invalid`, `ehr_create_fail_duplicate_id`,
`composition_already_exists`, `ehr_for_subject_already_exists`.

## 8. Versioning / audit / contribution semantics (the load-bearing core)

1. Every mutating call = one self-contained transaction; multi-call
   realizations must be jointly transactionally protected.
2. Implicit path (status/directory/composition create+update+delete): server
   creates CONTRIBUTION + ORIGINAL_VERSION(s) (+ VERSIONED_OBJECT on first
   version) per call.
3. Explicit path: `commit_contribution` bundles N `UPDATE_VERSION`s under one
   CONTRIBUTION + one `UPDATE_AUDIT` — atomic.
4. Client/server split: client supplies `UPDATE_VERSION`
   (`preceding_version_uid` [not for first version], `lifecycle_state`
   [always], `attestations?`, `data`, `audit`); server generates
   `time_committed`, `system_id`, ids.
5. Logical delete = new version, content removed, lifecycle `523|deleted|`;
   no physical delete outside Admin.
6. Directory update carries the optimistic-concurrency rule explicitly.

## 9. Spec defects flagged (verbatim in source)

`get_ehr_status_at_time` missing `an_ehr_id` arg; `get_versioned_ehr_status`
precondition references undeclared `a_version_uid`; `valid_content` vs
`content_valid` naming; `esubject_id_does_not_exist` typo;
`ehr_does_not_exist` vs `ehr_id_does_not_exist` used inconsistently;
`definition_unknown`/`content_invalid`/`version_does_not_exist` appear in no
vendored enum; `has_contribution` lacks a return type; `delete_composition`
takes plain `UUID` where sibling calls use `OBJECT_VERSION_ID`;
`composition_archetype_invalid` defined but never referenced.

## 10. Mapping note (current code)

Our `ehrbase-rest::backend::EhrService` seam + `ehrbase::service::vobject`
already realize this interface family almost 1:1 (implicit + explicit
contribution paths, temporal reads, logical delete, optimistic `If-Match`).
The gap analysis (doc 07) itemises call-level correspondence.
