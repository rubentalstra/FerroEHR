# A1 spec audit — Phase 1 (Extract) — chapter `sm-tdd`

- **Chapter:** sm-tdd (SM Simplified IM-B / Serial Data Formats / TDD transformation duties)
- **Date:** 2026-07-11
- **Scope note (blueprint row 17):** SIM-B / SDF is scheduled P17 and is **NOT
  conformance-gated** — it is interop-quality. Per the chapter note, pure
  interop gaps are severity-capped at low/minor **unless they corrupt stored
  data** (silent misparse of a clinical leaf value), which are exempt from the
  cap and rated high. Much of the SIM-B class layer is an explicit draft
  (`TODO: define` on `S_DV_QUANTITY`, `S_DV_ORDINAL`, `S_DV_PROPORTION`,
  `S_DV_COUNT`, `S_OBJECT_ID`); the authoritative, checkable encodings live in
  the SDT serial-format sections (SIM-B master04) and the SDF leaf tables (SDF
  master03), plus the app-context class definitions.

## Spec files read

- `SM/docs/simplified_im_b/master03-sim_base.adoc` (S_TYPE, S_OBJECT_REF/ID/GENERIC_ID includes)
- `SM/docs/simplified_im_b/master04-sim_data_types.adoc` (SDT serial forms for S_DV_* + class includes)
- `SM/docs/simplified_im_b/master05-sim_structures.adoc` (structure/common/composition class includes)
- `SM/docs/simplified_im_b/master06-app_context.adoc` (APP_CONTEXT, APP_COMPOSITION)
- `SM/docs/simplified_im_b/master07-transformation_rules.adoc` (SIM↔RM transformation rules)
- `SM/docs/serial_data_formats/master03-data_values.adoc` (SDF leaf encodings, EhrScape variants)
- `SM/docs/serial_data_formats/master04-syntax.adoc` (JSON syntax / string parser — largely TBD)
- Included UML class defs under `SM/docs/UML/classes/` (app_context, app_composition, s_composition, s_locatable, s_dv_text, s_dv_coded_text, s_code_phrase, s_dv_parsable, s_object_ref/id, s_generic_id, s_data_value, s_type)

## Requirements

| id | requirement | citation | category | risk |
|---|---|---|---|---|
| sm-tdd-R1 | An `S_DV_TEXT` leaf must be accepted in all four SDT forms: path+terse (`"a/b/c/d": "anxiety"`), path+regular (`"a/b/c/d\|value": "anxiety"`), nested-regular (`{"\|value": "anxiety"}`), nested-terse (`"d": "anxiety"`). | SIM-B master04 §Serial Formats / S_DV_TEXT | serialization | low |
| sm-tdd-R2 | An `S_DV_CODED_TEXT` terse leaf `"<terminology>::<code>\|<value>\|"` must be parsed into `terminology`, `code`, and `value` — misparsing the `::` / `\|…\|` delimiters silently stores a wrong coded value. | SIM-B master04 §S_DV_CODED_TEXT (path+terse form `"snomed_ct::48694002\|anxiety\|"`) | serialization | high |
| sm-tdd-R3 | An `S_DV_CODED_TEXT` regular leaf uses the suffixed keys `\|terminology`, `\|code`, `\|value`; all three carry the coded-text fields. | SIM-B master04 §S_DV_CODED_TEXT (path+regular form) | serialization | low |
| sm-tdd-R4 | An `S_DV_QUANTITY` terse leaf is **space-separated** `"<magnitude> <units>"` (e.g. `"125 mm[Hg]"`) — this diverges from the SDF `DV_QUANTITY` comma form (R34); a parser must not confuse the two or it corrupts magnitude/units. | SIM-B master04 §S_DV_QUANTITY (terse `"125 mm[Hg]"`) | serialization | high |
| sm-tdd-R5 | An `S_DV_QUANTITY` regular leaf uses keys `\|magnitude` (JSON number) and `\|units` (string), plus any further DvQuantity fields. | SIM-B master04 §S_DV_QUANTITY (path+regular form) | serialization | low |
| sm-tdd-R6 | An `S_DV_PARSABLE` leaf carries both `\|formalism` and `\|value` (regular form) or `formalism`/`value` (nested); both are mandatory per the class (`S_DV_PARSABLE.value 1..1`, `formalism 1..1`). | SIM-B master04 §S_DV_PARSABLE + UML `s_dv_parsable` | mandatory-attr | medium |
| sm-tdd-R7 | `APP_CONTEXT.language` is mandatory (`1..1`, ISO639-1 code) — maps to `COMPOSITION._language_`. | UML `app_context` (language, `1..1`) | mandatory-attr | medium |
| sm-tdd-R8 | `APP_CONTEXT.territory` is mandatory (`1..1`, ISO3166-1 code) — maps to `COMPOSITION._territory_`. | UML `app_context` (territory, `1..1`) | mandatory-attr | medium |
| sm-tdd-R9 | `APP_CONTEXT.composer_name` is mandatory (`1..1`) — maps to `COMPOSITION._composer.name_` for a `PARTY_IDENTIFIED` composer. | UML `app_context` (composer_name, `1..1`) | mandatory-attr | medium |
| sm-tdd-R10 | `APP_CONTEXT.time` is declared `1..1` but defaults to current time when absent; when supplied it is converted from `DV_DATE_TIME._value_` and used for context start-time / HISTORY.origin / event time defaults. | UML `app_context` (time, "If not specified current time will be used") | behaviour | low |
| sm-tdd-R11 | `APP_CONTEXT.category` value must be one of `"event"` or `"persistent"` (COMPOSITION category vocabulary); other values must be rejected / not silently coerced. | UML `app_context` (category: "event" or "persistent") | rejection-duty | medium |
| sm-tdd-R12 | `APP_CONTEXT.action_ism_transition_current_state` is typed `Integer` (not string) — a non-integer must be rejected. | UML `app_context` (action_ism_transition_current_state: Integer) | mandatory-attr | low |
| sm-tdd-R13 | `APP_CONTEXT.healthcare_facility` is a monomorphic slot typed `S_PARTY_IDENTIFIED` — must not accept a foreign `_type`. | UML `app_context` (healthcare_facility: S_PARTY_IDENTIFIED) | mandatory-attr | low |
| sm-tdd-R14 | `APP_CONTEXT.workflow_id` is a monomorphic slot typed `S_OBJECT_REF`. | UML `app_context` (workflow_id: S_OBJECT_REF) | mandatory-attr | low |
| sm-tdd-R15 | `APP_COMPOSITION` inherits `S_COMPOSITION` and adds `ctx` (`0..1`) typed `APP_CONTEXT` — the ctx slot is monomorphic. | UML `app_composition` (Inherit S_COMPOSITION; ctx: APP_CONTEXT, `0..1`) | mandatory-attr | low |
| sm-tdd-R16 | `S_COMPOSITION.content` is mandatory (`1..1`, typed `S_CONTENT_ITEM`); a converted COMPOSITION must have content. | UML `s_composition` (content, `1..1`) | mandatory-attr | medium |
| sm-tdd-R17 | `S_COMPOSITION.composer` is mandatory (`1..1`, typed `S_PARTY_PROXY`); converted from PARTY_SELF/PARTY_IDENTIFIED/PARTY_RELATED. | UML `s_composition` (composer, `1..1`) | mandatory-attr | medium |
| sm-tdd-R18 | `S_COMPOSITION.language` is mandatory (`1..1`). | UML `s_composition` (language, `1..1`) | mandatory-attr | medium |
| sm-tdd-R19 | `S_COMPOSITION.territory` is mandatory (`1..1`). | UML `s_composition` (territory, `1..1`) | mandatory-attr | medium |
| sm-tdd-R20 | `S_LOCATABLE.name` is mandatory (`1..1`, typed `S_DV_TEXT`) — every simplified locatable node carries a name. | UML `s_locatable` (name, `1..1`) | mandatory-attr | medium |
| sm-tdd-R21 | `S_DV_TEXT.value` is mandatory (`1..1`). | UML `s_dv_text` (value, `1..1`) | mandatory-attr | medium |
| sm-tdd-R22 | `S_DV_CODED_TEXT.code` and `.terminology` are both mandatory (`1..1`), from `DV_CODED_TEXT._defining_code_`. | UML `s_dv_coded_text` (code `1..1`, terminology `1..1`) | mandatory-attr | medium |
| sm-tdd-R23 | `S_CODE_PHRASE.code` and `.terminology` are both mandatory (`1..1`), from `CODE_PHRASE`. | UML `s_code_phrase` (code `1..1`, terminology `1..1`) | mandatory-attr | medium |
| sm-tdd-R24 | `S_OBJECT_REF.id` is mandatory (`1..1`, typed `S_OBJECT_ID`); `id_namespace` and `id_type` are optional (`0..1`). | UML `s_object_ref` | mandatory-attr | low |
| sm-tdd-R25 | `S_GENERIC_ID.scheme` is mandatory (`1..1`); S_GENERIC_ID inherits S_OBJECT_ID. | UML `s_generic_id` (scheme, `1..1`) | mandatory-attr | low |
| sm-tdd-R26 | `DV_BOOLEAN` is encoded as a native JSON `Boolean` (`true`/`false`). | SDF master03 §RM DATA_VALUE Types (DV_BOOLEAN → Boolean) | serialization | low |
| sm-tdd-R27 | `DV_COUNT` is encoded as a native JSON `Integer` (not a quoted string). | SDF master03 §RM DATA_VALUE Types (DV_COUNT → Integer) | serialization | medium |
| sm-tdd-R28 | `CODE_PHRASE` is encoded as a `Terminology_code` bracketed ODIN TERM_CODE_REF `"[<terminology_id>::<code>]"`. | SDF master03 §RM DATA_VALUE Types (CODE_PHRASE → Terminology_code) + §openEHR Primitives | serialization | medium |
| sm-tdd-R29 | `DV_TEXT` = `String`; `DV_CODED_TEXT` = `Terminology_term` `"[<terminology_id>::<code>\|<text>\|]"`; the bracket/`::`/`\|…\|` structure must parse into terminology, code and text or the coded value is corrupted. | SDF master03 §RM DATA_VALUE Types + §openEHR Primitives (Terminology_term) | serialization | high |
| sm-tdd-R30 | ISO 8601 leaf types (`DV_DATE`/`DV_TIME`/`DV_DATE_TIME`/`DV_DURATION`) are validated per `TIME_DEFINITIONS.valid_iso8601_*`, including partial forms; a value failing that validity function must be rejected (a bad date/time otherwise corrupts stored data). | SDF master03 §openEHR Primitives Represented as JSON String (Iso8601_date/time/date_time/duration → TIME_DEFINITIONS.valid_iso8601_*) | validity-fn | high |
| sm-tdd-R31 | `DV_EHR_URI` is encoded as a `Uri` (RFC 3986 string). | SDF master03 §RM DATA_VALUE Types (DV_EHR_URI → Uri) + §openEHR Primitives (Uri → RFC3986) | serialization | low |
| sm-tdd-R32 | `DV_ORDINAL` leaf = `"<ordinal_value>\|<terminology_code>"` or `"<ordinal_value>\|<terminology_term>"` (e.g. `"1\|[snomed_ct::313267000\|Stroke\|]"`); the leading integer value and the trailing coded symbol must both be parsed or the ordinal is corrupted. | SDF master03 §DATA_VALUE types with specific SDF syntax (DV_ORDINAL) | serialization | high |
| sm-tdd-R33 | `DV_SCALE` leaf = `"<scale_value>\|<terminology_code>"` or `"…\|<terminology_term>"` (e.g. `"1.5\|[snomed_ct::127840596\|minor difficulty\|]"`); scale_value is a real. | SDF master03 §DATA_VALUE types with specific SDF syntax (DV_SCALE) | serialization | medium |
| sm-tdd-R34 | `DV_QUANTITY` (SDF form) leaf = **comma-separated** `"<value>,<unit>"` (e.g. `"78.500,kg"`) — distinct from the SIM-B SDT space-separated form (R4); misapplying the wrong delimiter corrupts magnitude/units. | SDF master03 §DATA_VALUE types with specific SDF syntax (DV_QUANTITY `"78.500,kg"`) | serialization | high |
| sm-tdd-R35 | `DV_PROPORTION` leaf = `"<numerator>/<denominator>;<proportion_kind>"` where `proportion_kind ∈ {RATIO, UNITARY, PERCENT, FRACTION, INTEGER_FRACTION}`; an out-of-whitelist kind must be rejected. | SDF master03 §DATA_VALUE types with specific SDF syntax (DV_PROPORTION) | rejection-duty | high |
| sm-tdd-R36 | `DV_MULTIMEDIA` is a JSON object with fields `integrityCheckAlgorithm`, `mediaType`, `compressionAlgorithm`, `uri`. | SDF master03 §DATA_VALUE types (DV_MULTIMEDIA JSON) | serialization | low |
| sm-tdd-R37 | `DV_IDENTIFIER` is a JSON object with fields `id`, `issuer`, `assigner`, `type`. | SDF master03 §DATA_VALUE types (DV_IDENTIFIER JSON) | serialization | low |
| sm-tdd-R38 | `Terminology_code` / `Terminology_term` support an optional version id `"[<terminology_id>(<version_id>)::<code>]"`; the version-id parenthetical must be parsed (or accepted-and-noted) not treated as part of the code. | SDF master03 §openEHR Primitives (Terminology_code / Terminology_term ODIN forms) | serialization | medium |
| sm-tdd-R39 | `Interval<T:Ordered>` leaves are ODIN interval strings with all bound forms: `\|N .. M\|`, `\|> N .. M\|`, `\|N .. <M\|`, `\|> N .. <M\|`, `\|< N\|`, `\|> N\|`, `\|>= N\|`, `\|<= N\|`, `\|N +/-M\|`, `\|N±M\|`. | SDF master03 §openEHR Intervals Represented as JSON String | serialization | medium |
| sm-tdd-R40 | Lists of primitive/interval values are encoded as standard JSON arrays of the per-element encodings. | SDF master03 §Lists of Primitive Type and Intervals | serialization | low |
| sm-tdd-R41 | EhrScape variant of `Terminology_code` = object `{"\|code":…, "\|terminology":…}`; of `Terminology_term` = `{"\|code":…, "\|value":…, "\|terminology":…}` — supported only under EhrScape mode. | SDF master03 §openEHR Primitives / EhrScape Variants | serialization | low |
| sm-tdd-R42 | EhrScape variants exist for `DV_PARSABLE` (`{"\|value","\|formalism"}`), `DV_MULTIMEDIA` (`{"\|integrityCheckAlgorithm","\|mediaType","\|compressionAlgorithm","\|uri"}`) and `DV_IDENTIFIER` (`{"\|id","\|issuer","\|assigner","\|type"}`). | SDF master03 §DATA_VALUE Types / EhrScape Variants | serialization | low |
| sm-tdd-R43 | On transformation, `DV_TEXT._formatting_`, `._language_`, `._encoding_` are **skipped** (not carried into the simplified form). | SIM-B master07 §RM Data types Package (DV_TEXT formatting/language/encoding → skip) | behaviour | low |
| sm-tdd-R44 | `S_OBSERVATION` collapses the RM `HISTORY`: `data.events`→`data`, `state.events`→`state`, `data.origin`→`history_origin`, `data.period`→`history_period`, `data.duration`→`history_duration`, `data.summary`→`history_summary` (and `state.*` analogues); the reverse (to_rm) must reconstruct the HISTORY wrapper. | SIM-B master07 §Composition Package (OBSERVATION `collapse()` rows) | behaviour | medium |
| sm-tdd-R45 | Temporal leaves in transformation — `EVENT._time_`, `ACTION._time_`, `INSTRUCTION._expiry_time_` — are created as a `C_STRING` from `C_DATE_TIME` (i.e. rendered as the ISO 8601 string leaf form, R30). | SIM-B master07 §Data Structures / Composition Packages (create C_STRING from C_DATE_TIME) | behaviour | medium |
| sm-tdd-R46 | Coded transformation leaves — `INSTRUCTION._narrative_`, `INTERVAL_EVENT._math_function_`, `ACTION` careflow codes — are created as `C_STRING` from the `C_TERMINOLOGY_CODE` at `_defining_code_` (coded-text/code encoding, R28/R29). | SIM-B master07 §Composition / Data Structures Packages (create C_STRING from C_TERMINOLOGY_CODE) | behaviour | medium |
| sm-tdd-R47 | `S_PARTY_PROXY` maps `external_ref.id.value`→`id`, `external_ref.namespace`→`id_namespace`, and (only when `external_ref.id` is a `GENERIC_ID`) `external_ref.id.scheme`→`id_scheme`; and `S_OBJECT_REF` maps `namespace`→`id_namespace`, `type`→`id_type`. | SIM-B master07 §Common Package + §Simplified IM Package | behaviour | low |
