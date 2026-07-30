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

## `adl14-dadl/`

- **Hand-written in this repository — NOT vendored.** No upstream corpus
  exercises the breadth of the ADL 1.4 dADL leaf/structure grammar
  (`docs/specs/openehr/AM/docs/ADL1.4/master04-dadl.adoc`), so these fixtures
  are authored directly from that chapter (plus `master08-adl.adoc` §Revision
  History Section for the revision-history fixture).
- Each file name encodes its expected outcome, corpus-convention style: an
  `SDINV_*` prefix is a refusal fixture, everything else parses and validates
  clean and repeats that in its in-file `regression` tag.
- Owner: `crates/openehr-adl/tests/adl14_dadl_breadth.rs` (the per-file
  expectation table lives there; `corpus_coverage.rs` cross-checks the tree).
- Because it is not vendored, this tree may be edited — but a fixture is only
  ever added or corrected against the spec text, never weakened to make a
  build pass, and the accept/refuse twins stay paired.

## `adl14-cadl/`

- **Hand-written in this repository — NOT vendored.** Sibling of `adl14-dadl/`
  for the **cADL** half of an ADL 1.4 text
  (`docs/specs/openehr/AM/docs/ADL1.4/master05-cadl.adoc`, plus
  `master08-adl.adoc` §Validity Rules and `master09-customising_adl.adoc`); the
  vendored `adl2-reference` library is an ADL2 corpus and covers none of it.
- Three families: the **dialect gates** (a construct ADL 2 introduced is refused
  in a 1.4 text — master05 §Keywords L48-53 is a closed keyword set), the
  **inline dADL domain lowering** refusals with their accepting twin, and
  **positive fixtures** for behaviour an over-strict reader would break
  (`before`/`after` sibling order — a 1.4 keyword at L53 — and the effective
  occurrences default `{1..1}` at L316).
- File names encode the expected outcome corpus-convention style: an `S*_`
  prefix is a parse refusal with that syntax code, a `V*_` prefix is a phase-1
  validation error with that validation code, everything else parses and
  validates clean and repeats that in its in-file `regression` tag.
- The accepting twin of every DIALECT-GATE refusal is the vendored ADL2 corpus,
  which exercises the same construct in its own dialect; the twins of the
  1.4-only refusals live in this tree.
- Owner: `crates/openehr-adl/tests/adl14_cadl_gates.rs` (the per-file
  expectation table lives there; `corpus_coverage.rs` cross-checks the tree).
- Same editing rule as `adl14-dadl/`: fixtures are added or corrected against
  the spec text, never weakened to make a build pass.

Never hand-edit vendored fixtures; re-vendor and update the pins here.
