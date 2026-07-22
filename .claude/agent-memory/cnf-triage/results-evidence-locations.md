---
name: results-evidence-locations
description: Where the CNF observation, catalogue, and spec text live for triage
metadata:
  type: reference
---

For the committed ehrbase-rs run, per-row observation is in
`docs/conformance/ehrbase-rs/results.json` under `outcomes[]`: each entry has
`case`, `status` (passed/failed/not_applicable), `failing_step`, `reason`
(e.g. "expected `created`, observed `validation_failed`"). Long `reason`
strings are TRUNCATED in the committed file (…), so reproduce on the wire for
the full diff. `run-exceptions.json` = the not_applicable/unrealized list only.
`verdicts.json` = computed capability/profile roll-up.

- Case cores: `tools/cnf-runner/artifacts/schedule/<family>/<CASE>.yaml`
  (SM operation + outcome kinds + spec_refs + often a self-documenting TODO
  naming the real defect).
- Operation bindings (wire realization): `tools/cnf-runner/artifacts/bindings/its-rest/<SM.op>.yaml`.
- Corpus: `tools/cnf-runner/artifacts/corpus/MANIFEST.yaml` maps data-set
  aliases (`cnf.*`) → fixture files + `template_id`; fixtures under
  `corpus/fixtures/**`, templates under `corpus/templates/**`.
- Party/statement: `tools/cnf-runner/party/ehrbase-rs/{statement.json,ixit.json}`
  — `statement.options` holds the selected `option_select` branches.
- Spec oracle: `docs/specs/openehr/` (index in its README). SF cases →
  `ITS-REST/docs/simplified_formats/master0{2,4,5,6}-*.adoc`. ADL14 →
  `CNF/docs/platform_test_schedule/master04-func_tc_definition_adl.adoc` +
  `ITS-REST/specifications/parameters/path/template_id.yaml`.
