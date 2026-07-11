# A1 Spec Audit — Verify + Fix — chapter `rm-data-types-text-quantity`

- **Chapter:** RM 1.2.0 data_types text (master05) + quantity (master06)
- **Date:** 2026-07-11
- **Scope:** all 58 requirements `rm-data-types-text-quantity-R1 … R58`
- **Result (defer-nothing pass):** 3 gaps fixed (DV_TEXT/DV_CODED_TEXT had NO
  invariant enforcement at all — `Valid_value`/`Formatting_valid` now
  dispatched; `Mappings_valid` + `Other_reference_ranges_validity`
  present-empty lists now checked at the JSON level); one documented spec
  tension (R31). The quantity family was already thoroughly covered by the
  shared `push_dv_amount_invariants`/`push_normal_range_consistency` helpers.

## Verdict table

| id | classification | evidence / fix |
|---|---|---|
| R1 | verified | `DvTextData.value: String` non-optional — fail-closed deserialize |
| R2 | fixed-in-this-pass | `Valid_value` — new `dv_text_impl.rs` + `DV_TEXT`/`DV_CODED_TEXT` dispatch arms (there were NONE); corpus-scanned safe (0 empty rubrics) |
| R3 | fixed-in-this-pass | `Mappings_valid` — generic present-empty list check (`check_nonempty_lists`, attribute-keyed) |
| R4 | fixed-in-this-pass | `Formatting_valid` — `dv_text_impl.rs` |
| R5/R6 | verified | walker terminology pass `CodeSet::Languages`/`CharacterSets` (DV_TEXT/DV_CODED_TEXT arm) |
| R7 | verified | `language`/`encoding: Option<CodePhrase>` — monomorphic, fail-closed |
| R8 | verified-behavioural | formatting vocabulary is authoring guidance; the checkable arm (non-empty) is R4 |
| R9 | verified | `DV_PARAGRAPH` generated + accepted (deprecated but legal) |
| R10/R11 | verified | `CodePhrase.terminology_id`/`code_string` non-optional |
| R12 | verified | `code_phrase_impl.rs` `Code_string_valid` |
| R13 | verified | `DvCodedText.defining_code: CodePhrase` non-optional monomorphic |
| R14 | fixed-in-this-pass | inherited `Valid_value` on `DV_CODED_TEXT` — same dispatch arm |
| R15 | verified | `TermMapping.r#match: char` non-optional |
| R16 | verified | `term_mapping_impl.rs` `Match_valid` |
| R17/R18 | verified | `target: CodePhrase` / `purpose: Option<DvCodedText>` — fail-closed |
| R19 | verified | walker `Group::TermMappingPurpose` |
| R20 | fixed-in-this-pass | `Other_reference_ranges_validity` — generic present-empty check |
| R21 | verified | walker `CodeSet::NormalStatuses` (any node with `normal_status`) |
| R22 | verified | `push_normal_range_consistency` on QUANTITY/COUNT/ORDINAL/SCALE/PROPORTION |
| R23 | verified-derived | `is_simple` derived (no wire state); R28 checks the composable arm |
| R24 | verified | typed slots — fail-closed deserialize |
| R25 | verified-behavioural | `is_strictly_comparable_to` per subtype in `dv_ordered_impl`; gates `Limits_consistent` (R26) and the AQL ordered-magnitude semantics (`ext.openehr_magnitude`; chapter 16 cross-checks the engine) |
| R26 | verified | `dv_interval_impl.rs` `Limits_consistent` (+ `Lower/Upper_included_valid`) |
| R27 | verified | `ReferenceRange.meaning`/`range` non-optional |
| R28 | verified | `reference_range_impl.rs` `Range_is_simple` |
| R29/R30 | verified | `symbol: DvCodedText` + `value` non-optional on ORDINAL/SCALE |
| R31 | verified-with-PORT-NOTE | spec tension: `dv_scale.adoc` permits a blank symbol code_string, `Code_string_valid` forbids it — we enforce strict (corpus/CNF carry no blank-symbol scales; PORT NOTE in `dv_scale_impl.rs`) |
| R32 | verified-behavioural | `less_than` preconditions in `dv_ordered_impl` |
| R33 | verified | `Magnitude_status_valid` via `push_dv_amount_invariants` (QUANTITY/COUNT/PROPORTION) |
| R34–R36 | verified-behavioural | absent-status ≡ "=", magnitude()/accuracy semantics — consumer-side; the checkable invariants are R33/R37/R38 |
| R37/R38 | verified | `Accuracy_is_percent_validity`/`Accuracy_valid` via `push_dv_amount_invariants` |
| R39/R40 | verified-behavioural | accuracy_unknown/arithmetic are computational functions with no stored state to validate |
| R41 | verified | `DvQuantity.magnitude`/`units` non-optional |
| R42–R44 | verified-behavioural | precision/comparability/UCUM semantics; UCUM unit validation itself is archetype/terminology-level (C_DV_QUANTITY walker constraints enforce template-declared units) |
| R45 | verified | `DvCount.magnitude: i64` (Integer64) non-optional |
| R46 | verified-behavioural | trivial constant |
| R47 | verified | `numerator`/`denominator`/`type` non-optional |
| R48–R54 | verified | `dv_proportion_impl.rs`: Type/Valid_denominator/Unitary/Percent/Fraction/Precision validity (Is_integral_validity realized through the shared `is_integral` floor test the Fraction/Precision checks use) |
| R55 | verified-behavioural | comparability by kind (`dv_ordered_impl`) |
| R56 | verified | date/time `accuracy: Option<DvDuration>`-style typed slots (chapter 6 audits the absolute-quantity family in detail) |
| R57 | verified | abstract names have no enum variant — `_type: DV_ORDERED`/`DV_AMOUNT`/… fails the parent deserialize; the whole DATA_VALUE set is a closed untagged enum |
| R58 | verified-behavioural | `is_normal` is derived; its consistency arm is R22 |

## Fixes applied

- **R2/R4/R14** — `crates/openehr-rm/src/data_types/text/dv_text_impl.rs`
  (new) + `DV_TEXT`/`DV_CODED_TEXT` dispatch arms in `validate.rs`; test
  `value_and_formatting_invariants`. Corpus-scanned safe.
- **R3/R20** — attribute-keyed present-empty list checks (`mappings`,
  `other_reference_ranges`) in
  `crates/openehr-flat/src/validation/mod.rs::check_nonempty_lists`.
- **R31** — PORT NOTE (spec tension recorded in `dv_scale_impl.rs`).

## Deferred

None.

## Uncertain / runtime probes

None remaining.
