# A1 Spec Audit — Verify + Fix — chapter `rm-data-types-rest`

- **Chapter:** RM 1.2.0 data_types basic / date_time / time_specification /
  encapsulated / uri (master04, 07, 08, 09, 10)
- **Date:** 2026-07-11
- **Scope:** all 45 requirements `rm-data-types-rest-R1 … R45`
- **Result (defer-nothing pass):** 2 gaps fixed — the DV_ENCAPSULATED /
  DV_MULTIMEDIA code-set invariants (charset, language, compression +
  integrity-check algorithms) and the DV_PERIODIC_TIME_SPECIFICATION
  formalism invariant, both previously unenforced. Everything else verifies
  through the standing three layers (fail-closed typed deserialize,
  `*_impl.rs` invariants, walker terminology pass).

## Verdict table

| id | classification | evidence / fix |
|---|---|---|
| R1/R42/R43 | verified | abstract `_type`s have no enum variant/dispatch arm — parent deserialize fails |
| R2 | verified | `DvBoolean.value: bool` non-optional |
| R3/R4 | verified | `DvState.value: DvCodedText`, `is_terminal: bool` non-optional (monomorphic slots, fail-closed) |
| R5 | verified | `DvIdentifier.id: String` non-optional; others `Option` |
| R6 | verified | `dv_identifier_impl.rs` `Id_valid` |
| R7/R11/R14/R16 | verified | `Value_valid` in `dv_date/time/date_time/duration_impl.rs` over `validate.rs::is_valid_iso_*` (calendar-exact `Day_valid` included) |
| R8/R9 | verified | `is_valid_iso_date` accepts `YYYY`/`YYYY-MM`; the structure makes month-absent/day-present unrepresentable (tests in `validate.rs`) |
| R10/R13/R15/R19 | verified-behavioural | magnitude functions — realized for AQL by `ext.openehr_magnitude` (chapter 16 cross-checks) |
| R12 | verified | `is_valid_iso_time` partial forms + timezone handling (`validate.rs`) |
| R17/R18 | verified | duration deviations (leading `-`, `W` mixing) in `is_valid_iso_duration` (tests) |
| R20 | verified | temporal `accuracy: Option<DvDuration>` — monomorphic, fail-closed |
| R21 | verified | `push_magnitude_status_valid` on DV_DATE/TIME/DATE_TIME; duration via `push_dv_amount_invariants` |
| R22/R23 | verified | `dv_duration_impl.rs` `push_dv_amount_invariants` |
| R24/R25/R26 | fixed-in-this-pass | `Charset_valid`/`Language_valid` on DV_MULTIMEDIA + DV_PARSABLE — new walker code-set slots (`terminology.rs`); the CODE_PHRASE typing was already fail-closed |
| R27 | verified | walker `CodeSet::MediaTypes` + `media_type` non-optional |
| R28/R29/R30 | verified | `dv_multimedia_impl.rs` `Size_valid`/`Not_empty`/`Integrity_check_validity` |
| R31/R32 | fixed-in-this-pass | `Integrity_check_algorithm_validity`/`Compression_algorithm_validity` — new walker code sets over the TERM bundle's `integrity_check_algorithms`/`compression_algorithms`; corpus-scanned safe (0 occurrences) |
| R33/R34 | verified | `uri: Option<DvUri>`, `thumbnail: Option<Box<DvMultimedia>>`, byte arrays — typed, fail-closed |
| R35/R36/R37 | verified | `DvParsable.value/formalism` non-optional; `dv_parsable_impl.rs` `Formalism_valid` (+ `Size_valid` derived from value bytes) |
| R38 | verified | `dv_uri_impl.rs` `Value_valid` |
| R39 | verified | `dv_ehr_uri_impl.rs` `Scheme_valid` |
| R40 | verified | `DvPeriodicTimeSpecification.value: DvParsable` non-optional monomorphic |
| R41 | fixed-in-this-pass | `Value_valid` (HL7:PIVL / HL7:EIVL) — new `dv_periodic_time_specification_impl.rs` + dispatch arm; test `formalism_is_constrained` |
| R44 | verified-behavioural | `size` = unencoded byte count is a serialization convention; the codec stores `data` verbatim base64 and never rewrites `size` |
| R45 | verified-behavioural | usage guidance (duration ≠ point in time); no wire state |

## Fixes applied

- **R24/R25/R31/R32** — `crates/openehr-flat/src/validation/terminology.rs`:
  DV_MULTIMEDIA gains `charset`/`language`/`compression_algorithm`/
  `integrity_check_algorithm` code-set slots, DV_PARSABLE gains
  `charset`/`language`; two new `CodeSet` variants resolved against the TERM
  bundle's internal code sets.
- **R41** — `dv_periodic_time_specification_impl.rs` (new) + dispatch arm.

## Deferred

None.

## Uncertain / runtime probes

None remaining.
