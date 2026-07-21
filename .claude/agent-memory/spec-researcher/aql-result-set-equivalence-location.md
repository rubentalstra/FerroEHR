---
name: aql-result-set-equivalence-location
description: Where AQL 1.1 result-set semantics (row order, DISTINCT/duplicates, ORDER BY, TOP/LIMIT, cell values) and the ITS-REST RESULT_SET wire shape live — for conformance result-equivalence rules
metadata:
  type: reference
---

# AQL result-set equivalence — location map

**AQL 1.1 semantics** (QUERY component, `docs/specs/openehr/QUERY/docs/AQL/`):
- `master04-result_structure.adoc` — whole chapter, ~8 lines: raw result =
  `Array<Array<Any>>`; NULL = missing/unknown cell; annotated result sets are
  "not formally defined by this specification" (delegated to SM Query + REST).
- `master03-syntax.adoc` — the SELECT chapter owns everything:
  - `§DISTINCT` (~L1055) — bag-by-default; DISTINCT removes rows equal on every column.
  - `§TOP` (~L1070) — deprecated 1.1.0; not combinable with LIMIT.
  - `§Name alias` (~L1089) — `AS` alias = AQL variable syntax.
  - `§ORDER BY` (~L1094) — the load-bearing NOTE at L1098: no ORDER BY ⇒
    "default ordering in results is undefined" (impl-defined or random).
    ASC default (L1111); multi-key tie-break left-to-right (L1113); sort needs
    Ordered/primitive comparability (L1109). NO null-ordering rule stated.
  - `§LIMIT` (~L1115) — OFFSET/row_count 0-based; determinism "requires ORDER BY";
    DISTINCT applied before LIMIT/OFFSET.
- `master06-writing_AQL.adoc §Ordering and pagination` (~L98) — worked example only.
- Aggregate fns (MIN/MAX/SUM/AVG/COUNT) ignore NULL inputs (master03 ~L507-519).

**ITS-REST RESULT_SET wire shape** (`docs/specs/openehr/ITS-REST/specifications/`):
- `schemas/query/ResultSet.yaml` — required=`rows` ONLY; meta/name/q/columns optional;
  rows described "An ordered set of RESULT_SET rows" (server order authoritative on wire).
- `schemas/query/ResultSetColumn.yaml` — `name` REQUIRED, `path` optional; the
  `#0`/`#1` 0-based hash-index convention for unaliased columns is defined HERE
  (NOT in AQL — AQL only has AS).
- `schemas/query/ResultSetRow.yaml` — row = untyped array (`items: {}`); cell = ANY;
  example mixes primitives + full RM objects with `_type` (DV_TEXT).
- `schemas/query/ResultSetMetadata.yaml` — all meta fields optional, additionalProperties:true.
- `docs/query/Request.md` — offset/fetch params; fetch "cannot be combined with AQL-top".
- `docs/overview/Resources.md` — canonical JSON SHOULD (not MUST) validate vs ITS-JSON;
  NO integer-vs-real cell number-typing rule stated.

**SPEC SILENCES (a conformance schedule must legislate):** no-ORDER-BY row order,
NULL sort position, cross-type cell number typing (int vs real rendering),
missing-cell repr (JSON null vs absent), meta-field equivalence. CNF query
schedule `CNF/docs/platform_test_schedule/master11-func_tc_querying.adoc` is a
STUB (all "xx"/"TBD", 108 lines) — defines NO result-comparison rules.
