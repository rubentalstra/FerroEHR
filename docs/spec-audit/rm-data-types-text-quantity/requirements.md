# Spec-audit requirements — rm-data-types-text-quantity

- **Chapter:** rm-data-types-text-quantity
- **Date:** 2026-07-11
- **Phase:** A1 spec audit, Phase 1 (Extract)
- **Component:** RM 1.2.0 (pinned), openEHR Reference Model — `data_types.text` + `data_types.quantity`

## Spec files read (all under `docs/specs/openehr/`)

- `RM/docs/data_types/master05-text_package.adoc`
- `RM/docs/data_types/master06-quantity_package.adoc`
- `RM/docs/UML/classes/org.openehr.rm.data_types.dv_text.adoc`
- `RM/docs/UML/classes/org.openehr.rm.data_types.dv_coded_text.adoc`
- `RM/docs/UML/classes/org.openehr.rm.data_types.code_phrase.adoc`
- `RM/docs/UML/classes/org.openehr.rm.data_types.term_mapping.adoc`
- `RM/docs/UML/classes/org.openehr.rm.data_types.dv_ordered.adoc`
- `RM/docs/UML/classes/org.openehr.rm.data_types.dv_interval.adoc`
- `RM/docs/UML/classes/org.openehr.rm.data_types.reference_range.adoc`
- `RM/docs/UML/classes/org.openehr.rm.data_types.dv_ordinal.adoc`
- `RM/docs/UML/classes/org.openehr.rm.data_types.dv_scale.adoc`
- `RM/docs/UML/classes/org.openehr.rm.data_types.dv_quantified.adoc`
- `RM/docs/UML/classes/org.openehr.rm.data_types.dv_amount.adoc`
- `RM/docs/UML/classes/org.openehr.rm.data_types.dv_quantity.adoc`
- `RM/docs/UML/classes/org.openehr.rm.data_types.dv_count.adoc`
- `RM/docs/UML/classes/org.openehr.rm.data_types.dv_proportion.adoc`
- `RM/docs/UML/classes/org.openehr.rm.data_types.proportion_kind.adoc`
- `RM/docs/UML/classes/org.openehr.rm.data_types.dv_absolute_quantity.adoc`

No listed file was missing. `dv_absolute_quantity.adoc` was substituted for the
non-existent `dv_amount`/`dv_quantified` accuracy detail names — all present under
the classes directory.

## Requirements

| id | requirement | citation | category | risk |
|---|---|---|---|---|
| rm-data-types-text-quantity-R1 | `DV_TEXT.value` is mandatory (1..1, `String`); a `DV_TEXT`/`DV_CODED_TEXT` payload with no `value` must be rejected. | dv_text.adoc "Attributes" `*1..1* value: String` | mandatory-attr | high |
| rm-data-types-text-quantity-R2 | `DV_TEXT` invariant `Valid_value: not value.is_empty` — must reject an empty-string `value`. | dv_text.adoc Invariants `Valid_value` | rejection-duty | high |
| rm-data-types-text-quantity-R3 | `DV_TEXT` invariant `Mappings_valid: mappings /= void implies not mappings.is_empty` — if a `mappings` list is present it must be non-empty (reject empty list). | dv_text.adoc Invariants `Mappings_valid` | rejection-duty | high |
| rm-data-types-text-quantity-R4 | `DV_TEXT` invariant `Formatting_valid: formatting /= void implies not formatting.is_empty` — reject a present-but-empty `formatting` string. | dv_text.adoc Invariants `Formatting_valid` | rejection-duty | high |
| rm-data-types-text-quantity-R5 | `DV_TEXT` invariant `Language_valid: language /= Void implies code_set(Code_set_id_languages).has_code(language)` — a present `language` CODE_PHRASE must be a code in the openEHR languages code set. | dv_text.adoc Invariants `Language_valid` | validity-fn | medium |
| rm-data-types-text-quantity-R6 | `DV_TEXT` invariant `Encoding_valid: encoding /= Void implies code_set(Code_set_id_character_sets).has_code(encoding)` — a present `encoding` CODE_PHRASE must be a code in the character-sets code set. | dv_text.adoc Invariants `Encoding_valid` | validity-fn | medium |
| rm-data-types-text-quantity-R7 | `DV_TEXT.language` and `DV_TEXT.encoding` are each optional (0..1) and typed `CODE_PHRASE` (monomorphic slot, no subtypes) — a foreign `_type` in these slots must be rejected. | dv_text.adoc Attributes `language`/`encoding`: CODE_PHRASE | rejection-duty | medium |
| rm-data-types-text-quantity-R8 | `DV_TEXT.formatting`, when set, carries one of `"plain"`, `"plain_no_newlines"`, `"markdown"` (or a legacy CSS `"name:value; …"` string); `"plain_no_newlines"` means the value contains no newlines. | dv_text.adoc Description + master05 §Formatting lines 330-334 | behaviour | low |
| rm-data-types-text-quantity-R9 | `DV_PARAGRAPH` is deprecated but remains legal and must be supported "in at least a basic way". | master05 lines 315, 415 (WARNING) | behaviour | low |
| rm-data-types-text-quantity-R10 | `CODE_PHRASE.terminology_id` is mandatory (1..1, `TERMINOLOGY_ID`); reject a CODE_PHRASE without it. | code_phrase.adoc Attributes `*1..1* terminology_id` | mandatory-attr | high |
| rm-data-types-text-quantity-R11 | `CODE_PHRASE.code_string` is mandatory (1..1, `String`); reject a CODE_PHRASE without it. | code_phrase.adoc Attributes `*1..1* code_string` | mandatory-attr | high |
| rm-data-types-text-quantity-R12 | `CODE_PHRASE` invariant `Code_string_valid: not code_string.is_empty` — reject an empty `code_string`. | code_phrase.adoc Invariants `Code_string_valid` | rejection-duty | high |
| rm-data-types-text-quantity-R13 | `DV_CODED_TEXT.defining_code` is mandatory (1..1) and typed exactly `CODE_PHRASE` (monomorphic slot, no subtypes) — a missing or foreign-`_type` `defining_code` must be rejected. | dv_coded_text.adoc Attributes `*1..1* defining_code: CODE_PHRASE` | rejection-duty | high |
| rm-data-types-text-quantity-R14 | `DV_CODED_TEXT` inherits from `DV_TEXT`, so R1/R2 (mandatory non-empty `value`, the rubric) apply to it as well. | dv_coded_text.adoc Inherit `DV_TEXT` | mandatory-attr | high |
| rm-data-types-text-quantity-R15 | `TERM_MAPPING.match` is mandatory (1..1, `Character`); reject a mapping without it. | term_mapping.adoc Attributes `*1..1* match` | mandatory-attr | high |
| rm-data-types-text-quantity-R16 | `TERM_MAPPING` invariant `Match_valid: is_valid_match_code(match)` where `is_valid_match_code(c) := c ∈ {'>','=','<','?'}` — reject any `match` character outside that set. | term_mapping.adoc Invariants `Match_valid` + `is_valid_match_code` Post | rejection-duty | high |
| rm-data-types-text-quantity-R17 | `TERM_MAPPING.target` is mandatory (1..1) and typed exactly `CODE_PHRASE` (monomorphic, no subtypes) — reject missing/foreign-`_type` target. | term_mapping.adoc Attributes `*1..1* target: CODE_PHRASE` | rejection-duty | high |
| rm-data-types-text-quantity-R18 | `TERM_MAPPING.purpose` is optional (0..1) and typed exactly `DV_CODED_TEXT`; a foreign `_type` (e.g. plain `DV_TEXT`) in the `purpose` slot must be rejected. | term_mapping.adoc Attributes `*0..1* purpose: DV_CODED_TEXT` | rejection-duty | medium |
| rm-data-types-text-quantity-R19 | `TERM_MAPPING` invariant `Purpose_valid: purpose /= Void implies terminology(Terminology_id_openehr).has_code_for_group_id(Group_id_term_mapping_purpose, purpose.defining_code)` — a present `purpose` must be coded from the openEHR "term mapping purpose" group. | term_mapping.adoc Invariants `Purpose_valid` | validity-fn | medium |
| rm-data-types-text-quantity-R20 | `DV_ORDERED` invariant `Other_reference_ranges_validity: other_reference_ranges /= Void implies not other_reference_ranges.is_empty` — reject an empty `other_reference_ranges` list. | dv_ordered.adoc Invariants `Other_reference_ranges_validity` | rejection-duty | high |
| rm-data-types-text-quantity-R21 | `DV_ORDERED` invariant `Normal_status_validity: normal_status /= Void implies code_set(Code_set_id_normal_statuses).has_code(normal_status)` — a present `normal_status` CODE_PHRASE must be in the normal-statuses code set (HHH/HH/H/N/L/LL/LLL). | dv_ordered.adoc Invariants `Normal_status_validity`; master06 line 180 | validity-fn | medium |
| rm-data-types-text-quantity-R22 | `DV_ORDERED` invariant `Normal_range_and_status_consistency: (normal_range /= Void and normal_status /= Void) implies (normal_status.code_string.is_equal("N") xor not normal_range.has(self))` — the two must not disagree about normality. | dv_ordered.adoc Invariants `Normal_range_and_status_consistency` | invariant | medium |
| rm-data-types-text-quantity-R23 | `DV_ORDERED` invariant `Is_simple_validity: (normal_range = Void and other_reference_ranges = Void) implies is_simple` — an ordered value with no reference ranges is "simple". | dv_ordered.adoc Invariants `Is_simple_validity` | invariant | low |
| rm-data-types-text-quantity-R24 | `DV_ORDERED.normal_status` is typed exactly `CODE_PHRASE`; `normal_range` typed `DV_INTERVAL`; `other_reference_ranges` a `List<REFERENCE_RANGE<DV_ORDERED>>` — foreign `_type` in these slots must be rejected. | dv_ordered.adoc Attributes | mandatory-attr | medium |
| rm-data-types-text-quantity-R25 | `DV_ORDERED.is_strictly_comparable_to(other)` is required (and gates `<` and interval limits): two DV_ORDERED are comparable only per each subtype's effected definition. | dv_ordered.adoc Functions `is_strictly_comparable_to`; Description | behaviour | medium |
| rm-data-types-text-quantity-R26 | `DV_INTERVAL<T>` invariant `Limits_consistent: (not upper_unbounded and not lower_unbounded) implies (lower.is_strictly_comparable_to(upper) and lower <= upper)` — reject an interval whose bounded lower/upper are not strictly comparable or where lower > upper. | dv_interval.adoc Invariants `Limits_consistent` | rejection-duty | high |
| rm-data-types-text-quantity-R27 | `REFERENCE_RANGE.meaning` (1..1, `DV_TEXT`) and `REFERENCE_RANGE.range` (1..1, `DV_INTERVAL`) are both mandatory — reject a reference range missing either. | reference_range.adoc Attributes `*1..1* meaning` / `*1..1* range` | mandatory-attr | high |
| rm-data-types-text-quantity-R28 | `REFERENCE_RANGE` invariant `Range_is_simple: (range.lower_unbounded or else range.lower.is_simple) and (range.upper_unbounded or else range.upper.is_simple)` — the bounding values of a reference range must themselves be simple (no nested reference ranges). | reference_range.adoc Invariants `Range_is_simple` | invariant | medium |
| rm-data-types-text-quantity-R29 | `DV_ORDINAL.symbol` is mandatory (1..1) typed exactly `DV_CODED_TEXT`; `DV_ORDINAL.value` is mandatory (1..1) `Integer` (any integer, incl. negative/zero) — reject missing/foreign-typed symbol or a non-integer value. | dv_ordinal.adoc Attributes `*1..1* symbol: DV_CODED_TEXT` / `*1..1* value: Integer` | rejection-duty | high |
| rm-data-types-text-quantity-R30 | `DV_SCALE.symbol` is mandatory (1..1) typed exactly `DV_CODED_TEXT`; `DV_SCALE.value` is mandatory (1..1) `Real` — reject missing/foreign-typed symbol. | dv_scale.adoc Attributes `*1..1* symbol: DV_CODED_TEXT` / `*1..1* value: Real` | mandatory-attr | high |
| rm-data-types-text-quantity-R31 | `DV_SCALE` permits a symbol with a blank `code_string` (uncoded scale point): a `DV_CODED_TEXT` carrying `terminology_id` and an empty `code_string`. Note this conflicts with CODE_PHRASE `Code_string_valid` (R12) — a documented spec tension. | dv_scale.adoc Attributes `symbol` meaning (blank String value for code_string) | behaviour | medium |
| rm-data-types-text-quantity-R32 | `DV_ORDINAL`/`DV_SCALE` ordering (`<`) is defined only between strictly-comparable instances; `DV_SCALE.less_than` has `Pre_comparable: is_strictly_comparable_to(other)`. | dv_scale.adoc Functions `less_than` Pre_comparable; dv_ordinal.adoc | behaviour | low |
| rm-data-types-text-quantity-R33 | `DV_QUANTIFIED.magnitude_status`, when present, must satisfy `Magnitude_status_valid: valid_magnitude_status(magnitude_status)` where the value set is `{"=", "<", ">", "<=", ">=", "~"}` — reject any other string. | dv_quantified.adoc Invariants `Magnitude_status_valid` + `valid_magnitude_status` Post | rejection-duty | high |
| rm-data-types-text-quantity-R34 | `DV_QUANTIFIED.magnitude_status` when absent means `"="` (point value); consumers must treat a missing status as equality. | dv_quantified.adoc `magnitude_status` meaning "If not present, assumed meaning is =" | behaviour | low |
| rm-data-types-text-quantity-R35 | `DV_QUANTIFIED.magnitude()` is guaranteed available on every DV_QUANTIFIED subtype and carries the effective value; ordering (`<`) is `Result = magnitude < other.magnitude` gated by `is_strictly_comparable_to`. This is the DV_ORDERED magnitude-ordering semantics the AQL engine relies on. | dv_quantified.adoc Functions `magnitude`, `less_than` Post_result; master06 lines 117 | behaviour | medium |
| rm-data-types-text-quantity-R36 | `DV_AMOUNT.accuracy` is `Real`; a value of `0` means 100% accuracy (no error); the constant `unknown_accuracy_value = -1` marks "accuracy not recorded". | dv_amount.adoc Attributes `accuracy`; master06 lines 147-148 | behaviour | medium |
| rm-data-types-text-quantity-R37 | `DV_AMOUNT` invariant `Accuracy_is_percent_validity: accuracy = 0 implies not accuracy_is_percent` — reject `accuracy = 0` combined with `accuracy_is_percent = True`. | dv_amount.adoc Invariants `Accuracy_is_percent_validity` | rejection-duty | high |
| rm-data-types-text-quantity-R38 | `DV_AMOUNT` invariant `Accuracy_validity: accuracy_is_percent implies valid_percentage(accuracy)` where `valid_percentage(n) := 0 <= n <= 100` — reject a percent accuracy outside [0,100]. | dv_amount.adoc Invariants `Accuracy_validity`; Functions `valid_percentage` | rejection-duty | high |
| rm-data-types-text-quantity-R39 | `DV_AMOUNT.accuracy_unknown()` is True iff `accuracy = unknown_accuracy_value` (-1); this is the concrete implementation of the abstract `DV_QUANTIFIED.accuracy_unknown`. | dv_amount.adoc; dv_quantified.adoc Functions `accuracy_unknown`; master06 line 148 | behaviour | medium |
| rm-data-types-text-quantity-R40 | Accuracy arithmetic on `+`/`-`: if both operand accuracies present they are summed in the result; if either is unknown the result accuracy is unknown; if only one operand `accuracy_is_percent = True`, the result form follows the larger operand. | dv_amount.adoc Functions `add`/`subtract`; master06 lines 152-156 | behaviour | medium |
| rm-data-types-text-quantity-R41 | `DV_QUANTITY.magnitude` (1..1, `Real`) and `DV_QUANTITY.units` (1..1, `String`) are both mandatory — reject a DV_QUANTITY missing either. | dv_quantity.adoc Attributes `*1..1* magnitude` / `*1..1* units` | mandatory-attr | high |
| rm-data-types-text-quantity-R42 | `DV_QUANTITY.precision`, when set: `0` means integral magnitude (whole number), `-1` means no limit (any number of decimal places); `is_integral() := precision = 0`. | dv_quantity.adoc Attributes `precision`; Functions `is_integral` | behaviour | medium |
| rm-data-types-text-quantity-R43 | `DV_QUANTITY.is_strictly_comparable_to(other)` is True iff the two have the same `units` and, when present, the same `units_system` — this gates `<`, magnitude ordering, and use as interval limits. | dv_quantity.adoc Functions `is_strictly_comparable_to`; master06 line 139 | behaviour | medium |
| rm-data-types-text-quantity-R44 | `DV_QUANTITY.units` is a UCUM (case-sensitive) code by default, or from the system named in `units_system` (a URI) when set; `units_system`/`units_display_name` are optional (0..1). | dv_quantity.adoc Attributes `units`/`units_system`/`units_display_name`; master06 line 137 | behaviour | low |
| rm-data-types-text-quantity-R45 | `DV_COUNT.magnitude` is mandatory (1..1) and typed `Integer64` — reject a non-integer/absent magnitude; there are no units or precision on DV_COUNT. | dv_count.adoc Attributes `*1..1* magnitude: Integer64`; master06 line 134 | mandatory-attr | high |
| rm-data-types-text-quantity-R46 | `DV_COUNT.is_strictly_comparable_to` always returns True (any two DV_COUNT are comparable). | dv_count.adoc Functions `is_strictly_comparable_to` "Return True" | behaviour | low |
| rm-data-types-text-quantity-R47 | `DV_PROPORTION` mandatory attributes: `numerator` (1..1, Real), `denominator` (1..1, Real), `type` (1..1, Integer) — reject any missing. | dv_proportion.adoc Attributes `*1..1*` numerator/denominator/type | mandatory-attr | high |
| rm-data-types-text-quantity-R48 | `DV_PROPORTION` invariant `Type_validity: valid_proportion_kind(type)` — `type` must be one of {0 pk_ratio, 1 pk_unitary, 2 pk_percent, 3 pk_fraction, 4 pk_integer_fraction}; reject any other integer. | dv_proportion.adoc Invariants `Type_validity`; proportion_kind.adoc Constants | rejection-duty | high |
| rm-data-types-text-quantity-R49 | `DV_PROPORTION` invariant `Valid_denominator: denominator /= 0.0` — reject a zero denominator. | dv_proportion.adoc Invariants `Valid_denominator` | rejection-duty | high |
| rm-data-types-text-quantity-R50 | `DV_PROPORTION` invariant `Unitary_validity: type = pk_unitary implies denominator = 1` — reject a unitary proportion whose denominator ≠ 1. | dv_proportion.adoc Invariants `Unitary_validity` | rejection-duty | high |
| rm-data-types-text-quantity-R51 | `DV_PROPORTION` invariant `Percent_validity: type = pk_percent implies denominator = 100` — reject a percent proportion whose denominator ≠ 100. | dv_proportion.adoc Invariants `Percent_validity` | rejection-duty | high |
| rm-data-types-text-quantity-R52 | `DV_PROPORTION` invariant `Fraction_validity: (type = pk_fraction or type = pk_integer_fraction) implies is_integral` — reject a (integer_)fraction proportion whose numerator/denominator are not both integral. | dv_proportion.adoc Invariants `Fraction_validity` | rejection-duty | high |
| rm-data-types-text-quantity-R53 | `DV_PROPORTION` invariant `Is_integral_validity: is_integral implies (numerator.floor = numerator and denominator.floor = denominator)` — an integral proportion must have whole-number numerator and denominator. | dv_proportion.adoc Invariants `Is_integral_validity` | rejection-duty | high |
| rm-data-types-text-quantity-R54 | `DV_PROPORTION` invariant `Precision_validity: precision = 0 implies is_integral` — a proportion declaring precision 0 must be integral. | dv_proportion.adoc Invariants `Precision_validity` | rejection-duty | medium |
| rm-data-types-text-quantity-R55 | `DV_PROPORTION.is_strictly_comparable_to(other)` is True iff `type` matches — proportions of different kinds are not comparable. | dv_proportion.adoc Functions `is_strictly_comparable_to` | behaviour | low |
| rm-data-types-text-quantity-R56 | `DV_ABSOLUTE_QUANTITY.accuracy` is redefined to type `DV_AMOUNT` (differential); `accuracy_unknown` = Void/null accuracy. Foreign `_type` in the accuracy slot must be rejected. | dv_absolute_quantity.adoc Attributes `accuracy: DV_AMOUNT`; master06 line 148 | mandatory-attr | medium |
| rm-data-types-text-quantity-R57 | Type hierarchy for polymorphic `DATA_VALUE`/`DV_ORDERED` slots: the concrete descendants are `DV_ORDINAL`, `DV_SCALE`, `DV_QUANTITY`, `DV_COUNT`, `DV_PROPORTION`, `DV_DURATION`, and the date/time absolute types; `DV_QUANTIFIED`, `DV_AMOUNT`, `DV_ABSOLUTE_QUANTITY`, `DV_ORDERED` are abstract and must never appear as an instance `_type`. | master06 Design lines 113-134; class headers "(abstract)" | rejection-duty | high |
| rm-data-types-text-quantity-R58 | `DV_ORDERED.is_normal()` semantics: value is normal per `normal_range.has(self)` if `normal_range` present, else per `normal_status.code_string = "N"`; requires at least one of `normal_range`/`normal_status`. | dv_ordered.adoc Functions `is_normal` Pre/Post | behaviour | low |
</content>
</invoke>
