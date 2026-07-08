# SM digest 5/6 — Simplified Information Model 'B' (SIM-B)

Part of the SM-platform design set (`docs/design/sm-platform/README.md`).
Sources: `docs/specs/openehr/SM/docs/simplified_im_b/*.adoc` + the S_* UML
class files (all read in full).

## 1. Spec identity

"openEHR Simplified Information Model 'B' (SIM-B)", SM Release 1.0.0
(**unreleased**), issue **0.7.0** (2019-07-17, SPECSM-2), status
**DEVELOPMENT**. "Initial writing, adapted from the Marand Better Platform
'Web Templates' specification." Purpose (verbatim): "a simplified form of the
openEHR RM, intended to be used to generate the simplified JSON Data Template
(sJDT) format … designed to enable easier creation of openEHR content."

SIM-B is the *model* layer under the SDT/FLAT serial JSON encodings (digest
6 is the *serialization* layer). Conversion S_XXX ↔ canonical RM requires a
computable RM (the BMM) + micro-parsers — exactly the assets we already have
(`openehr_rm::model` via `emit-rm-model`, ADR-008).

## 2. Simplification principles (`master02-overview.adoc`, verbatim list)

1. String representation of RM-structured fields (e.g.
   `CODE_PHRASE.terminology_id: TERMINOLOGY_ID` → `String`).
2. Replacing `DV_CODED_TEXT`/`CODE_PHRASE` fields whose vocabulary is fixed
   by the RM (openEHR/IANA vocabularies) with Strings.
3. Compressing sub-part objects into the parent (e.g. `S_PARTY_PROXY`
   inlines `PARTY_PROXY.external_ref` fields) — shorter paths.

Naming: RM class `X` → `S_X`. Scope: `COMPOSITION` + everything reachable
(~45 classes). Structural classes stay RM-shaped with `S_` part types; leaf
classes carry the simplifications.

## 3. SDT serial format variants (`master04-sim_data_types.adoc` §Serial Formats)

Four variants per value: path-structure/terse, path-structure/regular,
regular-structure/regular, regular-structure/terse.

- `S_DV_TEXT` terse: `{"a/b/c/d": "anxiety"}`; regular:
  `{"a/b/c/d|value": "anxiety"}`.
- `S_DV_CODED_TEXT` terse: `{"a/b/c/d": "snomed_ct::48694002|anxiety|"}`
  (`<terminology>::<code>|<value>|`); regular: `|terminology`, `|code`,
  `|value` keys.
- `S_DV_QUANTITY` terse: `{"a/b/c/d": "125 mm[Hg]"}` (`<magnitude>
  <units>`, space-separated); regular: `|magnitude` (number), `|units`.
- `S_DV_PARSABLE` regular: `|formalism`, `|value`.
- Open TBD in source: "are the inner attributes supposed to have bars or
  not?" (twice).

Note the divergences inside the SM component itself: SIM-B terse quantity is
`"125 mm[Hg]"` (space), SDF (digest 6) says `"78.500,kg"` (comma), Better
FLAT uses `|magnitude`/`|unit` (singular). Our `openehr-flat` follows Better
(the interop target); record with `// PORT NOTE:`.

## 4. Class inventory (complete)

Root contract — `S_TYPE` (abstract interface): `to_rm(rm: BMM_MODEL): Any`
(abstract), `from_rm(a_val: Any)` (abstract). In-source TODO: the
`to_rm`/`from_rm` approach may be dropped in favour of the rules tables.

### 4.1 Base package

- `S_OBJECT_ID` (stub, no attrs) → `S_GENERIC_ID`: `scheme: String [1]`.
- `S_OBJECT_REF`: `id_namespace: String [0..1]` (= `OBJECT_REF.namespace`),
  `id_type: String [0..1]` (= `.type`), `id: S_OBJECT_ID [1]`.

### 4.2 Data types (`S_DATA_VALUE` subtree, inherits `S_TYPE`)

| Class | Attributes | Simplifies |
|---|---|---|
| `S_DV_TEXT` | `value: String [1]` | `DV_TEXT` — `formatting`/`language`/`encoding` **skipped (lossy)** |
| `S_DV_CODED_TEXT` (inherits `S_DV_TEXT`) | `code: String [1]`, `terminology: String [1]` | `defining_code.code_string` / `.terminology_id` |
| `S_CODE_PHRASE` | `code [1]`, `terminology [1]` | `CODE_PHRASE` |
| `S_DV_IDENTIFIER` | none added — **dual-inherits RM `DV_IDENTIFIER` + `S_DATA_VALUE`** | reuses RM shape |
| `S_DV_PARSABLE` | `value [1]`, `formalism [1]` | `DV_PARSABLE` |
| `S_DV_QUANTITY`, `S_DV_COUNT`, `S_DV_PROPORTION`, `S_DV_ORDINAL` | **`TODO: define` — attributes undefined in spec** | |
| `S_DV_BOOLEAN` | stub (no desc/attrs) | |

### 4.3 Data structures

- `S_ITEM` (abstract, inherits `S_LOCATABLE`) → `S_CLUSTER`: `items:
  List<S_ITEM> [0..1]`; `S_ELEMENT`: `value: S_DATA_VALUE [0..1]`,
  `null_flavour: String [0..1]` (coded → string).
- `S_EVENT` (abstract): `time: String [1]` (from DV_DATE_TIME), `state:
  List<S_ITEM> [0..1]`, `data: List<S_ITEM> [0..1]` (ITEM_TREE collapsed).
  → `S_POINT_EVENT` (nothing added); `S_INTERVAL_EVENT`: `width: String [1]`
  (DV_DURATION), `sample_count: Integer [0..1]`, `math_function: String [1]`
  (coded → string).

### 4.4 Common

- `S_LOCATABLE` (abstract, inherits `S_TYPE`): `name: S_DV_TEXT [1]`,
  `links: List<S_LINK> [0..1]`, `feeder_audit: S_FEEDER_AUDIT [0..1]`,
  `archetype_details: S_ARCHETYPED [0..1]`.
- `S_ARCHETYPED`: `archetype_id: String [1]`, `template_id: String [0..1]`,
  `rm_version: String [0..1]`.
- `S_LINK`: `meaning: String [1]`, `type: String [1]`, `target: String [1]`
  (TODO in source: "what if original was coded?").
- `S_FEEDER_AUDIT`: `originating_system_audit: S_FEEDER_AUDIT_DETAILS [1]`,
  `original_content: S_DV_PARSABLE [0..1]` (TODO: DV_MULTIMEDIA case?),
  `feeder_system_item_id` / `originating_system_item_id:
  List<S_DV_IDENTIFIER> [0..1]`.
- `S_FEEDER_AUDIT_DETAILS`: `system_id [1]`, `version_id [1]` — in-source
  TODO "removes 4 attributes from original RM form - lossy copy?".
- `S_PARTY_PROXY`: `id [0..1]` (= `external_ref.id.value`), `id_namespace
  [0..1]` (= `external_ref.namespace`), `id_scheme [0..1]` (=
  `external_ref.id.scheme` iff GENERIC_ID). → `S_PARTY_IDENTIFIED`: `name:
  String [1]` (TODO: `identifiers` not included).
- `S_PARTICIPATION` (performer PARTY_SELF): `id [1]` (=
  `performer.external_ref.id.value`), `mode [0..1]` (coded → string),
  `function [1]`. → `S_PARTICIPATION_IDENTIFIED`: `name [1]`.

### 4.5 Composition

- `S_COMPOSITION` (inherits `S_LOCATABLE`): `content: S_CONTENT_ITEM [1]`
  (**spec error: RM content is a list**), `composer: S_PARTY_PROXY [1]`,
  `language: String [1]` (meaning cell erroneously says "Converted from
  DV_DATE_TIME"), `territory: String [1]` (ISO 3166-1), `category: String
  [0..1]` ("event"/"persistent"), `context: S_EVENT_CONTEXT [0..1]`.
- `S_CONTENT_ITEM` (abstract) → `S_SECTION`: `items: List<S_CONTENT_ITEM>
  [0..1]`; `S_ENTRY` (abstract): `subject: S_PARTY_PROXY [0..1]` (RM 1..1
  relaxed — APP_CONTEXT defaults), `provider [0..1]`, `other_participations:
  List<S_PARTICIPATION> [0..1]`.
- `S_ADMIN_ENTRY`: `data: List<S_ITEM> [1]` (ITEM_STRUCTURE collapsed).
- `S_CARE_ENTRY` (abstract): `protocol: List<S_ITEM> [0..1]`.
- `S_OBSERVATION` — HISTORY wrappers collapsed: `data`/`state:
  List<S_EVENT> [0..1]` + `history_origin/period/duration: String [0..1]`,
  `history_summary: List<S_ITEM> [0..1]`, `state_origin/period/duration/
  summary` (TODOs: rename `history_*` → `data_*`).
- `S_EVALUATION`: `data: List<S_ITEM> [1]`.
- `S_INSTRUCTION`: `narrative: String [0..1]` (RM-mandatory relaxed),
  `expiry_time: String [0..1]`, `activities: List<S_ACTIVITY> [0..1]`.
- `S_ACTIVITY` (standalone): `timing: String [0..1]` (DV_PARSABLE),
  `action_archetype_id: String [0..1]`, `description: List<S_ITEM> [1]`.
- `S_ACTION` — `ism_transition` + `instruction_details` collapsed: `time:
  String [1]`, `description: List<S_ITEM> [1]`, `current_state: String [1]`,
  `careflow_step: S_DV_CODED_TEXT [0..1]`, `transition: String [0..1]`,
  `instruction_id: String [0..1]`, `activity_id: String [0..1]`.
- `S_EVENT_CONTEXT` (inherits `S_TYPE`): `time: String [1]` (RM
  `start_time`), `end_time [0..1]`, `location [0..1]`, `participations:
  List<S_PARTICIPATION> [0..1]`, `health_care_facility: S_PARTY_IDENTIFIED
  [0..1]`, `setting: String [0..1]`, `other_context: List<S_ITEM> [0..1]`.

### 4.6 App context (`sm.app_context`) — the FLAT `ctx/` model

- `APP_CONTEXT` — "application context data items that may be set for a
  COMPOSITION commit"; defaults enabling clients to build Compositions for
  the REST API. Attributes (each with To-RM/From-RM rules in source):
  `language [1]` (ISO 639-1 → openEHR CODE_PHRASE), `territory [1]`,
  `composer_name [1]`, `composer_id [0..1]`, `time [1]` (default for
  `context.start_time`, `HISTORY.origin`, event times; absent ⇒ now),
  `end_time [0..1]`, `category [0..1]`, `setting [0..1]` (TODO: EhrScape
  codes 435 laboratory / 436 imaging possibly not in openEHR terminology),
  `history_origin [0..1]`, `id_namespace [0..1]`, `id_scheme [0..1]`,
  `provider_name/provider_id [0..1]`, `participation_name/id/function/mode/
  identifiers: List<String> [0..1]`, `action_time [0..1]`,
  `action_ism_transition_current_state: Integer [0..1]`,
  `instruction_narrative [0..1]`, `healthcare_facility: S_PARTY_IDENTIFIED
  [0..1]`, `activity_timing [0..1]`, `location [0..1]`, `workflow_id:
  S_OBJECT_REF [0..1]`.
- `APP_COMPOSITION` (inherits `S_COMPOSITION`): `ctx: APP_CONTEXT [0..1]`.

This is the normative basis for the `ctx/*` FLAT keys `openehr-flat`
implements (Better semantics) — the one place SM does specify the FLAT
context vocabulary.

## 5. Transformation rules (`master07-transformation_rules.adoc`)

Rule vocabulary: `collapse()` (flatten intermediate container — HISTORY,
ITEM_TREE, ISM_TRANSITION, INSTRUCTION_DETAILS — promoting children into the
SIM parent), `copy()` (verbatim), `default`, `skip` (lossy drop), "create
C_STRING from C_DATE_TIME / C_TERMINOLOGY_CODE" (micro-parser / terminology
transform), "C_TERMINOLOGY to C_STRING".

Key mappings (complete tables in the spec, representative here):
- `OBSERVATION.data/state` collapse; `data.events → data`, `state.events →
  state`; `data.origin/period/duration/summary → history_*`;
  `state.* → state_*`.
- `ACTION.ism_transition` + `instruction_details` collapse to promoted
  fields; `INSTRUCTION.narrative`/`expiry_time` → strings.
- `EVENT.data/state` (ITEM_TREE) collapse to `List<S_ITEM>`.
- `PARTY_PROXY.external_ref.{id.value, namespace, id.scheme}` →
  `id`/`id_namespace`/`id_scheme` (scheme guarded on GENERIC_ID).
- `DV_TEXT.formatting/language/encoding` → skip.
- `DV_CODED_TEXT.defining_code.{code_string, terminology_id}` →
  `code`/`terminology`; RM-vocabulary coded fields → plain String.
- `OBJECT_REF.namespace/type` → `id_namespace`/`id_type`.

In-source TODO: rules representation may be formalised into BMM or a formal
language; today they are tables interspersed with class definitions.

**No S_* class declares any invariant.** The only formal constraints are
these rules + the APP_CONTEXT To/From-RM notes.

## 6. Gaps & defects (design consequences)

1. DEVELOPMENT/unreleased (0.7.0); `to_rm`/`from_rm` bodies empty; approach
   itself may change.
2. `S_DV_QUANTITY`/`COUNT`/`PROPORTION`/`ORDINAL` literally `TODO: define`;
   `S_DV_BOOLEAN`/`S_OBJECT_ID`/`T` stubs.
3. Spec errors: `S_COMPOSITION.language` meaning cell; `content` typed
   single not list.
4. Deliberately lossy: DV_TEXT presentation fields, FEEDER_AUDIT_DETAILS
   (4 attrs dropped), PARTY_IDENTIFIED.identifiers, coded→string collapses.
5. Serial-form divergence inside SM (space-quantity vs comma-quantity vs
   Better `|magnitude`) — Better remains our interop target;
   SIM-B/APP_CONTEXT is the *conceptual* reference for `ctx/` defaults and
   the collapse rules our WebTemplate/FLAT converters implement.

## 7. Mapping note (current code)

`openehr-flat` (P14) already implements the Better semantics that SIM-B
codifies: FLAT/STRUCTURED converters, `ctx/` defaults, collapse rules via
the WebTemplate. SIM-B gives us: (a) the normative names for the `ctx`
vocabulary, (b) the transformation-rule tables to audit our converters
against, (c) the documented-lossy list (what FLAT legitimately cannot round-
trip). P17 should include an audit of `openehr-flat` against §4.6 + §5.
