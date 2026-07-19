# Provenance — the ADL2 conformance corpus + flattener fixtures

## `adl2-reference/`

- Source: https://github.com/openEHR/adl-archetypes, directory
  `ADL2-reference/` ("openEHR ADL reference" library, per its
  `_repo_lib.idx`: "openEHR ADL 2 regression test archetypes").
- Commit: `093c77ea003742b9540e3dd377d615e2b26f2996` (2025-06-27)
- Content: ~300 `.adls`/`.adl` files under `validity/` (basics,
  specialisation, terminology, consistency, structure, rm_checking,
  annotations, domain_types, paths, slots, templates, legacy_adl_1.4),
  `features/`, `robustness/`, `upgrade/`.
- **File names encode the expected outcome**: the validity cases carry the
  AOM2/cADL rule code they must trigger (e.g.
  `openEHR-EHR-EVALUATION.VPOV_code_list_constrained.v1.0.0.adls`) or a
  `FAIL_`/`W…` prefix; this is the validator-conformance oracle keyed by
  rule code.
- Licensing: the repository carries no top-level LICENSE file; the content
  is openEHR Foundation test material (openEHR specifications and
  associated artefacts are published under CC-BY); individual archetype
  descriptions may carry their own `licence` field. Recorded as-is —
  test-fixture use with provenance retained.

## `flattener/`

- Source: https://github.com/openEHR/archie (Apache-2.0; `LICENSE`
  vendored alongside), test resources
  `tools/src/test/resources/com/nedap/archie/flattener/{specexamples,siblingorder}`.
- Commit: `e8d92f28aca33f92ea08a826ea19f9581d579720` (2026-07-08)
- `specexamples/` are the AOM2 specification's own worked flattening
  examples in archetype form; `siblingorder/` exercises the
  `before`/`after` insertion semantics of
  `docs/specs/openehr/AM/docs/ADL2/master09.04` §Ordering of Sibling
  Nodes.
- NOTE: each fixture is verified against the vendored spec text when the
  flattener lands — archie is prior art, and a fixture that contradicts
  the spec text is corrected/adjudicated (recorded here), never followed
  blindly.

Never hand-edit vendored fixtures; re-vendor and update the pins here.
