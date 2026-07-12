# A1 Spec Audit — Verify + Fix — chapter `rm-demographic`

- **Chapter:** RM 1.2.0 demographic (master02 + UML classes)
- **Date:** 2026-07-11
- **Scope:** all 42 requirements `rm-demographic-R1 … R42`
- **Result (defer-nothing pass):** 3 gaps fixed. The demographic wire is our
  own extension (no openEHR REST binding exists — flagged), but the RM
  semantics below are normative and enforced.

## Verdict table (condensed — mechanism per group)

| ids | classification | evidence / fix |
|---|---|---|
| R1/R10/R19/R24/R26/R29/R31/R32 (mandatory attrs) | verified | fail-closed typed deserialize in `demographic.rs::typed_check` / `relationship.rs::typed_check` (generated structs pin the 1..1 fields) |
| R2 | verified | `typed_check` `Identities_valid` |
| R3/R13/R22 + R4 first arm | fixed-in-this-pass | present-empty `contacts`/`relationships`/`roles`/`capabilities` rejected on the raw JSON (`typed_check`) — post-deserialize invisible |
| R4 second arm | fixed-in-this-pass | inline `relationships[].source` must reference THIS party (uid compare, case-folded) — relationships live under their source (party.adoc) |
| R5/R11/R27/R30/R36 (`type()`/`purpose()` = name) | verified-vacuous | derived functions; no independent wire state |
| R6 | verified-with-adjudication | `archetype_node_id` presence enforced (typed); `archetype_details` presence follows the standing CNF/corpus adjudication (see rm-ehr R14) |
| R7/R41 | verified | the read path injects `uid` from the version container (`version_response`); stored bodies never lose it |
| R8/R12/R21/R25/R28 (optional cardinalities) | verified | `Option`/`Vec` typed fields |
| R9/R35 | verified-by-design | reverse relationships are COMPUTED (SQL lookups in `relationship.rs`), never stored on the target |
| R14/R20/R33 (PARTY_REF slots) | verified | typed `PartyRef` fields, fail-closed; `party_ref_impl` Type_validity covers the ref type set |
| R15–R18/R23 (inheritance) | verified | PERSON/ORGANISATION/GROUP/AGENT/ROLE all route through the same `typed_check` + PARTY checks |
| R34 | verified-by-design | our wire stores relationships as their own versioned objects keyed to the source (an extension design — no openEHR spec governs the demographic REST wire); the source linkage is explicit in `source` and validated (R4 arm) |
| R37/R38 | verified | every party/relationship is a `VERSIONED_OBJECT` on the shared vo machinery (chapter 1 semantics: contribution + audit per write) |
| R39 | verified | `target: PartyRef` — a by-value party cannot deserialize into the slot |
| R40 | fixed-in-this-pass | `relationship.rs::typed_check`: `source`/`target` ids must identify the version CONTAINER — an `OBJECT_VERSION_ID` in the ref id is rejected (RM demographic master02) |
| R42 | verified-behavioural | modelling guidance (identities vs external ids); no checkable wire rule |

## Fixes applied

- `demographic.rs::typed_check` — present-empty list invariants + the
  relationships-source-is-self arm; test fixtures corrected (a present-empty
  `capabilities: []` violates `Capabilities_valid` — absence is the valid
  encoding), incl. the ECC demographic fixture.
- `relationship.rs::typed_check` — container-id (not version-id) refs.
- One missed tree-column SELECT from the chapter-1 storage rewrite fixed on
  the demographic contribution path.

## Deferred

None.

## Uncertain / runtime probes

None remaining.
