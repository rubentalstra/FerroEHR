# A1 Spec Audit — Verify + Fix — chapter `rm-data-structures`

- **Chapter:** RM 1.2.0 data_structures (item_structure / representation / history)
- **Date:** 2026-07-11
- **Scope:** all 37 requirements `rm-data-structures-R1 … R37`
- **Result (defer-nothing pass):** 3 gaps fixed (ITEM_TABLE row regularity;
  CLUSTER.items presence; uniform HISTORY event-data type); the rest verify
  clean through the fail-closed typed deserialize (`validate.rs::run`), the
  hand-written `*_impl.rs` invariants, and the walker terminology pass —
  the same three layers as rm-composition (see that chapter's header).

## Verdict table

| id | classification | evidence / fix |
|---|---|---|
| R1–R3 | verified | `Element` struct: `null_flavour: Option<DvCodedText>`, `value: Option<DataValue>` (closed enum), `null_reason: Option<DvText>` — foreign `_type`s fail the fail-closed deserialize |
| R4 | verified-vacuous | `is_null()` is derived (= `value.is_none()`); no independent wire state |
| R5 | verified | `element_impl.rs` `Inv_null_flavour_indicated` (XOR) | 
| R6 | verified | walker terminology pass `Group::NullFlavour` (any node) |
| R7 | verified | `element_impl.rs` `Inv_null_reason_valid` |
| R8 | verified-derived | entailed by R5 + R7 (`null_reason` ⇒ null ⇒ `null_flavour` set) |
| R9 | fixed-in-this-pass | `CLUSTER.items` presence (1..1; ITS-JSON requires it) — JSON-level check in the walker (`check_data_structure_shapes`); post-deserialize absent ≡ empty `Vec`. Corpus-scanned: 0 items-less clusters in corpus/CNF |
| R10 | verified | `ItemSingle.item: Element` — monomorphic, fail-closed |
| R11/R12 | verified | `ItemList.items: Vec<Element>` — a CLUSTER member fails deserialize (`Valid_structure` by type) |
| R13 | verified | `ItemTable.rows: Vec<Cluster>` |
| R14 | verified | `item_table_impl.rs` `Valid_structure` (row items all ELEMENT) |
| R15 | fixed-in-this-pass | row regularity beyond count: corresponding ELEMENTs must share names + value-type discriminants (`item_table_impl.rs` `Row_regularity`; `Valid_number_of_rows` already covered the count) |
| R16 | verified | `ItemTree.items: Vec<Item>` (both subtypes) |
| R17 | verified | `History.origin: DvDateTime` non-optional |
| R18 | verified | `events: Vec<Event<T>>`; on the OBSERVATION path T = ItemStructure |
| R19 | verified | `history_impl.rs` `Events_valid` |
| R20 | verified-vacuous | `is_periodic()` is derived from `period` presence — the XOR is tautological in the object model |
| R21 | verified | `history_impl.rs` `Period_consistency` (modulo with ε for nominal-seconds rounding) |
| R22 | fixed-in-this-pass | uniform event-data type per HISTORY ("HISTORY<ITEM_LIST> … and nothing else", master06) — JSON-level check (`check_data_structure_shapes`): the monomorphized runtime type cannot see T. Corpus-scanned: 0 mixed histories |
| R23/R24 | verified | `period`/`duration: Option<DvDuration>`, `summary: Option<ItemStructure>` |
| R25–R27 | verified | `Event.time: DvDateTime`, `data: T` non-optional, `state: Option<ItemStructure>` |
| R28 | verified | `Event` is an untagged closed enum (POINT_EVENT / INTERVAL_EVENT) — a bare `EVENT` `_type` fails deserialize |
| R29/R33 | verified-vacuous | derived functions (`offset`, `interval_start_time`) — no wire state |
| R30/R31 | verified | `IntervalEvent.width: DvDuration`, `math_function: DvCodedText` non-optional |
| R32 | verified | walker terminology pass `Group::EventMathFunction` |
| R34 | verified | `sample_count: Option<i32>` |
| R35 | verified | abstract names have no enum variant / dispatch arm — a `_type: ITEM` (etc.) fails the parent deserialize |
| R36 | verified-behavioural | the ISO-13606 table encoding is a data-authoring convention; the R14/R15 invariants police the checkable part (structure + regularity), and a void cell as null-flavoured ELEMENT is exactly what R5/R6 validate |
| R37 | verified-behavioural | `math_function` scope is semantics of interpretation, not a wire check; the group membership (R32) is the checkable duty |

## Fixes applied

- **R15** — `crates/openehr-rm/src/data_structures/item_structure/item_table_impl.rs`:
  `Row_regularity` (names + value-type discriminants per position); test
  `row_regularity_names_and_value_types`.
- **R9/R22** — `crates/openehr-flat/src/validation/mod.rs::check_data_structure_shapes`;
  test `data_structure_shapes_are_enforced`. Both corpus-scanned safe before
  enforcement (0 violations in the canonical corpus + CNF data sets).

## Deferred

None.

## Uncertain / runtime probes

None remaining.
