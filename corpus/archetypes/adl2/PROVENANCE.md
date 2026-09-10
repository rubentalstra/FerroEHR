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

**These files state no licence.** Measured over this vendored tree on
2026-09-10, searching every `*.adls` and `*.adl` for `licence` and
`license` in any case and any position:

| dialect | files | state a licence |
|---|---|---|
| `*.adls` (ADL 2) | 322 | 1 |
| `*.adl` (ADL 1.4 twins) | 330 | 0 |

The single exception is
`ckm-2013-12-09/composition/openEHR-EHR-COMPOSITION.t_encounter_opt_test.v1.0.0.adls`,
and what it states is `Creative Commons CC-BY 4.0 unported` — not
CC-BY-SA 3.0. The other 651 carry `copyright = <"© openEHR Foundation">`
in their description block and no `licence` key at all.

Upstream states nothing either: at the pinned commit the repository has no
`LICENSE`, `LICENSE.md`, `LICENSE.txt` or `COPYING`, and its `README.md` is
four lines describing the contents ("ADL test, reference and example
archetypes") with no licensing statement.

So the position for this tree is: **openEHR Foundation copyright, terms
unstated.** An unstated licence is not a permissive one, and this record says
so rather than inferring a grant from the material being published for
testing. `REUSE.toml` declares the subtree `NOASSERTION` for the same reason.

This record previously claimed the archetypes were "predominantly CC-BY-SA
3.0 where stated" and pointed at the root `LICENSE-CC-BY-SA-3.0`. That was
wrong on both counts and is corrected here (#3150). The wording appears to
have been inherited from the CKM template pack, where a mixed CC-BY-SA
population genuinely is what the files say.

Whether a tree whose terms are unstated may stay committed is a separate
decision, tracked on issue 3194; nothing here asserts that it may.

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
