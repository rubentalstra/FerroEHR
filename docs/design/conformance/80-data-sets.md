# W-10 register 80 — data-set strategy (the ECC-law tension resolved)

Orchestrator register, 2026-07-13.

## The tension (prompt §Mission 5)

ECC law (owner, 2026-07-08): *generated data sets, never a Robot mapping*.
Current reality: the loader points straight into the vendored Robot corpus —
`tools/conformance/src/testdata/fixtures.rs:38` hardcodes `CORPUS_ROOT =
…/CNF/tests/platform/robot/_resources/test_data_sets`, and suites resolve
corpus paths ad hoc (`fixtures::read("valid_templates/…")`,
`suites/support.rs:52`). The only deliberate ownership is the B2-era
owned-fixture register (`testdata/fixtures/REGISTER.md`, 3 files: corrected
copies of internally-defective corpus compositions + byte-pinned defective
originals).

The corpus is not illegitimate — it is the schedule's own referenced data
(`master15-content_tc_composition.adoc` §Implementation notes even says the
constraint archetypes *"should be generated"*, i.e. the schedule expects a
mix of referenced data and generation). What violates the law is the
*accident of a path constant*: nothing records which corpus files the
instrument depends on, or why, or what adaptations bridge them to RM 1.2.0.

## Ruling

1. **Every data set the suites use is named in a committed fixture
   manifest** (`tools/conformance/testdata/MANIFEST.tsv`), one row per
   fixture key: `key · kind (opt|composition|aql-golden|ehr-status|…) ·
   source (owned:<path> | generated:<author-fn> | corpus:<rel-path>) ·
   adaptation (none | named rule) · note/citation`. The loader resolves
   fixture keys through the manifest ONLY; the `CORPUS_ROOT` free-path seam
   is deleted. A suite cannot read a file the manifest does not name — the
   `cnf coverage guard` fails on unmanifested access and on manifest rows no
   suite uses (both directions logged, never silent).
2. **Preference order for new/rewritten cases:** `generated:` (programmatic
   authoring — `testdata/author.rs`, the master15–17 approach) →
   `owned:` (reviewed committed file) → `corpus:` (vendored Robot data as
   raw material, allowed where the data is large clinical payload the
   schedule itself references). `corpus:` rows carry the pinned CNF commit
   implicitly (repo-vendored, `CNF/PROVENANCE.md`) and the named adaptation
   rule where one applies (the RM-version bridge becomes an edition-ladder
   rung, register 90 §4 — not an ad-hoc normalizer).
3. **The B2 owned-fixture register carries forward unchanged** (corrected
   copy under `valid/`, byte-pinned defective original under `invalid/` with
   a negative case): those rows appear in the manifest as `owned:` with the
   defect citation.
4. **AQL goldens** get per-golden manifest rows; the B5/D3 dialect
   adjudications (LIMIT-before-ORDER-BY = corpus-dialect defect) live in the
   adjudication register and are referenced from the manifest note column —
   a golden is never edited to pass.
5. **No Robot *machinery***: the manifest names data files only; Robot
   `.robot` files are never parsed, mapped, or referenced by the runner
   (coverage evidence at register-authoring time only — the standing law).

This makes ownership, generation, and provenance deliberate design (the
prompt's requirement) while keeping the schedule's own referenced data
usable as raw material.
