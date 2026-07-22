---
name: cnf-opt-vitals-alias-mismatch
description: Live catalogue defect — cnf.opt.vitals resolves to minimal_action.en.v1, not a vitals OPT
metadata:
  type: project
---

The corpus alias `cnf.opt.vitals` maps (MANIFEST.yaml) to
`corpus/templates/vitals.opt`, whose actual `template_id` is
`minimal_action.en.v1` — concept "Minimal action", a root ACTION archetype
with web-template root `minimal` (NOT a vitals/body_temperature OBSERVATION).

**Why:** it caused 8 red SF rows in the 2026-07-22 ehrbase-rs baseline. Every
SF fixture/decision-table that references `vitals/body_temperature/.../temperature`
(flat.multi_event, structured.vitals.minimal, structured.raw_quantity, and the
SF-CTX/INDEX/MAP/NODEID content decision tables) commits against this alias and
the SUT correctly 422s ("Simplified-Format conversion failed: unknown simplified
path: vitals" — spec: ITS-REST simplified_formats master04 §Validation "Field
identifiers match WT metadata structure"). Only `fixtures/flat/vitals.minimal_ctx.json`
(root `minimal/minimal:0/...`) actually matches the template and commits (201).
Each affected case core carries a TODO admitting "commits against the removed
body_temperature carrier and 422s".

**How to apply:** these are CATALOGUE artifact defects, not app/runner. The
`created` expectation is wrong because the fixture is inconsistent with the bound
template. Fix path: author/restore a real vitals OPT (with
`openEHR-EHR-OBSERVATION.body_temperature.v1`, a repeating event, a DV_QUANTITY
leaf) as `cnf.opt.vitals`, OR repoint the SF fixtures/tables to `cnf.opt.maximal`
(`test_all_types.en.v1`) as the TODOs propose. Verify current state before
re-attributing — an implementer may have fixed the corpus.
