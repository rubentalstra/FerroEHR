---
name: ecc-own-conformance-framework
description: "2026-07-22 cutover — the acceptance instrument is the CNF 2.0 runner (tools/cnf-runner); the ECC harness is retired; official Robot DATA enters the corpus only as provenance-stamped re-adjudications"
metadata: 
  node_type: memory
  type: project
  originSessionId: 6e259293-e623-4384-b476-748dce5b3ab2
---

The conformance instrument is the **CNF 2.0 reference runner**
(`tools/cnf-runner`) — a ground-up rebuild on the CNF framework itself
(#202): machine-readable catalogue (case cores anchored on the official
schedule ids, per-ITS operation bindings, closed vocabularies, typed
ambiguity register), a data-driven interpreter under the five step laws,
and pure-function verdicts from (statement, results, catalogue, capability
matrix). `scripts/conformance.sh` is the pipeline; artifacts + baseline
live ONLY under `docs/conformance/<sut>/`.

**Why:** the 2026-07-08 "own framework (ECC)" pivot was superseded by the
owner-approved CNF 2.0 rebuild (#197/#202); the ECC harness retired
2026-07-22 with the reviewed comparison (`docs/conformance/cnf-comparison.md`,
gate clean). Its catalogue inventory is preserved at
`tools/cnf-runner/comparison/ecc-catalog.tsv`.

**How to apply:** never resurrect `tools/conformance` or ECC ids in new
work; expectations trace to spec text only; the upstream Robot suites stay
reference material, but their official DATA fixtures are adopted into the
runner corpus as provenance-stamped re-adjudications (the earlier
"no Robot ever" wording applied to CASE mapping, not data). Case authoring
defects are fixed with citations — see [[owner-work-style]] and the
catalogue-audit issue #231.
