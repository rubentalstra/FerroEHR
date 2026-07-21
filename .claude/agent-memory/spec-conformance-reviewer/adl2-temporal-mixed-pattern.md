---
name: adl2-temporal-mixed-pattern
description: ADL2 mixed pattern+interval temporal form is DURATION-ONLY; emitting it for C_DATE/C_TIME/C_DATE_TIME yields non-reparseable ADL2
metadata:
  type: feedback
---

The ADL2 `pattern/interval` mixed temporal constraint form (`PWD/|P0W..P50W|`)
is documented and grammar-supported for DURATIONS ONLY
(`docs/specs/openehr/AM/docs/ADL2/master04.5-cadl_primitive_types.adoc`
§Mixed Pattern and Interval sits under "Duration Constraints"; only
`parse_c_duration` in `crates/openehr-adl/src/cadl.rs` eats the `/` slash).

`parse_c_date`/`parse_c_time`/`parse_c_date_time` accept a pattern XOR a
value-list, never both.

**Why:** the AOM2 class model (`c_date`/`c_time`/`c_date_time` tables) DOES
carry both `constraint: List<Interval>` and the inherited `pattern_constraint`,
so an object with both is model-valid but SYNTAX-invalid — the printer's
`temporal()` helper emits `pattern/interval` for all four types, but 3 of 4
can't re-parse.

**How to apply:** when reviewing anything that populates temporal
`pattern_constraint` AND `constraint` together (notably the OPT-1.4→ADL2
converter `map_primitive_object`, which carries both 1.4 `pattern`+`range`
fields unconditionally), flag it: for date/time/date_time the converter must
pick ONE, or the printer must refuse the mixed form for non-duration types.
A print→parse corpus gate only catches this if the corpus happens to contain a
date/time with both set — it usually doesn't, so the gate is green while the
invariant is false in general.
