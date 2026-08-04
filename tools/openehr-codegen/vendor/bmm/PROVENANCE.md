# Vendored openEHR BMM meta-model (ITS-BMM), verbatim

Source: https://github.com/openEHR/specifications-ITS-BMM
Pinned commit: `37b317396eb9b5f03c4c422296a15bdd665028dd` (master)
Fetched: 2026-07-04 · Layout: **verbatim upstream `components/`** mirror.
License: Apache-2.0 (the ITS-BMM repo's `LICENSE`; root reference copy
`LICENSE-APACHE-2.0`). The one BASE-exception file below comes from
`specifications-BASE`, whose repo is CC-BY-SA 3.0 — it is the same BMM
artifact ITS-BMM redistributes under Apache-2.0, vendored from the component
repo only until ITS-BMM republishes it.

**Exception — BASE 1.3.0 json (2026-07-25):**
`components/BASE/json/openehr_base_1.3.0.bmm.json` is vendored verbatim from
the component repo https://github.com/openEHR/specifications-BASE
(`computable/BMM/openehr_base_1.3.0.bmm.json` @
`e48795762a0648cbe5701be58d42ec5df0c701a7`, master), because ITS-BMM had not
republished the 2026-07 BASE corrections (SPECBASE-48 invariants, SPECAM-82
CODE_PHRASE package move, SPECPR-426/386/460 fixes) at our ITS-BMM pin. The
component repo is the upstream source ITS-BMM redistributes from. Fold back
to a plain ITS-BMM pin at the next ITS-BMM bump. One knock-on: ITS-BMM had
independently fixed a doc typo (`senstive`→`sensitive`) in
`RESOURCE_DESCRIPTION_ITEM.other_details` (see the 2026-07-04 note below);
the specifications-BASE file still carries the typo, so the generated doc
comment regresses to authentic component-repo text.

The entire ITS-BMM `components/` tree is vendored under `components/` here —
all released versions in all three serializations (**json**, **odin**, **yaml**:
18 files each). The `json` files are upstream's source of truth (`odin`/`yaml`
are generated from them by `bmm-publisher`); we consume **json** for codegen.

## Codegen input (the pinned versions `openehr-codegen` loads)

`openehr-codegen`'s `main.rs` loads these JSON files (paths relative to this
dir), per `docs/VERSIONS.md`:

| Const | Path |
|---|---|
| BASE | `components/BASE/json/openehr_base_1.3.0.bmm.json` |
| RM   | `components/RM/json/openehr_rm_1.2.0.bmm.json` |
| TERM | `components/TERM/json/openehr_term_3.1.0.bmm.json` |
| AM 1.4 | `components/AM/json/openehr_am_1.4.0.bmm.json` |
| AM 2.4 | `components/AM/json/openehr_am_2.4.0.bmm.json` |
| LANG | `components/LANG/json/openehr_lang_1.1.0.bmm.json` |
| LANG (BMM-3) | `components/LANG/json/openehr_lang_1.1.0-bmm3.bmm.json` |

Why LANG spans two files: the primary one carries the persisted-BMM (`P_BMM_*`),
the `EXPR_*` model, and `STATEMENT_SET`/`ASSERTION` (referenced by AM's
`rules`/`includes`); the `-bmm3` file carries the full `BMM_*` object model and
the `EL_*` expression language (referenced by AM's persisted-archetype `rules`).
Neither alone resolves every AM reference, so both are merged
(`BmmSchema::combined`).

## Everything else vendored (available, not currently codegen input)

All other versions are present for completeness / future spec bumps and the
odin/yaml forms as reference: RM 1.0.2/1.0.3/1.0.4/1.1.0; BASE
1.0.4/1.1.0/1.2.0; AM 2.2.0/2.3.0; LANG 1.0.0; TERM 3.0.0 — plus every version's
`odin/` and `yaml/` serialization.

These are the deterministic input to `openehr-codegen`. The ODIN
reader in `openehr-lang::odin` is retained for ADL/ODIN *instance* parsing
(P8/P9), not for BMM ingestion.

Note (drift caught 2026-07-04): re-vendoring verbatim at this commit corrected a
one-character doc typo (`senstive`->`sensitive`) in BASE's
`RESOURCE_DESCRIPTION_ITEM.other_details`, so the generated `openehr-base` doc
comment now matches authentic upstream.
