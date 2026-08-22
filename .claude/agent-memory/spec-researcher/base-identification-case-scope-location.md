---
name: base-identification-case-scope-location
description: Where BASE's identifier case/equivalence rules live (base_types master05 section map) and exactly how far the "all composite identifiers" case rule reaches — the OBJECT_ID hierarchy incl. externally-issued GENERIC_ID / PARTY_REF.id
metadata:
  type: reference
---

# BASE identifier case + equivalence — spec navigation

Owning chapter: `docs/specs/openehr/BASE/docs/base_types/master05-identification_package.adoc`
(294 lines). Section map (line numbers verified 2026-08-22):

| L | section |
|---|---|
| 11 / 29 / 33 / 44 / 54 | `== Requirements` + RWE / IE / Versions / Referencing |
| 61 | `== Design` (L63: this package models only INFORMATIONAL identifiers; real-world ids -> `DV_IDENTIFIER`) |
| 67 | `=== Primitive Identifiers` (UID/UUID/ISO_OID/INTERNET_ID — "no further internal structure") |
| **75** | `=== Composite Identifiers` — the scope-setting section |
| 87 / 91 / 117 / 134 / 138 / 156 / 160 | UID-based / Archetype / Terminology / **Equivalence** / Version-in-container / **Generic and External** / Hierarchical |
| **164** | `==== Composite Identifiers and Case` |
| 179 | `==== Composite Identifiers and Language` (basic latin, no diacritics) |
| 183 / 187 / 222 | `=== References` (OBJECT_REF = the FK analogue) / `== Class Descriptions` (includes) / `== Syntaxes` |

## How far the case rule reaches (the load-bearing fact)
- Scope sentence, **L77**: "The `OBJECT_ID` type and its hierarchy of subtypes
  defines all identifier types used within openEHR systems." The table at
  **L79-85** then splits them into "openEHR-defined" vs **"Externally defined
  identifiers"** (`TERMINOLOGY_ID`, **`GENERIC_ID`**, `HIER_OBJECT_ID`) — so
  externally-issued identifiers are explicitly INSIDE this section, and
  §Generic and External Identifiers (L156) is one of its sub-subsections.
- The rule itself, **L166**: "All composite identifiers **should** follow two
  rules with regard to case" -> L168 case-PRESERVING, **L169 case-INSENSITIVE**
  ("two identifiers identical apart from case are considered to be identical,
  and therefore to identify the same thing"). Lowercase "should" — BASE declares
  no RFC-2119 vocabulary (only ITS-REST does; see
  [[cnf-ambiguity-register-verification-anchors]]).
- **L174** contemplates external issuers and still applies the rules: original
  case "should be as published by the relevant issuing organisation (e.g. NLM
  UMLS terminology names are all upper case)".
- The ONLY carve-out is **L177**: "These rules do not apply to any identifier
  constructed in a language in which case does not exist as a concept" (Turkish
  'I/i'). **Nothing scopes the rule by ISSUER** — grep-verified.
- Reach to `PARTY_REF.id`: `UML/classes/org.openehr.base.base_types.object_ref.adoc`
  types `id: OBJECT_ID` (1..1); `...party_ref.adoc` inherits OBJECT_REF and adds
  only `Type_validity` — so a PARTY_REF's id IS in the OBJECT_ID hierarchy the
  L77 sentence sweeps. `...generic_id.adoc` inherits OBJECT_ID, adds `scheme`
  ("may well be local", "not controlled or standardised in any way").
- Wire consequence (why it matters): ITS-REST `responses/409_EHR.yaml` makes
  "the same subject id, namespace pair" the POST /ehr conflict key — OAS-only,
  the ITS-REST docs text never says "subject" (see
  [[its-rest-wire-contract-location]]). Register entry AMB-63.
- Released-text defect en route: **L83 spells `ARCHEYTPE_ID`** (same
  transposition as `ARCHEYTPE_TERMINOLOGY` in `AM/docs/UML/classes/
  org.openehr.am.aom2.archetype.adoc` L9 + the AM 2.4.0 BMM).

Related: [[object-ref-resolvability-location]],
[[locatable-uid-and-owner-id-landmarks]], [[base-time-ordering-location]].
