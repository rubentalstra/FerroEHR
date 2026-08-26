# ADL 2 archetype pack (with ADL 1.4 twins) — provenance

Vendored verbatim from `https://github.com/openEHR/adl-archetypes`
(`Reference/CKM_2013_12_09/`) at commit `093c77ea003742b9540e3dd377d615e2b26f2996` by
`scripts/vendor/adl2-archetypes.sh` on 2026-08-01T12:30:22Z.

Upstream describes the tree as archetypes exported from the Clinical
Knowledge Manager (export time Mon Dec 09 15:42:23 CET 2013).

## Why this source and not CKM

The live openEHR CKM publishes **ADL 1.4 only** — `/archetypes/{cid}/adl`
returns `adl_version=1.4` and there is no ADL 2 export endpoint
(`/adl2`, `/opt2` 404; `?format=ADL2` is ignored). The ADL 1.4 corpus is
therefore vendored live (`corpus/archetypes/ckm/`, ADL 1.4) and the ADL 2
corpus comes from this pinned upstream library.

The ADL 2 side is NEVER produced by running our own ADL 1.4->2 converter
over CKM output: that converter has no spec basis (our own design) and
would then be validated against its own output.

## Licensing

The upstream repository carries no top-level LICENSE file; individual
archetypes carry their own `licence` metadata (predominantly CC-BY-SA
3.0 where stated — see the individual file). openEHR Foundation
test/reference material, vendored verbatim with metadata retained;
root reference copy: `LICENSE-CC-BY-SA-3.0`.

## Contents

- ADL 2 archetypes (`*.adls`): **322**
- ADL 1.4 twins (`*.adl`): **330**
- archetypes present in BOTH dialects: **321**

The dual-dialect pairing is the value here: the same clinical archetype
in 1.4 and in 2, as published upstream — an INDEPENDENT reference for
the conversion path and matched inputs for the DEFINITION API's ADL 1.4
and ADL 2 wire cases.

| RM class | ADL 2 files |
|---|---|
| openEHR-EHR-CLUSTER | 116 |
| openEHR-EHR-OBSERVATION | 100 |
| openEHR-EHR-EVALUATION | 29 |
| openEHR-DEMOGRAPHIC-CLUSTER | 14 |
| openEHR-EHR-COMPOSITION | 12 |
| openEHR-EHR-INSTRUCTION | 11 |
| openEHR-EHR-ACTION | 9 |
| openEHR-EHR-SECTION | 9 |
| openEHR-DEMOGRAPHIC-ADDRESS | 4 |
| openEHR-DEMOGRAPHIC-ROLE | 4 |
| openEHR-DEMOGRAPHIC-PARTY_IDENTITY | 3 |
| openEHR-EHR-ITEM_TREE | 3 |
| openEHR-DEMOGRAPHIC-PERSON | 2 |
| openEHR-EHR-ELEMENT | 2 |
| openEHR-DEMOGRAPHIC-CAPABILITY | 1 |
| openEHR-DEMOGRAPHIC-ITEM_TREE | 1 |
| openEHR-DEMOGRAPHIC-ORGANISATION | 1 |
| openEHR-EHR-ADMIN_ENTRY | 1 |

Never hand-edit a vendored fixture; re-run this script and bump the pin.
