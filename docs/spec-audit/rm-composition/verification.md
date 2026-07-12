# A1 Spec Audit — Verify + Fix — chapter `rm-composition`

- **Chapter:** RM 1.2.0 ehr composition/content/navigation/entry packages
- **Date:** 2026-07-11
- **Scope:** all 48 requirements `rm-composition-R1 … R48` (`requirements.md`)
- **Result (defer-nothing pass):** one systemic gap fixed (the five
  "present ⇒ non-empty" list invariants, JSON-level check); everything else
  verifies clean through three existing layers.

## How this chapter is enforced (the three layers)

1. **Fail-closed typed deserialize** — the RM-invariant pass
   (`openehr-flat validation/mod.rs::rm_invariant_pass` →
   `openehr_rm::validate::run::<T>`) deserializes every `_type`-carrying node
   into its generated struct; a non-conforming node (missing mandatory
   attribute, foreign `_type` in a monomorphic slot, wrong nested type) is a
   VIOLATION, never a skip (`validate.rs::run`, fail-closed by design). The
   generated `Composition` struct pins `language`/`territory` `CodePhrase`
   1..1, `category: DvCodedText`, `composer: PartyProxy`,
   `context: Option<EventContext>`, `content: Vec<ContentItem>` (untagged
   closed enum — foreign members fail) — so the whole-tree deserialize covers
   the nested slots including `_type`-less monomorphic children.
2. **Hand-written `*_impl.rs` invariants** dispatched per `_type`
   (`validate_rm_value`): Is_archetype_root on every ENTRY subtype,
   `Action_archetype_id_valid`, `Activity_path_valid`, `Location_validity`, …
3. **The walker terminology pass** (`validation/terminology.rs`):
   composition category / setting / instruction state + transition groups,
   language / territory / encoding code sets.

Both commit paths run all three (direct: `validate_composition_for_commit`;
CONTRIBUTION: `validate_for_commit` — chapter-1/2 verified).

## Verdict table

| id | classification | evidence / fix |
|---|---|---|
| R1 | verified | terminology pass `Group::CompositionCategory` (`terminology.rs` slots_for COMPOSITION) + test `bad_composition_category_reported` |
| R2 | verified | `CodeSet::Countries` (same site) |
| R3 | verified | `CodeSet::Languages` (same site) |
| R4 | fixed-in-this-pass | `Content_valid` — new JSON-level `check_nonempty_lists` (`validation/mod.rs`); absent ≠ present-empty is only distinguishable pre-deserialize |
| R5 | verified | `composition_impl.rs` `Is_archetype_root` |
| R6/R7 | verified | `Composition.language/territory: CodePhrase` (1..1) — fail-closed deserialize |
| R8 | verified | `category: DvCodedText` — a plain `DV_TEXT` payload (no `defining_code`) fails deserialize; a foreign `_type` fails the derive check |
| R9 | verified | `composer: PartyProxy` non-optional |
| R10 | verified | `context: Option<EventContext>` — monomorphic; foreign `_type` fails the derive (`type_dispatch.rs` pins the derive behaviour) |
| R11 | verified | `is_persistent` = category 431 (`composition.rs::is_persistent`, used by duplicate-persistent + Persistent_validity checks) |
| R12/R13 | verified | `EventContext.start_time: DvDateTime`, `setting: DvCodedText` non-optional |
| R14 | verified | `Group::Setting` terminology pass |
| R15 | fixed-in-this-pass | `Participations_validity` — `check_nonempty_lists` |
| R16 | verified | `event_context_impl.rs` `Location_validity` |
| R17 | verified | `health_care_facility: Option<PartyIdentified>` — foreign `_type` fails the derive |
| R18 | fixed-in-this-pass | `Items_valid` — `check_nonempty_lists` |
| R19 | verified | `items: Vec<ContentItem>` closed untagged enum — non-CONTENT_ITEM members fail deserialize |
| R20/R21 | verified | presence: `Entry.language/encoding: CodePhrase` non-optional; code-set membership: terminology pass (ENTRY subtypes arm) |
| R22 | verified | `subject: PartyProxy` non-optional on every ENTRY subtype |
| R23 | verified-vacuous | `subject_is_self` is a derived function with no wire attribute — the invariant has no independently checkable wire state |
| R24 | fixed-in-this-pass | `Other_participations_valid` — `check_nonempty_lists` (all five concrete ENTRY subtypes + GENERIC_ENTRY) |
| R25 | verified | `*_impl.rs` Is_archetype_root per ENTRY subtype |
| R26 | verified | `AdminEntry.data: ItemStructure` non-optional |
| R27/R28 | verified | `Observation.data: History<ItemStructure>` non-optional, `state: Option<History<…>>` — foreign `_type` fails deserialize |
| R29 | verified | `Evaluation.data: ItemStructure` non-optional |
| R30 | verified | `Instruction.narrative: DvText` non-optional (accepts the DV_CODED_TEXT subtype via the DvText closed enum) |
| R31 | fixed-in-this-pass | `Activities_valid` — `check_nonempty_lists` |
| R32 | verified | `activities: Vec<Activity>` — monomorphic member type, foreign `_type` fails the derive |
| R33 | verified | `activity_impl.rs` `Action_archetype_id_valid` |
| R34 | verified | `Activity.description: ItemStructure` non-optional |
| R35 | verified | `Action.time: DvDateTime` non-optional |
| R36 | verified | `ism_transition: IsmTransition` non-optional monomorphic |
| R37 | verified | `instruction_details: Option<InstructionDetails>` monomorphic |
| R38 | verified | `Action.description: ItemStructure` non-optional |
| R39 | verified | `IsmTransition.current_state: DvCodedText` non-optional |
| R40/R41 | verified | terminology pass `Group::InstructionState`/`InstructionTransition` |
| R42 | verified | `InstructionDetails.instruction_id: LocatableRef` non-optional |
| R43 | verified | `instruction_details_impl.rs` `Activity_path_valid` |
| R44 | verified | the openEHR terminology bundle carries the instruction state/transition groups verbatim (TERM 3.1.0 assets; chapter 15's surface) |
| R45 | verified-via-archetype-conformance | the careflow_step ↔ ISM-state mapping lives in the ACTION archetype; the walker enforces whatever the OPT constrains on `careflow_step`/`current_state` — no additional wire-checkable rule exists beyond the archetype constraint |
| R46 | verified | `protocol`/`guideline_id` optional (`Option<…>`); absence accepted |
| R47 | verified | as R19, for `COMPOSITION.content` |
| R48 | verified | `context: Option<EventContext>` — a persistent composition without context is accepted (and the duplicate-persistent guard does not require context) |

## Fixes applied

- **R4/R15/R18/R24/R31** — `crates/openehr-flat/src/validation/mod.rs::check_nonempty_lists`:
  the RM's "present implies non-empty" list invariants, enforced at the JSON
  level in the RM-invariant pass (post-deserialize, absent and present-empty
  collapse into one `Vec`, so the typed impls cannot see the difference).
  Test: `validation/tests.rs::present_empty_lists_are_rejected`.

## Deferred

None.

## Uncertain / runtime probes

None remaining.
