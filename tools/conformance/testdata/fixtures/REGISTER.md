# Owned fixture register

Corrected copies of vendored CNF fixtures that are **internally inconsistent**
with their own operational template when read against the vendored openEHR spec
text.

> **The fixture manifest governs access.** Every data
> set the suites use — owned, corpus-derived, or generated — is named in
> `tools/conformance/testdata/MANIFEST.tsv`, one row per fixture key, and the
> loader (`src/testdata/fixtures.rs`) resolves keys through that manifest ONLY
> (no free-path corpus seam). The owned files documented below appear there as
> `owned:` rows; their `key`s are:
> - `owned.composition.all-types.valid` — `valid/compositions/all_types.composition.json`
> - `owned.composition.all-types-v2.valid` — `valid/compositions/all_types_v2.composition.json`
> - `owned.composition.all-types.invalid` — `invalid/compositions/all_types.composition.json`
>
> The vendored originals they are checked against are the `corpus:` rows
> `composition.all-types.vendored` / `composition.all-types-v2.vendored`. This
> B2 policy text stands unchanged; the manifest is the access layer over it.

## Layout

Fixtures are separated by validity and then by kind:

```
testdata/fixtures/
├── valid/
│   └── compositions/     # corrected copies that are VALID against their OPT
└── invalid/
    └── compositions/     # byte-faithful copies of the DEFECTIVE originals
```

(`compositions/` is the only kind so far; `templates/` and other kinds slot in
beside it as the register grows.)

## Policy

The vendored CNF corpus under `docs/specs/openehr/` is read-only and is **never
edited** (hard rule). When a vendored fixture is proven defective against the
vendored spec:

- a corrected copy lives under `valid/<kind>/` as a reviewed file, produced by
  script from the vendored original with the minimum change needed to make it
  satisfy its stated template;
- a byte-faithful copy of the defective original lives under `invalid/<kind>/`
  and is committed by a companion **negative** ECC case that asserts the SUT
  rejects it, so the defect itself stays under test (a `fixtures` guard test
  pins the `invalid/` copy byte-identical to the vendored source so it cannot
  drift);
- conformance cases load these copies through the owned-fixture loader
  (`crate::fixtures::owned_fixture`); they never mutate vendored data in code.

Origin: owner ruling 2026-07-09 (B2 — validation-depth). This register replaces
the former in-code `adapt_all_types_date` mutation in
`suites/content/drive.rs`.

---

## `compositions/all_types.composition.json`

- **Vendored source:**
  `docs/specs/openehr/CNF/tests/platform/robot/_resources/test_data_sets/query/data_load/compositions/all_types.composition.json`
- **Constraining OPT:**
  `.../valid_templates/all_types/Test_all_types.opt`
- **Leaf changed** (JSON path
  `/content/2/items/0/items/0/items/0/activities/0/description/items/0/value/value`
  — the INSTRUCTION activity's `description/items` ELEMENT `at0003`, a `DV_DATE`):
  - **old:** `"2021-10-18"` (a full `yyyy-mm-dd` date)
  - **new:** `"2021-10"` (year-month only)
- **`valid/` copy:** the corrected composition (day dropped) — the accepted base
  of the `all_types` content-validation cases.
- **`invalid/` copy:** the defective original verbatim — committed by the
  negative case below.
- **Negative case:** `val/dv-date-day-disallowed-pattern` (`ECC-VAL-119`,
  `suites/content/data_types.rs`).

## `compositions/all_types_v2.composition.json`

- **Vendored source:**
  `docs/specs/openehr/CNF/tests/platform/robot/_resources/test_data_sets/query/data_load/compositions/all_types_v2.composition.json`
- **Constraining OPT:**
  `.../valid_templates/all_types/Test_all_types_v2.opt`
- **Leaf changed** (same JSON path as v1, ELEMENT `at0003`, a `DV_DATE`):
  - **old:** `"2021-10-20"` (a full `yyyy-mm-dd` date)
  - **new:** `"2021-10"` (year-month only)
- **`valid/` copy only.** v2 carries the same defect as v1, so it needs no
  separate negative case — v1's `val/dv-date-day-disallowed-pattern` already
  keeps the defect under test. The v2 corrected copy is retained because it is
  **load-bearing**: it is the accepted base of `val/dv-coded-text-local-codes`
  (`run_dv_coded_local`), which commits it, then commits a copy with the
  `DV_CODED_TEXT` code off the `local` `{at0023,at0024}` code list (rejected).
  Removing it would break that case.

### Why the vendored fixtures are defective

The `at0003` `DV_DATE` leaf is constrained by a `C_DATE` whose pattern is
`yyyy-??-XX` (OPT: `Test_all_types.opt` line 1950 / `Test_all_types_v2.opt`, the
`C_DATE` under the INSTRUCTION `activities/description/items` DATE `children`).

In the AOM 1.4 `C_DATE` pattern grammar
(`docs/specs/openehr/AM/docs/UML/classes/org.openehr.am.aom14.c_date.adoc`,
*C_DATE Class*): the month field `??` is **optional**
(`VALIDITY_KIND.optional`) and the day field `XX` is **disallowed**
(`VALIDITY_KIND.disallowed`) — "There is no validity flag for 'year', since it
must always be by definition mandatory". `C_DATE` further carries the invariants
`Month_validity_optional`/`Month_validity_disallowed`, and `XX` on the day means
a day component must **not** appear in a conforming value.

The vendored value `2021-10-18` (resp. `2021-10-20`) supplies a **day**
component, which `yyyy-??-XX` disallows. A spec-correct validator must therefore
**reject** the vendored fixture. EHRbase/archie is lenient on `day_validity` and
accepts it, which is how the inconsistency survived unnoticed upstream. The
corrected copies truncate the leaf to `2021-10` (year-month, day absent), which
conforms to `yyyy-??-XX`, so each corrected composition is a genuinely valid
base for the content-validation cases that build on it.

### Reproduction

Regenerate the corrected `valid/` copies from the vendored originals (loads each
vendored JSON, walks to the `at0003` node whose `value` is a `DV_DATE`,
truncates that one leaf to its `yyyy-mm` prefix, and re-serializes with
`indent=4` / no key sorting so only that single leaf differs). The `invalid/`
copies are `cp` of the vendored originals verbatim.

```python
import json
VDIR = "docs/specs/openehr/CNF/tests/platform/robot/_resources/test_data_sets/query/data_load/compositions"
ODIR = "tools/conformance/testdata/fixtures/valid/compositions"

def patch(node, changes):
    if isinstance(node, dict):
        if node.get("archetype_node_id") == "at0003":
            v = node.get("value")
            if isinstance(v, dict) and v.get("_type") == "DV_DATE" \
               and isinstance(v.get("value"), str) and len(v["value"]) > 7:
                changes.append((v["value"], v["value"][:7]))
                v["value"] = v["value"][:7]
        for val in node.values():
            patch(val, changes)
    elif isinstance(node, list):
        for val in node:
            patch(val, changes)

for name in ("all_types.composition.json", "all_types_v2.composition.json"):
    d = json.load(open(f"{VDIR}/{name}", encoding="utf-8"))
    changes = []
    patch(d, changes)
    assert len(changes) == 1, (name, changes)
    open(f"{ODIR}/{name}", "w", encoding="utf-8").write(
        json.dumps(d, ensure_ascii=False, indent=4))
```

---

# Owned vendored external templates (not corpus corrections)

The register also holds **owned, byte-faithful copies of official openEHR
templates that are not part of the CNF Robot corpus** — vendored so a
conformance case can drive a real, published operational template through the
wire. These are not corrections: they are copied verbatim from their published
source and their `owned:` manifest rows carry `adaptation = none`.

## `templates/international_patient_summary.opt`

- **Source:** the official openEHR **CKM** "International Patient Summary"
  template (CKM template id `1013.26.376`), CKM's own operational-template
  (OPT 1.4) export. Copied verbatim from
  `tools/benchmark/templates/ckm/international-patient-summary.opt` (the
  benchmark toolkit's vendored copy); OPT `<uid>` `937fca6c-ec24-4c0f-8986-623843b6ebca`,
  `template_id` `International Patient Summary`.
- **Manifest key:** `owned.template.ips`
  (`owned:valid/templates/international_patient_summary.opt`).
- **Driven by:** `tpl/adl14-example-roundtrip`
  (`suites/definition_adl14.rs`) — upload the OPT, `GET
  /definition/template/adl1.4/{template_id}/example`, then POST the generated
  COMPOSITION to a fresh EHR and require `201`.
- **Why this template:** its `ACTION.medication` constrains `description` to
  `ITEM_TREE[at0017]` with no leaf content, so the server's example generator
  must synthesise that structural attribute. A generator that stamps a blind
  `at0001` placeholder produces a COMPOSITION the server's own validator then
  rejects ("unexpected node 'at0001' under 'description'"); the example must be
  committable (AOM 1.4 `master04-constraint_model_package.adoc` §`Valid_value`;
  CNF `master15-content_tc_composition.adoc` L38 — a generated instance must be
  RM/template-valid). This is the end-to-end guard for that class of defect.

There is **no CNF-vendored source guard** for this file (unlike the corrected
`invalid/` copies above): it has no corpus counterpart, being an external CKM
export vendored as-is.
