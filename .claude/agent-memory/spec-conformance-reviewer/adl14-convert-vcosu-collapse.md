---
name: adl14-convert-vcosu-collapse
description: Verified spec facts for reviewing the ADL 1.4→2 converter's VCOSU re-mint + specialised-code depth-0 collapse (openehr-adl adl14/convert.rs)
metadata:
  type: feedback
---

Verified 2026-07-22 against the vendored AM specs when reviewing
`crates/openehr-adl/src/adl14/convert.rs` collapse + VCOSU machinery.

**The output-validity rules the converter output must satisfy** (conversion
mechanics themselves are spec-silent — ADL2 `master01-preface` line 103 says
1.4→2 is "implemented in the ADL Workbench tool", i.e. tool-delegated, not
normatively specified; `master09.02` line 5 confirms differential form needs
the resolvable lineage back to the ultimate parent):

- **VCOSU** (`AOM2/master04.5` §Validity Rules: C_OBJECT, line 337): "every
  object node must be unique within the archetype" — archetype-WIDE, not
  sibling. 1.4 at-codes are only sibling-unique, so a reused code's 2nd+
  occurrence MUST re-mint a fresh id. VCOID (line 334) = every node must have
  an id. Re-mint is correct + spec-required, and runs unconditionally (not
  gated on collapse).
- **VATDF** (`master03` line 220): each value/node code used must be DEFINED
  in term_definitions of the flattened form → a re-minted id must clone the
  first occurrence's term (and binding). Verified the clone keys off
  `node_map[old]` which is always inserted first, so `out.get(first)` resolves.
- **Depth-0 collapse satisfies**: VARCN (`master03` line 216/217 — root id must
  be `id1{.1}*`, #`.1`==spec depth, and defined; at depth 0 → `id1`, defined);
  VACSD/VASID (lines 266/260) are VACUOUS because the collapsed artefact is
  emitted UNSPECIALISED (no `specialise` clause, parent absent); VATCD
  (line 269 — code spec-level ≤ archetype level) trivial when all codes flat.
  VTSD (`master07` line 71) is why defined-but-UNUSED flattened-ontology codes
  also collapse, not just used ones. VTCBK (`master07` line 80) governs the
  binding-key ac remap.

**RESOURCE_DESCRIPTION.conversion_details** is a real BASE field
(`BASE/.../resource_description.adoc` line 80; BMM doc: "Details related to
conversion process… as name/value pairs, e.g. [tool]=<cem2adl v6.3.0>") —
using it for converter provenance is its documented purpose.

**No-collision proof for re-mint**: `alloc_id` monotonically advances
`next_id` (started at max_shifted_id+1, already past deferred-collapse ids);
`assigned_node_ids` catches any residual shift-fallback collision and re-mints
again. Sound.

**Two robot-corpus OPT fixtures are genuinely XSD-invalid** (parse-gate
adjudications in `openehr-its/tests/opt14_corpus.rs`): `Template.xsd:22`
`OPERATIONAL_TEMPLATE.language` and `BaseTypes.xsd:146` `DV_PROPORTION.type`
both have NO minOccurs → mandatory. `minimal_action_removed_language.opt` has
`<language>` only nested in description/details (not the direct OPT child);
`ehrn_vital_signs.v2.opt`'s DV_PROPORTION default_value omits `<type>`. The
gate asserts `is_err()` (rejection direction) + a seen-count guard — tighter
than a skip.
