# A1 Spec Audit — Verify + Fix — chapter `rm-ehr`

- **Chapter:** RM 1.2.0 ehr package (EHR, EHR_STATUS, EHR_ACCESS, VERSIONED_*)
- **Date:** 2026-07-11
- **Scope:** all 41 requirements `rm-ehr-R1 … R41` (`requirements.md`)
- **Result (defer-nothing pass):** 7 defects fixed
  (R6, R15, R18-adjacent guards were already in, R20/R22, R23, R24, R26, R30,
  R37); the rest verify clean. One spec-internal contradiction recorded
  (RM invariant vs the non-normative ITS-REST example — R5/R6).

## Verdict table

| id | classification | sev | evidence / fix | negative test |
|---|---|---|---|---|
| R1 | verified | high | The EHR object is SERVER-CONSTRUCTED (`service/ehr.rs::ehr_summary`) with all five mandatory attributes — no client write path exists for `EHR` itself | n/a (by construction) |
| R2 | verified | high | `system_id`/`ehr_id` emitted as `HIER_OBJECT_ID` by the builder; never client-supplied | n/a |
| R3 | verified | high | as R2; UUID form used (`uuidv7()` keys) | n/a |
| R4 | verified | med | `time_created` emitted as `DV_DATE_TIME` from the stored row | n/a |
| R5 | verified | high | `ehr_access` ref emitted with `type: VERSIONED_EHR_ACCESS` (`ehr.rs` ehr_summary) per `Ehr_access_valid` | n/a |
| R6 | fixed-in-this-pass | high | `ehr_status` ref emitted `type: EHR_STATUS` → now `VERSIONED_EHR_STATUS` per the NORMATIVE `Ehr_status_valid` invariant; PORT NOTE records the contradictory non-normative ITS-REST example (`schemas/ehr/Ehr.yaml`) | shape asserted in service tests |
| R7/R8 | verified | med | `contributions`/`compositions` are not emitted on the wire EHR (optional emission; ITS-REST Ehr schema omits them); no invalid ref can arise | n/a |
| R9/R10/R11 | verified | med | only `directory` (= `folders[1]`) exists; refs built with `type: VERSIONED_FOLDER` by the directory read (`directory.rs`); multi-hierarchy `folders` is not exposed on any wire — the RM attribute is optional (0..1), absence conforms | n/a |
| R12 | verified | high | fixed pre-chapter (PR #69): `validate_ehr_status` deserializes subject through `PartySelf`; derive rejects foreign `_type`; anonymous `{}` accepted | `ehr.rs::ehr_status_subject_wrong_type_is_rejected`, `type_dispatch.rs` |
| R13 | verified | high | `validate_ehr_status`: subject/is_queryable/is_modifiable presence enforced on create + PUT + CONTRIBUTION (`validate_for_commit`) | invalid-fixture test (11 CNF sets) |
| R14 | verified-with-adjudication | med | `Is_archetype_root` (archetype_details presence) is NOT enforced: the CNF's own VALID EHR_STATUS data sets carry no `archetype_details` (checked all `ehr/valid/*.json`) — the CNF corpus outranks the prose reading (spec-adherence rule); `archetype_node_id` presence IS enforced | corpus fixtures |
| R15 | fixed-in-this-pass | low | `validate_ehr_status`: `other_details` must be a concrete `ITEM_STRUCTURE` (ITEM_TREE/LIST/SINGLE/TABLE); all CNF valid fixtures verified to carry those `_type`s | `ehr.rs::ehr_status_other_details_type_is_enforced` |
| R16 | verified | low | the read path injects `uid` from the version container (`version_response`) — the SHOULD is honoured | — |
| R17 | verified | med | AQL population filter honours `is_queryable` (`aql/sql.rs::queryable_ehr_subquery`) | AQL tests |
| R18 | verified | high | `ensure_content_writable` blocks composition/directory/contribution content writes when `is_modifiable = False` (B2) | `service_ehr.rs` modifiable tests |
| R19 | verified | high | `status_update` does NOT call `ensure_content_writable` — the EHR_STATUS itself stays writable | `service_ehr.rs` set/clear tests |
| R20 | fixed-in-this-pass | med | EHR_ACCESS CONTRIBUTION commits were entirely unvalidated (`validate_for_commit` returned `Ok(())`); now `validate_ehr_access`: structure + `Scheme_valid` (a present `settings` carries its concrete scheme `_type`) | `ehr.rs::ehr_access_commit_validation` |
| R21 | verified-with-adjudication | low | as R14 — `archetype_node_id` enforced; `archetype_details` presence corpus-adjudicated | — |
| R22 | fixed-in-this-pass | low | covered by the R20 fix (settings must be a concrete ACCESS_CONTROL_SETTINGS subtype) | same |
| R23 | fixed-in-this-pass | high | `Archetype_node_id_valid` on VERSIONED_COMPOSITION: enforced at `vobject::apply_change` (Modify) against the FIRST version's root — covers direct update + CONTRIBUTION | `service_ehr_package.rs::versioned_composition_cannot_switch_archetype` |
| R24 | fixed-in-this-pass | high | `Persistent_validity`: category-431 flip across versions rejected at the same seam | `service_ehr_package.rs::versioned_composition_cannot_flip_persistence` |
| R25 | verified | low | `is_persistent` derived from the stored category (never stored independently); the R24 seam pins first-version consistency | same |
| R26 | fixed-in-this-pass | high | `EHR.system_id` was served from LIVE config (`effective_system_id()`); now the STORED per-EHR value in both the EHR body and the SM summary (master04 §Root EHR Object immutability); `ehr.system_id`/`time_created` are never UPDATEd anywhere (checked all `UPDATE ehr` statements — only subject columns) | service tests |
| R27 | verified | med | `create_ehr` commits EHR_STATUS + EHR_ACCESS under ONE contribution (`ehr.rs` create path) | `service_ehr.rs` |
| R28 | verified | med | clone/import creates the ehr row with the LOCAL `effective_system_id()` (`message.rs` import) | `service_import.rs` |
| R29 | verified | med | `import_ehr` reuses the source EHR id (or the fixed id); a new EHR gets `uuidv7()` | `service_import.rs` |
| R30 | fixed-in-this-pass | med | FOLDER trees were entirely unvalidated on all write paths; now `directory::validate_folder` (items must be OBJECT_REFs — never content by value; LOCATABLE structure per node) on directory create/update + the CONTRIBUTION FOLDER path | `directory.rs::folder_items_must_be_object_refs` |
| R31 | verified | med | the directory is its own versioned object (`Kind::Folder` container); no other folder hierarchy exists on any wire | — |
| R32 | verified | med | change-type 250/251 preserved verbatim on modification commits (`contribution.rs::classify`) | codes tests |
| R33 | verified | med | feeder import: `COMPOSITION.feeder_audit` is content (stored verbatim in the canonical body); IMPORTED_VERSION machinery per chapter 1 | `service_import.rs` |
| R34 | verified | low | every commit writes a LOCAL audit row (`insert_audit`/`insert_audit_at` keeps the original as the version's own commit_audit, the local act in the import CONTRIBUTION) | chapter-1 audit |
| R35 | verified | high | indelibility: all writes are new `vo_version` rows; supersession closes periods, never deletes (chapter-1/PR #33 audit) | — |
| R36 | verified | low | a CONTRIBUTION with only attestations/status changes commits fine (no healthcare-event requirement anywhere) | `service_contribution.rs` |
| R37 | fixed-in-this-pass | med | tag targets were unscoped — a tag could target another EHR's object; now the target must exist in the same EHR (`item_tag.rs::replace_tags`) | `service_ehr_package.rs::tag_targets_must_be_within_the_same_ehr` |
| R38 | verified | low | one row per (ehr, target, key) — distinct instances per use; deleting one leaves others | `item_tag` tests |
| R39 | verified | med | tag writes touch only `item_tag` (no `vo_version` write) — no re-versioning | — |
| R40 | verified | low | directory create/update at any time relative to compositions; no temporal coupling exists | — |
| R41 | verified | low | lifecycle 800/801 accepted on commit (`resolve_lifecycle`, chapter-1 R8) | codes tests |

## Fixes applied

See rows R6, R15, R20/R22, R23, R24, R26, R30, R37 — files:
`app/ehrbase/src/service/{ehr,composition,directory,item_tag,vobject}.rs`,
tests in `app/ehrbase/tests/service_ehr_package.rs` + in-file unit tests.

## Deferred

None.

## Uncertain / runtime probes

None remaining.
