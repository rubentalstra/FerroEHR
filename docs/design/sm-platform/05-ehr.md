# EHR Service (SM `master05`) — spec-conformance audit

Read-only audit (2026-07-12) of the SM **EHR service** chapter against the
implementation. Scope: `I_EHR_SERVICE` and the five per-EHR interfaces
(`I_EHR`, `I_EHR_STATUS`, `I_EHR_DIRECTORY`, `I_EHR_COMPOSITION`,
`I_EHR_CONTRIBUTION`) plus the supporting classes `EHR_SUMMARY`, `UV_FOLDER`,
`UV_COMPOSITION` (and the referenced `UPDATE_VERSION` / `UPDATE_AUDIT`). The
overall verdict is at the end of §1; §3 is the gap register (`G-n`), §4 the
target design for the open items, §5 the honest PORT-NOTE residue.

**Spec oracle** (read before any change):

- `docs/specs/openehr/SM/docs/openehr_platform/master05-ehr_service.adoc`
  (the chapter; `include::`s the 9 class files below)
- `docs/specs/openehr/SM/docs/UML/classes/` —
  `i_ehr_service.adoc`, `i_ehr.adoc`, `i_ehr_status.adoc`,
  `i_ehr_directory.adoc`, `i_ehr_composition.adoc`,
  `i_ehr_contribution.adoc`, `ehr_summary.adoc`, `uv_folder.adoc`,
  `uv_composition.adoc`, plus `update_version.adoc`, `update_audit.adoc`
- Adjacent RM (the semantics the SM service realizes):
  `docs/specs/openehr/RM/docs/ehr/master04-ehr_package.adoc` (EHR class,
  EHR Creation, EHR Active Status, Folders); RM common `master06` (change
  control / versioning); `docs/specs/openehr/RM/docs/UML/classes/…ehr_status.adoc`,
  `…ehr_access.adoc`, `…folder.adoc`

**Current implementation** (verified 2026-07-12):

- SM native traits (one per interface):
  `app/ehrbase-sm/src/services/ehr.rs` (`EhrService`, 133 lines),
  `ehr_status.rs` (`EhrStatusService`, 102), `directory.rs`
  (`EhrDirectoryService`, 78), `composition.rs` (`EhrCompositionService`, 125),
  `contribution.rs` (`EhrContributionService`, 77). Wired in
  `app/ehrbase-sm/src/services/mod.rs:39-64`. `I_EHR` is realized as the
  generic handle `crate::IEhr` built by `EhrService::i_ehr`
  (`ehr.rs:94-99`).
- Trait impls on `EhrbaseService`: `app/ehrbase/src/service/api/ehr.rs`
  (`EhrService` @132, `EhrStatusService` @240, `EhrCompositionService` @328,
  `EhrDirectoryService` @466, `EhrContributionService` @551) — thin adapters
  onto the domain modules.
- Domain logic: `app/ehrbase/src/service/ehr.rs` (EHR + `EHR_STATUS`),
  `directory.rs` (FOLDER), `composition.rs` (COMPOSITION), `contribution.rs`
  (CONTRIBUTION), all on the shared `vobject` versioned-object machinery
  (`app/ehrbase/src/service/vobject.rs`).
- REST wire: `app/ehrbase-rest/src/dispatch/ehr.rs` (the ITS-REST 1.0.3 EHR /
  EHR_STATUS / DIRECTORY / COMPOSITION / CONTRIBUTION operations dispatch on
  the generated route table).

---

## 1. Compliance verdict

**Faithful and near-complete on the read + commit surface; three real gaps on
the mutation surface.** Every read, create, update, delete, and versioned-read
operation of all six interfaces is present with parameter-name and return-type
parity to the SM signatures, and the change-control semantics behind them
(implicit CONTRIBUTION creation, `OBJECT_VERSION_ID` construction, logical
delete = lifecycle `523`, audit-copy rule, optimistic `If-Match`) are
correctly realized. The gaps are:

- `I_EHR_STATUS`'s five fine-grained mutators (`set/clear_ehr_queryable`,
  `set/clear_ehr_modifiable`, `update_other_details`) do not exist as discrete
  SM calls — folded into a whole-object `replace_ehr_status` (G-1).
- `I_EHR_DIRECTORY.get_versioned_directory` (→ `VERSIONED_FOLDER`) and
  `has_directory_version` are not implemented at all — the DIRECTORY analogue
  of `get_versioned_composition` is missing (G-2).
- The `create_ehr` / `create_ehr_with_id` `Pre_no_subject` precondition is not
  enforced, and one-EHR-per-subject narrows `get_ehrs_for_subject` (G-4, G-5).

None of these block CORE/STANDARD conformance today (the ECC baseline is
341/315/0); they are SM-surface fidelity gaps. Faithfully realized items are
listed in §2 so the audit records what is *right*, not only what is missing.

---

## 2. What is faithfully realized (verified)

`I_EHR_SERVICE` (`i_ehr_service.adoc`) — all 9 members present, signatures
matched (`ehrbase-sm/src/services/ehr.rs`, impl `api/ehr.rs:132-237`):

| SM call | Trait method | Impl evidence |
|---|---|---|
| `has_ehr` | `has_ehr` | `api/ehr.rs:133` → `ensure_ehr_exists` |
| `has_ehr_for_subject` | `has_ehr_for_subject` | `api/ehr.rs:141` (see G-4) |
| `create_ehr` | `create_ehr` | `api/ehr.rs:152` → `service/ehr.rs:27` |
| `create_ehr_with_id` | `create_ehr_with_id` | `api/ehr.rs:159`; dup id → `Conflict` (`ehr_create_fail_duplicate_id`), `service/ehr.rs:48-52` |
| `create_ehr_for_subject` | `create_ehr_for_subject` | `api/ehr.rs:169`; subject set via `status_for_subject` (`api/ehr.rs:108-129`) |
| `create_ehr_for_subject_with_id` | `create_ehr_for_subject_with_id` | `api/ehr.rs:183` |
| `get_ehr: EHR_SUMMARY` | `get_ehr` | `api/ehr.rs:197` → `summarize_ehr` (`service/ehr.rs:230-268`) — all 6 `EHR_SUMMARY` attrs (`ehr_summary.adoc`), incl. `composition_count` = distinct versioned objects, `system_id` from the stored per-EHR value |
| `get_ehrs_for_subject: List<EHR_SUMMARY>` | `get_ehrs_for_subject` | `api/ehr.rs:201` (see G-4) |
| `i_ehr: I_EHR` | `i_ehr` → `IEhr` handle | `ehr.rs:94-99` |

EHR creation obeys RM ehr `master04` §EHR Creation: root `EHR`, `EHR_STATUS`,
and `EHR_ACCESS` all committed under **one** CONTRIBUTION
(`service/ehr.rs:54-88`); default `EHR_STATUS` is queryable + modifiable +
`PARTY_SELF` (`service/ehr.rs:691-700`). `EHR.system_id` is stored immutably at
creation and served from storage, never the live config (`service/ehr.rs:39-52,
117-149`).

`I_EHR_STATUS` reads (`i_ehr_status.adoc`) — all present
(`ehr_status.rs`, impl `api/ehr.rs:240-325`): `has_ehr_status_version`
(`@241`), `get_ehr_status` (`@254`), `get_ehr_status_at_time` (`@258`),
`get_ehr_status_at_version` (`@267`, returns the **bare** `EHR_STATUS`, F-01-03),
`get_versioned_ehr_status` (`@281`). The `EHR_STATUS.is_modifiable = False`
write-guard (RM ehr `master04` §EHR Active Status) is implemented in
`ensure_content_writable` (`service/ehr.rs:567-577`) and applied to all content
writes — composition (`composition.rs:24,177,298`), directory
(`directory.rs:24,117,148`), and content-CONTRIBUTION
(`contribution.rs:563-567`) — while leaving `EHR_STATUS` itself always writable.

`I_EHR_COMPOSITION` (`i_ehr_composition.adoc`) — all 8 members present with
signatures matched (`composition.rs`, impl `api/ehr.rs:328-462`):
`has_composition`, `get_composition_latest`, `get_composition_at_time`,
`get_composition_at_version`, `get_versioned_composition`, `create_composition`,
`update_composition` (optimistic `If-Match` via `ensure_if_match`,
`api/ehr.rs:61-76,408-411`), `delete_composition` (logical delete → lifecycle
`523|deleted|` per the SM meaning, `composition.rs:90-98`). Creates a
`VERSIONED_OBJECT` + `ORIGINAL_VERSION` + CONTRIBUTION as specified.

`I_EHR_DIRECTORY` (`i_ehr_directory.adoc`) — 7 of 10 members present
(`directory.rs`, impl `api/ehr.rs:466-548`): `has_directory` (`@467`),
`has_path` (`@475`, slash-separated Folder-name path, `service/directory.rs:270-282`),
`create_directory`, `get_directory`/`get_directory_at_time` (the SM's
`get_directory` is the `a_time = Void` case of the one method), `update_directory`,
`delete_directory` (logical delete), `get_directory_at_version`. FOLDER content
validation enforces `items` = `OBJECT_REF` only (RM ehr `master04` §Folders,
`service/directory.rs:209-266`). See G-2 for the 3 absent members.

`I_EHR_CONTRIBUTION` (`i_ehr_contribution.adoc`) — all 5 members present
(`contribution.rs`, impl `api/ehr.rs:551-629`): `has_contribution`,
`get_contribution` (+ `Prefer: resolve_refs` variant), `commit_contribution`
(atomic multi-`UPDATE_VERSION` set under one CONTRIBUTION + `UPDATE_AUDIT`,
`service/contribution.rs:327-604`; the master06 §Committal m4 audit-copy rule
is implemented at `service/contribution.rs:357-374`), `list_contributions`
(time-range + paging, `@839`), `contribution_count` (`@873`). `UV_FOLDER` /
`UV_COMPOSITION` are the `T`-bound forms of `UPDATE_VERSION` — realized by the
shared native `UpdateVersion` type (`ehrbase_sm::types::UpdateVersion`) carrying
`preceding_version_uid`, `lifecycle_state`, `attestations`, `data`, and the
`commit_audit` (`UPDATE_AUDIT`), matching `update_version.adoc` /
`update_audit.adoc`.

---

## 3. Gap register

Every gap cites the governing spec text.

| # | Gap | Spec citation | Today |
|---|-----|---------------|-------|
| G-1 | **The five `I_EHR_STATUS` mutators are absent as discrete SM calls.** The SM defines `set_ehr_queryable`, `clear_ehr_queryable`, `set_ehr_modifiable`, `clear_ehr_modifiable` (each `0..1`, each with a `Post_is_*_set/cleared` post-condition) and `update_other_details(an_ehr_id, a_details: ITEM_TREE)`. `EhrStatusService` exposes none of them; all `EHR_STATUS` mutation goes through the composite `replace_ehr_status` (whole-object PUT). The per-flag calls and their post-conditions are therefore not individually invokable or asserted. | `i_ehr_status.adoc` (`set_ehr_queryable`/`clear_ehr_queryable`/`set_ehr_modifiable`/`clear_ehr_modifiable`/`update_other_details`) | Documented deviation: `ehr_status.rs:63-79` PORT NOTE argues the ITS-REST wire replaces the whole object, so the toggles are realized "jointly". True for the *wire*, but the SM native surface still lacks the calls. |
| G-2 | **`get_versioned_directory` (→ `VERSIONED_FOLDER`) and `has_directory_version` are not implemented.** COMPOSITION has `get_versioned_composition`; DIRECTORY has no analogue — no `VERSIONED_FOLDER` container object is ever assembled (grep: no `VERSIONED_FOLDER` producer in `app/ehrbase*/src`). `has_directory_version(an_ehr_id, a_version_uid): Boolean` is likewise missing from `EhrDirectoryService`. | `i_ehr_directory.adoc` (`get_versioned_directory: VERSIONED_FOLDER`, `has_directory_version`) | Absent. The ECC runner adjudicates `get_versioned_directory` onto `GET /directory/{version_uid}` (= `get_directory_at_version`, blueprint B5/D2), but that returns a bare `FOLDER`, not the `VERSIONED_FOLDER` version container the SM call names. |
| G-3 | **`EHR.folders` multi-hierarchy write is single-slot.** RM allows an EHR to index several FOLDER hierarchies (`EHR.folders [0..1] List<...>`, directory = `folders.item(1)`). Reads already expose all live hierarchies (`live_folder_hierarchies`, `service/ehr.rs:509-524`; emitted in the EHR body `service/ehr.rs:195-213`), but `create_directory` rejects a second hierarchy through the single directory slot (`service/directory.rs:30-34`), and only a raw CONTRIBUTION can add further hierarchies — there is no management API for non-directory hierarchies. | RM ehr `master04` §Folders; `EHR` class `Folders_valid`/`Directory_in_folders` | Cross-reference **WORKLIST W-6** (active branch `claude/w3b-arch-gaps`); do not re-plan here. ITS-REST binds only `/directory`, so a multi-hierarchy surface is an extension. |
| G-4 | **One EHR per subject narrows `get_ehrs_for_subject` / `has_ehr_for_subject`.** The SM types the result `List<EHR_SUMMARY>` and describes "EHR or EHRs" / "there are EHR(s)". The impl resolves a single EHR (`ehr_by_subject`, `service/ehr.rs:97-113`; unique constraint `ehr_subject_uq`), so `get_ehrs_for_subject` returns a list of at most one (`api/ehr.rs:201-221`). | `i_ehr_service.adoc` (`get_ehrs_for_subject: List<EHR_SUMMARY>`, `has_ehr_for_subject`) | One-EHR-per-subject is a deliberate storage design, but the SM leaves the cardinality open. Flag + decide (PORT NOTE with a CNF cross-check, or lift the constraint). LOW. |
| G-5 | **`create_ehr` / `create_ehr_with_id` `Pre_no_subject` is not enforced.** Both SM calls carry `Pre_no_subject: an_ehr_status.subject = Void` (subject is set only via the `*_for_subject` variants). The impl accepts any client `EHR_STATUS`, subject included (`api/ehr.rs:152-167`; `validate_ehr_status` never checks `subject = Void` for this path, `service/ehr.rs:764-834`). | `i_ehr_service.adoc` (`create_ehr` / `create_ehr_with_id` `Pre_no_subject`) | ITS-REST `POST /ehr` intentionally allows a full `EHR_STATUS` body, so this is a genuine SM-vs-ITS-REST tension. Record the deviation with a citation rather than silently ignore the precondition. MED/LOW. |
| G-6 | **`commit_contribution` / `list`/`count` `has_ehr` precondition partially deferred.** `list_contributions`/`contribution_count` correctly `ensure_ehr_exists` first (`contribution.rs:845,878`), but `commit_version_set` has no `has_ehr` check at entry (`contribution.rs:327-355`); a **create-only** CONTRIBUTION to a non-existent EHR relies on the DB FK / `require_kind`, so the failure may surface as a storage error rather than a clean `ehr_does_not_exist` (404). | `i_ehr_contribution.adoc` (`commit_contribution` `Pre_has_ehr`) | Verify whether `dispatch/ehr.rs` pre-checks EHR existence for `POST /ehr/{id}/contribution`; if not, add an `ensure_ehr_exists` at the top of `commit_version_set` when `ehr_id = Some`. LOW. |
| G-7 | **`delete_composition` parameter type vs the spec.** The SM types `delete_composition(a_version_uid: UUID)` (and `delete_directory` takes no version at all), yet `has_composition`/`get_composition_at_version` type the same concept as `OBJECT_VERSION_ID`. The impl uses the stronger `ObjectVersionId` throughout (`composition.rs:94-98`). This is a *stricter* (better) choice, but it diverges from the literal `UUID` in the delete signature and from `delete_directory`'s no-argument form (the impl adds an `If-Match`-derived `preceding_version_uid`, `directory.rs:64-68`). | `i_ehr_composition.adoc` (`delete_composition: a_version_uid UUID`); `i_ehr_directory.adoc` (`delete_directory`) | The spec is internally inconsistent (`UUID` vs `OBJECT_VERSION_ID`); the impl is the safer reading. Record as a deliberate, cited strengthening — not a defect. INFO. |

Minor, recorded but not carried as `G-`rows: `get_ehr_status_at_time` restores
the `an_ehr_id` argument the SM signature omits (spec defect, PORT NOTE
`ehr_status.rs:33-37`); `get_ehr_status_at_version` adds a `VERSION_TREE_ID`
string beside the `UUID` to fully address branch versions; the spec typo
`esubject_id_does_not_exist` is preserved in the doc-comment
(`ehr.rs:80-81`).

---

## 4. Target design for the open items

Only G-1, G-2 need new native surface; G-3 is W-6; G-4/G-5/G-6 are
decisions + small guards.

### 4.1 G-1 — restore the fine-grained `I_EHR_STATUS` mutators

Add the five SM calls to `EhrStatusService` (`ehrbase-sm/src/services/ehr_status.rs`)
as first-class methods, each committing a new `EHR_STATUS` version + implicit
CONTRIBUTION over the existing `status_update` machinery:

```
set_ehr_queryable(an_ehr_id)      clear_ehr_queryable(an_ehr_id)
set_ehr_modifiable(an_ehr_id)     clear_ehr_modifiable(an_ehr_id)
update_other_details(an_ehr_id, a_details /* ITEM_TREE */)
```

Each reads the current `EHR_STATUS`, flips the one scalar (`is_queryable` /
`is_modifiable`) or replaces `other_details`, re-validates
(`validate_ehr_status`), and commits — asserting the SM post-condition
(`Post_is_queryable_set`, …) in a unit test. `replace_ehr_status` stays as the
wire composite (keep the PORT NOTE, now framed as "the wire aggregate of these
five SM calls" rather than "the SM calls have no discrete form"). No new wire is
required (ITS-REST has no per-flag endpoint); the calls are exercised natively
and via the whole-object PUT.

### 4.2 G-2 — `get_versioned_directory` + `has_directory_version`

Add both to `EhrDirectoryService`, mirroring the COMPOSITION path:

- `has_directory_version(an_ehr_id, a_version_uid) -> bool` — resolve the
  directory `vo_id` (`directory_vo_opt`), test the version exists.
- `get_versioned_directory(an_ehr_id) -> VERSIONED_FOLDER` — assemble the
  `VERSIONED_FOLDER` container from the directory `vo_id` via the existing
  `versioned_object` builder that already backs `get_versioned_composition` /
  `get_versioned_ehr_status` (`service/ehr.rs:347-353` pattern), typed
  `VERSIONED_FOLDER`. No new storage; the `vobject` machinery already holds the
  version tree.

This closes the DIRECTORY hole and lets the ECC runner bind
`get_versioned_directory` to a genuine `VERSIONED_FOLDER` producer rather than
adjudicating it onto `get_directory_at_version`.

### 4.3 G-5, G-6 — precondition guards

- G-5: on `create_ehr` / `create_ehr_with_id`, if the supplied `EHR_STATUS`
  carries a non-anonymous `subject.external_ref`, either reject (strict
  `Pre_no_subject`) **or** record a cited PORT NOTE that the ITS-REST wire
  intentionally relaxes it (`POST /ehr` accepts a subject-bearing status). Owner
  decision required; whichever way, cite `i_ehr_service.adoc` + the ITS-REST
  `ehr` schema.
- G-6: add `ensure_ehr_exists(ehr_id)` at the top of `commit_version_set` when
  `ehr_id = Some` so a create-only CONTRIBUTION to a missing EHR is a clean
  `ehr_does_not_exist` (404), not a storage FK error.

### 4.4 G-3, G-4 — cross-referenced / decisions

- G-3: owned by **W-6** on `claude/w3b-arch-gaps`; this audit only records that
  the *read* side is already multi-hierarchy and the *write* side is single-slot
  by design.
- G-4: decide one-EHR-per-subject vs `List` cardinality; the safe path is a
  cited PORT NOTE after a CNF `master06` cross-check (the CNF `two_ehrs_same_patient`
  case already expects a **409**, which supports one-EHR-per-subject — verify and
  cite before lifting the constraint).

---

## 5. Standing PORT-NOTE residue (the honest record)

- `replace_ehr_status` is the wire aggregate of the five SM `EHR_STATUS`
  mutators (G-1) — kept even after 4.1 restores the discrete calls, because
  ITS-REST has only the whole-object PUT.
- `get_ehr_status_at_time` restores the `an_ehr_id` argument the SM signature
  omits (spec defect, `ehr_status.rs:33-37`).
- `delete_composition` uses `OBJECT_VERSION_ID`, not the SM's literal `UUID`
  (G-7) — a deliberate, cited strengthening; the spec is internally
  inconsistent.
- `get_ehr`/`EHR_SUMMARY` is native-only: the ITS-REST `GET /ehr/{id}` wire
  returns the RM `EHR` object, not `EHR_SUMMARY` (`ehr.rs:101-132` adapter
  seam). No SM call emits the RM `EHR`; that is an ITS-REST extension
  (documented).
- One EHR per subject (G-4) — pending the cited decision above.
- `EHR.folders` multi-hierarchy management API (G-3) is an extension beyond the
  ITS-REST-bound `/directory`; owned by W-6.
