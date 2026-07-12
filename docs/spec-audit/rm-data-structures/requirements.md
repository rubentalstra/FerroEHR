# RM data_structures — machine-checkable requirements

- **Chapter:** rm-data-structures
- **Date:** 2026-07-11
- **Spec files read (relative to `docs/specs/openehr/`):**
  - `RM/docs/data_structures/master04-item_structure_package.adoc`
  - `RM/docs/data_structures/master05-representation_package.adoc`
  - `RM/docs/data_structures/master06-history_package.adoc`
  - `RM/docs/UML/classes/org.openehr.rm.data_structures.data_structure.adoc`
  - `RM/docs/UML/classes/org.openehr.rm.data_structures.item_structure.adoc`
  - `RM/docs/UML/classes/org.openehr.rm.data_structures.item_single.adoc`
  - `RM/docs/UML/classes/org.openehr.rm.data_structures.item_list.adoc`
  - `RM/docs/UML/classes/org.openehr.rm.data_structures.item_table.adoc`
  - `RM/docs/UML/classes/org.openehr.rm.data_structures.item_tree.adoc`
  - `RM/docs/UML/classes/org.openehr.rm.data_structures.item.adoc`
  - `RM/docs/UML/classes/org.openehr.rm.data_structures.cluster.adoc`
  - `RM/docs/UML/classes/org.openehr.rm.data_structures.element.adoc`
  - `RM/docs/UML/classes/org.openehr.rm.data_structures.history.adoc`
  - `RM/docs/UML/classes/org.openehr.rm.data_structures.event.adoc`
  - `RM/docs/UML/classes/org.openehr.rm.data_structures.point_event.adoc`
  - `RM/docs/UML/classes/org.openehr.rm.data_structures.interval_event.adoc`

All listed files exist; no corrections needed. Note: the class detail lives in
`RM/docs/UML/classes/org.openehr.rm.data_structures.*.adoc` (included by the
master files), which is where invariants/attributes are declared.

---

## Requirements

| id | requirement | citation | category | risk |
|----|-------------|----------|----------|------|
| rm-data-structures-R1 | `ELEMENT.null_flavour`, when present, must be typed `DV_CODED_TEXT` (monomorphic slot — reject any other `_type`). | `.../element.adoc` L18-20 (`0..1 null_flavour: DV_CODED_TEXT`) | mandatory-attr | medium |
| rm-data-structures-R2 | `ELEMENT.value`, when present, must be a concrete subtype of `DATA_VALUE`. | `.../element.adoc` L22-24 (`0..1 value: DATA_VALUE`; "any concrete subtype of DATA_VALUE can be used") | mandatory-attr | low |
| rm-data-structures-R3 | `ELEMENT.null_reason`, when present, must be typed `DV_TEXT` (monomorphic slot). | `.../element.adoc` L26-28 (`0..1 null_reason: DV_TEXT`) | mandatory-attr | low |
| rm-data-structures-R4 | `ELEMENT.is_null()` must equal `(value = Void)`: an ELEMENT is null exactly when it has no `value`. | `.../element.adoc` L38-39 (`Inv_is_null_valid: is_null() = (value = Void)`) | invariant | medium |
| rm-data-structures-R5 | An ELEMENT must have exactly one of `value` / `null_flavour` set: reject an ELEMENT carrying both a `value` and a `null_flavour`, and reject one carrying neither (`is_null() xor null_flavour = Void`). | `.../element.adoc` L41-42 (`Inv_null_flavour_indicated: is_null() xor null_flavour = Void`) | rejection-duty | high |
| rm-data-structures-R6 | When an ELEMENT is null, its `null_flavour.defining_code` must be a code in the openEHR terminology `null flavour` group; reject a null ELEMENT whose null_flavour code is outside that group. | `.../element.adoc` L44-45 (`Inv_null_flavour_valid: is_null implies terminology(...).has_code_for_group_id(Group_id_null_flavour, null_flavour.defining_code)`); master04 L18 | rejection-duty | high |
| rm-data-structures-R7 | If `ELEMENT.null_reason` is set then the ELEMENT must be null (`value = Void`); reject a `null_reason` accompanying a non-null value. | `.../element.adoc` L47-48 (`Inv_null_reason_valid: null_reason /= Void implies is_null()`) | rejection-duty | medium |
| rm-data-structures-R8 | If `ELEMENT.null_reason` is set then `null_flavour` must also be set. | `.../element.adoc` L26-28 ("if set, `_null_flavour_` must be set") | rejection-duty | medium |
| rm-data-structures-R9 | `CLUSTER.items` is mandatory (1..1): an ordered `List<ITEM>` of `CLUSTER`/`ELEMENT` objects must be present. | `.../cluster.adoc` L18-20 (`1..1 items: List<ITEM>`) | cardinality | medium |
| rm-data-structures-R10 | `ITEM_SINGLE.item` is mandatory (1..1) and monomorphically typed `ELEMENT`: reject any non-ELEMENT (e.g. CLUSTER) in this slot. | `.../item_single.adoc` L18-19 (`1..1 item: ELEMENT`) | rejection-duty | high |
| rm-data-structures-R11 | `ITEM_LIST.items`, when present, is a `List<ELEMENT>`. | `.../item_list.adoc` L22-23 (`0..1 items: List<ELEMENT>`) | mandatory-attr | low |
| rm-data-structures-R12 | Every member of `ITEM_LIST.items` must be an `ELEMENT`; reject an ITEM_LIST containing a `CLUSTER` (or any non-ELEMENT). | `.../item_list.adoc` L55-56 (`Valid_structure: items.forall(i:ITEM | i.type = "ELEMENT")`) | rejection-duty | high |
| rm-data-structures-R13 | `ITEM_TABLE.rows`, when present, is a `List<CLUSTER>` (monomorphic): each row must be a `CLUSTER`; reject a non-CLUSTER row. | `.../item_table.adoc` L24-25 (`0..1 rows: List<CLUSTER>`) | rejection-duty | high |
| rm-data-structures-R14 | Every item of every row CLUSTER in `ITEM_TABLE.rows` must be an `ELEMENT`; reject a row containing a nested CLUSTER. | `.../item_table.adoc` L96-97 (`Valid_structure: rows.for_all(items.for_all(instance_of("ELEMENT")))`) | rejection-duty | high |
| rm-data-structures-R15 | ITEM_TABLE row regularity: each row CLUSTER must have an identical number of ELEMENTs, and the ELEMENTs must have identical names and value types in corresponding positions across all rows. | `.../item_table.adoc` L9 ("Each row Cluster must have an identical number of Elements, each of which in turn must have identical names and value types in the corresponding positions in each row") | rejection-duty | high |
| rm-data-structures-R16 | `ITEM_TREE.items`, when present, is a `List<ITEM>` and may contain 0+ `CLUSTER`s and/or 0+ `ELEMENT`s (both subtypes permitted). | `.../item_tree.adoc` L18-20 (`0..1 items: List<ITEM>`) | mandatory-attr | low |
| rm-data-structures-R17 | `HISTORY.origin` is mandatory (1..1) and typed `DV_DATE_TIME`. | `.../history.adoc` L20-21 (`1..1 origin: DV_DATE_TIME`) | mandatory-attr | medium |
| rm-data-structures-R18 | `HISTORY.events`, when present, is a `List<EVENT<T>>` whose T is a descendant of `ITEM_STRUCTURE`. | `.../history.adoc` L36-38 (`0..1 events: List<EVENT<T>>`); L8-9 | mandatory-attr | low |
| rm-data-structures-R19 | A HISTORY must have either a non-empty `events` list or a `summary`; reject a HISTORY with empty/absent events and no summary. | `.../history.adoc` L48-49 (`Events_valid: (events /= Void and then not events.is_empty) or summary /= Void`) | rejection-duty | high |
| rm-data-structures-R20 | `HISTORY.period` must be set iff the history is periodic (`is_periodic xor period = Void`); reject a period on an aperiodic history and a periodic history without a period. | `.../history.adoc` L51-52 (`Periodic_validity: is_periodic xor period = Void`) | rejection-duty | high |
| rm-data-structures-R21 | For a periodic HISTORY, every event's offset (in seconds) must be an integer multiple of `period` (offset mod period = 0); reject an off-grid event. | `.../history.adoc` L54-55 (`Period_consistency: is_periodic implies events.for_all(e: EVENT | e.offset.to_seconds.mod(period.to_seconds) = 0)`); master06 L11 | rejection-duty | high |
| rm-data-structures-R22 | All events in one HISTORY must carry the same `ITEM_STRUCTURE` subtype in `EVENT.data` (the generic parameter T locks every event's data type); reject a HISTORY mixing e.g. ITEM_LIST and ITEM_TREE event data. | master06 L21 ("A History of type HISTORY<ITEM_LIST> ... constrains the type of the data at each Event (EVENT._item_) to be of type ITEM_LIST and nothing else"); `.../history.adoc` L8-9 | rejection-duty | high |
| rm-data-structures-R23 | `HISTORY.period` and `HISTORY.duration`, when present, are typed `DV_DURATION`. | `.../history.adoc` L24-30 (`0..1 period: DV_DURATION`, `0..1 duration: DV_DURATION`) | mandatory-attr | low |
| rm-data-structures-R24 | `HISTORY.summary`, when present, is typed `ITEM_STRUCTURE` (any concrete subtype). | `.../history.adoc` L32-34 (`0..1 summary: ITEM_STRUCTURE`) | mandatory-attr | low |
| rm-data-structures-R25 | `EVENT.time` is mandatory (1..1) and typed `DV_DATE_TIME`; for a non-zero-width event it is the trailing/end edge of the interval. | `.../event.adoc` L18-20 (`1..1 time: DV_DATE_TIME`); master06 L28,L50 | mandatory-attr | medium |
| rm-data-structures-R26 | `EVENT.data` is mandatory (1..1) and typed `T`, constrained to a descendant of `ITEM_STRUCTURE`. | `.../event.adoc` L26-28 (`1..1 data: T`); master06 L13 | mandatory-attr | high |
| rm-data-structures-R27 | `EVENT.state`, when present, is typed `ITEM_STRUCTURE`. | `.../event.adoc` L22-23 (`0..1 state: ITEM_STRUCTURE`) | mandatory-attr | low |
| rm-data-structures-R28 | `EVENT` is abstract: a concrete event must be a `POINT_EVENT` or `INTERVAL_EVENT`; reject a bare `EVENT` instance. | `.../event.adoc` L6 (`EVENT<T> (abstract)`); `.../point_event.adoc`; `.../interval_event.adoc` | rejection-duty | medium |
| rm-data-structures-R29 | `EVENT.offset()` must equal `time.diff(parent.origin)`. | `.../event.adoc` L34-41 (`Post_condition Result = time.diff(parent.origin)`; `Offset_validity1: offset = time.diff(parent.origin)`) | validity-fn | low |
| rm-data-structures-R30 | `INTERVAL_EVENT.width` is mandatory (1..1) and typed `DV_DURATION`. | `.../interval_event.adoc` L18-20 (`1..1 width: DV_DURATION`) | mandatory-attr | high |
| rm-data-structures-R31 | `INTERVAL_EVENT.math_function` is mandatory (1..1) and typed `DV_CODED_TEXT`, defaulting to `640|actual|`. | `.../interval_event.adoc` L26-28 (`1..1 math_function: DV_CODED_TEXT`; "Default value 640|actual|") | mandatory-attr | medium |
| rm-data-structures-R32 | `INTERVAL_EVENT.math_function.defining_code` must be a code in the openEHR terminology `event math function` group; reject any code outside that group. | `.../interval_event.adoc` L38-39 (`Math_function_validity: terminology(...).has_code_for_group_id(Group_id_event_math_function, math_function.defining_code)`); master06 L52 | rejection-duty | high |
| rm-data-structures-R33 | `INTERVAL_EVENT.interval_start_time()` must equal `time - width`. | `.../interval_event.adoc` L41-42 (`Interval_start_time_valid: interval_start_time = time - width`) | validity-fn | low |
| rm-data-structures-R34 | `INTERVAL_EVENT.sample_count`, when present, is typed `Integer`. | `.../interval_event.adoc` L22-23 (`0..1 sample_count: Integer`) | mandatory-attr | low |
| rm-data-structures-R35 | The abstract classes `DATA_STRUCTURE`, `ITEM_STRUCTURE`, and `ITEM` are not directly instantiable; reject an instance whose `_type` names an abstract class. | `.../data_structure.adoc` L6 (`DATA_STRUCTURE (abstract)`); `.../item_structure.adoc` L6 (`ITEM_STRUCTURE (abstract)`); `.../item.adoc` L6 (`ITEM (abstract)`) | rejection-duty | low |
| rm-data-structures-R36 | ITEM_TABLE ISO-13606 encoding: an empty/void column value in a row is represented by an ELEMENT with no `value` and its `null_flavour` set; each row CLUSTER's name is the stringified row number and each row ELEMENT's name is its column name. | master04 L38-43 (ITEM_TABLE encoding rules) | behaviour | low |
| rm-data-structures-R37 | For an INTERVAL_EVENT the event's `math_function` applies to all data points attached to that event's `data`. | master06 L52 ("The math function value on a particular event applies to all the data points attached to the event `_data_` attribute") | behaviour | low |
