# A1 Spec Audit — Phase 1 (Extract) — chapter `rm-data-types-rest`

- **Date:** 2026-07-11
- **Component:** openEHR RM 1.2.0 — Data Types (basic, date_time, time_specification, encapsulated, uri packages)
- **Spec files read (relative to `docs/specs/openehr/`):**
  - `RM/docs/data_types/master04-basic_package.adoc` (DV_BOOLEAN / DV_STATE / DV_IDENTIFIER + DATA_VALUE)
  - `RM/docs/data_types/master07-date_time_package.adoc` (DV_TEMPORAL / DV_DATE / DV_TIME / DV_DATE_TIME / DV_DURATION incl. partial-precision + magnitude semantics)
  - `RM/docs/data_types/master08-time_specification_package.adoc` (DV_TIME_SPECIFICATION / DV_PERIODIC_TIME_SPECIFICATION / DV_GENERAL_TIME_SPECIFICATION)
  - `RM/docs/data_types/master09-encapsulated_package.adoc` (DV_ENCAPSULATED / DV_MULTIMEDIA / DV_PARSABLE)
  - `RM/docs/data_types/master10-uri_package.adoc` (DV_URI / DV_EHR_URI)
  - Class detail: `RM/docs/UML/classes/org.openehr.rm.data_types.*.adoc` (data_value, dv_boolean, dv_state, dv_identifier, dv_temporal, dv_absolute_quantity, dv_quantified, dv_amount, dv_date, dv_time, dv_date_time, dv_duration, dv_encapsulated, dv_multimedia, dv_parsable, dv_uri, dv_ehr_uri, dv_time_specification, dv_periodic_time_specification, dv_general_time_specification)

No listed file was missing; no path correction required.

**Note on inheritance:** the date/time value types inherit through
`DV_TEMPORAL → DV_ABSOLUTE_QUANTITY → DV_QUANTIFIED → DV_ORDERED`, and
`DV_DURATION` inherits `DV_AMOUNT → DV_QUANTIFIED`. The invariants of
`DV_QUANTIFIED` (magnitude_status) and `DV_AMOUNT` (accuracy) therefore apply
transitively and are extracted below because they are machine-checkable on the
concrete leaf types this chapter covers.

---

## Requirements

| ID | Requirement | Citation | Category | Risk |
|----|-------------|----------|----------|------|
| rm-data-types-rest-R1 | `DATA_VALUE` is abstract — no instance may be serialized/persisted with `_type` = `DATA_VALUE`; every value must resolve to a concrete `DV_` subtype. | `UML/classes/…data_value.adoc` L6 (`DATA_VALUE (abstract)`) | rejection-duty | medium |
| rm-data-types-rest-R2 | `DV_BOOLEAN.value` is mandatory (1..1) and of type `Boolean`; absent or non-boolean value must be rejected. | `UML/classes/…dv_boolean.adoc` L20-21 | mandatory-attr | medium |
| rm-data-types-rest-R3 | `DV_STATE.value` is mandatory (1..1) and is a **monomorphic slot typed `DV_CODED_TEXT`** — must reject any foreign `_type` (e.g. plain `DV_TEXT`) in this slot. | `UML/classes/…dv_state.adoc` L20-22 | rejection-duty | high |
| rm-data-types-rest-R4 | `DV_STATE.is_terminal` is mandatory (1..1) of type `Boolean`. | `UML/classes/…dv_state.adoc` L24-26 | mandatory-attr | medium |
| rm-data-types-rest-R5 | `DV_IDENTIFIER.id` is mandatory (1..1) of type `String`; the other three fields (`issuer`, `assigner`, `type`) are optional (0..1). | `UML/classes/…dv_identifier.adoc` L22-36 | mandatory-attr | medium |
| rm-data-types-rest-R6 | `DV_IDENTIFIER` invariant `Id_valid`: `not id.is_empty` — must reject an empty `id`. | `UML/classes/…dv_identifier.adoc` L38-39 | invariant | high |
| rm-data-types-rest-R7 | `DV_DATE.value` is mandatory (1..1) `String`; invariant `Value_valid`: `valid_iso8601_date(value)` — must reject a value that is not a valid ISO 8601 date. | `UML/classes/…dv_date.adoc` L18-20, L78-79; `master07…` L142-149 | invariant | high |
| rm-data-types-rest-R8 | `DV_DATE` must accept ISO 8601 **reduced-accuracy (partial)** dates: year-only (`YYYY`) and year-month (`YYYY-MM`); day may be absent from the right-hand end. | `master07…` L30-40, L95-98; `UML/classes/…dv_date.adoc` L9 | behaviour | medium |
| rm-data-types-rest-R9 | Partial-date rule: parts may only be missing from the **right-hand end** — a date with month unknown but day known must be rejected (e.g. no `YYYY--DD`). | `master07…` L96-98 | rejection-duty | high |
| rm-data-types-rest-R10 | `DV_DATE.magnitude()` = days since calendar origin `0001-01-01`; comparison (`less_than`) is defined as `magnitude < other.magnitude`. | `UML/classes/…dv_date.adoc` L28-29, L60-69 | behaviour | low |
| rm-data-types-rest-R11 | `DV_TIME.value` is mandatory (1..1) `String`; invariant `Value_valid`: `valid_iso8601_time(value)` — must reject a value that is not a valid ISO 8601 time. | `UML/classes/…dv_time.adoc` L20-22, L73-74 | invariant | high |
| rm-data-types-rest-R12 | `DV_TIME` must accept partial times per ISO 8601: either hours only (`hh`) or hours+minutes (`hh:mm`) present, with seconds absent from the right; and may carry a timezone. | `master07…` L21, L41-45, L95-98 | behaviour | medium |
| rm-data-types-rest-R13 | `DV_TIME.magnitude()` = seconds since start of day `00:00:00` (Real). | `UML/classes/…dv_time.adoc` L30-32 | behaviour | low |
| rm-data-types-rest-R14 | `DV_DATE_TIME.value` is mandatory (1..1) `String`; invariant `Value_valid`: `valid_iso8601_date_time(value)` — must reject a value that is not a valid ISO 8601 date-time. | `UML/classes/…dv_date_time.adoc` L20-22, L73-74 | invariant | high |
| rm-data-types-rest-R15 | `DV_DATE_TIME.magnitude()` = seconds since origin `0001-01-01T00:00:00Z` (Double); may carry timezone. | `UML/classes/…dv_date_time.adoc` L30-32; `master07…` L22, L67-69 | behaviour | low |
| rm-data-types-rest-R16 | `DV_DURATION.value` is mandatory (1..1) `String`; invariant `Value_valid`: `valid_iso8601_duration(value)` — must reject a value that is not a valid ISO 8601 duration (with the two allowed deviations). | `UML/classes/…dv_duration.adoc` L24-27, L84-85 | invariant | high |
| rm-data-types-rest-R17 | `DV_DURATION` deviation 1: a leading negative sign is allowed (e.g. `-P3M`) — must accept negative durations. | `master07…` L75-80; `UML/classes/…dv_duration.adoc` L10-11, L27 | behaviour | medium |
| rm-data-types-rest-R18 | `DV_DURATION` deviation 2: the `W` (week) designator may be mixed with other designators — must accept e.g. `P1W2D` even though strict ISO 8601 forbids it. | `master07…` L78; `UML/classes/…dv_duration.adoc` L10-11 | behaviour | medium |
| rm-data-types-rest-R19 | `DV_DURATION.magnitude()` = number of seconds (Double), computed from `to_seconds()`; comparison via `magnitude`. | `UML/classes/…dv_duration.adoc` L79-82, L54-63 | behaviour | low |
| rm-data-types-rest-R20 | `DV_TEMPORAL.accuracy` is a **monomorphic slot typed `DV_DURATION`** (redefined 0..1) — must reject a non-`DV_DURATION` `_type` in `accuracy` on any date/time value type. | `UML/classes/…dv_temporal.adoc` L18-21 | rejection-duty | medium |
| rm-data-types-rest-R21 | Inherited `DV_QUANTIFIED.magnitude_status` (0..1 `String`) invariant `Magnitude_status_valid`: if present it must be one of `{"=", "<", ">", "<=", ">=", "~"}`; reject any other value. Applies to DV_DATE/DV_TIME/DV_DATE_TIME/DV_DURATION. | `UML/classes/…dv_quantified.adoc` L18-29, L39-43, L72-73 | invariant | high |
| rm-data-types-rest-R22 | Inherited `DV_AMOUNT` invariant `Accuracy_is_percent_validity`: `accuracy = 0 implies not accuracy_is_percent` — reject `accuracy_is_percent = True` when `accuracy = 0`. Applies to `DV_DURATION`. | `UML/classes/…dv_amount.adoc` L93-94 | invariant | medium |
| rm-data-types-rest-R23 | Inherited `DV_AMOUNT` invariant `Accuracy_validity`: `accuracy_is_percent implies valid_percentage(accuracy)` i.e. accuracy between 0 and 100 — reject a percent accuracy outside 0..100. Applies to `DV_DURATION`. | `UML/classes/…dv_amount.adoc` L33-37, L95-97 | invariant | medium |
| rm-data-types-rest-R24 | `DV_ENCAPSULATED` invariant `Charset_valid`: `charset /= Void implies code_set(Code_set_id_character_sets).has_code(charset)` — reject a `charset` code not in the openEHR character-sets code set. | `UML/classes/…dv_encapsulated.adoc` L18-20, L29-30 | invariant | high |
| rm-data-types-rest-R25 | `DV_ENCAPSULATED` invariant `Language_valid`: `language /= Void implies code_set(Code_set_id_languages).has_code(language)` — reject a `language` code not in the openEHR languages code set. | `UML/classes/…dv_encapsulated.adoc` L22-24, L26-27 | invariant | high |
| rm-data-types-rest-R26 | `DV_ENCAPSULATED.charset` and `.language` are each a **monomorphic slot typed `CODE_PHRASE`** — must reject a foreign `_type` in either slot. | `UML/classes/…dv_encapsulated.adoc` L18-24 | rejection-duty | medium |
| rm-data-types-rest-R27 | `DV_MULTIMEDIA.media_type` is mandatory (1..1) `CODE_PHRASE`; invariant `Media_type_valid`: `media_type /= Void and then code_set(Code_set_id_media_types).has_code(media_type)` — reject an absent media_type or a code not in the openEHR media-types code set. | `UML/classes/…dv_multimedia.adoc` L30-32, L77-78 | invariant | high |
| rm-data-types-rest-R28 | `DV_MULTIMEDIA.size` is mandatory (1..1) `Integer`; invariant `Size_valid`: `size >= 0` — reject a negative size. | `UML/classes/…dv_multimedia.adoc` L50-52, L89-90 | invariant | high |
| rm-data-types-rest-R29 | `DV_MULTIMEDIA` invariant `Not_empty`: `is_inline or is_external` — reject a DV_MULTIMEDIA that has neither inline `data` nor a `uri`. | `UML/classes/…dv_multimedia.adoc` L74-75, L58-64 | invariant | high |
| rm-data-types-rest-R30 | `DV_MULTIMEDIA` invariant `Integrity_check_validity`: `integrity_check /= Void implies integrity_check_algorithm /= Void` — reject an `integrity_check` present without an `integrity_check_algorithm`. | `UML/classes/…dv_multimedia.adoc` L83-84 | invariant | high |
| rm-data-types-rest-R31 | `DV_MULTIMEDIA` invariant `Integrity_check_algorithm_validity`: `integrity_check_algorithm /= Void implies code_set(Code_set_id_integrity_check_algorithms).has_code(integrity_check_algorithm)` — reject an integrity-check-algorithm code not in the openEHR integrity-check code set. | `UML/classes/…dv_multimedia.adoc` L85-87 | invariant | high |
| rm-data-types-rest-R32 | `DV_MULTIMEDIA` invariant `Compression_algorithm_validity`: `compression_algorithm /= Void implies code_set(Code_set_id_compression_algorithms).has_code(compression_algorithm)` — reject a compression-algorithm code not in the openEHR code set. | `UML/classes/…dv_multimedia.adoc` L80-81 | invariant | high |
| rm-data-types-rest-R33 | `DV_MULTIMEDIA.uri` (0..1) is a **monomorphic slot typed `DV_URI`**, and `.thumbnail` (0..1) is a monomorphic slot typed `DV_MULTIMEDIA` (recursion) — reject a foreign `_type` in either slot. | `UML/classes/…dv_multimedia.adoc` L22-24, L46-48 | rejection-duty | medium |
| rm-data-types-rest-R34 | `DV_MULTIMEDIA.media_type`, `.compression_algorithm`, `.integrity_check_algorithm` are each typed `CODE_PHRASE` (monomorphic); `data` and `integrity_check` are `Array<Octet>` (byte arrays). | `UML/classes/…dv_multimedia.adoc` L26-52 | mandatory-attr | low |
| rm-data-types-rest-R35 | `DV_PARSABLE.value` is mandatory (1..1) `String` (may validly be empty in some syntaxes). | `UML/classes/…dv_parsable.adoc` L18-20 | mandatory-attr | medium |
| rm-data-types-rest-R36 | `DV_PARSABLE.formalism` is mandatory (1..1) `String`; invariant `Formalism_valid`: `not formalism.is_empty` — reject an empty `formalism`. | `UML/classes/…dv_parsable.adoc` L22-24, L34-35 | invariant | high |
| rm-data-types-rest-R37 | `DV_PARSABLE` invariant `Size_valid`: `size >= 0` (size is bytes of `value`). | `UML/classes/…dv_parsable.adoc` L30-32, L37-38 | invariant | low |
| rm-data-types-rest-R38 | `DV_URI.value` is mandatory (1..1) `String`; invariant `Value_valid`: `not value.is_empty` — reject an empty URI value. | `UML/classes/…dv_uri.adoc` L18-20, L42-43 | invariant | high |
| rm-data-types-rest-R39 | `DV_EHR_URI` invariant `Scheme_valid`: `scheme.is_equal(Ehr_scheme)` where `Ehr_scheme = "ehr"` — must reject a DV_EHR_URI whose URI scheme is not `ehr`. | `UML/classes/…dv_ehr_uri.adoc` L17-18; `master10…` L42-43 | invariant | high |
| rm-data-types-rest-R40 | `DV_TIME_SPECIFICATION.value` is mandatory (1..1) and is a **monomorphic slot typed `DV_PARSABLE`** — reject a foreign `_type` in this slot. | `UML/classes/…dv_time_specification.adoc` L18-19 | rejection-duty | medium |
| rm-data-types-rest-R41 | `DV_PERIODIC_TIME_SPECIFICATION` invariant `Value_valid`: `value.formalism.is_equal("HL7:PIVL") or value.formalism.is_equal("HL7:EIVL")` — reject a periodic time spec whose inner `DV_PARSABLE.formalism` is not `HL7:PIVL` or `HL7:EIVL`. | `UML/classes/…dv_periodic_time_specification.adoc` L39-40 | invariant | high |
| rm-data-types-rest-R42 | `DV_TIME_SPECIFICATION` is abstract — instances must resolve to `DV_PERIODIC_TIME_SPECIFICATION` or `DV_GENERAL_TIME_SPECIFICATION`; reject `_type` = `DV_TIME_SPECIFICATION`. | `UML/classes/…dv_time_specification.adoc` L6 | rejection-duty | low |
| rm-data-types-rest-R43 | `DV_ENCAPSULATED` is abstract — instances must resolve to `DV_MULTIMEDIA` or `DV_PARSABLE`; reject `_type` = `DV_ENCAPSULATED`. | `master09…` L53-54; `UML/classes/…dv_encapsulated.adoc` L6 | rejection-duty | low |
| rm-data-types-rest-R44 | `DV_MULTIMEDIA.size` records the original unencoded byte count — encodings such as base64/hex do **not** change `size` (serialization semantics). | `UML/classes/…dv_multimedia.adoc` L50-52 | serialization | low |
| rm-data-types-rest-R45 | `DV_DURATION` may not be used to represent points in time or intervals of time (a duration is relative to an unstated origin). | `UML/classes/…dv_duration.adoc` L15; `master07…` L23, L71-73 | behaviour | low |
