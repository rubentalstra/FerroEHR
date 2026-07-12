# A1 Spec Audit — Verify + Fix — chapter `rm-ehr-extract`

- **Chapter:** RM 1.2.0 ehr_extract (master04–09 + UML classes)
- **Date:** 2026-07-11
- **Scope:** all 50 requirements `rm-ehr-extract-R1 … R50`
- **Result (defer-nothing pass):** 6 gaps fixed — the export silently ignored
  `include_multimedia` and `link_depth`, accepted any `extract_type`, built no
  demographics chapter; the import accepted `Item_validity`-violating masked
  items and could not land demographics-chapter parties.

## Verdict table (condensed)

| ids | classification | evidence / fix |
|---|---|---|
| R1/R44 | verified | `assemble_extract` sequence_nr from the entity index (≥ 1); sole response = 1 |
| R2 | fixed-in-this-pass | import enforces `Item_validity` (`is_masked xor item`): masked-with-item and unmasked-without-item both rejected; masked-without-item skipped |
| R3 | verified | `version_selection` rejects `include_data=false ∧ include_revision_history=false` |
| R4–R6 | verified-no-surface | EXTRACT_UPDATE_SPEC governs persisted extract requests — no request-persistence surface exists (single-shot export only); the generated struct pins the shape for any future surface. Flagged: our extract wire is the SM native API, no openEHR REST binding exists |
| R7/R8 | verified | EXTRACT_PARTICIPATION function/mode groups — walker terminology pass covers PARTICIPATION-family groups on any instance |
| R9/R11/R12/R48 | verified | `build_openehr_content_item` builds uid/owner_id/time_created from the versioned-object surface; counts = `versions.len()` / total stored |
| R10 | verified | chapter-1 R20 fix: `versions[]` members must be `ORIGINAL_VERSION` |
| R13 | verified | `kind_from_x_versioned` rejects non-`X_VERSIONED_*` items (+ party routing below) |
| R14/R15/R30–R34 | verified-no-surface | X_CONTRIBUTION/SYNC_*/MESSAGE/ADDRESSED_MESSAGE: generated structs pin the shapes; no message-transport surface produces or consumes them (flagged extension boundary) |
| R16/R17 | verified | `assemble_extract`: time_created/system_id (HIER_OBJECT_ID)/sequence_nr all present |
| R18/R20–R25/R27–R29 | verified | typed `ExtractSpec`/`ExtractVersionSpec`/manifest structs — fail-closed deserialize at the SM seam |
| R19 | fixed-in-this-pass | `validate_extract_type`: member of the extract content type group (openehr-ehr / openehr-demographic / generic-emr / other) — flagged: the group is published in the EHR-Extract spec, not the terminology XML |
| R26 | verified-no-surface | EXTRACT_ACTION_REQUEST has no surface (no request persistence); struct pins the shape |
| R35 | verified | `EXTRACT_PARTICIPATION.performer: String` in the generated type |
| R36/R37 | verified | exports serve the exact stored `ORIGINAL_VERSION`s (whole-form, each with its own commit audit) — never diffs |
| R38/R41 | fixed-in-this-pass | demographics chapter: locally-held parties referenced via `PARTY_REF` (namespace `demographic`) are written into a second `EXTRACT_CHAPTER` as `X_VERSIONED_PARTY` items (`is_primary=false`); import lands them into the demographic repository (skip-if-exists), under an ehr-less import CONTRIBUTION |
| R39/R50 | fixed-in-this-pass | `link_depth` following: same-EHR `DV_LINK` targets added iteratively with `is_primary=false` to depth n (targets outside this repository cannot be included — flagged); `is_primary` now parameterised |
| R40 | verified | stored refs are namespace-`local` already; export serves them verbatim |
| R42 | fixed-in-this-pass | `include_multimedia=false` strips inline `DV_MULTIMEDIA.data` (metadata + uri remain) from every exported version incl. demographics |
| R43 | verified | `include_data=false` → `versions` empty + revision_history (R3 guarantees it) |
| R45 | verified | absent `version_spec` → latest-only |
| R46 | verified-no-surface | SYNC family (see R30) |
| R47 | verified | EXTRACT_FOLDER items 0..1 — empty folders permitted (typed) |
| R49 | verified | GENERIC_CONTENT_ITEM import rejected with a typed error (not silently dropped) |

## Fixes applied

`app/ehrbase/src/service/message.rs` — `validate_extract_type`,
`strip_inline_multimedia`, `link_target_uuids` + the following loop,
`demographic_chapter_items`, `Item_validity` on import, X_VERSIONED_PARTY
routing; `app/ehrbase/src/service/vobject.rs` —
`commit_demographic_import` (+ `commit_import_scoped` refactor,
`insert_imported_vo_version` scope becomes optional — demographic parties
have no owning EHR). Test: `service_extract.rs::extract_spec_flags_are_honoured`.

## Deferred

None.

## Uncertain / runtime probes

None remaining.
