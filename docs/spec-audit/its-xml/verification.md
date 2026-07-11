# A1 Spec Audit — Verify + Fix — chapter `its-xml`

- **Chapter:** ITS-XML canonical XML (XSD 1.0.2 nsv1 bundle, namespace
  `http://schemas.openehr.org/v1`)
- **Date:** 2026-07-12
- **Scope:** all 60 requirements `its-xml-R1 … R60`
- **Result (defer-nothing pass):** zero defects — the XML surface is
  **generated from the vendored XSDs** (element order, attribute/element
  split, `xsi:type` slots are codegen inputs, so the wire shape conforms by
  construction), and the executable gates prove it: full-corpus XML
  round-trip, the C14N gate against the CNF canonical fixtures, the
  archetype-node-id-as-attribute suite, EHRbase-emitted-XML read tests, and
  the string-Hash/StringDictionaryItem round-trip. Zero deferrals.

## Verdict table (condensed)

| ids | classification | evidence |
|---|---|---|
| R1 | verified | serialize-time namespace selection (v1); `xml_c14n` gate matches the CNF canonical fixtures byte-wise |
| R2, R3 | verified | `archetype_node_id` emitted/read as an XML **attribute** on every LOCATABLE (`xml_locatable_attr` suite, incl. demographic/extract types); value validity is the RM `Archetype_node_id_valid` layer (ch1/ch10) |
| R4–R9 | verified | `xsi:type` dispatch is the generated runtime's core: abstract slots require it (missing → typed parse error), deep descendants route through the full descendant→variant map, foreign types error, equal-static-type omits it (ADR-005 emitters; round-trip gates) |
| R10–R22, R26–R40 | verified | element order + mandatoriness are emitted from the XSD `xs:sequence` (inherited-first); missing mandatory elements are typed `FromXml` errors; corpus + EHRbase-fixture read gates exercise the shapes (incl. DV_INTERVAL bounds flags, CODE_PHRASE, quantity/count/ordinal/proportion field types, base64 multimedia, OBJECT_REF/AUDIT_DETAILS/ATTESTATION/VERSION family, CONTRIBUTION min-1 versions, EHR_STATUS/EHR orders, SECTION recursion) |
| R23 | verified | PROPORTION_KIND ∈ {0..4} — RM `Type_validity` (ch4/ch6 validation layer) |
| R24 | verified | TERM_MAPPING.match ∈ {`>`,`=`,`<`,`?`} (`term_mapping_impl::is_valid_match_code` + Validate dispatch) |
| R25 | verified | ISO 8601 value patterns enforced at the RM validity layer (`valid_iso8601_*`, chs 4/11 — incl. the asymmetric timezone bounds), a stricter superset of the XSD patterns |
| R41–R60 | verified | the remaining class shapes (demographic, extract, folder/versioned-object, party family, generic entry) are the same generated surface, exercised by the corpus + fixture gates; the version-family XML wire (F-05-06) landed at B6 (ECC-COM-022) |

## Fixes applied

None required (gates green: `xml_roundtrip`, `xml_c14n`, `xml_ehrbase`,
`xml_locatable_attr`, `xml_hash`, `opt14_corpus` — 21/21).

## Deferred

None.

## Uncertain / runtime probes

None remaining.
