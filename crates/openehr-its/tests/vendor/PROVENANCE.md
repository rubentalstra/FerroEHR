# Vendored real-world openEHR canonical-JSON test corpus

These are **real** openEHR canonical-JSON instances used as the integration
oracle for `openehr-serde` (deserialize → re-serialize → equality + ITS-JSON
schema validation). They are external ground truth — not hand-authored in this
repo — so the round-trip suite tests genuine interoperability, not our own
assumptions. Do not edit the vendored files; if a fixture needs correcting,
re-vendor from upstream and update the pin below.

## openehr_sdk/

| Item     | Value                                                                                               |
|----------|-----------------------------------------------------------------------------------------------------|
| Upstream | `ferroehr/openEHR_SDK`                                                                               |
| Ref      | branch `develop`, commit `22b01e0c99b53669394e56da29c2410838b5cf7e`                                 |
| Path     | `test-data/src/main/resources/{composition,contribution,ehr,folder,item_structure}/canonical_json/` |
| License  | Apache-2.0 (see the upstream `LICENSE.md`)                                                          |
| Fetched  | 2026-07-03                                                                                          |

`ferroehr/openEHR_SDK` is the serialization library EHRbase itself uses, so its
`canonical_json` corpus is the closest available match to our parity baseline
(EHRbase v2.33.0). Files were downloaded verbatim from the pinned commit via
the GitHub contents API (raw), preserving the upstream sub-directory layout.

### Corpus composition (72 files)

- `composition/canonical_json/` — 57 COMPOSITION instances (all-types
  systematic tests, feeder-audit variants, nested PARTY_* forms, minimal
  per-entry-type, interval/duration/datetime/time-series, IPS, laboratory,
  GECCO, etc.).
- `contribution/canonical_json/` — 6 files (2 canonical CONTRIBUTION RM
  objects; 4 are EHRbase `{versions, audit}` request DTOs, **not** RM
  objects — excluded from the RM round-trip corpus).
- `ehr/canonical_json/` — 2 EHR_STATUS instances.
- `folder/canonical_json/` — 6 FOLDER instances (one, `folder_with_items`,
  omits the redundant top-level `_type`, legal per ITS-JSON when the declared
  slot is the concrete FOLDER).
- `item_structure/canonical_json/` — 1 ITEM_TREE instance.

### Files excluded from the corpus gates (documented, not silently)

**There is exactly ONE exclusion registry: `excluded()` in
`crates/openehr-its/tests/it/common.rs`.** It is the authoritative,
per-file list with each entry's adjudication and spec citation; every
harness (the readability / round-trip / ITS-JSON-schema gates in
`fidelity.rs`, the canonical-output contract gate, the XML round-trip gate,
the RM-validation mutation battery) consumes it — none keeps a second
by-name list, and none absorbs a non-canonical document through a shape
heuristic. A corpus file that is not a canonical single-RM-object root and
has NO registry entry **fails** the readability gate, so an exclusion
cannot stop applying unnoticed.

This document deliberately does not restate the list (a second copy is how
the three mechanisms drifted apart). Read the registry. The families it
covers: legacy Jackson `@class` documents, the EHRbase raw-DB
row-per-locatable shapes, ITS-REST contribution request DTOs, deliberately
invalid negatives, defective upstream fixtures (each with a repo-authored
VALID TWIN under `tests/fixtures/twins/`), and RM-1.1-era documents that
omit members RM 1.2 makes mandatory.

## RM version note

The SDK corpus is authored for RM 1.0.4 (its `archetype_details.rm_version`
is typically `"1.0.4"`), while this workspace targets RM 1.1.0 and validates
against the vendored ITS-JSON 1.1.0 schema. The canonical JSON wire shape is
identical between 1.0.4 and 1.1.0 for every class these files exercise; any
genuine version-specific divergence surfaced by the round-trip suite is
documented at the point it is skipped (with the specific field/class), never
worked around by weakening an assertion.

## Coverage gap

No authoritative real-world corpus exists for the `rm.demographic` package
(`PARTY`, `PERSON`, `ORGANISATION`, `ROLE`, `ACTOR`, `AGENT`, `GROUP`,
`CONTACT`, `ADDRESS`, `CAPABILITY`, `PARTY_IDENTITY`, `PARTY_RELATIONSHIP`) —
archie, `specifications-RM`, and `openEHR_SDK` ship none, reflecting that
openEHR deployments keep demographics in a separate repository and rarely
serialize these classes. These classes remain covered by `openehr-rm`
unit tests and by minimal synthetic fixtures in `tests/gap_fixtures.rs`
(schema-validated), which the coverage test lists explicitly.
