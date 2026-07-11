# rm-composition — machine-checkable normative requirements

- **Chapter:** rm-composition
- **Date:** 2026-07-11
- **Component / spec version:** RM 1.2.0 (vendored `docs/specs/openehr/RM/`)
- **Spec files read (all existed as listed):**
  - `RM/docs/ehr/master05-composition_package.adoc` (COMPOSITION / EVENT_CONTEXT prose + includes)
  - `RM/docs/ehr/master06-content_package.adoc` (CONTENT_ITEM)
  - `RM/docs/ehr/master07-navigation_package.adoc` (SECTION)
  - `RM/docs/ehr/master08-entry_package.adoc` (ENTRY hierarchy, ISM, careflow mapping)
  - `RM/docs/UML/classes/org.openehr.rm.composition.composition.adoc`
  - `…event_context.adoc`, `…content_item.adoc`, `…section.adoc`
  - `…entry.adoc`, `…care_entry.adoc`, `…admin_entry.adoc`
  - `…observation.adoc`, `…evaluation.adoc`, `…instruction.adoc`, `…activity.adoc`,
    `…action.adoc`, `…instruction_details.adoc`, `…ism_transition.adoc`

Notes on approach: invariants are quoted from each class's `*Invariants*` block;
mandatory/monomorphic-slot rejection duties are derived from the `*..*`
cardinality column and the declared attribute type. "Monomorphic slot" = a slot
typed to a concrete class with no RM subtypes (`EVENT_CONTEXT`, `ISM_TRANSITION`,
`INSTRUCTION_DETAILS`, `ACTIVITY`, `HISTORY`, `DV_CODED_TEXT`), which must reject
any foreign `_type` payload.

---

| id | requirement | citation | category | risk |
|----|-------------|----------|----------|------|
| rm-composition-R1 | `COMPOSITION.category` invariant `Category_validity`: `category.defining_code` MUST be a code in the openEHR terminology group `composition category` (`terminology(openehr).has_code_for_group_id(Group_id_composition_category, category.defining_code)`); reject otherwise. | composition.composition.adoc, Invariants "Category_validity" (l.60) | rejection-duty | high |
| rm-composition-R2 | `COMPOSITION.territory` invariant `Territory_valid`: `territory` MUST be a code in the openEHR `countries` code set (ISO 3166); reject otherwise. | composition.composition.adoc, Invariants "Territory_valid" (l.63) | rejection-duty | high |
| rm-composition-R3 | `COMPOSITION.language` invariant `Language_valid`: `language` MUST be a code in the openEHR `languages` code set; reject otherwise. | composition.composition.adoc, Invariants "Language_valid" (l.66) | rejection-duty | high |
| rm-composition-R4 | `COMPOSITION.content` invariant `Content_valid`: `content /= Void implies not content.is_empty` — if `content` is present it MUST be a non-empty List (reject an empty `content` array). | composition.composition.adoc, Invariants "Content_valid" (l.69) | invariant | high |
| rm-composition-R5 | `COMPOSITION` invariant `Is_archetype_root`: a COMPOSITION MUST be an archetype root (`is_archetype_root` true). | composition.composition.adoc, Invariants "Is_archetype_root" (l.72) | invariant | medium |
| rm-composition-R6 | `COMPOSITION.language` is mandatory (1..1) and typed `CODE_PHRASE`; reject a COMPOSITION with no `language`. | composition.composition.adoc, Attributes language 1..1 (l.21-23) | mandatory-attr | high |
| rm-composition-R7 | `COMPOSITION.territory` is mandatory (1..1) and typed `CODE_PHRASE`; reject a COMPOSITION with no `territory`. | composition.composition.adoc, Attributes territory 1..1 (l.25-27) | mandatory-attr | high |
| rm-composition-R8 | `COMPOSITION.category` is mandatory (1..1) and typed exactly `DV_CODED_TEXT` (a monomorphic-leaf slot); reject a missing category or a non-`DV_CODED_TEXT` payload (e.g. plain `DV_TEXT`). | composition.composition.adoc, Attributes category 1..1 (l.29-37) | mandatory-attr | high |
| rm-composition-R9 | `COMPOSITION.composer` is mandatory (1..1) and typed `PARTY_PROXY`; reject a COMPOSITION with no composer (all content must be created by some person/agent). | composition.composition.adoc, Attributes composer 1..1 (l.43-45); master05 §Composer (l.25) | mandatory-attr | high |
| rm-composition-R10 | `COMPOSITION.context` is optional (0..1) and typed exactly `EVENT_CONTEXT` (monomorphic slot, no subtypes); when present reject any foreign `_type`. | composition.composition.adoc, Attributes context 0..1 (l.39-41) | rejection-duty | medium |
| rm-composition-R11 | `COMPOSITION.is_persistent()` returns True iff `category` code is `431|persistent|`, False otherwise. | composition.composition.adoc, Functions is_persistent (l.55-57) | behaviour | low |
| rm-composition-R12 | `EVENT_CONTEXT.start_time` is mandatory (1..1) and typed `DV_DATE_TIME`; reject an EVENT_CONTEXT with no start_time. | event_context.adoc, Attributes start_time 1..1 (l.18-20); master05 §Time (l.67) | mandatory-attr | high |
| rm-composition-R13 | `EVENT_CONTEXT.setting` is mandatory (1..1) and typed exactly `DV_CODED_TEXT`; reject a missing or non-`DV_CODED_TEXT` setting. | event_context.adoc, Attributes setting 1..1 (l.30-32) | mandatory-attr | high |
| rm-composition-R14 | `EVENT_CONTEXT` invariant `Setting_valid`: `setting.defining_code` MUST be a code in the openEHR terminology group `setting`; reject otherwise. | event_context.adoc, Invariants "Setting_valid" (l.47) | rejection-duty | high |
| rm-composition-R15 | `EVENT_CONTEXT` invariant `Participations_validity`: `participations /= Void implies not participations.is_empty` — if present, `participations` MUST be non-empty. | event_context.adoc, Invariants "Participations_validity" (l.50) | invariant | medium |
| rm-composition-R16 | `EVENT_CONTEXT` invariant `location_valid`: `location /= Void implies not location.is_empty` — if present, `location` string MUST be non-empty. | event_context.adoc, Invariants "location_valid" (l.53) | invariant | medium |
| rm-composition-R17 | `EVENT_CONTEXT.health_care_facility`, when present (0..1), is typed exactly `PARTY_IDENTIFIED`; reject a foreign `_type` in that slot. | event_context.adoc, Attributes health_care_facility 0..1 (l.38-40) | rejection-duty | low |
| rm-composition-R18 | `SECTION` invariant `Items_valid`: `items /= Void implies not items.is_empty` — if `items` is present it MUST be a non-empty List (reject an empty `items` array). | section.adoc, Invariants "Items_valid" (l.26) | invariant | medium |
| rm-composition-R19 | `SECTION.items` (0..1) is a `List<CONTENT_ITEM>` whose members may only be `SECTION` or `ENTRY`-subtype instances (concrete `CONTENT_ITEM` descendants); reject a member whose `_type` is not a concrete CONTENT_ITEM subtype. | section.adoc, Attributes items (l.18-24); content_item.adoc (abstract ancestor); master07 §Overview | rejection-duty | medium |
| rm-composition-R20 | `ENTRY.language` is mandatory (1..1) `CODE_PHRASE` and invariant `Language_valid`: MUST be a code in the openEHR `languages` code set; reject a missing or invalid language on any ENTRY subtype. | entry.adoc, Attributes language 1..1 (l.20-22) + Invariants "Language_valid" (l.67) | rejection-duty | high |
| rm-composition-R21 | `ENTRY.encoding` is mandatory (1..1) `CODE_PHRASE` and invariant `Encoding_valid`: MUST be a code in the openEHR `character sets` code set; reject a missing or invalid encoding on any ENTRY subtype. | entry.adoc, Attributes encoding 1..1 (l.24-26) + Invariants "Encoding_valid" (l.70) | rejection-duty | high |
| rm-composition-R22 | `ENTRY.subject` is mandatory (1..1) and typed `PARTY_PROXY`; reject any ENTRY subtype with no subject. | entry.adoc, Attributes subject 1..1 (l.36-44) | mandatory-attr | high |
| rm-composition-R23 | `ENTRY` invariant `Subject_validity` / `subject_is_self` post-condition: `subject_is_self implies subject.generating_type = "PARTY_SELF"` — when the entry is about the record subject, `subject` MUST be a `PARTY_SELF` instance. | entry.adoc, Functions subject_is_self post-condition (l.63) + Invariants "Subject_validity" (l.73) | invariant | medium |
| rm-composition-R24 | `ENTRY` invariant `Other_participations_valid`: `other_participations /= Void implies not other_participations.is_empty` — if present, MUST be non-empty. | entry.adoc, Invariants "Other_participations_valid" (l.76) | invariant | low |
| rm-composition-R25 | `ENTRY` invariant `Is_archetype_root`: every ENTRY MUST be an archetype root. | entry.adoc, Invariants "Is_archetype_root" (l.79) | invariant | medium |
| rm-composition-R26 | `ADMIN_ENTRY.data` is mandatory (1..1) and typed `ITEM_STRUCTURE`; reject an ADMIN_ENTRY with no data. | admin_entry.adoc, Attributes data 1..1 (l.22-24) | mandatory-attr | high |
| rm-composition-R27 | `OBSERVATION.data` is mandatory (1..1) and typed exactly `HISTORY<ITEM_STRUCTURE>` (monomorphic slot); reject a missing data or a non-`HISTORY` payload. | observation.adoc, Attributes data 1..1 (l.20-22) | mandatory-attr | high |
| rm-composition-R28 | `OBSERVATION.state`, when present (0..1), is typed exactly `HISTORY<ITEM_STRUCTURE>`; reject a non-`HISTORY` payload in that slot. | observation.adoc, Attributes state 0..1 (l.24-26) | rejection-duty | medium |
| rm-composition-R29 | `EVALUATION.data` is mandatory (1..1) and typed `ITEM_STRUCTURE`; reject an EVALUATION with no data. | evaluation.adoc, Attributes data 1..1 (l.20-22) | mandatory-attr | high |
| rm-composition-R30 | `INSTRUCTION.narrative` is mandatory (1..1) and typed `DV_TEXT` (accepts `DV_CODED_TEXT` subtype); reject an INSTRUCTION with no narrative. | instruction.adoc, Attributes narrative 1..1 (l.20-22) | mandatory-attr | high |
| rm-composition-R31 | `INSTRUCTION` invariant `Activities_valid`: `activities /= Void implies not activities.is_empty` — if present, `activities` MUST be a non-empty List. | instruction.adoc, Invariants "Activities_valid" (l.37) | invariant | medium |
| rm-composition-R32 | `INSTRUCTION.activities` (0..1) is `List<ACTIVITY>` where `ACTIVITY` is a monomorphic concrete class; reject a member whose `_type` is not `ACTIVITY`. | instruction.adoc, Attributes activities 0..1 (l.32-34); activity.adoc | rejection-duty | medium |
| rm-composition-R33 | `ACTIVITY.action_archetype_id` is mandatory (1..1) `String` and invariant `Action_archetype_id_valid`: `not action_archetype_id.is_empty` — reject a missing or empty value (spec default `/.*/`). | activity.adoc, Attributes action_archetype_id 1..1 (l.27-31) + Invariants (l.38) | invariant | high |
| rm-composition-R34 | `ACTIVITY.description` is mandatory (1..1) and typed `ITEM_STRUCTURE`; reject an ACTIVITY with no description. | activity.adoc, Attributes description 1..1 (l.33-35) | mandatory-attr | high |
| rm-composition-R35 | `ACTION.time` is mandatory (1..1) and typed `DV_DATE_TIME`; reject an ACTION with no time. | action.adoc, Attributes time 1..1 (l.18-21) | mandatory-attr | high |
| rm-composition-R36 | `ACTION.ism_transition` is mandatory (1..1) and typed exactly `ISM_TRANSITION` (monomorphic slot); reject a missing value or a foreign `_type`. | action.adoc, Attributes ism_transition 1..1 (l.22-24) | mandatory-attr | high |
| rm-composition-R37 | `ACTION.instruction_details`, when present (0..1), is typed exactly `INSTRUCTION_DETAILS` (monomorphic slot); reject a foreign `_type`. | action.adoc, Attributes instruction_details 0..1 (l.26-28) | rejection-duty | medium |
| rm-composition-R38 | `ACTION.description` is mandatory (1..1) and typed `ITEM_STRUCTURE`; reject an ACTION with no description. | action.adoc, Attributes description 1..1 (l.30-32) | mandatory-attr | high |
| rm-composition-R39 | `ISM_TRANSITION.current_state` is mandatory (1..1) and typed exactly `DV_CODED_TEXT`; reject a missing or non-`DV_CODED_TEXT` current_state. | ism_transition.adoc, Attributes current_state 1..1 (l.18-20) | mandatory-attr | high |
| rm-composition-R40 | `ISM_TRANSITION` invariant `Current_state_valid`: `current_state.defining_code` MUST be a code in the openEHR terminology group `instruction states`; reject otherwise. | ism_transition.adoc, Invariants "Current_state_valid" (l.35) | rejection-duty | high |
| rm-composition-R41 | `ISM_TRANSITION` invariant `Transition_valid`: `transition /= Void implies transition.defining_code` MUST be a code in the openEHR terminology group `instruction transitions`; reject an out-of-group transition code. | ism_transition.adoc, Invariants "Transition_valid" (l.38-39) | rejection-duty | high |
| rm-composition-R42 | `INSTRUCTION_DETAILS.instruction_id` is mandatory (1..1) and typed `LOCATABLE_REF`; reject INSTRUCTION_DETAILS with no instruction_id. | instruction_details.adoc, Attributes instruction_id 1..1 (l.18-20) | mandatory-attr | high |
| rm-composition-R43 | `INSTRUCTION_DETAILS.activity_id` is mandatory (1..1) `String` and invariant `Activity_path_valid`: `not activity_id.is_empty` — reject a missing or empty activity_id. | instruction_details.adoc, Attributes activity_id 1..1 (l.22-24) + Invariants (l.37) | invariant | high |
| rm-composition-R44 | ISM `current_state`/`transition` codes are drawn from the openEHR terminology groups "Instruction states" and "Instruction transitions"; the state set is INITIAL/PLANNED/POSTPONED/SCHEDULED/CANCELLED/ACTIVE/SUSPENDED/ABORTED/COMPLETED/EXPIRED (CANCELLED, ABORTED, COMPLETED terminal; EXPIRED pseudo-terminal). | master08 §Standard ISM (l.284-304) | behaviour | low |
| rm-composition-R45 | An `ACTION`'s recorded careflow step MUST be one of the careflow steps defined in the corresponding Instruction's archetype mapping (careflow→ISM state mapping is specified in the ACTION archetype). | master08 §Careflow Process to State Machine Mapping (l.338) | behaviour | low |
| rm-composition-R46 | `CARE_ENTRY.protocol` (0..1 `ITEM_STRUCTURE`) and `guideline_id` (0..1 `OBJECT_REF`) are both optional; no rejection on absence. | care_entry.adoc, Attributes (l.18-24) | mandatory-attr | low |
| rm-composition-R47 | A COMPOSITION `content` List (when present) may hold `SECTION` and/or `ENTRY`-subtype instances directly or in any combination (SECTIONs, SECTION trees, or bare ENTRYs); each member's `_type` MUST be a concrete `CONTENT_ITEM` descendant. | master05 §Composition Content (l.87-96); content_item.adoc | rejection-duty | medium |
| rm-composition-R48 | Persistent COMPOSITIONs MAY optionally carry an EVENT_CONTEXT (relaxed after release 1.0.3); absence of context on a persistent composition MUST NOT be rejected. | master05 §Occurrence in Data NOTE (l.63) | behaviour | low |
